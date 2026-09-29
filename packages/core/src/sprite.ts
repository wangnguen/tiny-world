import type { Point, Rect } from "./types";

/**
 * Animation engine dùng, mỗi cái ứng với một state của pet. Chỉ `idle` là bắt buộc:
 * pack thiếu animation nào thì app dùng animation gần giống hoặc `idle` thay (bảng thay thế: assets/README.md).
 */
export const ANIMATION_NAMES = [
  "idle",
  "walk",
  "run",
  "sleep",
  "dragged",
  "fall",
  "land",
  "react",
  "dizzy",
] as const;

export type AnimationName = (typeof ANIMATION_NAMES)[number];

export interface AnimationSpec {
  /** File PNG/WebP, đường dẫn tương đối so với pet.json. */
  image: string;
  /** Số frame. `undefined`: lấy hết các frame từ `start` tới cuối ảnh. */
  frames: number | undefined;
  fps: number;
  /** Vị trí frame đầu tiên, đếm từ trái sang phải rồi xuống hàng. */
  start: number;
  /** Hàng bắt đầu, khi nhiều animation nằm chung một ảnh. */
  row: number;
  /** false: chạy một lần rồi dừng ở frame cuối. */
  loop: boolean;
}

/** Nội dung pet.json của một sprite pack (định dạng: assets/README.md). */
export interface SpriteManifest {
  name: string;
  frameWidth: number;
  frameHeight: number;
  /** Tỉ lệ vẽ ở cỡ 100%: frame 192×192 với scale 0.5 thành 96×96 CSS pixel. */
  scale: number;
  /** true: phóng to kiểu pixel art, không làm mờ. */
  pixelArt: boolean;
  /** Hướng nhân vật nhìn trong ảnh gốc. */
  facing: "left" | "right";
  /** Điểm chân nhân vật trong frame (pixel của frame), dùng để đặt pet lên mặt đất. */
  anchor: Point;
  animations: { idle: AnimationSpec } & Partial<Record<AnimationName, AnimationSpec>>;
}

const MAX_FRAME_SIZE = 1024;
const MAX_FRAMES = 256;

type Json = Record<string, unknown>;

/** Đọc và kiểm tra pet.json. Sai chỗ nào thì ném Error chỉ rõ trường đó. */
export function parseSpriteManifest(value: unknown): SpriteManifest {
  const root = object(value, "pet.json");
  const frameWidth = integer(root.frameWidth, "frameWidth", 1, MAX_FRAME_SIZE);
  const frameHeight = integer(root.frameHeight, "frameHeight", 1, MAX_FRAME_SIZE);

  const animations: Partial<Record<AnimationName, AnimationSpec>> = {};
  for (const [name, spec] of Object.entries(object(root.animations, "animations"))) {
    if (!isAnimationName(name)) {
      throw new Error(
        `animations.${name}: tên không hợp lệ, chỉ nhận ${ANIMATION_NAMES.join(", ")}.`,
      );
    }
    animations[name] = parseAnimation(spec, `animations.${name}`);
  }
  const idle = animations.idle;
  if (!idle) throw new Error("animations.idle: bắt buộc phải có.");

  const anchor = optional(root.anchor, { x: frameWidth / 2, y: frameHeight }, (v) => {
    const point = object(v, "anchor");
    return {
      x: number(point.x, "anchor.x", 0, frameWidth),
      y: number(point.y, "anchor.y", 0, frameHeight),
    };
  });

  return {
    name: optional(root.name, "pet", (v) => string(v, "name")),
    frameWidth,
    frameHeight,
    scale: optional(root.scale, 2, (v) => number(v, "scale", 0.25, 16)),
    pixelArt: optional(root.pixelArt, true, (v) => boolean(v, "pixelArt")),
    facing: optional(root.facing, "right", (v) => oneOf(v, "facing", ["left", "right"] as const)),
    anchor,
    animations: { ...animations, idle },
  };
}

function parseAnimation(value: unknown, path: string): AnimationSpec {
  const spec = object(value, path);
  return {
    image: imagePath(spec.image, `${path}.image`),
    frames: optional(spec.frames, undefined, (v) => integer(v, `${path}.frames`, 1, MAX_FRAMES)),
    fps: number(spec.fps, `${path}.fps`, 0.5, 60),
    start: optional(spec.start, 0, (v) => integer(v, `${path}.start`, 0, MAX_FRAMES * 16)),
    row: optional(spec.row, 0, (v) => integer(v, `${path}.row`, 0, MAX_FRAME_SIZE)),
    loop: optional(spec.loop, true, (v) => boolean(v, `${path}.loop`)),
  };
}

/**
 * Vị trí từng frame trong ảnh. Frame xếp từ trái sang phải, hết hàng thì xuống hàng dưới,
 * nên dùng được cho cả ảnh dải ngang (mỗi animation một file) lẫn sheet nhiều hàng.
 */
export function frameRects(
  spec: AnimationSpec,
  frameWidth: number,
  frameHeight: number,
  imageWidth: number,
  imageHeight: number,
): Rect[] {
  const columns = Math.floor(imageWidth / frameWidth);
  const rows = Math.floor(imageHeight / frameHeight);
  if (columns < 1 || rows <= spec.row) {
    throw new Error(
      `${spec.image}: ảnh ${imageWidth}×${imageHeight} không chứa được frame ${frameWidth}×${frameHeight} ở hàng ${spec.row}.`,
    );
  }
  const available = (rows - spec.row) * columns - spec.start;
  const count = spec.frames ?? available;
  if (count < 1 || count > available) {
    throw new Error(
      `${spec.image}: cần ${count} frame từ vị trí ${spec.start} nhưng ảnh chỉ còn ${Math.max(available, 0)}.`,
    );
  }
  const rects: Rect[] = [];
  for (let i = 0; i < count; i++) {
    const index = spec.start + i;
    rects.push({
      x: (index % columns) * frameWidth,
      y: (spec.row + Math.floor(index / columns)) * frameHeight,
      width: frameWidth,
      height: frameHeight,
    });
  }
  return rects;
}

/** Frame cần vẽ sau `time` giây kể từ lúc animation bắt đầu. */
export function frameIndex(frameCount: number, fps: number, loop: boolean, time: number): number {
  const index = Math.floor(Math.max(0, time) * fps);
  return loop ? index % frameCount : Math.min(index, frameCount - 1);
}

function isAnimationName(name: string): name is AnimationName {
  return (ANIMATION_NAMES as readonly string[]).includes(name);
}

function optional<T, D>(value: unknown, fallback: D, parse: (value: unknown) => T): T | D {
  return value === undefined ? fallback : parse(value);
}

function object(value: unknown, path: string): Json {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`${path}: phải là object.`);
  }
  return value as Json;
}

function number(value: unknown, path: string, min: number, max: number): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < min || value > max) {
    throw new Error(`${path}: phải là số trong khoảng ${min}–${max}.`);
  }
  return value;
}

function integer(value: unknown, path: string, min: number, max: number): number {
  const n = number(value, path, min, max);
  if (!Number.isInteger(n)) throw new Error(`${path}: phải là số nguyên.`);
  return n;
}

function boolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") throw new Error(`${path}: phải là true hoặc false.`);
  return value;
}

function string(value: unknown, path: string): string {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${path}: phải là chuỗi không rỗng.`);
  }
  return value.trim();
}

function oneOf<T extends string>(value: unknown, path: string, options: readonly T[]): T {
  if (typeof value !== "string" || !(options as readonly string[]).includes(value)) {
    throw new Error(`${path}: chỉ nhận ${options.join(" hoặc ")}.`);
  }
  return value as T;
}

/** Đường dẫn ảnh tương đối, không cho ra ngoài thư mục pack. */
function imagePath(value: unknown, path: string): string {
  const file = string(value, path).replace(/\\/g, "/").replace(/^(\.\/)+/, "");
  if (file.startsWith("/") || file.includes(":") || file.split("/").includes("..")) {
    throw new Error(`${path}: phải là đường dẫn nằm trong thư mục pack.`);
  }
  return file;
}
