import { Rng } from "./rng";

/**
 * Thời tiết quanh pet. Trời không mây: ban ngày là `sunny` (mùa xuân thì `petals`), ban đêm là `clear` (sao),
 * xem `daySky`.
 */
export type Sky = "sunny" | "clear" | "cloudy" | "fog" | "rain" | "snow" | "storm" | "petals";

/** Từ nhiệt độ này trở lên là nóng (hơi nóng bốc lên quanh pet), °C. */
export const HOT_FROM = 33;
/** Từ nhiệt độ này trở xuống là lạnh (pet thở ra khói), °C. */
export const COLD_UNTIL = 15;

export type Warmth = "hot" | "cold";

/** Nóng, lạnh hay bình thường (`null`, kể cả khi chưa biết nhiệt độ). */
export function warmthOf(temperature: number | null): Warmth | null {
  if (temperature === null) return null;
  if (temperature >= HOT_FROM) return "hot";
  if (temperature <= COLD_UNTIL) return "cold";
  return null;
}

/** Kết quả thật (`WeatherReport`) quá chừng này giây thì coi như không có. */
export const REPORT_MAX_AGE = 3 * 3600;

/**
 * Mã thời tiết WMO (Open-Meteo) thành thời tiết quanh pet: 0–3 quang/nhiều mây, 45/48 sương mù, 51–67 và
 * 80–82 mưa, 71–77 và 85–86 tuyết, 95–99 sấm. Mã lạ thì coi như nhiều mây. Trời quang ban ngày: gọi `daySky`
 * sau đó.
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

/**
 * Trời quang ban ngày là nắng; mùa xuân (tháng 2–4, quanh Tết tới hết tháng 4) thì cánh hoa rơi. Ban đêm vẫn
 * là trời quang (sao). `month`: 0–11.
 */
export function daySky(sky: Sky, month: number, day: boolean): Sky {
  if (sky !== "clear" || !day) return sky;
  return month >= 1 && month <= 3 ? "petals" : "sunny";
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
 * cùng một thời tiết, mở lại app không bị đổi. Chưa phân nắng, cánh hoa: gọi `daySky` sau đó.
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
