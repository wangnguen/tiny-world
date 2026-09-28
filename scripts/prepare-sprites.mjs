// Normalize generated atlases into bottom-aligned 48px sprite strips or one sheet.
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
const SIZE = 48;
const BASELINE = 44;
const COLUMNS = 4;
const ROWS = [
  ["idle", 5], ["walk", 8], ["run", 12], ["sleep", 3],
  ["react", 8, false], ["fall", 8], ["dragged", 6],
  ["land", 16, false], ["dizzy", 5],
  ["climb", 8], ["perch", 4], ["jump", 10, false],
];
const PETS = [
  { source: "momo", folder: "a-momo", name: "Momo — Axolotl" },
  { source: "bong", folder: "b-bong", name: "Bông — Bunny" },
  { source: "kitsu", folder: "b-kitsu", name: "Kitsu — Fox" },
  { source: "mam", folder: "c-mam", name: "Mầm — Sprout" },
  { source: "bip", folder: "c-bip", name: "Bíp — Robot", overrides: { dragged: "dragged-v2.png" } },
  { source: "lumi", folder: "c-lumi", name: "Lumi — Star Spirit" },
  { source: "nam", folder: "c-nam", name: "Nấm — Mushroom" },
  { source: "may", folder: "c-may", name: "Mây — Cloud" },
  { source: "tan", folder: "c-tan", name: "Tàn — Ember" },
  { source: "reu", folder: "c-reu", name: "Rêu — Leaf Dragon" },
  { source: "cuc", folder: "c-cuc", name: "Cục — Pebble", atlas: "atlas-v2.png" },
  { source: "muc", folder: "c-muc", name: "Mực — Octopus" },
  { source: "dua", folder: "c-dua", name: "Dứa — Pineapple" },
  { source: "su", folder: "c-su", name: "Su — Astronaut" },
  { source: "bap", folder: "c-bap", name: "Bắp — Bumblebee" },
  { source: "boggo", folder: "c-boggo", name: "Boggo — Coder Frog" },
  { source: "wobi", folder: "c-wobi", name: "Wobi — Clown Frog", singleSheet: true },
  { source: "gloop", folder: "c-gloop", name: "Gloop — Chaos Frog" },
  { source: "bep", folder: "c-bep", name: "Bẹp — Grumpy Toad" },
  { source: "frobu", folder: "c-frobu", name: "Frobu — Night Frog" },
  { source: "byte", folder: "c-byte", name: "Byte — Coder Penguin", singleSheet: true },
  { source: "patch", folder: "c-patch", name: "Patch — Coder Red Panda", singleSheet: true },
];

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

// All poses share ONE scale. Never stretch each frame independently. Snap the
// crop to pixels with nearest-neighbor sampling, then align to the same baseline.
function normalizeFrame(source, scale) {
  const { bounds: b } = source;
  const width = Math.max(1, Math.round(b.width * scale));
  const height = Math.max(1, Math.round(b.height * scale));
  const frame = emptyImage(SIZE, SIZE);
  const left = Math.round((SIZE - width) / 2);
  const top = BASELINE - height + 1;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const sx = b.left + Math.min(b.width - 1, Math.floor((x + 0.5) * b.width / width));
      const sy = b.top + Math.min(b.height - 1, Math.floor((y + 0.5) * b.height / height));
      const from = (sy * source.width + sx) * 4;
      source.pixels.copy(frame.pixels, ((top + y) * SIZE + left + x) * 4, from, from + 4);
    }
  }
  // Resampling a very thin toe can leave a transparent last row; move the whole
  // sprite by that integer offset, without changing any pose or drawing pixels.
  const bottom = boundsOf(frame).bottom;
  if (bottom !== BASELINE) {
    const aligned = emptyImage(SIZE, SIZE);
    blit(aligned, frame, 0, BASELINE - bottom);
    return aligned;
  }
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

// Imagegen pose edits use a separate 2×2 sheet; resize it to the original atlas's
// source-pixel scale before applying the single scale shared by the entire pack.
function resizeSource(image, width, height) {
  const resized = emptyImage(width, height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const sx = Math.min(image.width - 1, Math.floor((x + 0.5) * image.width / width));
      const sy = Math.min(image.height - 1, Math.floor((y + 0.5) * image.height / height));
      const from = (sy * image.width + sx) * 4;
      image.pixels.copy(resized.pixels, (y * width + x) * 4, from, from + 4);
    }
  }
  return resized;
}

function prepareFrames(pet) {
  const atlas = decodePng(join(SOURCES, pet.source, pet.atlas ?? "atlas.png"));
  assert.equal(atlas.width % COLUMNS, 0, "atlas columns");
  const boundaries = rowBoundaries(atlas);
  const cells = ROWS.flatMap(([name], row) => {
    const override = pet.overrides?.[name];
    if (override) {
      const edited = decodePng(join(SOURCES, pet.source, override));
      assert.equal(edited.width, edited.height, `${override}: expected a 2×2 pose sheet`);
      const sheet = resizeSource(edited, atlas.width / 2, atlas.width / 2);
      const rows = rowBoundaries(sheet, 2, 2);
      return Array.from({ length: COLUMNS }, (_, col) => {
        const r = Math.floor(col / 2);
        return extractCell(sheet, col % 2, r, rows, columnBoundaries(sheet, rows[r], rows[r + 1], 2));
      });
    }
    const columns = columnBoundaries(atlas, boundaries[row], boundaries[row + 1]);
    return Array.from({ length: COLUMNS }, (_, col) => extractCell(atlas, col, row, boundaries, columns));
  });
  const scale = Math.min(40 / Math.max(...cells.map((c) => c.bounds.width)), 40 / Math.max(...cells.map((c) => c.bounds.height)));
  return cells.map((c) => normalizeFrame(c, scale));
}

function animationSpec(pet, row) {
  const [name, fps, loop = true] = ROWS[row];
  return {
    image: pet.singleSheet ? "atlas.png" : `${row >= 9 ? "phase2/" : ""}${name}.png`,
    frames: COLUMNS,
    fps,
    loop,
    ...(pet.singleSheet ? { row } : {}),
  };
}

function prepare(pet) {
  const frames = prepareFrames(pet);
  const palette = quantize(frames);
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
  const manifest = { name: pet.name, frameWidth: SIZE, frameHeight: SIZE, scale: 2, pixelArt: true, facing: "right", anchor: { x: SIZE / 2, y: BASELINE + 1 }, outline: false, animations };
  writeFileSync(join(dir, "pet.json"), JSON.stringify(manifest, null, 2) + "\n");
  // Pack dải riêng không cần sheet gộp: trang xem thử (index.html) đọc thẳng các dải trong pack.
  if (pet.singleSheet) savePng(join(dir, "atlas.png"), sheet);
  writeFileSync(join(SOURCES, pet.source, "palette.json"), JSON.stringify(palette.map((c) => "#" + c.map((v) => v.toString(16).padStart(2, "0")).join("")), null, 2) + "\n");
  console.log(`${pet.folder}: ${pet.singleSheet ? "1 sheet, 12 rows" : "12 strips"}, ${COLUMNS} frames each, 48×48, ${palette.length} colors`);
}

async function check(pets = PETS) {
  // Run the exact application parser and frame-layout logic, not a copied schema.
  const ts = (await import("typescript")).default;
  const code = ts.transpileModule(readFileSync(join(ROOT, "packages/core/src/sprite.ts"), "utf8"), { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
  const { ANIMATION_NAMES, parseSpriteManifest, frameRects } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
  for (const pet of pets) {
    const dir = join(PACKS, pet.folder);
    const expectedFrames = prepareFrames(pet);
    const manifest = parseSpriteManifest(JSON.parse(readFileSync(join(dir, "pet.json"), "utf8")));
    assert.equal(manifest.facing, "right");
    assert.equal(manifest.pixelArt, true);
    assert.equal(manifest.frameWidth, SIZE);
    assert.equal(manifest.frameHeight, SIZE);
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
      assert.equal(rects.length, COLUMNS);
      for (let p = 0; p < image.pixels.length; p += 4) {
        assert.ok(image.pixels[p + 3] === 0 || image.pixels[p + 3] === 255, `${path}: partial alpha`);
        if (image.pixels[p + 3]) colors.add(image.pixels.subarray(p, p + 3).toString("hex"));
      }
      for (let col = 0; col < COLUMNS; col++) {
        const frame = emptyImage(SIZE, SIZE);
        const rect = rects[col];
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
  const first = readdirSync(PACKS).filter((dir) => PETS.some((p) => p.folder === dir) || dir === "cat").sort()[0];
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
          name, fps: spec.fps, loop: spec.loop, phase2: row >= 9,
          row: spec.row ?? 0,
          image: `../sprites/${pet.folder}/${spec.image}`,
        };
      }),
    }));
    writeFileSync(join(SOURCES, "gallery-data.js"), "// Generated by scripts/prepare-sprites.mjs\nwindow.SPRITE_PETS = " + JSON.stringify(gallery, null, 2) + ";\n");
    await check(selectedPets);
  }
}

export { PETS, ROWS, boundsOf, columnBoundaries, decodePng, emptyImage, extractCell, normalizeFrame, prepareFrames, rowBoundaries, savePng };

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
