//! Cửa sổ overlay trong suốt phủ vùng làm việc của một màn hình, nơi vẽ pet. Luôn nằm trên cùng,
//! không có nút trên taskbar, mặc định để chuột đi xuyên qua (frontend tắt khi con trỏ nằm trên pet).
//! Toạ độ và trạng thái ẩn/hiện ở `tinyworld_backend::geometry`.
//!
//! Mở app thì overlay nằm trên màn hình chính. Pet bị kéo, bị ném hoặc tự đi sang màn hình khác thì
//! overlay sang theo (`move_to`). Cứ mỗi giây đo lại màn hình (`watch`): đổi độ phân giải, DPI, taskbar,
//! rút màn hình ra thì đặt lại overlay cho khớp.

pub use crate::geometry::{Display, Geometry, Overlay, PhysicalRect, Rect, ScreenChange, ScreenInfo};

use crate::error::{AppError, AppResult};
use crate::{events, window_list};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

pub const LABEL: &str = "overlay";

/// Đo lại màn hình chừng này một lần.
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

fn display_of(monitor: &Monitor) -> Display {
    let (position, size, work) = (monitor.position(), monitor.size(), monitor.work_area());
    Display {
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

fn displays(app: &AppHandle) -> AppResult<Vec<Display>> {
    Ok(app.available_monitors()?.iter().map(display_of).collect())
}

/// HWND của overlay (0 nếu chưa có), để luồng theo dõi cửa sổ bỏ qua nó.
pub fn hwnd(app: &AppHandle) -> isize {
    app.get_webview_window(LABEL)
        .and_then(|window| window.hwnd().ok())
        .map_or(0, |hwnd| hwnd.0 as isize)
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
    let primary = display_of(&monitor);
    let mut all = displays(app)?;
    if !all.contains(&primary) {
        all.push(primary.clone());
    }
    let geometry = Geometry::new(&primary, &all);
    place(&window, geometry.window)?;
    keep_tool_window(&window);
    window.set_ignore_cursor_events(true)?;
    window.show()?;
    app.manage(Arc::new(Overlay::new(geometry)));
    Ok(())
}

/// Giữ overlay là cửa sổ công cụ (`WS_EX_TOOLWINDOW`). Lúc con trỏ nằm trên pet (kéo pet, click chuột phải
/// để chat), overlay tắt click-through và thành cửa sổ đục phủ hết vùng làm việc: Chrome, Edge tính là cửa
/// sổ của nó bị che kín nên ẩn trang, Google Meet thấy trang ẩn thì tắt video người khác chia sẻ (màn hình
/// đen). Chromium không tính cửa sổ công cụ là che. tao ghi đè cả ex-style mỗi lần đổi click-through, nên
/// thêm cờ này ở `WM_STYLECHANGING`, trước mỗi lần đổi. Phải gọi trên luồng tạo cửa sổ (main thread).
#[cfg(windows)]
fn keep_tool_window(window: &WebviewWindow) {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, STYLESTRUCT, WM_STYLECHANGING, WS_EX_TOOLWINDOW,
    };

    unsafe extern "system" fn subclass(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if msg == WM_STYLECHANGING && wparam as i32 == GWL_EXSTYLE {
            // SAFETY: với WM_STYLECHANGING, `lparam` trỏ tới STYLESTRUCT mà Windows cho sửa trước khi áp dụng.
            let style = unsafe { &mut *(lparam as *mut STYLESTRUCT) };
            style.styleNew |= WS_EX_TOOLWINDOW;
        }
        // SAFETY: chuyển tiếp nguyên message cho thủ tục cửa sổ phía sau.
        unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
    }

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let hwnd = hwnd.0 as HWND;
    // SAFETY: `hwnd` là overlay còn sống, tạo trên luồng đang chạy; `subclass` không giữ con trỏ nào.
    unsafe {
        if SetWindowSubclass(hwnd, Some(subclass), 1, 0) == 0 {
            log!("Không giữ được overlay là cửa sổ công cụ, Google Meet có thể bị đen khi chạm vào pet.");
            return;
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, GetWindowLongPtrW(hwnd, GWL_EXSTYLE) | WS_EX_TOOLWINDOW as isize);
    }
}

#[cfg(not(windows))]
fn keep_tool_window(_window: &WebviewWindow) {}

/// Đo lại màn hình mỗi giây, có gì đổi thì đặt lại overlay (`refit`).
pub fn watch(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(WATCH_INTERVAL);
        if let Err(e) = refit(&app) {
            log!("Không đặt lại được overlay theo màn hình: {e}");
        }
    });
}

/// Màn hình đổi độ phân giải, DPI, taskbar, hoặc màn hình khác được cắm/rút: đặt lại overlay trên màn
/// hình đang ở. Màn hình đó bị rút ra thì sang màn hình chính.
fn refit(app: &AppHandle) -> AppResult<()> {
    let overlay = app.state::<Arc<Overlay>>();
    let _moving = overlay.moving();
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
                primary = display_of(&monitor);
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
    let overlay = app.state::<Arc<Overlay>>();
    let _moving = overlay.moving();
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
    overlay.set_geometry(next);
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
    let Some(overlay) = app.try_state::<Arc<Overlay>>() else {
        return false;
    };
    let paused = overlay.toggle_paused();
    if let Err(e) = app.emit_to(LABEL, events::PAUSED, paused) {
        log!("Không báo được trạng thái tạm dừng cho overlay: {e}");
    }
    paused
}

/// Ẩn/hiện theo lựa chọn của người dùng (tray, mở app lần nữa).
pub fn set_user_hidden(app: &AppHandle, hidden: bool) {
    if let Some(overlay) = app.try_state::<Arc<Overlay>>() {
        overlay.set_user_hidden(hidden);
        apply_visibility(app, &overlay);
    }
}

pub fn toggle(app: &AppHandle) {
    if let Some(overlay) = app.try_state::<Arc<Overlay>>() {
        set_user_hidden(app, !overlay.is_user_hidden());
    }
}

/// Ẩn khi có app fullscreen, hiện lại khi hết (trừ khi người dùng đang tự ẩn).
pub fn set_auto_hidden(app: &AppHandle, hidden: bool) {
    if let Some(overlay) = app.try_state::<Arc<Overlay>>() {
        overlay.set_auto_hidden(hidden);
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
        log!("Không đổi được trạng thái ẩn/hiện của overlay: {e}");
    }
    // Cửa sổ ẩn nhưng trang vẫn chạy như đang hiện, báo để frontend dừng vòng lặp vẽ.
    if let Err(e) = app.emit_to(LABEL, events::OVERLAY_VISIBILITY, visible) {
        log!("Không báo được trạng thái ẩn/hiện cho overlay: {e}");
    }
    // Lúc ẩn luồng theo dõi cửa sổ gỡ hook; hiện lại thì đánh thức để nó gắn lại và đọc ngay.
    if visible {
        window_list::refresh();
    }
    apply_memory_level(&window, overlay);
}

/// Pet ngủ (frontend dừng vòng lặp vẽ) hoặc thức dậy.
pub fn set_resting(app: &AppHandle, resting: bool) {
    let Some(overlay) = app.try_state::<Arc<Overlay>>() else {
        return;
    };
    if !overlay.set_resting(resting) {
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
    let low = overlay.is_resting() || !overlay.is_visible();
    if let Err(e) = memory::set_low(window, low) {
        log!("Không đổi được mức dùng RAM của WebView2: {e}");
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
                log!("WebView2 không hỗ trợ đổi mức dùng RAM: {e}");
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
