//! Cửa sổ overlay trong suốt phủ vùng làm việc của màn hình chính, nơi vẽ pet. Luôn nằm trên cùng,
//! không có nút trên taskbar, mặc định để chuột đi xuyên qua (frontend tắt khi con trỏ nằm trên pet).
//!
//! Overlay không được phủ kín cả màn hình: cửa sổ luôn nằm trên mà che hết màn hình thì Windows coi
//! là app fullscreen (tắt thông báo, fullscreen.rs tưởng đang xem video nên ẩn pet rồi lại hiện).

use crate::error::{AppError, AppResult};
use crate::events;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize};

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

/// Màn hình và vùng overlay phủ.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    /// Vị trí, kích thước cửa sổ overlay (pixel vật lý).
    pub window: PhysicalRect,
    pub screen: ScreenInfo,
}

impl Geometry {
    pub fn new(monitor: PhysicalRect, work_area: PhysicalRect, scale_factor: f64) -> Self {
        let mut window = work_area;
        // Taskbar tự ẩn thì vùng làm việc trùng cả màn hình: bớt 1 pixel để không bị coi là fullscreen.
        if window == monitor {
            window.height = window.height.saturating_sub(1);
        }
        let to_css = |r: PhysicalRect| Rect {
            x: f64::from(r.x - window.x) / scale_factor,
            y: f64::from(r.y - window.y) / scale_factor,
            width: f64::from(r.width) / scale_factor,
            height: f64::from(r.height) / scale_factor,
        };
        Self {
            window,
            screen: ScreenInfo {
                scale_factor,
                bounds: to_css(monitor),
                work_area: to_css(window),
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
            (x - f64::from(self.window.x)) / scale,
            (y - f64::from(self.window.y)) / scale,
        )
    }
}

/// State dùng chung giữa command, tray và các luồng nền.
pub struct Overlay {
    geometry: Geometry,
    /// Người dùng ẩn pet từ tray.
    user_hidden: AtomicBool,
    /// Đang có app fullscreen (fullscreen.rs).
    auto_hidden: AtomicBool,
    /// Tray bật Tạm dừng: pet đứng yên, chuột đi xuyên qua pet.
    paused: AtomicBool,
}

impl Overlay {
    pub fn geometry(&self) -> Geometry {
        self.geometry
    }

    pub fn is_visible(&self) -> bool {
        !self.user_hidden.load(Ordering::Relaxed) && !self.auto_hidden.load(Ordering::Relaxed)
    }
}

/// Đặt overlay phủ vùng làm việc của màn hình chính, bật click-through rồi mới hiện (cửa sổ tạo sẵn
/// ở dạng ẩn).
pub fn setup(app: &AppHandle) -> AppResult<()> {
    let window = app.get_webview_window(LABEL).ok_or_else(AppError::no_window)?;
    let monitor = match app.primary_monitor()? {
        Some(monitor) => monitor,
        None => window
            .current_monitor()?
            .ok_or_else(|| AppError::internal("Không tìm thấy màn hình."))?,
    };
    let geometry = Geometry::from_monitor(&monitor);
    let w = geometry.window;
    window.set_position(PhysicalPosition::new(w.x, w.y))?;
    window.set_size(PhysicalSize::new(w.width, w.height))?;
    window.set_ignore_cursor_events(true)?;
    window.show()?;
    app.manage(Overlay {
        geometry,
        user_hidden: AtomicBool::new(false),
        auto_hidden: AtomicBool::new(false),
        paused: AtomicBool::new(false),
    });
    Ok(())
}

/// Bật/tắt Tạm dừng, trả về trạng thái mới.
pub fn toggle_paused(app: &AppHandle) -> bool {
    let Some(overlay) = app.try_state::<Overlay>() else {
        return false;
    };
    let paused = !overlay.paused.fetch_xor(true, Ordering::Relaxed);
    if let Err(e) = app.emit_to(LABEL, events::PAUSED, paused) {
        eprintln!("Không báo được trạng thái tạm dừng cho overlay: {e}");
    }
    paused
}

/// Ẩn/hiện theo lựa chọn của người dùng (tray, mở app lần nữa).
pub fn set_user_hidden(app: &AppHandle, hidden: bool) {
    if let Some(overlay) = app.try_state::<Overlay>() {
        overlay.user_hidden.store(hidden, Ordering::Relaxed);
        apply_visibility(app, &overlay);
    }
}

pub fn toggle(app: &AppHandle) {
    if let Some(overlay) = app.try_state::<Overlay>() {
        let hidden = overlay.user_hidden.load(Ordering::Relaxed);
        set_user_hidden(app, !hidden);
    }
}

/// Ẩn khi có app fullscreen, hiện lại khi hết (trừ khi người dùng đang tự ẩn).
pub fn set_auto_hidden(app: &AppHandle, hidden: bool) {
    if let Some(overlay) = app.try_state::<Overlay>() {
        overlay.auto_hidden.store(hidden, Ordering::Relaxed);
        apply_visibility(app, &overlay);
    }
}

fn apply_visibility(app: &AppHandle, overlay: &Overlay) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let visible = overlay.is_visible();
    let result = if visible { window.show() } else { window.hide() };
    if let Err(e) = result {
        eprintln!("Không đổi được trạng thái ẩn/hiện của overlay: {e}");
    }
    // Cửa sổ ẩn nhưng trang vẫn chạy như đang hiện, báo để frontend dừng vòng lặp vẽ.
    if let Err(e) = app.emit_to(LABEL, events::OVERLAY_VISIBILITY, visible) {
        eprintln!("Không báo được trạng thái ẩn/hiện cho overlay: {e}");
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
        assert_eq!(g.window, work);
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
    fn taskbar_ben_trai_thi_goc_toa_do_la_mep_vung_lam_viec() {
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
        assert_eq!(g.window, work);
        assert_eq!(g.screen.work_area.x, 0.0);
        assert_eq!(g.screen.work_area.width, 1858.0);
        assert_eq!(g.screen.bounds.x, -62.0);
        assert_eq!(g.to_local(62.0 + 100.0, 50.0), (100.0, 50.0));
    }

    #[test]
    fn taskbar_tu_an_thi_overlay_khong_phu_kin_man_hinh() {
        let monitor = PhysicalRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let g = Geometry::new(monitor, monitor, 1.0);
        assert_ne!(g.window, monitor);
        assert_eq!(g.window.height, 1079);
        assert_eq!(g.screen.work_area.height, 1079.0);
    }
}
