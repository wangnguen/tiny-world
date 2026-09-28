import assert from "node:assert/strict";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import {
  PETS, boundsOf, columnBoundaries, decodePng, emptyImage, extractCell, prepareFrames, rowBoundaries,
} from "./prepare-sprites.mjs";

const SOURCES = fileURLToPath(new URL("../assets/sprite-sources/", import.meta.url));

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

test("Bíp's pose edit changes only the four dragged source frames", () => {
  const pet = PETS.find((pet) => pet.source === "bip");
  const edited = prepareFrames(pet);
  const original = prepareFrames({ ...pet, overrides: undefined });
  for (let frame = 0; frame < edited.length; frame++) {
    if (frame >= 24 && frame < 28) assert.notDeepEqual(edited[frame].pixels, original[frame].pixels);
    else assert.deepEqual(edited[frame].pixels, original[frame].pixels);
  }
});
