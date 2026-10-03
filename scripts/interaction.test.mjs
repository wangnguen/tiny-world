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

function setup(foot, hooks = {}) {
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
  function pointer(type, x = 100, y = 190, time = 0, id = 1, shiftKey = false) {
    const event = new Event(type);
    Object.defineProperties(event, {
      pointerId: { value: id }, button: { value: 0 },
      clientX: { value: x }, clientY: { value: y }, timeStamp: { value: time }, shiftKey: { value: shiftKey },
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
  const view = { element, foot, raises: 0, raise() { this.raises++; } };
  const interaction = new PetInteraction(() => ({ pet, view }), {
    onHold: (held) => calls.holds.push(held), onActivity: () => {},
    ...hooks,
  });
  function drag() {
    pointer("pointerdown");
    pointer("pointermove", 120, 100, 20);
    assert.equal(pet.state, "dragged");
  }
  return { target, pet, view, calls, captured, interaction, pointer, drag };
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

test("Shift + click calls the aura hook instead of poking or double-clicking", () => {
  const auras = [];
  const { pet, calls, pointer } = setup(undefined, { onAura: (target) => auras.push(target) });
  pointer("pointerdown", 100, 190, 100, 1, true);
  pointer("pointerup", 100, 190, 120, 1, true);
  assert.deepEqual(auras, [pet]);
  assert.equal(calls.pokes, 0);
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

test("grabbing a flying pet keeps it where it was drawn, not where the sim already is", () => {
  const { pet, pointer } = setup({ x: 80, y: 170 });
  pointer("pointerdown");
  pointer("pointermove", 120, 100, 20);
  assert.deepEqual([pet.x, pet.y], [100, 80]);
});

test("pressing a pet raises it above the others", () => {
  const { view, pointer } = setup();
  pointer("pointerdown");
  assert.equal(view.raises, 1);
});

test("with several pets only the one under the cursor is grabbed, empty spots go through", () => {
  globalThis.window = new EventTarget();
  const make = (name) => ({
    name, x: 0, y: 0, state: "idle", pokes: 0,
    grab() { this.state = "dragged"; }, dragTo(x, y) { this.x = x; this.y = y; },
    release() { this.state = "fall"; }, poke() { this.pokes++; },
  });
  const element = { setPointerCapture() {}, hasPointerCapture() { return false; }, releasePointerCapture() {} };
  const a = { pet: make("a"), view: { element, raise() {} } };
  const b = { pet: make("b"), view: { element, raise() {} } };
  const interaction = new PetInteraction((p) => (p.x < 100 ? a : p.x < 200 ? b : null), {
    onHold: () => {}, onActivity: () => {},
  });
  const fire = (type, x, time = 0) => {
    const event = new Event(type);
    Object.defineProperties(event, {
      pointerId: { value: 1 }, button: { value: 0 },
      clientX: { value: x }, clientY: { value: 50 }, timeStamp: { value: time },
    });
    window.dispatchEvent(event);
  };
  fire("pointerdown", 150);
  assert.equal(interaction.held, b.pet);
  fire("pointerup", 150, 10);
  assert.deepEqual([a.pet.pokes, b.pet.pokes], [0, 1]);
  fire("pointerdown", 300);
  assert.equal(interaction.held, null);
  fire("pointerup", 300, 20);
  assert.deepEqual([a.pet.pokes, b.pet.pokes], [0, 1]);
});

test("two quick clicks on the same pet are a double click, a third click starts over", () => {
  globalThis.window = new EventTarget();
  const pet = { pokes: 0, poke() { this.pokes++; } };
  const element = { setPointerCapture() {}, hasPointerCapture() { return false; }, releasePointerCapture() {} };
  const doubles = [];
  new PetInteraction(() => ({ pet, view: { element, raise() {} } }), {
    onHold: () => {}, onActivity: () => {}, onDoubleClick: (p) => doubles.push(p),
  });
  const click = (time) => {
    for (const type of ["pointerdown", "pointerup"]) {
      const event = new Event(type);
      Object.defineProperties(event, {
        pointerId: { value: 1 }, button: { value: 0 },
        clientX: { value: 10 }, clientY: { value: 10 }, timeStamp: { value: time },
      });
      window.dispatchEvent(event);
    }
  };
  click(0);
  click(250);
  assert.equal(doubles.length, 1);
  click(500);
  assert.equal(doubles.length, 1);
  // Hai lần cách nhau lâu thì không phải bấm đúp.
  click(2000);
  assert.equal(doubles.length, 1);
  assert.equal(pet.pokes, 4);
});
