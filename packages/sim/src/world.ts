import { Pet, type PetOptions } from "./pet";
import { Rng } from "./rng";

/** Mặt đất: đoạn nằm ngang từ `left` tới `right` ở độ cao `y` (CSS pixel của overlay). */
export interface Floor {
  left: number;
  right: number;
  y: number;
}

export class World {
  readonly pets: Pet[] = [];
  readonly rng: Rng;

  constructor(
    readonly floor: Floor,
    seed: number,
  ) {
    this.rng = new Rng(seed);
  }

  /** Thêm pet đứng trên mặt đất; `x` bị kẹp lại để cả con nằm trong màn hình. */
  spawn(options: Omit<PetOptions, "y">): Pet {
    const half = options.width / 2;
    const x = clamp(options.x, this.floor.left + half, this.floor.right - half);
    const pet = new Pet({ ...options, x, y: this.floor.y });
    this.pets.push(pet);
    return pet;
  }

  step(dt: number): void {
    for (const pet of this.pets) pet.step(dt);
  }
}

/** Kẹp `value` vào [min, max]; khoảng rỗng (pet rộng hơn màn hình) thì lấy điểm giữa. */
function clamp(value: number, min: number, max: number): number {
  if (max < min) return (min + max) / 2;
  return Math.min(Math.max(value, min), max);
}
