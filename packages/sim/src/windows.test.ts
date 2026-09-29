import type { WindowInfo } from "@tinyworld/core";
import { describe, expect, it } from "vitest";
import type { Pet, PetState } from "./pet";
import { TUNING } from "./tuning";
import { World } from "./world";

const DT = 1 / 30;
const bounds = { left: 0, right: 1000, top: 0, floor: 700 };

function win(id: number, x: number, y: number, width: number, height: number): WindowInfo {
  return { id, rect: { x, y, width, height } };
}

function setup(windows: WindowInfo[], seed = 1, x = 500) {
  const world = new World(bounds, seed);
  const pet = world.spawn({ id: "p", x, width: 60, height: 60 });
  world.setWindows(windows);
  return { world, pet };
}

function simulate(world: World, seconds: number, each?: (pet: Pet) => void): PetState[] {
  const seen: PetState[] = [];
  for (let t = 0; t < seconds; t += DT) {
    world.step(DT);
    for (const pet of world.pets) {
      each?.(pet);
      if (seen[seen.length - 1] !== pet.state) seen.push(pet.state);
    }
  }
  return seen;
}

function drop(pet: Pet, x: number, y: number, vx = 0, vy = 0) {
  pet.grab();
  pet.dragTo(x, y);
  pet.release(vx, vy);
}

/**
 * Thử lần lượt từng seed, chạy tới khi `until` đúng rồi trả về pet lúc đó. Hành vi ngẫu nhiên nên
 * không phải seed nào cũng xảy ra trong 40 giây; 20 seed không cái nào xảy ra thì test lỗi.
 */
function firstPet(make: (seed: number) => { world: World; pet: Pet }, until: (pet: Pet) => boolean): Pet {
  for (let seed = 1; seed <= 20; seed++) {
    const { world, pet } = make(seed);
    for (let t = 0; t < 40; t += DT) {
      world.step(DT);
      if (until(pet)) return pet;
    }
  }
  throw new Error("Không seed nào xảy ra.");
}

/** Thả pet lên mép cửa sổ rồi chờ nó đứng yên. */
function standOn(world: World, pet: Pet, x: number, y: number) {
  drop(pet, x, y - 40);
  simulate(world, 1);
  expect(pet.mount).not.toBeNull();
}

describe("đứng trên cửa sổ", () => {
  it("thả phía trên mép cửa sổ thì đáp lên mép, không rơi xuống đất", () => {
    const { world, pet } = setup([win(1, 300, 400, 400, 300)]);
    drop(pet, 500, 200);
    expect(simulate(world, 1)).toContain("land");
    expect(pet.y).toBe(400);
    expect(pet.mount?.id).toBe(1);
  });

  it("thả ngay chỗ bị cửa sổ khác che thì rơi qua, xuống đất", () => {
    // Cửa sổ 2 nằm trên, che mép cửa sổ 1 ở chỗ thả; mép của chính nó ở cao hơn chỗ thả.
    const { world, pet } = setup([win(2, 450, 150, 100, 300), win(1, 300, 400, 400, 300)]);
    drop(pet, 500, 200);
    simulate(world, 2);
    expect(pet.mount).toBeNull();
    expect(pet.y).toBe(700);
  });

  it("đi theo khi cửa sổ bị kéo", () => {
    const { world, pet } = setup([win(1, 300, 400, 400, 300)]);
    standOn(world, pet, 500, 400);
    const x = pet.x;
    world.setWindows([win(1, 350, 300, 400, 300)]);
    expect(pet.x).toBe(x + 50);
    expect(pet.y).toBe(300);
  });

  it("rơi khi cửa sổ thu nhỏ hoặc đóng", () => {
    const { world, pet } = setup([win(1, 300, 400, 400, 300)]);
    standOn(world, pet, 500, 400);
    world.setWindows([]);
    expect(simulate(world, 2)).toContain("fall");
    expect(pet.y).toBe(700);
    expect(pet.mount).toBeNull();
  });

  it("rơi khi cửa sổ bị kéo lên sát trần, không còn chỗ đứng", () => {
    const { world, pet } = setup([win(1, 300, 400, 400, 300)]);
    standOn(world, pet, 500, 400);
    world.setWindows([win(1, 300, 20, 400, 300)]);
    simulate(world, 2);
    expect(pet.mount).toBeNull();
  });

  it("cửa sổ bị kéo ra ngoài màn hình thì pet lùi vào trong", () => {
    const { world, pet } = setup([win(1, 300, 400, 400, 300)]);
    standOn(world, pet, 350, 400);
    world.setWindows([win(1, -200, 400, 400, 300)]);
    expect(pet.x).toBe(30);
    simulate(world, DT);
    expect(pet.mount?.id).toBe(1);
  });

  it("đi lại trong mép cửa sổ, không bao giờ lơ lửng ngoài mép", () => {
    for (let seed = 1; seed <= 5; seed++) {
      const { world, pet } = setup([win(1, 300, 400, 400, 300)], seed);
      standOn(world, pet, 500, 400);
      simulate(world, 60, (p) => {
        if (p.mount?.id === 1 && p.state !== "climb" && p.state !== "jump") {
          expect(p.x).toBeGreaterThanOrEqual(300);
          expect(p.x).toBeLessThanOrEqual(700);
        }
        if (p.grounded && !p.mount && p.state !== "react" && p.state !== "jump") expect(p.y).toBe(700);
      });
    }
  });

  it("cửa sổ khác đè lên thì pet bị che, rồi đi ra chỗ không bị che", () => {
    const { world, pet } = setup([win(1, 200, 400, 600, 300)], 3);
    standOn(world, pet, 500, 400);
    world.setWindows([win(2, 420, 250, 160, 300), win(1, 200, 400, 600, 300)]);
    pet.planned = 0;
    expect(world.occluders(pet)).toEqual([{ x: 420, y: 250, width: 160, height: 300 }]);
    simulate(world, 6);
    expect(world.terrain.covered(1, pet.x, pet.y - 1)).toBe(false);
  });
});

describe("leo, nhảy, ngồi mép", () => {
  it("đứng dưới đất cạnh cửa sổ thì có lúc leo lên mép trên", () => {
    let climbed = false;
    for (let seed = 1; seed <= 20 && !climbed; seed++) {
      const { world, pet } = setup([win(1, 600, 450, 300, 250)], seed, 450);
      let lastY = pet.y;
      simulate(world, 40, (p) => {
        if (p.state === "climb") {
          // Mặt quay vào tường, điểm chân cách tường một khoảng cố định, chỉ đi lên.
          expect(p.facing).toBe(1);
          expect(p.x).toBeCloseTo(600 - p.reach);
          expect(p.y).toBeLessThanOrEqual(lastY);
        }
        lastY = p.y;
        if (p.mount?.id === 1 && p.state !== "climb" && p.y === 450) climbed = true;
      });
    }
    expect(climbed).toBe(true);
  });

  it("cửa sổ lơ lửng: nhảy thẳng lên bám cạnh rồi leo tiếp lên mép trên", () => {
    // Đáy cửa sổ cao hơn mặt đất 150 px, mép trên cao hơn 350 px: không leo thẳng, không nhảy lên mép được.
    const grabs: number[] = [];
    const pet = firstPet(
      (seed) => {
        grabs.length = 0;
        return setup([win(1, 600, 350, 300, 200)], seed, 450);
      },
      (p) => {
        if (p.state === "jump" && p.jumping?.grab === -1 && p.jumping.phase === "air") grabs.push(p.y);
        if (p.state === "climb") {
          // Bám lúc chân thấp hơn đáy cửa sổ nửa thân, rồi chỉ đi lên.
          expect(grabs.length).toBeGreaterThan(0);
          expect(p.y).toBeLessThanOrEqual(550 + 30 + 1);
          expect(p.x).toBeCloseTo(600 - p.reach);
        }
        return p.mount?.id === 1 && p.state !== "climb" && p.y === 350;
      },
    );
    expect(Math.min(...grabs)).toBeGreaterThanOrEqual(579);
    expect(pet.mount?.id).toBe(1);
  });

  it("cửa sổ lơ lửng cao quá tầm nhảy thì không lên được", () => {
    for (let seed = 1; seed <= 10; seed++) {
      const { world } = setup([win(1, 600, 100, 300, 200)], seed, 450);
      simulate(world, 40, (p) => expect(p.mount).toBeNull());
    }
  });

  it("nhảy sang cửa sổ bên cạnh mà không chạm đất", () => {
    let jumped = false;
    for (let seed = 1; seed <= 20 && !jumped; seed++) {
      const { world, pet } = setup([win(1, 100, 400, 300, 300), win(2, 500, 350, 300, 350)], seed);
      standOn(world, pet, 250, 400);
      // Rời cửa sổ 1 rồi tới cửa sổ 2 mà giữa chừng không đứng dưới đất.
      let fromOne = false;
      simulate(world, 40, (p) => {
        if (p.mount?.id === 1) fromOne = true;
        if (!p.mount && p.y === 700) fromOne = false;
        if (p.mount?.id === 2 && p.y === 350 && fromOne) jumped = true;
      });
    }
    expect(jumped).toBe(true);
  });

  it("nhảy sang cửa sổ nào thì đáp đúng cửa sổ đó, kể cả chỗ đáp sát đầu mép", () => {
    // Đúng bố cục thử trên máy thật: B nằm bên trái, cao hơn A 80 px, cách 119 px.
    let jumps = 0;
    for (let seed = 1; seed <= 20; seed++) {
      const world = new World({ left: 0, right: 1920, top: 0, floor: 1040 }, seed);
      world.speed = 2;
      const pet = world.spawn({ id: "p", x: 1300, width: 96, height: 96, reach: 30.5 });
      world.setWindows([win(2, 707, 560, 366, 293), win(1, 1192, 640, 506, 401)]);
      standOn(world, pet, 1300 + seed * 10, 640);
      let target: number | null = null;
      simulate(world, 60, (p) => {
        if (p.state === "jump" && p.jumping?.phase === "air") target = p.jumping.id;
        else if (target !== null) {
          expect(p.mount?.id).toBe(target);
          jumps++;
          target = null;
        }
      });
    }
    expect(jumps).toBeGreaterThan(5);
  });

  it("có lúc ngồi ở mép cửa sổ", () => {
    const seen = new Set<PetState>();
    for (let seed = 1; seed <= 10; seed++) {
      const { world, pet } = setup([win(1, 300, 200, 250, 500)], seed);
      standOn(world, pet, 400, 200);
      for (const state of simulate(world, 60)) seen.add(state);
    }
    expect(seen).toContain("perch");
  });

  it("click lúc đang leo thì tuột tay rơi xuống", () => {
    const pet = firstPet((seed) => setup([win(1, 600, 300, 300, 400)], seed, 450), (p) => p.state === "climb");
    pet.poke();
    expect(pet.state).toBe("fall");
    expect(pet.mount).toBeNull();
  });

  it("buồn ngủ trên cửa sổ thì xuống taskbar rồi mới ngủ", () => {
    for (const y of [300, 100]) {
      const { world, pet } = setup([win(1, 300, y, 400, 700 - y)]);
      standOn(world, pet, 500, y);
      pet.sinceInteraction = TUNING.sleepAfter;
      simulate(world, 60);
      expect(pet.state).toBe("sleep");
      expect(pet.y).toBe(700);
      expect(pet.mount).toBeNull();
    }
  });

  it("frame của cú nhảy: lấy đà, bay, tiếp đất", () => {
    const poses = new Set<number>();
    firstPet(
      (seed) => {
        const { world, pet } = setup([win(1, 100, 400, 300, 300), win(2, 500, 350, 300, 350)], seed);
        standOn(world, pet, 250, 400);
        poses.clear();
        return { world, pet };
      },
      (p) => {
        if (p.pose !== undefined) poses.add(p.pose);
        return p.state === "idle" && poses.has(3);
      },
    );
    expect([...poses].sort()).toEqual([0, 1, 2, 3]);
  });
});

describe("phản ứng với cửa sổ", () => {
  /** Đếm số cú nhảy `react` bắt đầu trong `seconds` giây (state chạy lại từ đầu cũng tính). */
  function countHops(world: World, pet: Pet, seconds: number): number {
    let hops = pet.state === "react" ? 1 : 0;
    let prev = { state: pet.state, time: pet.stateTime };
    for (let t = 0; t < seconds; t += DT) {
      world.step(DT);
      if (pet.state === "react" && (prev.state !== "react" || pet.stateTime < prev.time)) hops++;
      prev = { state: pet.state, time: pet.stateTime };
    }
    return hops;
  }

  it("cửa sổ bị kéo lại gần thì chạy trốn về phía ngược lại", () => {
    const { world, pet } = setup([win(1, 700, 560, 200, 100)]);
    world.setWindows([win(1, 600, 560, 200, 100)]);
    expect(pet.state).toBe("run");
    expect(pet.facing).toBe(-1);
    simulate(world, 0.8);
    expect(pet.x).toBeLessThan(450);
  });

  it("đang đi hay đang chạy mà bị cửa sổ kéo tới thì vẫn là chạy trốn, không phải chạy chơi", () => {
    for (const moving of ["walk", "run"] as const) {
      const pet = firstPet(
        (seed) => setup([win(1, 700, 560, 200, 100)], seed),
        (p) => p.state === moving && p.facing === 1 && p.x < 560,
      );
      const world = pet.env as World;
      world.setWindows([win(1, pet.x + 100, 560, 200, 100)]);
      expect(pet.state).toBe("run");
      expect(pet.goal?.kind).toBe("flee");
      expect(pet.facing).toBe(-1);
    }
  });

  it("không chạy khi cửa sổ đi xa ra, bị phóng to, pet đứng trên nó hoặc đang ngủ", () => {
    const away = setup([win(1, 600, 560, 200, 100)]);
    away.world.setWindows([win(1, 700, 560, 200, 100)]);
    expect(away.pet.state).toBe("idle");

    const maximized = setup([win(1, 700, 560, 200, 100)]);
    maximized.world.setWindows([win(1, 0, 0, 1000, 700)]);
    expect(maximized.pet.state).toBe("idle");

    const riding = setup([win(1, 300, 400, 400, 300)]);
    standOn(riding.world, riding.pet, 500, 400);
    const state = riding.pet.state;
    riding.world.setWindows([win(1, 350, 400, 400, 300)]);
    expect(riding.pet.state).toBe(state);

    const asleep = setup([win(1, 700, 560, 200, 100)]);
    asleep.pet.restore({ id: "p", x: 500, facing: 1, asleep: true, sinceInteraction: 0 });
    asleep.world.setWindows([win(1, 600, 560, 200, 100)]);
    expect(asleep.pet.state).toBe("sleep");
  });

  it("bị dồn vào mép màn hình thì giật mình nhảy lên chứ không chạy về phía cửa sổ", () => {
    const { world, pet } = setup([win(1, 200, 560, 200, 100)], 1, 40);
    world.setWindows([win(1, 100, 560, 200, 100)]);
    expect(pet.state).toBe("react");
  });

  it("đang chạy trốn trên mép cửa sổ mà hết đường thì xuống luôn", () => {
    // Pet đứng cách đầu trái mép 88 px, cửa sổ 2 bị kéo tới từ bên phải.
    const { world, pet } = setup([win(1, 300, 400, 400, 300), win(2, 560, 250, 150, 100)]);
    standOn(world, pet, 400, 400);
    world.setWindows([win(1, 300, 400, 400, 300), win(2, 460, 250, 150, 100)]);
    expect(pet.goal?.kind).toBe("flee");
    simulate(world, 4);
    expect(pet.mount?.id).not.toBe(1);
  });

  it("cửa sổ gần đó bị đóng thì nhảy cẫng lên hai cái, quay về phía cửa sổ", () => {
    let cheered = 0;
    for (let seed = 1; seed <= 10; seed++) {
      const { world, pet } = setup([win(1, 700, 400, 200, 200)], seed);
      pet.facing = -1;
      world.setWindows([], [win(1, 700, 400, 200, 200)]);
      if (pet.state !== "react") continue;
      cheered++;
      expect(pet.facing).toBe(1);
      expect(countHops(world, pet, 1.5)).toBe(2);
    }
    // Xác suất 0,8 mỗi lần.
    expect(cheered).toBeGreaterThan(5);
  });

  it("không ăn mừng khi cửa sổ ở xa, chỉ thu nhỏ (không có trong `closed`), hoặc vừa ăn mừng xong", () => {
    const far = setup([win(1, 0, 100, 50, 50)], 1, 950);
    far.world.setWindows([], [win(1, 0, 100, 50, 50)]);
    expect(far.pet.state).toBe("idle");

    const minimized = setup([win(1, 700, 400, 200, 200)]);
    minimized.world.setWindows([]);
    expect(minimized.pet.state).toBe("idle");

    for (let seed = 1; seed <= 10; seed++) {
      const { world, pet } = setup([win(1, 700, 400, 200, 200), win(2, 200, 400, 200, 200)], seed);
      world.setWindows([win(2, 200, 400, 200, 200)], [win(1, 700, 400, 200, 200)]);
      if (pet.state !== "react") continue;
      simulate(world, 3);
      const state = pet.state;
      world.setWindows([], [win(2, 200, 400, 200, 200)]);
      expect(pet.state).toBe(state);
      return;
    }
  });
});
