import { ANIMATION_NAMES, type AnimationName, type Point, type Rect } from "@tinyworld/core";

/** Độ đục tối thiểu (0–255) để một pixel được tính là có hình khi kiểm tra con trỏ. */
const ALPHA_THRESHOLD = 32;

export interface Animation {
  image: CanvasImageSource;
  frames: Rect[];
  /** Mặt nạ alpha của từng frame, để click vào chỗ trống quanh pet vẫn lọt xuống. */
  masks: Mask[];
  fps: number;
  loop: boolean;
}

/**
 * Pixel nào của một frame có hình, 1 bit mỗi pixel (một pack khoảng 45 frame 192×192 chỉ tốn chừng
 * 200 KB). Tính sẵn khung chứa phần có hình để khỏi quét lại cả frame.
 */
export class Mask {
  private readonly bits: Uint8Array;
  /** Cột ngoài cùng bên trái, bên phải có hình; frame trống thì `left` > `right`. */
  readonly left: number;
  readonly right: number;
  /** Hàng trên cùng có hình (frame trống thì bằng `height`). */
  readonly top: number;

  /** `solid(i)`: pixel thứ `i` (theo hàng, từ góc trên trái) có hình không. */
  constructor(
    readonly width: number,
    readonly height: number,
    solid: (i: number) => boolean,
  ) {
    this.bits = new Uint8Array(Math.ceil((width * height) / 8));
    let left = width;
    let right = -1;
    let top = height;
    for (let i = 0; i < width * height; i++) {
      if (!solid(i)) continue;
      this.bits[i >> 3] |= 1 << (i & 7);
      const x = i % width;
      left = Math.min(left, x);
      right = Math.max(right, x);
      top = Math.min(top, Math.floor(i / width));
    }
    this.left = left;
    this.right = right;
    this.top = top;
  }

  has(x: number, y: number): boolean {
    const i = y * this.width + x;
    return (this.bits[i >> 3] & (1 << (i & 7))) !== 0;
  }
}

/** Sprite đã nạp xong, sẵn sàng để vẽ. */
export interface SpriteSet {
  /** Tên thư mục pack, `null` là pet tạm vẽ bằng code. */
  id: string | null;
  name: string;
  frameWidth: number;
  frameHeight: number;
  scale: number;
  pixelArt: boolean;
  facing: "left" | "right";
  anchor: Point;
  animations: Record<AnimationName, Animation>;
}

export function buildAnimation(
  image: CanvasImageSource,
  frames: Rect[],
  fps: number,
  loop: boolean,
): Animation {
  const masks = frames.map((frame) => alphaMask(image, frame));
  return { image, frames, masks, fps, loop };
}

/** Pack thiếu animation thì mượn animation gần giống trước khi dùng `idle`. */
const SIMILAR: Partial<Record<AnimationName, AnimationName>> = {
  walk: "run",
  run: "walk",
  dragged: "fall",
  fall: "dragged",
  // Bám tường trông gần giống bị nhấc lên (tay chân buông thõng), nhảy thì như nhảy lên khi bị click.
  climb: "dragged",
  jump: "react",
};

/** Ngủ và choáng mà pack không có thì đứng yên ở frame đầu của `idle` (choáng thì kèm sao bay quanh đầu). */
const STILL: readonly AnimationName[] = ["sleep", "dizzy"];

/** Điền đủ mọi animation cho pack chỉ có một phần. */
export function withFallback(
  animations: Partial<Record<AnimationName, Animation>> & { idle: Animation },
): Record<AnimationName, Animation> {
  const { idle } = animations;
  const still: Animation = { ...idle, frames: [idle.frames[0]], masks: [idle.masks[0]], fps: 1 };
  const result = {} as Record<AnimationName, Animation>;
  for (const name of ANIMATION_NAMES) {
    const similar = SIMILAR[name];
    result[name] =
      animations[name] ??
      (similar && animations[similar]) ??
      (STILL.includes(name) ? still : idle);
  }
  return result;
}

/** Đỉnh đầu nhân vật trong frame (pixel của frame), để đặt hiệu ứng ngay trên đầu. */
export interface Head {
  /** Hàng pixel trên cùng có hình. */
  top: number;
  /** Giữa đầu theo chiều ngang. */
  centerX: number;
}

export function headOf(mask: Mask): Head {
  const { width, height, top } = mask;
  if (mask.right < 0) return { top: 0, centerX: width / 2 };
  // Lấy vài hàng trên cùng (tai, đỉnh đầu) chứ không lấy cả hình, để đuôi hay tay không kéo lệch tâm.
  const rows = Math.max(1, Math.round(height / 8));
  let min = width;
  let max = -1;
  for (let y = top; y < Math.min(height, top + rows); y++) {
    for (let x = mask.left; x <= mask.right; x++) {
      if (!mask.has(x, y)) continue;
      min = Math.min(min, x);
      max = Math.max(max, x);
    }
  }
  return { top, centerX: (min + max + 1) / 2 };
}

/** Chỗ đội mũ trên đầu nhân vật trong một frame (pixel của frame, nhân vật quay theo `SpriteSet.facing`). */
export interface Crown {
  /** Giữa đỉnh đầu theo chiều ngang. */
  x: number;
  /** Hàng đỉnh đầu, chỗ vành mũ đặt lên. */
  y: number;
}

/** Hàng đã rộng chừng này phần bề ngang đầu thì là đỉnh đầu; hẹp hơn là tai, râu, ăng-ten, lá. */
export const CROWN_SHARE = 0.5;
/** Đầu nằm trong chừng này phần trên cùng của hình. */
const HEAD_SHARE = 0.45;
const crowns = new WeakMap<Mask, Map<number, Crown>>();

/** Đoạn liền dài nhất có hình trong hàng `y`: [đầu, cuối). */
function widestRun(mask: Mask, y: number): [number, number] {
  let best: [number, number] = [0, 0];
  let start = -1;
  for (let x = mask.left; x <= mask.right + 1; x++) {
    const solid = x <= mask.right && mask.has(x, y);
    if (solid && start < 0) start = x;
    if (!solid && start >= 0) {
      if (x - start > best[1] - best[0]) best = [start, x];
      start = -1;
    }
  }
  return best;
}

/**
 * Đỉnh đầu thật của nhân vật: hàng đầu tiên (từ trên xuống) mà đoạn liền dài nhất đã rộng bằng `share`
 * phần cái đầu, nên tai thỏ, râu ong, ăng-ten, ngọn lá mảnh phía trên bị bỏ qua. Tính một lần cho mỗi
 * frame và mỗi `share`.
 */
export function crownOf(mask: Mask, share = CROWN_SHARE): Crown {
  let cache = crowns.get(mask);
  if (!cache) {
    cache = new Map();
    crowns.set(mask, cache);
  }
  const cached = cache.get(share);
  if (cached) return cached;
  const { width, height, top } = mask;
  let crown: Crown = { x: width / 2, y: top };
  if (mask.right >= 0) {
    const span = height - top;
    const bottom = Math.min(height, top + Math.max(1, Math.ceil(span * HEAD_SHARE)));
    const runs: [number, number][] = [];
    for (let y = top; y < bottom; y++) runs.push(widestRun(mask, y));
    const widest = Math.max(...runs.map(([a, b]) => b - a));
    const i = runs.findIndex(([a, b]) => b - a >= widest * share);
    const [a, b] = runs[i];
    crown = { x: (a + b) / 2, y: top + i };
  }
  cache.set(share, crown);
  return crown;
}

function alphaMask(image: CanvasImageSource, frame: Rect): Mask {
  const canvas = new OffscreenCanvas(frame.width, frame.height);
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("Không tạo được canvas để đọc sprite.");
  ctx.drawImage(image, frame.x, frame.y, frame.width, frame.height, 0, 0, frame.width, frame.height);
  const { data } = ctx.getImageData(0, 0, frame.width, frame.height);
  return new Mask(frame.width, frame.height, (i) => data[i * 4 + 3] > ALPHA_THRESHOLD);
}
