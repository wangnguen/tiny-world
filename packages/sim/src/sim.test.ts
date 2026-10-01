import { describe, expect, it } from "vitest";
import { FixedStep } from "./fixedStep";
import { StateMachine, type StateTable } from "./fsm";
import { Rng } from "./rng";
import { StepBlend } from "./stepBlend";

describe("Rng", () => {
  it("cùng seed cho cùng chuỗi số", () => {
    const a = new Rng(42);
    const b = new Rng(42);
    const seqA = Array.from({ length: 5 }, () => a.next());
    expect(Array.from({ length: 5 }, () => b.next())).toEqual(seqA);
    expect(new Rng(43).next()).not.toBe(seqA[0]);
  });

  it("nằm trong khoảng", () => {
    const rng = new Rng(1);
    for (let i = 0; i < 1000; i++) {
      const v = rng.range(-2, 3);
      expect(v).toBeGreaterThanOrEqual(-2);
      expect(v).toBeLessThan(3);
    }
    expect(() => rng.pick([])).toThrow();
    expect(["a", "b"]).toContain(rng.pick(["a", "b"]));
  });
});

describe("FixedStep", () => {
  it("dồn thời gian lẻ sang lần sau", () => {
    const step = new FixedStep(1 / 30);
    expect(step.advance(1 / 60)).toBe(0);
    expect(step.advance(1 / 60)).toBe(1);
    expect(step.advance(3 / 30)).toBe(3);
  });

  it("chỉ chạy bù tối đa maxSteps bước", () => {
    const step = new FixedStep(1 / 30, 5);
    expect(step.advance(10)).toBe(5);
    expect(step.advance(1 / 30)).toBe(1);
  });

  it("bỏ qua thời gian âm", () => {
    expect(new FixedStep(1 / 30).advance(-1)).toBe(0);
  });

  it("alpha là phần bước kế tiếp đã trôi", () => {
    const step = new FixedStep(1 / 30);
    expect(step.alpha).toBe(0);
    step.advance(1 / 60);
    expect(step.alpha).toBeCloseTo(0.5);
    step.advance(1 / 30);
    expect(step.alpha).toBeCloseTo(0.5);
  });
});

describe("StepBlend", () => {
  it("vẽ giữa hai bước theo alpha", () => {
    const blend = new StepBlend();
    const pet = { x: 0, y: 100 };
    const other = { x: 500, y: 100 };
    blend.step([pet, other], () => {
      pet.x = 50;
      pet.y = 80;
      other.x = 400;
    });
    expect(blend.at(pet, 0)).toEqual({ x: 0, y: 100 });
    expect(blend.at(pet, 0.5)).toEqual({ x: 25, y: 90 });
    expect(blend.at(pet, 1)).toEqual({ x: 50, y: 80 });
    expect(blend.at(other, 0.5)).toEqual({ x: 450, y: 100 });
  });

  it("chưa có bước nào, bị dời ngoài mô phỏng, hoặc không còn trong bước, thì vẽ đúng chỗ", () => {
    const blend = new StepBlend();
    const pet = { x: 10, y: 20 };
    expect(blend.at(pet, 0.5)).toEqual({ x: 10, y: 20 });
    blend.step([pet], () => {
      pet.x = 50;
    });
    pet.x = 300;
    expect(blend.at(pet, 0.5)).toEqual({ x: 300, y: 20 });
    const removed = { x: 0, y: 0 };
    blend.step([removed], () => {
      removed.x = 10;
    });
    blend.step([pet], () => {});
    expect(blend.at(removed, 0.5)).toEqual({ x: 10, y: 0 });
  });
});

describe("StateMachine", () => {
  type S = "a" | "b";

  it("chuyển state khi update trả về state mới, reset thời gian", () => {
    const log: string[] = [];
    const table: StateTable<S, string[]> = {
      a: {
        exit: (l) => l.push("exit a"),
        update: (_l, time) => (time >= 1 ? "b" : undefined),
      },
      b: { enter: (l) => l.push("enter b") },
    };
    const fsm = new StateMachine(table, "a");
    fsm.update(log, 0.5);
    expect(fsm.state).toBe("a");
    expect(fsm.time).toBe(0.5);
    fsm.update(log, 0.5);
    expect(fsm.state).toBe("b");
    expect(fsm.time).toBe(0);
    expect(log).toEqual(["exit a", "enter b"]);
  });

  it("trả về chính state hiện tại thì chạy lại state đó từ đầu", () => {
    let enters = 0;
    const table: StateTable<S, null> = {
      a: { enter: () => enters++, update: (_c, time) => (time >= 1 ? "a" : undefined) },
      b: {},
    };
    const fsm = new StateMachine(table, "a");
    fsm.update(null, 1);
    expect(fsm.state).toBe("a");
    expect(fsm.time).toBe(0);
    expect(enters).toBe(1);
  });
});
