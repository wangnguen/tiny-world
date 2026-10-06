import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import ts from "typescript";

// Đường bay của con ma (ghost.ts): file chỉ import kiểu, hàm ghostAt không đụng tới DOM.
const code = ts.transpileModule(readFileSync(new URL("../desktop/src/overlay/ghost.ts", import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { ghostAt, GHOST_SIZE } = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

const bounds = { left: 0, right: 1920, top: 0, floor: 1040 };
const half = GHOST_SIZE.height / 2;

function samples(flight, targets, steps = 2000) {
  return Array.from({ length: steps + 1 }, (_, i) => ghostAt(flight, targets, i / steps));
}

test("bay từ ngoài mép này sang ngoài mép kia", () => {
  for (const dir of [1, -1]) {
    const flight = { bounds, dir, phase: 1 };
    const [first, last] = [ghostAt(flight, [], 0), ghostAt(flight, [], 1)];
    const outside = (p) => p.x + GHOST_SIZE.width / 2 <= bounds.left || p.x - GHOST_SIZE.width / 2 >= bounds.right;
    assert.ok(outside(first) && outside(last));
    assert.equal(Math.sign(last.x - first.x), dir);
  }
});

test("lượn khắp chiều cao màn hình mà không ra ngoài mép trên, mép dưới", () => {
  for (let phase = 0; phase < 6.3; phase += 0.7) {
    const ys = samples({ bounds, dir: 1, phase }, []).map((p) => p.y);
    assert.ok(Math.min(...ys) - half >= bounds.top, `phase ${phase}: thò lên trên`);
    assert.ok(Math.max(...ys) + half <= bounds.floor, `phase ${phase}: thò xuống dưới`);
    assert.ok(Math.min(...ys) < bounds.floor * 0.25, `phase ${phase}: không lên cao`);
    assert.ok(Math.max(...ys) > bounds.floor * 0.75, `phase ${phase}: không xuống thấp`);
  }
});

test("sà qua đúng chỗ từng pet, kể cả pet đứng trên cửa sổ", () => {
  const targets = [
    { x: 300, y: 980 },
    { x: 1500, y: 420 },
  ];
  for (let phase = 0; phase < 6.3; phase += 0.7) {
    for (const dir of [1, -1]) {
      const path = samples({ bounds, dir, phase }, targets);
      for (const target of targets) {
        const near = path.reduce((a, b) => (Math.abs(b.x - target.x) < Math.abs(a.x - target.x) ? b : a));
        assert.ok(Math.abs(near.y - target.y) < 12, `phase ${phase} dir ${dir}: cách pet ${near.y - target.y}px`);
      }
    }
  }
});

test("bay mượt: không giật, nghiêng vừa phải", () => {
  const targets = [
    { x: 900, y: 980 },
    { x: 960, y: 500 },
  ];
  const path = samples({ bounds, dir: 1, phase: 2 }, targets, 14 * 30);
  for (let i = 1; i < path.length; i++) {
    assert.ok(Math.abs(path[i].y - path[i - 1].y) < 40, `bước ${i} nhảy ${path[i].y - path[i - 1].y}px`);
    assert.ok(Math.abs(path[i].tilt) <= 0.45 + 1e-9);
  }
});
