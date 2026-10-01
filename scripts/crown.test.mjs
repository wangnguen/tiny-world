import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import ts from "typescript";

// Chạy đúng `Mask` và `crownOf` của overlay; import duy nhất cần giá trị là ANIMATION_NAMES, ở đây không dùng.
const source = readFileSync(new URL("../desktop/src/overlay/spriteSet.ts", import.meta.url), "utf8").replace(
  /import\s*\{[^}]*\}\s*from\s*"@tinyworld\/core";/,
  "const ANIMATION_NAMES = [];",
);
const code = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { Mask, crownOf } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

/** Mặt nạ từ hình vẽ bằng chữ: `#` là có hình. */
function maskOf(rows) {
  const width = rows[0].length;
  return new Mask(width, rows.length, (i) => rows[Math.floor(i / width)][i % width] === "#");
}

// Đầu rộng 12 cột (hàng 4–9), thân rộng 16 cột bên dưới, tuỳ hình mà thêm tai hay ăng-ten ở trên.
const head = [
  "....########....",
  "..############..",
  "..############..",
  "..############..",
  "..############..",
  "..############..",
];
const body = Array.from({ length: 14 }, () => "################");

test("đầu tròn không có gì phía trên: đỉnh đầu là hàng trên cùng đã đủ rộng", () => {
  const crown = crownOf(maskOf([...head, ...body]));
  assert.deepEqual(crown, { x: 8, y: 0 });
});

test("tai mảnh dựng trên đầu bị bỏ qua, mũ đội giữa hai tai", () => {
  const ears = ["...##....##.....", "...##....##.....", "...##....##.....", "...##....##....."];
  const crown = crownOf(maskOf([...ears, ...head, ...body]));
  assert.equal(crown.y, 4);
  assert.equal(crown.x, 8);
});

test("ăng-ten bị bỏ qua", () => {
  const antenna = [".....###........", "......#.........", "......#........."];
  assert.equal(crownOf(maskOf([...antenna, ...head, ...body])).y, 3);
});

test("tai to sát nhau: tăng ngưỡng thì mũ xuống dưới tai", () => {
  const ears = ["...####.####....", "...####.####....", "...#########....", "...#########...."];
  const mask = maskOf([...ears, ...head, ...body]);
  assert.equal(crownOf(mask).y, 2);
  assert.equal(crownOf(mask, 0.7).y, 5);
});

test("frame trống không lỗi", () => {
  const crown = crownOf(maskOf(["....", "...."]));
  assert.equal(crown.x, 2);
});
