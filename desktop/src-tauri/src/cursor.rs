//! Đọc vị trí con trỏ khoảng 60 lần/giây, gửi cho overlay khi con trỏ di chuyển. Overlay đang để
//! chuột đi xuyên nên không tự nhận được sự kiện chuột; frontend dùng vị trí này để biết con trỏ có
//! nằm trên pet không.

use crate::overlay::{self, Overlay};
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Tên event, khớp `EVENTS.cursorMoved` trong packages/core.
const EVENT: &str = "cursor-moved";
const POLL_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Copy, Serialize)]
struct Point {
    x: f64,
    y: f64,
}

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let mut last = None;
        loop {
            thread::sleep(POLL_INTERVAL);
            let Some(position) = cursor_position(&app) else {
                continue;
            };
            if last == Some(position) {
                continue;
            }
            last = Some(position);
            let overlay = app.state::<Overlay>();
            if !overlay.is_visible() {
                continue;
            }
            let (x, y) = overlay.geometry().to_local(position.0, position.1);
            if let Err(e) = app.emit_to(overlay::LABEL, EVENT, Point { x, y }) {
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
