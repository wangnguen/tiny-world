import "./overlay.css";
import {
  errorMessage,
  type CursorInfo,
  type Point,
  type Rect,
  type ScreenInfo,
  type Settings,
  type WindowList,
} from "@tinyworld/core";
import {
  CHAT_OFF_LINE,
  FixedStep,
  StepBlend,
  TUNING,
  World,
  clamp,
  parseWorldSnapshot,
  type Bounds,
  type Facing,
  type PetSnapshot,
} from "@tinyworld/sim";
import { api } from "./api";
import { Ambience, type Resident } from "./overlay/ambience";
import { AutoSave } from "./overlay/autosave";
import { ClickThrough } from "./overlay/clickThrough";
import { PetInteraction } from "./overlay/interaction";
import { LongKit } from "./overlay/longAction";
import { LONG_PACK, LongCombo } from "./overlay/longCombo";
import { PetView } from "./overlay/petView";
import { listPacks, loadSpriteSet, resolvePacks } from "./overlay/sprites";
import type { SpriteSet } from "./overlay/spriteSet";
import { WeatherEffect } from "./overlay/weather";

/**
 * Tần số mô phỏng, cũng là tần số vẽ lúc pet đi đứng bình thường: sprite thường chỉ 8–12 fps nên 30 là đủ
 * mượt mà vẫn nhẹ. Lúc bay hay bị kéo (`FAST_STATES`) thì vẽ theo tần số màn hình.
 */
const FPS = 30;
/**
 * Có con đang bay (bị ném, rơi, nhảy) hoặc đang bị kéo: vẽ lại mỗi lần làm tươi màn hình, ở vị trí nội suy
 * giữa hai bước mô phỏng (`StepBlend`). Vẽ 30 fps thì pet bị ném nhanh nhảy cóc từng đoạn dài, nhìn giật.
 * Chỉ kéo dài vài giây nên không tốn thêm CPU đáng kể.
 */
const FAST_STATES: ReadonlySet<string> = new Set(["fall", "jump", "dragged"]);
/**
 * Vẽ bằng hẹn giờ rồi mới xin `requestAnimationFrame`, chứ không xin rAF liên tục: rAF chạy theo tần số
 * màn hình (60–144 Hz), WebView phải thức dậy gấp mấy lần cần. Hẹn sớm chừng này ms để chờ tới lần làm
 * tươi màn hình kế tiếp thì vừa đúng nhịp.
 */
const VSYNC_SLACK_MS = 8;
const FRAME_MS = 1000 / FPS - VSYNC_SLACK_MS;
/**
 * Cả nhóm đứng yên (đứng, ngồi mép, ngủ trong lúc con khác còn thức) thì chỉ cần thức dậy lúc có con đổi
 * frame (animation đứng yên chỉ 3–6 fps), nhưng tối đa chừng này ms một lần để mô phỏng không dồn quá
 * `MAX_STEPS` bước một lượt.
 */
const CALM_MAX_MS = 250;
/** Số bước mô phỏng tối đa mỗi lượt vẽ: 10 bước là 333 ms, dư cho lần chờ lâu nhất lúc pet đứng yên. */
const MAX_STEPS = 10;
const CALM_STATES: ReadonlySet<string> = new Set(["idle", "perch", "sleep"]);
/**
 * Rust chỉ gửi vị trí con trỏ khi nó ở gần một con: con thức thì trong tầm nhìn theo con trỏ, con ngủ thì
 * chỉ quanh thân để bắt click. Báo rộng thêm `INTEREST_SLACK` để pet đi một đoạn mới phải báo lại.
 */
const INTEREST_AWAKE = TUNING.lookRange;
const INTEREST_ASLEEP = 16;
const INTEREST_SLACK = 64;
/** Khoảng cách từ mép phải vùng làm việc tới con đầu tiên lúc xuất hiện lần đầu (CSS pixel). */
const SPAWN_MARGIN = 48;
/** Các con xuất hiện lần đầu đứng cách nhau chừng này lần bề ngang pet. */
const SPAWN_GAP = 1.4;
/** Chu kỳ lưu world.json (chỉ ghi khi có thay đổi). */
const SAVE_INTERVAL_MS = 30_000;
/** Vừa có phím hay chuột trong chừng này ms là người dùng đang ngồi máy (con ma lúc 2 giờ sáng). */
const PRESENT_MS = 60_000;
/** Hai lần xin Rust cho overlay sang màn hình khác cách nhau ít nhất chừng này (ms). */
const MOVE_INTERVAL_MS = 200;
/** Id của pet tạm vẽ bằng code, khi bản build chưa có sprite pack nào. */
const PLACEHOLDER_ID = "placeholder";
/** Tên nhân vật theo pack (`name` trong pet.json, ví dụ "Momo — Axolotl"), cho tiêu đề cửa sổ chat. */
const PACK_NAMES = new Map(listPacks().map((pack) => [pack.id, pack.name]));
/** Bản cũ chỉ có một pet, lưu trong world.json với id này: lấy làm chỗ của con đầu tiên. */
const LEGACY_ID = "pet-1";

/**
 * Pet sống trong vùng làm việc; mặt đất là mép dưới, tức là mép trên taskbar. Taskbar tự ẩn đang trồi
 * lên (`taskbarTop`) thì mặt đất là mép trên của nó.
 */
function boundsOf(screen: ScreenInfo, taskbarTop: number | null = null): Bounds {
  const { x, y, width, height } = screen.workArea;
  const floor = taskbarTop === null ? y + height : Math.min(y + height, taskbarTop);
  return { left: x, right: x + width, top: y, floor };
}

function inflate(r: Rect, by: number): Rect {
  return { x: r.x - by, y: r.y - by, width: r.width + 2 * by, height: r.height + 2 * by };
}

function containsRect(outer: Rect, inner: Rect): boolean {
  return (
    inner.x >= outer.x &&
    inner.y >= outer.y &&
    inner.x + inner.width <= outer.x + outer.width &&
    inner.y + inner.height <= outer.y + outer.height
  );
}

/** File hỏng hay đọc lỗi thì bắt đầu lại từ đầu, lần lưu sau sẽ ghi đè. */
async function loadSaved(): Promise<unknown> {
  try {
    return await api.loadState();
  } catch (error) {
    console.warn("Không đọc được trạng thái đã lưu, bắt đầu lại:", errorMessage(error));
    return null;
  }
}

/** Một con trên màn hình: sprite pack (`null`: pet tạm), phần mô phỏng, phần vẽ và thời tiết quanh nó. */
interface Member extends Resident {
  pack: string | null;
  /** Đồ diễn riêng của Long (aura, cinematic, vệt tốc biến); con khác không có. */
  kit: LongKit | null;
}

/** Chỗ của một con vừa bị bỏ (đổi nhân vật trong Settings): con mới đứng đúng chỗ đó. */
interface Spot {
  x: number;
  facing: Facing;
}

async function start(): Promise<void> {
  const container = document.getElementById("world");
  if (!container) throw new Error("Thiếu phần tử #world.");
  const [screen, settings, saved] = await Promise.all([
    api.screenInfo(),
    api.getSettings(),
    loadSaved(),
  ]);

  const world = new World(boundsOf(screen), Date.now());
  world.setScreen(boundsOf(screen), screen.neighbors);
  world.speed = settings.speed;
  /** Cỡ nhân vật trong Settings. */
  let size = settings.size;
  /** Các con đang hiện, theo thứ tự xuất hiện. */
  const members: Member[] = [];
  /** Trạng thái đã lưu, chỉ dùng cho các con hiện ra lúc mở app (`syncPets` lần đầu). */
  const restoring = new Map((parseWorldSnapshot(saved)?.pets ?? []).map((p) => [p.id, p]));

  let paused = false;
  /** Màn hình overlay đang phủ, và mép trên taskbar tự ẩn đang trồi lên (`WindowList.taskbarTop`). */
  let current = screen;
  let taskbarTop: number | null = null;
  // Khai báo trước khi đăng ký event: event có thể tới ngay khi vừa đăng ký xong.
  const step = new FixedStep(1 / FPS, MAX_STEPS);
  const blend = new StepBlend();
  let last = 0;
  let running = false;
  let hidden = false;
  /** Combo quật đuôi Long đang diễn (Shift + click vào Long), `null` nếu không có. */
  let combo: LongCombo | null = null;

  /** Con nằm trên cùng có phần hình dưới điểm `point` (CSS pixel của overlay). */
  const pick = (point: Point): Member | null => {
    let top: Member | null = null;
    for (const member of members) {
      if (member.view.spriteHidden) continue;
      if ((!top || member.view.depth > top.view.depth) && member.view.hitTest(point)) top = member;
    }
    return top;
  };

  // Overlay để chuột đi xuyên nên không tự nhận được sự kiện chuột: Rust gửi vị trí con trỏ sang,
  // con trỏ nằm trên phần có hình của một con (và không giữ Ctrl) thì overlay nhận chuột.
  const clickThrough = new ClickThrough(api.setClickThrough);
  let cursor: CursorInfo | null = null;
  const refreshClickThrough = () =>
    clickThrough.update(!paused && cursor !== null && !cursor.passThrough && pick(cursor) !== null);
  await api.onCursorMoved((info) => {
    // Vừa bấm chuột ở bất kỳ đâu (kể cả ngoài pet): các con đang ngủ đều thức dậy.
    if (info.pressed && !cursor?.pressed && !paused && world.pets.some((pet) => pet.state === "sleep")) {
      world.wakeAll();
      wake();
    }
    cursor = info;
    // Con đứng yên thì quay đầu nhìn theo con trỏ. Con đang ngủ không để ý con trỏ.
    if (running) world.moveCursor(info.x, info.y);
    refreshClickThrough();
  });
  /** Chat với pet đang bật trong Cài đặt (Phase 6). */
  let chatOn = settings.chat;
  /** Con đang chat: đứng yên cạnh khung chat tới khi đóng khung chat. */
  let listening: Member | null = null;
  const stopListening = () => {
    listening?.pet.stopListening();
    listening = null;
  };
  // Click chuột phải vào pet: mở cửa sổ chat cạnh con đó; chưa bật chat thì con đó nhắc bật trong Cài đặt.
  window.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    const member = pick({ x: event.clientX, y: event.clientY });
    const box = member?.view.bounds;
    if (!member || !box || paused) return;
    if (!chatOn) {
      member.pet.say(CHAT_OFF_LINE);
      wake();
      return;
    }
    const name = (PACK_NAMES.get(member.pack ?? "") ?? "Pet").split(" — ")[0];
    api
      .openChat(member.pet.id, name, box)
      .then((side) => {
        if (listening !== member) stopListening();
        listening = member;
        member.pet.listen(side === 1 ? 1 : -1);
        wake();
      })
      .catch((error: unknown) => console.warn("Không mở được cửa sổ chat:", errorMessage(error)));
  });

  /** Vùng đã báo Rust cho từng con (`api.setCursorInterest`), lúc báo con đó có đang ngủ không. */
  let interest = new Map<Member, { rect: Rect; asleep: boolean }>();
  const refreshInterest = () => {
    const needed: { member: Member; rect: Rect; asleep: boolean }[] = [];
    let stale = false;
    for (const member of members) {
      const box = member.view.bounds;
      if (!box) continue;
      const asleep = member.pet.state === "sleep";
      const rect = inflate(box, asleep ? INTEREST_ASLEEP : INTEREST_AWAKE);
      needed.push({ member, rect, asleep });
      const known = interest.get(member);
      if (!known || known.asleep !== asleep || !containsRect(known.rect, rect)) stale = true;
    }
    if (!stale && needed.length === interest.size) return;
    interest = new Map(needed.map(({ member, rect, asleep }) => [member, { rect: inflate(rect, INTEREST_SLACK), asleep }]));
    api
      .setCursorInterest([...interest.values()].map(({ rect }) => rect))
      .catch((error: unknown) => console.warn("Không báo được vùng quanh pet:", errorMessage(error)));
  };

  /** Hẹn giờ đang chờ để vẽ lần sau (0: không có), và lần đó có phải là chờ lâu lúc cả nhóm đứng yên không. */
  let timer = 0;
  let calmWait = false;
  /** Đã báo Rust cả nhóm đang ngủ (`api.setResting`). */
  let resting = false;
  const setResting = (value: boolean) => {
    if (value === resting) return;
    resting = value;
    api.setResting(value).catch((error: unknown) => console.warn("Không báo được pet ngủ/thức:", errorMessage(error)));
  };
  const schedule = (delay: number) => {
    calmWait = delay > FRAME_MS;
    timer = window.setTimeout(() => {
      timer = 0;
      requestAnimationFrame(frame);
    }, delay);
  };
  /** Cả nhóm đứng yên thì chờ tới lúc có con đổi frame; có con đi lại thì vẽ 30 lần/giây. */
  const nextDelay = () => {
    // Long đang diễn combo (cinematic, vệt tốc biến): vẽ đủ nhịp.
    if (combo) return FRAME_MS;
    // Đang mưa, tuyết quanh pet hay con ma đang bay thì vẽ đủ nhịp của hiệu ứng.
    let wait = ambience.frameMs / 1000;
    for (const { pet, view } of members) {
      // Con đang biến mất đứng nguyên một chỗ, không cần vẽ; chờ lâu nhất `CALM_MAX_MS` nên hiện lại kịp.
      if (pet.vanished > 0) continue;
      if (!CALM_STATES.has(pet.state)) return FRAME_MS;
      wait = Math.min(wait, view.nextFrameIn(pet));
    }
    return clamp(wait * 1000 - VSYNC_SLACK_MS, FRAME_MS, CALM_MAX_MS);
  };
  const frame = (now: number) => {
    if (hidden || paused) {
      running = false;
      return;
    }
    const elapsed = (now - last) / 1000;
    last = now;
    const { pets } = world;
    for (let n = step.advance(elapsed); n > 0; n--) blend.step(pets, () => world.step(step.dt));
    ambience.act(now);
    // Có con đang đi hoặc bay ra khỏi mép giáp màn hình khác: overlay sang bên đó, cả nhóm theo sang.
    const leaving = pets.find((pet) => pet.leaving)?.leaving;
    if (leaving) moveOverlay(leaving);
    combo?.update(now);
    if (combo?.done) combo = null;
    redraw();
    // Cả nhóm ngủ thì dừng hẳn vòng lặp (trừ lúc con ma còn đang bay); click hoặc kéo sẽ chạy lại.
    if (world.resting && !ambience.flying) {
      running = false;
      setResting(true);
      return;
    }
    if (pets.some((pet) => FAST_STATES.has(pet.state))) requestAnimationFrame(frame);
    else schedule(nextDelay());
  };
  /** Vẽ lại cả nhóm; con đổi frame, bị cửa sổ che, hoặc đi dưới con trỏ đứng yên thì cũng tính lại click-through. */
  const redraw = () => {
    const now = performance.now();
    for (const { pet, view, kit } of members) {
      view.update(pet, world.occluders(pet), blend.at(pet, step.alpha));
      kit?.update(view, pet, now);
    }
    ambience.place(now);
    refreshClickThrough();
    refreshInterest();
  };
  /** Chạy lại vòng lặp; đang chờ lâu lúc cả nhóm đứng yên mà có chuyện (click, cửa sổ đổi) thì vẽ ngay lần sau. */
  const wake = () => {
    if (hidden || paused) return;
    if (running) {
      if (timer !== 0 && calmWait) {
        window.clearTimeout(timer);
        schedule(0);
      }
      return;
    }
    running = true;
    setResting(false);
    last = performance.now();
    requestAnimationFrame(frame);
  };

  let moving = false;
  let lastMove = Number.NEGATIVE_INFINITY;
  /** Điểm `point` (CSS pixel của overlay) nằm trên màn hình khác thì Rust cho overlay sang đó (`onScreenChanged`). */
  const moveOverlay = (point: Point) => {
    const now = performance.now();
    if (moving || now - lastMove < MOVE_INTERVAL_MS) return;
    moving = true;
    lastMove = now;
    api
      .moveOverlay(point.x, point.y)
      .catch((error: unknown) => console.warn("Không đưa được overlay sang màn hình khác:", errorMessage(error)))
      .finally(() => {
        moving = false;
      });
  };

  const interaction = new PetInteraction(pick, {
    onHold: (held) => clickThrough.hold(held),
    onActivity: wake,
    // Kéo một con sang màn hình khác.
    onDragOutside: moveOverlay,
    // Bấm đúp: nói giờ, ngày, âm lịch, thời tiết.
    onDoubleClick: (pet) => {
      ambience.tellTime(pet);
      wake();
    },
    // Shift + click vào Long: aura; có con khác thì diễn combo quật đuôi. Con khác chưa có kỹ năng riêng.
    onSkill: (pet) => {
      const long = members.find((member) => member.pet === pet);
      const kit = long?.kit;
      if (!long || !kit) return false;
      const now = performance.now();
      // Tính là có đụng tới như click: cả nhóm không lăn ra ngủ ngay sau màn diễn.
      pet.sinceInteraction = 0;
      kit.aura.activate(now);
      const targets = members.filter((member) => member !== long && member.pet.vanished === 0);
      if (!combo && targets.length > 0) combo = new LongCombo(long, kit, targets, now);
      wake();
      return true;
    },
  });

  const ambience = new Ambience(
    {
      world,
      residents: () => members,
      live: () => !hidden && !paused,
      wake,
      present: () =>
        api
          .idleMs()
          .then((idle) => idle !== null && idle < PRESENT_MS)
          .catch(() => false),
    },
    container,
    settings,
  );

  /** Chỗ đứng cho con mới: từ góc phải dọc theo mặt đất, chỗ đầu tiên không đứng sát con nào. */
  const spawnX = (width: number): number => {
    const { left, right } = world.bounds;
    for (let x = right - SPAWN_MARGIN - width / 2; x >= left + width / 2; x -= width * SPAWN_GAP) {
      if (members.every(({ pet }) => Math.abs(pet.x - x) >= width)) return x;
    }
    return right - SPAWN_MARGIN - width / 2;
  };

  const addMember = (pack: string | null, sprite: SpriteSet, spot?: Spot) => {
    const view = new PetView(sprite, container, size);
    const effect = new WeatherEffect();
    view.attach(effect.element);
    const kit = pack === LONG_PACK ? new LongKit(view) : null;
    const pet = world.spawn({
      id: pack ?? PLACEHOLDER_ID,
      x: spot?.x ?? spawnX(view.width),
      width: view.width,
      height: view.height,
      reach: view.reach,
    });
    if (spot) pet.facing = spot.facing;
    // Mở app: con nào đã lưu thì về chỗ cũ (đang ngủ thì ngủ tiếp). Bản cũ chỉ có một pet: chỗ của nó là
    // chỗ của con đầu tiên.
    const previous: PetSnapshot | undefined =
      restoring.get(pet.id) ?? (members.length === 0 ? restoring.get(LEGACY_ID) : undefined);
    if (previous) pet.restore(previous);
    const member = { pack, pet, view, effect, kit, aura: kit?.aura ?? null };
    members.push(member);
    ambience.adopt(member);
  };

  const removeMember = (member: Member): Spot => {
    if (interaction.held === member.pet) interaction.cancel();
    // Bỏ Long giữa combo thì dừng combo; bỏ con đang bị nhắm thì Long sang con khác.
    combo?.drop(member);
    member.kit?.dispose();
    member.view.destroy();
    world.remove(member.pet.id);
    members.splice(members.indexOf(member), 1);
    return { x: member.pet.x, facing: member.pet.facing };
  };

  /** Sprite đã nạp của các pack đang dùng; bớt nhân vật thì bỏ để đỡ tốn RAM. */
  const sprites = new Map<string | null, Promise<SpriteSet>>();
  const spriteOf = (pack: string | null) => {
    let sprite = sprites.get(pack);
    if (!sprite) {
      sprite = loadSpriteSet(pack);
      sprites.set(pack, sprite);
    }
    return sprite;
  };
  /** Danh sách pack của lần đổi gần nhất; lần đổi cũ nạp xong sau lần mới thì bỏ. */
  let wanted: readonly (string | null)[] = [];
  /**
   * Hiện đúng các con trong `packs` (`resolvePacks`). Nạp xong pack mới rồi mới bỏ con cũ, để lúc đổi
   * nhân vật không có lúc trống; con mới đứng đúng chỗ con vừa bị bỏ.
   */
  const syncPets = async (packs: readonly (string | null)[]) => {
    wanted = packs;
    const missing = packs.filter((pack) => !members.some((m) => m.pack === pack));
    const loaded = await Promise.all(missing.map(spriteOf));
    if (wanted !== packs) return;
    const spots = members.filter((m) => !packs.includes(m.pack)).map(removeMember);
    missing.forEach((pack, i) => addMember(pack, loaded[i], spots.shift()));
    for (const pack of sprites.keys()) if (!packs.includes(pack)) sprites.delete(pack);
    redraw();
    wake();
  };
  await syncPets(resolvePacks(settings.pets));
  restoring.clear();

  const autosave = new AutoSave(world, api.saveState);
  autosave.markSaved();
  autosave.start(SAVE_INTERVAL_MS);

  // Overlay ẩn (tray, app fullscreen) thì dừng hẳn, pet đứng nguyên chỗ cũ; hiện lại thì chạy tiếp.
  await api.onVisibilityChanged((visible) => {
    hidden = !visible;
    if (hidden) interaction.cancel();
    if (visible) wake();
  });
  // Overlay sang màn hình khác (pet bị kéo, ném, tự đi sang), hoặc màn hình đổi độ phân giải, DPI, taskbar:
  // đổi toạ độ pet và cửa sổ sang overlay mới. Danh sách cửa sổ theo toạ độ mới tới ngay sau đó.
  await api.onScreenChanged(({ screen, remap }) => {
    // Mép trên taskbar tự ẩn tính theo overlay cũ: bỏ, danh sách cửa sổ tới ngay sau đó sẽ báo lại.
    current = screen;
    taskbarTop = null;
    world.setScreen(boundsOf(screen), screen.neighbors, remap);
    interaction.remap(remap);
    for (const { view } of members) view.resize();
    redraw();
    if (!paused) wake();
  });
  // Tạm dừng: pet đứng yên và chuột đi xuyên qua pet, bỏ tạm dừng thì sống tiếp.
  await api.onPaused((value) => {
    paused = value;
    if (paused) interaction.cancel();
    refreshClickThrough();
    if (!paused) wake();
  });
  // Cửa sổ mở, đóng, di chuyển: dựng lại địa hình. Đăng ký trước rồi mới lấy danh sách hiện tại, để
  // không lỡ thay đổi nào ở giữa; đã nhận event thì danh sách lấy về (cũ hơn) bỏ đi.
  let windowsSeen = false;
  const applyWindows = (list: WindowList) => {
    world.setWindows(list.windows, list.closed);
    // Taskbar tự ẩn trồi lên hoặc thụt xuống: pet đứng lên mép trên của nó, hoặc rơi xuống lại đáy màn hình.
    const top = list.taskbarTop ?? null;
    const floorMoved = top !== taskbarTop;
    if (floorMoved) {
      taskbarTop = top;
      world.setScreen(boundsOf(current, taskbarTop), current.neighbors);
    }
    // Cả nhóm ngủ trên taskbar không bị cửa sổ ảnh hưởng: không chạy lại vòng lặp, mặt đất đổi thì chỉ vẽ lại.
    if (world.resting) {
      if (floorMoved) redraw();
      return;
    }
    // Tạm dừng thì không có vòng lặp, pet đứng trên cửa sổ vẫn phải đi theo.
    if (paused) redraw();
    else wake();
  };
  await api.onWindowsChanged((list) => {
    windowsSeen = true;
    applyWindows(list);
  });
  const windows = await api.listWindows().catch((error: unknown) => {
    console.warn("Không lấy được danh sách cửa sổ:", errorMessage(error));
    return null;
  });
  if (windows && !windowsSeen) applyWindows(windows);

  const applySettings = async (next: Settings) => {
    world.speed = next.speed;
    chatOn = next.chat;
    ambience.setSettings(next);
    if (next.size !== size) {
      size = next.size;
      for (const { pet, view } of members) {
        view.setSize(size);
        pet.resize(view.width, view.height, view.reach);
      }
    }
    redraw();
    await syncPets(resolvePacks(next.pets));
  };
  await api.onSettingsChanged(applySettings);
  // Thời tiết của thành phố đã chọn: đăng ký trước rồi mới lấy kết quả đã có, như danh sách cửa sổ.
  let weatherSeen = false;
  await api.onWeatherChanged((report) => {
    weatherSeen = true;
    ambience.setReport(report);
  });
  await api.onWeatherFailed((failure) => ambience.fail(failure));
  const report = await api.getWeather().catch((error: unknown) => {
    console.warn("Không lấy được thời tiết đã lưu:", errorMessage(error));
    return null;
  });
  if (!weatherSeen) ambience.setReport(report);
  // Phase 5: đang gõ phím thì không nói câu cho vui; nhắc nghỉ, nhắc khuya, spam Ctrl+S.
  await api.onActivityChanged((busy) => {
    world.busy = busy;
  });
  await api.onReminder((reminder) => {
    if (!hidden && !paused) ambience.remind(reminder);
  });
  // Phase 6: đóng khung chat (hay tắt chat trong Cài đặt) thì con đang chat lại đi lại.
  await api.onChatClosed(() => {
    stopListening();
    wake();
  });
  await api.onQuitRequested(async () => {
    await autosave.flush();
    await api.quit();
  });

  redraw();
  wake();

  // WebView đổi DPI (chuyển màn hình/Windows scaling) thì vẽ lại cả con đang ngủ.
  window.addEventListener("resize", () => {
    for (const { view } of members) view.resize();
    redraw();
  });

  // Chỉ khi chạy dev: xem và chỉnh pet từ DevTools (tray → Mở DevTools), ví dụ
  // `__tinyworld.pet.sinceInteraction = 1e6` để con đầu tiên đi ngủ ngay. Bản build không có dòng này.
  if (import.meta.env.DEV) {
    Object.assign(window, {
      __tinyworld: {
        world,
        members,
        ambience,
        wake,
        autosave,
        get pet() {
          return members[0]?.pet;
        },
        get view() {
          return members[0]?.view;
        },
      },
    });
  }
}

start().catch((error) => console.error("Không khởi động được overlay:", errorMessage(error)));
