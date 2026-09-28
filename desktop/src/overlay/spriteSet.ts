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

/** `outline`: màu viền thêm quanh nhân vật (xem `SpriteManifest.outline`), `null` là giữ nguyên ảnh. */
export function buildAnimation(
  image: CanvasImageSource,
  frames: Rect[],
  fps: number,
  loop: boolean,
  outline: string | null = null,
): Animation {
  const source = outline ? outlinedStrip(image, frames, outline) : { image, frames };
  const masks = source.frames.map((frame) => alphaMask(source.image, frame));
  return { ...source, masks, fps, loop };
}

/**
 * Chép các frame sang một dải ảnh mới, mỗi frame thêm viền 1 pixel quanh phần có hình: nhân vật
 * màu sáng đứng trên nền trắng (trang web, tài liệu) vẫn thấy rõ. Làm một lần lúc nạp pack.
 */
function outlinedStrip(
  image: CanvasImageSource,
  frames: Rect[],
  outline: string,
): { image: OffscreenCanvas; frames: Rect[] } {
  const { width, height } = frames[0];
  const canvas = new OffscreenCanvas(width * frames.length, height);
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) throw new Error("Không tạo được canvas để vẽ viền sprite.");
  const color = hexColor(outline);
  const rects = frames.map((frame, i) => {
    const x = i * width;
    ctx.drawImage(image, frame.x, frame.y, width, height, x, 0, width, height);
    const pixels = ctx.getImageData(x, 0, width, height);
    addOutline(pixels.data, width, height, color);
    ctx.putImageData(pixels, x, 0);
    return { x, y: 0, width, height };
  });
  return { image: canvas, frames: rects };
}

/** Tô màu viền vào mọi pixel trống nằm sát (trên, dưới, trái, phải) một pixel có hình. */
function addOutline(data: Uint8ClampedArray, width: number, height: number, color: number[]): void {
  const solid = (x: number, y: number) =>
    x >= 0 && y >= 0 && x < width && y < height && data[(y * width + x) * 4 + 3] > ALPHA_THRESHOLD;
  const edge: number[] = [];
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (solid(x, y)) continue;
      if (solid(x - 1, y) || solid(x + 1, y) || solid(x, y - 1) || solid(x, y + 1)) {
        edge.push(y * width + x);
      }
    }
  }
  for (const i of edge) data.set(color, i * 4);
}

/** "#rrggbb" / "#rrggbbaa" (đã kiểm tra trong parseSpriteManifest) -> [r, g, b, a]. */
function hexColor(hex: string): number[] {
  const channels = hex.slice(1).match(/../g) ?? [];
  const [r, g, b, a = 255] = channels.map((c) => parseInt(c, 16));
  return [r, g, b, a];
}

/** Pack thiếu animation thì mượn animation gần giống trước khi dùng `idle`. */
const SIMILAR: Partial<Record<AnimationName, AnimationName>> = {
  walk: "run",
  run: "walk",
  dragged: "fall",
  fall: "dragged",
};

/** Ngủ và choáng mà pack không có thì đứng yên ở frame đầu của `idle` (kèm hiệu ứng 💤 / 💫). */
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
