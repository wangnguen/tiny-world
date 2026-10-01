import { describe, expect, it } from "vitest";
import { simulatedSky, skyOf, withPetals, type Sky } from "./weather";

describe("mã thời tiết WMO", () => {
  it("đổi đúng thành hiệu ứng", () => {
    const cases: [number, Sky][] = [
      [0, "clear"],
      [1, "clear"],
      [2, "cloudy"],
      [3, "cloudy"],
      [45, "fog"],
      [48, "fog"],
      [51, "rain"],
      [61, "rain"],
      [67, "rain"],
      [80, "rain"],
      [82, "rain"],
      [71, "snow"],
      [77, "snow"],
      [85, "snow"],
      [86, "snow"],
      [95, "storm"],
      [99, "storm"],
      [42, "cloudy"],
    ];
    for (const [code, sky] of cases) expect(skyOf(code), String(code)).toBe(sky);
  });

  it("cánh hoa chỉ rơi mùa xuân, trời quang, ban ngày", () => {
    expect(withPetals("clear", 2, true)).toBe("petals");
    expect(withPetals("clear", 2, false)).toBe("clear");
    expect(withPetals("rain", 2, true)).toBe("rain");
    expect(withPetals("clear", 7, true)).toBe("clear");
  });
});

describe("thời tiết giả lập", () => {
  it("cùng khung 3 tiếng thì như nhau, mở lại app không đổi", () => {
    const a = simulatedSky(new Date(2026, 9, 1, 13, 5));
    expect(simulatedSky(new Date(2026, 9, 1, 14, 59))).toBe(a);
  });

  it("đổi theo ngày, theo mùa: mùa hè không có tuyết, có lúc mưa hay sấm", () => {
    const summer = new Set<Sky>();
    for (let day = 1; day <= 30; day++) for (let h = 0; h < 24; h += 3) summer.add(simulatedSky(new Date(2026, 5, day, h)));
    expect(summer.has("snow")).toBe(false);
    expect(summer.has("rain") || summer.has("storm")).toBe(true);
    expect(summer.has("clear")).toBe(true);
  });
});
