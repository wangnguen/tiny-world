import type { Point, Rect, Remap, WindowInfo } from "@tinyworld/core";
import { clamp } from "./math";
import { Pet, type Facing, type PetEnv, type PetOptions } from "./pet";
import { Rng } from "./rng";
import { SNAPSHOT_VERSION, type WorldSnapshot } from "./snapshot";
import { Terrain, type Bounds } from "./terrain";
import { TUNING } from "./tuning";

/** Hai màn hình lệch nhau chừng này (CSS pixel, do làm tròn DPI) vẫn tính là giáp mép. */
const EDGE_SLACK = 2;
/**
 * Các con chạy theo sang màn hình mới: con ở gần mép sang trước, con ở xa sang sau. Xuất hiện ngoài mép
 * cách mép `FOLLOW_SCALE` lần khoảng cách cũ tới mép (tối đa `FOLLOW_MAX` px, chạy vào mất khoảng 3 giây),
 * hai con cách nhau ít nhất `FOLLOW_GAP` lần bề ngang pet để không chồng lên nhau.
 */
const FOLLOW_SCALE = 0.15;
const FOLLOW_MAX = 320;
const FOLLOW_GAP = 0.8;

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
  /** Đang là ban đêm (overlay đặt theo giờ máy hoặc thời tiết thật): đi chậm hơn, buồn ngủ sớm hơn. */
  night = false;
  /** Người dùng đang gõ phím (Rust báo): không nói câu cho vui, không làm gì nổi bật. */
  busy = false;
  /** Cho pet nói câu cho vui (Settings): tắt thì `chat` không nói gì. */
  chatter = true;
  /** Số giây (lúc vòng lặp chạy) kể từ câu nói cho vui gần nhất của cả nhóm. Mở app thì chờ nửa `chatGap`. */
  private sinceChat = TUNING.chatGap / 2;

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
    // Con mới vào nhóm: tính chung đồng hồ "không đụng tới" với cả nhóm (`step`).
    const peer = this.pets[0];
    if (peer) pet.sinceInteraction = peer.sinceInteraction;
    this.pets.push(pet);
    return pet;
  }

  /** Bỏ pet `id` (bớt nhân vật trong Settings). */
  remove(id: string): void {
    const index = this.pets.findIndex((pet) => pet.id === id);
    if (index >= 0) this.pets.splice(index, 1);
  }

  /** Chỉ có một con thì được tự đi sang màn hình khác (`PetEnv.wander`). */
  get wander(): boolean {
    return this.pets.length < 2;
  }

  /** Người dùng click ở chỗ khác trên màn hình: các con đang ngủ đều thức dậy. */
  wakeAll(): void {
    for (const pet of this.pets) pet.wake();
  }

  /**
   * `pet` nói một câu cho vui: cả nhóm `TUNING.chatGap` giây mới nói một câu (tối đa 4 câu mỗi giờ), không
   * nói lúc người dùng đang gõ phím, lúc pet đang ngủ hay lúc tắt `chatter`. Trả về `true` nếu đã nói.
   */
  chat(pet: Pet, text: string): boolean {
    if (!this.chatter || this.busy || this.sinceChat < TUNING.chatGap || pet.state === "sleep") return false;
    this.sinceChat = 0;
    pet.say(text);
    return true;
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
    if (remap) for (const pet of this.pets) pet.remap(remap);
    // Overlay sang màn hình khác theo một con (bị kéo, ném sang): con nào còn nằm ngoài màn hình mới (ở màn
    // hình cũ) thì chạy theo, vào từ mép phía màn hình cũ. Màn hình đổi tại chỗ (DPI, taskbar) thì pet vẫn
    // nằm trong màn hình, chỉ đứng lên mặt đất mới.
    const followers: { pet: Pet; side: Facing; distance: number }[] = [];
    for (const pet of this.pets) {
      const away = remap && !pet.crossing && pet.state !== "dragged" ? this.outside(pet) : null;
      if (away) followers.push({ pet, ...away });
      else pet.rebound();
    }
    followers.sort((a, b) => a.distance - b.distance);
    const behind = { [-1]: Number.NEGATIVE_INFINITY, [1]: Number.NEGATIVE_INFINITY };
    for (const { pet, side, distance } of followers) {
      const at = Math.max(Math.min(distance * FOLLOW_SCALE, FOLLOW_MAX), behind[side] + pet.width * FOLLOW_GAP);
      pet.joinFrom(side, at);
      behind[side] = at;
    }
  }

  /** Pet nằm ngoài màn hình về phía nào (-1: bên trái, 1: bên phải) và cách mép đó bao xa; `null` nếu vẫn ở trong. */
  private outside(pet: Pet): { side: Facing; distance: number } | null {
    const { left, right } = this.bounds;
    if (pet.x < left) return { side: -1, distance: left - pet.x };
    if (pet.x > right) return { side: 1, distance: pet.x - right };
    return null;
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

  /** Mọi pet đều đang ngủ, không con nào đang nói: không có gì chuyển động, overlay có thể dừng vòng lặp vẽ. */
  get resting(): boolean {
    return this.pets.every((pet) => pet.state === "sleep" && !pet.speech);
  }

  step(dt: number): void {
    this.sinceChat += dt;
    for (const pet of this.pets) pet.step(dt);
    // Đồng hồ "không đụng tới" tính chung cả nhóm: click, kéo con nào cũng tính, nên cả nhóm cùng buồn ngủ
    // và lần lượt đi ngủ. Cả nhóm ngủ thì overlay dừng hẳn vòng lặp vẽ (`resting`).
    if (this.pets.length < 2) return;
    const since = Math.min(...this.pets.map((pet) => pet.sinceInteraction));
    for (const pet of this.pets) pet.sinceInteraction = since;
  }

  snapshot(): WorldSnapshot {
    return { version: SNAPSHOT_VERSION, pets: this.pets.map((pet) => pet.snapshot()) };
  }
}
