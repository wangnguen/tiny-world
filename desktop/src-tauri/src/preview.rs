//! Command của mục Xem thử trong Cài đặt. Kiểu dữ liệu và phần kiểm tra ở `tinyworld_backend::preview`.

use crate::error::AppResult;
use crate::events;
use crate::overlay::{self, Overlay};
use crate::settings;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tinyworld_backend::preview::{ready, Preview, PreviewState, WeatherPreview};

/// Xem thử thời tiết, nhiệt độ trong `PREVIEW_SECONDS` giây; cả hai trống là thôi xem thử (lúc nào cũng
/// được).
#[tauri::command]
pub fn preview_weather(
    app: AppHandle,
    overlay: State<'_, Arc<Overlay>>,
    preview: WeatherPreview,
) -> AppResult<()> {
    preview.check()?;
    if !preview.is_stop() {
        ready(&overlay)?;
    }
    app.emit_to(overlay::LABEL, events::WEATHER_PREVIEW, preview)?;
    Ok(())
}

/// Cho con ma bay qua ngay.
#[tauri::command]
pub fn preview_ghost(app: AppHandle, overlay: State<'_, Arc<Overlay>>) -> AppResult<()> {
    ready(&overlay)?;
    app.emit_to(overlay::LABEL, events::GHOST_PREVIEW, ())?;
    Ok(())
}

/// Overlay báo đang xem thử gì: giữ lại và gửi sang Cài đặt.
#[tauri::command]
pub fn report_preview(app: AppHandle, preview: State<'_, Preview>, state: PreviewState) {
    preview.set(state);
    if let Err(e) = app.emit_to(settings::WINDOW_LABEL, events::PREVIEW_CHANGED, state) {
        log!("Không báo được trạng thái xem thử cho Cài đặt: {e}");
    }
}

/// Cài đặt vừa mở: đang xem thử gì.
#[tauri::command]
pub fn get_preview(preview: State<'_, Preview>) -> PreviewState {
    preview.get()
}
