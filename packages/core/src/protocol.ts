/** Event Rust gửi cho overlay, tên phải khớp với desktop/src-tauri/src/events.rs. */
export const EVENTS = {
  /** `CursorInfo`, chỉ gửi khi con trỏ di chuyển, phím Ctrl hoặc nút chuột đổi trạng thái. */
  cursorMoved: "cursor-moved",
  /** `boolean`: overlay vừa hiện (`true`) hoặc ẩn (tray, có app fullscreen). */
  overlayVisibility: "overlay-visibility",
  /** `boolean`: tray bật/tắt Tạm dừng. */
  paused: "pet-paused",
  /** `Settings`: người dùng vừa đổi cài đặt. */
  settingsChanged: "settings-changed",
  /** Không có payload: tray bấm Thoát, overlay lưu trạng thái rồi gọi command `quit`. */
  quitRequested: "quit-requested",
} as const;
