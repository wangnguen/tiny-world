import { clamp } from "./math";
import { Pet, type Bounds, type PetEnv, type PetOptions } from "./pet";
import { Rng } from "./rng";
import { SNAPSHOT_VERSION, type WorldSnapshot } from "./snapshot";

export class World implements PetEnv {
  readonly pets: Pet[] = [];
  readonly rng: Rng;
  /** Hệ số tốc độ đi/chạy của mọi pet (Settings). */
  speed = 1;

  constructor(
    readonly bounds: Bounds,
    seed: number,
  ) {
    this.rng = new Rng(seed);
  }

  /** Thêm pet đứng trên mặt đất; `x` bị kẹp lại để cả con nằm trong màn hình. */
  spawn(options: Omit<PetOptions, "y">): Pet {
    const half = options.width / 2;
    const x = clamp(options.x, this.bounds.left + half, this.bounds.right - half);
    const pet = new Pet({ ...options, x, y: this.bounds.floor }, this);
    this.pets.push(pet);
    return pet;
  }

  /** Mọi pet đều đang ngủ: không có gì chuyển động, overlay có thể dừng vòng lặp vẽ. */
  get resting(): boolean {
    return this.pets.every((pet) => pet.state === "sleep");
  }

  step(dt: number): void {
    for (const pet of this.pets) pet.step(dt);
  }

  snapshot(): WorldSnapshot {
    return { version: SNAPSHOT_VERSION, pets: this.pets.map((pet) => pet.snapshot()) };
  }
}
