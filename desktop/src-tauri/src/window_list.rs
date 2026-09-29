//! Theo dõi các cửa sổ đang hiện để pet đứng, leo, nhảy trên đó. Chỉ lấy khung, thứ tự chồng và tên
//! lớp (để bỏ desktop, taskbar); không đọc tiêu đề hay nội dung cửa sổ.
//!
//! Windows báo mỗi khi cửa sổ mở, đóng, di chuyển, đổi thứ tự chồng (`SetWinEventHook`). Luồng nền
//! gom các lần báo lại, đọc danh sách tối đa 30 lần/giây và chỉ gửi cho overlay khi có gì khác. Không
//! có gì thay đổi thì luồng ngủ; cứ 2 giây đọc lại một lần phòng khi sót.

use crate::events;
use crate::overlay::{self, Geometry, Overlay, PhysicalRect, Rect};
#[cfg(test)]
use crate::overlay::Display;
use serde::Serialize;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Khoảng cách tối thiểu giữa hai lần đọc khi cửa sổ đang thay đổi liên tục (kéo cửa sổ).
const MIN_INTERVAL: Duration = Duration::from_millis(33);
/// Không có event nào thì cứ chừng này đọc lại một lần.
const REFRESH: Duration = Duration::from_secs(2);
/// Cửa sổ nhỏ hơn chừng này (CSS pixel) thì bỏ: không đủ chỗ đứng, thường là cửa sổ phụ.
const MIN_SIZE: f64 = 24.0;
/// Cửa sổ biến khỏi danh sách mà chưa bị huỷ (đang ẩn trước khi đóng, thu nhỏ) thì theo dõi chừng này
/// để kịp báo nếu nó bị đóng.
const DEPARTURE_WINDOW: Duration = Duration::from_secs(3);
/// Đang theo dõi cửa sổ vừa biến mất thì đọc lại ít nhất chừng này một lần: huỷ cửa sổ đã ẩn không
/// phải lúc nào cũng có event.
const DEPARTURE_POLL: Duration = Duration::from_millis(250);

/// Khớp `WindowInfo` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct WindowInfo {
    /// HWND, không đổi trong suốt đời cửa sổ.
    pub id: isize,
    /// CSS pixel của overlay.
    pub rect: Rect,
}

/// Khớp `WindowList` trong packages/core. `windows` xếp từ trên xuống dưới theo thứ tự chồng.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct WindowList {
    pub windows: Vec<WindowInfo>,
    /// Cửa sổ vừa bị đóng hẳn (không phải thu nhỏ hay ẩn) kể từ lần gửi trước, kèm khung lúc còn hiện.
    pub closed: Vec<WindowInfo>,
}

/// Danh sách gửi lần gần nhất, cho command `list_windows` lúc overlay vừa mở.
#[derive(Default)]
pub struct Windows(Mutex<Vec<WindowInfo>>);

impl Windows {
    pub fn current(&self) -> WindowList {
        let windows = self.0.lock().map(|w| w.clone()).unwrap_or_default();
        WindowList {
            windows,
            closed: Vec::new(),
        }
    }

    fn set(&self, windows: &[WindowInfo]) {
        if let Ok(mut current) = self.0.lock() {
            current.clear();
            current.extend_from_slice(windows);
        }
    }
}

/// Một cửa sổ đọc từ Win32, toạ độ desktop theo pixel vật lý.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawWindow {
    pub id: isize,
    pub rect: PhysicalRect,
}

/// Đổi sang CSS pixel của overlay, bỏ cửa sổ không chạm vào overlay hoặc quá nhỏ. Giữ thứ tự chồng.
pub fn to_overlay(raw: &[RawWindow], geometry: &Geometry) -> Vec<WindowInfo> {
    raw.iter()
        .filter(|w| intersects(w.rect, geometry.window))
        .map(|w| WindowInfo {
            id: w.id,
            rect: geometry.rect_to_local(w.rect),
        })
        .filter(|w| w.rect.width >= MIN_SIZE && w.rect.height >= MIN_SIZE)
        .collect()
}

/// Cửa sổ vừa biến khỏi danh sách. Đóng cửa sổ thì Windows ẩn nó trước rồi mới huỷ; lần đọc rơi vào
/// giữa hai bước đó chỉ thấy cửa sổ bị ẩn. Nên giữ lại một lúc, bị huỷ thì báo là đã đóng.
#[derive(Default)]
pub struct Departures(Vec<(WindowInfo, Instant)>);

impl Departures {
    /// Cửa sổ bị đóng hẳn kể từ lần trước: có trong `previous` (hoặc đang theo dõi) mà không còn trong
    /// `now` và `alive` báo đã bị huỷ.
    pub fn update(
        &mut self,
        previous: &[WindowInfo],
        now: &[WindowInfo],
        alive: impl Fn(isize) -> bool,
        at: Instant,
    ) -> Vec<WindowInfo> {
        let listed = |id: isize| now.iter().any(|w| w.id == id);
        for window in previous {
            if !listed(window.id) && !self.0.iter().any(|(w, _)| w.id == window.id) {
                self.0.push((*window, at));
            }
        }
        let mut closed = Vec::new();
        self.0.retain(|(window, since)| {
            if listed(window.id) || at.duration_since(*since) > DEPARTURE_WINDOW {
                return false;
            }
            if alive(window.id) {
                return true;
            }
            closed.push(*window);
            false
        });
        closed
    }

    /// Còn cửa sổ đang chờ xem có bị đóng không.
    pub fn pending(&self) -> bool {
        !self.0.is_empty()
    }
}

fn intersects(a: PhysicalRect, b: PhysicalRect) -> bool {
    let right = |r: PhysicalRect| i64::from(r.x) + i64::from(r.width);
    let bottom = |r: PhysicalRect| i64::from(r.y) + i64::from(r.height);
    i64::from(a.x) < right(b)
        && i64::from(b.x) < right(a)
        && i64::from(a.y) < bottom(b)
        && i64::from(b.y) < bottom(a)
}

/// Overlay vừa đổi chỗ hoặc đổi cỡ: đọc lại danh sách ngay theo toạ độ mới, không chờ lần đọc lại 2 giây.
pub fn refresh() {
    win32::wake();
}

pub fn spawn(app: AppHandle) {
    // Bỏ chính overlay: nó phủ cả vùng làm việc, lúc con trỏ nằm trên pet thì không còn click-through.
    let overlay_hwnd = app
        .get_webview_window(overlay::LABEL)
        .and_then(|window| window.hwnd().ok())
        .map_or(0, |hwnd| hwnd.0 as isize);
    thread::spawn(move || {
        win32::install_hooks();
        let mut sent: Vec<WindowInfo> = Vec::new();
        let mut departures = Departures::default();
        let mut last_read: Option<Instant> = None;
        loop {
            let now = Instant::now();
            let due = match last_read {
                None => now,
                Some(last) if win32::dirty() => last + MIN_INTERVAL,
                Some(last) if departures.pending() => last + DEPARTURE_POLL,
                Some(last) => last + REFRESH,
            };
            if now < due {
                win32::wait(due - now);
                continue;
            }
            // Xoá cờ trước khi đọc: cửa sổ đổi trong lúc đang đọc thì lần sau đọc lại.
            win32::clear_dirty();
            last_read = Some(now);
            let geometry = app.state::<Overlay>().geometry();
            let windows = to_overlay(&win32::list(overlay_hwnd), &geometry);
            let closed = departures.update(&sent, &windows, win32::alive, now);
            if windows == sent && closed.is_empty() {
                continue;
            }
            app.state::<Windows>().set(&windows);
            let list = WindowList {
                windows: windows.clone(),
                closed,
            };
            if let Err(e) = app.emit_to(overlay::LABEL, events::WINDOWS_CHANGED, &list) {
                eprintln!("Không gửi được danh sách cửa sổ: {e}");
            }
            sent = windows;
        }
    });
}

#[cfg(windows)]
mod win32 {
    use super::RawWindow;
    use crate::overlay::PhysicalRect;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::time::Duration;
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
    };
    use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EnumWindows, GetAncestor, GetClassNameW, GetWindowLongPtrW,
        GetWindowRect, IsIconic, IsWindow, IsWindowVisible, MsgWaitForMultipleObjects,
        PeekMessageW, PostThreadMessageW, CHILDID_SELF, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_CREATE,
        EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_REORDER,
        EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND,
        EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART,
        GA_ROOT, GWL_EXSTYLE, MSG, OBJID_WINDOW, PM_REMOVE, QS_ALLINPUT, WINEVENT_OUTOFCONTEXT,
        WM_NULL, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT,
    };
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;

    /// Có cửa sổ vừa thay đổi, chưa đọc lại danh sách.
    static DIRTY: AtomicBool = AtomicBool::new(true);
    /// Luồng theo dõi cửa sổ (luồng gọi `install_hooks`), để `wake` đánh thức.
    static THREAD: AtomicU32 = AtomicU32::new(0);

    /// Event cần theo dõi, theo từng khoảng [min, max].
    const EVENTS: [(u32, u32); 6] = [
        (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
        (EVENT_SYSTEM_MOVESIZESTART, EVENT_SYSTEM_MOVESIZEEND),
        (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
        // Tạo, huỷ, hiện, ẩn, đổi thứ tự chồng.
        (EVENT_OBJECT_CREATE, EVENT_OBJECT_REORDER),
        (EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE),
        // Cửa sổ UWP ẩn/hiện, chuyển desktop ảo.
        (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
    ];

    /// Desktop (hình nền) và taskbar: không phải cửa sổ để pet đứng.
    const SHELL_CLASSES: [&str; 4] = ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

    pub fn dirty() -> bool {
        DIRTY.load(Ordering::Relaxed)
    }

    pub fn clear_dirty() {
        DIRTY.store(false, Ordering::Relaxed);
    }

    /// Hook chạy trên luồng gọi hàm này, luồng đó phải xử lý message (`wait`).
    pub fn install_hooks() {
        // SAFETY: GetCurrentThreadId không có điều kiện gì.
        THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Relaxed);
        for (min, max) in EVENTS {
            // SAFETY: `on_event` đúng chữ ký WINEVENTPROC; WINEVENT_OUTOFCONTEXT không cần DLL.
            let hook = unsafe {
                SetWinEventHook(min, max, std::ptr::null_mut(), Some(on_event), 0, 0, WINEVENT_OUTOFCONTEXT)
            };
            if hook.is_null() {
                eprintln!("Không theo dõi được cửa sổ (event {min:#x}–{max:#x}).");
            }
        }
    }

    unsafe extern "system" fn on_event(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        if hwnd.is_null() || id_object != OBJID_WINDOW || id_child != CHILDID_SELF as i32 {
            return;
        }
        // Cửa sổ con (nút, ô nhập...) cũng báo, chỉ cần cửa sổ cấp cao nhất. Cửa sổ vừa bị huỷ thì không
        // hỏi được nữa nên luôn tính.
        // SAFETY: GetAncestor chỉ đọc, HWND không hợp lệ thì trả về null.
        if event != EVENT_OBJECT_DESTROY && unsafe { GetAncestor(hwnd, GA_ROOT) } != hwnd {
            return;
        }
        DIRTY.store(true, Ordering::Relaxed);
    }

    /// Đánh dấu cần đọc lại và đánh thức luồng đang chờ trong `wait`.
    pub fn wake() {
        DIRTY.store(true, Ordering::Relaxed);
        let thread = THREAD.load(Ordering::Relaxed);
        if thread != 0 {
            // SAFETY: gửi message rỗng tới một luồng; luồng đã thoát thì hàm chỉ trả về lỗi.
            unsafe { PostThreadMessageW(thread, WM_NULL, 0, 0) };
        }
    }

    /// Chờ event hoặc hết `timeout`, rồi xử lý hết message đang chờ (hook được gọi trong lúc này).
    pub fn wait(timeout: Duration) {
        let ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX).max(1);
        // SAFETY: không truyền handle nào; MSG là biến cục bộ hợp lệ.
        unsafe {
            MsgWaitForMultipleObjects(0, std::ptr::null(), 0, ms, QS_ALLINPUT);
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                DispatchMessageW(&msg);
            }
        }
    }

    /// Cửa sổ đang hiện, xếp từ trên xuống dưới theo thứ tự chồng.
    pub fn list(skip: isize) -> Vec<RawWindow> {
        let mut state = (skip, Vec::new());
        // SAFETY: `collect` chỉ dùng `lparam` như con trỏ tới `state`, còn sống suốt lúc EnumWindows chạy.
        unsafe { EnumWindows(Some(collect), &mut state as *mut (isize, Vec<RawWindow>) as LPARAM) };
        state.1
    }

    pub fn alive(id: isize) -> bool {
        // SAFETY: IsWindow chỉ kiểm tra handle.
        unsafe { IsWindow(id as HWND) != 0 }
    }

    unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // SAFETY: `list` truyền con trỏ tới `(isize, Vec<RawWindow>)`.
        let (skip, windows) = unsafe { &mut *(lparam as *mut (isize, Vec<RawWindow>)) };
        if hwnd as isize != *skip {
            if let Some(window) = inspect(hwnd) {
                windows.push(window);
            }
        }
        1
    }

    fn inspect(hwnd: HWND) -> Option<RawWindow> {
        // SAFETY: các hàm dưới chỉ đọc thông tin của `hwnd`, handle hỏng thì trả về lỗi/0.
        unsafe {
            if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                return None;
            }
            // Cửa sổ công cụ (tooltip, menu, overlay của app khác) và cửa sổ để chuột đi xuyên thì không
            // phải chỗ đứng, cũng không che pet.
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            if ex_style & (WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT) != 0 {
                return None;
            }
            let mut cloaked: u32 = 0;
            let ok = DwmGetWindowAttribute(
                hwnd,
                DWMWA_CLOAKED as u32,
                (&mut cloaked as *mut u32).cast::<c_void>(),
                size_of::<u32>() as u32,
            ) >= 0;
            if ok && cloaked != 0 {
                return None;
            }
            let mut class = [0u16; 64];
            let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
            let class = String::from_utf16_lossy(&class[..usize::try_from(len).unwrap_or(0)]);
            if SHELL_CLASSES.contains(&class.as_str()) {
                return None;
            }
            frame(hwnd).map(|rect| RawWindow {
                id: hwnd as isize,
                rect,
            })
        }
    }

    /// Khung nhìn thấy của cửa sổ. `GetWindowRect` trên Windows 10/11 tính cả viền kéo giãn trong suốt
    /// (khoảng 7 px mỗi bên) nên hỏi DWM trước.
    unsafe fn frame(hwnd: HWND) -> Option<PhysicalRect> {
        let mut r = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: `r` là biến cục bộ đúng kích thước truyền vào.
        let dwm = unsafe {
            DwmGetWindowAttribute(
                hwnd,
                DWMWA_EXTENDED_FRAME_BOUNDS as u32,
                (&mut r as *mut RECT).cast::<c_void>(),
                size_of::<RECT>() as u32,
            )
        } >= 0;
        // SAFETY: như trên.
        if !dwm && unsafe { GetWindowRect(hwnd, &mut r) } == 0 {
            return None;
        }
        let width = u32::try_from(r.right - r.left).ok().filter(|w| *w > 0)?;
        let height = u32::try_from(r.bottom - r.top).ok().filter(|h| *h > 0)?;
        Some(PhysicalRect {
            x: r.left,
            y: r.top,
            width,
            height,
        })
    }
}

#[cfg(not(windows))]
mod win32 {
    use super::RawWindow;
    use std::time::Duration;

    pub fn dirty() -> bool {
        false
    }
    pub fn clear_dirty() {}
    pub fn install_hooks() {}
    pub fn wake() {}
    pub fn wait(timeout: Duration) {
        std::thread::sleep(timeout);
    }
    pub fn list(_skip: isize) -> Vec<RawWindow> {
        Vec::new()
    }
    pub fn alive(_id: isize) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(id: isize, x: i32, y: i32, width: u32, height: u32) -> RawWindow {
        RawWindow {
            id,
            rect: PhysicalRect {
                x,
                y,
                width,
                height,
            },
        }
    }

    #[test]
    fn doi_sang_css_pixel_bo_cua_so_ngoai_man_hinh_va_qua_nho() {
        // Màn hình chính 1920×1080 ở scale 150%, taskbar cao 60 px vật lý ở dưới.
        let monitor = PhysicalRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let work = PhysicalRect {
            height: 1020,
            ..monitor
        };
        let display = Display {
            name: "1".into(),
            monitor,
            work_area: work,
            scale_factor: 1.5,
        };
        let g = Geometry::new(&display, &[]);
        let windows = to_overlay(
            &[
                raw(1, 300, 150, 600, 450),
                // Ở màn hình thứ hai.
                raw(2, 2000, 100, 800, 600),
                // Nhỏ hơn 24 CSS pixel.
                raw(3, 100, 100, 30, 30),
                // Thò một phần ra ngoài màn hình vẫn giữ, toạ độ âm.
                raw(4, -150, 900, 600, 300),
            ],
            &g,
        );
        assert_eq!(
            windows,
            vec![
                WindowInfo {
                    id: 1,
                    rect: Rect {
                        x: 200.0,
                        y: 100.0,
                        width: 400.0,
                        height: 300.0
                    }
                },
                WindowInfo {
                    id: 4,
                    rect: Rect {
                        x: -100.0,
                        y: 600.0,
                        width: 400.0,
                        height: 200.0
                    }
                },
            ]
        );
    }

    fn info(id: isize) -> WindowInfo {
        WindowInfo {
            id,
            rect: Rect {
                x: id as f64 * 10.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        }
    }

    #[test]
    fn chi_bao_cua_so_bi_dong_han() {
        let mut departures = Departures::default();
        let at = Instant::now();
        // 1 vẫn còn, 2 bị thu nhỏ (handle còn sống), 3 bị đóng.
        let closed = departures.update(&[info(1), info(2), info(3)], &[info(1)], |id| id == 2, at);
        assert_eq!(closed, vec![info(3)]);
        // 2 chưa bị huỷ thì không báo, vẫn theo dõi tiếp.
        assert!(departures.update(&[info(1)], &[info(1)], |id| id == 2, at).is_empty());
        assert!(departures.pending());
    }

    #[test]
    fn cua_so_an_roi_moi_bi_huy_van_bao_la_dong() {
        let mut departures = Departures::default();
        let at = Instant::now();
        // Lần đọc thứ nhất rơi vào lúc cửa sổ 5 đã ẩn mà chưa bị huỷ.
        assert!(departures.update(&[info(5)], &[], |_| true, at).is_empty());
        // Lần sau cửa sổ đã bị huỷ: báo đóng, kèm khung lúc còn hiện.
        let later = at + Duration::from_millis(100);
        assert_eq!(departures.update(&[], &[], |_| false, later), vec![info(5)]);
        assert!(!departures.pending());
    }

    #[test]
    fn hien_lai_hoac_an_qua_lau_thi_thoi_theo_doi() {
        let mut departures = Departures::default();
        let at = Instant::now();
        departures.update(&[info(1), info(2)], &[], |_| true, at);
        // 1 hiện lại (khôi phục sau khi thu nhỏ).
        departures.update(&[], &[info(1)], |_| true, at + Duration::from_millis(100));
        // 2 thu nhỏ quá lâu rồi mới bị đóng: không tính là vừa đóng.
        let late = at + DEPARTURE_WINDOW + Duration::from_millis(1);
        assert!(departures.update(&[info(1)], &[info(1)], |_| false, late).is_empty());
        assert!(!departures.pending());
    }
}
