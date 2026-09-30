import type { Point, Rect, Remap, WindowInfo } from "@tinyworld/core";
import { clamp } from "./math";
import { Pet, type Facing, type PetEnv, type PetOptions } from "./pet";
import { Rng } from "./rng";
import { SNAPSHOT_VERSION, type WorldSnapshot } from "./snapshot";
import { Terrain, type Bounds } from "./terrain";

/** Hai màn hình lệch nhau chừng này (CSS pixel, do làm tròn DPI) vẫn tính là giáp mép. */
const EDGE_SLACK = 2;

export class World implements PetEnv {
  readonly pets: Pet[] = [];
  readonly rng: Rng;
  /** Hệ số tốc độ đi/chạy của mọi pet (Settings). */
  speed = 1;
  terrain: Terrain;
  /** Vùng làm việc của các màn hình khác, toạ độ của overlay này (`ScreenInfo.neighbors`). */
  neighbors: readonly Rect[] = [];
  /** Con trỏ chuột (CSS pixel của overlay), `null` khi chưa biết. */
  cursor: Point | null = null;

  constructor(bounds: Bounds, seed: number) {
    this.rng = new Rng(seed);
    this.terrain = new Terrain(bounds);
  }

  get bounds(): Bounds {
    return this.terrain.bounds;
  }

  /** Thêm pet đứng trên mặt đất; `x` bị kẹp lại để cả con nằm trong màn hình. */
  spawn(options: Omit<PetOptions, "y">): Pet {
    const half = options.width / 2;
    const x = clamp(options.x, this.bounds.left + half, this.bounds.right - half);
    const pet = new Pet({ ...options, x, y: this.bounds.floor }, this);
    this.pets.push(pet);
    return pet;
  }

  /**
   * Cửa sổ trên màn hình vừa đổi (xếp từ trên xuống theo thứ tự chồng): dựng lại địa hình, pet đang
   * đứng trên cửa sổ nào thì đi theo cửa sổ đó. Pet phản ứng với cửa sổ bị kéo lại gần và cửa sổ vừa bị
   * đóng hẳn (`closed`, kèm khung lúc còn hiện).
   */
  setWindows(windows: readonly WindowInfo[], closed: readonly WindowInfo[] = []): void {
    const before = this.terrain;
    this.terrain = new Terrain(this.bounds, windows);
    for (const pet of this.pets) {
      pet.follow();
      for (const { id, rect } of windows) {
        const old = before.window(id);
        // Lệch dưới 1 pixel là do làm tròn (đổi DPI), không phải bị kéo.
        if (old && (Math.abs(old.x - rect.x) >= 1 || Math.abs(old.y - rect.y) >= 1)) pet.windowMoved(id, old, rect);
      }
      for (const { id, rect } of closed) pet.windowClosed(id, rect);
    }
  }

  /**
   * Overlay vừa sang màn hình khác, hoặc màn hình đổi độ phân giải, DPI, taskbar. `remap` đổi toạ độ cũ
   * (pet, cửa sổ, con trỏ) sang toạ độ mới; Rust sẽ gửi lại danh sách cửa sổ theo toạ độ mới ngay sau đó.
   */
  setScreen(bounds: Bounds, neighbors: readonly Rect[], remap?: Remap): void {
    const map = (r: Rect): Rect =>
      remap
        ? { x: r.x * remap.scale + remap.x, y: r.y * remap.scale + remap.y, width: r.width * remap.scale, height: r.height * remap.scale }
        : r;
    this.terrain = new Terrain(
      bounds,
      this.terrain.windows.map(({ id, rect }) => ({ id, rect: map(rect) })),
    );
    this.neighbors = neighbors;
    if (remap && this.cursor) {
      this.cursor = { x: this.cursor.x * remap.scale + remap.x, y: this.cursor.y * remap.scale + remap.y };
    }
    for (const pet of this.pets) {
      if (remap) pet.remap(remap);
      pet.rebound();
    }
  }

  exit(side: Facing, y: number): boolean {
    const { left, right } = this.bounds;
    return this.neighbors.some((n) => {
      const edge = side > 0 ? n.x - right : left - (n.x + n.width);
      return Math.abs(edge) <= EDGE_SLACK && y >= n.y && y < n.y + n.height;
    });
  }

  /** Con trỏ vừa di chuyển (CSS pixel của overlay). */
  moveCursor(x: number, y: number): void {
    this.cursor = { x, y };
  }

  /** Các cửa sổ đang che một phần `pet` (pet đứng trên cửa sổ nằm dưới chúng). */
  occluders(pet: Pet): Rect[] {
    if (!pet.mount) return [];
    const left = pet.x - pet.width;
    const right = pet.x + pet.width;
    const top = pet.y - pet.height * 1.5;
    const bottom = pet.y + 1;
    return this.terrain
      .above(pet.mount.id)
      .filter((r) => r.x < right && r.x + r.width > left && r.y < bottom && r.y + r.height > top);
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
