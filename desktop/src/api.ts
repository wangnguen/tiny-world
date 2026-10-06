import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  EVENTS,
  type ChatTarget,
  type CityResult,
  type CursorInfo,
  type PreviewState,
  type WeatherPreview,
  type Rect,
  type ScreenChange,
  type Reminder,
  type ScreenInfo,
  type ScreenStats,
  type Settings,
  type WeatherFailure,
  type WeatherReport,
  type WindowList,
} from "@tinyworld/core";

/** Wrapper cho các Tauri command trong src-tauri/src/commands.rs. */
export const api = {
  screenInfo: () => invoke<ScreenInfo>("screen_info"),
  /**
   * Pet bị kéo hoặc đi ra khỏi overlay tới điểm này (CSS pixel của overlay): điểm đó nằm trên màn hình
   * khác thì overlay sang đó, rồi báo qua `onScreenChanged`.
   */
  moveOverlay: (x: number, y: number) => invoke<void>("move_overlay", { x, y }),
  /** Các cửa sổ đang hiện, lấy một lần lúc mở; sau đó theo `onWindowsChanged`. */
  listWindows: () => invoke<WindowList>("list_windows"),
  /** `true`: chuột đi xuyên overlay xuống app bên dưới. */
  setClickThrough: (enabled: boolean) => invoke<void>("set_click_through", { enabled }),
  /**
   * Vùng quanh từng pet (CSS pixel của overlay) cần biết vị trí con trỏ; ngoài các vùng này Rust chỉ báo
   * lúc bấm/nhả chuột. `null`: mọi chỗ.
   */
  setCursorInterest: (rects: Rect[] | null) => invoke<void>("set_cursor_interest", { rects }),
  /** Mọi pet đều ngủ, vòng lặp vẽ dừng (`true`), hoặc có con thức dậy: Rust bảo WebView2 dùng ít RAM lúc ngủ. */
  setResting: (resting: boolean) => invoke<void>("set_resting", { resting }),
  /** Trạng thái thế giới pet đã lưu, `null` nếu chưa có. */
  loadState: () => invoke<unknown>("load_state"),
  saveState: (state: unknown) => invoke<void>("save_state", { state }),
  getSettings: () => invoke<Settings>("get_settings"),
  /** Trả về giá trị thật sự được lưu (Rust kẹp vào khoảng cho phép). */
  setSettings: (settings: Settings) => invoke<Settings>("set_settings", { settings }),
  /** Chạy cùng Windows (khoá Run trong registry). */
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  quit: () => invoke<void>("quit"),
  /** Version ghi trong `tauri.conf.json` lúc build (workflow Build và Release ghi vào trước khi build). */
  version: () => getVersion(),
  /** Thời tiết gần nhất của thành phố đang chọn; `null` nếu chưa chọn hoặc chưa lấy được. */
  getWeather: () => invoke<WeatherReport | null>("get_weather"),
  /** Tìm thành phố theo tên (Open-Meteo); mất mạng thì lỗi `OFFLINE`. */
  searchCity: (query: string) => invoke<CityResult[]>("search_city", { query }),
  /** Giờ ngồi máy đã lưu (chỉ có khi bật `screenTime`). */
  getStats: () => invoke<ScreenStats>("get_stats"),
  /** Xoá hết giờ ngồi máy đã lưu. */
  clearStats: () => invoke<void>("clear_stats"),
  /** Số ms từ lần có phím hay chuột cuối cùng (ở bất kỳ app nào); `null` nếu không đọc được. */
  idleMs: () => invoke<number | null>("idle_ms"),
  /**
   * Mục Xem thử trong Cài đặt: pet gặp ngay thời tiết, nhiệt độ `preview` (overlay nhận qua
   * `onWeatherPreview`); cả hai `null` là thôi xem thử. Pet đang ẩn, tạm dừng thì lỗi.
   */
  previewWeather: (preview: WeatherPreview) => invoke<void>("preview_weather", { preview }),
  /** Mục Xem thử: con ma bay qua ngay (overlay nhận qua `onGhostPreview`). */
  previewGhost: () => invoke<void>("preview_ghost"),
  /** Overlay báo đang xem thử gì, Rust gửi sang Cài đặt (`onPreviewChanged`). */
  reportPreview: (state: PreviewState) => invoke<void>("report_preview", { state }),
  /** Cài đặt vừa mở: đang xem thử gì. */
  getPreview: () => invoke<PreviewState>("get_preview"),
  /**
   * Mở cửa sổ chat với `pet` cạnh pet (`box`: khung của pet, CSS pixel của overlay). Trả về phía của cửa sổ so
   * với pet: -1 bên trái, 1 bên phải.
   */
  openChat: (pet: string, name: string, box: Rect) => invoke<-1 | 1>("open_chat", { pet, name, ...box }),
  /** Câu gợi ý của hôm nay do Gemini viết (`today` "2026-10-01", `date` "Thứ Năm 01/10/2026"); rỗng là chưa có. */
  chatSuggestions: (today: string, date: string) => invoke<string[]>("chat_suggestions", { today, date }),
  /** Cửa sổ chat đang chat với con nào. */
  chatTarget: () => invoke<ChatTarget | null>("chat_target"),
  /** Gửi câu hỏi, chờ câu trả lời; gửi dồn dập thì lỗi `BUSY`, mất mạng `OFFLINE`, Google lỗi `UNAVAILABLE`. */
  sendChat: (prompt: string) => invoke<string>("send_chat", { prompt }),
  /** Mở link http/https bằng trình duyệt. */
  openLink: (url: string) => invoke<void>("open_link", { url }),

  /** Vị trí con trỏ và phím Ctrl, Rust chỉ gửi khi có thay đổi (xem `setCursorInterest`). */
  onCursorMoved: (callback: (cursor: CursorInfo) => void) =>
    listen<CursorInfo>(EVENTS.cursorMoved, (event) => callback(event.payload)),
  /** Overlay vừa sang màn hình khác, hoặc màn hình đổi độ phân giải, DPI, taskbar. */
  onScreenChanged: (callback: (change: ScreenChange) => void) =>
    listen<ScreenChange>(EVENTS.screenChanged, (event) => callback(event.payload)),
  /** Cửa sổ mở, đóng, di chuyển, đổi thứ tự chồng; Rust gửi tối đa 30 lần/giây. */
  onWindowsChanged: (callback: (windows: WindowList) => void) =>
    listen<WindowList>(EVENTS.windowsChanged, (event) => callback(event.payload)),
  /**
   * Overlay ẩn/hiện (tray, app fullscreen). Cửa sổ ẩn mà WebView2 vẫn chạy requestAnimationFrame
   * như thường, nên frontend phải tự dừng vòng lặp.
   */
  onVisibilityChanged: (callback: (visible: boolean) => void) =>
    listen<boolean>(EVENTS.overlayVisibility, (event) => callback(event.payload)),
  onPaused: (callback: (paused: boolean) => void) =>
    listen<boolean>(EVENTS.paused, (event) => callback(event.payload)),
  onSettingsChanged: (callback: (settings: Settings) => void) =>
    listen<Settings>(EVENTS.settingsChanged, (event) => callback(event.payload)),
  onQuitRequested: (callback: () => void) => listen(EVENTS.quitRequested, () => callback()),
  onWeatherChanged: (callback: (report: WeatherReport | null) => void) =>
    listen<WeatherReport | null>(EVENTS.weatherChanged, (event) => callback(event.payload)),
  onWeatherFailed: (callback: (failure: WeatherFailure) => void) =>
    listen<WeatherFailure>(EVENTS.weatherFailed, (event) => callback(event.payload)),
  /** Người dùng bắt đầu (`true`) hoặc thôi gõ phím: lúc gõ pet không nói câu cho vui. */
  onActivityChanged: (callback: (busy: boolean) => void) =>
    listen<boolean>(EVENTS.activityChanged, (event) => callback(event.payload)),
  onReminder: (callback: (reminder: Reminder) => void) =>
    listen<Reminder>(EVENTS.reminder, (event) => callback(event.payload)),
  onChatTarget: (callback: (target: ChatTarget) => void) =>
    listen<ChatTarget>(EVENTS.chatTarget, (event) => callback(event.payload)),
  /** Cửa sổ chat vừa đóng. */
  onChatClosed: (callback: () => void) => listen(EVENTS.chatClosed, () => callback()),
  /** Người dùng bấm xem thử thời tiết, nhiệt độ trong Cài đặt (cả hai `null` là thôi xem thử). */
  onWeatherPreview: (callback: (preview: WeatherPreview) => void) =>
    listen<WeatherPreview>(EVENTS.weatherPreview, (event) => callback(event.payload)),
  /** Người dùng bấm xem thử con ma trong Cài đặt. */
  onGhostPreview: (callback: () => void) => listen(EVENTS.ghostPreview, () => callback()),
  /** Overlay vừa đổi thứ đang xem thử (cửa sổ Cài đặt nghe). */
  onPreviewChanged: (callback: (state: PreviewState) => void) =>
    listen<PreviewState>(EVENTS.previewChanged, (event) => callback(event.payload)),
};
