// Normalize original atlases into detailed, bottom-aligned 192px sprite frames.
// No image library or service is needed. Originals stay in assets/sprite-sources.
// Usage: node scripts/prepare-sprites.mjs [--check] [--only-new] [--pet=source]
import assert from "node:assert/strict";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync, inflateSync } from "node:zlib";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const SOURCES = join(ROOT, "assets/sprite-sources");
const PACKS = join(ROOT, "assets/sprites");
// At 200% each source pixel maps to one CSS pixel. Sample the original atlas,
// never upscale the reduced runtime strips. Keep the default 96 CSS px size.
const SIZE = 192;
const BASELINE = 179;
// Largest pose allowed in a frame. Atlas poses that fit are copied 1:1: any
// fractional nearest-neighbor scale drops rows, breaking thin outlines/limbs.
const MAX_WIDTH = SIZE - 6;
const MAX_HEIGHT = BASELINE - 2;
// Outline, pupil and mouth colours. Resampling favours them so lines survive.
const DARK = 90;
const DISPLAY_SIZE = 96;
const COLUMNS = 4;
const ROWS = [
  ["idle", 5], ["walk", 8], ["run", 12], ["sleep", 3],
  ["react", 8, false], ["fall", 8], ["dragged", 6],
  ["land", 16, false], ["dizzy", 5],
  ["climb", 8], ["perch", 4], ["jump", 10, false],
];
// Per-pet options: `atlas` source file; `locomotion` / `poses` (one sheet or a
// list; 4 columns, one row per name in `rows`, `"ref"` = idle copy for size
// only) / `overrides` imagegen sheets that redraw rows; `keep` rows that stay from the atlas even
// when a sheet redraws them; `dizzy` the source column of the one dizzy pose
// the app shows; `reuse` a row's column order, replaying good poses in place of
// broken ones; `recolor` palette swaps within some rows, optionally only some
// `frames` and a `box` [left, top, right, bottom] in frame pixels; `fill` paints
// the region enclosed by outline around `at` in one frame with one colour.
// Recolor/fill colours resolve to the nearest pack colour, so a rule survives the
// small palette shifts a regenerated sheet causes.
const PETS = [
  {
    source: "momo", folder: "a-momo", name: "Momo — Axolotl",
    poses: [{ image: "poses-v3.png", rows: ["idle", "fall", "land", "dizzy"] }, { image: "fix-v1.png", rows: ["ref", "react", "dragged"] }, { image: "fix-v2.png", rows: ["ref", "dragged", "dizzy", "fall"] }],
    palette: "pose-palette.json",
    // Walk frame 2's far hind leg is gill coral and its lower gill is only an outline.
    recolor: { rows: ["walk"], frames: [2], box: [44, 154, 70, 179], colors: { "#fd6159": "#bf646c", "#c53f41": "#883741" } },
    fill: [{ row: "walk", frame: 2, at: [88, 131], box: [83, 121, 109, 141], color: "#fd6159" }],
  },
  {
    source: "bong", folder: "b-bong", name: "Bông — Bunny", dizzy: 1,
    poses: [{ image: "fix-v1.png", rows: ["ref", "fall"] }, { image: "fix-v2.png", rows: ["ref", "land"] }],
    // The redrawn far legs are tan; both legs are cream in the design.
    recolor: { rows: ["idle", "walk", "run", "fall"], colors: { "#b4967d": "#ebc6a3", "#d6a584": "#fbd5b7" } },
  },
  { source: "kitsu", folder: "b-kitsu", name: "Kitsu — Fox", poses: { image: "fix-v1.png", rows: ["ref", "walk", "run", "fall"] } },
  { source: "mam", folder: "c-mam", name: "Mầm — Sprout", poses: [{ image: "fix-v1.png", rows: ["ref", "fall"] }, { image: "fix-v2.png", rows: ["ref", "react", "dragged"] }], reuse: { climb: [2, 1, 2, 3] } },
  { source: "bip", folder: "c-bip", name: "Bíp — Robot", poses: [{ image: "fix-v1.png", rows: ["ref", "dragged"] }, { image: "fix-v2.png", rows: ["ref", "climb", "perch"] }] },
  { source: "lumi", folder: "c-lumi", name: "Lumi — Star Spirit", poses: [{ image: "fix-v1.png", rows: ["ref", "fall"] }, { image: "fix-v2.png", rows: ["ref", "climb"] }], reuse: { dragged: [0, 1, 0, 3] } },
  { source: "nam", folder: "c-nam", name: "Nấm — Mushroom", poses: [{ image: "fix-v1.png", rows: ["ref", "sleep", "fall", "dragged"] }, { image: "fix-v2.png", rows: ["ref", "climb"] }] },
  { source: "may", folder: "c-may", name: "Mây — Cloud", poses: { image: "fix-v3.png", rows: ["ref", "dragged", "climb"] }, reuse: { fall: [0, 1, 0, 3] } },
  { source: "tan", folder: "c-tan", name: "Tàn — Ember", poses: [{ image: "fix-v1.png", rows: ["ref", "fall"] }, { image: "fix-v2.png", rows: ["ref", "run"] }], reuse: { climb: [0, 3, 2, 3] } },
  { source: "reu", folder: "c-reu", name: "Rêu — Leaf Dragon", poses: [{ image: "fix-v1.png", rows: ["ref", "fall"] }, { image: "fix-v2.png", rows: ["ref", "land", "dragged"] }], reuse: { idle: [0, 1, 3, 3], fall: [0, 3, 0, 3] } },
  { source: "cuc", folder: "c-cuc", name: "Cục — Pebble", atlas: "atlas-v2.png", poses: [{ image: "fix-v1.png", rows: ["ref", "fall", "dragged"] }, { image: "fix-v2.png", rows: ["ref", "land"] }] },
  // The redrawn walk/run lose one or two of Mực's five tentacles; keep the atlas gait.
  { source: "muc", folder: "c-muc", name: "Mực — Octopus", overrides: { dragged: "dragged-v2.png" }, keep: ["walk", "run"], poses: [{ image: "fix-v1.png", rows: ["ref", "idle", "fall"] }, { image: "fix-v2.png", rows: ["ref", "fall", "climb"] }] },
  {
    source: "dua", folder: "c-dua", name: "Dứa — Pineapple", poses: [{ image: "fix-v1.png", rows: ["ref", "fall"] }, { image: "fix-v2.png", rows: ["ref", "react"] }], reuse: { climb: [0, 0, 2, 2] },
  },
  { source: "su", folder: "c-su", name: "Su — Astronaut", dizzy: 1, poses: [{ image: "fix-v1.png", rows: ["ref", "fall"] }, { image: "fix-v2.png", rows: ["ref", "dizzy", "react"] }], reuse: { dragged: [0, 1, 2, 1] } },
  { source: "bap", folder: "c-bap", name: "Bắp — Bumblebee", poses: [{ image: "fix-v1.png", rows: ["ref", "fall", "dragged", "dizzy"] }, { image: "fix-v2.png", rows: ["ref", "jump"] }] },
  { source: "boggo", folder: "c-boggo", name: "Boggo — Coder Frog", poses: [{ image: "fix-v1.png", rows: ["ref", "sleep", "fall", "dizzy"] }, { image: "fix-v2.png", rows: ["ref", "react", "climb"] }] },
  { source: "gloop", folder: "c-gloop", name: "Gloop — Chaos Frog", poses: { image: "fix-v2.png", rows: ["ref", "land", "react"] } },
  { source: "bep", folder: "c-bep", name: "Bẹp — Grumpy Toad", poses: [{ image: "fix-v1.png", rows: ["ref", "sleep"] }, { image: "fix-v2.png", rows: ["ref", "react"] }], reuse: { fall: [0, 1, 0, 3] } },
  {
    source: "frobu", folder: "c-frobu", name: "Frobu — Night Frog", poses: { image: "fix-v1.png", rows: ["ref", "idle", "walk", "run", "land"] },
    // A dark eye-like blot on the forehead in react frame 2.
    // Paint it over with the head's shading: darker under the headphone band, lighter below.
    recolor: [
      { rows: ["react"], frames: [2], box: [111, 70, 122, 74], colors: { "#010202": "#8ca57d", "#0c0f0f": "#8ca57d", "#202e3f": "#8ca57d", "#464c3b": "#8ca57d", "#5a635e": "#8ca57d" } },
      { rows: ["react"], frames: [2], box: [111, 75, 123, 81], colors: { "#010202": "#98b890", "#0c0f0f": "#98b890", "#202e3f": "#98b890", "#464c3b": "#98b890", "#5a635e": "#98b890", "#8ca57d": "#98b890" } },
    ],
  },
  { source: "byte", folder: "c-byte", name: "Byte — Coder Penguin", singleSheet: true, poses: { image: "fix-v1.png", rows: ["ref", "idle", "walk", "sleep", "fall"] }, reuse: { fall: [2, 1, 2, 3] } },
  { source: "patch", folder: "c-patch", name: "Patch — Coder Red Panda", singleSheet: true, dizzy: 1, poses: [{ image: "fix-v1.png", rows: ["ref", "idle"] }, { image: "fix-v2.png", rows: ["ref", "run"] }], reuse: { dragged: [0, 2, 0, 2] } },
].map((pet) => ({ ...pet, locomotion: "locomotion-v2.png" }));

// RGBA PNG decoding, including all five PNG scanline filters. Generated input
// is 8-bit, non-interlaced RGBA; reject other encodings instead of guessing.
function decodePng(path) {
  const png = readFileSync(path);
  assert.equal(png.subarray(0, 8).toString("hex"), "89504e470d0a1a0a", `${path}: PNG signature`);
  let width, height;
  const compressed = [];
  for (let p = 8; p < png.length;) {
    const length = png.readUInt32BE(p);
    const type = png.toString("ascii", p + 4, p + 8);
    const data = png.subarray(p + 8, p + 8 + length);
    if (type === "IHDR") {
      width = data.readUInt32BE(0);
      height = data.readUInt32BE(4);
      assert.equal(data[8], 8, `${path}: bit depth`);
      assert.equal(data[9], 6, `${path}: RGBA required`);
      assert.equal(data[12], 0, `${path}: interlace unsupported`);
    }
    if (type === "IDAT") compressed.push(data);
    p += length + 12;
  }
  const stride = width * 4;
  const scanlines = inflateSync(Buffer.concat(compressed));
  assert.equal(scanlines.length, (stride + 1) * height);
  const pixels = Buffer.alloc(stride * height);
  for (let y = 0; y < height; y++) {
    const start = y * (stride + 1);
    const filter = scanlines[start];
    assert.ok(filter <= 4, `${path}: scanline filter`);
    for (let x = 0; x < stride; x++) {
      const left = x >= 4 ? pixels[y * stride + x - 4] : 0;
      const up = y > 0 ? pixels[(y - 1) * stride + x] : 0;
      const corner = y > 0 && x >= 4 ? pixels[(y - 1) * stride + x - 4] : 0;
      const prediction = [0, left, up, Math.floor((left + up) / 2), paeth(left, up, corner)][filter];
      pixels[y * stride + x] = (scanlines[start + x + 1] + prediction) & 255;
    }
  }
  return { width, height, pixels };
}

function paeth(a, b, c) {
  const p = a + b - c;
  const da = Math.abs(p - a), db = Math.abs(p - b), dc = Math.abs(p - c);
  return da <= db && da <= dc ? a : db <= dc ? b : c;
}

function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const name = Buffer.from(type);
  const size = Buffer.alloc(4), crc = Buffer.alloc(4);
  size.writeUInt32BE(data.length);
  crc.writeUInt32BE(crc32(Buffer.concat([name, data])));
  return Buffer.concat([size, name, data, crc]);
}

function savePng(path, image) {
  const { width, height, pixels } = image;
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 6;
  const scanlines = Buffer.alloc(height * (width * 4 + 1));
  for (let y = 0; y < height; y++) {
    pixels.copy(scanlines, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  }
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, Buffer.concat([
    Buffer.from("89504e470d0a1a0a", "hex"), chunk("IHDR", header),
    chunk("IDAT", deflateSync(scanlines, { level: 9 })), chunk("IEND", Buffer.alloc(0)),
  ]));
}

function emptyImage(width, height) {
  return { width, height, pixels: Buffer.alloc(width * height * 4) };
}

// Generated atlases have four columns, but their pose rows are not necessarily
// evenly spaced. Find the transparent gaps before cutting, or a later row can
// contain the previous pose's feet and lose its own head.
function rowBoundaries(atlas, rowCount = ROWS.length, columnCount = COLUMNS) {
  const bands = [];
  let start = -1;
  for (let y = 0; y <= atlas.height; y++) {
    let solid = 0;
    if (y < atlas.height) {
      for (let x = 0; x < atlas.width; x++) {
        if (atlas.pixels[(y * atlas.width + x) * 4 + 3] >= 128) solid++;
      }
    }
    if (solid >= 3) {
      if (start < 0) start = y;
    } else if (start >= 0) {
      // Isolated speckles must not become extra animation rows. Crop boundaries
      // go in the gaps, rather than at the bands, to retain thin toes/antennae.
      if (y - start >= atlas.width / columnCount / 4) bands.push({ top: start, bottom: y - 1 });
      start = -1;
    }
  }
  assert.equal(bands.length, rowCount, `atlas: expected ${rowCount} distinct pose rows, found ${bands.length}`);
  return [0, ...bands.slice(1).map((band, i) => Math.floor((bands[i].bottom + band.top + 1) / 2)), atlas.height];
}

// A wide tail or extended hand can also cross an evenly spaced column boundary.
// Find each row's horizontal gaps separately, including those edge poses.
function columnBoundaries(atlas, top, bottom, columnCount = COLUMNS) {
  const bands = [];
  let start = -1;
  for (let x = 0; x <= atlas.width; x++) {
    let solid = 0;
    if (x < atlas.width) {
      for (let y = top; y < bottom; y++) {
        if (atlas.pixels[(y * atlas.width + x) * 4 + 3] >= 128) solid++;
      }
    }
    if (solid >= 3) {
      if (start < 0) start = x;
    } else if (start >= 0) {
      if (x - start >= atlas.width / columnCount / 4) bands.push({ left: start, right: x - 1 });
      start = -1;
    }
  }
  assert.equal(bands.length, columnCount, `atlas: expected ${columnCount} poses in row, found ${bands.length}`);
  return [0, ...bands.slice(1).map((band, i) => Math.floor((bands[i].right + band.left + 1) / 2)), atlas.width];
}

// Ignore near-transparent fringes and tiny detached noise, while preserving
// meaningful disconnected parts (ears, hands, antennae) as well as the body.
function extractCell(atlas, column, row, boundaries = rowBoundaries(atlas), columns = columnBoundaries(atlas, boundaries[row], boundaries[row + 1])) {
  const width = columns[column + 1] - columns[column];
  const height = boundaries[row + 1] - boundaries[row];
  const pixels = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const from = ((boundaries[row] + y) * atlas.width + columns[column] + x) * 4;
      const to = (y * width + x) * 4;
      if (atlas.pixels[from + 3] >= 128) {
        atlas.pixels.copy(pixels, to, from, from + 3);
        pixels[to + 3] = 255;
      }
    }
  }
  const visited = new Uint8Array(width * height);
  const components = [];
  let largest = [];
  for (let i = 0; i < visited.length; i++) {
    if (visited[i] || !pixels[i * 4 + 3]) continue;
    const connected = [i];
    visited[i] = 1;
    for (let p = 0; p < connected.length; p++) {
      const x = connected[p] % width, y = Math.floor(connected[p] / width);
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          const nx = x + dx, ny = y + dy, n = ny * width + nx;
          if (nx < 0 || ny < 0 || nx >= width || ny >= height || visited[n] || !pixels[n * 4 + 3]) continue;
          visited[n] = 1;
          connected.push(n);
        }
      }
    }
    components.push(connected);
    if (connected.length > largest.length) largest = connected;
  }
  assert.ok(largest.length > atlas.width / (columns.length - 1), `row ${row}, column ${column}: empty or invalid pose`);
  const clean = Buffer.alloc(pixels.length);
  const minimum = Math.max(4, Math.round(width * height / 4096), largest.length * 0.002);
  for (const component of components) {
    if (component !== largest && component.length < minimum) continue;
    for (const i of component) pixels.copy(clean, i * 4, i * 4, i * 4 + 4);
  }
  const image = { width, height, pixels: clean };
  const bounds = boundsOf(image);
  assert.ok(bounds.left > 0 && bounds.right < width - 1 && bounds.top > 0 && bounds.bottom < height - 1,
    `row ${row}, column ${column}: source pose touches crop boundary`);
  return { ...image, bounds };
}

function boundsOf(image) {
  let left = image.width, top = image.height, right = -1, bottom = -1;
  for (let y = 0; y < image.height; y++) {
    for (let x = 0; x < image.width; x++) {
      if (!image.pixels[(y * image.width + x) * 4 + 3]) continue;
      left = Math.min(left, x); right = Math.max(right, x);
      top = Math.min(top, y); bottom = Math.max(bottom, y);
    }
  }
  return { left, top, right, bottom, width: right - left + 1, height: bottom - top + 1 };
}

// Area resampling of a pose crop. At factor 1 it is an exact copy. Otherwise a
// pixel is opaque when at least half of its source footprint is, and takes the
// footprint's dominant colour; dark line colours count double, so 1px outlines,
// pupils and limb edges survive instead of vanishing between sampled rows.
function resampleCell(cell, factor) {
  const b = cell.bounds;
  const width = Math.max(1, Math.round(b.width * factor));
  const height = Math.max(1, Math.round(b.height * factor));
  const out = emptyImage(width, height);
  const fx = b.width / width, fy = b.height / height;
  for (let y = 0; y < height; y++) {
    const y0 = b.top + y * fy, y1 = y0 + fy;
    for (let x = 0; x < width; x++) {
      const x0 = b.left + x * fx, x1 = x0 + fx;
      const groups = new Map();
      let area = 0, covered = 0;
      for (let sy = Math.floor(y0); sy < Math.ceil(y1 - 1e-9); sy++) {
        const wy = Math.min(y1, sy + 1) - Math.max(y0, sy);
        for (let sx = Math.floor(x0); sx < Math.ceil(x1 - 1e-9); sx++) {
          const w = (Math.min(x1, sx + 1) - Math.max(x0, sx)) * wy;
          if (w <= 0) continue;
          area += w;
          const p = (sy * cell.width + sx) * 4;
          if (!cell.pixels[p + 3]) continue;
          covered += w;
          const [r, g, bl] = cell.pixels.subarray(p, p + 3);
          const key = (r >> 4) << 8 | (g >> 4) << 4 | bl >> 4;
          let group = groups.get(key);
          if (!group) groups.set(key, group = { w: 0, r: 0, g: 0, b: 0, dark: luma(r, g, bl) < DARK });
          group.w += w; group.r += r * w; group.g += g * w; group.b += bl * w;
        }
      }
      if (covered * 2 < area) continue;
      let best, score = -1;
      for (const group of groups.values()) {
        const s = group.w * (group.dark ? 2 : 1);
        if (s > score) { score = s; best = group; }
      }
      out.pixels.set([best.r / best.w, best.g / best.w, best.b / best.w, 255].map(Math.round), (y * width + x) * 4);
    }
  }
  return { ...out, bounds: boundsOf(out) };
}

function luma(r, g, b) {
  return 0.299 * r + 0.587 * g + 0.114 * b;
}

// Centre a resampled pose in the frame with its lowest pixel on the baseline.
function normalizeFrame(pose) {
  const { bounds: b } = pose;
  assert.ok(b.width <= MAX_WIDTH && b.height <= MAX_HEIGHT, "normalized pose exceeds frame; source scale must be corrected");
  const frame = emptyImage(SIZE, SIZE);
  blit(frame, pose, Math.round((SIZE - b.width) / 2) - b.left, BASELINE - b.bottom);
  return frame;
}

function blit(target, source, x, y) {
  for (let sy = 0; sy < source.height; sy++) {
    for (let sx = 0; sx < source.width; sx++) {
      const tx = x + sx, ty = y + sy;
      if (tx < 0 || ty < 0 || tx >= target.width || ty >= target.height) continue;
      const from = (sy * source.width + sx) * 4;
      if (source.pixels[from + 3]) source.pixels.copy(target.pixels, (ty * target.width + tx) * 4, from, from + 4);
    }
  }
}

// Weighted median-cut palette shared by every frame in one pack. Quantization
// removes generation's almost-identical shades without dithering or smoothing.
function quantize(frames, count = 24) {
  const histogram = new Map();
  for (const frame of frames) {
    for (let p = 0; p < frame.pixels.length; p += 4) {
      if (!frame.pixels[p + 3]) continue;
      const rgb = [0, 1, 2].map((n) => Math.round(frame.pixels[p + n] / 8) * 8);
      const key = rgb.join(",");
      const color = histogram.get(key) ?? { rgb, weight: 0 };
      color.weight++;
      histogram.set(key, color);
    }
  }
  const measure = (colors) => {
    const ranges = [0, 1, 2].map((c) => Math.max(...colors.map((v) => v.rgb[c])) - Math.min(...colors.map((v) => v.rgb[c])));
    return { colors, ranges, weight: colors.reduce((s, c) => s + c.weight, 0) };
  };
  const boxes = [measure([...histogram.values()])];
  while (boxes.length < count) {
    boxes.sort((a, b) => Math.max(...b.ranges) * Math.sqrt(b.weight) - Math.max(...a.ranges) * Math.sqrt(a.weight));
    const index = boxes.findIndex((b) => b.colors.length > 1);
    if (index === -1) break;
    const box = boxes.splice(index, 1)[0];
    const axis = box.ranges.indexOf(Math.max(...box.ranges));
    box.colors.sort((a, b) => a.rgb[axis] - b.rgb[axis]);
    let sum = 0, split = 1;
    for (let i = 0; i < box.colors.length - 1; i++) {
      sum += box.colors[i].weight; split = i + 1;
      if (sum >= box.weight / 2) break;
    }
    boxes.push(measure(box.colors.slice(0, split)), measure(box.colors.slice(split)));
  }
  const palette = boxes.map((b) => [0, 1, 2].map((c) => Math.min(255, Math.round(b.colors.reduce((s, v) => s + v.rgb[c] * v.weight, 0) / b.weight))));
  return applyPalette(frames, palette);
}

// A targeted pose edit keeps the existing palette so untouched animations
// retain their exact colors, rather than changing with the new histogram.
function applyPalette(frames, palette) {
  assert.ok(palette.length > 0 && palette.length <= 24, "invalid pack palette");
  const cache = new Map();
  for (const frame of frames) {
    for (let p = 0; p < frame.pixels.length; p += 4) {
      if (!frame.pixels[p + 3]) continue;
      const rgb = [...frame.pixels.subarray(p, p + 3)], key = rgb.join(",");
      let color = cache.get(key);
      if (!color) {
        let distance = Infinity;
        for (const candidate of palette) {
          const d = candidate.reduce((s, v, i) => s + (v - rgb[i]) ** 2, 0);
          if (d < distance) { color = candidate; distance = d; }
        }
        cache.set(key, color);
      }
      frame.pixels.set(color, p);
    }
  }
  return palette;
}

// Median pose height of one sheet row.
function rowHeight(sheet, rows, row, columnCount = COLUMNS) {
  const columns = columnBoundaries(sheet, rows[row], rows[row + 1], columnCount);
  const heights = Array.from({ length: columnCount }, (_, col) => extractCell(sheet, col, row, rows, columns).bounds.height);
  return heights.sort((a, b) => a - b)[Math.floor(columnCount / 2)];
}

function prepareFrames(pet) {
  const atlas = decodePng(join(SOURCES, pet.source, pet.atlas ?? "atlas.png"));
  assert.equal(atlas.width % COLUMNS, 0, "atlas columns");
  const boundaries = rowBoundaries(atlas);
  const cells = ROWS.flatMap((_, row) => {
    const columns = columnBoundaries(atlas, boundaries[row], boundaries[row + 1]);
    return Array.from({ length: COLUMNS }, (_, col) => extractCell(atlas, col, row, boundaries, columns));
  });
  // One scale for the whole pack, taken from the original atlas. It is 1 (an
  // exact copy) whenever the largest pose fits the frame.
  const scale = Math.min(1, MAX_WIDTH / Math.max(...cells.map((c) => c.bounds.width)),
    MAX_HEIGHT / Math.max(...cells.map((c) => c.bounds.height)));
  const factors = cells.map(() => scale);
  const atlasIdle = rowHeight(atlas, boundaries, 0);
  const keep = new Set((pet.keep ?? []).map((name) => rowIndex(name)));
  // Imagegen edits are larger sheets. Cut them at their own resolution, then
  // resample once: sheet cell to atlas cell, and, when the sheet has an idle
  // row, by the atlas idle height over the edit's, so the character keeps the
  // same size when switching between edited and original states.
  const edit = (image, sheetColumns, sheetRows, targets, idleRow) => {
    const sheet = decodePng(join(SOURCES, pet.source, image));
    assert.ok(Math.abs(sheet.width / sheetColumns - sheet.height / sheetRows) <= 1,
      `${image}: expected a ${sheetColumns}×${sheetRows} pose sheet`);
    const rows = rowBoundaries(sheet, sheetRows, sheetColumns);
    const cellRatio = atlas.width / COLUMNS / (sheet.width / sheetColumns);
    const match = idleRow === undefined ? 1
      : Math.min(1.15, Math.max(0.85, atlasIdle / (rowHeight(sheet, rows, idleRow, sheetColumns) * cellRatio)));
    for (let row = 0; row < sheetRows; row++) {
      const columns = columnBoundaries(sheet, rows[row], rows[row + 1], sheetColumns);
      for (let col = 0; col < sheetColumns; col++) {
        const target = targets[row * sheetColumns + col];
        if (target < 0 || keep.has(Math.floor(target / COLUMNS))) continue;
        cells[target] = extractCell(sheet, col, row, rows, columns);
        factors[target] = scale * cellRatio * match;
      }
    }
  };
  const frameIndices = (name) => Array.from({ length: COLUMNS }, (_, col) => rowIndex(name) * COLUMNS + col);
  // 2×2 sheets redraw one animation's four poses.
  for (const [name, image] of Object.entries(pet.overrides ?? {})) edit(image, 2, 2, frameIndices(name));
  // Four phases per row: idle, walk, run.
  if (pet.locomotion) edit(pet.locomotion, COLUMNS, 3, ["idle", "walk", "run"].flatMap(frameIndices), 0);
  // Apply action corrections after locomotion, so a corrected planted idle
  // overrides that row without replacing the accepted walk/run frames. Later
  // sheets win. A "ref" row is a copy of the idle pose used only for size.
  for (const { image, rows: names } of [pet.poses ?? []].flat()) {
    const targets = names.flatMap((name) => name === "ref" ? Array(COLUMNS).fill(-1) : frameIndices(name));
    const size = names.includes("ref") ? names.indexOf("ref") : names.indexOf("idle");
    edit(image, COLUMNS, names.length, targets, size >= 0 ? size : undefined);
  }
  // The app shows only the first dizzy slot; move the clearly dazed pose there.
  if (pet.dizzy) {
    const first = rowIndex("dizzy") * COLUMNS, chosen = first + pet.dizzy;
    [cells[first], cells[chosen]] = [cells[chosen], cells[first]];
    [factors[first], factors[chosen]] = [factors[chosen], factors[first]];
  }
  return cells.map((cell, i) => normalizeFrame(resampleCell(cell, factors[i])));
}

// Replay good poses of a row in place of broken ones. Runs after the palette is
// built from the drawn poses, so reusing a frame never shifts the pack colours.
function reuseFrames(pet, frames) {
  const out = [...frames];
  for (const [name, order] of Object.entries(pet.reuse ?? {})) {
    assert.ok(order.length === COLUMNS && order.every((col) => col >= 0 && col < COLUMNS), `${pet.source}: reuse.${name} needs ${COLUMNS} columns`);
    const first = rowIndex(name) * COLUMNS;
    order.forEach((from, col) => { out[first + col] = { ...frames[first + from], pixels: Buffer.from(frames[first + from].pixels) }; });
  }
  return out;
}

function rowIndex(name) {
  const row = ROWS.findIndex(([animation]) => animation === name);
  assert.ok(row >= 0, `unknown animation ${name}`);
  return row;
}

function animationSpec(pet, row) {
  const [name, fps, loop = true] = ROWS[row];
  return {
    image: pet.singleSheet ? "atlas.png" : `${row >= 9 ? "phase2/" : ""}${name}.png`,
    // The source dizzy row mixes standing, seated and recovered poses. Keep
    // the initial dazed pose throughout the state; PetView supplies the sway
    // and DizzyStars supplies the orbiting stars without changing posture.
    frames: name === "dizzy" ? 1 : COLUMNS,
    fps: name === "dizzy" ? 1 : fps,
    loop,
    ...(pet.singleSheet ? { row } : {}),
  };
}

// Soft generated shading turns into single-pixel flecks once reduced to the
// pack palette. Replace a light fleck with the colour surrounding it; dark
// pixels are left alone because they draw outlines, pupils and mouths.
function despeckle(frames, passes = 2) {
  for (const frame of frames) {
    for (let pass = 0; pass < passes; pass++) {
      const before = Buffer.from(frame.pixels);
      for (let y = 1; y < SIZE - 1; y++) {
        for (let x = 1; x < SIZE - 1; x++) {
          const p = (y * SIZE + x) * 4;
          if (!before[p + 3] || luma(before[p], before[p + 1], before[p + 2]) < DARK) continue;
          const self = before.readUIntBE(p, 3);
          const counts = new Map();
          let same = 0;
          for (let dy = -1; dy <= 1; dy++) {
            for (let dx = -1; dx <= 1; dx++) {
              const n = p + (dy * SIZE + dx) * 4;
              if ((!dx && !dy) || !before[n + 3]) continue;
              const color = before.readUIntBE(n, 3);
              if (color === self) same++;
              else if (luma(before[n], before[n + 1], before[n + 2]) >= DARK) counts.set(color, (counts.get(color) ?? 0) + 1);
            }
          }
          if (same > 1) continue;
          let best = -1, most = 0;
          for (const [color, count] of counts) if (count > most) { best = color; most = count; }
          if (most >= 5) frame.pixels.writeUIntBE(best, p, 3);
        }
      }
    }
  }
}

// The original atlases have thin, partly faded outlines that disappear where the
// alpha threshold drops them. Close the silhouette with the pack's own outline
// colour (the most common dark edge colour), so every pose has an unbroken
// edge like the redrawn walk/run sheets. Only light edge pixels are repainted.
function closeOutline(frames) {
  const edge = (frame, p) => {
    const x = (p / 4) % SIZE, y = Math.floor(p / 4 / SIZE);
    return [[1, 0], [-1, 0], [0, 1], [0, -1]].some(([dx, dy]) => {
      const nx = x + dx, ny = y + dy;
      return nx < 0 || ny < 0 || nx >= SIZE || ny >= SIZE || !frame.pixels[((ny * SIZE + nx) * 4) + 3];
    });
  };
  const counts = new Map();
  for (const frame of frames) {
    for (let p = 0; p < frame.pixels.length; p += 4) {
      if (!frame.pixels[p + 3] || !edge(frame, p) || luma(...frame.pixels.subarray(p, p + 3)) >= DARK) continue;
      const color = frame.pixels.readUIntBE(p, 3);
      counts.set(color, (counts.get(color) ?? 0) + 1);
    }
  }
  if (!counts.size) return;
  const outline = [...counts].sort((a, b) => b[1] - a[1])[0][0];
  const dark = Math.max(DARK, luma(outline >> 16, (outline >> 8) & 255, outline & 255) + 40);
  for (const frame of frames) {
    // Two pixels thick, matching the weight of the redrawn sheets' outlines.
    const ring = new Uint8Array(SIZE * SIZE);
    for (let p = 0; p < frame.pixels.length; p += 4) if (frame.pixels[p + 3] && edge(frame, p)) ring[p / 4] = 1;
    for (let i = 0; i < ring.length; i++) {
      if (ring[i] || !frame.pixels[i * 4 + 3]) continue;
      const x = i % SIZE;
      if ((x > 0 && ring[i - 1] === 1) || (x < SIZE - 1 && ring[i + 1] === 1) || ring[i - SIZE] === 1 || ring[i + SIZE] === 1) ring[i] = 2;
    }
    for (let i = 0; i < ring.length; i++) {
      if (ring[i] && luma(...frame.pixels.subarray(i * 4, i * 4 + 3)) >= dark) frame.pixels.writeUIntBE(outline, i * 4, 3);
    }
  }
}

// Reduce the resampled frames to the pack palette, then clean them up.
function finishFrames(pet, frames) {
  const palette = pet.palette
    ? applyPalette(frames, JSON.parse(readFileSync(join(SOURCES, pet.source, pet.palette), "utf8"))
      .map((hex) => [1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16))))
    : quantize(frames);
  despeckle(frames);
  closeOutline(frames);
  frames.splice(0, frames.length, ...reuseFrames(pet, frames));
  const hex = (color) => color.map((v) => v.toString(16).padStart(2, "0")).join("");
  const packColor = (wanted) => {
    const rgb = [1, 3, 5].map((start) => parseInt(wanted.slice(start, start + 2), 16));
    let best, distance = Infinity;
    for (const color of palette) {
      const d = Math.hypot(...color.map((v, i) => v - rgb[i]));
      if (d < distance) { best = color; distance = d; }
    }
    assert.ok(distance <= 24, `${pet.source}: ${wanted} is not near any pack colour (closest #${hex(best)}); update the rule`);
    return best;
  };
  const inBox = ([left, top, right, bottom] = [0, 0, SIZE - 1, SIZE - 1]) => (x, y) => x >= left && x <= right && y >= top && y <= bottom;
  for (const rule of [pet.recolor ?? []].flat()) {
    const colors = new Map(Object.entries(rule.colors).map(([from, to]) => [hex(packColor(from)), packColor(to)]));
    for (const [from, to] of colors) assert.notEqual(from, hex(to), `${pet.source}: recolor #${from} maps to itself`);
    const inside = inBox(rule.box);
    for (const name of rule.rows) {
      for (const col of rule.frames ?? [0, 1, 2, 3]) {
        const { pixels } = frames[rowIndex(name) * COLUMNS + col];
        for (let p = 0; p < pixels.length; p += 4) {
          const to = pixels[p + 3] && inside((p / 4) % SIZE, Math.floor(p / 4 / SIZE)) && colors.get(pixels.subarray(p, p + 3).toString("hex"));
          if (to) pixels.set(to, p);
        }
      }
    }
  }
  for (const { row, frame, at: [x, y], color, box } of pet.fill ?? []) {
    const { pixels } = frames[rowIndex(row) * COLUMNS + frame];
    const to = packColor(color), inside = inBox(box), done = new Uint8Array(SIZE * SIZE);
    const open = (x, y) => inside(x, y) && pixels[(y * SIZE + x) * 4 + 3] && luma(...pixels.subarray((y * SIZE + x) * 4, (y * SIZE + x) * 4 + 3)) >= DARK;
    assert.ok(open(x, y), `${pet.source}: fill ${row} ${frame} starts on outline or background at ${x},${y}`);
    for (const stack = [[x, y]]; stack.length;) {
      const [px, py] = stack.pop();
      if (done[py * SIZE + px] || !open(px, py)) continue;
      done[py * SIZE + px] = 1;
      pixels.set(to, (py * SIZE + px) * 4);
      stack.push([px + 1, py], [px - 1, py], [px, py + 1], [px, py - 1]);
    }
  }
  return palette;
}

function prepare(pet) {
  const frames = prepareFrames(pet);
  const palette = finishFrames(pet, frames);
  const dir = join(PACKS, pet.folder);
  mkdirSync(dir, { recursive: true });
  const animations = {};
  const sheet = emptyImage(SIZE * COLUMNS, SIZE * ROWS.length);
  for (const [row, [name]] of ROWS.entries()) {
    const strip = emptyImage(SIZE * COLUMNS, SIZE);
    for (let col = 0; col < COLUMNS; col++) {
      blit(strip, frames[row * COLUMNS + col], col * SIZE, 0);
      blit(sheet, frames[row * COLUMNS + col], col * SIZE, row * SIZE);
    }
    const spec = animationSpec(pet, row);
    if (!pet.singleSheet) savePng(join(dir, spec.image), strip);
    if (row < 9) animations[name] = spec;
  }
  const manifest = { name: pet.name, frameWidth: SIZE, frameHeight: SIZE, scale: DISPLAY_SIZE / SIZE, pixelArt: true, facing: "right", anchor: { x: SIZE / 2, y: BASELINE + 1 }, outline: false, animations };
  writeFileSync(join(dir, "pet.json"), JSON.stringify(manifest, null, 2) + "\n");
  // Pack dải riêng không cần sheet gộp: trang xem thử (index.html) đọc thẳng các dải trong pack.
  if (pet.singleSheet) savePng(join(dir, "atlas.png"), sheet);
  writeFileSync(join(SOURCES, pet.source, "palette.json"), JSON.stringify(palette.map((c) => "#" + c.map((v) => v.toString(16).padStart(2, "0")).join("")), null, 2) + "\n");
  console.log(`${pet.folder}: ${pet.singleSheet ? "1 sheet, 12 rows" : "12 strips"}, ${COLUMNS} frames each, ${SIZE}×${SIZE}, ${palette.length} colors`);
}

async function check(pets = PETS) {
  // Run the exact application parser and frame-layout logic, not a copied schema.
  const ts = (await import("typescript")).default;
  const code = ts.transpileModule(readFileSync(join(ROOT, "packages/core/src/sprite.ts"), "utf8"), { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
  const { ANIMATION_NAMES, parseSpriteManifest, frameRects } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
  for (const pet of pets) {
    const dir = join(PACKS, pet.folder);
    const expectedFrames = reuseFrames(pet, prepareFrames(pet));
    const manifest = parseSpriteManifest(JSON.parse(readFileSync(join(dir, "pet.json"), "utf8")));
    assert.equal(manifest.facing, "right");
    assert.equal(manifest.pixelArt, true);
    assert.equal(manifest.frameWidth, SIZE);
    assert.equal(manifest.frameHeight, SIZE);
    assert.equal(manifest.frameWidth * manifest.scale, DISPLAY_SIZE, `${pet.folder}: default display size`);
    assert.equal(manifest.scale * 2, 1, `${pet.folder}: native source resolution at 200%`);
    assert.deepEqual(manifest.anchor, { x: SIZE / 2, y: BASELINE + 1 });
    assert.deepEqual(Object.keys(manifest.animations).sort(), [...ANIMATION_NAMES].sort());
    if (pet.singleSheet) {
      assert.deepEqual(readdirSync(dir, { recursive: true }).filter((file) => /\.png$/i.test(file)).sort(), ["atlas.png"], `${pet.folder}: keep only the shared sheet`);
    }
    const colors = new Set();
    for (const [row, [name]] of ROWS.entries()) {
      const spec = row < 9 ? manifest.animations[name] : animationSpec(pet, row);
      const path = spec.image;
      const image = decodePng(join(dir, path));
      assert.equal(image.width, SIZE * COLUMNS, path);
      assert.equal(image.height, SIZE * (pet.singleSheet ? ROWS.length : 1), path);
      const rects = frameRects({ start: 0, row: 0, ...spec }, SIZE, SIZE, image.width, image.height);
      assert.equal(spec.frames, name === "dizzy" ? 1 : COLUMNS);
      assert.equal(rects.length, spec.frames);
      if (name === "dizzy") {
        assert.equal(spec.start ?? 0, 0, `${pet.folder}: canonical dizzy pose`);
        assert.equal(spec.fps, 1);
      }
      // All four source slots are stored for provenance, even when the app
      // uses only the canonical dizzy frame. Validate every stored slot too.
      const storedRects = frameRects({ start: 0, row: 0, ...spec, frames: COLUMNS }, SIZE, SIZE, image.width, image.height);
      for (let p = 0; p < image.pixels.length; p += 4) {
        assert.ok(image.pixels[p + 3] === 0 || image.pixels[p + 3] === 255, `${path}: partial alpha`);
        if (image.pixels[p + 3]) colors.add(image.pixels.subarray(p, p + 3).toString("hex"));
      }
      for (let col = 0; col < COLUMNS; col++) {
        const frame = emptyImage(SIZE, SIZE);
        const rect = storedRects[col];
        for (let y = 0; y < SIZE; y++) image.pixels.copy(frame.pixels, y * SIZE * 4, ((rect.y + y) * image.width + rect.x) * 4, ((rect.y + y) * image.width + rect.x + SIZE) * 4);
        const b = boundsOf(frame);
        const expected = expectedFrames[row * COLUMNS + col];
        for (let p = 3; p < frame.pixels.length; p += 4) {
          assert.equal(frame.pixels[p], expected.pixels[p], `${pet.folder}/${path}: source silhouette mismatch in frame ${col}; regenerate sprites`);
        }
        assert.ok(b.width > 0 && b.height > 0, `${path}: empty frame ${col}`);
        assert.equal(b.bottom, BASELINE, `${path}: foot baseline ${col}`);
        assert.ok(b.left >= 2 && b.right < SIZE - 2 && b.top >= 2, `${path}: clipped frame ${col}`);
        for (const corner of [0, SIZE - 1, SIZE * (SIZE - 1), SIZE * SIZE - 1]) assert.equal(frame.pixels[corner * 4 + 3], 0, `${path}: opaque background`);
      }
    }
    assert.ok(colors.size <= 24, `${pet.folder}: inconsistent palette`);
    console.log(`${pet.folder}: source silhouettes, manifest, 48 frames, binary alpha, 24-color palette and y=${BASELINE} baseline OK`);
  }
  // The app defaults to the first pack folder (with a pet.json) by name.
  const first = readdirSync(PACKS).filter((dir) => existsSync(join(PACKS, dir, "pet.json"))).sort()[0];
  assert.equal(first, PETS[0].folder);
  console.log(`Default pack: ${first}`);
}

async function main() {
  const source = process.argv.find((arg) => arg.startsWith("--pet="))?.slice(6);
  const selectedPets = source === undefined ? PETS : PETS.filter((pet) => pet.source === source);
  assert.ok(selectedPets.length, `Unknown pet source: ${source}`);
  if (process.argv.includes("--check")) await check(selectedPets);
  else {
    for (const pet of selectedPets) {
      if (process.argv.includes("--only-new") && existsSync(join(PACKS, pet.folder, "pet.json"))) continue;
      prepare(pet);
    }
    const gallery = PETS.map((pet) => ({
      name: pet.name,
      folder: pet.folder,
      source: pet.source,
      animations: ROWS.map(([name], row) => {
        const spec = animationSpec(pet, row);
        return {
          name, fps: spec.fps, frames: spec.frames, loop: spec.loop, phase2: row >= 9, frameSize: SIZE,
          row: spec.row ?? 0,
          image: `../sprites/${pet.folder}/${spec.image}`,
        };
      }),
    }));
    writeFileSync(join(SOURCES, "gallery-data.js"), "// Generated by scripts/prepare-sprites.mjs\nwindow.SPRITE_PETS = " + JSON.stringify(gallery, null, 2) + ";\n");
    await check(selectedPets);
  }
}

export { PETS, ROWS, boundsOf, columnBoundaries, decodePng, emptyImage, extractCell, finishFrames, normalizeFrame, prepareFrames, resampleCell, rowBoundaries, savePng };

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
