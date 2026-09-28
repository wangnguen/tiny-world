//! Tên event Rust gửi cho frontend, khớp `EVENTS` trong packages/core/src/protocol.ts.

/// `CursorInfo`, chỉ gửi khi con trỏ di chuyển hoặc phím Ctrl đổi trạng thái.
pub const CURSOR_MOVED: &str = "cursor-moved";
/// `bool`: overlay vừa hiện hoặc ẩn (tray, app fullscreen).
pub const OVERLAY_VISIBILITY: &str = "overlay-visibility";
/// `bool`: tray bật/tắt Tạm dừng.
pub const PAUSED: &str = "pet-paused";
/// `Settings`: người dùng vừa đổi cài đặt.
pub const SETTINGS_CHANGED: &str = "settings-changed";
/// Không có payload: tray bấm Thoát, overlay lưu trạng thái rồi gọi command `quit`.
pub const QUIT_REQUESTED: &str = "quit-requested";
