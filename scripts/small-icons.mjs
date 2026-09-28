// Vẽ lại icon cỡ nhỏ (16, 24, 32 px) theo đúng lưới pixel rồi ghép vào icon.ico.
//
// Logo gốc (assets/icon.png) là ảnh lớn; thu nhỏ xuống 16–32 px thì mắt, miệng nhoè thành vệt, mà
// tray và thanh tiêu đề lại dùng đúng các cỡ này. Nên các cỡ nhỏ được vẽ lại bằng code: cùng hình
// (pet màu kem ôm hành tinh xanh, có mầm cây), cùng bảng màu, mỗi pixel đặt có chủ đích.
//
// Chạy sau `tauri icon` (scripts/generate-icons.ps1 gọi sẵn): ghi đè icons/32x32.png và dựng lại
// icons/icon.ico từ ba cỡ vẽ tay + các cỡ lớn tauri đã tạo (64, 128, 256).
//
//   node scripts/small-icons.mjs [--preview <file.png>]

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

const ICONS = join(dirname(fileURLToPath(import.meta.url)), "../desktop/src-tauri/icons");

const PALETTE = {
  o: [55, 43, 42], // viền, mắt, miệng
  w: [255, 227, 179], // thân kem
  s: [245, 201, 138], // bóng dưới thân
  p: [255, 154, 162], // má
  h: [255, 255, 255], // chấm sáng trong mắt
  g: [109, 190, 126], // lá mầm
  m: [167, 224, 189], // hành tinh
  t: [63, 139, 121], // đất trên hành tinh
  l: [205, 239, 216], // chỗ sáng trên hành tinh
};

/**
 * Hình dạng vẽ trên hệ toạ độ 32×32 rồi lấy mẫu ở tâm từng pixel của cỡ thật, nên ba cỡ cùng một
 * dáng. Mặt (mắt, má, miệng) thì đặt tay từng pixel cho mỗi cỡ.
 */
const inEllipse = (x, y, cx, cy, rx, ry) => ((x - cx) / rx) ** 2 + ((y - cy) / ry) ** 2 <= 1;

const SHAPES = [
  // Hành tinh và đất.
  { color: "m", test: (x, y) => inEllipse(x, y, 16, 23.5, 8.6, 7.8) },
  { color: "t", test: (x, y) => inEllipse(x, y, 16, 23.5, 8.6, 7.8) && (inEllipse(x, y, 12.5, 26.5, 4.2, 2.6) || inEllipse(x, y, 21, 22, 3.2, 2.2)) },
  // Thân pet (nửa trên hình bầu dục), bóng ở đáy.
  { color: "w", test: (x, y) => y <= 19 && inEllipse(x, y, 16, 13, 10, 8) },
  { color: "s", test: (x, y) => y > 16.5 && y <= 19 && inEllipse(x, y, 16, 13, 10, 8) },
  // Hai tay ôm hành tinh.
  { color: "w", test: (x, y) => inEllipse(x, y, 10.5, 19.2, 2.4, 1.8) || inEllipse(x, y, 21.5, 19.2, 2.4, 1.8) },
  // Mầm cây trên đầu (cỡ 16 vẽ tay, xem SPROUT_16).
  {
    color: "g",
    minSize: 24,
    test: (x, y) => (x >= 15.2 && x <= 16.8 && y >= 2.8 && y <= 5.6) || inEllipse(x, y, 12.6, 3, 2.7, 1.6) || inEllipse(x, y, 19.4, 3, 2.7, 1.6),
  },
];

/** Ở 16 px hai lá và viền dính thành một thanh, nên đặt tay: hai lá xoè hai bên, cuống ở giữa. */
const SPROUT_16 = [[5, 1], [6, 1], [9, 1], [10, 1], [7, 2], [8, 2]];

/** Mặt cho từng cỡ: [cột, hàng, màu]. */
const FACES = {
  16: [
    [5, 6, "o"], [5, 7, "o"], [10, 6, "o"], [10, 7, "o"],
    [4, 8, "p"], [11, 8, "p"],
    [7, 8, "o"], [8, 8, "o"],
  ],
  24: [
    [7, 8, "o"], [8, 8, "h"], [7, 9, "o"], [8, 9, "o"],
    [15, 8, "o"], [16, 8, "h"], [15, 9, "o"], [16, 9, "o"],
    [5, 11, "p"], [6, 11, "p"], [17, 11, "p"], [18, 11, "p"],
    [10, 11, "o"], [11, 12, "o"], [12, 12, "o"], [13, 11, "o"],
  ],
  32: [
    [10, 11, "o"], [11, 11, "h"], [10, 12, "o"], [11, 12, "o"], [10, 13, "o"], [11, 13, "o"],
    [20, 11, "o"], [21, 11, "h"], [20, 12, "o"], [21, 12, "o"], [20, 13, "o"], [21, 13, "o"],
    [7, 15, "p"], [8, 15, "p"], [23, 15, "p"], [24, 15, "p"],
    [14, 15, "o"], [15, 16, "o"], [16, 16, "o"], [17, 15, "o"],
    [11, 26, "l"], [10, 25, "l"], [10, 24, "l"],
  ],
};

function draw(size) {
  const k = 32 / size;
  const grid = Array.from({ length: size * size }, (_, i) => {
    const x = ((i % size) + 0.5) * k;
    const y = (Math.floor(i / size) + 0.5) * k;
    let color = null;
    for (const shape of SHAPES) {
      if (size >= (shape.minSize ?? 0) && shape.test(x, y)) color = shape.color;
    }
    return color;
  });
  if (size === 16) for (const [x, y] of SPROUT_16) grid[y * size + x] = "g";
  const at = (x, y) => (x < 0 || y < 0 || x >= size || y >= size ? null : grid[y * size + x]);
  const pet = (c) => c === "w" || c === "s";
  const planet = (c) => c === "m" || c === "t";
  const outline = [];
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const c = at(x, y);
      const around = [at(x - 1, y), at(x + 1, y), at(x, y - 1), at(x, y + 1)];
      // Viền ngoài, và một đường viền ngăn thân pet với hành tinh.
      if (c === null ? around.some((n) => n !== null) : planet(c) && around.some(pet)) {
        outline.push(y * size + x);
      }
    }
  }
  for (const i of outline) grid[i] = "o";
  for (const [x, y, color] of FACES[size]) grid[y * size + x] = color;
  return grid;
}

function rgba(grid) {
  const data = Buffer.alloc(grid.length * 4);
  grid.forEach((key, i) => {
    if (key) data.set([...PALETTE[key], 255], i * 4);
  });
  return data;
}

// PNG tối giản: RGBA 8 bit, một khối IDAT.
const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});
function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const out = Buffer.alloc(12 + data.length);
  out.writeUInt32BE(data.length, 0);
  out.write(type, 4, "ascii");
  data.copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + data.length)), 8 + data.length);
  return out;
}
function png(width, height, pixels) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 6, 0, 0, 0], 8);
  const raw = Buffer.alloc((width * 4 + 1) * height);
  for (let y = 0; y < height; y++) pixels.copy(raw, y * (width * 4 + 1) + 1, y * width * 4, (y + 1) * width * 4);
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** ICO chứa ảnh PNG (Windows Vista trở lên đọc được mọi cỡ). */
function ico(images) {
  const header = Buffer.alloc(6 + 16 * images.length);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);
  let offset = header.length;
  images.forEach(({ size, data }, i) => {
    const entry = 6 + 16 * i;
    header.writeUInt8(size >= 256 ? 0 : size, entry);
    header.writeUInt8(size >= 256 ? 0 : size, entry + 1);
    header.writeUInt16LE(1, entry + 4);
    header.writeUInt16LE(32, entry + 6);
    header.writeUInt32LE(data.length, entry + 8);
    header.writeUInt32LE(offset, entry + 12);
    offset += data.length;
  });
  return Buffer.concat([header, ...images.map((image) => image.data)]);
}

const small = [16, 24, 32].map((size) => ({ size, grid: draw(size) }));
const previewAt = process.argv.indexOf("--preview");
if (previewAt > 0) {
  // Ảnh xem thử: ba cỡ phóng to 8 lần đặt cạnh nhau trên nền xám.
  const zoom = 8;
  const width = (16 + 24 + 32 + 4 * 2) * zoom;
  const height = (32 + 4) * zoom;
  const pixels = Buffer.alloc(width * height * 4);
  for (let i = 0; i < width * height; i++) pixels.set([48, 52, 68, 255], i * 4);
  let left = 2;
  for (const { size, grid } of small) {
    const colors = rgba(grid);
    for (let y = 0; y < size * zoom; y++) {
      for (let x = 0; x < size * zoom; x++) {
        const src = (Math.floor(y / zoom) * size + Math.floor(x / zoom)) * 4;
        if (colors[src + 3] === 0) continue;
        pixels.set(colors.subarray(src, src + 4), ((y + 2 * zoom) * width + (left * zoom + x)) * 4);
      }
    }
    left += size + 2;
  }
  writeFileSync(process.argv[previewAt + 1], png(width, height, pixels));
} else {
  const drawn = small.map(({ size, grid }) => ({ size, data: png(size, size, rgba(grid)) }));
  writeFileSync(join(ICONS, "32x32.png"), drawn[2].data);
  const large = [
    { size: 64, file: "64x64.png" },
    { size: 128, file: "128x128.png" },
    { size: 256, file: "128x128@2x.png" },
  ].map(({ size, file }) => ({ size, data: readFileSync(join(ICONS, file)) }));
  writeFileSync(join(ICONS, "icon.ico"), ico([...drawn, ...large]));
  console.log("Đã vẽ icon 16/24/32 px và dựng lại icon.ico.");
}
