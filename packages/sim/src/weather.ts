import { Rng } from "./rng";

/** Thời tiết quanh pet. `clear`, `cloudy` không có hiệu ứng gì. */
export type Sky = "clear" | "cloudy" | "fog" | "rain" | "snow" | "storm" | "petals";

/** Thời tiết có hiệu ứng vẽ quanh pet. */
export const ANIMATED_SKIES: ReadonlySet<Sky> = new Set(["fog", "rain", "snow", "storm", "petals"]);

/** Kết quả thật (`WeatherReport`) quá chừng này giây thì coi như không có. */
export const REPORT_MAX_AGE = 3 * 3600;

/**
 * Mã thời tiết WMO (Open-Meteo) thành thời tiết quanh pet: 0–3 quang/nhiều mây, 45/48 sương mù, 51–67 và
 * 80–82 mưa, 71–77 và 85–86 tuyết, 95–99 sấm. Mã lạ thì coi như nhiều mây.
 */
export function skyOf(code: number): Sky {
  if (code <= 1) return "clear";
  if (code <= 3) return "cloudy";
  if (code === 45 || code === 48) return "fog";
  if ((code >= 51 && code <= 67) || (code >= 80 && code <= 82)) return "rain";
  if ((code >= 71 && code <= 77) || code === 85 || code === 86) return "snow";
  if (code >= 95 && code <= 99) return "storm";
  return "cloudy";
}

/** Mùa xuân (tháng 2–4, quanh Tết tới hết tháng 4), trời quang, ban ngày: cánh hoa rơi. `month`: 0–11. */
export function withPetals(sky: Sky, month: number, day: boolean): Sky {
  return sky === "clear" && day && month >= 1 && month <= 3 ? "petals" : sky;
}

type Season = "spring" | "summer" | "autumn" | "winter";

function seasonOf(month: number): Season {
  if (month >= 1 && month <= 3) return "spring";
  if (month >= 4 && month <= 7) return "summer";
  if (month >= 8 && month <= 10) return "autumn";
  return "winter";
}

/** Tỉ lệ từng kiểu thời tiết giả lập theo mùa (cộng lại bằng 1). */
const SEASONS: Record<Season, [Sky, number][]> = {
  spring: [
    ["clear", 0.45],
    ["cloudy", 0.3],
    ["rain", 0.15],
    ["fog", 0.1],
  ],
  summer: [
    ["clear", 0.4],
    ["cloudy", 0.25],
    ["rain", 0.25],
    ["storm", 0.1],
  ],
  autumn: [
    ["clear", 0.45],
    ["cloudy", 0.3],
    ["rain", 0.2],
    ["fog", 0.05],
  ],
  winter: [
    ["clear", 0.35],
    ["cloudy", 0.35],
    ["fog", 0.15],
    ["rain", 0.1],
    ["snow", 0.05],
  ],
};

/** Mỗi khung giờ thời tiết giả lập dài chừng này giờ. */
const SIMULATED_BLOCK_HOURS = 3;

/**
 * Thời tiết giả lập (chưa chọn thành phố): đổi theo mùa mỗi 3 tiếng. Cùng ngày, cùng khung giờ thì luôn ra
 * cùng một thời tiết, mở lại app không bị đổi. Chưa có cánh hoa: gọi `withPetals` sau đó.
 */
export function simulatedSky(date: Date): Sky {
  const block = Math.floor(date.getHours() / SIMULATED_BLOCK_HOURS);
  const day = date.getFullYear() * 10_000 + (date.getMonth() + 1) * 100 + date.getDate();
  const roll = new Rng(day * 8 + block).next();
  let sum = 0;
  for (const [sky, share] of SEASONS[seasonOf(date.getMonth())]) {
    sum += share;
    if (roll < sum) return sky;
  }
  return "clear";
}
