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
const breathe = (rows: string[]) => [EMPTY, ...rows.slice(0, 6), ...rows.slice(7)];
const BREATHE = breathe(BASE);
/** Nhắm mắt: chớp mắt khi đứng, và lúc ngủ. */
const BLINK = replaceRows(BASE, { 7: BASE[6], 8: "owwkkkwwwwkkkwwo" });
/** Bị click hoặc tiếp đất: bẹp xuống, mắt cười ^ ^. */
const HAPPY = replaceRows(BASE, { 8: "owwkwkwwwwkwkwwo" });
const SQUASH = [EMPTY, EMPTY, ...HAPPY.slice(0, 6), ...HAPPY.slice(7, 11), ...HAPPY.slice(12)];
/** Đang rơi hoặc bị nhấc lên: miệng há tròn. */
const SURPRISED = replaceRows(BASE, { 11: "owwwwwwkkwwwwwwo" });
/** Choáng: mắt ✕. */
const DIZZY = replaceRows(BASE, {
  6: ".owkwkwwwwkwkwo.",
  8: "owwkwkwwwwkwkwwo",
});

const FRAMES = [BASE, BREATHE, BLINK, SQUASH, breathe(BLINK), SURPRISED, DIZZY];
const [F_BASE, F_BREATHE, F_BLINK, F_SQUASH, F_SLEEP, F_SURPRISED, F_DIZZY] = FRAMES.keys();

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
  // Lưới pixel đã có viền (màu "o") nên không thêm viền nữa.
  const anim = (frames: number[], fps: number, loop = true) =>
    buildAnimation(sheet, frames.map(rect), fps, loop);
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
      idle: anim(idleFrames, 4),
      // Blob không có chân: đi và chạy là nảy tưng tưng.
      walk: anim([F_BASE, F_BREATHE], 6),
      run: anim([F_BASE, F_BREATHE], 12),
      sleep: anim([F_BLINK, F_SLEEP], 1),
      react: anim([F_SQUASH, F_SQUASH, F_BASE], 6, false),
      fall: anim([F_SURPRISED], 1),
      dragged: anim([F_SURPRISED], 1),
      land: anim([F_SQUASH], 1, false),
      dizzy: anim([F_DIZZY], 1),
    }),
  };
}
