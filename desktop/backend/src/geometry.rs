//! Toạ độ của overlay: overlay trong suốt phủ vùng làm việc của một màn hình (không phủ kín cả màn hình: cửa
//! sổ luôn nằm trên mà che hết màn hình thì Windows coi là app fullscreen). Pet, cửa sổ, con trỏ đều tính theo
//! CSS pixel của overlay, gốc là góc trên trái vùng làm việc; Win32 dùng pixel vật lý, toạ độ desktop.
//!
//! `Overlay` giữ màn hình overlay đang phủ và các cờ ẩn/hiện, dùng chung giữa luồng chính, tray và luồng nền.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Hình chữ nhật theo CSS pixel, gốc là góc trên trái overlay. Khớp `Rect` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }
}

/// Khớp `ScreenInfo` trong packages/core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenInfo {
    pub scale_factor: f64,
    pub bounds: Rect,
    pub work_area: Rect,
    /// Vùng làm việc của các màn hình khác, theo toạ độ của overlay này.
    pub neighbors: Vec<Rect>,
}

/// Khớp `Remap` trong packages/core: toạ độ CSS pixel cũ `p` thành `p * scale + (x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Remap {
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}

/// Khớp `ScreenChange` trong packages/core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenChange {
    pub screen: ScreenInfo,
    pub remap: Remap,
}

/// Hình chữ nhật theo pixel vật lý, toạ độ desktop (Win32 dùng hệ này).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl PhysicalRect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let (left, top) = (f64::from(self.x), f64::from(self.y));
        let (right, bottom) = (left + f64::from(self.width), top + f64::from(self.height));
        x >= left && x < right && y >= top && y < bottom
    }

    pub fn center(&self) -> (f64, f64) {
        (
            f64::from(self.x) + f64::from(self.width) / 2.0,
            f64::from(self.y) + f64::from(self.height) / 2.0,
        )
    }
}

/// Một màn hình, toạ độ desktop theo pixel vật lý.
#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    /// Tên thiết bị (`\\.\DISPLAY1`), để nhận ra màn hình khi danh sách màn hình đổi.
    pub name: String,
    pub monitor: PhysicalRect,
    pub work_area: PhysicalRect,
    pub scale_factor: f64,
}

/// Màn hình và vùng overlay phủ.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// Vị trí, kích thước cửa sổ overlay (pixel vật lý).
    pub window: PhysicalRect,
    /// Tên màn hình overlay đang phủ.
    pub display: String,
    pub screen: ScreenInfo,
}

impl Geometry {
    /// Overlay phủ vùng làm việc của `display`; `all` là mọi màn hình (kể cả `display`).
    pub fn new(display: &Display, all: &[Display]) -> Self {
        let scale_factor = display.scale_factor;
        let mut window = display.work_area;
        // Taskbar tự ẩn thì vùng làm việc trùng cả màn hình: bớt 1 pixel để không bị coi là fullscreen.
        if window == display.monitor {
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
            display: display.name.clone(),
            screen: ScreenInfo {
                scale_factor,
                bounds: to_css(display.monitor),
                work_area: to_css(window),
                neighbors: all
                    .iter()
                    .filter(|d| d.monitor != display.monitor)
                    .map(|d| to_css(d.work_area))
                    .collect(),
            },
        }
    }

    /// Đổi hình chữ nhật toạ độ desktop (pixel vật lý) sang CSS pixel của overlay.
    pub fn rect_to_local(&self, r: PhysicalRect) -> Rect {
        let (x, y) = self.to_local(f64::from(r.x), f64::from(r.y));
        let scale = self.screen.scale_factor;
        Rect {
            x,
            y,
            width: f64::from(r.width) / scale,
            height: f64::from(r.height) / scale,
        }
    }

    /// Đổi toạ độ desktop (pixel vật lý) sang CSS pixel của overlay.
    pub fn to_local(&self, x: f64, y: f64) -> (f64, f64) {
        let scale = self.screen.scale_factor;
        (
            (x - f64::from(self.window.x)) / scale,
            (y - f64::from(self.window.y)) / scale,
        )
    }

    /// Đổi CSS pixel của overlay sang toạ độ desktop (pixel vật lý).
    pub fn to_physical(&self, x: f64, y: f64) -> (f64, f64) {
        let scale = self.screen.scale_factor;
        (
            f64::from(self.window.x) + x * scale,
            f64::from(self.window.y) + y * scale,
        )
    }

    /// Đổi toạ độ của overlay này sang toạ độ của overlay `next`, qua toạ độ desktop.
    pub fn remap_to(&self, next: &Geometry) -> Remap {
        let (from, to) = (self.screen.scale_factor, next.screen.scale_factor);
        Remap {
            scale: from / to,
            x: f64::from(self.window.x - next.window.x) / to,
            y: f64::from(self.window.y - next.window.y) / to,
        }
    }
}

/// Màn hình overlay đang phủ và các cờ ẩn/hiện, dùng chung giữa luồng chính, tray và các luồng nền.
pub struct Overlay {
    geometry: Mutex<Geometry>,
    /// Chỉ một nơi được đổi chỗ overlay tại một lúc (luồng đo màn hình, pet sang màn hình khác).
    moving: Mutex<()>,
    /// Người dùng ẩn pet từ tray.
    user_hidden: AtomicBool,
    /// Đang có app fullscreen (fullscreen.rs).
    auto_hidden: AtomicBool,
    /// Tray bật Tạm dừng: pet đứng yên, chuột đi xuyên qua pet.
    paused: AtomicBool,
    /// Pet đang ngủ, overlay dừng vòng lặp vẽ.
    resting: AtomicBool,
}

impl Overlay {
    pub fn new(geometry: Geometry) -> Self {
        Self {
            geometry: Mutex::new(geometry),
            moving: Mutex::new(()),
            user_hidden: AtomicBool::new(false),
            auto_hidden: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            resting: AtomicBool::new(false),
        }
    }

    pub fn geometry(&self) -> Geometry {
        lock(&self.geometry).clone()
    }

    pub fn set_geometry(&self, geometry: Geometry) {
        *lock(&self.geometry) = geometry;
    }

    /// Giữ quyền đổi chỗ overlay tới khi bỏ giá trị trả về. Đừng giữ trên luồng mà việc đổi chỗ phải chờ.
    pub fn moving(&self) -> MutexGuard<'_, ()> {
        lock(&self.moving)
    }

    /// Đổi toạ độ desktop (pixel vật lý) sang CSS pixel của overlay.
    pub fn to_local(&self, x: f64, y: f64) -> (f64, f64) {
        lock(&self.geometry).to_local(x, y)
    }

    pub fn is_visible(&self) -> bool {
        !self.user_hidden.load(Ordering::Relaxed) && !self.auto_hidden.load(Ordering::Relaxed)
    }

    /// Tray đang bật Tạm dừng.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Bật/tắt Tạm dừng, trả về trạng thái mới.
    pub fn toggle_paused(&self) -> bool {
        !self.paused.fetch_xor(true, Ordering::Relaxed)
    }

    pub fn is_user_hidden(&self) -> bool {
        self.user_hidden.load(Ordering::Relaxed)
    }

    pub fn set_user_hidden(&self, hidden: bool) {
        self.user_hidden.store(hidden, Ordering::Relaxed);
    }

    pub fn set_auto_hidden(&self, hidden: bool) {
        self.auto_hidden.store(hidden, Ordering::Relaxed);
    }

    /// Pet đang ngủ (trên taskbar), overlay dừng vòng lặp vẽ.
    pub fn is_resting(&self) -> bool {
        self.resting.load(Ordering::Relaxed)
    }

    /// Đặt cờ ngủ; `true` nếu cờ vừa đổi.
    pub fn set_resting(&self, resting: bool) -> bool {
        self.resting.swap(resting, Ordering::Relaxed) != resting
    }
}

/// Mutex bị poison (luồng khác panic lúc đang giữ) thì vẫn dùng tiếp: dữ liệu bên trong luôn đầy đủ.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display(
        name: &str,
        monitor: PhysicalRect,
        work_area: PhysicalRect,
        scale_factor: f64,
    ) -> Display {
        Display {
            name: name.into(),
            monitor,
            work_area,
            scale_factor,
        }
    }

    fn rect(x: i32, y: i32, width: u32, height: u32) -> PhysicalRect {
        PhysicalRect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn doi_sang_css_pixel_theo_dpi() {
        // Màn hình thứ hai 2560×1440 ở scale 150%, taskbar cao 72 px vật lý ở dưới.
        let monitor = rect(1920, 0, 2560, 1440);
        let work = PhysicalRect {
            height: 1368,
            ..monitor
        };
        let second = display("2", monitor, work, 1.5);
        let g = Geometry::new(&second, std::slice::from_ref(&second));
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
        assert_eq!(g.to_physical(200.0, 100.0), (1920.0 + 300.0, 150.0));
        assert!(g.screen.neighbors.is_empty());
    }

    #[test]
    fn taskbar_ben_trai_thi_goc_toa_do_la_mep_vung_lam_viec() {
        let monitor = rect(0, 0, 1920, 1080);
        let work = PhysicalRect {
            x: 62,
            width: 1858,
            ..monitor
        };
        let g = Geometry::new(&display("1", monitor, work, 1.0), &[]);
        assert_eq!(g.window, work);
        assert_eq!(g.screen.work_area.x, 0.0);
        assert_eq!(g.screen.work_area.width, 1858.0);
        assert_eq!(g.screen.bounds.x, -62.0);
        assert_eq!(g.to_local(62.0 + 100.0, 50.0), (100.0, 50.0));
    }

    #[test]
    fn taskbar_tu_an_thi_overlay_khong_phu_kin_man_hinh() {
        let monitor = rect(0, 0, 1920, 1080);
        let g = Geometry::new(&display("1", monitor, monitor, 1.0), &[]);
        assert_ne!(g.window, monitor);
        assert_eq!(g.window.height, 1079);
        assert_eq!(g.screen.work_area.height, 1079.0);
    }

    #[test]
    fn man_hinh_ben_canh_tinh_theo_toa_do_cua_overlay() {
        // Màn hình chính 1920×1080 ở 125%; màn hình phụ 2560×1440 ở 100% đặt bên phải, taskbar của nó cao 48 px.
        let main = display("1", rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1030), 1.25);
        let side = display("2", rect(1920, -200, 2560, 1440), rect(1920, -200, 2560, 1392), 1.0);
        let all = [main.clone(), side.clone()];
        let g = Geometry::new(&main, &all);
        assert_eq!(
            g.screen.neighbors,
            vec![Rect {
                x: 1536.0,
                y: -160.0,
                width: 2048.0,
                height: 1113.6
            }]
        );
        // Mép phải vùng làm việc (1920 / 1,25 = 1536 CSS pixel) giáp mép trái màn hình phụ.
        assert_eq!(g.screen.work_area.x + g.screen.work_area.width, g.screen.neighbors[0].x);
    }

    #[test]
    fn doi_toa_do_khi_sang_man_hinh_khac_dpi() {
        let main = display("1", rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1030), 1.0);
        let side = display("2", rect(1920, -200, 2560, 1440), rect(1920, -200, 2560, 1392), 1.5);
        let all = [main.clone(), side.clone()];
        let (from, to) = (Geometry::new(&main, &all), Geometry::new(&side, &all));
        let remap = from.remap_to(&to);
        // Điểm vừa qua mép phải màn hình chính: cùng một chỗ trên desktop, tính theo toạ độ overlay mới.
        let (x, y) = (1930.0, 1000.0);
        let (px, py) = from.to_physical(x, y);
        let (nx, ny) = to.to_local(px, py);
        assert!((x * remap.scale + remap.x - nx).abs() < 1e-9);
        assert!((y * remap.scale + remap.y - ny).abs() < 1e-9);
        assert!((nx - 10.0 / 1.5).abs() < 1e-9);
        // Đi rồi về thì trở lại đúng toạ độ cũ.
        let back = to.remap_to(&from);
        assert!(((nx * back.scale + back.x) - x).abs() < 1e-9);
    }

    #[test]
    fn co_an_hien_tam_dung_ngu() {
        let g = Geometry::new(&display("1", rect(0, 0, 1920, 1080), rect(0, 0, 1920, 1040), 1.0), &[]);
        let overlay = Overlay::new(g);
        assert!(overlay.is_visible());
        overlay.set_auto_hidden(true);
        assert!(!overlay.is_visible());
        overlay.set_auto_hidden(false);
        overlay.set_user_hidden(true);
        assert!(!overlay.is_visible() && overlay.is_user_hidden());
        assert!(overlay.toggle_paused());
        assert!(!overlay.toggle_paused());
        assert!(overlay.set_resting(true));
        assert!(!overlay.set_resting(true));
        assert!(overlay.is_resting());
    }
}
