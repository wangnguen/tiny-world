//! Tự ẩn pet khi đang có app fullscreen thật (game, video, trình chiếu) để không đè lên, hết thì hiện lại.
//!
//! Kiểm tra cửa sổ đang dùng có phủ kín cả màn hình (bao gồm cả taskbar) hay không. Cửa sổ phóng to
//! (maximize) còn thanh tiêu đề không bị coi là fullscreen → pet vẫn hiện, kể cả khi nó phủ kín màn hình
//! (taskbar tự ẩn, màn hình phụ không có taskbar).

use crate::overlay;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let mut last = false;
        loop {
            thread::sleep(POLL_INTERVAL);
            let busy = fullscreen_app_active();
            if busy != last {
                last = busy;
                overlay::set_auto_hidden(&app, busy);
            }
        }
    });
}

#[cfg(windows)]
fn fullscreen_app_active() -> bool {
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, IsZoomed, GWL_STYLE,
        WS_CAPTION,
    };

    // SAFETY: chỉ đọc thông tin cửa sổ và màn hình; mọi con trỏ truyền vào đều trỏ tới biến cục bộ hợp lệ.
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_null() {
            return false;
        }
        // Cửa sổ phóng to còn thanh tiêu đề không phải fullscreen. Taskbar tự ẩn hay màn hình không có
        // taskbar thì vùng làm việc trùng cả màn hình, khung cửa sổ phóng to (tràn ra ngoài vài pixel viền)
        // cũng phủ kín màn hình. Phóng to mà bỏ viền (cách WPF, WinForms làm fullscreen) thì vẫn tính.
        let style = GetWindowLongPtrW(fg, GWL_STYLE) as u32;
        if IsZoomed(fg) != 0 && style & WS_CAPTION == WS_CAPTION {
            return false;
        }
        // Bỏ desktop và taskbar.
        let mut class = [0u16; 64];
        let len = GetClassNameW(fg, class.as_mut_ptr(), class.len() as i32);
        let class_name = String::from_utf16_lossy(&class[..usize::try_from(len).unwrap_or(0)]);
        if matches!(
            class_name.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        ) {
            return false;
        }
        // Cửa sổ bị ẩn (cloaked, desktop ảo khác) thì không tính.
        let mut cloaked: u32 = 0;
        let ok = DwmGetWindowAttribute(
            fg,
            DWMWA_CLOAKED as u32,
            (&mut cloaked as *mut u32).cast::<c_void>(),
            size_of::<u32>() as u32,
        ) >= 0;
        if ok && cloaked != 0 {
            return false;
        }
        // Lấy khung cửa sổ và khung cả màn hình (kể cả taskbar).
        let mut wr = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if GetWindowRect(fg, &mut wr) == 0 {
            return false;
        }
        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return false;
        }
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut mi) == 0 {
            return false;
        }
        // Cửa sổ phủ kín cả màn hình (bao gồm taskbar) → fullscreen thật.
        let mr = mi.rcMonitor;
        wr.left <= mr.left && wr.top <= mr.top && wr.right >= mr.right && wr.bottom >= mr.bottom
    }
}

#[cfg(not(windows))]
fn fullscreen_app_active() -> bool {
    false
}
