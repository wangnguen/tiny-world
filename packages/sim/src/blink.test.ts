import type { WindowInfo } from "@tinyworld/core";
import { describe, expect, it } from "vitest";
import type { Pet } from "./pet";
import { TUNING } from "./tuning";
import { World } from "./world";

// Những gì combo quật đuôi của Long cần ở mô phỏng: bắt đứng yên, tốc biến tới cạnh con khác, biến mất.

const DT = 1 / 30;
const bounds = { left: 0, right: 1000, top: 0, floor: 700 };

function win(id: number, x: number, y: number, width: number, height: number): WindowInfo {
  return { id, rect: { x, y, width, height } };
}

/** Một nhóm pet 60 px đứng ở các chỗ `xs`. */
function setup(xs: number[], windows: WindowInfo[] = [], seed = 1, area = bounds) {
  const world = new World(area, seed);
  const pets = xs.map((x, i) => world.spawn({ id: `p${i}`, x, width: 60, height: 60 }));
  world.setWindows(windows);
  return { world, pets };
}

function run(world: World, seconds: number, each?: () => void) {
  for (let t = 0; t < seconds; t += DT) {
    world.step(DT);
    each?.();
  }
}

function drop(pet: Pet, x: number, y: number) {
  pet.grab();
  pet.dragTo(x, y);
  pet.release(0, 0);
}

/** Thả pet lên mép cửa sổ rồi chờ nó đứng yên. */
function standOn(world: World, pet: Pet, x: number, y: number) {
  drop(pet, x, y - 40);
  run(world, 1);
  expect(pet.mount).not.toBeNull();
}

/** Đang đứng (không nhảy nhót) thì chân phải chạm mép cửa sổ đang đứng hoặc mặt đất, không lơ lửng. */
function expectStanding(world: World, pet: Pet) {
  if (!pet.grounded || pet.state === "react" || pet.state === "jump") return;
  const ledge = pet.mount && world.terrain.ledge(pet.mount.id);
  expect(pet.y).toBe(ledge ? ledge.y : bounds.floor);
}

describe("bắt đứng yên (hold)", () => {
  it("đang rảnh thì đứng yên đúng chừng ấy giây, quay về phía được bảo", () => {
    const { world, pets: [pet] } = setup([500]);
    expect(pet.hold(-1, 2)).toBe(true);
    const x = pet.x;
    run(world, 1.9, () => {
      expect(pet.state).toBe("idle");
      expect(pet.x).toBe(x);
      expect(pet.facing).toBe(-1);
    });
  });

  it("đang ngủ thì dậy đứng yên", () => {
    const { world, pets: [pet] } = setup([500]);
    pet.sinceInteraction = 1e6;
    run(world, 30);
    expect(pet.state).toBe("sleep");
    expect(pet.hold(1, 2)).toBe(true);
    expect(pet.state).toBe("idle");
  });

  it("đang bị kéo hay đang rơi thì không ép đứng giữa không trung", () => {
    const { world, pets: [pet] } = setup([500]);
    pet.grab();
    expect(pet.hold(1, 2)).toBe(false);
    expect(pet.state).toBe("dragged");
    pet.dragTo(500, 300);
    pet.release(0, 0);
    expect(pet.hold(1, 2)).toBe(false);
    expect(pet.state).toBe("fall");
    run(world, 2);
    expect(pet.y).toBe(700);
  });

  it("đang biến mất thì không bắt được", () => {
    const { pets: [pet] } = setup([500]);
    pet.vanish(5);
    expect(pet.hold(1, 2)).toBe(false);
  });
});

describe("tốc biến (blinkTo)", () => {
  it("từ mặt đất tới cạnh con đứng trên mép cửa sổ: đứng hẳn lên mép, cửa sổ đi đâu theo đó", () => {
    const { world, pets: [long, target] } = setup([100, 500], [win(1, 300, 400, 400, 300)]);
    standOn(world, target, 500, 400);
    target.hold(-1, 5);
    long.blinkTo(target, target.x - 50, 2);
    expect(long.mount?.id).toBe(1);
    expect(long.y).toBe(400);
    expect(long.x).toBe(target.x - 50);
    expect(long.facing).toBe(1);
    run(world, 1.5, () => expect(long.y).toBe(400));
    world.setWindows([win(1, 350, 300, 400, 300)]);
    expect(long.y).toBe(300);
    run(world, 30, () => expectStanding(world, long));
  });

  it("từ mép cửa sổ tới cạnh con dưới đất: xuống đất, không còn bám cửa sổ cũ", () => {
    const { world, pets: [long, target] } = setup([500, 100], [win(1, 300, 400, 400, 300)]);
    standOn(world, long, 500, 400);
    target.hold(1, 5);
    long.blinkTo(target, target.x + 50, 2);
    expect(long.mount).toBeNull();
    expect(long.y).toBe(700);
    expect(long.facing).toBe(-1);
    world.setWindows([win(1, 350, 300, 400, 300)]);
    expect(long.y).toBe(700);
    run(world, 30, () => expectStanding(world, long));
  });

  it("con kia đứng sát mép màn hình thì vẫn đứng trong màn hình", () => {
    const { pets: [long, target] } = setup([500, 30]);
    long.blinkTo(target, target.x - 50, 2);
    expect(long.x).toBe(30);
    expect(long.y).toBe(700);
  });

  it("đang rơi giữa chừng cũng đứng hẳn xuống chỗ mới rồi đứng yên", () => {
    const { world, pets: [long, target] } = setup([800, 300]);
    drop(long, 800, 200);
    long.blinkTo(target, target.x + 50, 2);
    expect(long.state).toBe("idle");
    run(world, 1.5, () => {
      expect(long.state).toBe("idle");
      expect(long.y).toBe(700);
      expect(long.x).toBe(350);
    });
  });
});

describe("biến mất (vanish)", () => {
  it("đứng nguyên như lúc biến mất, hết giờ thì làm tiếp việc đang dở", () => {
    const { world, pets: [pet] } = setup([300]);
    pet.stroll(1, 10);
    run(world, 0.5);
    const { x, state } = pet;
    pet.vanish(5);
    run(world, 4.9, () => {
      expect(pet.x).toBe(x);
      expect(pet.state).toBe(state);
    });
    run(world, 0.5);
    expect(pet.vanished).toBe(0);
    expect(pet.x).toBeGreaterThan(x);
  });

  it("không giật mình, không nói câu cho vui; câu đang nói để dành tới lúc hiện lại", () => {
    const { world, pets: [pet, other] } = setup([300, 700]);
    // Chờ hết `chatGap` mà cả nhóm vẫn thức, không câu chào nào nói mất lượt.
    world.chatter = false;
    run(world, TUNING.chatGap, () => {
      pet.sinceInteraction = 0;
    });
    world.chatter = true;
    pet.say("câu đang dở", 3);
    pet.vanish(5);
    pet.startle();
    expect(pet.state).not.toBe("react");
    expect(world.chat(pet, "chào")).toBe(false);
    expect(world.chat(other, "chào")).toBe(true);
    run(world, 4);
    expect(pet.speech?.text).toBe("câu đang dở");
  });

  it("con khác đi ngang qua không dừng lại chào", () => {
    const narrow = { ...bounds, right: 300 };
    const met = (vanish: boolean) => {
      const { world, pets: [walker, ghost] } = setup([60, 240], [], 3, narrow);
      if (vanish) ghost.vanish(90);
      run(world, 90);
      return Number.isFinite(walker.sinceMeet);
    };
    expect(met(false)).toBe(true);
    expect(met(true)).toBe(false);
  });
});
