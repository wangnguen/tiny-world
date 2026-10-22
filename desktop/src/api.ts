import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  EVENTS,
  type ChatReply,
  type ChatTarget,
  type CityResult,
  type PreviewState,
  type WeatherPreview,
  type ScreenStats,
  type Settings,
  type UpdateInfo,
  type UpdateProgress,
  type WeatherFailure,
  type WeatherReport,
} from "@tinyworld/core";

/**
 * Gọi command `name` của overlay (native/src/commands.rs). Cửa sổ Cài đặt và khung chat chạy ở tiến trình riêng,
 * mọi command đi qua một Tauri command `call` (desktop/src-tauri/src/ui.rs) rồi sang overlay.
 */
function call<T>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  return invoke<T>("call", { name, args });
}

export const api = {
  getSettings: () => call<Settings>("get_settings"),
  /** Trả về giá trị thật sự được lưu (Rust kẹp vào khoảng cho phép). */
  setSettings: (settings: Settings) => call<Settings>("set_settings", { settings }),
  /** Chạy cùng Windows (khoá Run trong registry). */
  getAutostart: () => call<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => call<void>("set_autostart", { enabled }),
  /** Version ghi trong `tauri.conf.json` lúc build (workflow Build và Release ghi vào trước khi build). */
  version: () => getVersion(),
  /** Thời tiết gần nhất của thành phố đang chọn; `null` nếu chưa chọn hoặc chưa lấy được. */
  getWeather: () => call<WeatherReport | null>("get_weather"),
  /** Tìm thành phố theo tên (Open-Meteo); mất mạng thì lỗi `OFFLINE`. */
  searchCity: (query: string) => call<CityResult[]>("search_city", { query }),
  /** Giờ ngồi máy đã lưu (chỉ có khi bật `screenTime`). */
  getStats: () => call<ScreenStats>("get_stats"),
  /** Xoá hết giờ ngồi máy đã lưu. */
  clearStats: () => call<void>("clear_stats"),
  /**
   * Mục Xem thử trong Cài đặt: pet gặp ngay thời tiết, nhiệt độ `preview`; cả hai `null` là thôi xem thử. Pet đang
   * ẩn, tạm dừng thì lỗi.
   */
  previewWeather: (preview: WeatherPreview) => call<void>("preview_weather", { preview }),
  /** Mục Xem thử: con ma bay qua ngay. */
  previewGhost: () => call<void>("preview_ghost"),
  /** Cài đặt vừa mở: đang xem thử gì. */
  getPreview: () => call<PreviewState>("get_preview"),
  /** Câu gợi ý của hôm nay do Gemini viết (`today` "2026-10-01", `date` "Thứ Năm 01/10/2026"); rỗng là chưa có. */
  chatSuggestions: (today: string, date: string) => call<string[]>("chat_suggestions", { today, date }),
  /** Cửa sổ chat đang chat với con nào. */
  chatTarget: () => call<ChatTarget | null>("chat_target"),
  /** Gửi câu hỏi, chờ câu trả lời; gửi dồn dập thì lỗi `BUSY`, mất mạng `OFFLINE`, Google lỗi `UNAVAILABLE`. */
  sendChat: (prompt: string) => call<ChatReply>("send_chat", { prompt }),
  /** Mở link http/https bằng trình duyệt. */
  openLink: (url: string) => call<void>("open_link", { url }),
  /** Mở thư mục chứa `tinyworld.log` bằng Explorer. */
  openLogFolder: () => call<void>("open_log_folder"),
  /** Bản mới theo lần hỏi GitHub gần nhất; `null` là chưa thấy. */
  getUpdate: () => call<UpdateInfo | null>("get_update"),
  /** Hỏi GitHub ngay; `null` là đang dùng bản mới nhất, mất mạng thì lỗi `OFFLINE`. */
  checkUpdate: () => call<UpdateInfo | null>("check_update"),
  /**
   * Tải bộ cài bản mới (tiến độ qua `onUpdateProgress`) rồi thoát app; bộ cài chạy lúc app thoát hẳn, cài đè rồi tự
   * mở lại TinyWorld. Đang tải thì lỗi `BUSY`.
   */
  installUpdate: () => call<void>("install_update"),

  onSettingsChanged: (callback: (settings: Settings) => void) =>
    listen<Settings>(EVENTS.settingsChanged, (event) => callback(event.payload)),
  onWeatherChanged: (callback: (report: WeatherReport | null) => void) =>
    listen<WeatherReport | null>(EVENTS.weatherChanged, (event) => callback(event.payload)),
  onWeatherFailed: (callback: (failure: WeatherFailure) => void) =>
    listen<WeatherFailure>(EVENTS.weatherFailed, (event) => callback(event.payload)),
  onChatTarget: (callback: (target: ChatTarget) => void) =>
    listen<ChatTarget>(EVENTS.chatTarget, (event) => callback(event.payload)),
  /** Chữ của câu trả lời đã có tới giờ, trong lúc chờ `sendChat`. */
  onChatPartial: (callback: (text: string) => void) =>
    listen<string>(EVENTS.chatPartial, (event) => callback(event.payload)),
  /** Overlay vừa đổi thứ đang xem thử (cửa sổ Cài đặt nghe). */
  onPreviewChanged: (callback: (state: PreviewState) => void) =>
    listen<PreviewState>(EVENTS.previewChanged, (event) => callback(event.payload)),
  /** GitHub có bản mới hơn bản đang chạy (vòng hỏi nền hoặc nút Kiểm tra bản mới). */
  onUpdateAvailable: (callback: (info: UpdateInfo) => void) =>
    listen<UpdateInfo>(EVENTS.updateAvailable, (event) => callback(event.payload)),
  /** Đang tải bộ cài bản mới (sau khi bấm Cập nhật ngay). */
  onUpdateProgress: (callback: (progress: UpdateProgress) => void) =>
    listen<UpdateProgress>(EVENTS.updateProgress, (event) => callback(event.payload)),
};
