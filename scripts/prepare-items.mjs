// Chuẩn hoá ảnh đồ đội đầu (mũ của lịch sự kiện) từ assets/sprite-sources/items/<tên>.png thành
// assets/items/<tên>.webp và assets/items/items.json cho overlay (desktop/src/overlay/hats.ts).
// Ảnh nguồn tạo bằng imagegen, cỡ bất kỳ, nền trong suốt, vẽ cho nhân vật quay sang phải. Script cắt
// sát phần có hình, thu nhỏ về bề ngang ghi trong ITEMS (pixel của frame nhân vật 192×192), làm alpha
// 0 hoặc 255 như sprite pack, rồi tìm điểm đặt: giữa vành mũ ở mép dưới.
// Usage: node scripts/prepare-items.mjs [--check]
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const sourceDir = join(root, "assets/sprite-sources/items");
const outDir = join(root, "assets/items");

/** Bề ngang mỗi món khi vẽ (pixel của frame 192): đầu nhân vật rộng chừng 70–110. */
export const ITEMS = {
  party: { width: 56 },
  noel: { width: 84 },
  tet: { width: 70 },
};

/** Pixel có độ đục từ chừng này trở lên là có hình (cả lúc cắt sát và lúc làm alpha 0/255). */
const ALPHA_CUT = 128;
/** Điểm đặt tính theo các hàng có hình ở chừng này phần dưới cùng của món đồ (vành mũ). */
const BRIM_SHARE = 0.1;

function bbox(data, width, height, cut) {
  let left = width, right = -1, top = height, bottom = -1;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (data[(y * width + x) * 4 + 3] < cut) continue;
      left = Math.min(left, x);
      right = Math.max(right, x);
      top = Math.min(top, y);
      bottom = Math.max(bottom, y);
    }
  }
  assert.ok(right >= 0, "ảnh không có phần nào có hình");
  return { left, top, width: right - left + 1, height: bottom - top + 1 };
}

/** Ảnh WebP lossless và mô tả của một món. */
export async function prepareItem(name, file = join(sourceDir, `${name}.png`)) {
  const spec = ITEMS[name];
  const source = await sharp(file).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const box = bbox(source.data, source.info.width, source.info.height, 32);
  const { data, info } = await sharp(file)
    .ensureAlpha()
    .extract(box)
    .resize({ width: spec.width, kernel: "lanczos3" })
    .raw()
    .toBuffer({ resolveWithObject: true });
  const { width, height } = info;
  // Alpha 0 hoặc 255 như sprite pack: vẽ kiểu pixel art cùng nhân vật mà không bị viền mờ.
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] >= ALPHA_CUT) data[i + 3] = 255;
    else data.fill(0, i, i + 4);
  }
  const solid = bbox(data, width, height, ALPHA_CUT);
  const bottom = solid.top + solid.height;
  const band = Math.max(2, Math.round(solid.height * BRIM_SHARE));
  let min = width, max = -1;
  for (let y = bottom - band; y < bottom; y++) {
    for (let x = 0; x < width; x++) {
      if (data[(y * width + x) * 4 + 3] === 0) continue;
      min = Math.min(min, x);
      max = Math.max(max, x);
    }
  }
  const webp = await sharp(data, { raw: { width, height, channels: 4 } }).webp({ lossless: true, effort: 6 }).toBuffer();
  return {
    webp,
    meta: { file: `${name}.webp`, width, height, anchor: { x: (min + max + 1) / 2, y: bottom } },
  };
}

async function main() {
  const check = process.argv.includes("--check");
  const manifest = {};
  for (const name of Object.keys(ITEMS)) {
    const { webp, meta } = await prepareItem(name);
    manifest[name] = meta;
    const out = join(outDir, meta.file);
    if (check) {
      assert.ok(existsSync(out), `${out}: chưa tạo, chạy node scripts/prepare-items.mjs`);
      const [want, have] = await Promise.all([
        sharp(webp).raw().toBuffer(),
        sharp(readFileSync(out)).ensureAlpha().raw().toBuffer(),
      ]);
      assert.ok(want.equals(have), `${out}: khác ảnh nguồn, chạy lại node scripts/prepare-items.mjs`);
    } else {
      mkdirSync(outDir, { recursive: true });
      writeFileSync(out, webp);
    }
  }
  const json = `${JSON.stringify(manifest, null, 2)}\n`;
  const jsonPath = join(outDir, "items.json");
  if (check) {
    assert.equal(readFileSync(jsonPath, "utf8").replace(/\r\n/g, "\n"), json, `${jsonPath}: cũ, chạy lại script`);
    console.log(`items: ${Object.keys(ITEMS).length} món khớp ảnh nguồn`);
  } else {
    writeFileSync(jsonPath, json);
    console.log(`items: đã ghi ${Object.keys(ITEMS).length} món vào assets/items/`);
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
