export interface Point {
  x: number;
  y: number;
}

/** Event `cursor-moved`: vị trí con trỏ theo CSS pixel của overlay. */
export interface CursorInfo extends Point {
  /** Đang giữ Ctrl: click xuyên qua pet xuống app bên dưới. */
  passThrough: boolean;
  /** Đang giữ một nút chuột, ở bất kỳ đâu trên màn hình (click ra ngoài cũng làm pet đang ngủ thức dậy). */
  pressed: boolean;
}

/** Hình chữ nhật theo CSS pixel, gốc toạ độ là góc trên trái của overlay. */
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Màn hình overlay đang nằm (command `screen_info`), đã đổi sang CSS pixel. Overlay phủ vùng làm
 * việc chứ không phủ cả màn hình, nên gốc toạ độ là góc trên trái vùng làm việc.
 */
export interface ScreenInfo {
  /** Tỉ lệ DPI của màn hình: 1 là 100%, 1.5 là 150%. */
  scaleFactor: number;
  /** Toàn bộ màn hình, kể cả taskbar (taskbar ở trên/trái thì `x`/`y` âm). */
  bounds: Rect;
  /** Vùng làm việc (trừ taskbar), trùng với kích thước overlay. Mép dưới là chỗ pet đứng. */
  workArea: Rect;
  /**
   * Vùng làm việc của các màn hình khác, tính theo toạ độ của overlay này. Màn hình nào giáp mép trái/phải
   * của `workArea` thì pet đi hoặc bay sang được bên đó.
   */
  neighbors: Rect[];
}

/** Đổi toạ độ CSS pixel của overlay cũ sang overlay mới: `p * scale + (x, y)`. */
export interface Remap {
  scale: number;
  x: number;
  y: number;
}

/**
 * Event `screen-changed`: overlay vừa sang màn hình khác, hoặc màn hình đang ở đổi độ phân giải, DPI,
 * taskbar. `remap` đổi toạ độ cũ (pet, cửa sổ) sang toạ độ mới.
 */
export interface ScreenChange {
  screen: ScreenInfo;
  remap: Remap;
}

/** Một cửa sổ thật trên màn hình, khớp `WindowInfo` trong windows.rs. */
export interface WindowInfo {
  /** HWND, không đổi trong suốt đời cửa sổ. */
  id: number;
  /** Khung nhìn thấy (không tính viền kéo giãn trong suốt), CSS pixel; có thể thò ra ngoài overlay. */
  rect: Rect;
}

/**
 * Event `windows-changed` và command `list_windows`: các cửa sổ đang hiện (không tính cửa sổ thu
 * nhỏ, ẩn, ở desktop ảo khác), xếp từ trên xuống dưới theo thứ tự chồng.
 */
export interface WindowList {
  windows: WindowInfo[];
  /** Cửa sổ vừa bị đóng hẳn (không phải thu nhỏ hay ẩn) kể từ lần gửi trước, kèm khung lúc còn hiện. */
  closed: WindowInfo[];
  /** Taskbar tự ẩn đang trồi lên che mép dưới overlay: mép trên của nó (CSS pixel), pet đứng trên đó. */
  taskbarTop: number | null;
}

/** Số nhân vật tối đa cùng sống trên màn hình, khớp `MAX_PETS` trong settings.rs. */
export const MAX_PETS = 3;

/** Giới hạn của lịch sự kiện, khớp `MAX_OCCASIONS`, `OCCASION_*` trong settings.rs. */
export const MAX_OCCASIONS = 30;
export const OCCASION_NAME_MAX = 40;
export const OCCASION_MESSAGE_MAX = 80;
export const OCCASION_DAYS_MAX = 10;

/**
 * Thời tiết quanh pet, khớp `Sky` trong preview.rs. Trời không mây: ban ngày là `sunny` (mùa xuân thì
 * `petals`), ban đêm là `clear` (sao), xem `daySky` trong packages/sim.
 */
export type Sky = "sunny" | "clear" | "cloudy" | "fog" | "rain" | "snow" | "storm" | "petals";

/**
 * Xem thử thời tiết, nhiệt độ (mục Xem thử trong Cài đặt), khớp `WeatherPreview` trong preview.rs: `null` là
 * theo trời thật, cả hai `null` là thôi xem thử. Xem thử kéo dài `PREVIEW_SECONDS` giây.
 */
export interface WeatherPreview {
  sky: Sky | null;
  /** °C. */
  temperature: number | null;
}

/** Overlay đang xem thử gì (nút đang bật trong Cài đặt), khớp `PreviewState` trong preview.rs. */
export interface PreviewState extends WeatherPreview {
  /** Hết xem thử thời tiết, nhiệt độ lúc này (ms từ 1970); `null` là không xem thử. */
  until: number | null;
  /** Con ma đang bay. */
  ghost: boolean;
}

export const PREVIEW_SECONDS = 30;
/** Nút Nóng, Lạnh trong mục Xem thử (°C). */
export const PREVIEW_HOT = 36;
export const PREVIEW_COLD = 8;

/** Ngôn ngữ chọn trong Cài đặt, khớp `Language` trong i18n.rs: theo Windows, tiếng Việt, tiếng Anh. */
export type Language = "auto" | "vi" | "en";

/** Ngôn ngữ đang dùng, khớp `Lang` trong i18n.rs. */
export type Lang = "vi" | "en";

/** Cài đặt người dùng (command `get_settings` / `set_settings`), khớp `Settings` trong settings.rs. */
export interface Settings {
  /** Cỡ nhân vật so với cỡ gốc của sprite pack, 0.5–2. */
  size: number;
  /** Hệ số tốc độ đi/chạy, 0.5–2. */
  speed: number;
  /**
   * Các nhân vật đang hiện (tối đa `MAX_PETS`, không trùng), theo thứ tự chọn: tên thư mục sprite pack
   * trong `assets/sprites/`. Rỗng (hoặc không pack nào còn) là pack đầu tiên theo tên thư mục.
   */
  pets: string[];
  /** Thành phố để lấy thời tiết thật; `null` là thời tiết giả lập, không gọi mạng. */
  city: City | null;
  /** Hiệu ứng thời tiết quanh pet (nắng, mây, sao, mưa, tuyết, sương mù, sấm, cánh hoa; nóng, lạnh). */
  weather: boolean;
  /** Nhãn nhiệt độ cạnh pet (cần chọn thành phố). */
  temperatureTag: boolean;
  /** Pet nói câu cho vui (chào nhau, thời tiết). */
  chatter: boolean;
  /** Lịch sự kiện: đúng dịp thì một con nói câu của dịp đó. */
  events: boolean;
  /** Các dịp trong lịch sự kiện; mặc định là các ngày lễ Việt Nam. */
  occasions: Occasion[];
  /** Sự kiện hiếm: ma bay qua lúc 2 giờ sáng. */
  ghost: boolean;
  /** Phase 5, mặc định tắt hết. Đếm giờ ngồi máy theo ngày (`stats.json`). */
  screenTime: boolean;
  /** Ngồi liền `breakMinutes` phút (`BREAK_MINUTES`) thì pet nhắc nghỉ. */
  breakReminder: boolean;
  breakMinutes: number;
  /** Cứ ngồi máy đủ `waterMinutes` phút (`WATER_MINUTES`) thì pet nhắc uống nước. */
  waterReminder: boolean;
  waterMinutes: number;
  /** Còn ngồi máy sau `bedtime` (phút trong ngày, giờ máy) thì pet nhắc đi ngủ, tối đa 30 phút một lần. */
  bedtimeReminder: boolean;
  bedtime: number;
  /** Bấm Ctrl+S dồn dập (5 lần trong 10 giây) thì pet kêu. */
  saveSpam: boolean;
  /** Phase 6, mặc định tắt: click chuột phải vào pet để chat (gửi câu hỏi tới Gemini). */
  chat: boolean;
  /** Ngôn ngữ chọn trong Cài đặt; mặc định theo Windows. */
  language: Language;
  /** Ngôn ngữ đang dùng, Rust tính từ `language` (và ngôn ngữ Windows); gửi lại Rust thì bị bỏ qua. */
  lang: Lang;
}

/** Đang chat với con nào, khớp `ChatTarget` trong chat.rs. */
export interface ChatTarget {
  /** Tên thư mục sprite pack ("placeholder" là pet tạm). */
  pet: string;
  /** Tên ngắn để hiện, ví dụ "Momo". */
  name: string;
}

/** Khoảng chọn được của `Settings.breakMinutes`, khớp `BREAK_MINUTES` trong settings.rs. */
export const BREAK_MINUTES = { min: 15, max: 120 } as const;
/** Khoảng chọn được của `Settings.waterMinutes`, khớp `WATER_MINUTES` trong settings.rs. */
export const WATER_MINUTES = { min: 20, max: 180 } as const;

/** Event `reminder`, khớp `Reminder` trong activity.rs. */
export type Reminder =
  | { kind: "break"; minutes: number }
  | { kind: "water" }
  | { kind: "bedtime" }
  | { kind: "saveSpam" };

/** Giờ ngồi máy một ngày (giờ máy), khớp `Day` trong activity.rs. */
export interface DayStats {
  activeMs: number;
  /** Lượt ngồi liền lâu nhất trong ngày (ms). */
  longestMs: number;
  /** Số lần đứng dậy nghỉ (vắng từ 5 phút) sau một lượt ngồi. */
  breaks: number;
}

/** Command `get_stats`, khớp `StatsView` trong activity.rs. */
export interface ScreenStats {
  /** Hôm nay theo giờ máy, "2026-10-01". */
  today: string;
  /** Đang ngồi liền bao lâu (ms). */
  sessionMs: number;
  /** Tối đa 30 ngày gần nhất, theo ngày "2026-10-01". */
  days: Record<string, DayStats>;
}

/**
 * Một dịp trong lịch sự kiện, khớp `Occasion` trong settings.rs: lặp lại mỗi năm vào ngày `day/month`
 * (dương lịch, hoặc âm lịch Việt Nam nếu `lunar`) trong `days` ngày.
 */
export interface Occasion {
  name: string;
  day: number;
  month: number;
  lunar: boolean;
  /** Kéo dài 1–10 ngày kể từ ngày đó. */
  days: number;
  /** Câu một con nói (mỗi ngày một lần) trong dịp đó; rỗng là không nói. */
  message: string;
  enabled: boolean;
  /**
   * Dịp có sẵn chưa sửa tên hay câu nói (mục trong `occasions` của file chữ): tên và câu hiện theo ngôn ngữ
   * đang dùng (`occasionText`), `name`/`message` giữ bản tiếng Việt. Sửa rồi thì bỏ.
   */
  preset?: string;
}

/** Thành phố đã chọn trong Cài đặt, khớp `City` trong settings.rs. */
export interface City {
  /** Tên để hiện, ví dụ "Hà Nội, Việt Nam". */
  name: string;
  latitude: number;
  longitude: number;
  /** Múi giờ IANA, ví dụ "Asia/Bangkok": giờ của pet theo múi giờ này; `null` là giờ máy. */
  timezone: string | null;
}

/** Một kết quả tìm thành phố (command `search_city`), khớp `CityResult` trong weather.rs. */
export interface CityResult {
  label: string;
  latitude: number;
  longitude: number;
  timezone: string | null;
}

/** Thời tiết hiện tại ở thành phố đã chọn (Open-Meteo), khớp `Report` trong weather.rs. */
export interface WeatherReport {
  latitude: number;
  longitude: number;
  /** Mã thời tiết WMO. */
  code: number;
  /** Mặt trời đang mọc ở chỗ đó. */
  isDay: boolean;
  /** °C. */
  temperature: number;
  /** km/h. */
  windSpeed: number;
  /** Lúc lấy về, số giây từ 1970 (UTC). */
  fetchedAt: number;
}

/** Event `weather-failed`: không lấy được thời tiết, khớp `Failure` trong weather.rs. */
export interface WeatherFailure {
  /** Mất mạng hoặc máy chủ không trả lời; `false` là máy chủ trả lỗi. */
  offline: boolean;
}

/** Event `update-available` và command `get_update`/`check_update`, khớp `UpdateInfo` trong update.rs. */
export interface UpdateInfo {
  /** Bản mới, ví dụ "1.6.0". */
  version: string;
  /** Bản đang chạy. */
  current: string;
  /** Trang release trên GitHub (có gì mới). */
  page: string;
  /** Cỡ bộ cài (byte). */
  size: number;
}

/** Event `update-progress`: đang tải bộ cài, khớp `UpdateProgress` trong update.rs. */
export interface UpdateProgress {
  /** Byte đã tải. */
  received: number;
  total: number;
}

export interface AppError {
  code: ErrorCode;
  message: string;
}

/** `OFFLINE`: mất mạng hoặc máy chủ không trả lời (thời tiết, chat). */
/** `BUSY`: app tự chặn để không gửi dồn dập; `UNAVAILABLE`: dịch vụ bên ngoài lỗi hay đang chặn. */
export type ErrorCode = "BAD_REQUEST" | "NO_WINDOW" | "INTERNAL" | "OFFLINE" | "BUSY" | "UNAVAILABLE";

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value
  );
}

export function errorMessage(value: unknown): string {
  if (isAppError(value)) return value.message;
  if (value instanceof Error) return value.message;
  return String(value);
}
