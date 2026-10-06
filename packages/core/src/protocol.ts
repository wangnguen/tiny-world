/** Event Rust gửi cho overlay, tên phải khớp với desktop/src-tauri/src/events.rs. */
export const EVENTS = {
  /** `CursorInfo`, chỉ gửi khi con trỏ di chuyển, phím Ctrl hoặc nút chuột đổi trạng thái. */
  cursorMoved: "cursor-moved",
  /** `WindowList`: cửa sổ vừa mở, đóng, di chuyển, đổi thứ tự chồng, taskbar tự ẩn trồi lên/thụt xuống; tối đa 30 lần/giây. */
  windowsChanged: "windows-changed",
  /** `ScreenChange`: overlay vừa sang màn hình khác, hoặc màn hình đổi độ phân giải, DPI, taskbar. */
  screenChanged: "screen-changed",
  /** `boolean`: overlay vừa hiện (`true`) hoặc ẩn (tray, có app fullscreen). */
  overlayVisibility: "overlay-visibility",
  /** `boolean`: tray bật/tắt Tạm dừng. */
  paused: "pet-paused",
  /** `Settings`: người dùng vừa đổi cài đặt. */
  settingsChanged: "settings-changed",
  /** Không có payload: tray bấm Thoát, overlay lưu trạng thái rồi gọi command `quit`. */
  quitRequested: "quit-requested",
  /** `WeatherReport | null`: vừa lấy được thời tiết mới, hoặc `null` khi đổi/bỏ thành phố. */
  weatherChanged: "weather-changed",
  /** `WeatherFailure`: không lấy được thời tiết, chỉ gửi ở lần lỗi đầu của mỗi đợt lỗi. */
  weatherFailed: "weather-failed",
  /** `boolean`: người dùng vừa bắt đầu (`true`) hoặc thôi gõ phím. */
  activityChanged: "activity-changed",
  /** `Reminder`: pet nhắc nghỉ, nhắc khuya, kêu vì spam Ctrl+S. */
  reminder: "reminder",
  /** `ChatTarget`, chỉ gửi cho cửa sổ chat: đang mở mà click chuột phải vào con khác. */
  chatTarget: "chat-target",
  /** Không có payload, chỉ gửi cho overlay: cửa sổ chat vừa đóng. */
  chatClosed: "chat-closed",
  /** `WeatherPreview`, chỉ gửi cho overlay: người dùng bấm xem thử thời tiết, nhiệt độ trong Cài đặt. */
  weatherPreview: "weather-preview",
  /** Không có payload, chỉ gửi cho overlay: người dùng bấm xem thử con ma trong Cài đặt. */
  ghostPreview: "ghost-preview",
  /** `PreviewState`, chỉ gửi cho cửa sổ Cài đặt: overlay vừa đổi thứ đang xem thử. */
  previewChanged: "preview-changed",
} as const;
