import type { Occasion } from "@tinyworld/core";
import { describe, expect, it } from "vitest";
import { activeOccasions, hatOf, wallClock } from "./calendar";

const occasion = (o: Partial<Occasion> & Pick<Occasion, "name" | "day" | "month">): Occasion => ({
  lunar: false,
  days: 1,
  hat: "none",
  message: "",
  enabled: true,
  ...o,
});

const tet = occasion({ name: "Tết Nguyên Đán", day: 1, month: 1, lunar: true, days: 5, hat: "tet" });
const noel = occasion({ name: "Giáng sinh", day: 24, month: 12, days: 2, hat: "noel" });
const birthday = occasion({ name: "Sinh nhật", day: 17, month: 2, hat: "party", message: "Chúc mừng sinh nhật :)))" });
const midAutumn = occasion({ name: "Trung thu", day: 15, month: 8, lunar: true });

const at = (y: number, m: number, d: number) => new Date(y, m - 1, d, 9);
const names = (date: Date, list: Occasion[]) => activeOccasions(list, date).map((o) => o.name);

describe("giờ theo thành phố", () => {
  it("đổi sang giờ ở múi giờ của thành phố", () => {
    // 1/10/2026 08:30 UTC: Hà Nội 15:30, Tokyo 17:30, New York 04:30.
    const now = new Date(Date.UTC(2026, 9, 1, 8, 30));
    const clock = (tz: string) => {
      const d = wallClock(now, tz);
      return [d.getDate(), d.getHours(), d.getMinutes()];
    };
    expect(clock("Asia/Bangkok")).toEqual([1, 15, 30]);
    expect(clock("Asia/Tokyo")).toEqual([1, 17, 30]);
    expect(clock("America/New_York")).toEqual([1, 4, 30]);
  });

  it("chưa có hoặc sai múi giờ thì dùng giờ máy", () => {
    const now = new Date(2026, 9, 1, 15, 30);
    expect(wallClock(now, null)).toBe(now);
    expect(wallClock(now, "Hành tinh/Sao Hoả")).toBe(now);
  });
});

describe("lịch sự kiện", () => {
  it("Tết theo âm lịch, kéo dài từ mùng 1 tới mùng 5", () => {
    expect(names(at(2026, 2, 17), [tet])).toEqual(["Tết Nguyên Đán"]);
    expect(names(at(2026, 2, 21), [tet])).toEqual(["Tết Nguyên Đán"]);
    expect(names(at(2026, 2, 22), [tet])).toEqual([]);
    expect(names(at(2026, 2, 16), [tet])).toEqual([]);
    // Năm sau Tết sang ngày khác.
    expect(names(at(2027, 2, 6), [tet])).toEqual(["Tết Nguyên Đán"]);
  });

  it("dương lịch kéo dài qua ngày; âm lịch như rằm trung thu", () => {
    expect(names(at(2026, 12, 25), [noel])).toEqual(["Giáng sinh"]);
    expect(names(at(2026, 12, 26), [noel])).toEqual([]);
    expect(names(at(2026, 9, 25), [midAutumn])).toEqual(["Trung thu"]);
  });

  it("nhiều dịp cùng ngày: mũ theo dịp xếp trước có mũ; dịp tắt thì không tính", () => {
    const list = [midAutumn, birthday, tet];
    const date = at(2026, 2, 17);
    expect(names(date, list)).toEqual(["Sinh nhật", "Tết Nguyên Đán"]);
    expect(hatOf(activeOccasions(list, date))).toBe("party");
    expect(hatOf(activeOccasions([{ ...birthday, enabled: false }, tet], date))).toBe("tet");
    expect(hatOf(activeOccasions(list, at(2026, 5, 5)))).toBe("none");
  });
});
