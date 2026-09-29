import assert from "node:assert/strict";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import {
  PETS, ROWS, boundsOf, columnBoundaries, decodePng, emptyImage, extractCell, finishFrames, prepareFrames, resampleCell, rowBoundaries, smooth,
} from "./prepare-sprites.mjs";

const SOURCES = fileURLToPath(new URL("../assets/sprite-sources/", import.meta.url));
// Buffer.equals, not deepEqual: a failing deepEqual on frame buffers builds a
// diff big enough to exhaust the heap.
const samePixels = (a, b) => a.pixels.equals(b.pixels);

function fill(image, left, top, width, height, color = [80, 160, 200, 255]) {
  for (let y = top; y < top + height; y++) {
    for (let x = left; x < left + width; x++) image.pixels.set(color, (y * image.width + x) * 4);
  }
}

function unevenAtlas() {
  const atlas = emptyImage(80, 90);
  const poses = [{ top: 4, height: 18 }, { top: 26, height: 19 }, { top: 53, height: 25 }];
  for (let col = 0; col < 4; col++) {
    for (const pose of poses) fill(atlas, col * 20 + 5, pose.top, 10, pose.height);
  }
  return { atlas, poses };
}

test("uneven animation rows keep every pose's full head and feet", () => {
  const { atlas, poses } = unevenAtlas();
  const boundaries = rowBoundaries(atlas, poses.length);
  for (let row = 0; row < poses.length; row++) {
    for (let col = 0; col < 4; col++) {
      const cell = extractCell(atlas, col, row, boundaries);
      assert.equal(cell.bounds.height, poses[row].height);
      assert.equal(cell.pixels.filter((_, i) => i % 4 === 3 && cell.pixels[i] > 0).length, 10 * poses[row].height);
    }
  }
});

test("stray pixels in transparent gaps do not create animation rows", () => {
  const { atlas } = unevenAtlas();
  const clean = rowBoundaries(atlas, 3);
  fill(atlas, 1, 48, 3, 1);
  assert.deepEqual(rowBoundaries(atlas, 3), clean);
});

test("meaningful detached parts survive while isolated speckles are removed", () => {
  const atlas = emptyImage(80, 45);
  for (let col = 0; col < 4; col++) {
    fill(atlas, col * 20 + 5, 15, 10, 20);
    fill(atlas, col * 20 + 8, 6, 2, 2); // Detached antenna cap.
    fill(atlas, col * 20 + 2, 2, 1, 1); // Noise.
  }
  const cell = extractCell(atlas, 0, 0, rowBoundaries(atlas, 1));
  assert.equal(cell.pixels[(6 * 20 + 8) * 4 + 3], 255);
  assert.equal(cell.pixels[(2 * 20 + 2) * 4 + 3], 0);
  assert.equal(cell.bounds.top, 6);
});

test("ambiguous row layouts fail instead of silently exporting damaged sprites", () => {
  const { atlas } = unevenAtlas();
  assert.throws(() => rowBoundaries(atlas, 4), /expected 4 distinct pose rows, found 3/);
});

test("uneven columns do not cut an extended pose at a nominal cell edge", () => {
  const atlas = emptyImage(120, 30);
  const poses = [{ left: 3, width: 18 }, { left: 27, width: 22 }, { left: 55, width: 38 }, { left: 98, width: 18 }];
  for (const pose of poses) fill(atlas, pose.left, 5, pose.width, 20);
  const rows = rowBoundaries(atlas, 1);
  const columns = columnBoundaries(atlas, 0, 30);
  for (let col = 0; col < 4; col++) {
    assert.equal(extractCell(atlas, col, 0, rows, columns).bounds.width, poses[col].width);
  }
});

test("source clipping is rejected before normalization can conceal it", () => {
  const atlas = emptyImage(80, 30);
  for (let col = 0; col < 4; col++) fill(atlas, col * 20 + 5, 0, 10, 20);
  assert.throws(() => extractCell(atlas, 0, 0, rowBoundaries(atlas, 1)), /source pose touches crop boundary/);
});

test("Lumi, Nấm and Bắp keep complete heads in every dragged frame", () => {
  for (const source of ["lumi", "nam", "bap"]) {
    const pet = PETS.find((pet) => pet.source === source);
    const atlas = decodePng(join(SOURCES, source, "atlas.png"));
    const boundaries = rowBoundaries(atlas);
    for (let col = 0; col < 4; col++) {
      assert.ok(extractCell(atlas, col, 6, boundaries).bounds.height >= 130, `${source}: full dragged pose ${col}`);
    }
    const frames = prepareFrames(pet);
    for (let col = 0; col < 4; col++) {
      assert.ok(boundsOf(frames[6 * 4 + col]).height >= 30, `${source}: normalized dragged pose ${col}`);
    }
  }
});

test("Mực's dragged edit changes only the four dragged source frames", () => {
  const pet = PETS.find((pet) => pet.source === "muc");
  const edited = prepareFrames({ ...pet, locomotion: undefined });
  const original = prepareFrames({ ...pet, locomotion: undefined, overrides: undefined });
  for (let frame = 0; frame < edited.length; frame++) {
    if (frame >= 24 && frame < 28) assert.ok(!samePixels(edited[frame], original[frame]), `edited dragged ${frame}`);
    else assert.ok(samePixels(edited[frame], original[frame]), `untouched ${frame}`);
  }
});

test("consistent idle and alternating gait edits preserve every other silhouette and scale", () => {
  for (const pet of PETS) {
    const edited = prepareFrames({ ...pet, poses: undefined });
    const original = prepareFrames({ ...pet, poses: undefined, locomotion: undefined });
    const kept = new Set((pet.keep ?? []).map((name) => ROWS.findIndex(([row]) => row === name)));
    for (let frame = 0; frame < edited.length; frame++) {
      if (frame < 12 && !kept.has(Math.floor(frame / 4))) assert.ok(!samePixels(edited[frame], original[frame]), `${pet.source}: motion frame ${frame}`);
      else assert.ok(samePixels(edited[frame], original[frame]), `${pet.source}: untouched frame ${frame}`);
    }
  }
});

test("redrawn walk/run sheets keep the character at the atlas idle height", () => {
  for (const pet of PETS) {
    const edited = prepareFrames({ ...pet, poses: undefined });
    const original = prepareFrames({ ...pet, poses: undefined, locomotion: undefined });
    const idle = (frames) => frames.slice(0, 4).map((f) => boundsOf(f).height).sort((a, b) => a - b)[2];
    assert.ok(Math.abs(idle(edited) - idle(original)) <= 2, `${pet.source}: idle ${idle(edited)}px vs atlas ${idle(original)}px`);
  }
});

test("resampling copies exactly at 1:1 and keeps 1px outlines when reducing", () => {
  const cell = emptyImage(40, 40);
  fill(cell, 4, 4, 32, 32, [240, 200, 80, 255]);
  for (let i = 4; i < 36; i++) {
    cell.pixels.set([30, 20, 20, 255], (4 * 40 + i) * 4); // Top outline.
    cell.pixels.set([30, 20, 20, 255], (i * 40 + 4) * 4); // Left outline.
  }
  const bounds = boundsOf(cell);
  assert.deepEqual(resampleCell({ ...cell, bounds }, 1).pixels, extractRegion(cell, bounds));
  for (const factor of [0.5, 0.9, 0.46]) {
    const small = resampleCell({ ...cell, bounds }, factor);
    const w = small.width;
    for (let i = 1; i < w - 1; i++) {
      assert.deepEqual([...small.pixels.subarray(i * 4, i * 4 + 3)], [30, 20, 20], `factor ${factor}: top outline at ${i}`);
      assert.deepEqual([...small.pixels.subarray(i * w * 4, i * w * 4 + 3)], [30, 20, 20], `factor ${factor}: left outline at ${i}`);
    }
  }
});

function extractRegion(image, b) {
  const out = Buffer.alloc(b.width * b.height * 4);
  for (let y = 0; y < b.height; y++) {
    image.pixels.copy(out, y * b.width * 4, ((b.top + y) * image.width + b.left) * 4, ((b.top + y) * image.width + b.right + 1) * 4);
  }
  return out;
}

test("the app's single dizzy frame is the chosen dazed pose", () => {
  for (const pet of PETS.filter((p) => p.dizzy)) {
    const moved = prepareFrames(pet);
    const unmoved = prepareFrames({ ...pet, dizzy: undefined });
    assert.ok(samePixels(moved[32], unmoved[32 + pet.dizzy]), `${pet.source}: dizzy pose ${pet.dizzy}`);
  }
});

test("dizzy smoothing flattens grain but keeps outlines and high-contrast detail", () => {
  const teal = [40, 140, 150], tealLight = [52, 152, 160], white = [240, 240, 240], outline = [20, 20, 30];
  const palette = [teal, tealLight, white, outline];
  const frame = emptyImage(192, 192);
  // A grainy teal body: flecks of a close colour, with a 2px white bar and a dark line across it.
  for (let y = 40; y < 140; y++) {
    for (let x = 40; x < 140; x++) frame.pixels.set([...((x * 7 + y * 13) % 4 ? teal : tealLight), 255], (y * 192 + x) * 4);
  }
  fill(frame, 60, 80, 40, 2, [...white, 255]);
  fill(frame, 60, 110, 40, 1, [...outline, 255]);
  const frames = Array.from({ length: ROWS.length * 4 }, () => emptyImage(192, 192));
  const dizzy = ROWS.findIndex(([name]) => name === "dizzy") * 4;
  frames[dizzy] = frame;
  frames[0] = { ...frame, pixels: Buffer.from(frame.pixels) };
  const idle = Buffer.from(frames[0].pixels);
  smooth(frames, palette);
  const at = (x, y) => [...frame.pixels.subarray((y * 192 + x) * 4, (y * 192 + x) * 4 + 3)];
  const body = new Set();
  for (let y = 50; y < 70; y++) for (let x = 50; x < 130; x++) body.add(at(x, y).join());
  assert.equal(body.size, 1, "grain is flattened to one colour");
  for (let x = 60; x < 100; x++) {
    assert.deepEqual(at(x, 80), white, `white bar at ${x}`);
    assert.deepEqual(at(x, 110), outline, `dark line at ${x}`);
  }
  assert.ok(frames[0].pixels.equals(idle), "other rows are untouched");
});

test("Momo's pose corrections preserve accepted gait and other states", () => {
  const pet = PETS.find((pet) => pet.source === "momo");
  const edited = prepareFrames(pet);
  const original = prepareFrames({ ...pet, poses: undefined });
  // idle, react, fall, dragged, land and dizzy come from the pose sheets.
  const correctedRows = new Set(["idle", "react", "fall", "dragged", "land", "dizzy"].map((name) => ROWS.findIndex(([row]) => row === name)));
  for (let frame = 0; frame < edited.length; frame++) {
    if (correctedRows.has(Math.floor(frame / 4))) {
      assert.ok(!samePixels(edited[frame], original[frame]), `corrected pose ${frame}`);
    } else {
      assert.ok(samePixels(edited[frame], original[frame]), `preserved pose ${frame}`);
    }
  }
});

test("a regenerated pose sheet with a ref row is sized like the idle and leaves idle untouched", () => {
  // Reuse Bông's 4×3 idle/walk/run sheet as if it were a regenerated fix sheet.
  const pet = PETS.find((p) => p.source === "bong");
  const base = prepareFrames(pet);
  const fixed = prepareFrames({ ...pet, poses: { image: "locomotion-v2.png", rows: ["ref", "sleep", "react"] } });
  for (let col = 0; col < 4; col++) {
    assert.ok(samePixels(fixed[col], base[col]), `idle ${col}`);
    assert.ok(samePixels(fixed[12 + col], base[4 + col]), `sleep ${col}`);
    assert.ok(samePixels(fixed[16 + col], base[8 + col]), `react ${col}`);
  }
});

test("reused poses replay exactly without shifting the pack palette", () => {
  for (const pet of PETS.filter((p) => p.reuse)) {
    const frames = prepareFrames(pet), plain = prepareFrames(pet);
    assert.deepEqual(finishFrames(pet, frames), finishFrames({ ...pet, reuse: undefined, recolor: undefined, fill: undefined }, plain), `${pet.source}: palette`);
    for (const [name, order] of Object.entries(pet.reuse)) {
      const first = ROWS.findIndex(([row]) => row === name) * 4;
      order.forEach((from, col) => assert.ok(samePixels(frames[first + col], plain[first + from]), `${pet.source}: ${name} ${col} replays ${from}`));
    }
  }
});

test("recolor and fill rules leave none of the wrong colour in their frames", () => {
  const nearest = (palette, wanted) => {
    const rgb = [1, 3, 5].map((start) => parseInt(wanted.slice(start, start + 2), 16));
    return palette.reduce((best, c) => (Math.hypot(...c.map((v, i) => v - rgb[i])) < Math.hypot(...best.map((v, i) => v - rgb[i])) ? c : best));
  };
  for (const pet of PETS.filter((p) => p.recolor || p.fill)) {
    const frames = prepareFrames(pet);
    const palette = finishFrames(pet, frames);
    for (const rule of [pet.recolor ?? []].flat()) {
      const [left, top, right, bottom] = rule.box ?? [0, 0, 191, 191];
      const wrong = Object.keys(rule.colors).map((from) => Buffer.from(nearest(palette, from)).toString("hex"));
      for (const name of rule.rows) {
        for (const col of rule.frames ?? [0, 1, 2, 3]) {
          const { pixels } = frames[ROWS.findIndex(([row]) => row === name) * 4 + col];
          for (let y = top; y <= bottom; y++) {
            for (let x = left; x <= right; x++) {
              const p = (y * 192 + x) * 4;
              assert.ok(!pixels[p + 3] || !wrong.includes(pixels.subarray(p, p + 3).toString("hex")), `${pet.source}: ${name} ${col} keeps a recolored colour at ${x},${y}`);
            }
          }
        }
      }
    }
    for (const { row, frame, at: [x, y], color } of pet.fill ?? []) {
      const { pixels } = frames[ROWS.findIndex(([name]) => name === row) * 4 + frame];
      assert.equal(pixels.subarray((y * 192 + x) * 4, (y * 192 + x) * 4 + 3).toString("hex"), Buffer.from(nearest(palette, color)).toString("hex"), `${pet.source}: ${row} ${frame} fill`);
    }
  }
});
