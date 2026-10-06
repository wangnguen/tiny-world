import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import ts from "typescript";

// Exercise the real combo state machine; its imports are types only, so fake
// pets/views/effects are enough (the sim side is covered by blink.test.ts).
const code = ts.transpileModule(readFileSync(new URL("../desktop/src/overlay/longCombo.ts", import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { LongCombo, TRANSFORM_MS, WAIT_MS, VANISH_SECONDS } = await import(
  `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`
);

/** A fighter standing at `x`; `busy` pets refuse `hold` like a dragged/falling/climbing pet. */
function fighter(x, y = 700) {
  const pet = {
    x, y, facing: 1, state: "idle", vanished: 0, startled: 0, busy: false,
    hold(facing) {
      if (this.busy || this.state === "dragged") return false;
      this.facing = facing;
      this.state = "idle";
      return true;
    },
    blinkTo(other, x) {
      this.x = x;
      this.y = other.y;
      this.state = "idle";
      if (Math.abs(other.x - x) > 1) this.facing = other.x > x ? 1 : -1;
    },
    startle() { this.startled++; this.state = "react"; },
    vanish(seconds) { this.vanished = seconds; },
  };
  const view = { width: 96, height: 96, covered: false, setCovered(covered) { this.covered = covered; } };
  return { pet, view };
}

function kit() {
  const log = [];
  return {
    log,
    action: { play: (action, _now, facing) => log.push(["play", action, facing]), stop: () => log.push(["stop"]) },
    teleport: { play: (point, _w, _h, direction, reverse) => log.push(["blink", reverse ? "in" : "out", direction, point.x]) },
  };
}

/** Run `combo` every 10 ms from `from` to `to` (or until it is done); `each(t)` runs before each update. */
function run(combo, from, to, each) {
  let t = from;
  for (; t <= to && !combo.done; t += 10) {
    each?.(t);
    combo.update(t);
  }
  return t;
}

test("Long transforms, then blinks next to each pet in turn and swipes it away", () => {
  const long = fighter(100);
  const a = fighter(400);
  const b = fighter(900, 400);
  const k = kit();
  const combo = new LongCombo(long, k, [a, b], 0);
  assert.equal(long.view.covered, true);
  run(combo, 0, TRANSFORM_MS - 10);
  assert.equal(a.pet.startled, 0);
  run(combo, TRANSFORM_MS, 20_000);
  assert.equal(combo.done, true);
  assert.equal(long.view.covered, false);
  for (const target of [a, b]) {
    assert.equal(target.pet.startled, 1);
    assert.equal(target.pet.vanished, VANISH_SECONDS);
  }
  const plays = k.log.filter(([kind]) => kind === "play");
  assert.deepEqual(plays, [["play", "transform", 1], ["play", "tail-swipe", 1], ["play", "tail-swipe", 1]]);
  // Out from where Long stood, in next to the target, on the target's own ground.
  const blinks = k.log.filter(([kind]) => kind === "blink");
  assert.deepEqual(blinks.map(([, way]) => way), ["out", "in", "out", "in"]);
  assert.equal(blinks[0][3], 100);
  assert.ok(Math.abs(blinks[1][3] - (400 - 192 * 0.42)) < 1e-9);
  assert.equal(long.pet.y, 400);
});

test("the swipe keeps facing the target even if the sim turns Long toward the cursor", () => {
  const long = fighter(800);
  const target = fighter(300);
  const k = kit();
  const combo = new LongCombo(long, k, [target], 0);
  // Idle pets look at the cursor; the user just clicked where Long used to stand (to the right).
  run(combo, 0, 20_000, () => {
    long.pet.facing = 1;
    target.pet.facing = 1;
  });
  assert.deepEqual(k.log.filter(([kind]) => kind === "play")[1], ["play", "tail-swipe", -1]);
  assert.equal(target.pet.startled, 1);
});

test("a busy target is waited for, then skipped; the next one is still hit", () => {
  const long = fighter(100);
  const busy = fighter(400);
  const free = fighter(700);
  busy.pet.busy = true;
  const combo = new LongCombo(long, kit(), [busy, free], 0);
  const end = run(combo, 0, 20_000);
  assert.equal(busy.pet.vanished, 0);
  assert.equal(busy.pet.startled, 0);
  assert.equal(free.pet.vanished, VANISH_SECONDS);
  assert.ok(end > TRANSFORM_MS + WAIT_MS);
});

test("a target that gets free while Long waits is hit", () => {
  const long = fighter(100);
  const target = fighter(400);
  target.pet.busy = true;
  const combo = new LongCombo(long, kit(), [target], 0);
  run(combo, 0, 20_000, (t) => {
    if (t === TRANSFORM_MS + 500) target.pet.busy = false;
  });
  assert.equal(target.pet.vanished, VANISH_SECONDS);
});

test("picking Long up mid-combo stops it and shows Long again", () => {
  const long = fighter(100);
  const target = fighter(400);
  const k = kit();
  const combo = new LongCombo(long, k, [target], 0);
  run(combo, 0, TRANSFORM_MS + 300);
  long.pet.state = "dragged";
  combo.update(TRANSFORM_MS + 310);
  assert.equal(combo.done, true);
  assert.equal(long.view.covered, false);
  assert.deepEqual(k.log.at(-1), ["stop"]);
  run(combo, TRANSFORM_MS + 320, 20_000);
  assert.equal(target.pet.vanished, 0);
});

test("removing Long from the screen stops the combo", () => {
  const long = fighter(100);
  const target = fighter(400);
  const combo = new LongCombo(long, kit(), [target], 0);
  combo.drop(long);
  assert.equal(combo.done, true);
  assert.equal(long.view.covered, false);
});

test("a target removed or picked up mid-attack is spared; Long moves on to the next one", () => {
  const long = fighter(100);
  const removed = fighter(400);
  const grabbed = fighter(600);
  const last = fighter(800);
  const combo = new LongCombo(long, kit(), [removed, grabbed, last], 0);
  // Removed during the blink to it.
  run(combo, 0, TRANSFORM_MS + 100);
  combo.drop(removed);
  // Picked up by the user while Long swings at it.
  run(combo, TRANSFORM_MS + 110, 20_000, () => {
    if (long.view.covered && long.pet.x > 400 && long.pet.x < 600) grabbed.pet.state = "dragged";
  });
  assert.equal(removed.pet.vanished, 0);
  assert.equal(grabbed.pet.vanished, 0);
  assert.equal(grabbed.pet.startled, 0);
  assert.equal(last.pet.vanished, VANISH_SECONDS);
  assert.equal(long.view.covered, false);
});

test("with no target left the combo ends and Long is shown again", () => {
  const long = fighter(100);
  const target = fighter(400);
  target.pet.busy = true;
  const combo = new LongCombo(long, kit(), [target], 0);
  run(combo, 0, 20_000);
  assert.equal(combo.done, true);
  assert.equal(long.view.covered, false);
  assert.equal(target.pet.startled, 0);
});
