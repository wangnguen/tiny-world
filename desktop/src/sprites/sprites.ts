import {
  ANIMATION_NAMES,
  MAX_PETS,
  frameRects,
  parseSpriteManifest,
  type AnimationName,
  type Rect,
  type SpriteManifest,
} from "@tinyworld/core";
import { createPlaceholderSprite } from "./placeholder";
import { buildAnimation, withFallback, type Animation, type SpriteSet } from "./spriteSet";

// Mỗi sprite pack là một thư mục trong assets/sprites/ có pet.json (định dạng: assets/README.md).
// Vite gom pack vào bản build, nên thêm pack mới chỉ cần chạy lại app.
const manifests = import.meta.glob<unknown>("../../../assets/sprites/*/pet.json", {
  eager: true,
  import: "default",
});
const imageFiles = import.meta.glob<string>("../../../assets/sprites/**/*.{png,webp,PNG,WEBP}", {
  eager: true,
  query: "?url",
  import: "default",
});
// Windows không phân biệt hoa thường, nên pet.json ghi "idle.png" cho file "Idle.png" vẫn phải chạy.
const imageUrls = new Map(Object.entries(imageFiles).map(([path, url]) => [path.toLowerCase(), url]));

/** Một nhân vật chọn được trong Settings. */
export interface PackInfo {
  /** Tên thư mục trong assets/sprites/, lưu vào `Settings.pet`. */
  id: string;
  /** `name` trong pet.json. */
  name: string;
}

/** Pack đọc được `pet.json`, xếp theo tên thư mục; pack lỗi bị bỏ qua (in lý do ra console). */
const packs: { info: PackInfo; path: string; manifest: SpriteManifest }[] = Object.keys(manifests)
  .sort()
  .flatMap((path) => {
    const id = packName(path);
    try {
      const manifest = parseSpriteManifest(manifests[path]);
      return [{ info: { id, name: manifest.name }, path, manifest }];
    } catch (error) {
      console.warn(`Sprite pack ${id} lỗi, bỏ qua:`, error);
      return [];
    }
  });

export function listPacks(): PackInfo[] {
  return packs.map((pack) => pack.info);
}

/** Pack sẽ được dùng cho `id`: đúng pack đó nếu còn, không thì pack đầu tiên; không có pack nào thì `null`. */
export function resolvePack(id: string | null | undefined): string | null {
  return (packs.find((pack) => pack.info.id === id) ?? packs[0])?.info.id ?? null;
}

/**
 * Các pack sẽ hiện cho `Settings.pets`: bỏ pack không còn trong bản build và pack trùng, tối đa `MAX_PETS`,
 * giữ thứ tự chọn. Không còn pack nào thì pack đầu tiên; chưa có pack nào thì `[null]` (một pet tạm).
 */
export function resolvePacks(ids: readonly string[]): (string | null)[] {
  const known = [...new Set(ids)].filter((id) => packs.some((pack) => pack.info.id === id));
  return known.length > 0 ? known.slice(0, MAX_PETS) : [resolvePack(null)];
}

/** Nạp pack `id` (xem `resolvePack`). Chưa có pack hoặc pack lỗi thì dùng pet tạm. */
export async function loadSpriteSet(id?: string | null): Promise<SpriteSet> {
  const pack = packs.find((p) => p.info.id === resolvePack(id));
  if (!pack) return createPlaceholderSprite();
  try {
    return await loadPack(pack.info.id, pack.path, pack.manifest);
  } catch (error) {
    console.warn(`Sprite pack ${pack.info.id} lỗi, dùng pet tạm:`, error);
    return createPlaceholderSprite();
  }
}

/** Frame đầu của `idle` để làm ảnh nhỏ trong Settings, không nạp cả pack. */
export interface Thumbnail {
  image: HTMLImageElement;
  /** Frame đầu của `idle`. */
  frame: Rect;
  /** Mọi frame của `idle` (cùng ảnh `image`) và tốc độ, để vẽ nhân vật động mà chỉ nạp một ảnh. */
  frames: Rect[];
  fps: number;
  /** Điểm chân trong frame. */
  anchor: { x: number; y: number };
  pixelArt: boolean;
  facing: "left" | "right";
}

export async function loadThumbnail(id: string): Promise<Thumbnail> {
  const pack = packs.find((p) => p.info.id === id);
  if (!pack) throw new Error(`Không có sprite pack ${id}.`);
  const { manifest } = pack;
  const idle = manifest.animations.idle;
  const image = await decodeImage(dirOf(pack.path), idle.image);
  const frames = frameRects(idle, manifest.frameWidth, manifest.frameHeight, image.naturalWidth, image.naturalHeight);
  return {
    image,
    frame: frames[0],
    frames,
    fps: idle.fps,
    anchor: manifest.anchor,
    pixelArt: manifest.pixelArt,
    facing: manifest.facing,
  };
}

async function loadPack(id: string, manifestPath: string, manifest: SpriteManifest): Promise<SpriteSet> {
  const dir = dirOf(manifestPath);
  const images = new Map<string, Promise<HTMLImageElement>>();
  const loadImage = (file: string) => {
    let image = images.get(file);
    if (!image) {
      image = decodeImage(dir, file);
      images.set(file, image);
    }
    return image;
  };

  const animations: Partial<Record<AnimationName, Animation>> = {};
  for (const name of ANIMATION_NAMES) {
    const spec = manifest.animations[name];
    if (!spec) continue;
    const image = await loadImage(spec.image);
    const frames = frameRects(
      spec,
      manifest.frameWidth,
      manifest.frameHeight,
      image.naturalWidth,
      image.naturalHeight,
    );
    animations[name] = buildAnimation(image, frames, spec.fps, spec.loop);
  }
  const idle = animations.idle;
  if (!idle) throw new Error("Thiếu animation idle.");

  return {
    id,
    name: manifest.name,
    frameWidth: manifest.frameWidth,
    frameHeight: manifest.frameHeight,
    scale: manifest.scale,
    pixelArt: manifest.pixelArt,
    facing: manifest.facing,
    anchor: manifest.anchor,
    mouthY: manifest.mouthY,
    animations: withFallback({ ...animations, idle }),
  };
}

async function decodeImage(dir: string, file: string): Promise<HTMLImageElement> {
  const url = imageUrls.get((dir + file).toLowerCase());
  if (!url) throw new Error(`Không thấy file ${file} trong ${packName(dir)}.`);
  const image = new Image();
  image.src = url;
  await image.decode();
  return image;
}

function dirOf(path: string): string {
  return path.slice(0, path.lastIndexOf("/") + 1);
}

/** "../../../assets/sprites/a-momo/pet.json" -> "a-momo". */
function packName(path: string): string {
  return path.split("/sprites/")[1]?.split("/")[0] ?? path;
}
