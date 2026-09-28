//! Tauri commands gọi từ frontend qua `invoke()`.

use crate::error::{AppError, AppResult};
use crate::overlay::{self, Overlay, ScreenInfo};
use crate::storage::Storage;
use serde_json::Value;
use tauri::{State, WebviewWindow};

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
