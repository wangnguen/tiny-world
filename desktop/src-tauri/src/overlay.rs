//! Cửa sổ overlay trong suốt phủ vùng làm việc của một màn hình, nơi vẽ pet. Luôn nằm trên cùng,
//! không có nút trên taskbar, mặc định để chuột đi xuyên qua (frontend tắt khi con trỏ nằm trên pet).
//!
//! Overlay không được phủ kín cả màn hình: cửa sổ luôn nằm trên mà che hết màn hình thì Windows coi
//! là app fullscreen (tắt thông báo, fullscreen.rs tưởng đang xem video nên ẩn pet rồi lại hiện).
//!
//! Mở app thì overlay nằm trên màn hình chính. Pet bị kéo, bị ném hoặc tự đi sang màn hình khác thì
//! overlay sang theo (`move_to`). Cứ mỗi giây đo lại màn hình (`watch`): đổi độ phân giải, DPI, taskbar,
//! rút màn hình ra thì đặt lại overlay cho khớp.

use crate::error::{AppError, AppResult};
use crate::{events, window_list};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const LABEL: &str = "overlay";

/// Đo lại màn hình chừng này một lần.
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

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
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenInfo {
    pub scale_factor: f64,
    pub bounds: Rect,
    pub work_area: Rect,
    /// Vùng làm việc của các màn hình khác, theo toạ độ của overlay này.
    pub neighbors: Vec<Rect>,
}

/// Khớp `Remap` trong packages/core: toạ độ CSS pixel cũ `p` thành `p * scale + (x, y)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Remap {
    pub scale: f64,
    pub x: f64,
    pub y: f64,
}

/// Khớp `ScreenChange` trong packages/core.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    fn contains(&self, x: f64, y: f64) -> bool {
        let (left, top) = (f64::from(self.x), f64::from(self.y));
        let (right, bottom) = (left + f64::from(self.width), top + f64::from(self.height));
        x >= left && x < right && y >= top && y < bottom
    }

    fn center(&self) -> (f64, f64) {
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

impl Display {
    fn from_monitor(monitor: &Monitor) -> Self {
        let (position, size, work) = (monitor.position(), monitor.size(), monitor.work_area());
        Self {
            name: monitor.name().cloned().unwrap_or_default(),
            monitor: PhysicalRect {
                x: position.x,
                y: position.y,
                width: size.width,
                height: size.height,
            },
            work_area: PhysicalRect {
                x: work.position.x,
                y: work.position.y,
                width: work.size.width,
                height: work.size.height,
            },
            scale_factor: monitor.scale_factor(),
        }
    }
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

/// State dùng chung giữa command, tray và các luồng nền.
pub struct Overlay {
    geometry: Mutex<Geometry>,
    /// Chỉ một nơi được đổi chỗ overlay tại một lúc (luồng đo màn hình, command `move_overlay`). Không
    /// bao giờ giữ trên main thread: đổi chỗ cửa sổ phải chờ main thread.
    moving: Mutex<()>,
    /// Người dùng ẩn pet từ tray.
    user_hidden: AtomicBool,
    /// Đang có app fullscreen (fullscreen.rs).
    auto_hidden: AtomicBool,
    /// Tray bật Tạm dừng: pet đứng yên, chuột đi xuyên qua pet.
    paused: AtomicBool,
    /// Pet đang ngủ, overlay dừng vòng lặp vẽ (command `set_resting`).
    resting: AtomicBool,
}

impl Overlay {
    pub fn geometry(&self) -> Geometry {
        lock(&self.geometry).clone()
    }

    /// Đổi toạ độ desktop (pixel vật lý) sang CSS pixel của overlay.
    pub fn to_local(&self, x: f64, y: f64) -> (f64, f64) {
        lock(&self.geometry).to_local(x, y)
    }

    pub fn is_visible(&self) -> bool {
        !self.user_hidden.load(Ordering::Relaxed) && !self.auto_hidden.load(Ordering::Relaxed)
    }

    /// Pet đang ngủ (trên taskbar), overlay dừng vòng lặp vẽ.
    pub fn is_resting(&self) -> bool {
        self.resting.load(Ordering::Relaxed)
    }
}

/// Mutex bị poison (luồng khác panic lúc đang giữ) thì vẫn dùng tiếp: dữ liệu bên trong luôn đầy đủ.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn displays(app: &AppHandle) -> AppResult<Vec<Display>> {
    Ok(app.available_monitors()?.iter().map(Display::from_monitor).collect())
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
    let primary = Display::from_monitor(&monitor);
    let mut all = displays(app)?;
    if !all.contains(&primary) {
        all.push(primary.clone());
    }
    let geometry = Geometry::new(&primary, &all);
    place(&window, geometry.window)?;
    window.set_ignore_cursor_events(true)?;
    window.show()?;
    app.manage(Overlay {
        geometry: Mutex::new(geometry),
        moving: Mutex::new(()),
        user_hidden: AtomicBool::new(false),
        auto_hidden: AtomicBool::new(false),
        paused: AtomicBool::new(false),
        resting: AtomicBool::new(false),
    });
    Ok(())
}

/// Đo lại màn hình mỗi giây, có gì đổi thì đặt lại overlay (`refit`).
pub fn watch(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(WATCH_INTERVAL);
        if let Err(e) = refit(&app) {
            eprintln!("Không đặt lại được overlay theo màn hình: {e}");
        }
    });
}

/// Màn hình đổi độ phân giải, DPI, taskbar, hoặc màn hình khác được cắm/rút: đặt lại overlay trên màn
/// hình đang ở. Màn hình đó bị rút ra thì sang màn hình chính.
fn refit(app: &AppHandle) -> AppResult<()> {
    let overlay = app.state::<Overlay>();
    let _moving = lock(&overlay.moving);
    let all = displays(app)?;
    let current = overlay.geometry();
    let (cx, cy) = current.window.center();
    let target = all
        .iter()
        .find(|d| !d.name.is_empty() && d.name == current.display)
        .or_else(|| all.iter().find(|d| d.monitor.contains(cx, cy)));
    let primary;
    let target = match target {
        Some(target) => target,
        None => match app.primary_monitor()? {
            Some(monitor) => {
                primary = Display::from_monitor(&monitor);
                &primary
            }
            None => return Ok(()),
        },
    };
    let next = Geometry::new(target, &all);
    if next == current {
        return Ok(());
    }
    apply(app, &overlay, &current, next)
}

/// Pet bị kéo hoặc đi ra khỏi overlay tới điểm (`x`, `y`) (CSS pixel của overlay): điểm đó nằm trên màn
/// hình khác thì overlay sang màn hình đó. Không thuộc màn hình nào, hoặc vẫn là màn hình này, thì thôi.
pub fn move_to(app: &AppHandle, x: f64, y: f64) -> AppResult<()> {
    let overlay = app.state::<Overlay>();
    let _moving = lock(&overlay.moving);
    let current = overlay.geometry();
    let (px, py) = current.to_physical(x, y);
    let all = displays(app)?;
    let Some(target) = all.iter().find(|d| d.monitor.contains(px, py)) else {
        return Ok(());
    };
    let (cx, cy) = current.window.center();
    if target.monitor.contains(cx, cy) {
        return Ok(());
    }
    apply(app, &overlay, &current, Geometry::new(target, &all))
}

/// Đổi chỗ overlay, rồi báo frontend (kèm cách đổi toạ độ cũ sang mới) và đọc lại danh sách cửa sổ theo
/// toạ độ mới.
fn apply(app: &AppHandle, overlay: &Overlay, current: &Geometry, next: Geometry) -> AppResult<()> {
    let window = app.get_webview_window(LABEL).ok_or_else(AppError::no_window)?;
    if next.window != current.window {
        place(&window, next.window)?;
    }
    let change = ScreenChange {
        remap: current.remap_to(&next),
        screen: next.screen.clone(),
    };
    *lock(&overlay.geometry) = next;
    window_list::refresh();
    app.emit_to(LABEL, events::SCREEN_CHANGED, change)?;
    Ok(())
}

/// Đặt cửa sổ đúng chỗ, đúng cỡ. Sang màn hình khác DPI thì Windows tự co giãn cửa sổ theo DPI mới ngay
/// lúc đổi chỗ, nên đặt lại cỡ sau đó, lệch thì đặt lần nữa.
fn place(window: &WebviewWindow, r: PhysicalRect) -> AppResult<()> {
    let position = PhysicalPosition::new(r.x, r.y);
    let size = PhysicalSize::new(r.width, r.height);
    for _ in 0..2 {
        window.set_position(position)?;
        window.set_size(size)?;
        if window.outer_position()? == position && window.outer_size()? == size {
            break;
        }
    }
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
    // Lúc ẩn luồng theo dõi cửa sổ gỡ hook; hiện lại thì đánh thức để nó gắn lại và đọc ngay.
    if visible {
        window_list::refresh();
    }
    apply_memory_level(&window, overlay);
}

/// Pet ngủ (frontend dừng vòng lặp vẽ) hoặc thức dậy.
pub fn set_resting(app: &AppHandle, resting: bool) {
    let Some(overlay) = app.try_state::<Overlay>() else {
        return;
    };
    if overlay.resting.swap(resting, Ordering::Relaxed) == resting {
        return;
    }
    // Luồng theo dõi cửa sổ đổi cách theo dõi (pet ngủ thì chỉ cần taskbar); thức dậy thì đọc lại ngay.
    window_list::refresh();
    if let Some(window) = app.get_webview_window(LABEL) {
        apply_memory_level(&window, &overlay);
    }
}

/// Pet ngủ hoặc overlay ẩn thì không có gì chuyển động: bảo WebView2 dùng ít RAM (bỏ bớt cache, đẩy
/// phần không dùng ra khỏi RAM). Pet thức dậy thì trả về bình thường.
fn apply_memory_level(window: &WebviewWindow, overlay: &Overlay) {
    let low = overlay.resting.load(Ordering::Relaxed) || !overlay.is_visible();
    if let Err(e) = memory::set_low(window, low) {
        eprintln!("Không đổi được mức dùng RAM của WebView2: {e}");
    }
}

#[cfg(windows)]
mod memory {
    use tauri::WebviewWindow;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
    };
    use windows_core::Interface;

    pub fn set_low(window: &WebviewWindow, low: bool) -> tauri::Result<()> {
        window.with_webview(move |webview| {
            let level = if low {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
            } else {
                COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
            };
            // SAFETY: `with_webview` chạy trên main thread, nơi controller WebView2 được tạo và dùng.
            // WebView2 Runtime cũ (trước 1.0.2210) không có ICoreWebView2_19 thì thôi.
            let result = unsafe {
                webview
                    .controller()
                    .CoreWebView2()
                    .and_then(|core| core.cast::<ICoreWebView2_19>())
                    .and_then(|core| core.SetMemoryUsageTargetLevel(level))
            };
            if let Err(e) = result {
                eprintln!("WebView2 không hỗ trợ đổi mức dùng RAM: {e}");
            }
        })
    }
}

#[cfg(not(windows))]
mod memory {
    pub fn set_low(_window: &tauri::WebviewWindow, _low: bool) -> tauri::Result<()> {
        Ok(())
    }
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
        let g = Geometry::new(&second, &[second.clone()]);
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
}
