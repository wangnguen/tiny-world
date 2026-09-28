import { ANIMATION_NAMES, frameRects, parseSpriteManifest, type AnimationName } from "@tinyworld/core";
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

/** Pack đầu tiên theo tên thư mục. Chưa có pack hoặc pack lỗi thì dùng pet tạm. */
export async function loadSpriteSet(): Promise<SpriteSet> {
  const [path] = Object.keys(manifests).sort();
  if (path === undefined) return createPlaceholderSprite();
  try {
    return await loadPack(path);
  } catch (error) {
    console.warn(`Sprite pack ${packName(path)} lỗi, dùng pet tạm:`, error);
    return createPlaceholderSprite();
  }
}

async function loadPack(manifestPath: string): Promise<SpriteSet> {
  const manifest = parseSpriteManifest(manifests[manifestPath]);
  const dir = manifestPath.slice(0, manifestPath.lastIndexOf("/") + 1);
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
    animations[name] = buildAnimation(image, frames, spec.fps, spec.loop, manifest.outline);
  }
  const idle = animations.idle;
  if (!idle) throw new Error("Thiếu animation idle.");

  return {
    name: manifest.name,
    frameWidth: manifest.frameWidth,
    frameHeight: manifest.frameHeight,
    scale: manifest.scale,
    pixelArt: manifest.pixelArt,
    facing: manifest.facing,
    anchor: manifest.anchor,
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

/** "../../../assets/sprites/cat/pet.json" -> "cat". */
function packName(path: string): string {
  return path.split("/sprites/")[1]?.split("/")[0] ?? path;
}
