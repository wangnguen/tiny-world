//! Mục Xem thử trong Cài đặt: cho pet gặp ngay một kiểu thời tiết, một mức nhiệt hay con ma để thử. Cài đặt
//! gửi lệnh qua đây sang overlay; overlay báo lại đang xem thử gì (`PreviewState`), Rust giữ bản mới nhất để
//! mở lại Cài đặt vẫn biết, và gửi sang Cài đặt mỗi lần đổi (nút đang bật sáng lên, đếm ngược).

use crate::error::{AppError, AppResult};
use crate::overlay::{self, Overlay};
use crate::{events, i18n};
use crate::settings;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, PoisonError};
use tauri::{AppHandle, Emitter, State};

/// Nhiệt độ xem thử được (°C).
const TEMPERATURE: (f64, f64) = (-60.0, 60.0);

/// Kiểu thời tiết quanh pet, khớp `Sky` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sky {
    Sunny,
    Clear,
    Cloudy,
    Fog,
    Rain,
    Snow,
    Storm,
    Petals,
}

/// Thời tiết xem thử, khớp `WeatherPreview` trong packages/core: `None` là theo trời thật, cả hai `None` là
/// thôi xem thử.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherPreview {
    pub sky: Option<Sky>,
    /// °C.
    pub temperature: Option<f64>,
}

impl WeatherPreview {
    fn is_stop(&self) -> bool {
        self.sky.is_none() && self.temperature.is_none()
    }

    fn check(&self) -> AppResult<()> {
        match self.temperature {
            Some(t) if !t.is_finite() || t < TEMPERATURE.0 || t > TEMPERATURE.1 => {
                Err(AppError::bad_request(i18n::t("errors.previewTemperature")))
            }
            _ => Ok(()),
        }
    }
}

/// Overlay đang xem thử gì, khớp `PreviewState` trong packages/core.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewState {
    pub sky: Option<Sky>,
    pub temperature: Option<f64>,
    /// Hết xem thử thời tiết, nhiệt độ lúc này (ms từ 1970); `None` là không xem thử.
    pub until: Option<f64>,
    /// Con ma đang bay.
    pub ghost: bool,
}

/// Trạng thái xem thử overlay báo gần nhất.
#[derive(Default)]
pub struct Preview(Mutex<PreviewState>);

impl Preview {
    fn get(&self) -> PreviewState {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set(&self, state: PreviewState) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = state;
    }
}

/// Pet đang ẩn hay tạm dừng thì overlay không vẽ, xem thử cũng không thấy gì: báo để người dùng biết.
fn ready(overlay: &Overlay) -> AppResult<()> {
    if !overlay.is_visible() {
        return Err(AppError::bad_request(i18n::t("errors.previewHidden")));
    }
    if overlay.is_paused() {
        return Err(AppError::bad_request(i18n::t("errors.previewPaused")));
    }
    Ok(())
}

/// Xem thử thời tiết, nhiệt độ trong `PREVIEW_SECONDS` giây; cả hai trống là thôi xem thử (lúc nào cũng
/// được).
#[tauri::command]
pub fn preview_weather(
    app: AppHandle,
    overlay: State<'_, Overlay>,
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
pub fn preview_ghost(app: AppHandle, overlay: State<'_, Overlay>) -> AppResult<()> {
    ready(&overlay)?;
    app.emit_to(overlay::LABEL, events::GHOST_PREVIEW, ())?;
    Ok(())
}

/// Overlay báo đang xem thử gì: giữ lại và gửi sang Cài đặt.
#[tauri::command]
pub fn report_preview(app: AppHandle, preview: State<'_, Preview>, state: PreviewState) {
    preview.set(state);
    if let Err(e) = app.emit_to(settings::WINDOW_LABEL, events::PREVIEW_CHANGED, state) {
        eprintln!("Không báo được trạng thái xem thử cho Cài đặt: {e}");
    }
}

/// Cài đặt vừa mở: đang xem thử gì.
#[tauri::command]
pub fn get_preview(preview: State<'_, Preview>) -> PreviewState {
    preview.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_lenh_xem_thu_tu_cai_dat() {
        let preview: WeatherPreview =
            serde_json::from_str(r#"{"sky":"sunny","temperature":36}"#).unwrap();
        assert_eq!(preview.sky, Some(Sky::Sunny));
        assert!(preview.check().is_ok() && !preview.is_stop());
        let stop: WeatherPreview =
            serde_json::from_str(r#"{"sky":null,"temperature":null}"#).unwrap();
        assert!(stop.is_stop());
        // Thời tiết lạ, nhiệt độ vô lý thì từ chối.
        let tornado = r#"{"sky":"tornado","temperature":null}"#;
        assert!(serde_json::from_str::<WeatherPreview>(tornado).is_err());
        let hot: WeatherPreview =
            serde_json::from_str(r#"{"sky":null,"temperature":500}"#).unwrap();
        assert!(hot.check().is_err());
    }

    #[test]
    fn trang_thai_gui_sang_cai_dat_dung_ten_truong() {
        let state = PreviewState {
            sky: Some(Sky::Storm),
            temperature: Some(8.0),
            until: Some(1_000.0),
            ghost: true,
        };
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(
            json,
            r#"{"sky":"storm","temperature":8.0,"until":1000.0,"ghost":true}"#
        );
        let preview = Preview::default();
        assert_eq!(preview.get(), PreviewState::default());
        preview.set(state);
        assert_eq!(preview.get(), state);
    }
}
