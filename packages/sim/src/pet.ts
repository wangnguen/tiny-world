import type { AnimationName } from "@tinyworld/core";
import { StateMachine, type StateTable } from "./fsm";

/** State hiện có, trùng tên với animation. Phase 1 thêm walk, run, sleep, dragged, fall... */
export type PetState = Extract<AnimationName, "idle" | "react">;

/** 1: nhìn sang phải, -1: nhìn sang trái. */
export type Facing = 1 | -1;

/** Thời gian phản ứng khi bị click (giây). */
export const REACT_DURATION = 0.5;

const STATES: StateTable<PetState, Pet> = {
  idle: {},
  react: {
    update: (_pet, time) => (time >= REACT_DURATION ? "idle" : undefined),
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
  /** Kích thước khi vẽ (CSS pixel), dùng để giữ cả con nằm trong màn hình. */
  readonly width: number;
  readonly height: number;
  /** Điểm chân (giữa mép dưới) theo CSS pixel của overlay. */
  x: number;
  y: number;
  facing: Facing = 1;
  private readonly brain = new StateMachine<PetState, Pet>(STATES, "idle");

  constructor(options: PetOptions) {
    this.id = options.id;
    this.x = options.x;
    this.y = options.y;
    this.width = options.width;
    this.height = options.height;
  }

  get state(): PetState {
    return this.brain.state;
  }

  /** Số giây đã ở state hiện tại, renderer dùng để chọn frame. */
  get stateTime(): number {
    return this.brain.time;
  }

  /** Người dùng click vào pet. */
  poke(): void {
    this.brain.go(this, "react");
  }

  step(dt: number): void {
    this.brain.update(this, dt);
  }
}
