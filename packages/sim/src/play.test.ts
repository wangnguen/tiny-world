import { describe, expect, it } from "vitest";
import type { Pet } from "./pet";
import { TUNING } from "./tuning";
import { World } from "./world";

const DT = 1 / 30;
const bounds = { left: 0, right: 1000, top: 0, floor: 700 };

function setup(xs: number[], seed = 1) {
  const world = new World(bounds, seed);
  const pets = xs.map((x, i) => world.spawn({ id: `p${i}`, x, width: 60, height: 60 }));
  return { world, pets };
}

function run(world: World, seconds: number, until?: () => boolean): boolean {
  for (let t = 0; t < seconds; t += DT) {
    world.step(DT);
    if (until?.()) return true;
  }
  return false;
}

/** Thử từng seed tới khi `scenario` trả về `true`; 30 seed không cái nào được thì test lỗi. */
function someSeed(scenario: (seed: number) => boolean): void {
  for (let seed = 1; seed <= 30; seed++) if (scenario(seed)) return;
  throw new Error("Không seed nào xảy ra.");
}

/** Cho `pet` đi về phía `facing` ngay bây giờ. */
function walk(pet: Pet, facing: 1 | -1) {
  pet.stroll(facing, 30);
  pet.goal = null;
}

describe("gặp nhau", () => {
  it("đi tới sát con khác thì cả hai dừng lại, quay mặt vào nhau", () => {
    const { world, pets } = setup([300, 400]);
    const [a, b] = pets;
    b.pause(1, 30);
    walk(a, 1);
    expect(run(world, 5, () => a.state === "idle")).toBe(true);
    expect(a.facing).toBe(1);
    expect(b.facing).toBe(-1);
    expect(b.state).toBe("idle");
    expect(b.x - a.x).toBeLessThan(60);
  });

  it("vừa chào nhau xong thì một lúc sau mới chào lại", () => {
    const { world, pets } = setup([300, 400]);
    const [a, b] = pets;
    b.pause(1, 30);
    walk(a, 1);
    run(world, 5, () => a.state === "idle");
    expect(a.sinceMeet).toBeLessThan(1);
    // Đi tiếp ngay qua mặt con kia: không dừng lại chào nữa.
    walk(a, 1);
    run(world, 2);
    expect(a.state).toBe("walk");
  });

  it("chào xong có lúc rủ nhau đi cùng một đoạn, cùng hướng", () => {
    someSeed((seed) => {
      const { world, pets } = setup([300, 400], seed);
      const [a, b] = pets;
      b.pause(1, 30);
      walk(a, 1);
      run(world, 5, () => a.state === "idle");
      return run(world, 6, () => a.goal?.kind === "stroll" && b.goal?.kind === "stroll" && a.facing === b.facing);
    });
  });

  it("chào nhau có lúc nói một câu, nhưng cả nhóm có giới hạn tần suất", () => {
    someSeed((seed) => {
      const { world, pets } = setup([300, 400], seed);
      const [a, b] = pets;
      run(world, TUNING.chatGap);
      b.pause(1, 30);
      walk(a, 1);
      run(world, 5, () => a.state === "idle");
      return a.speech !== null;
    });
    const { world, pets } = setup([300, 400]);
    expect(world.chat(pets[0], "a")).toBe(false);
    run(world, TUNING.chatGap / 2 + 1);
    // Chạy lâu thì cả nhóm đã ngủ: con đang ngủ không nói.
    expect(world.chat(pets[0], "a")).toBe(false);
    world.wakeAll();
    expect(world.chat(pets[0], "a")).toBe(true);
    expect(world.chat(pets[1], "b")).toBe(false);
    run(world, TUNING.chatGap + 1);
    world.wakeAll();
    world.busy = true;
    expect(world.chat(pets[1], "b")).toBe(false);
    world.busy = false;
    expect(world.chat(pets[1], "b")).toBe(true);
  });
});

describe("nói", () => {
  it("câu nói hết giờ thì tắt; còn đang nói thì vòng lặp chưa được nghỉ dù cả nhóm ngủ", () => {
    const { world, pets } = setup([300]);
    pets[0].restore({ id: "p0", x: 300, facing: 1, asleep: true, sinceInteraction: 0 });
    pets[0].say("Khuya rồi", 2);
    expect(world.resting).toBe(false);
    run(world, 2.1);
    expect(pets[0].speech).toBeNull();
    expect(world.resting).toBe(true);
  });
});

describe("ngủ cạnh nhau", () => {
  it("buồn ngủ mà có con đang ngủ gần đó thì đi tới nằm cạnh", () => {
    const { world, pets } = setup([300, 600]);
    const [sleeper, sleepy] = pets;
    sleeper.restore({ id: "p0", x: 300, facing: 1, asleep: true, sinceInteraction: 1e6 });
    sleepy.sinceInteraction = 1e6;
    expect(run(world, 30, () => sleepy.state === "sleep")).toBe(true);
    // Nằm sát bên phải con đang ngủ, hơi chồng lên nhau.
    expect(sleepy.x).toBeCloseTo(300 + 60 * TUNING.napGap, 0);
  });

  it("con đang ngủ ở xa thì ngủ luôn tại chỗ", () => {
    const { world, pets } = setup([100, 900]);
    const [sleeper, sleepy] = pets;
    sleeper.restore({ id: "p0", x: 100, facing: 1, asleep: true, sinceInteraction: 1e6 });
    sleepy.sinceInteraction = 1e6;
    const x = sleepy.x;
    expect(run(world, 30, () => sleepy.state === "sleep")).toBe(true);
    expect(Math.abs(sleepy.x - x)).toBeLessThan(TUNING.runSpeed * 3);
    expect(sleepy.x).toBeGreaterThan(100 + TUNING.napRange);
  });
});

describe("ban đêm", () => {
  it("đi chậm hơn và buồn ngủ sớm hơn", () => {
    const { world, pets } = setup([500]);
    const [pet] = pets;
    world.night = true;
    expect(pet.pace).toBeCloseTo(TUNING.nightPace);
    expect(run(world, TUNING.sleepAfterNight + 30, () => pet.state === "sleep")).toBe(true);
    // Ban ngày thì chưa ngủ sau chừng ấy thời gian.
    const day = setup([500]);
    expect(run(day.world, TUNING.sleepAfterNight + 30, () => day.pets[0].state === "sleep")).toBe(false);
  });
});

describe("bubble", () => {
  it("bấm vào con đang nói thì tắt bubble, cả lời nhắc dài", () => {
    const { world, pets } = setup([300]);
    const [pet] = pets;
    pet.say("Ngồi liền 50 phút rồi, đứng dậy nghỉ mắt chút đi :)))", 30);
    run(world, 1);
    expect(pet.speech).not.toBeNull();
    pet.poke();
    expect(pet.speech).toBeNull();
  });
});

describe("đang chat", () => {
  it("đứng yên quay về phía khung chat, không ngủ, không bị rủ đi; đóng chat thì đi lại", () => {
    const { world, pets } = setup([300, 400]);
    const [a, b] = pets;
    walk(a, 1);
    a.listen(-1);
    const x = a.x;
    // Con kia đi ngang qua cũng không dừng lại chào con đang chat.
    walk(b, -1);
    run(world, TUNING.sleepAfter + 30, () => a.state !== "idle");
    expect(a.state).toBe("idle");
    expect(a.facing).toBe(-1);
    expect(a.x).toBe(x);
    a.stopListening();
    expect(run(world, 60, () => a.state !== "idle")).toBe(true);
  });

  it("đang ngủ thì dậy", () => {
    const { world, pets } = setup([300]);
    const [pet] = pets;
    run(world, TUNING.sleepAfter + 30, () => pet.state === "sleep");
    expect(pet.state).toBe("sleep");
    pet.listen(1);
    run(world, 5);
    expect(pet.state).not.toBe("sleep");
  });
});
