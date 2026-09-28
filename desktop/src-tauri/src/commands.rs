//! Tauri commands gọi từ frontend qua `invoke()`.

use crate::autostart;
use crate::error::{AppError, AppResult};
use crate::events;
use crate::overlay::{self, Overlay, ScreenInfo};
use crate::settings::{Settings, SettingsStore};
use crate::storage::Storage;
use serde_json::Value;
use tauri::{AppHandle, Emitter, State, WebviewWindow};

#[tauri::command]
pub fn screen_info(overlay: State<'_, Overlay>) -> ScreenInfo {
    overlay.geometry().screen
}

/// `enabled = true`: chuột đi xuyên overlay xuống app bên dưới. Chỉ overlay được gọi.
#[tauri::command]
pub fn set_click_through(window: WebviewWindow, enabled: bool) -> AppResult<()> {
    if window.label() != overlay::LABEL {
        return Err(AppError::bad_request("Chỉ overlay được đổi click-through."));
    }
    window.set_ignore_cursor_events(enabled)?;
    Ok(())
}

/// Trạng thái thế giới pet đã lưu, `null` nếu chưa có.
#[tauri::command]
pub fn load_state(storage: State<'_, Storage>) -> AppResult<Option<Value>> {
    storage.load()
}

#[tauri::command]
pub fn save_state(storage: State<'_, Storage>, state: Value) -> AppResult<()> {
    storage.save(&state)
}

#[tauri::command]
pub fn get_settings(store: State<'_, SettingsStore>) -> Settings {
    store.get()
}

/// Lưu cài đặt rồi báo overlay áp dụng ngay. Trả về giá trị đã kẹp vào khoảng cho phép.
#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    settings: Settings,
) -> AppResult<Settings> {
    let settings = store.set(settings)?;
    app.emit_to(overlay::LABEL, events::SETTINGS_CHANGED, &settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn get_autostart() -> AppResult<bool> {
    autostart::is_enabled()
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> AppResult<()> {
    autostart::set_enabled(enabled)
}

/// Overlay gọi sau khi đã lưu trạng thái (xem `app::request_quit`).
#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}
