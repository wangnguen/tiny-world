import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  EVENTS,
  type CursorInfo,
  type Rect,
  type ScreenChange,
  type ScreenInfo,
  type Settings,
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
   * Vùng quanh pet (CSS pixel của overlay) cần biết vị trí con trỏ; ngoài vùng này Rust chỉ báo lúc
   * bấm/nhả chuột. `null`: mọi chỗ.
   */
  setCursorInterest: (rect: Rect | null) => invoke<void>("set_cursor_interest", { rect }),
  /** Pet ngủ, vòng lặp vẽ dừng (`true`), hoặc thức dậy: Rust bảo WebView2 dùng ít RAM lúc ngủ. */
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
};
