// Replace one pose row of a pet's source sheet with the second row of a smaller generated sheet,
// so one bad animation can be redrawn without redrawing (and re-checking) the whole sheet.
// The patch sheet has 4 columns and 2 rows: a `ref` row (the standing idle pose) and the new row.
// The new poses are scaled by the ratio of the two `ref` rows, centred on the old poses and
// bottom-aligned on the old row's baseline; every other row keeps its exact pixels.
// Usage: node scripts/replace-row.mjs <sheet.png> <row index> <patch.png>
//   e.g. node scripts/replace-row.mjs assets/sprite-sources/lu/climb-perch-jump.png 1 climb.png
import sharp from "sharp";

const [sheetPath, rowArg, patchPath] = process.argv.slice(2);
const ROW = Number(rowArg);
if (!sheetPath || !patchPath || !Number.isInteger(ROW) || ROW < 1) {
  console.error("usage: node scripts/replace-row.mjs <sheet.png> <row index, 1-3> <patch.png>");
  process.exit(1);
}

async function load(path) {
  const { data, info } = await sharp(path).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  return { w: info.width, h: info.height, px: data };
}
const solid = (img, x, y) => img.px[(y * img.w + x) * 4 + 3] >= 128;

// Pose bands along one axis, separated by transparent gaps (same rule as prepare-sprites.mjs).
function bands(count, length, solidCount, min) {
  const out = [];
  let start = -1;
  for (let i = 0; i <= length; i++) {
    if (i < length && solidCount(i) >= 3) {
      if (start < 0) start = i;
    } else if (start >= 0) {
      if (i - start >= min) out.push([start, i - 1]);
      start = -1;
    }
  }
  if (out.length !== count) throw new Error(`expected ${count} pose bands, found ${out.length}`);
  return out;
}
const rowBands = (img, n) => bands(n, img.h, (y) => {
  let c = 0;
  for (let x = 0; x < img.w; x++) c += solid(img, x, y);
  return c;
}, img.w / 16);
const columnBands = (img, top, bottom) => bands(4, img.w, (x) => {
  let c = 0;
  for (let y = top; y <= bottom; y++) c += solid(img, x, y);
  return c;
}, img.w / 16);
const cuts = (found, length) => [0, ...found.slice(1).map((band, i) => Math.floor((found[i][1] + band[0] + 1) / 2)), length];

// One pose: binary alpha, the largest 8-connected part plus detached parts that are not specks.
function pose(img, x0, x1, y0, y1) {
  const w = x1 - x0, h = y1 - y0, px = Buffer.alloc(w * h * 4);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const from = ((y0 + y) * img.w + x0 + x) * 4, to = (y * w + x) * 4;
      if (img.px[from + 3] >= 128) {
        img.px.copy(px, to, from, from + 3);
        px[to + 3] = 255;
      }
    }
  }
  const seen = new Uint8Array(w * h), parts = [];
  let largest = [];
  for (let i = 0; i < w * h; i++) {
    if (seen[i] || !px[i * 4 + 3]) continue;
    const part = [i];
    seen[i] = 1;
    for (let p = 0; p < part.length; p++) {
      const x = part[p] % w, y = Math.floor(part[p] / w);
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const nx = x + dx, ny = y + dy, n = ny * w + nx;
          if (nx < 0 || ny < 0 || nx >= w || ny >= h || seen[n] || !px[n * 4 + 3]) continue;
          seen[n] = 1;
          part.push(n);
        }
      }
    }
    parts.push(part);
    if (part.length > largest.length) largest = part;
  }
  const minimum = Math.max(4, Math.round(w * h / 4096), largest.length * 0.002);
  const keep = parts.filter((part) => part === largest || part.length >= minimum).flat();
  let left = w, top = h, right = -1, bottom = -1;
  for (const i of keep) {
    const x = i % w, y = Math.floor(i / w);
    left = Math.min(left, x); right = Math.max(right, x); top = Math.min(top, y); bottom = Math.max(bottom, y);
  }
  const cw = right - left + 1, ch = bottom - top + 1, out = Buffer.alloc(cw * ch * 4);
  for (const i of keep) {
    const x = i % w - left, y = Math.floor(i / w) - top;
    px.copy(out, (y * cw + x) * 4, i * 4, i * 4 + 4);
  }
  return { w: cw, h: ch, px: out, x: x0 + left, y: y0 + top };
}
function rowPoses(img, found, row) {
  const rows = cuts(found, img.h);
  const columns = cuts(columnBands(img, found[row][0], found[row][1]), img.w);
  return { rows, columns, poses: [0, 1, 2, 3].map((c) => pose(img, columns[c], columns[c + 1], rows[row], rows[row + 1])) };
}
const median = (values) => [...values].sort((a, b) => a - b)[2];

const sheet = await load(sheetPath), patch = await load(patchPath);
const corners = [[2, 2], [patch.w - 3, 2], [2, patch.h - 3], [patch.w - 3, patch.h - 3]]
  .map(([x, y]) => patch.px[(y * patch.w + x) * 4 + 3]);
if (corners.some((alpha) => alpha > 64)) throw new Error(`patch background is not transparent (corner alpha ${corners})`);
const sheetRows = rowBands(sheet, 4), patchRows = rowBands(patch, 2);
const factor = median(rowPoses(sheet, sheetRows, 0).poses.map((p) => p.h)) / median(rowPoses(patch, patchRows, 0).poses.map((p) => p.h));
const target = rowPoses(sheet, sheetRows, ROW), fresh = rowPoses(patch, patchRows, 1).poses;
const top = target.rows[ROW], bottom = target.rows[ROW + 1];
const baseline = Math.max(...target.poses.map((p) => p.y + p.h - 1));
const out = Buffer.from(sheet.px);
out.fill(0, top * sheet.w * 4, bottom * sheet.w * 4);
for (let c = 0; c < 4; c++) {
  const p = fresh[c], w = Math.max(1, Math.round(p.w * factor)), h = Math.max(1, Math.round(p.h * factor));
  const scaled = await sharp(p.px, { raw: { width: p.w, height: p.h, channels: 4 } }).resize(w, h, { kernel: "nearest" }).raw().toBuffer();
  const old = target.poses[c], left = target.columns[c] + 6, right = target.columns[c + 1] - 6;
  if (w > right - left) throw new Error(`frame ${c}: new pose is ${w}px wide, the cell has ${right - left}px`);
  const x = Math.min(Math.max(Math.round(old.x + old.w / 2 - w / 2), left), right - w);
  const y = Math.min(baseline, bottom - 7) - h + 1;
  if (y < top + 6) throw new Error(`frame ${c}: new pose is ${h}px tall, the row has ${bottom - top - 12}px`);
  for (let j = 0; j < h; j++) {
    for (let i = 0; i < w; i++) {
      const s = (j * w + i) * 4;
      if (scaled[s + 3] < 128) continue;
      const d = ((y + j) * sheet.w + x + i) * 4;
      scaled.copy(out, d, s, s + 3);
      out[d + 3] = 255;
    }
  }
}
await sharp(out, { raw: { width: sheet.w, height: sheet.h, channels: 4 } }).png().toFile(sheetPath);
console.log(`${sheetPath}: row ${ROW} replaced, poses scaled ×${factor.toFixed(3)}`);
