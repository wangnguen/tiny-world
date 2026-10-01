import { describe, expect, it } from "vitest";
import type { Pet } from "./pet";
import { TUNING } from "./tuning";
import { World } from "./world";

const DT = 1 / 30;
const bounds = { left: 0, right: 1000, top: 0, floor: 700 };
/** Màn hình bên phải cao bằng, cùng DPI. */
const right = { x: 1000, y: 0, width: 800, height: 700 };

/** Một nhóm pet 60 px đứng ở các chỗ `xs`. */
function setup(xs: number[], seed = 1) {
  const world = new World(bounds, seed);
  world.setScreen(bounds, [right]);
  const pets = xs.map((x, i) => world.spawn({ id: `p${i}`, x, width: 60, height: 60 }));
  return { world, pets };
}

/** Chạy `seconds` giây; `until` đúng thì dừng sớm, trả về `true`. */
function run(world: World, seconds: number, until?: () => boolean): boolean {
  for (let t = 0; t < seconds; t += DT) {
    world.step(DT);
    if (until?.()) return true;
  }
  return false;
}

/** Overlay sang màn hình bên phải: gốc toạ độ dời sang phải 1000 px. */
function overlayMovesRight(world: World) {
  world.setScreen({ left: 0, right: 800, top: 0, floor: 700 }, [{ x: -1000, y: 0, width: 1000, height: 700 }], {
    scale: 1,
    x: -1000,
    y: 0,
  });
}

/** Ném `pet` sang màn hình bên phải, chạy tới lúc overlay cần sang. */
function throwRight(world: World, pet: Pet) {
  pet.grab();
  pet.dragTo(900, 300);
  pet.release(1500, -200);
  expect(run(world, 1, () => pet.leaving !== null)).toBe(true);
}

describe("nhiều pet", () => {
  it("bỏ một con thì các con còn lại vẫn sống tiếp", () => {
    const { world, pets } = setup([200, 500, 800]);
    world.remove("p1");
    expect(world.pets.map((p) => p.id)).toEqual(["p0", "p2"]);
    run(world, 5);
    expect(pets[0].y).toBe(700);
  });

  it("đồng hồ không đụng tới tính chung: click một con thì cả nhóm cùng tỉnh táo, rồi cùng đi ngủ", () => {
    const { world, pets } = setup([200, 500, 800]);
    run(world, TUNING.sleepAfter - 30);
    pets[1].poke();
    run(world, 1);
    for (const pet of pets) expect(pet.sinceInteraction).toBeLessThan(2);
    expect(run(world, TUNING.sleepAfter + 20, () => world.resting)).toBe(true);
    for (const pet of pets) expect(pet.state).toBe("sleep");
  });

  it("con mới thêm vào tính chung đồng hồ với cả nhóm", () => {
    const { world, pets } = setup([200]);
    run(world, 60);
    const late = world.spawn({ id: "late", x: 600, width: 60, height: 60 });
    expect(late.sinceInteraction).toBe(pets[0].sinceInteraction);
  });

  it("click ở đâu cũng đánh thức cả nhóm", () => {
    const { world, pets } = setup([200, 500]);
    for (const pet of pets) pet.sinceInteraction = 1e6;
    expect(run(world, 60, () => world.resting)).toBe(true);
    world.wakeAll();
    for (const pet of pets) expect(pet.state).not.toBe("sleep");
  });

  it("con khác đang đi lại thì con này hay đứng yên hơn: ít khi cả nhóm cùng đi một lúc", () => {
    const tuning = TUNING as { groupCalm: number };
    const together = (calm: number) => {
      tuning.groupCalm = calm;
      let all = 0;
      for (let seed = 1; seed <= 5; seed++) {
        const { world } = setup([150, 500, 850], seed);
        for (let t = 0; t < 120; t += DT) {
          world.step(DT);
          if (world.pets.every((p) => p.state === "walk" || p.state === "run")) all++;
        }
      }
      return all;
    };
    const original = tuning.groupCalm;
    try {
      expect(together(original)).toBeLessThan(together(0) * 0.6);
    } finally {
      tuning.groupCalm = original;
    }
  });

  it("từ 2 con trở lên thì không tự đi sang màn hình khác", () => {
    for (let seed = 1; seed <= 10; seed++) {
      const { world } = setup([850, 950], seed);
      expect(run(world, 60, () => world.pets.some((p) => p.crossing !== null))).toBe(false);
    }
    // Chỉ còn một con thì lại đi sang như trước.
    const crossed = [1, 2, 3, 4, 5].some((seed) => {
      const { world } = setup([950], seed);
      return run(world, 60, () => world.pets[0].leaving !== null);
    });
    expect(crossed).toBe(true);
  });
});

describe("ném một con sang màn hình khác", () => {
  it("vẫn bay sang được, các con còn lại chạy theo vào từ mép, con gần mép vào trước", () => {
    const { world, pets } = setup([100, 400, 800]);
    const [far, near, thrown] = pets;
    throwRight(world, thrown);
    overlayMovesRight(world);
    // Con bị ném đang bay vào; hai con kia ở ngoài mép trái của màn hình mới, chạy vào.
    expect(thrown.crossing?.phase).toBe("in");
    for (const pet of [far, near]) {
      expect(pet.state).toBe("run");
      expect(pet.facing).toBe(1);
      expect(pet.crossing).toEqual({ dir: 1, phase: "in", elapsed: 0 });
      expect(pet.x).toBeLessThan(-30);
      expect(pet.y).toBe(700);
    }
    expect(near.x).toBeGreaterThan(far.x);
    expect(near.x - far.x).toBeGreaterThanOrEqual(60 * 0.8);
    // Cả nhóm vào hẳn màn hình mới trong vài giây.
    expect(run(world, 8, () => pets.every((p) => p.crossing === null))).toBe(true);
    for (const pet of pets) {
      expect(pet.x).toBeGreaterThanOrEqual(30);
      expect(pet.x).toBeLessThanOrEqual(770);
    }
  });

  it("con ở xa cũng không bắt chờ quá lâu", () => {
    const { world, pets } = setup([40, 900]);
    throwRight(world, pets[1]);
    overlayMovesRight(world);
    // Cách mép cũ gần 1000 px nhưng chỉ xuất hiện cách mép mới tối đa 320 px.
    expect(pets[0].x).toBeGreaterThanOrEqual(-30 - 320);
  });

  it("con đang đứng trên cửa sổ ở màn hình cũ thì xuống đất rồi chạy theo", () => {
    const { world, pets } = setup([300, 800]);
    const [mounted, thrown] = pets;
    world.setWindows([{ id: 7, rect: { x: 200, y: 400, width: 300, height: 300 } }]);
    mounted.grab();
    mounted.dragTo(300, 380);
    mounted.release(0, 0);
    run(world, 1, () => mounted.mount !== null && mounted.state === "idle");
    expect(mounted.mount?.id).toBe(7);
    throwRight(world, thrown);
    overlayMovesRight(world);
    expect(mounted.mount).toBeNull();
    expect(mounted.y).toBe(700);
    expect(mounted.state).toBe("run");
  });

  it("con đang ngủ thì nằm luôn ở sát mép màn hình mới", () => {
    const { world, pets } = setup([200, 800]);
    const [sleeper, thrown] = pets;
    world.step(DT);
    sleeper.restore({ id: sleeper.id, x: 200, facing: 1, asleep: true, sinceInteraction: 0 });
    throwRight(world, thrown);
    overlayMovesRight(world);
    expect(sleeper.state).toBe("sleep");
    expect(sleeper.x).toBe(30);
  });

  it("màn hình đổi tại chỗ (DPI, taskbar) thì không con nào chạy theo ai", () => {
    const { world, pets } = setup([200, 500, 800]);
    world.setScreen({ ...bounds, floor: 680 }, [right], { scale: 1, x: 0, y: 0 });
    for (const pet of pets) expect(pet.crossing).toBeNull();
  });
});
