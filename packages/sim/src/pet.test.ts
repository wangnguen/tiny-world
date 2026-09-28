import { describe, expect, it } from "vitest";
import { TUNING, type Pet, type PetState } from "./pet";
import { parseWorldSnapshot } from "./snapshot";
import { World } from "./world";

const DT = 1 / 30;
const bounds = { left: 0, right: 1000, top: 0, floor: 700 };

function setup(seed = 1, x = 500, area = bounds) {
  const world = new World(area, seed);
  const pet = world.spawn({ id: "p", x, width: 60, height: 60 });
  return { world, pet };
}

/** Chạy mô phỏng `seconds` giây, trả về danh sách state theo thứ tự (bỏ lặp liên tiếp). */
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

/** Nhấc pet lên tới điểm chân (x, y) rồi thả với vận tốc (vx, vy). */
function drop(pet: Pet, x: number, y: number, vx = 0, vy = 0) {
  pet.grab();
  pet.dragTo(x, y);
  pet.release(vx, vy);
}

describe("xuất hiện", () => {
  it("đứng trên mặt đất, nằm trọn trong màn hình", () => {
    const { world, pet } = setup(1, 990);
    expect(pet.y).toBe(700);
    expect(pet.x).toBe(970);
    expect(world.spawn({ id: "q", x: -50, width: 60, height: 60 }).x).toBe(30);
  });
});

describe("tự đi lại", () => {
  it("đứng, đi, chạy xen kẽ, luôn trong màn hình và quay đầu ở mép", () => {
    // Màn hình hẹp 300 px để pet chắc chắn đi tới mép.
    const { world } = setup(7, 150, { ...bounds, right: 300 });
    let turnedAtEdge = false;
    let lastFacing = world.pets[0].facing;
    const states = simulate(world, 150, (pet) => {
      expect(pet.x).toBeGreaterThanOrEqual(30);
      expect(pet.x).toBeLessThanOrEqual(270);
      expect(pet.y).toBe(700);
      const atEdge = pet.x === 30 || pet.x === 270;
      if (atEdge && pet.facing !== lastFacing) turnedAtEdge = true;
      lastFacing = pet.facing;
    });
    expect(states).toContain("walk");
    expect(states).toContain("run");
    expect(states).not.toContain("sleep");
    expect(turnedAtEdge).toBe(true);
  });

  it("cùng seed thì đi y hệt nhau", () => {
    const a = setup(3);
    const b = setup(3);
    simulate(a.world, 30);
    simulate(b.world, 30);
    expect(a.pet.x).toBe(b.pet.x);
    expect(a.pet.state).toBe(b.pet.state);
  });
});

describe("click", () => {
  it("nhảy lên tại chỗ rồi đi hoặc chạy tiếp, không đứng khựng lại", () => {
    const after = new Set<PetState>();
    for (let seed = 1; seed <= 10; seed++) {
      const { world, pet } = setup(seed);
      pet.poke();
      expect(pet.state).toBe("react");
      world.step(TUNING.reactTime / 2);
      expect(pet.y).toBeCloseTo(700 - TUNING.hopHeight);
      world.step(TUNING.reactTime / 2);
      expect(pet.y).toBe(700);
      after.add(pet.state);
    }
    expect([...after].sort()).toEqual(["run", "walk"]);
  });

  it("không phản ứng khi đang bị kéo hoặc đang rơi", () => {
    const { pet } = setup();
    pet.grab();
    pet.poke();
    expect(pet.state).toBe("dragged");
    pet.release(0, 0);
    pet.poke();
    expect(pet.state).toBe("fall");
  });
});

describe("kéo thả", () => {
  it("kéo đi theo chuột nhưng không ra khỏi màn hình", () => {
    const { pet } = setup();
    pet.grab();
    pet.dragTo(200, 300);
    expect([pet.x, pet.y]).toEqual([200, 300]);
    pet.dragTo(-100, -100);
    expect([pet.x, pet.y]).toEqual([30, 60]);
    pet.dragTo(5000, 5000);
    expect([pet.x, pet.y]).toEqual([970, 700]);
  });

  it("thả thấp: rơi, nảy một chút rồi tiếp đất", () => {
    const { world, pet } = setup();
    drop(pet, 500, 600);
    let bounced = false;
    const states = simulate(world, 1.5, (p) => {
      if (p.state === "fall" && p.vy < 0) bounced = true;
    });
    expect(states.slice(0, 3)).toEqual(["fall", "land", "idle"]);
    expect(bounced).toBe(true);
    expect(pet.y).toBe(700);
  });

  it("thả từ cao: choáng rồi tỉnh lại", () => {
    const { world, pet } = setup();
    drop(pet, 500, 100);
    const states = simulate(world, 1 + TUNING.dizzyTime);
    expect(states.slice(0, 3)).toEqual(["fall", "dizzy", "idle"]);
  });

  it("ném mạnh sang trái: bay theo quán tính, đập tường bật lại", () => {
    const { world, pet } = setup();
    drop(pet, 500, 400, -2500, -500);
    let hitWall = false;
    simulate(world, 3, (p) => {
      expect(p.x).toBeGreaterThanOrEqual(30);
      expect(p.y).toBeGreaterThanOrEqual(60);
      if (p.x === 30 && p.vx > 0) hitWall = true;
    });
    expect(hitWall).toBe(true);
    expect(pet.grounded).toBe(true);
  });

  it("giới hạn vận tốc ném", () => {
    const { pet } = setup();
    drop(pet, 500, 400, 30000, 40000);
    expect(Math.hypot(pet.vx, pet.vy)).toBeCloseTo(TUNING.maxThrowSpeed);
  });

  it("bắt được pet giữa lúc đang rơi", () => {
    const { world, pet } = setup();
    drop(pet, 500, 100);
    simulate(world, 0.2);
    pet.grab();
    expect(pet.state).toBe("dragged");
    expect([pet.vx, pet.vy]).toEqual([0, 0]);
  });
});

describe("ngủ", () => {
  it("lâu không ai đụng thì ngủ, ngủ yên cho tới khi được click", () => {
    const { world, pet } = setup(5);
    simulate(world, TUNING.sleepAfter + 10);
    expect(pet.state).toBe("sleep");
    expect(world.resting).toBe(true);
    simulate(world, 600);
    expect(pet.state).toBe("sleep");

    pet.poke();
    expect(pet.state).toBe("react");
    expect(pet.sinceInteraction).toBe(0);
    expect(world.resting).toBe(false);
    const states = simulate(world, 60);
    expect(states).not.toContain("sleep");
  });

  it("kéo cũng đánh thức", () => {
    const { world, pet } = setup(5);
    simulate(world, TUNING.sleepAfter + 10);
    pet.grab();
    expect(pet.state).toBe("dragged");
    expect(pet.sinceInteraction).toBe(0);
  });
});

describe("settings", () => {
  /** Quãng đường đi được trong một lượt walk đầu tiên, với hệ số tốc độ `speed`. */
  function walkDistance(speed: number): number {
    const { world, pet } = setup(2);
    world.speed = speed;
    while (pet.state !== "walk") world.step(DT);
    const start = pet.x;
    world.step(DT);
    return Math.abs(pet.x - start);
  }

  it("tốc độ nhân vào vận tốc đi", () => {
    expect(walkDistance(1)).toBeCloseTo(TUNING.walkSpeed * DT);
    expect(walkDistance(2)).toBeCloseTo(TUNING.walkSpeed * DT * 2);
  });

  it("đổi cỡ thì vẫn nằm trọn trong màn hình", () => {
    const { pet } = setup(1, 970);
    pet.resize(120, 120);
    expect(pet.x).toBe(940);
    expect(pet.width).toBe(120);
  });
});

describe("lưu trạng thái", () => {
  it("đang ngủ thì mở lại vẫn ngủ ở chỗ cũ", () => {
    const { world, pet } = setup(5, 300);
    simulate(world, TUNING.sleepAfter + 10);
    const saved = parseWorldSnapshot(JSON.parse(JSON.stringify(world.snapshot())));

    const next = setup(9, 800);
    next.pet.restore(saved!.pets[0]);
    expect(next.pet.state).toBe("sleep");
    expect(next.pet.x).toBe(Math.round(pet.x));
    expect(next.pet.facing).toBe(pet.facing);
    expect(next.world.resting).toBe(true);
  });

  it("đang bị kéo trên cao thì mở lại đứng trên mặt đất", () => {
    const { world, pet } = setup();
    drop(pet, 200, 100);
    pet.grab();
    const next = setup();
    next.pet.restore(world.snapshot().pets[0]);
    expect(next.pet.state).toBe("idle");
    expect(next.pet.y).toBe(700);
    expect(next.pet.x).toBe(200);
  });

  it("vị trí cũ ngoài màn hình (đổi độ phân giải) thì kẹp lại", () => {
    const { pet } = setup();
    pet.restore({ id: "p", x: 5000, facing: -1, asleep: false, sinceInteraction: 12 });
    expect(pet.x).toBe(970);
    expect(pet.sinceInteraction).toBe(12);
  });

  it("file sai version, sai kiểu thì bỏ qua", () => {
    expect(parseWorldSnapshot(null)).toBeNull();
    expect(parseWorldSnapshot({ version: 2, pets: [] })).toBeNull();
    expect(parseWorldSnapshot({ version: 1, pets: {} })).toBeNull();
    const good = { id: "p", x: 1, facing: 1, asleep: false, sinceInteraction: 0 };
    const parsed = parseWorldSnapshot({
      version: 1,
      pets: [good, { ...good, x: "1" }, { ...good, facing: 0 }, { ...good, sinceInteraction: -1 }],
    });
    expect(parsed?.pets).toEqual([good]);
  });
});
