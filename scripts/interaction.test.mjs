import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import ts from "typescript";

// Exercise the actual interaction class; its imports are types only, so no DOM
// library or duplicate copy of the drag implementation is needed.
const code = ts.transpileModule(readFileSync(new URL("../desktop/src/overlay/interaction.ts", import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { PetInteraction } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

function setup() {
  const target = new EventTarget();
  globalThis.window = target;
  const captured = new Set();
  const calls = { holds: [], releases: [], pokes: 0 };
  const pet = {
    x: 100, y: 200, state: "idle",
    grab() { this.state = "dragged"; },
    dragTo(x, y) { this.x = x; this.y = y; },
    release(vx, vy) { calls.releases.push([vx, vy]); this.state = "fall"; },
    poke() { calls.pokes++; },
  };
  function pointer(type, x = 100, y = 190, time = 0, id = 1) {
    const event = new Event(type);
    Object.defineProperties(event, {
      pointerId: { value: id }, button: { value: 0 },
      clientX: { value: x }, clientY: { value: y }, timeStamp: { value: time },
    });
    target.dispatchEvent(event);
  }
  const element = {
    setPointerCapture(id) { captured.add(id); },
    hasPointerCapture(id) { return captured.has(id); },
    releasePointerCapture(id) {
      captured.delete(id);
      pointer("lostpointercapture", 0, 0, 0, id);
    },
  };
  const interaction = new PetInteraction(pet, { element, hitTest: () => true }, {
    onHold: (held) => calls.holds.push(held), onActivity: () => {},
  });
  function drag() {
    pointer("pointerdown");
    pointer("pointermove", 120, 100, 20);
    assert.equal(pet.state, "dragged");
  }
  return { target, pet, calls, captured, interaction, pointer, drag };
}

test("losing pointer capture releases a dragged pet and restores click-through", () => {
  const { pet, calls, captured, pointer, drag } = setup();
  drag();
  captured.clear();
  pointer("lostpointercapture");
  assert.equal(pet.state, "fall");
  assert.deepEqual(calls.releases, [[0, 0]]);
  assert.deepEqual(calls.holds, [true, false]);
  pointer("pointerup");
  assert.equal(calls.releases.length, 1);
});

test("hiding or pausing during a drag cancels capture without leaving a stuck hold", () => {
  const { pet, calls, captured, interaction, drag } = setup();
  drag();
  interaction.cancel();
  interaction.cancel();
  assert.equal(pet.state, "fall");
  assert.equal(captured.size, 0);
  assert.deepEqual(calls.holds, [true, false]);
  assert.deepEqual(calls.releases, [[0, 0]]);
});

test("a window blur cancels an active drag", () => {
  const { target, pet, calls, drag } = setup();
  drag();
  target.dispatchEvent(new Event("blur"));
  assert.equal(pet.state, "fall");
  assert.deepEqual(calls.releases, [[0, 0]]);
});

test("a capture event from another pointer does not cancel the current drag", () => {
  const { pet, calls, pointer, drag } = setup();
  drag();
  pointer("lostpointercapture", 0, 0, 0, 2);
  assert.equal(pet.state, "dragged");
  assert.deepEqual(calls.holds, [true]);
});

test("a regular click still pokes once when releasing capture emits an event", () => {
  const { calls, captured, pointer } = setup();
  pointer("pointerdown");
  pointer("pointerup", 100, 190, 20);
  assert.equal(calls.pokes, 1);
  assert.equal(captured.size, 0);
  assert.deepEqual(calls.holds, [true, false]);
  assert.deepEqual(calls.releases, []);
});

test("a regular drag still follows the grab offset and throws on release", () => {
  const { pet, calls, pointer, drag } = setup();
  drag();
  pointer("pointermove", 140, 80, 40);
  assert.deepEqual([pet.x, pet.y], [140, 90]);
  pointer("pointerup", 140, 80, 45);
  assert.equal(pet.state, "fall");
  assert.deepEqual(calls.releases, [[1000, -2750]]);
  assert.deepEqual(calls.holds, [true, false]);
});
