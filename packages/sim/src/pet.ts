import type { AnimationName, Point, Rect, Remap } from "@tinyworld/core";
import { StateMachine, type StateTable } from "./fsm";
import { GREETINGS } from "./lines";
import { clamp } from "./math";
import {
  buried,
  climbTargets,
  groundOf,
  hidden,
  jumpTargets,
  landingLedge,
  nearestOpenX,
  openWall,
  roomy,
  type Ground,
  type JumpTarget,
} from "./moves";
import type { Rng } from "./rng";
import type { PetSnapshot } from "./snapshot";
import type { Bounds, Ledge, Terrain, Wall } from "./terrain";
import { TUNING } from "./tuning";

export { TUNING } from "./tuning";

/** State của pet, trùng tên với animation. */
export type PetState = AnimationName;

/** 1: nhìn sang phải, -1: nhìn sang trái. */
export type Facing = 1 | -1;

/** Môi trường pet sống, do `World` cung cấp. */
export interface PetEnv {
  readonly bounds: Bounds;
  readonly terrain: Terrain;
  readonly rng: Rng;
  /** Hệ số tốc độ đi/chạy (Settings), 1 là bình thường. */
  readonly speed: number;
  /** Con trỏ chuột, `null` khi chưa biết. */
  readonly cursor: Point | null;
  /** Mép `side` của màn hình có giáp màn hình khác ở độ cao `y` không: pet đi hoặc bay sang được. */
  exit(side: Facing, y: number): boolean;
  /**
   * Pet được tự đi sang màn hình khác không. Cả nhóm dùng chung một overlay, nên có từ 2 con trở lên thì
   * chỉ sang khi bị kéo hoặc ném, các con còn lại chạy theo (`Pet.joinFrom`).
   */
  readonly wander: boolean;
  /** Mọi pet đang sống trên màn hình, kể cả con này. */
  readonly pets: readonly Pet[];
  /** Đang là ban đêm: đi lại chậm hơn, buồn ngủ sớm hơn. */
  readonly night: boolean;
  /**
   * `pet` nói một câu cho vui (chào nhau, thời tiết). Cả nhóm có giới hạn tần suất, người dùng đang gõ
   * phím thì thôi; trả về `true` nếu đã nói.
   */
  chat(pet: Pet, text: string): boolean;
}

/** Câu pet đang nói, hiện thành speech bubble trên đầu. */
export interface Speech {
  text: string;
  /** Số giây còn hiện. */
  left: number;
}

/** State đang đi lại: con khác đang ở các state này thì con này hay đứng yên hơn (`TUNING.groupCalm`). */
const MOVING: ReadonlySet<PetState> = new Set(["walk", "run", "climb", "jump"]);

/**
 * Pet đang đứng trên (hoặc leo) cửa sổ `id`: điểm chân cách góc trên trái cửa sổ (dx, dy). Cửa sổ di
 * chuyển thì pet đi theo.
 */
export interface Mount {
  id: number;
  dx: number;
  dy: number;
}

/** Việc pet đang đi tới để làm. */
export type Goal =
  /** Đi tới chân tường rồi leo. */
  | { kind: "climb"; id: number; side: -1 | 1 }
  /** Ra chỗ không bị che, `dx` tính từ mép trái cửa sổ đang đứng. */
  | { kind: "peek"; dx: number }
  /** Buồn ngủ: đi tới đầu mép rồi xuống taskbar. */
  | { kind: "down" }
  /** Chạy trốn cửa sổ đang bị kéo tới: hết đường trên mép cửa sổ thì xuống luôn. */
  | { kind: "flee" }
  /** Đi cùng con khác một đoạn sau khi chào nhau (không gặp lại, không sang màn hình khác giữa chừng). */
  | { kind: "stroll" }
  /** Buồn ngủ: đi tới chỗ `x` dưới đất, nằm cạnh con đang ngủ. */
  | { kind: "nap"; x: number };

/** State pet đang tỉnh, đứng trên mặt đất hoặc mép cửa sổ và không bận việc gì: phản ứng được với cửa sổ. */
const CALM: ReadonlySet<PetState> = new Set(["idle", "walk", "run", "perch", "land"]);

/** State đứng yên ngay được (`Pet.hold`): đang rảnh, đang ngủ hay đang choáng trên mặt đất hoặc mép cửa sổ. */
const HOLDABLE: ReadonlySet<PetState> = new Set([...CALM, "sleep", "dizzy"]);

/** Đang sang màn hình bên cạnh qua mép `dir`. */
interface Crossing {
  dir: Facing;
  /** `out`: đang ra khỏi màn hình này, chờ overlay sang màn hình kia; `in`: đã sang, đang đi vào. */
  phase: "out" | "in";
  /** Số giây ở phase hiện tại. */
  elapsed: number;
  /**
   * Bị ném ra quá mép: vận tốc ngang lúc bị giữ lại ngay ngoài mép chờ overlay sang. Overlay sang lúc
   * pet còn đang rơi thì bay tiếp vào màn hình mới với vận tốc này, không rơi thẳng xuống ngoài mép.
   */
  vx?: number;
}

interface Climbing {
  /** Cạnh cửa sổ đang bám: -1 trái, 1 phải. */
  side: -1 | 1;
  /** -1: leo lên, 1: leo xuống. */
  dir: -1 | 1;
}

interface Jumping {
  /** Cửa sổ đáp lên hoặc bám vào (`null`: mặt đất). */
  id: number | null;
  /** Nhảy thẳng lên bám cạnh này của cửa sổ `id` (cửa sổ lơ lửng) rồi leo tiếp; `null`: nhảy để đáp. */
  grab: -1 | 1 | null;
  /** Điểm chân lúc đáp, tính từ mép trái cửa sổ (mặt đất: toạ độ overlay). */
  offset: number;
  phase: "crouch" | "air" | "touchdown";
  /**
   * Lúc bật nhảy: thời điểm (giây trong state `jump`), điểm chân, vận tốc. Lúc bay tính vị trí thẳng
   * theo đường parabol chứ không cộng dồn từng bước, để rơi đúng chỗ đã ngắm.
   */
  launch: { at: number; x: number; y: number; vx: number; vy: number };
  /** Thời điểm chạm chân (giây trong state `jump`). */
  landedAt: number;
}

const STATES: StateTable<PetState, Pet> = {
  idle: {
    enter: (pet) => plan(pet, TUNING.idleTime),
    update: (pet, time) => (time >= pet.planned ? nextActivity(pet) : undefined),
    exit: (pet) => {
      // Chưa kịp rủ đi cùng thì thôi (bị click, bị kéo, cửa sổ kéo tới...).
      pet.buddy = null;
    },
  },
  walk: {
    enter: (pet) => startMoving(pet, TUNING.walkTime),
    update: (pet, time, dt) => move(pet, TUNING.walkSpeed, time, dt),
    exit: (pet) => {
      pet.goal = null;
    },
  },
  run: {
    enter: (pet) => startMoving(pet, TUNING.runTime),
    update: (pet, time, dt) => move(pet, TUNING.runSpeed, time, dt),
    exit: (pet) => {
      pet.goal = null;
    },
  },
  // Chỉ thức dậy khi người dùng click hoặc kéo.
  sleep: {},
  react: {
    update: (pet, time) => hop(pet, time),
  },
  // Vị trí do chuột điều khiển qua `dragTo`.
  dragged: {},
  fall: {
    update: (pet, _time, dt) => fall(pet, dt),
  },
  land: {
    update: (_pet, time) => (time >= TUNING.landTime ? "idle" : undefined),
  },
  dizzy: {
    update: (_pet, time) => (time >= TUNING.dizzyTime ? "idle" : undefined),
  },
  climb: {
    update: (pet, _time, dt) => climb(pet, dt),
    exit: (pet) => {
      pet.climbing = null;
    },
  },
  perch: {
    enter: (pet) => plan(pet, TUNING.perchTime),
    update: (pet, time) => (time >= pet.planned ? leavePerch(pet) : undefined),
  },
  jump: {
    update: (pet, time) => jump(pet, time),
    exit: (pet) => {
      pet.jumping = null;
    },
  },
};

export interface PetOptions {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  /** Khoảng từ điểm chân tới tường lúc leo (CSS pixel), mặc định theo bề ngang. */
  reach?: number;
}

export class Pet {
  readonly id: string;
  /** Kích thước khi vẽ (CSS pixel), dùng để giữ cả con nằm trong màn hình. Đổi bằng `resize`. */
  width: number;
  height: number;
  /** Khoảng từ điểm chân tới tường lúc leo (CSS pixel). */
  reach: number;
  /** Điểm chân (giữa mép dưới) theo CSS pixel của overlay. */
  x: number;
  y: number;
  /** Vận tốc khi đang rơi, bị ném hoặc đang nhảy (px/s). */
  vx = 0;
  vy = 0;
  facing: Facing = 1;
  /** Số giây kể từ lần cuối người dùng click hoặc kéo pet. */
  sinceInteraction = 0;
  /** Số giây kể từ lần click trước, kể cả click bị bỏ qua. */
  private sincePoke = Number.POSITIVE_INFINITY;
  /** Thời lượng đã chọn cho lượt đứng / đi / chạy / ngồi mép hiện tại. */
  planned = 0;
  /** Cửa sổ đang đứng hoặc leo; `null` là đang ở mặt đất hoặc trên không. */
  mount: Mount | null = null;
  goal: Goal | null = null;
  climbing: Climbing | null = null;
  jumping: Jumping | null = null;
  /** Rơi qua mép cửa sổ này mà không đáp (vừa bước hoặc nhảy khỏi nó). */
  skipLedge: number | null = null;
  /** Nhảy thêm chừng này cái nữa sau cú nhảy `react` đang dở (ăn mừng). */
  hops = 0;
  /** Số giây kể từ lần ăn mừng trước. */
  private sinceCheer = Number.POSITIVE_INFINITY;
  /** Số giây kể từ lần quay đầu theo con trỏ. */
  private sinceTurn = Number.POSITIVE_INFINITY;
  crossing: Crossing | null = null;
  /** Câu đang nói (speech bubble), `null` là không nói gì. */
  speech: Speech | null = null;
  /** Số giây kể từ lần dừng lại chào con khác. */
  sinceMeet = Number.POSITIVE_INFINITY;
  /** Vừa chào con này: đứng chào xong thì rủ nó đi cùng một đoạn. */
  buddy: Pet | null = null;
  /**
   * Đang chat (khung chat mở cạnh con này, ở phía này): đứng yên quay về phía khung chat, không ngủ, không
   * đi theo con khác. `null` là không chat.
   */
  listening: Facing | null = null;
  /**
   * Còn biến mất chừng này giây (bị Long quật bay, `vanish`): đứng nguyên như lúc biến mất, không làm gì,
   * con khác không thấy; hết giờ thì hiện lại đúng chỗ cũ và làm tiếp việc đang dở. 0 là đang hiện.
   */
  vanished = 0;
  private readonly brain = new StateMachine<PetState, Pet>(STATES, "idle");

  constructor(
    options: PetOptions,
    readonly env: PetEnv,
  ) {
    this.id = options.id;
    this.x = options.x;
    this.y = options.y;
    this.width = options.width;
    this.height = options.height;
    this.reach = options.reach ?? options.width * TUNING.reach;
    // State đầu tiên không chạy `enter`, nên tự chọn thời lượng đứng.
    plan(this, TUNING.idleTime);
  }

  get state(): PetState {
    return this.brain.state;
  }

  /** Số giây đã ở state hiện tại, renderer dùng để chọn frame. */
  get stateTime(): number {
    return this.brain.time;
  }

  /** Frame bắt buộc của animation hiện tại (nhảy: lấy đà, bay, tiếp đất); `undefined` là chạy theo thời gian. */
  get pose(): number | undefined {
    const jumping = this.jumping;
    if (this.state !== "jump" || !jumping) return undefined;
    if (jumping.phase === "crouch") return 0;
    if (jumping.phase === "air") return 1;
    return this.stateTime - jumping.landedAt < TUNING.jumpTouchdown / 2 ? 2 : 3;
  }

  /** Hệ số tốc độ đi, chạy, leo: Settings, ban đêm chậm hơn. */
  get pace(): number {
    return this.env.speed * (this.env.night ? TUNING.nightPace : 1);
  }

  /** Nói `text` trong `seconds` giây (speech bubble). Không qua giới hạn tần suất: gọi `env.chat` cho câu nói cho vui. */
  say(text: string, seconds: number = TUNING.speechTime): void {
    this.speech = { text, left: seconds };
  }

  /** Dừng lại đứng yên `seconds` giây, quay về phía `facing` (gặp con khác). */
  pause(facing: Facing, seconds: number): void {
    this.goal = null;
    this.facing = facing;
    this.brain.go(this, "idle");
    this.planned = seconds;
  }

  /** Đi về phía `dir` trong `seconds` giây, cùng một con khác (`Goal` stroll). */
  stroll(dir: Facing, seconds: number): void {
    this.facing = dir;
    this.setOff("walk", { kind: "stroll" });
    this.planned = seconds;
  }

  /**
   * Đứng yên `seconds` giây quay về phía `facing`, nếu đang rảnh trên mặt đất hoặc mép cửa sổ (đang ngủ,
   * đang choáng thì dậy luôn). Đang bị kéo, rơi, leo, nhảy, chat, sang màn hình khác hay đang biến mất thì
   * thôi, trả về `false`: ép đứng yên lúc đó thì pet đứng khựng giữa không trung.
   */
  hold(facing: Facing, seconds: number): boolean {
    if (!HOLDABLE.has(this.state) || this.crossing || this.listening !== null || this.vanished > 0) return false;
    this.pause(facing, seconds);
    return true;
  }

  /**
   * Tốc biến tới đứng cạnh `other` (đang đứng trên mặt đất hoặc mép cửa sổ): cùng chỗ đứng với nó, điểm
   * chân ở `x` (kẹp trong chỗ đứng), quay mặt về phía nó rồi đứng yên `seconds` giây. Đang rơi, leo, nhảy
   * cũng được, đứng hẳn lên chỗ mới chứ không lơ lửng. Không gọi lúc đang bị kéo.
   */
  blinkTo(other: Pet, x: number, seconds: number): void {
    this.pause(this.facing, seconds);
    this.vx = 0;
    this.vy = 0;
    this.crossing = null;
    this.skipLedge = null;
    const mount = other.grounded ? other.mount : null;
    this.mount = mount ? { id: mount.id, dx: 0, dy: mount.dy } : null;
    const ground = groundOf(this);
    // Cửa sổ vừa đóng thì đứng xuống mặt đất.
    if (!ground.ledge) this.mount = null;
    // Mép hẹp hơn chỗ đứng: đứng giữa mép, bước sau sẽ rơi (`mountHolds`) như mọi lúc khác.
    this.x = ground.to < ground.from ? (ground.from + ground.to) / 2 : clamp(x, ground.from, ground.to);
    this.y = ground.y;
    syncMount(this);
    if (Math.abs(other.x - this.x) > 1) this.facing = other.x > this.x ? 1 : -1;
  }

  /** Biến mất `seconds` giây (`vanished`); câu đang nói để dành tới lúc hiện lại. */
  vanish(seconds: number): void {
    this.vanished = Math.max(this.vanished, seconds);
  }

  /** Đang đứng trên mặt đất hoặc mép cửa sổ: không bị kéo, không đang rơi, leo hay bay. */
  get grounded(): boolean {
    const { state } = this;
    if (state === "dragged" || state === "fall" || state === "climb") return false;
    return !(state === "jump" && this.jumping?.phase === "air");
  }

  /**
   * Đang ra khỏi mép giáp màn hình khác, giữa thân đã qua mép: điểm (CSS pixel của overlay, nằm trên màn
   * hình kia) mà overlay cần sang màn hình chứa nó. `null` là không đi đâu.
   */
  get leaving(): Point | null {
    const crossing = this.crossing;
    if (!crossing || crossing.phase !== "out") return null;
    const { left, right } = this.env.bounds;
    if (crossing.dir > 0 ? this.x <= right : this.x >= left) return null;
    return { x: this.x, y: this.state === "fall" ? this.y - this.height / 2 : this.y - 1 };
  }

  /**
   * Người dùng click vào pet: phản ứng, đang ngủ thì thức dậy, đang leo thì tuột tay. Đang nhảy hoặc
   * click dồn dập thì chỉ nhảy một lần: cú nhảy không bị bắt đầu lại, phải ngừng click một lúc mới nhảy tiếp.
   */
  poke(): void {
    // Bấm vào con đang nói thì tắt bubble (cả lời nhắc nghỉ, nhắc khuya).
    this.speech = null;
    if (this.state === "climb") {
      this.sincePoke = 0;
      this.sinceInteraction = 0;
      this.letGo();
      return;
    }
    if (!this.grounded) return;
    const spam = this.sincePoke < TUNING.pokeCooldown;
    this.sincePoke = 0;
    this.sinceInteraction = 0;
    if (spam || this.state === "react") return;
    this.hops = 0;
    this.brain.go(this, "react");
  }

  /**
   * Cửa sổ `id` vừa di chuyển từ `from` tới `to`. Giữ nguyên cỡ (đang bị kéo, không phải phóng to hay
   * đổi cỡ) mà lại gần sát pet thì chạy trốn; hết đường chạy dưới đất thì giật mình nhảy lên.
   */
  windowMoved(id: number, from: Rect, to: Rect): void {
    // Đang vắt qua mép sang màn hình khác: đi nốt, không quay đầu giữa chừng.
    if (this.mount?.id === id || !CALM.has(this.state) || this.crossing || this.vanished > 0) return;
    if (Math.abs(from.width - to.width) >= 1 || Math.abs(from.height - to.height) >= 1) return;
    const after = gap(this, to);
    if (after >= TUNING.fleeRange || after >= gap(this, from)) return;
    this.runFrom(this.x < to.x + to.width / 2 ? -1 : 1);
  }

  /** Cửa sổ `id` (khung cuối cùng `rect`) vừa bị đóng: ở gần thì quay về phía đó nhảy cẫng lên ăn mừng. */
  windowClosed(id: number, rect: Rect): void {
    if (this.mount?.id === id || !CALM.has(this.state) || this.vanished > 0) return;
    if (this.sinceCheer < TUNING.cheerCooldown || gap(this, rect) > TUNING.cheerRange) return;
    if (!this.env.rng.chance(TUNING.cheerChance)) return;
    this.sinceCheer = 0;
    const center = rect.x + rect.width / 2;
    if (Math.abs(center - this.x) > 1) this.facing = center > this.x ? 1 : -1;
    this.hops = TUNING.cheerHops - 1;
    this.brain.go(this, "react");
  }

  /**
   * Giật mình (sấm, sự kiện hiếm): đang rảnh đứng trên mặt đất hoặc mép cửa sổ thì nhảy dựng lên một cái.
   * Không tính là người dùng đụng vào, nên không làm cả nhóm tỉnh ngủ lâu hơn.
   */
  startle(): void {
    if (!CALM.has(this.state) || !this.grounded || this.crossing || this.vanished > 0) return;
    this.hops = 0;
    this.brain.go(this, "react");
  }

  /**
   * Người dùng mở khung chat với con này, khung chat ở phía `side`: đang đi thì dừng lại, đang ngủ thì dậy,
   * rồi đứng yên quay về phía đó tới khi `stopListening`. Đang leo, nhảy, rơi thì xong rồi mới đứng yên.
   */
  listen(side: Facing): void {
    this.listening = side;
    this.sinceInteraction = 0;
    this.wake();
    if (this.state === "walk" || this.state === "run" || this.state === "idle") this.pause(side, TUNING.idleTime[0]);
  }

  /** Khung chat đã đóng hoặc đổi sang con khác: lại đi lại như thường. */
  stopListening(): void {
    this.listening = null;
  }

  /** Người dùng click ở chỗ khác trên màn hình: đang ngủ thì giật mình thức dậy như bị click. */
  wake(): void {
    if (this.state === "sleep") this.poke();
  }

  /** Người dùng bắt đầu kéo pet, bắt được cả khi pet đang rơi, leo hay nhảy. */
  grab(): void {
    this.sinceInteraction = 0;
    this.vx = 0;
    this.vy = 0;
    this.mount = null;
    this.skipLedge = null;
    this.crossing = null;
    this.brain.go(this, "dragged");
  }

  /** Đặt điểm chân của pet theo chuột trong lúc kéo, luôn giữ trong vùng cho phép. */
  dragTo(x: number, y: number): void {
    if (this.state !== "dragged") return;
    const { left, right, top, floor } = this.env.bounds;
    const half = this.width / 2;
    this.x = clamp(x, left + half, right - half);
    this.y = clamp(y, top + this.height, floor);
  }

  /** Thả pet với vận tốc chuột lúc buông (px/s): rơi xuống, ném mạnh thì bay theo quán tính. */
  release(vx: number, vy: number): void {
    if (this.state !== "dragged") return;
    const speed = Math.hypot(vx, vy);
    const k = speed > TUNING.maxThrowSpeed ? TUNING.maxThrowSpeed / speed : 1;
    this.vx = vx * k;
    this.vy = vy * k;
    this.brain.go(this, "fall");
  }

  step(dt: number): void {
    this.sinceInteraction += dt;
    this.sincePoke += dt;
    this.sinceCheer += dt;
    this.sinceTurn += dt;
    this.sinceMeet += dt;
    // Đang biến mất: đứng nguyên như lúc biến mất (câu đang nói để dành), hết giờ thì làm tiếp.
    if (this.vanished > 0) {
      this.vanished = Math.max(0, this.vanished - dt);
      return;
    }
    if (this.speech && (this.speech.left -= dt) <= 0) this.speech = null;
    if (this.crossing) {
      // Chỉ tính giờ chờ overlay sang từ lúc giữa thân qua mép: đi bộ từ đầu mặt đất ra tới đó mất hơn một giây.
      if (this.leaving) this.crossing.elapsed += dt;
      // Bị click, bị kéo giữa chừng: thôi sang màn hình kia.
      if (this.state !== "walk" && this.state !== "run" && this.state !== "fall") this.stayInside();
    }
    // Cửa sổ đang đứng vừa đóng, thu nhỏ, bị kéo tới chỗ không đứng được: rơi.
    if (this.mount && !this.mountHolds()) this.dropOff();
    // Bị che hết: rơi ra trước mọi cửa sổ (đang leo thì buông tay), không đáp lại mép đó. Trước đó pet
    // đi ra đầu mép rồi leo xuống, khuất sau cửa sổ cả chục giây như đã biến mất.
    else if (this.mount && buried(this)) {
      this.skipLedge = this.mount.id;
      if (this.state === "climb") this.letGo();
      else this.dropOff();
    }
    this.watchCursor();
    this.brain.update(this, dt);
  }

  /**
   * Overlay vừa sang màn hình khác theo một con khác (bị kéo hoặc ném sang) mà con này còn ở màn hình cũ:
   * xuống đất ngay ngoài mép `side` của màn hình mới (-1: mép trái), cách mép thêm `behind` px, rồi chạy
   * vào. Đang ngủ thì nằm luôn ở sát mép đó.
   */
  joinFrom(side: Facing, behind: number): void {
    const { left, right, floor } = this.env.bounds;
    const half = this.width / 2;
    this.mount = null;
    this.goal = null;
    this.skipLedge = null;
    this.vx = 0;
    this.vy = 0;
    this.y = floor;
    this.facing = side < 0 ? 1 : -1;
    if (this.state === "sleep") {
      this.crossing = null;
      this.x = side < 0 ? left + half : right - half;
      return;
    }
    this.x = side < 0 ? left - half - behind : right + half + behind;
    this.brain.go(this, "run", () => {
      this.crossing = { dir: this.facing, phase: "in", elapsed: 0 };
    });
  }

  /** Thôi sang màn hình bên cạnh, về hẳn trong màn hình này. */
  stayInside(): void {
    const { left, right } = this.env.bounds;
    this.crossing = null;
    this.x = clamp(this.x, left + this.width / 2, right - this.width / 2);
  }

  /**
   * Overlay vừa đổi chỗ hoặc đổi cỡ (sang màn hình khác, đổi DPI, taskbar): toạ độ cũ `p` thành
   * `p * scale + (x, y)`. Gọi `rebound` sau khi đã có vùng màn hình mới.
   */
  remap({ scale, x, y }: Remap): void {
    this.x = this.x * scale + x;
    this.y = this.y * scale + y;
    this.vx *= scale;
    this.vy *= scale;
    if (this.mount) {
      this.mount.dx *= scale;
      this.mount.dy *= scale;
    }
    // Đường leo, đường nhảy tính theo toạ độ cũ: buông ra rơi cho chắc (hiếm khi xảy ra đúng lúc này).
    if (this.state === "climb" || this.state === "jump") this.dropOff();
    // Overlay đã sang màn hình pet đang đi tới. Bị giữ ngay ngoài mép lúc đang bay thì bay tiếp vào.
    const crossing = this.crossing;
    if (crossing?.phase === "out") {
      if (this.state === "fall" && crossing.vx !== undefined) this.vx = crossing.vx * scale;
      this.crossing = { dir: crossing.dir, phase: "in", elapsed: 0 };
    }
  }

  /** Vùng màn hình vừa đổi: giữ pet trong vùng mới, đang ở dưới đất thì đứng lên (hoặc rơi xuống) mặt đất mới. */
  rebound(): void {
    const { left, right, top, floor } = this.env.bounds;
    const half = this.width / 2;
    if (this.mount) {
      this.follow();
      return;
    }
    const { state } = this;
    if (state === "fall") return;
    if (!this.crossing) this.x = clamp(this.x, left + half, right - half);
    if (state === "dragged") {
      this.y = clamp(this.y, top + this.height, floor);
      return;
    }
    // Đang nhảy lên vì bị click: mỗi bước tự tính độ cao theo mặt đất mới.
    if (state === "react") return;
    // Đang ngủ thì nằm yên trên mặt đất mới, không bị đánh thức.
    if (this.y < floor - 1 && state !== "sleep") {
      this.vx = this.crossing ? this.facing * TUNING.walkSpeed * this.pace : 0;
      this.vy = 0;
      this.brain.go(this, "fall");
      return;
    }
    this.y = floor;
  }

  /** Địa hình vừa đổi: pet đang đứng hoặc leo trên cửa sổ nào thì đi theo cửa sổ đó. */
  follow(): void {
    const mount = this.mount;
    const rect = mount && this.env.terrain.window(mount.id);
    if (!mount || !rect) return;
    this.x = rect.x + mount.dx;
    this.y = rect.y + mount.dy;
    if (this.state === "climb") return;
    // Cửa sổ bị kéo ra ngoài màn hình: pet lùi vào trong chứ không ra theo.
    const { left, right } = this.env.bounds;
    const x = clamp(this.x, left + this.width / 2, right - this.width / 2);
    if (x !== this.x) {
      this.x = x;
      mount.dx = x - rect.x;
    }
  }

  /** Đổi cỡ pet (Settings), kẹp lại để cả con vẫn nằm trong màn hình. */
  resize(width: number, height: number, reach = width * TUNING.reach): void {
    const { left, right, top, floor } = this.env.bounds;
    this.width = width;
    this.height = height;
    this.reach = reach;
    // Đang đứng trên cửa sổ thì vẫn đứng đúng mép; to quá không còn chỗ thì bước sau sẽ rơi.
    if (this.mount) {
      this.follow();
      return;
    }
    this.x = clamp(this.x, left + width / 2, right - width / 2);
    this.y = clamp(this.y, top + height, floor);
  }

  snapshot(): PetSnapshot {
    return {
      id: this.id,
      x: Math.round(this.x),
      facing: this.facing,
      asleep: this.state === "sleep",
      sinceInteraction: Math.floor(this.sinceInteraction),
    };
  }

  /** Đặt lại theo trạng thái đã lưu: đứng trên mặt đất ở chỗ cũ, đang ngủ thì ngủ tiếp. */
  restore(saved: PetSnapshot): void {
    const { left, right, floor } = this.env.bounds;
    this.x = clamp(saved.x, left + this.width / 2, right - this.width / 2);
    this.y = floor;
    this.vx = 0;
    this.vy = 0;
    this.mount = null;
    this.facing = saved.facing;
    this.sinceInteraction = saved.sinceInteraction;
    this.brain.go(this, saved.asleep ? "sleep" : "idle");
  }

  private mountHolds(): boolean {
    const mount = this.mount;
    const { terrain } = this.env;
    const rect = mount && terrain.window(mount.id);
    if (!mount || !rect) return false;
    if (this.state === "climb") {
      const side = this.climbing?.side;
      if (!side || !terrain.wall(mount.id, side)) return false;
      return this.y >= rect.y - 1 && this.y <= rect.y + rect.height + this.height;
    }
    const ledge = terrain.ledge(mount.id);
    return !!ledge && roomy(this, ledge) && this.x >= ledge.from - 1 && this.x <= ledge.to + 1;
  }

  private dropOff(): void {
    this.mount = null;
    this.goal = null;
    this.vx = 0;
    this.vy = 0;
    this.brain.go(this, "fall");
  }

  /** Chạy về phía `away`; dưới đất mà sát mép màn hình, không còn đường thì giật mình nhảy lên. */
  private runFrom(away: Facing): void {
    if (this.state === "run" && this.goal?.kind === "flee") {
      this.facing = away;
      return;
    }
    const ground = groundOf(this);
    const room = away > 0 ? ground.to - this.x : this.x - ground.from;
    if (!ground.ledge && room < this.width) {
      this.hops = 0;
      this.brain.go(this, "react");
      return;
    }
    this.facing = away;
    this.setOff("run", { kind: "flee" });
  }

  /** Đi/chạy để làm `goal`. `exit` của walk/run xoá việc đang làm, nên đặt việc mới sau đó. */
  private setOff(state: "walk" | "run", goal: Goal): void {
    this.brain.go(this, state, () => {
      this.goal = goal;
    });
  }

  /** Con trỏ ở gần thì pet quay về phía con trỏ. */
  private watchCursor(): void {
    const cursor = this.env.cursor;
    if (!cursor || this.state !== "idle" || !this.grounded || this.crossing) return;
    const dx = cursor.x - this.x;
    if (
      Math.abs(dx) >= TUNING.lookDeadZone &&
      Math.hypot(dx, cursor.y - (this.y - this.height / 2)) <= TUNING.lookRange &&
      this.sinceTurn >= TUNING.lookTurn
    ) {
      const side: Facing = dx > 0 ? 1 : -1;
      if (side !== this.facing) {
        this.facing = side;
        this.sinceTurn = 0;
      }
    }
  }

  /** Đang leo thì buông tay, bật nhẹ ra khỏi tường. */
  private letGo(): void {
    const side = this.climbing?.side ?? -this.facing;
    this.mount = null;
    this.vx = side * 60;
    this.vy = 0;
    this.brain.go(this, "fall");
  }
}

function plan(pet: Pet, [min, max]: readonly [number, number]): void {
  pet.planned = pet.env.rng.range(min, max);
}

/** Việc tiếp theo sau lượt đứng yên; `undefined` là đã tự chuyển state (rủ con khác đi cùng). */
function nextActivity(pet: Pet): PetState | undefined {
  if (pet.listening !== null) {
    // Đang chat: đứng yên nhìn về phía khung chat, không tính là bỏ mặc nên không buồn ngủ.
    pet.facing = pet.listening;
    pet.sinceInteraction = 0;
    return "idle";
  }
  const ground = groundOf(pet);
  // Chỉ ngủ trên taskbar: đang ở trên cửa sổ thì xuống trước. Có con đang ngủ gần đó thì ra nằm cạnh.
  const sleepAfter = pet.env.night ? TUNING.sleepAfterNight : TUNING.sleepAfter;
  if (pet.sinceInteraction >= sleepAfter) return ground.ledge ? goDown(pet, ground) : nap(pet);
  // Bị che hết thì `Pet.step` đã cho rơi: ở đây còn chỗ để ra.
  if (hidden(pet)) {
    const x = nearestOpenX(pet, ground);
    if (x !== null) return walkTo(pet, x);
  }
  const { rng } = pet.env;
  // Vừa chào con khác xong, nó vẫn đứng cạnh: rủ đi cùng một đoạn.
  const buddy = pet.buddy;
  pet.buddy = null;
  if (buddy && buddy.state === "idle" && neighbors(pet, buddy, TUNING.meetReach * 1.5)) {
    const dir: Facing = rng.chance(0.5) ? 1 : -1;
    const time = rng.range(...TUNING.strollTime);
    buddy.stroll(dir, time);
    pet.stroll(dir, time);
    return undefined;
  }
  // Con khác đang đi lại thì con này hay đứng yên hơn: cả nhóm ít khi cùng chạy nhảy một lúc, đỡ rối mắt
  // và vòng lặp vẽ được nghỉ nhiều hơn. Chỉ đổi xác suất, không bốc thêm số ngẫu nhiên, nên một con thì
  // sống y như trước.
  const movers = pet.env.pets.filter(
    (other) => other !== pet && other.vanished === 0 && MOVING.has(other.state),
  ).length;
  const keep = Math.max(0, 1 - TUNING.groupCalm * movers);
  // Chỉ bốc số ngẫu nhiên khi có chỗ để nhảy / leo, để không có cửa sổ thì pet sống y như Phase 1.
  const jumps = jumpTargets(pet);
  if (jumps.length > 0 && rng.chance(TUNING.jumpChance * keep)) return startJump(pet, rng.pick(jumps));
  const walls = climbTargets(pet, ground);
  if (walls.length > 0 && rng.chance(TUNING.climbChance * keep)) return approach(pet, rng.pick(walls));
  // Mép hẹp hơn chỗ đứng thì không đi được.
  if (ground.to - ground.from < 1) return "idle";
  const roll = rng.next();
  if (roll < 0.45 * keep) return "walk";
  if (roll < 0.6 * keep) return "run";
  return "idle";
}

function startMoving(pet: Pet, time: readonly [number, number]): void {
  if (pet.goal?.kind === "flee") {
    plan(pet, TUNING.fleeTime);
    return;
  }
  if (pet.goal) {
    pet.planned = TUNING.goalTime;
    return;
  }
  plan(pet, time);
  // Đang đi vào từ mép màn hình mới (bị ném sang, chạm đất ngoài mép) thì không quay đầu.
  if (!pet.crossing && pet.env.rng.chance(TUNING.turnChance)) pet.facing = pet.facing === 1 ? -1 : 1;
}

function move(pet: Pet, speed: number, time: number, dt: number): PetState | undefined {
  const ground = groundOf(pet);
  if (ground.to - ground.from < 1) return "idle";
  pet.x += pet.facing * speed * pet.pace * dt;
  if (pet.crossing) return cross(pet, pet.crossing, time);
  const goal = pet.goal;
  // Đi tới sát con khác đang đứng hoặc đi trên cùng mặt đất: dừng lại chào nhau.
  if (!goal && pet.sinceMeet >= TUNING.meetCooldown) {
    const friend = pet.env.pets.find((other) => other !== pet && meetable(pet, other));
    if (friend) {
      meet(pet, friend);
      return undefined;
    }
  }
  if (goal && (goal.kind === "climb" || goal.kind === "peek" || goal.kind === "nap")) {
    const target = goalX(pet, goal, ground);
    if (target === null) {
      pet.goal = null;
    } else if ((target - pet.x) * pet.facing <= 0) {
      pet.x = target;
      syncMount(pet);
      return arrive(pet, goal);
    }
  }
  if (pet.x >= ground.to) {
    pet.x = ground.to;
    syncMount(pet);
    return atEnd(pet, ground, 1);
  }
  if (pet.x <= ground.from) {
    pet.x = ground.from;
    syncMount(pet);
    return atEnd(pet, ground, -1);
  }
  syncMount(pet);
  return time >= pet.planned ? "idle" : undefined;
}

/** Điểm chân cần đi tới cho `goal`, `null` nếu không còn tới được. */
function goalX(pet: Pet, goal: Goal, ground: Ground): number | null {
  if (goal.kind === "climb") {
    const wall = pet.env.terrain.wall(goal.id, goal.side);
    if (!wall) return null;
    const x = wall.x + goal.side * pet.reach;
    return x >= ground.from && x <= ground.to ? x : null;
  }
  if (goal.kind === "peek") {
    const rect = pet.mount && pet.env.terrain.window(pet.mount.id);
    return rect ? clamp(rect.x + goal.dx, ground.from, ground.to) : null;
  }
  if (goal.kind === "nap") return ground.ledge ? null : clamp(goal.x, ground.from, ground.to);
  return null;
}

function arrive(pet: Pet, goal: Goal): PetState {
  if (goal.kind === "climb") {
    const wall = pet.env.terrain.wall(goal.id, goal.side);
    if (wall) return climbOrGrab(pet, wall);
  }
  if (goal.kind === "nap") return "sleep";
  return "idle";
}

/** Hai con đứng trên cùng mặt đất (taskbar hoặc cùng mép cửa sổ), cách nhau dưới `reach` lần nửa bề ngang cộng lại. */
function neighbors(pet: Pet, other: Pet, reach: number): boolean {
  if (!pet.grounded || !other.grounded || (pet.mount?.id ?? null) !== (other.mount?.id ?? null)) return false;
  if (Math.abs(pet.y - other.y) > 2) return false;
  return Math.abs(other.x - pet.x) < ((pet.width + other.width) / 2) * reach;
}

/** `pet` đang đi vừa tới sát `other` ở phía trước, cả hai đều rảnh: dừng lại chào được. */
function meetable(pet: Pet, other: Pet): boolean {
  if (other.sinceMeet < TUNING.meetCooldown || other.crossing || other.goal || other.listening !== null) return false;
  // Con đang biến mất thì không thấy để mà chào.
  if (other.vanished > 0) return false;
  if (other.state !== "idle" && other.state !== "walk" && other.state !== "run") return false;
  return (other.x - pet.x) * pet.facing > 0 && neighbors(pet, other, TUNING.meetReach);
}

/** Hai con gặp nhau: đứng lại quay mặt vào nhau, có khi chào một câu, có khi rủ nhau đi cùng một đoạn. */
function meet(pet: Pet, other: Pet): void {
  const { rng } = pet.env;
  const time = rng.range(...TUNING.meetTime);
  const facing: Facing = other.x > pet.x ? 1 : -1;
  pet.sinceMeet = 0;
  other.sinceMeet = 0;
  pet.pause(facing, time);
  // Con kia đứng lâu hơn một chút để còn kịp được rủ đi cùng (`nextActivity` của con này).
  other.pause(facing === 1 ? -1 : 1, time + 1);
  if (rng.chance(TUNING.meetChatChance)) pet.env.chat(pet, rng.pick(GREETINGS));
  if (rng.chance(TUNING.strollChance)) pet.buddy = other;
}

/**
 * Buồn ngủ dưới đất: có con đang ngủ dưới đất trong `napRange` thì đi tới nằm cạnh nó (bên gần hơn, còn
 * chỗ trong màn hình, không đè lên con ngủ khác), không thì ngủ luôn tại chỗ.
 */
function nap(pet: Pet): PetState {
  const { left, right } = pet.env.bounds;
  const half = pet.width / 2;
  const sleepers = pet.env.pets.filter((other) => other !== pet && other.state === "sleep" && !other.mount);
  let best: number | null = null;
  for (const other of sleepers) {
    const gap = ((pet.width + other.width) / 2) * TUNING.napGap;
    for (const x of [other.x - gap, other.x + gap]) {
      if (x < left + half || x > right - half || Math.abs(x - pet.x) > TUNING.napRange) continue;
      if (sleepers.some((s) => Math.abs(s.x - x) < gap * 0.9)) continue;
      if (best === null || Math.abs(x - pet.x) < Math.abs(best - pet.x)) best = x;
    }
  }
  if (best === null || Math.abs(best - pet.x) < 2) return "sleep";
  pet.facing = best > pet.x ? 1 : -1;
  pet.goal = { kind: "nap", x: best };
  return "walk";
}

/**
 * Đang sang màn hình bên cạnh. Ra hẳn khỏi mép thì chờ overlay sang bên kia (`Pet.leaving`), lâu quá thì
 * quay lại; đã sang thì đi vào tới khi cả con nằm trong màn hình mới.
 */
function cross(pet: Pet, crossing: Crossing, time: number): PetState | undefined {
  const { left, right } = pet.env.bounds;
  const half = pet.width / 2;
  // Không dừng lại giữa chừng lúc đang nằm vắt qua mép.
  pet.planned = Math.max(pet.planned, time + 1);
  if (crossing.phase === "in") {
    // Vào hẳn rồi thì thôi; lỡ quay đầu ra lại thì vào luôn trong màn hình, không đi lạc ra ngoài. Màn hình
    // đổi tại chỗ (taskbar, DPI) lúc đang đi ra thì cũng thành "in" mà pet đang ở mép bên kia: vào luôn.
    if (!entering(pet, crossing) || pet.facing !== crossing.dir) pet.stayInside();
    return undefined;
  }
  if (pet.facing !== crossing.dir) {
    pet.crossing = { dir: pet.facing, phase: "in", elapsed: 0 };
    return undefined;
  }
  const out = crossing.dir > 0 ? right + half : left - half;
  if ((pet.x - out) * crossing.dir > 0) pet.x = out;
  if (crossing.elapsed >= TUNING.crossTimeout) {
    pet.facing = crossing.dir > 0 ? -1 : 1;
    pet.crossing = { dir: pet.facing, phase: "in", elapsed: 0 };
  }
  return undefined;
}

/** Đã sang màn hình mới mà còn nằm ngoài (hoặc vắt qua) mép phía đi vào. */
function entering(pet: Pet, crossing: Crossing): boolean {
  const { left, right } = pet.env.bounds;
  const half = pet.width / 2;
  return crossing.dir > 0 ? pet.x < left + half : pet.x > right - half;
}

/**
 * Đi tới đầu mép. Mặt đất, hoặc cửa sổ chạm cạnh màn hình: quay đầu như chạm tường. Mép cửa sổ:
 * quay lại, ngồi mép hoặc xuống.
 */
function atEnd(pet: Pet, ground: Ground, dir: Facing): PetState | undefined {
  const { ledge } = ground;
  const { bounds } = pet.env;
  const half = pet.width / 2;
  const screenEdge = dir > 0 ? ground.to >= bounds.right - half : ground.from <= bounds.left + half;
  const goal = pet.goal?.kind;
  if (ledge && (goal === "down" || goal === "flee")) return stepOff(pet, ledge, dir);
  // Chạy trốn mà bị dồn vào mép màn hình: đứng lại, không quay đầu chạy về phía cửa sổ.
  if (goal === "flee") return "idle";
  // Mép màn hình giáp màn hình khác: có khi đi sang bên đó (chỉ khi có một con, xem `PetEnv.wander`).
  if (
    !ledge &&
    !pet.goal &&
    pet.env.wander &&
    pet.env.exit(dir, pet.y - 1) &&
    pet.env.rng.chance(TUNING.crossChance)
  ) {
    pet.crossing = { dir, phase: "out", elapsed: 0 };
    return undefined;
  }
  if (!ledge || screenEdge || pet.goal) {
    pet.goal = null;
    pet.facing = dir === 1 ? -1 : 1;
    return undefined;
  }
  const roll = pet.env.rng.next();
  if (roll < TUNING.edgeTurn) {
    pet.facing = dir === 1 ? -1 : 1;
    return undefined;
  }
  if (roll < TUNING.edgeTurn + TUNING.edgePerch) return "perch";
  return stepOff(pet, ledge, dir);
}

function leavePerch(pet: Pet): PetState {
  const { ledge } = groundOf(pet);
  if (!ledge) return "idle";
  if (pet.env.rng.chance(TUNING.perchLeaveTurn)) {
    pet.facing = pet.facing === 1 ? -1 : 1;
    return "walk";
  }
  return stepOff(pet, ledge, pet.facing);
}

/**
 * Xuống khỏi mép cửa sổ về phía `dir`: nhảy xuống chỗ thấp hơn nếu với tới, không thì leo xuống (cạnh
 * không bị che), không nữa thì nhảy khỏi mép.
 */
function stepOff(pet: Pet, ledge: Ledge, dir: Facing): PetState {
  pet.goal = null;
  pet.facing = dir;
  const below = jumpTargets(pet).filter((t) => t.y > ledge.y + 8 && (t.x - pet.x) * dir > 0);
  if (below.length > 0) {
    const nearest = below.reduce((a, b) => (Math.abs(b.x - pet.x) < Math.abs(a.x - pet.x) ? b : a));
    return startJump(pet, nearest);
  }
  const wall = pet.env.terrain.wall(ledge.id, dir);
  const { bounds } = pet.env;
  if (wall && openWall(wall)) {
    const x = wall.x + dir * pet.reach;
    if (x >= bounds.left + pet.width / 2 && x <= bounds.right - pet.width / 2) {
      return startClimb(pet, wall, 1);
    }
  }
  pet.skipLedge = ledge.id;
  pet.mount = null;
  pet.vx = dir * TUNING.stepOffSpeed;
  pet.vy = -TUNING.stepOffHop;
  return "fall";
}

/** Buồn ngủ trên cửa sổ: nhảy thẳng xuống taskbar nếu không quá cao, không thì đi ra đầu mép gần hơn. */
function goDown(pet: Pet, ground: Ground): PetState {
  const floor = jumpTargets(pet).find((t) => t.id === null);
  if (floor) return startJump(pet, floor);
  pet.facing = pet.x - ground.from <= ground.to - pet.x ? -1 : 1;
  pet.goal = { kind: "down" };
  return "walk";
}

function walkTo(pet: Pet, x: number): PetState {
  const rect = pet.mount && pet.env.terrain.window(pet.mount.id);
  if (!rect || Math.abs(x - pet.x) < 1) return "idle";
  pet.facing = x > pet.x ? 1 : -1;
  pet.goal = { kind: "peek", dx: x - rect.x };
  return "walk";
}

function approach(pet: Pet, wall: Wall): PetState {
  const x = wall.x + wall.side * pet.reach;
  if (Math.abs(x - pet.x) < 1) return climbOrGrab(pet, wall);
  pet.facing = x > pet.x ? 1 : -1;
  pet.goal = { kind: "climb", id: wall.id, side: wall.side };
  return "walk";
}

/** Đứng ở chân tường: cửa sổ chạm tới thì leo luôn, cửa sổ lơ lửng thì lấy đà nhảy lên bám. */
function climbOrGrab(pet: Pet, wall: Wall): PetState {
  const rect = pet.env.terrain.window(wall.id);
  if (!rect) return "idle";
  if (rect.y + rect.height + pet.height * TUNING.grabDepth >= pet.y - 1) return startClimb(pet, wall, -1);
  pet.goal = null;
  pet.facing = wall.side === 1 ? -1 : 1;
  pet.x = wall.x + wall.side * pet.reach;
  pet.jumping = {
    id: wall.id,
    grab: wall.side,
    offset: 0,
    phase: "crouch",
    launch: { at: 0, x: 0, y: 0, vx: 0, vy: 0 },
    landedAt: 0,
  };
  return "jump";
}

/** Bám vào cạnh `wall`, mặt quay vào tường, rồi leo lên (`dir` -1) hoặc xuống (1). */
function startClimb(pet: Pet, wall: Wall, dir: -1 | 1): PetState {
  const rect = pet.env.terrain.window(wall.id);
  if (!rect) return "idle";
  pet.goal = null;
  pet.facing = wall.side === 1 ? -1 : 1;
  pet.x = wall.x + wall.side * pet.reach;
  if (dir > 0) pet.y = Math.max(pet.y, rect.y + 1);
  pet.climbing = { side: wall.side, dir };
  pet.mount = { id: wall.id, dx: pet.x - rect.x, dy: pet.y - rect.y };
  return "climb";
}

function climb(pet: Pet, dt: number): PetState | undefined {
  const { climbing, mount } = pet;
  const { terrain, bounds } = pet.env;
  const rect = mount && terrain.window(mount.id);
  if (!climbing || !mount || !rect) return "fall";
  pet.y += climbing.dir * TUNING.climbSpeed * pet.pace * dt;
  if (climbing.dir < 0) {
    if (pet.y > rect.y) {
      mount.dy = pet.y - rect.y;
      return undefined;
    }
    // Lên tới đỉnh: nhún một cái qua mép, đáp lên cửa sổ, mặt quay vào trong. Đặt thẳng lên mép thì
    // pet giật ngang cả chục pixel (từ chỗ bám tường tới chỗ đứng).
    const ledge = terrain.ledge(mount.id);
    if (!ledge || !roomy(pet, ledge)) {
      pet.mount = null;
      return "fall";
    }
    const margin = pet.width * TUNING.ledgeMargin;
    const half = pet.width / 2;
    const x = clamp(
      climbing.side < 0 ? ledge.from + margin : ledge.to - margin,
      bounds.left + half,
      bounds.right - half,
    );
    pet.y = ledge.y;
    pet.jumping = {
      id: mount.id,
      grab: null,
      offset: x - rect.x,
      phase: "air",
      launch: { at: 0, x: 0, y: 0, vx: 0, vy: 0 },
      landedAt: 0,
    };
    if (!launch(pet, pet.jumping, 0, TUNING.mantleArc)) return "fall";
    return "jump";
  }
  if (pet.y >= bounds.floor) {
    pet.y = bounds.floor;
    pet.mount = null;
    return "land";
  }
  // Hết cạnh cửa sổ mà chưa tới đất: buông tay.
  if (pet.y >= rect.y + rect.height) {
    pet.mount = null;
    pet.vx = climbing.side * 30;
    pet.vy = 0;
    return "fall";
  }
  mount.dy = pet.y - rect.y;
  return undefined;
}

function startJump(pet: Pet, target: JumpTarget): PetState {
  const rect = target.id === null ? undefined : pet.env.terrain.window(target.id);
  pet.goal = null;
  pet.jumping = {
    id: target.id,
    grab: null,
    offset: rect ? target.x - rect.x : target.x,
    phase: "crouch",
    launch: { at: 0, x: 0, y: 0, vx: 0, vy: 0 },
    landedAt: 0,
  };
  if (Math.abs(target.x - pet.x) > 1) pet.facing = target.x > pet.x ? 1 : -1;
  return "jump";
}

function jump(pet: Pet, time: number): PetState | undefined {
  const jumping = pet.jumping;
  if (!jumping) return "idle";
  if (jumping.phase === "crouch") {
    if (time < TUNING.jumpCrouch) return undefined;
    const launched = jumping.grab === null ? launch(pet, jumping, time) : leap(pet, jumping, time);
    if (!launched) return "idle";
    jumping.phase = "air";
    return undefined;
  }
  if (jumping.phase === "touchdown") {
    return time - jumping.landedAt >= TUNING.jumpTouchdown ? "idle" : undefined;
  }
  const { left, right, floor } = pet.env.bounds;
  const half = pet.width / 2;
  const { launch: from } = jumping;
  const t = time - from.at;
  const prevX = pet.x;
  const prevY = pet.y;
  pet.x = clamp(from.x + from.vx * t, left + half, right - half);
  pet.y = from.y + from.vy * t + (TUNING.gravity * t * t) / 2;
  pet.vx = from.vx;
  pet.vy = from.vy + TUNING.gravity * t;
  if (pet.vy <= 0) return undefined;
  if (jumping.grab !== null && jumping.id !== null) {
    // Lên tới đỉnh cú nhảy: bám vào cạnh rồi leo. Cửa sổ biến mất giữa chừng thì rơi.
    const wall = pet.env.terrain.wall(jumping.id, jumping.grab);
    if (wall) return startClimb(pet, wall, -1);
    pet.vx = 0;
    return "fall";
  }
  const landing = landingLedge(pet, prevX, prevY);
  const ground = landing ? landing.ledge.y : floor;
  if (pet.y < ground) return undefined;
  if (landing) pet.x = landing.x;
  pet.y = ground;
  const impact = pet.vy;
  settle(pet, landing?.ledge ?? null);
  // Cửa sổ định đáp biến mất giữa chừng, rơi từ quá cao: choáng.
  if (impact >= TUNING.dizzySpeed) return "dizzy";
  jumping.phase = "touchdown";
  jumping.landedAt = time;
  return undefined;
}

/** Nhảy thẳng lên, đỉnh cú nhảy đúng chỗ bám cạnh cửa sổ lơ lửng. */
function leap(pet: Pet, jumping: Jumping, time: number): boolean {
  const rect = jumping.id === null ? undefined : pet.env.terrain.window(jumping.id);
  if (!rect) return false;
  const y = rect.y + rect.height + pet.height * TUNING.grabDepth;
  // Cửa sổ vừa bị kéo xuống, không cần nhảy nữa: lần sau leo thẳng.
  if (y >= pet.y) return false;
  const up = Math.sqrt(2 * TUNING.gravity * (pet.y - y));
  pet.vx = 0;
  pet.vy = -up;
  jumping.launch = { at: time, x: pet.x, y: pet.y, vx: 0, vy: -up };
  pet.skipLedge = pet.mount?.id ?? null;
  pet.mount = null;
  return true;
}

/**
 * Tính vận tốc lúc bật nhảy để rơi đúng chỗ đáp (cửa sổ có thể đã di chuyển từ lúc chọn). `arc`: đỉnh
 * cao hơn điểm cao hơn giữa chỗ đứng và chỗ đáp, mặc định theo khoảng cách ngang.
 */
function launch(pet: Pet, jumping: Jumping, time: number, arc?: number): boolean {
  const { terrain, bounds } = pet.env;
  let x = jumping.offset;
  let y = bounds.floor;
  if (jumping.id !== null) {
    const rect = terrain.window(jumping.id);
    const ledge = terrain.ledge(jumping.id);
    if (!rect || !ledge) return false;
    x += rect.x;
    y = ledge.y;
  }
  const dx = x - pet.x;
  const highest = Math.min(pet.y, y);
  // Không nhảy cao quá trần, nhưng đỉnh luôn cao hơn cả chỗ đứng lẫn chỗ đáp.
  const peak = highest - (arc ?? TUNING.jumpArc + TUNING.jumpArcPerPx * Math.abs(dx));
  const apex = Math.min(highest, Math.max(peak, bounds.top + pet.height * TUNING.headroom));
  const g = TUNING.gravity;
  const up = Math.sqrt(2 * g * (pet.y - apex));
  const flight = up / g + Math.sqrt((2 * (y - apex)) / g);
  pet.vx = flight > 0 ? dx / flight : 0;
  pet.vy = -up;
  jumping.launch = { at: time, x: pet.x, y: pet.y, vx: pet.vx, vy: pet.vy };
  if (Math.abs(dx) > 1) pet.facing = dx > 0 ? 1 : -1;
  // Không đáp lại chỗ vừa rời, trừ khi nhảy lên chính cửa sổ đang leo.
  const here = pet.mount?.id ?? null;
  pet.skipLedge = here === jumping.id ? null : here;
  pet.mount = null;
  return true;
}

function hop(pet: Pet, time: number): PetState | undefined {
  const { y } = groundOf(pet);
  const t = Math.min(time / TUNING.reactTime, 1);
  pet.y = y - TUNING.hopHeight * Math.sin(Math.PI * t);
  if (time < TUNING.reactTime) return undefined;
  pet.y = y;
  if (pet.hops > 0) {
    pet.hops--;
    return "react";
  }
  // Nhảy xong thì đi hoặc chạy tiếp luôn: đứng lại ngay sau khi bị click trông như bị đơ.
  return pet.env.rng.chance(TUNING.runAfterPoke) ? "run" : "walk";
}

function fall(pet: Pet, dt: number): PetState | undefined {
  const { left, right, top, floor } = pet.env.bounds;
  const half = pet.width / 2;
  const prevX = pet.x;
  const prevY = pet.y;
  pet.vy += TUNING.gravity * dt;
  pet.x += pet.vx * dt;
  pet.y += pet.vy * dt;

  if (pet.x < left + half) {
    if (!throughEdge(pet, -1)) {
      pet.x = left + half;
      pet.vx = Math.abs(pet.vx) * TUNING.wallBounce;
    }
  } else if (pet.x > right - half) {
    if (!throughEdge(pet, 1)) {
      pet.x = right - half;
      pet.vx = -Math.abs(pet.vx) * TUNING.wallBounce;
    }
  } else if (pet.crossing?.phase === "in") {
    pet.crossing = null;
  }
  if (pet.y - pet.height < top) {
    pet.y = top + pet.height;
    pet.vy = Math.abs(pet.vy) * TUNING.wallBounce;
  }
  if (Math.abs(pet.vx) > 20) pet.facing = pet.vx > 0 ? 1 : -1;
  // Chỉ đáp lên mép cửa sổ khi đang rơi xuống qua nó, không đáp khi đang bay lên.
  const landing = pet.vy > 0 ? landingLedge(pet, prevX, prevY) : null;
  const ledge = landing?.ledge ?? null;
  const ground = ledge ? ledge.y : floor;
  if (pet.y < ground) return undefined;

  // Bay qua mép sang màn hình khác, giữa thân đã qua mép mà chạm đất: đứng lại chờ overlay sang bên kia,
  // không kéo về. Chưa qua tới mép thì overlay không sang: đáp luôn trong màn hình này.
  const crossing = pet.crossing;
  if (crossing && pet.leaving) {
    pet.y = ground;
    pet.vx = 0;
    pet.vy = 0;
    // Đã đứng trên đất: overlay sang thì đi bộ vào, không bay tiếp.
    crossing.vx = undefined;
    return undefined;
  }
  // Đã sang màn hình mới mà chạm đất lúc còn ở ngoài mép: đi bộ vào, không hiện ra đột ngột trong màn hình.
  if (crossing?.phase === "in" && !landing && entering(pet, crossing)) {
    pet.y = ground;
    pet.vx = 0;
    pet.vy = 0;
    pet.facing = crossing.dir;
    return "walk";
  }
  // Chạm đất hoặc mép cửa sổ. Đang bay vào từ mép sang màn hình mới thì đáp luôn trong màn hình này.
  if (landing) pet.x = landing.x;
  if (pet.crossing) pet.stayInside();
  pet.y = ground;
  const impact = pet.vy;
  if (impact >= TUNING.dizzySpeed) return settle(pet, ledge, "dizzy");
  if (impact >= TUNING.bounceSpeed) {
    pet.vy = -impact * TUNING.restitution;
    pet.vx *= TUNING.groundFriction;
    return undefined;
  }
  return settle(pet, ledge, "land");
}

/**
 * Bị ném vào mép `side` của màn hình: mép đó giáp màn hình khác thì bay qua luôn, không nảy lại. Bay ra
 * hẳn mà overlay chưa sang bên kia thì dừng ngay ngoài mép, rơi thẳng xuống chờ, lâu quá thì nảy lại vào trong.
 * Overlay sang lúc pet còn đang rơi thì bay tiếp vào với vận tốc lúc bị giữ lại (`Crossing.vx`).
 */
function throughEdge(pet: Pet, side: Facing): boolean {
  let crossing = pet.crossing;
  // Vừa sang màn hình mới, đang bay vào từ mép bên kia.
  if (crossing?.phase === "in") return crossing.dir === -side;
  if (!crossing) {
    if (pet.vx * side <= 0 || !pet.env.exit(side, pet.y - pet.height / 2)) return false;
    crossing = pet.crossing = { dir: side, phase: "out", elapsed: 0 };
  } else if (crossing.dir !== side || crossing.elapsed >= TUNING.crossTimeout) {
    pet.stayInside();
    return false;
  }
  const { left, right } = pet.env.bounds;
  const out = side > 0 ? right + pet.width / 2 : left - pet.width / 2;
  if ((pet.x - out) * side > 0) {
    pet.x = out;
    crossing.vx ??= pet.vx;
    pet.vx = 0;
    pet.vy = 0;
  }
  return true;
}

/** Dừng hẳn trên `ledge` (hoặc mặt đất nếu `null`). */
function settle(pet: Pet, ledge: Ledge | null): void;
function settle(pet: Pet, ledge: Ledge | null, next: PetState): PetState;
function settle(pet: Pet, ledge: Ledge | null, next?: PetState): PetState | void {
  pet.vx = 0;
  pet.vy = 0;
  pet.skipLedge = null;
  const rect = ledge && pet.env.terrain.window(ledge.id);
  pet.mount = ledge && rect ? { id: ledge.id, dx: pet.x - rect.x, dy: 0 } : null;
  return next;
}

/** Khoảng cách từ thân pet (khung một frame, đứng trên điểm chân) tới hình chữ nhật `rect`. */
function gap(pet: Pet, rect: Rect): number {
  const half = pet.width / 2;
  const dx = Math.max(0, rect.x - (pet.x + half), pet.x - half - (rect.x + rect.width));
  const dy = Math.max(0, rect.y - pet.y, pet.y - pet.height - (rect.y + rect.height));
  return Math.hypot(dx, dy);
}

/** Pet vừa đi trên mép cửa sổ: ghi lại chỗ đứng so với cửa sổ để lần sau cửa sổ di chuyển thì đi theo. */
function syncMount(pet: Pet): void {
  const mount = pet.mount;
  const rect = mount && pet.env.terrain.window(mount.id);
  if (mount && rect) mount.dx = pet.x - rect.x;
}
