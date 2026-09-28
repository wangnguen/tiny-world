/** Event Rust gửi cho overlay, tên phải khớp với desktop/src-tauri/src/cursor.rs. */
export const EVENTS = {
  /** Vị trí con trỏ (`Point`, CSS pixel của overlay), chỉ gửi khi con trỏ di chuyển. */
  cursorMoved: "cursor-moved",
} as const;
