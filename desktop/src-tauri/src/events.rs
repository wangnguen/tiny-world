//! Tên event Rust gửi cho frontend, khớp `EVENTS` trong packages/core/src/protocol.ts.

/// `CursorInfo`, chỉ gửi khi con trỏ di chuyển hoặc phím Ctrl đổi trạng thái.
pub const CURSOR_MOVED: &str = "cursor-moved";
/// `WindowList`: cửa sổ vừa mở, đóng, di chuyển, đổi thứ tự chồng, taskbar tự ẩn trồi lên/thụt
/// xuống; tối đa 30 lần/giây.
pub const WINDOWS_CHANGED: &str = "windows-changed";
/// `ScreenChange`: overlay vừa sang màn hình khác, hoặc màn hình đổi độ phân giải, DPI, taskbar.
pub const SCREEN_CHANGED: &str = "screen-changed";
/// `bool`: overlay vừa hiện hoặc ẩn (tray, app fullscreen).
pub const OVERLAY_VISIBILITY: &str = "overlay-visibility";
/// `bool`: tray bật/tắt Tạm dừng.
pub const PAUSED: &str = "pet-paused";
/// `Settings`: người dùng vừa đổi cài đặt.
pub const SETTINGS_CHANGED: &str = "settings-changed";
/// Không có payload: tray bấm Thoát, overlay lưu trạng thái rồi gọi command `quit`.
pub const QUIT_REQUESTED: &str = "quit-requested";
/// `Option<Report>` (weather.rs): vừa lấy được thời tiết mới, hoặc `null` khi bỏ thành phố.
pub const WEATHER_CHANGED: &str = "weather-changed";
/// `WeatherFailure` (weather.rs): không lấy được thời tiết, chỉ gửi ở lần lỗi đầu của mỗi đợt lỗi.
pub const WEATHER_FAILED: &str = "weather-failed";
/// `bool` (activity.rs): người dùng vừa bắt đầu (`true`) hoặc thôi gõ phím.
pub const ACTIVITY_CHANGED: &str = "activity-changed";
/// `Reminder` (activity.rs): pet nhắc nghỉ, nhắc khuya, kêu vì spam Ctrl+S.
pub const REMINDER: &str = "reminder";
/// `ChatTarget` (chat.rs), chỉ gửi cho cửa sổ chat: đang mở mà click chuột phải vào con khác thì chat với con đó.
pub const CHAT_TARGET: &str = "chat-target";
/// Không có payload (chat.rs), chỉ gửi cho overlay: cửa sổ chat vừa đóng, pet đang chat lại đi lại như thường.
pub const CHAT_CLOSED: &str = "chat-closed";
/// `WeatherPreview` (preview.rs), chỉ gửi cho overlay: người dùng bấm xem thử thời tiết, nhiệt độ trong
/// Cài đặt (cả hai `null` là thôi xem thử).
pub const WEATHER_PREVIEW: &str = "weather-preview";
/// Không có payload, chỉ gửi cho overlay: người dùng bấm xem thử con ma trong Cài đặt.
pub const GHOST_PREVIEW: &str = "ghost-preview";
/// `PreviewState` (preview.rs), chỉ gửi cho cửa sổ Cài đặt: overlay vừa đổi thứ đang xem thử.
pub const PREVIEW_CHANGED: &str = "preview-changed";
/// `UpdateInfo` (update.rs): GitHub có bản mới hơn bản đang chạy (Cài đặt hiện thẻ cập nhật, pet nói).
pub const UPDATE_AVAILABLE: &str = "update-available";
/// `UpdateProgress` (update.rs): đang tải bộ cài bản mới.
pub const UPDATE_PROGRESS: &str = "update-progress";
