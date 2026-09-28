import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { EVENTS, type Point, type ScreenInfo } from "@tinyworld/core";

/** Wrapper cho các Tauri command trong src-tauri/src/commands.rs. */
export const api = {
  screenInfo: () => invoke<ScreenInfo>("screen_info"),
  /** `true`: chuột đi xuyên overlay xuống app bên dưới. */
  setClickThrough: (enabled: boolean) => invoke<void>("set_click_through", { enabled }),
  /** Trạng thái thế giới pet đã lưu, `null` nếu chưa có. */
  loadState: <T>() => invoke<T | null>("load_state"),
  saveState: (state: unknown) => invoke<void>("save_state", { state }),
  /** Vị trí con trỏ theo CSS pixel của overlay, Rust chỉ gửi khi con trỏ di chuyển. */
  onCursorMoved: (callback: (position: Point) => void) =>
    listen<Point>(EVENTS.cursorMoved, (event) => callback(event.payload)),
};
