//! Tên event Rust gửi cho frontend, khớp `EVENTS` trong packages/core/src/protocol.ts.

/// `CursorInfo`, chỉ gửi khi con trỏ di chuyển hoặc phím Ctrl đổi trạng thái.
pub const CURSOR_MOVED: &str = "cursor-moved";
/// `WindowList`: cửa sổ vừa mở, đóng, di chuyển, đổi thứ tự chồng; tối đa 30 lần/giây.
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
