import type { AnimationName } from "@tinyworld/core";
import { StateMachine, type StateTable } from "./fsm";
import { clamp } from "./math";
import type { Rng } from "./rng";
import type { PetSnapshot } from "./snapshot";

/** State của pet, trùng tên với animation. */
export type PetState = AnimationName;

/** 1: nhìn sang phải, -1: nhìn sang trái. */
export type Facing = 1 | -1;

/** Vùng pet được ở, theo CSS pixel của overlay: tường trái, tường phải, trần, mặt đất. */
export interface Bounds {
  left: number;
  right: number;
  top: number;
  floor: number;
}

/** Môi trường pet sống, do `World` cung cấp. */
export interface PetEnv {
  readonly bounds: Bounds;
  readonly rng: Rng;
  /** Hệ số tốc độ đi/chạy (Settings), 1 là bình thường. */
  readonly speed: number;
}

/** Thông số hành vi. Đơn vị: CSS pixel và giây. */
export const TUNING = {
  walkSpeed: 30,
  runSpeed: 110,
  /** Thời lượng ngẫu nhiên [min, max] của mỗi lượt đứng / đi / chạy. */
  idleTime: [2, 5],
  walkTime: [2, 6],
  runTime: [1, 2.5],
  /** Xác suất đổi hướng khi bắt đầu đi hoặc chạy. */
  turnChance: 0.4,
  gravity: 2000,
  /** Chạm đất nhanh hơn mức này thì nảy lên; mỗi lần nảy giữ lại `restitution` vận tốc. */
  bounceSpeed: 350,
  restitution: 0.4,
  /** Vận tốc ngang còn lại sau mỗi lần nảy. */
  groundFriction: 0.6,
  /** Chạm đất nhanh hơn mức này thì choáng, tương đương thả từ độ cao khoảng 500 px. */
  dizzySpeed: 1400,
  /** Va vào tường hoặc trần thì bật lại, giữ lại phần này vận tốc. */
  wallBounce: 0.5,
  maxThrowSpeed: 3000,
  reactTime: 0.5,
  /** Click cách lần click trước ít hơn khoảng này là click dồn dập: không nhảy thêm. */
  pokeCooldown: 0.4,
  /** Độ cao cú nhảy khi bị click. */
  hopHeight: 20,
  /** Nhảy xong thì chạy (còn lại là đi) với xác suất này. */
  runAfterPoke: 0.5,
  landTime: 0.25,
  dizzyTime: 2.5,
  /** Không ai click hoặc kéo trong khoảng này thì đi ngủ. */
  sleepAfter: 180,
} as const;

const STATES: StateTable<PetState, Pet> = {
  idle: {
    enter: (pet) => plan(pet, TUNING.idleTime),
    update: (pet, time) => (time >= pet.planned ? nextActivity(pet) : undefined),
  },
  walk: {
    enter: (pet) => startMoving(pet, TUNING.walkTime),
    update: (pet, time, dt) => move(pet, TUNING.walkSpeed, time, dt),
  },
  run: {
    enter: (pet) => startMoving(pet, TUNING.runTime),
    update: (pet, time, dt) => move(pet, TUNING.runSpeed, time, dt),
  },
  // Chỉ thức dậy khi người dùng click hoặc kéo.
  sleep: {},
  react: {
    // Nhảy một cái tại chỗ.
    update: (pet, time) => {
      const t = Math.min(time / TUNING.reactTime, 1);
      pet.y = pet.env.bounds.floor - TUNING.hopHeight * Math.sin(Math.PI * t);
      if (time < TUNING.reactTime) return undefined;
      // Nhảy xong thì đi hoặc chạy tiếp luôn: đứng lại ngay sau khi bị click trông như bị đơ.
      return pet.env.rng.chance(TUNING.runAfterPoke) ? "run" : "walk";
    },
    exit: (pet) => {
      pet.y = pet.env.bounds.floor;
    },
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
};

export interface PetOptions {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

export class Pet {
  readonly id: string;
  /** Kích thước khi vẽ (CSS pixel), dùng để giữ cả con nằm trong màn hình. Đổi bằng `resize`. */
  width: number;
  height: number;
  /** Điểm chân (giữa mép dưới) theo CSS pixel của overlay. */
  x: number;
  y: number;
  /** Vận tốc khi đang rơi hoặc bị ném (px/s). */
  vx = 0;
  vy = 0;
  facing: Facing = 1;
  /** Số giây kể từ lần cuối người dùng click hoặc kéo pet. */
  sinceInteraction = 0;
  /** Số giây kể từ lần click trước, kể cả click bị bỏ qua. */
  private sincePoke = Number.POSITIVE_INFINITY;
  /** Thời lượng đã chọn cho lượt đứng / đi / chạy hiện tại. */
  planned = 0;
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

  /** Đang ở trên mặt đất: không bị kéo, không đang rơi. */
  get grounded(): boolean {
    return this.state !== "dragged" && this.state !== "fall";
  }

  /**
   * Người dùng click vào pet: phản ứng, đang ngủ thì thức dậy. Đang nhảy hoặc click dồn dập thì chỉ
   * nhảy một lần: cú nhảy không bị bắt đầu lại, phải ngừng click một lúc mới nhảy tiếp.
   */
  poke(): void {
    if (!this.grounded) return;
    const spam = this.sincePoke < TUNING.pokeCooldown;
    this.sincePoke = 0;
    this.sinceInteraction = 0;
    if (spam || this.state === "react") return;
    this.brain.go(this, "react");
  }

  /** Người dùng click ở chỗ khác trên màn hình: đang ngủ thì giật mình thức dậy như bị click. */
  wake(): void {
    if (this.state === "sleep") this.poke();
  }

  /** Người dùng bắt đầu kéo pet, bắt được cả khi pet đang rơi. */
  grab(): void {
    this.sinceInteraction = 0;
    this.vx = 0;
    this.vy = 0;
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
    this.brain.update(this, dt);
  }

  /** Đổi cỡ pet (Settings), kẹp lại để cả con vẫn nằm trong màn hình. */
  resize(width: number, height: number): void {
    const { left, right, top, floor } = this.env.bounds;
    this.width = width;
    this.height = height;
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
    this.facing = saved.facing;
    this.sinceInteraction = saved.sinceInteraction;
    this.brain.go(this, saved.asleep ? "sleep" : "idle");
  }
}

function plan(pet: Pet, [min, max]: readonly [number, number]): void {
  pet.planned = pet.env.rng.range(min, max);
}

function nextActivity(pet: Pet): PetState {
  if (pet.sinceInteraction >= TUNING.sleepAfter) return "sleep";
  const roll = pet.env.rng.next();
  if (roll < 0.45) return "walk";
  if (roll < 0.6) return "run";
  return "idle";
}

function startMoving(pet: Pet, time: readonly [number, number]): void {
  plan(pet, time);
  if (pet.env.rng.chance(TUNING.turnChance)) pet.facing = pet.facing === 1 ? -1 : 1;
}

function move(pet: Pet, speed: number, time: number, dt: number): PetState | undefined {
  const { left, right } = pet.env.bounds;
  const half = pet.width / 2;
  pet.x += pet.facing * speed * pet.env.speed * dt;
  // Chạm mép màn hình thì quay đầu.
  if (pet.x >= right - half) {
    pet.x = right - half;
    pet.facing = -1;
  } else if (pet.x <= left + half) {
    pet.x = left + half;
    pet.facing = 1;
  }
  return time >= pet.planned ? "idle" : undefined;
}

function fall(pet: Pet, dt: number): PetState | undefined {
  const { left, right, top, floor } = pet.env.bounds;
  const half = pet.width / 2;
  pet.vy += TUNING.gravity * dt;
  pet.x += pet.vx * dt;
  pet.y += pet.vy * dt;

  if (pet.x < left + half) {
    pet.x = left + half;
    pet.vx = Math.abs(pet.vx) * TUNING.wallBounce;
  } else if (pet.x > right - half) {
    pet.x = right - half;
    pet.vx = -Math.abs(pet.vx) * TUNING.wallBounce;
  }
  if (pet.y - pet.height < top) {
    pet.y = top + pet.height;
    pet.vy = Math.abs(pet.vy) * TUNING.wallBounce;
  }
  if (Math.abs(pet.vx) > 20) pet.facing = pet.vx > 0 ? 1 : -1;
  if (pet.y < floor) return undefined;

  // Chạm đất.
  pet.y = floor;
  const impact = pet.vy;
  if (impact >= TUNING.dizzySpeed) return stop(pet, "dizzy");
  if (impact >= TUNING.bounceSpeed) {
    pet.vy = -impact * TUNING.restitution;
    pet.vx *= TUNING.groundFriction;
    return undefined;
  }
  return stop(pet, "land");
}

function stop(pet: Pet, next: PetState): PetState {
  pet.vx = 0;
  pet.vy = 0;
  return next;
}
