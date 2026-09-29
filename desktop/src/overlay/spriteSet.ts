import { ANIMATION_NAMES, type AnimationName, type Point, type Rect } from "@tinyworld/core";

/** Độ đục tối thiểu (0–255) để một pixel được tính là có hình khi kiểm tra con trỏ. */
const ALPHA_THRESHOLD = 32;

export interface Animation {
  image: CanvasImageSource;
  frames: Rect[];
  /** Mặt nạ alpha của từng frame (1 = có hình), để click vào chỗ trống quanh pet vẫn lọt xuống. */
  masks: Uint8Array[];
  fps: number;
  loop: boolean;
}

/** Sprite đã nạp xong, sẵn sàng để vẽ. */
export interface SpriteSet {
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

export function headOf(mask: Uint8Array, width: number): Head {
  const height = mask.length / width;
  const first = mask.indexOf(1);
  if (first < 0) return { top: 0, centerX: width / 2 };
  const top = Math.floor(first / width);
  // Lấy vài hàng trên cùng (tai, đỉnh đầu) chứ không lấy cả hình, để đuôi hay tay không kéo lệch tâm.
  const rows = Math.max(1, Math.round(height / 8));
  let min = width;
  let max = -1;
  for (let y = top; y < Math.min(height, top + rows); y++) {
    for (let x = 0; x < width; x++) {
      if (mask[y * width + x] === 0) continue;
      min = Math.min(min, x);
      max = Math.max(max, x);
    }
  }
  return { top, centerX: (min + max + 1) / 2 };
}

function alphaMask(image: CanvasImageSource, frame: Rect): Uint8Array {
  const canvas = new OffscreenCanvas(frame.width, frame.height);
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("Không tạo được canvas để đọc sprite.");
  ctx.drawImage(image, frame.x, frame.y, frame.width, frame.height, 0, 0, frame.width, frame.height);
  const { data } = ctx.getImageData(0, 0, frame.width, frame.height);
  const mask = new Uint8Array(frame.width * frame.height);
  for (let i = 0; i < mask.length; i++) mask[i] = data[i * 4 + 3] > ALPHA_THRESHOLD ? 1 : 0;
  return mask;
}
