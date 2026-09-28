//! Tự ẩn pet khi đang có app fullscreen (video, game, trình chiếu) để không đè lên, hết thì hiện lại.
//!
//! Hỏi thẳng Windows qua `SHQueryUserNotificationState`, cùng cách Windows quyết định tắt thông báo.
//! Overlay không phủ kín màn hình (xem overlay.rs) nên không tự làm Windows tưởng có app fullscreen.

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
    use windows_sys::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
    };

    let mut state = 0;
    // SAFETY: SHQueryUserNotificationState chỉ ghi vào `state` là biến cục bộ hợp lệ.
    let ok = unsafe { SHQueryUserNotificationState(&mut state) } >= 0;
    ok && matches!(state, QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE)
}

#[cfg(not(windows))]
fn fullscreen_app_active() -> bool {
    false
}
