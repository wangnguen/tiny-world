//! Cửa sổ overlay trong suốt phủ màn hình chính, nơi vẽ pet. Luôn nằm trên cùng, không có nút
//! trên taskbar, mặc định để chuột đi xuyên qua (frontend tắt khi con trỏ nằm trên pet).

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize};

pub const LABEL: &str = "overlay";

/// Hình chữ nhật theo CSS pixel, gốc là góc trên trái overlay. Khớp `Rect` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Khớp `ScreenInfo` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenInfo {
    pub scale_factor: f64,
    pub bounds: Rect,
    pub work_area: Rect,
}

/// Hình chữ nhật theo pixel vật lý, toạ độ desktop (Win32 dùng hệ này).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Màn hình overlay đang phủ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub monitor: PhysicalRect,
    pub screen: ScreenInfo,
}

impl Geometry {
    pub fn new(monitor: PhysicalRect, work_area: PhysicalRect, scale_factor: f64) -> Self {
        let to_css = |r: PhysicalRect| Rect {
            x: f64::from(r.x - monitor.x) / scale_factor,
            y: f64::from(r.y - monitor.y) / scale_factor,
            width: f64::from(r.width) / scale_factor,
            height: f64::from(r.height) / scale_factor,
        };
        Self {
            monitor,
            screen: ScreenInfo {
                scale_factor,
                bounds: to_css(monitor),
                work_area: to_css(work_area),
            },
        }
    }

    fn from_monitor(monitor: &Monitor) -> Self {
        let (position, size, work) = (monitor.position(), monitor.size(), monitor.work_area());
        Self::new(
            PhysicalRect {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            },
            PhysicalRect {
                x: work.position.x,
                y: work.position.y,
                width: work.size.width,
                height: work.size.height,
            },
            monitor.scale_factor(),
        )
    }

    /// Đổi toạ độ desktop (pixel vật lý) sang CSS pixel của overlay.
    pub fn to_local(&self, x: f64, y: f64) -> (f64, f64) {
        let scale = self.screen.scale_factor;
        (
            (x - f64::from(self.monitor.x)) / scale,
            (y - f64::from(self.monitor.y)) / scale,
        )
    }
}

/// State dùng chung giữa command, tray và luồng đọc con trỏ.
pub struct Overlay {
    geometry: Geometry,
    visible: AtomicBool,
}

impl Overlay {
    pub fn geometry(&self) -> Geometry {
        self.geometry
    }

    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::Relaxed)
    }
}

/// Đặt overlay phủ màn hình chính, bật click-through rồi mới hiện (cửa sổ tạo sẵn ở dạng ẩn).
pub fn setup(app: &AppHandle) -> AppResult<()> {
    let window = app.get_webview_window(LABEL).ok_or_else(AppError::no_window)?;
    let monitor = match app.primary_monitor()? {
        Some(monitor) => monitor,
        None => window
            .current_monitor()?
            .ok_or_else(|| AppError::internal("Không tìm thấy màn hình."))?,
    };
    let geometry = Geometry::from_monitor(&monitor);
    let m = geometry.monitor;
    window.set_position(PhysicalPosition::new(m.x, m.y))?;
    window.set_size(PhysicalSize::new(m.width, m.height))?;
    window.set_ignore_cursor_events(true)?;
    window.show()?;
    app.manage(Overlay {
        geometry,
        visible: AtomicBool::new(true),
    });
    Ok(())
}

pub fn set_visible(app: &AppHandle, visible: bool) {
    let (Some(window), Some(overlay)) = (app.get_webview_window(LABEL), app.try_state::<Overlay>())
    else {
        return;
    };
    let result = if visible { window.show() } else { window.hide() };
    match result {
        Ok(()) => overlay.visible.store(visible, Ordering::Relaxed),
        Err(e) => eprintln!("Không đổi được trạng thái ẩn/hiện của overlay: {e}"),
    }
}

pub fn toggle(app: &AppHandle) {
    if let Some(overlay) = app.try_state::<Overlay>() {
        set_visible(app, !overlay.is_visible());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doi_sang_css_pixel_theo_dpi() {
        // Màn hình thứ hai 2560×1440 ở scale 150%, taskbar cao 72 px vật lý ở dưới.
        let monitor = PhysicalRect {
            x: 1920,
            y: 0,
            width: 2560,
            height: 1440,
        };
        let work = PhysicalRect {
            height: 1368,
            ..monitor
        };
        let g = Geometry::new(monitor, work, 1.5);
        assert_eq!(
            g.screen.bounds,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 2560.0 / 1.5,
                height: 960.0
            }
        );
        assert_eq!(g.screen.work_area.height, 912.0);
        assert_eq!(g.to_local(1920.0 + 300.0, 150.0), (200.0, 100.0));
    }

    #[test]
    fn taskbar_ben_trai_day_vung_lam_viec_sang_phai() {
        let monitor = PhysicalRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let work = PhysicalRect {
            x: 62,
            width: 1858,
            ..monitor
        };
        let g = Geometry::new(monitor, work, 1.0);
        assert_eq!(g.screen.work_area.x, 62.0);
        assert_eq!(g.screen.work_area.width, 1858.0);
    }
}
