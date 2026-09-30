import { describe, expect, it } from "vitest";
import type { Pet } from "./pet";
import { TUNING } from "./tuning";
import { World } from "./world";

const DT = 1 / 30;
const bounds = { left: 0, right: 1000, top: 0, floor: 700 };

interface Sim {
  world: World;
  pet: Pet;
}

function setup(seed = 1, x = 500, width = 60): Sim {
  const world = new World(bounds, seed);
  const pet = world.spawn({ id: "p", x, width, height: width });
  return { world, pet };
}

/**
 * Chạy `seconds` giây; mỗi bước hỏi `cursor` vị trí con trỏ (theo số giây đã chạy) rồi báo cho world.
 * `until` đúng thì dừng sớm, trả về `true`.
 */
function run(
  sim: Sim,
  seconds: number,
  cursor?: (t: number) => { x: number; y: number } | null,
  until?: (pet: Pet) => boolean,
): boolean {
  for (let t = 0; t < seconds; t += DT) {
    const at = cursor?.(t);
    if (at) sim.world.moveCursor(at.x, at.y);
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

describe("sang màn hình bên cạnh", () => {
  /** Màn hình bên phải cao bằng, cùng DPI. */
  const right = { x: 1000, y: 0, width: 800, height: 700 };

  function leave(seed: number, width = 60): Sim | null {
    const sim = setup(seed, 900, width);
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

  it("pet to đi bộ chậm ra mép vẫn kịp ra tới chỗ chờ overlay sang", () => {
    // Pet 96 px đi 30 px/s: từ đầu mặt đất tới lúc giữa thân qua mép mất 1,6 giây, lâu hơn crossTimeout.
    someSeed((seed) => leave(seed, 96) !== null);
  });

  it("màn hình đổi tại chỗ (taskbar, DPI) lúc đang đi ra mép thì vào lại trong màn hình, không đi lạc", () => {
    let sim: Sim | null = null;
    someSeed((seed) => (sim = leave(seed)) !== null);
    const { world, pet } = sim!;
    world.setScreen(bounds, [right], { scale: 1, x: 0, y: 0 });
    run(sim!, 5);
    expect(pet.crossing).toBeNull();
    expect(pet.x).toBeLessThanOrEqual(970);
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

  it("ném qua mép mà rơi chạm đất vẫn chờ overlay sang, không bị kéo về", () => {
    const sim = setup(1, 800);
    sim.world.setScreen(bounds, [right]);
    sim.pet.grab();
    sim.pet.dragTo(900, 300);
    sim.pet.release(1500, -200);
    run(sim, 0.5, undefined, (pet) => pet.leaving !== null);
    expect(sim.pet.crossing?.phase).toBe("out");
    // Chạy thêm 0,8 giây (đủ rơi chạm đất nhưng chưa hết crossTimeout 1,5 giây).
    run(sim, 0.8);
    expect(sim.pet.y).toBe(700);
    expect(sim.pet.crossing?.phase).toBe("out");
    expect(sim.pet.leaving).not.toBeNull();
  });

  /** Overlay sang màn hình bên phải: gốc toạ độ dời sang phải 1000 px. */
  function overlayMovesRight(sim: Sim) {
    sim.world.setScreen({ left: 0, right: 800, top: 0, floor: 700 }, [{ x: -1000, y: 0, width: 1000, height: 700 }], {
      scale: 1,
      x: -1000,
      y: 0,
    });
  }

  it("ném qua mép, rơi chạm đất rồi overlay sang thì đáp bình thường", () => {
    const sim = setup(1, 800);
    sim.world.setScreen(bounds, [right]);
    sim.pet.grab();
    sim.pet.dragTo(900, 300);
    sim.pet.release(1500, -200);
    run(sim, 0.5, undefined, (pet) => pet.leaving !== null);
    run(sim, 0.8);
    expect(sim.pet.crossing?.phase).toBe("out");
    overlayMovesRight(sim);
    expect(sim.pet.crossing?.phase).toBe("in");
    run(sim, 2, undefined, (p) => p.crossing === null);
    expect(sim.pet.crossing).toBeNull();
    expect(sim.pet.y).toBe(700);
  });

  it("ném thấp qua mép, chạm đất lúc mới qua mép một phần rồi overlay sang thì đi bộ vào, không giật vào", () => {
    const sim = setup(1, 800);
    sim.world.setScreen(bounds, [right]);
    sim.pet.grab();
    sim.pet.dragTo(970, 680);
    sim.pet.release(300, 0);
    run(sim, 0.5, undefined, (pet) => pet.leaving !== null);
    run(sim, 0.3);
    // Đứng chờ trên đất, giữa thân đã qua mép nhưng một phần thân còn bên này.
    expect(sim.pet.crossing?.phase).toBe("out");
    expect(sim.pet.y).toBe(700);
    expect(sim.pet.x).toBeGreaterThan(1000);
    expect(sim.pet.x).toBeLessThan(1030);
    overlayMovesRight(sim);
    expect(sim.pet.crossing?.phase).toBe("in");
    // Mỗi bước chỉ nhích một chút (đi bộ vào), không nhảy vọt cả bề ngang pet vào trong màn hình.
    let x = sim.pet.x;
    const jumps: number[] = [];
    run(sim, 4, undefined, (p) => {
      jumps.push(Math.abs(p.x - x));
      x = p.x;
      return p.crossing === null;
    });
    expect(sim.pet.crossing).toBeNull();
    expect(sim.pet.state).toBe("walk");
    expect(sim.pet.x).toBeGreaterThanOrEqual(30);
    expect(sim.pet.y).toBe(700);
    expect(Math.max(...jumps)).toBeLessThan(5);
  });

  it("ném mạnh qua mép, overlay sang lúc pet còn đang bay thì bay tiếp vào, không rơi thẳng ngoài mép", () => {
    const sim = setup(1, 800);
    sim.world.setScreen(bounds, [right]);
    sim.pet.grab();
    sim.pet.dragTo(900, 300);
    sim.pet.release(3000, -200);
    // Bay ra hẳn ngoài mép, bị giữ lại chờ overlay sang.
    expect(run(sim, 0.5, undefined, (pet) => pet.x === 1030)).toBe(true);
    overlayMovesRight(sim);
    run(sim, 0.1);
    expect(sim.pet.state).toBe("fall");
    expect(sim.pet.y).toBeLessThan(700);
    expect(sim.pet.x).toBeGreaterThan(30);
    expect(sim.pet.crossing).toBeNull();
  });

  it("thả sát mép hơi hất ra, chạm đất lúc giữa thân chưa qua mép thì đáp luôn, không đứng khựng chờ", () => {
    const sim = setup(1, 800);
    sim.world.setScreen(bounds, [right]);
    sim.pet.grab();
    sim.pet.dragTo(965, 690);
    sim.pet.release(100, 0);
    const landed = run(sim, 0.5, undefined, (pet) => pet.state === "land");
    expect(landed).toBe(true);
    expect(sim.pet.crossing).toBeNull();
    expect(sim.pet.x).toBeLessThanOrEqual(970);
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
