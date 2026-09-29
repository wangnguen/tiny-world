import { describe, expect, it } from "vitest";
import type { Pet } from "./pet";
import { TUNING } from "./tuning";
import { World } from "./world";

const DT = 1 / 30;
const bounds = { left: 0, right: 1000, top: 0, floor: 700 };

interface Sim {
  world: World;
  pet: Pet;
  /** Đồng hồ (giây) cho các lần báo vị trí con trỏ. */
  clock: number;
}

function setup(seed = 1, x = 500): Sim {
  const world = new World(bounds, seed);
  const pet = world.spawn({ id: "p", x, width: 60, height: 60 });
  return { world, pet, clock: 0 };
}

/**
 * Chạy `seconds` giây; mỗi bước hỏi `cursor` vị trí con trỏ (theo số giây đã chạy) rồi báo cho world,
 * như Rust báo khoảng 60 lần/giây. `until` đúng thì dừng sớm, trả về `true`.
 */
function run(
  sim: Sim,
  seconds: number,
  cursor?: (t: number) => { x: number; y: number; pressed?: boolean } | null,
  until?: (pet: Pet) => boolean,
): boolean {
  for (let t = 0; t < seconds; t += DT) {
    for (const half of [0, DT / 2]) {
      const at = cursor?.(t + half);
      if (at) sim.world.moveCursor(at.x, at.y, at.pressed ?? false, sim.clock + half);
    }
    sim.clock += DT;
    sim.world.step(DT);
    if (until?.(sim.pet)) return true;
  }
  return false;
}

/** Thử từng seed tới khi `scenario` trả về `true`; 20 seed không cái nào được thì test lỗi. */
function someSeed(scenario: (seed: number) => boolean): number {
  for (let seed = 1; seed <= 20; seed++) if (scenario(seed)) return seed;
  throw new Error("Không seed nào xảy ra.");
}

/** Đợi pet đứng yên (`idle`) rồi mới bắt đầu. */
function untilIdle(sim: Sim) {
  run(sim, 10, undefined, (pet) => pet.state === "idle" && pet.stateTime < DT * 1.5);
}

describe("nhìn theo con trỏ", () => {
  it("đứng yên thì quay về phía con trỏ ở gần", () => {
    const sim = setup();
    untilIdle(sim);
    const side = -sim.pet.facing;
    run(sim, 0.2, () => ({ x: sim.pet.x + side * 120, y: 650 }));
    expect(sim.pet.facing).toBe(side);
  });

  it("con trỏ ngay trên đầu hoặc ở xa thì không quay", () => {
    const sim = setup();
    untilIdle(sim);
    const facing = sim.pet.facing;
    run(sim, 0.2, () => ({ x: sim.pet.x - facing * 5, y: 600 }));
    expect(sim.pet.facing).toBe(facing);
    run(sim, 0.2, () => ({ x: sim.pet.x - facing * 400, y: 650 }));
    expect(sim.pet.facing).toBe(facing);
  });
});

describe("lại gần ngửi", () => {
  it("con trỏ đứng yên ngang tầm thì đi tới cạnh, quay mặt vào", () => {
    const sim = setup();
    const target = { x: 650, y: 680 };
    const arrived = run(
      sim,
      12,
      () => target,
      (pet) => pet.state === "idle" && Math.abs(pet.x - (target.x - 60 * TUNING.sniffGap)) < 1,
    );
    expect(arrived).toBe(true);
    expect(sim.pet.facing).toBe(1);
  });

  it("con trỏ vừa dời sang chỗ khác thì chờ nó đứng yên lại rồi mới lại gần", () => {
    const sim = setup();
    run(sim, 3, () => ({ x: 900, y: 100 }));
    sim.world.moveCursor(650, 680, false, sim.clock);
    const early = run(sim, TUNING.sniffDelay - 0.2, () => ({ x: 650, y: 680 }), (pet) => pet.goal?.kind === "sniff");
    expect(early).toBe(false);
  });

  it("con trỏ ở cao quá tầm thì không lại gần", () => {
    const sim = setup();
    const sniffed = run(sim, 12, () => ({ x: 650, y: 400 }), (pet) => pet.goal?.kind === "sniff");
    expect(sniffed).toBe(false);
  });
});

describe("đuổi theo con trỏ", () => {
  it("con trỏ lướt qua chậm ngang tầm thì có lúc chạy đuổi theo", () => {
    someSeed((seed) => {
      const sim = setup(seed);
      untilIdle(sim);
      const start = sim.pet.x + 150;
      // Con trỏ lướt sang phải 300 px/s, sát mặt đất.
      const chased = run(sim, 3, (t) => ({ x: start + 300 * t, y: 680 }), (pet) => pet.goal?.kind === "chase");
      if (!chased) return false;
      expect(sim.pet.state).toBe("run");
      expect(sim.pet.facing).toBe(1);
      return true;
    });
  });

  it("đuổi kịp thì đứng lại", () => {
    someSeed((seed) => {
      const sim = setup(seed);
      untilIdle(sim);
      const start = sim.pet.x + 150;
      if (!run(sim, 3, (t) => ({ x: start + 300 * t, y: 680 }), (pet) => pet.goal?.kind === "chase")) return false;
      // Con trỏ dừng lại tại chỗ, pet chạy tới sát.
      const stop = { x: sim.world.cursor!.x, y: 680 };
      expect(run(sim, 3, () => stop, (pet) => pet.state === "idle")).toBe(true);
      expect(Math.abs(stop.x - sim.pet.x)).toBeLessThanOrEqual(60 * TUNING.sniffGap + 5);
      return true;
    });
  });

  it("đang giữ chuột (kéo cửa sổ, bôi đen chữ) thì không đuổi", () => {
    for (let seed = 1; seed <= 10; seed++) {
      const sim = setup(seed);
      untilIdle(sim);
      const start = sim.pet.x + 150;
      const chased = run(
        sim,
        3,
        (t) => ({ x: start + 300 * t, y: 680, pressed: true }),
        (pet) => pet.goal?.kind === "chase",
      );
      expect(chased).toBe(false);
    }
  });
});

describe("né và giật mình", () => {
  it("con trỏ lao tới nhanh thì chạy né về phía ngược lại", () => {
    const sim = setup();
    untilIdle(sim);
    const from = sim.pet.x - 140;
    run(sim, 0.1, (t) => ({ x: from + 1500 * t, y: 670 }));
    expect(sim.pet.goal?.kind).toBe("flee");
    expect(sim.pet.state).toBe("run");
    expect(sim.pet.facing).toBe(1);
  });

  it("giật chuột quét qua người thì nhảy dựng lên rồi ngã choáng", () => {
    const sim = setup();
    untilIdle(sim);
    const x0 = sim.pet.x;
    // Quét ngang qua bụng pet với tốc độ 4000 px/s.
    run(sim, 0.1, (t) => ({ x: x0 - 200 + 4000 * t, y: 670 }));
    expect(sim.pet.state).toBe("react");
    expect(run(sim, 2, undefined, (pet) => pet.state === "dizzy")).toBe(true);
  });

  it("giật chuột ở xa thì không sao, giật mình xong một lúc sau mới giật mình lại", () => {
    const sim = setup();
    untilIdle(sim);
    const x0 = sim.pet.x;
    run(sim, 0.1, (t) => ({ x: x0 - 200 + 4000 * t, y: 400 }));
    expect(sim.pet.state).not.toBe("react");

    run(sim, 0.1, (t) => ({ x: x0 - 200 + 4000 * t, y: 670 }));
    expect(sim.pet.state).toBe("react");
    run(sim, TUNING.reactTime + TUNING.dizzyTime + 0.2);
    untilIdle(sim);
    const x1 = sim.pet.x;
    run(sim, 0.1, (t) => ({ x: x1 + 200 - 4000 * t, y: 670 }));
    expect(sim.pet.state).not.toBe("react");
  });

  it("đang ngủ thì con trỏ không làm gì được", () => {
    const sim = setup();
    sim.pet.sinceInteraction = TUNING.sleepAfter;
    run(sim, 10, undefined, (pet) => pet.state === "sleep");
    expect(sim.pet.state).toBe("sleep");
    const x0 = sim.pet.x;
    run(sim, 3, (t) => ({ x: x0 - 200 + 4000 * (t % 0.1), y: 670 }));
    expect(sim.pet.state).toBe("sleep");
  });
});

describe("sang màn hình bên cạnh", () => {
  /** Màn hình bên phải cao bằng, cùng DPI. */
  const right = { x: 1000, y: 0, width: 800, height: 700 };

  function leave(seed: number): Sim | null {
    const sim = setup(seed, 900);
    sim.world.setScreen(bounds, [right]);
    return run(sim, 40, undefined, (pet) => pet.leaving !== null) ? sim : null;
  }

  it("đi tới mép giáp màn hình khác thì có lúc đi ra khỏi mép, chờ overlay sang bên kia", () => {
    let sim: Sim | null = null;
    someSeed((seed) => (sim = leave(seed)) !== null);
    const pet = sim!.pet;
    expect(pet.leaving).toEqual({ x: pet.x, y: 699 });
    expect(pet.x).toBeGreaterThan(1000);
  });

  it("overlay sang rồi thì đi vào hẳn màn hình mới", () => {
    let sim: Sim | null = null;
    someSeed((seed) => (sim = leave(seed)) !== null);
    const { world, pet } = sim!;
    const x = pet.x;
    // Overlay sang màn hình bên phải: gốc toạ độ dời sang phải 1000 px.
    world.setScreen({ left: 0, right: 800, top: 0, floor: 700 }, [{ x: -1000, y: 0, width: 1000, height: 700 }], {
      scale: 1,
      x: -1000,
      y: 0,
    });
    expect(pet.x).toBeCloseTo(x - 1000);
    expect(pet.leaving).toBeNull();
    run(sim!, 3, undefined, (p) => p.crossing === null);
    expect(pet.crossing).toBeNull();
    expect(pet.x).toBeGreaterThanOrEqual(30);
    expect(pet.y).toBe(700);
  });

  it("overlay không sang được thì quay lại", () => {
    let sim: Sim | null = null;
    someSeed((seed) => (sim = leave(seed)) !== null);
    run(sim!, TUNING.crossTimeout + 3, undefined, (p) => p.crossing === null);
    expect(sim!.pet.crossing).toBeNull();
    expect(sim!.pet.x).toBeLessThanOrEqual(970);
  });

  it("đang vắt qua mép thì cửa sổ kéo tới cũng không làm pet quay đầu đi lạc ra ngoài", () => {
    let sim: Sim | null = null;
    someSeed((seed) => (sim = leave(seed)) !== null);
    const { world, pet } = sim!;
    world.setWindows([{ id: 9, rect: { x: 700, y: 560, width: 150, height: 100 } }]);
    world.setWindows([{ id: 9, rect: { x: 800, y: 560, width: 150, height: 100 } }]);
    expect(pet.goal).toBeNull();
    // Không sang được: quay vào, cả con nằm lại trong màn hình.
    run(sim!, TUNING.crossTimeout + 4, undefined, (p) => p.crossing === null);
    expect(pet.crossing).toBeNull();
    expect(pet.x).toBeLessThanOrEqual(970);
    run(sim!, 5);
    expect(pet.x).toBeGreaterThanOrEqual(30);
    expect(pet.x).toBeLessThanOrEqual(970);
  });

  it("đã sang màn hình mới mà quay đầu ra lại thì vào hẳn trong màn hình", () => {
    let sim: Sim | null = null;
    someSeed((seed) => (sim = leave(seed)) !== null);
    const { world, pet } = sim!;
    world.setScreen({ left: 0, right: 800, top: 0, floor: 700 }, [{ x: -1000, y: 0, width: 1000, height: 700 }], {
      scale: 1,
      x: -1000,
      y: 0,
    });
    pet.facing = -1;
    run(sim!, DT);
    expect(pet.crossing).toBeNull();
    expect(pet.x).toBe(30);
  });

  it("màn hình bên cạnh thấp hơn (mặt đất ở cao hơn) thì không đi sang", () => {
    for (let seed = 1; seed <= 10; seed++) {
      const sim = setup(seed, 900);
      sim.world.setScreen(bounds, [{ ...right, height: 500 }]);
      expect(run(sim, 40, undefined, (pet) => pet.crossing !== null)).toBe(false);
    }
  });

  it("ném về phía màn hình bên cạnh thì bay qua mép, không nảy lại", () => {
    const sim = setup(1, 800);
    sim.world.setScreen(bounds, [right]);
    sim.pet.grab();
    sim.pet.dragTo(900, 300);
    sim.pet.release(1500, -200);
    expect(run(sim, 1, undefined, (pet) => pet.leaving !== null)).toBe(true);
    expect(sim.pet.state).toBe("fall");
  });

  it("không có màn hình bên cạnh thì ném vào mép vẫn nảy lại", () => {
    const sim = setup(1, 800);
    sim.pet.grab();
    sim.pet.dragTo(900, 300);
    sim.pet.release(1500, -200);
    run(sim, 1);
    expect(sim.pet.crossing).toBeNull();
    expect(sim.pet.x).toBeLessThanOrEqual(970);
  });
});

describe("màn hình đổi độ phân giải, DPI, taskbar", () => {
  it("taskbar cao lên thì pet đứng lên mặt đất mới, thấp xuống thì rơi xuống", () => {
    const sim = setup();
    untilIdle(sim);
    sim.world.setScreen({ ...bounds, floor: 650 }, []);
    expect(sim.pet.y).toBe(650);
    sim.world.setScreen({ ...bounds, floor: 750 }, []);
    expect(sim.pet.state).toBe("fall");
    run(sim, 1);
    expect(sim.pet.y).toBe(750);
  });

  it("đang nhảy vì bị click thì nhảy nốt trên mặt đất mới, không bị bắt rơi", () => {
    const sim = setup();
    untilIdle(sim);
    sim.pet.poke();
    run(sim, 0.1);
    sim.world.setScreen({ ...bounds, floor: 720 }, []);
    expect(sim.pet.state).toBe("react");
    run(sim, TUNING.reactTime);
    expect(sim.pet.y).toBe(720);
  });

  it("đang ngủ thì nằm yên trên mặt đất mới", () => {
    const sim = setup();
    sim.pet.sinceInteraction = TUNING.sleepAfter;
    run(sim, 10, undefined, (pet) => pet.state === "sleep");
    sim.world.setScreen({ ...bounds, floor: 750 }, []);
    expect(sim.pet.state).toBe("sleep");
    expect(sim.pet.y).toBe(750);
  });

  it("màn hình hẹp lại thì pet lùi vào trong", () => {
    const sim = setup(1, 950);
    untilIdle(sim);
    sim.world.setScreen({ ...bounds, right: 800 }, []);
    expect(sim.pet.x).toBeLessThanOrEqual(770);
  });

  it("đổi DPI thì pet đứng trên cửa sổ vẫn đứng đúng mép cửa sổ đó", () => {
    const sim = setup();
    sim.world.setWindows([{ id: 1, rect: { x: 300, y: 400, width: 400, height: 300 } }]);
    sim.pet.grab();
    sim.pet.dragTo(500, 350);
    sim.pet.release(0, 0);
    run(sim, 1);
    expect(sim.pet.mount?.id).toBe(1);
    // 100% → 125%: toạ độ CSS nhỏ lại còn 0,8.
    sim.world.setScreen({ left: 0, right: 800, top: 0, floor: 560 }, [], { scale: 0.8, x: 0, y: 0 });
    expect(sim.pet.mount?.id).toBe(1);
    expect(sim.pet.y).toBeCloseTo(320);
    expect(sim.world.terrain.window(1)).toEqual({ x: 240, y: 320, width: 320, height: 240 });
    run(sim, 1);
    expect(sim.pet.mount?.id).toBe(1);
  });
});
