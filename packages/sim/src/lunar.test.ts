import { describe, expect, it } from "vitest";
import { noNetwork, nowText, setLang, weatherLine } from "./lines";
import { toLunar } from "./lunar";

const lunar = (y: number, m: number, d: number) => toLunar(new Date(y, m - 1, d, 12));

describe("âm lịch", () => {
  it("mùng 1 Tết đúng các năm đã biết", () => {
    const tet: [number, number, number][] = [
      [2023, 1, 22],
      [2024, 2, 10],
      [2025, 1, 29],
      [2026, 2, 17],
      [2027, 2, 6],
      [2028, 1, 26],
    ];
    for (const [y, m, d] of tet) {
      expect(lunar(y, m, d), `${d}/${m}/${y}`).toEqual({ day: 1, month: 1, year: y, leap: false });
    }
  });

  it("ngày thường, rằm trung thu và tháng nhuận", () => {
    // Trung thu 2026 (rằm tháng 8) là 25/9/2026.
    expect(lunar(2026, 9, 25)).toEqual({ day: 15, month: 8, year: 2026, leap: false });
    expect(lunar(2026, 10, 1)).toEqual({ day: 21, month: 8, year: 2026, leap: false });
    // Năm 2025 nhuận tháng 6: mùng 1 tháng 6 nhuận là 25/7/2025.
    expect(lunar(2025, 7, 25)).toEqual({ day: 1, month: 6, year: 2025, leap: true });
    // 31/12 dương lịch vẫn thuộc năm âm lịch trước Tết.
    expect(lunar(2026, 12, 31).year).toBe(2026);
  });
});

describe("bấm đúp: giờ, ngày, thời tiết", () => {
  it("đủ giờ, thứ, ngày dương, ngày âm và thời tiết", () => {
    const date = new Date(2026, 9, 1, 15, 4);
    const text = nowText(date, toLunar(date), { place: "Hà Nội", sky: "rain", temperature: 26.6 });
    expect(text).toBe("15:04 · Thứ Năm 01/10\nÂm lịch 21/8\nHà Nội: mưa, 27°C");
  });

  it("chưa chọn thành phố thì không nói thời tiết; chưa có thời tiết thì nói rõ", () => {
    // Mùng 1 Tết 2026.
    const date = new Date(2026, 1, 17, 8, 30);
    expect(nowText(date, toLunar(date), null)).toBe("08:30 · Thứ Ba 17/02\nÂm lịch 1/1");
    expect(nowText(date, toLunar(date), { place: "Huế", sky: null })).toContain("Huế: chưa có thời tiết");
    expect(nowText(date, toLunar(date), { place: "Huế", sky: null, problem: noNetwork() })).toBe(
      "08:30 · Thứ Ba 17/02\nÂm lịch 1/1\nKhông có mạng :))",
    );
  });

  it("tháng nhuận ghi rõ", () => {
    const date = new Date(2025, 6, 25, 8, 30);
    expect(nowText(date, toLunar(date), null)).toContain("Âm lịch 1/6 nhuận");
  });

  it("tiếng Anh: tên thứ, tháng và thời tiết bằng tiếng Anh", () => {
    setLang("en");
    try {
      const date = new Date(2026, 9, 1, 15, 4);
      expect(nowText(date, toLunar(date), { place: "Hanoi", sky: "rain", temperature: 26.6 })).toBe(
        "15:04 · Thursday, Oct 1\nLunar date 8/21\nHanoi: rainy, 27°C",
      );
      expect(weatherLine({ place: "Hue", sky: null, problem: noNetwork() })).toBe("No internet :))");
    } finally {
      setLang("vi");
    }
  });
});
