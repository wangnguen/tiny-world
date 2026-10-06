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

function alphaMask(image: CanvasImageSource, frame: Rect): Mask {
  const canvas = new OffscreenCanvas(frame.width, frame.height);
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("Không tạo được canvas để đọc sprite.");
  ctx.drawImage(image, frame.x, frame.y, frame.width, frame.height, 0, 0, frame.width, frame.height);
  const { data } = ctx.getImageData(0, 0, frame.width, frame.height);
  return new Mask(frame.width, frame.height, (i) => data[i * 4 + 3] > ALPHA_THRESHOLD);
}
