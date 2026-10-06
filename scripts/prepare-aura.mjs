// Tách sprite sheet effect của Long (aura, biến hình, quật đuôi) ở assets/sprite-sources/long/ thành từng
// frame WebP trong assets/effects/ cho overlay (desktop/src/overlay/aura.ts, longAction.ts). Mỗi sheet có
// đúng 4 frame bằng nhau xếp ngang, nền trong suốt; mỗi frame được thu nhỏ về cao `HEIGHT`.
// Usage: node scripts/prepare-aura.mjs [--check]
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = join(root, "assets/sprite-sources/long");
const outDir = join(root, "assets/effects");
const FRAMES = 4;
/**
 * Cao mỗi frame (px). Cinematic vẽ cao gấp đôi Long, aura gấp 1,8 lần; Long cao 192 CSS px ở cỡ 200%,
 * nên 384 đủ nét ở mọi cỡ, mà ảnh giải nén nhẹ hơn khoảng 3,5 lần so với sheet nguồn (cao 724).
 */
const HEIGHT = 384;
/** Sheet nguồn và thư mục frame của nó (trong assets/effects/). */
export const SHEETS = [
  { source: "aura.png", output: "long-aura" },
  { source: "transform.png", output: "long-actions/transform" },
  { source: "tail-swipe.png", output: "long-actions/tail-swipe" },
];

/** Các frame WebP lossless của sheet `source`, từ trái sang phải. */
export async function prepareSheet(source) {
  const file = join(sourceDir, source);
  const { width, height, hasAlpha } = await sharp(file).metadata();
  assert.ok(width % FRAMES === 0, `${file}: phải là ${FRAMES} frame xếp ngang, bề ngang chia hết cho ${FRAMES}`);
  assert.ok(hasAlpha, `${file}: phải có nền trong suốt (alpha)`);
  const frameWidth = width / FRAMES;
  return Promise.all(
    Array.from({ length: FRAMES }, (_, i) =>
      sharp(file)
        .extract({ left: i * frameWidth, top: 0, width: frameWidth, height })
        .resize({ height: HEIGHT, kernel: "lanczos3" })
        .webp({ lossless: true, effort: 6 })
        .toBuffer(),
    ),
  );
}

async function main() {
  const check = process.argv.includes("--check");
  let count = 0;
  for (const { source, output } of SHEETS) {
    const frames = await prepareSheet(source);
    const dir = join(outDir, output);
    for (const [i, webp] of frames.entries()) {
      const out = join(dir, `frame-${i}.webp`);
      if (check) {
        assert.ok(existsSync(out), `${out}: chưa tạo, chạy node scripts/prepare-aura.mjs`);
        const [want, have] = await Promise.all([
          sharp(webp).ensureAlpha().raw().toBuffer(),
          sharp(readFileSync(out)).ensureAlpha().raw().toBuffer(),
        ]);
        assert.ok(want.equals(have), `${out}: khác ảnh nguồn, chạy lại node scripts/prepare-aura.mjs`);
      } else {
        mkdirSync(dir, { recursive: true });
        writeFileSync(out, webp);
      }
      count++;
    }
  }
  console.log(check ? `effects: ${count} frame khớp ảnh nguồn` : `effects: đã ghi ${count} frame vào assets/effects/`);
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
