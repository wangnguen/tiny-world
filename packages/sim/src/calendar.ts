import type { Occasion } from "@tinyworld/core";
import { toLunar } from "./lunar";

/**
 * Giờ đồng hồ ở múi giờ `timeZone` (IANA, của thành phố đã chọn): trả về một `Date` mà `getHours()`,
 * `getDate()`... ra đúng giờ, ngày ở đó. Không có hoặc sai múi giờ thì dùng giờ máy.
 */
export function wallClock(now: Date, timeZone: string | null | undefined): Date {
  if (!timeZone) return now;
  try {
    const parts = new Intl.DateTimeFormat("en-US", {
      timeZone,
      hourCycle: "h23",
      year: "numeric",
      month: "numeric",
      day: "numeric",
      hour: "numeric",
      minute: "numeric",
      second: "numeric",
    }).formatToParts(now);
    const get = (type: Intl.DateTimeFormatPartTypes) => Number(parts.find((p) => p.type === type)?.value);
    const wall = new Date(get("year"), get("month") - 1, get("day"), get("hour"), get("minute"), get("second"));
    return Number.isNaN(wall.getTime()) ? now : wall;
  } catch {
    return now;
  }
}

/** Ngày bắt đầu của dịp `occasion` có nằm trong `occasion.days` ngày gần nhất tính tới `today` không. */
function happening(occasion: Occasion, today: Date): boolean {
  for (let back = 0; back < occasion.days; back++) {
    const day = new Date(today.getFullYear(), today.getMonth(), today.getDate() - back, 12);
    if (occasion.lunar) {
      const lunar = toLunar(day);
      if (!lunar.leap && lunar.day === occasion.day && lunar.month === occasion.month) return true;
    } else if (day.getDate() === occasion.day && day.getMonth() + 1 === occasion.month) {
      return true;
    }
  }
  return false;
}

/**
 * Các dịp đang diễn ra hôm nay (`today` theo giờ ở thành phố đã chọn, `wallClock`), theo thứ tự trong lịch.
 * Ngày không có trong năm đó (30 âm lịch tháng thiếu, 29/2 năm không nhuận) thì năm đó không có.
 */
export function activeOccasions(occasions: readonly Occasion[], today: Date): Occasion[] {
  return occasions.filter((occasion) => occasion.enabled && happening(occasion, today));
}
