import { describe, expect, it } from "vitest";
import { FixedStep } from "./fixedStep";
import { StateMachine, type StateTable } from "./fsm";
import { Rng } from "./rng";

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
