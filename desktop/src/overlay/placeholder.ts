import type { Rect } from "@tinyworld/core";
import { buildAnimation, withFallback, type SpriteSet } from "./spriteSet";

// Pet tạm: một cục blob pixel art 16×16, dùng khi assets/sprites chưa có sprite pack nào.
const SIZE = 16;

const COLORS: Record<string, string> = {
  o: "#5a3d2b", // viền
  w: "#ffe3b3", // thân
  s: "#f5c98a", // bóng
  h: "#ffffff", // chỗ sáng
  k: "#2b1d14", // mắt, miệng
  p: "#ff9aa2", // má
};

const EMPTY = ".".repeat(SIZE);

const BASE = [
  EMPTY,
  EMPTY,
  EMPTY,
  ".....oooooo.....",
  "...oowwwwhwoo...",
  "..owwwwwwwhwwo..",
  ".owwwwwwwwwwwwo.",
  ".owwkwwwwwwkwwo.",
  "owwwkwwwwwwkwwwo",
  "owppwwwwwwwwppwo",
  "owwwwwwkkwwwwwwo",
  "owwwwwwwwwwwwwwo",
  "owswwwwwwwwwwswo",
  ".osswwwwwwwwsso.",
  "..oossssssssoo..",
  "....oooooooo....",
];

const replaceRows = (rows: string[], changes: Record<number, string>) =>
  rows.map((row, i) => changes[i] ?? row);

/** Thở: bớt một hàng giữa thân, chân vẫn chạm đất. */
const BREATHE = [EMPTY, ...BASE.slice(0, 6), ...BASE.slice(7)];
const BLINK = replaceRows(BASE, { 7: BASE[6], 8: "owwkkkwwwwkkkwwo" });
/** Bị click: bẹp xuống, mắt cười ^ ^. */
const HAPPY = replaceRows(BASE, { 8: "owwkwkwwwwkwkwwo" });
const SQUASH = [EMPTY, EMPTY, ...HAPPY.slice(0, 6), ...HAPPY.slice(7, 11), ...HAPPY.slice(12)];

const FRAMES = [BASE, BREATHE, BLINK, SQUASH];
const [F_BASE, F_BREATHE, F_BLINK, F_SQUASH] = [0, 1, 2, 3];

function drawSheet(): OffscreenCanvas {
  const sheet = new OffscreenCanvas(SIZE * FRAMES.length, SIZE);
  const ctx = sheet.getContext("2d");
  if (!ctx) throw new Error("Không tạo được canvas cho pet tạm.");
  FRAMES.forEach((rows, frame) =>
    rows.forEach((row, y) =>
      [...row].forEach((pixel, x) => {
        const color = COLORS[pixel];
        if (!color) return;
        ctx.fillStyle = color;
        ctx.fillRect(frame * SIZE + x, y, 1, 1);
      }),
    ),
  );
  return sheet;
}

const rect = (frame: number): Rect => ({ x: frame * SIZE, y: 0, width: SIZE, height: SIZE });

export function createPlaceholderSprite(): SpriteSet {
  const sheet = drawSheet();
  const idleFrames = [F_BASE, F_BASE, F_BREATHE, F_BREATHE, F_BASE, F_BASE, F_BREATHE, F_BLINK];
  return {
    name: "placeholder",
    frameWidth: SIZE,
    frameHeight: SIZE,
    scale: 3,
    pixelArt: true,
    facing: "right",
    anchor: { x: SIZE / 2, y: SIZE },
    animations: withFallback({
      idle: buildAnimation(sheet, idleFrames.map(rect), 4, true),
      react: buildAnimation(sheet, [F_SQUASH, F_SQUASH, F_BASE].map(rect), 6, false),
    }),
  };
}
