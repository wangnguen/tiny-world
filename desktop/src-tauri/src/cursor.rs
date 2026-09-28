//! Đọc vị trí con trỏ khoảng 60 lần/giây, gửi cho overlay khi con trỏ di chuyển hoặc phím Ctrl đổi
//! trạng thái. Overlay đang để chuột đi xuyên nên không tự nhận được sự kiện chuột; frontend dùng
//! thông tin này để biết con trỏ có nằm trên pet không.

use crate::events;
use crate::overlay::{self, Overlay};
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const POLL_INTERVAL: Duration = Duration::from_millis(16);

/// Khớp `CursorInfo` trong packages/core.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct CursorInfo {
    x: f64,
    y: f64,
    /// Đang giữ Ctrl: click xuyên qua pet xuống app bên dưới.
    pass_through: bool,
}

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let mut last = None;
        loop {
            thread::sleep(POLL_INTERVAL);
            let Some((x, y)) = cursor_position(&app) else {
                continue;
            };
            let pass_through = ctrl_pressed();
            if last == Some((x, y, pass_through)) {
                continue;
            }
            last = Some((x, y, pass_through));
            let overlay = app.state::<Overlay>();
            if !overlay.is_visible() {
                continue;
            }
            let (x, y) = overlay.geometry().to_local(x, y);
            let info = CursorInfo { x, y, pass_through };
            if let Err(e) = app.emit_to(overlay::LABEL, events::CURSOR_MOVED, info) {
                eprintln!("Không gửi được vị trí con trỏ: {e}");
            }
        }
    });
}

/// Toạ độ desktop theo pixel vật lý. Gọi thẳng Win32 để không phải chờ main thread mỗi lần đọc.
#[cfg(windows)]
fn cursor_position(_app: &AppHandle) -> Option<(f64, f64)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: GetCursorPos chỉ ghi vào `point` là biến cục bộ hợp lệ.
    let ok = unsafe { GetCursorPos(&mut point) } != 0;
    ok.then(|| (f64::from(point.x), f64::from(point.y)))
}

#[cfg(not(windows))]
fn cursor_position(app: &AppHandle) -> Option<(f64, f64)> {
    app.cursor_position().ok().map(|p| (p.x, p.y))
}

/// Chỉ đọc trạng thái phím Ctrl, không theo dõi phím nào khác.
#[cfg(windows)]
fn ctrl_pressed() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};

    // SAFETY: GetAsyncKeyState chỉ đọc trạng thái phím.
    let state = unsafe { GetAsyncKeyState(i32::from(VK_CONTROL)) };
    // Bit cao bật (số âm) nghĩa là đang giữ phím.
    state < 0
}

#[cfg(not(windows))]
fn ctrl_pressed() -> bool {
    false
}
