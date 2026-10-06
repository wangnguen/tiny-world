import { describe, expect, it } from "vitest";
import { skyLine, warmthLine } from "./lines";
import { daySky, simulatedSky, skyOf, warmthOf, type Sky } from "./weather";

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

  it("trời quang: ban ngày nắng, mùa xuân cánh hoa rơi, ban đêm vẫn quang", () => {
    expect(daySky("clear", 2, true)).toBe("petals");
    expect(daySky("clear", 2, false)).toBe("clear");
    expect(daySky("rain", 2, true)).toBe("rain");
    expect(daySky("clear", 7, true)).toBe("sunny");
    expect(daySky("clear", 7, false)).toBe("clear");
    expect(daySky("cloudy", 7, true)).toBe("cloudy");
  });

  it("nóng từ 33°C, lạnh từ 15°C trở xuống, chưa biết nhiệt độ thì bình thường", () => {
    expect(warmthOf(33)).toBe("hot");
    expect(warmthOf(32.9)).toBe(null);
    expect(warmthOf(15)).toBe("cold");
    expect(warmthOf(15.1)).toBe(null);
    expect(warmthOf(-5)).toBe("cold");
    expect(warmthOf(null)).toBe(null);
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

describe("câu khi trời đổi", () => {
  it("kèm nhiệt độ nếu biết, nắng mà nóng thì kêu nóng", () => {
    expect(skyLine("rain", 24.4, null)).toBe("Mưa rồi, 24°C :(");
    expect(skyLine("rain", null, null)).toBe("Mưa rồi :(");
    expect(skyLine("sunny", 36.2, "hot")).toBe("Nắng to quá, 36°C :(((");
    expect(skyLine("sunny", 28, null)).toBe("Nắng đẹp ghê, 28°C :)))");
    expect(warmthLine("cold", 9.6)).toBe("Lạnh ghê, 10°C :(((");
    expect(warmthLine("hot", 35)).toBe("Nóng quá, 35°C :(((");
  });
});
