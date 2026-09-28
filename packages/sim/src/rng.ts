/**
 * Số ngẫu nhiên có seed (mulberry32): cùng seed cho ra cùng một chuỗi số,
 * nên hành vi ngẫu nhiên của pet test được và tái hiện được.
 */
export class Rng {
  private state: number;

  constructor(seed: number) {
    this.state = seed >>> 0;
  }

  /** Số thực trong [0, 1). */
  next(): number {
    this.state = (this.state + 0x6d2b79f5) >>> 0;
    let t = this.state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }

  /** Số thực trong [min, max). */
  range(min: number, max: number): number {
    return min + (max - min) * this.next();
  }

  /** true với xác suất `p` (0–1). */
  chance(p: number): boolean {
    return this.next() < p;
  }

  pick<T>(items: readonly T[]): T {
    if (items.length === 0) throw new Error("Rng.pick: danh sách rỗng.");
    return items[Math.floor(this.next() * items.length)] as T;
  }
}
