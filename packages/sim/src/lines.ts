/**
 * Mọi câu pet nói (speech bubble), gom một chỗ để dễ sửa câu chữ. Giọng thân mật, dùng ":)))" thay emoji.
 */
import type { LunarDate } from "./lunar";
import type { Sky, Warmth } from "./weather";

/** Câu chào khi gặp nhau (chọn ngẫu nhiên một câu). */
export const GREETINGS = ["Chào :)))", "Xin chào :)))", "Ơ, cậu đây rồi!", ":))))?", "Đi đâu đấy :)?"];

/** Trời vừa chuyển sang kiểu thời tiết này: một con nói một câu (theo giới hạn tần suất). [câu, mặt cười] */
const SKY_TALK: Record<Sky, [string, string]> = {
  sunny: ["Nắng đẹp ghê", ":)))"],
  clear: ["Trời quang, sao đẹp ghê", ":)))"],
  cloudy: ["Trời nhiều mây quá", ":)"],
  rain: ["Mưa rồi", ":("],
  storm: ["Sấm to quá", ":((("],
  snow: ["Tuyết rơi kìa", ":)))"],
  fog: ["Sương mù dày ghê", ":)"],
  petals: ["Hoa rơi đẹp quá", ":)))"],
};

const degrees = (temperature: number) => `${Math.round(temperature)}°C`;

/** Câu khi trời vừa đổi, kèm nhiệt độ nếu biết, ví dụ "Mưa rồi, 24°C :(". Nắng mà nóng thì kêu nóng. */
export function skyLine(sky: Sky, temperature: number | null, warmth: Warmth | null): string {
  const [text, mood] = sky === "sunny" && warmth === "hot" ? ["Nắng to quá", ":((("] : SKY_TALK[sky];
  return temperature === null ? `${text} ${mood}` : `${text}, ${degrees(temperature)} ${mood}`;
}

/** Câu khi trời vừa chuyển nóng hay lạnh (thời tiết không đổi), ví dụ "Nóng quá, 36°C :(((". */
export function warmthLine(warmth: Warmth, temperature: number): string {
  return warmth === "hot" ? `Nóng quá, ${degrees(temperature)} :(((` : `Lạnh ghê, ${degrees(temperature)} :(((`;
}

/** Không lấy được thời tiết: mất mạng, hoặc máy chủ thời tiết lỗi. */
export const NO_NETWORK = "Không có mạng :))";
export const NO_WEATHER = "Không có thời tiết :))";

/** Con ma bay qua lúc 2 giờ sáng: con đầu tiên bị giật mình nói câu này. */
export const GHOST_LINE = "Ma... ma kìa :(((";

/** Nhắc nghỉ sau `minutes` phút ngồi liền. */
export const breakLine = (minutes: number) => `Ngồi liền ${minutes} phút rồi, đứng dậy nghỉ mắt chút đi :)))`;
/** Nhắc uống nước sau một lúc ngồi máy. */
export const WATER_LINE = "Uống ngụm nước đi nè :)))";
/** Nhắc đi ngủ lúc khuya, `time` ví dụ "23:40". */
export const bedtimeLine = (time: string) => `${time} rồi, đi ngủ thôi :(((`;
/** Bấm Ctrl+S dồn dập. */
export const SAVE_SPAM_LINE = "Lưu rồi mà :((((";

/** Click chuột phải vào pet mà chưa bật chat. */
export const CHAT_OFF_LINE = "Bật Chat với pet trong Cài đặt rồi nói chuyện nhé :)))";

/** Thời tiết khi bấm đúp vào pet (trời nắng mùa xuân thì là `petals`). */
export const SKY_NAMES: Record<Sky, string> = {
  sunny: "trời nắng",
  clear: "trời quang",
  cloudy: "nhiều mây",
  fog: "sương mù",
  rain: "mưa",
  snow: "tuyết",
  storm: "có sấm",
  petals: "trời nắng",
};

/** Đã chọn thành phố mà chưa lấy được thời tiết lần nào (hoặc kết quả đã quá cũ). */
export const WEATHER_UNKNOWN = "chưa có thời tiết";

export const WEEKDAYS = ["Chủ nhật", "Thứ Hai", "Thứ Ba", "Thứ Tư", "Thứ Năm", "Thứ Sáu", "Thứ Bảy"];

/** Thời tiết ở thành phố đã chọn để nói khi bấm đúp; `sky` null là chưa có thời tiết. */
export interface PlaceWeather {
  /** Tên ngắn của thành phố, ví dụ "Hà Nội". */
  place: string;
  sky: Sky | null;
  temperature?: number;
  /** Lần lấy thời tiết gần nhất bị lỗi: nói câu này (`NO_NETWORK`, `NO_WEATHER`) thay cho thời tiết. */
  problem?: string;
}

const pad = (n: number) => String(n).padStart(2, "0");

/** Một dòng thời tiết, ví dụ "Hà Nội: mưa, 27°C"; lấy không được thì là câu báo lỗi (`problem`). */
export function weatherLine(weather: PlaceWeather): string {
  if (weather.sky && weather.temperature !== undefined) {
    return `${weather.place}: ${SKY_NAMES[weather.sky]}, ${degrees(weather.temperature)}`;
  }
  return weather.problem ?? `${weather.place}: ${WEATHER_UNKNOWN}`;
}

/**
 * Câu khi bấm đúp vào pet: giờ, thứ, ngày dương lịch, ngày âm lịch, và thời tiết nếu đã chọn thành phố.
 * Ví dụ "15:42 · Thứ Tư 01/10", "Âm lịch 21/8", "Hà Nội: mưa, 27°C", mỗi phần một dòng.
 */
export function nowText(date: Date, lunar: LunarDate, weather: PlaceWeather | null): string {
  const lines = [
    `${pad(date.getHours())}:${pad(date.getMinutes())} · ${WEEKDAYS[date.getDay()]} ${pad(date.getDate())}/${pad(date.getMonth() + 1)}`,
    `Âm lịch ${lunar.day}/${lunar.month}${lunar.leap ? " nhuận" : ""}`,
  ];
  if (weather) lines.push(weatherLine(weather));
  return lines.join("\n");
}
