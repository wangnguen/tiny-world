//! Mục Xem thử trong Cài đặt: cho pet gặp ngay một kiểu thời tiết, một mức nhiệt hay con ma để thử. Cài đặt
//! gửi lệnh qua đây sang overlay; overlay báo lại đang xem thử gì (`PreviewState`), Rust giữ bản mới nhất để
//! mở lại Cài đặt vẫn biết, và gửi sang Cài đặt mỗi lần đổi (nút đang bật sáng lên, đếm ngược).

use crate::error::{AppError, AppResult};
use crate::geometry::Overlay;
use crate::i18n;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, PoisonError};

/// Xem thử kéo dài chừng này giây, khớp `PREVIEW_SECONDS` trong packages/core.
pub const PREVIEW_SECONDS: u32 = 30;

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

impl Sky {
    /// Tên như trong packages/core ("rain"), cũng là tên mục trong file chữ.
    pub fn key(self) -> &'static str {
        match self {
            Self::Sunny => "sunny",
            Self::Clear => "clear",
            Self::Cloudy => "cloudy",
            Self::Fog => "fog",
            Self::Rain => "rain",
            Self::Snow => "snow",
            Self::Storm => "storm",
            Self::Petals => "petals",
        }
    }
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
    /// Cả hai trống: thôi xem thử.
    pub fn is_stop(&self) -> bool {
        self.sky.is_none() && self.temperature.is_none()
    }

    /// Nhiệt độ trong khoảng xem thử được.
    pub fn check(&self) -> AppResult<()> {
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
    pub fn get(&self) -> PreviewState {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set(&self, state: PreviewState) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = state;
    }
}

/// Pet đang ẩn hay tạm dừng thì overlay không vẽ, xem thử cũng không thấy gì: báo để người dùng biết.
pub fn ready(overlay: &Overlay) -> AppResult<()> {
    if !overlay.is_visible() {
        return Err(AppError::bad_request(i18n::t("errors.previewHidden")));
    }
    if overlay.is_paused() {
        return Err(AppError::bad_request(i18n::t("errors.previewPaused")));
    }
    Ok(())
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
