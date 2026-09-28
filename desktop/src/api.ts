import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { EVENTS, type CursorInfo, type ScreenInfo, type Settings } from "@tinyworld/core";

/** Wrapper cho các Tauri command trong src-tauri/src/commands.rs. */
export const api = {
  screenInfo: () => invoke<ScreenInfo>("screen_info"),
  /** `true`: chuột đi xuyên overlay xuống app bên dưới. */
  setClickThrough: (enabled: boolean) => invoke<void>("set_click_through", { enabled }),
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

  /** Vị trí con trỏ và phím Ctrl, Rust chỉ gửi khi có thay đổi. */
  onCursorMoved: (callback: (cursor: CursorInfo) => void) =>
    listen<CursorInfo>(EVENTS.cursorMoved, (event) => callback(event.payload)),
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
};
