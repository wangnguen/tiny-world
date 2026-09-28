import { describe, expect, it } from "vitest";
import { FixedStep } from "./fixedStep";
import { StateMachine, type StateTable } from "./fsm";
import { REACT_DURATION } from "./pet";
import { Rng } from "./rng";
import { World } from "./world";

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
  const log: string[] = [];
  const table: StateTable<S, string[]> = {
    a: {
      exit: (l) => l.push("exit a"),
      update: (_l, time) => (time >= 1 ? "b" : undefined),
    },
    b: { enter: (l) => l.push("enter b") },
  };

  it("chuyển state khi update trả về state mới, reset thời gian", () => {
    const fsm = new StateMachine(table, "a");
    fsm.update(log, 0.5);
    expect(fsm.state).toBe("a");
    expect(fsm.time).toBe(0.5);
    fsm.update(log, 0.5);
    expect(fsm.state).toBe("b");
    expect(fsm.time).toBe(0);
    expect(log).toEqual(["exit a", "enter b"]);
  });
});

describe("World / Pet", () => {
  const floor = { left: 0, right: 1000, y: 700 };

  it("pet xuất hiện trên mặt đất, nằm trọn trong màn hình", () => {
    const world = new World(floor, 1);
    const pet = world.spawn({ id: "p", x: 990, width: 80, height: 80 });
    expect(pet.y).toBe(700);
    expect(pet.x).toBe(960);
    expect(world.spawn({ id: "q", x: -50, width: 80, height: 80 }).x).toBe(40);
  });

  it("click thì phản ứng rồi quay về idle", () => {
    const world = new World(floor, 1);
    const pet = world.spawn({ id: "p", x: 500, width: 80, height: 80 });
    pet.poke();
    expect(pet.state).toBe("react");
    world.step(REACT_DURATION / 2);
    expect(pet.state).toBe("react");
    world.step(REACT_DURATION / 2);
    expect(pet.state).toBe("idle");
  });

  it("click lại khi đang phản ứng thì phản ứng lại từ đầu", () => {
    const world = new World(floor, 1);
    const pet = world.spawn({ id: "p", x: 500, width: 80, height: 80 });
    pet.poke();
    world.step(REACT_DURATION * 0.8);
    pet.poke();
    expect(pet.stateTime).toBe(0);
    world.step(REACT_DURATION * 0.8);
    expect(pet.state).toBe("react");
  });
});
