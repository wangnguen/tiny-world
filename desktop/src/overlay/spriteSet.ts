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
  return { image, frames, masks: frames.map((frame) => alphaMask(image, frame)), fps, loop };
}

/** Animation nào pack không có thì dùng `idle`. */
export function withFallback(
  animations: Partial<Record<AnimationName, Animation>> & { idle: Animation },
): Record<AnimationName, Animation> {
  const result = {} as Record<AnimationName, Animation>;
  for (const name of ANIMATION_NAMES) result[name] = animations[name] ?? animations.idle;
  return result;
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
