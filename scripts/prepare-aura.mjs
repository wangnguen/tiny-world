import { mkdir, stat } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import sharp from "sharp";

const SOURCE = "assets/sprite-sources/long/aura.png";
const OUTPUT = "assets/effects/long-aura";
const FRAMES = 4;
const ACTIONS = [
  ["assets/sprite-sources/long/transform.png", "assets/effects/long-actions/transform"],
  ["assets/sprite-sources/long/tail-swipe.png", "assets/effects/long-actions/tail-swipe"],
];

/**
 * Tách sprite sheet aura của Long thành từng frame WebP để overlay chỉ cần nạp
 * frame hiện tại. Sheet nguồn phải có đúng bốn cột bằng nhau và giữ alpha.
 */
export async function prepareAura(source = SOURCE, output = OUTPUT) {
  const image = sharp(source, { animated: false });
  const { width, height, hasAlpha } = await image.metadata();
  if (!width || !height || width % FRAMES !== 0) {
    throw new Error(`${source}: aura phải là sprite sheet ${FRAMES} cột có bề ngang chia hết cho ${FRAMES}.`);
  }
  if (!hasAlpha) throw new Error(`${source}: aura phải có nền trong suốt (alpha).`);

  const frameWidth = width / FRAMES;
  await mkdir(output, { recursive: true });
  await Promise.all(
    Array.from({ length: FRAMES }, (_, index) =>
      image
        .clone()
        .extract({ left: index * frameWidth, top: 0, width: frameWidth, height })
        .webp({ lossless: true })
        .toFile(resolve(output, `frame-${index}.webp`)),
    ),
  );
}

/** Tách hai sheet cinematic (biến hình và quật đuôi) của Long. */
export async function prepareLongActions() {
  for (const [source, output] of ACTIONS) await prepareAura(source, output);
}

if (import.meta.main) {
  await stat(SOURCE);
  await prepareAura();
  await Promise.all(ACTIONS.map(([source]) => stat(source)));
  await prepareLongActions();
}
