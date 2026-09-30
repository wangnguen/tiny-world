//! Đọc vị trí con trỏ khoảng 60 lần/giây, gửi cho overlay khi con trỏ di chuyển, phím Ctrl hoặc nút
//! chuột đổi trạng thái. Overlay đang để chuột đi xuyên nên không tự nhận được sự kiện chuột; frontend
//! dùng thông tin này để biết con trỏ có nằm trên pet không, và click ở đâu thì pet đang ngủ cũng dậy.
//!
//! Chỉ gửi vị trí khi con trỏ ở trong vùng quanh pet mà frontend báo (`CursorInterest`): ngoài vùng đó
//! thì chỉ gửi lúc bấm/nhả chuột và một lần lúc vừa ra khỏi vùng, để WebView không phải thức dậy mỗi lần
//! chuột di chuyển.

use crate::events;
use crate::overlay::{self, Overlay, Rect};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const POLL_INTERVAL: Duration = Duration::from_millis(16);
/// Overlay đang ẩn (app fullscreen, tray): không gửi gì, chỉ đọc thưa để kịp biết lúc hiện lại.
const HIDDEN_POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Vùng quanh pet (CSS pixel của overlay) mà frontend cần biết vị trí con trỏ. `None`: mọi chỗ.
#[derive(Default)]
pub struct CursorInterest {
    rect: Mutex<Option<Rect>>,
    /// Vùng vừa đổi: gửi lại vị trí con trỏ dù nó đứng yên, vì pet có thể vừa đi tới dưới con trỏ.
    changed: AtomicBool,
}

impl CursorInterest {
    pub fn set(&self, rect: Option<Rect>) {
        if let Ok(mut current) = self.rect.lock() {
            *current = rect;
        }
        self.changed.store(true, Ordering::Relaxed);
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        self.rect
            .lock()
            .map_or(true, |rect| rect.is_none_or(|r| r.contains(x, y)))
    }

    fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::Relaxed)
    }
}

/// Khớp `CursorInfo` trong packages/core.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct CursorInfo {
    x: f64,
    y: f64,
    /// Đang giữ Ctrl: click xuyên qua pet xuống app bên dưới.
    pass_through: bool,
    /// Đang giữ một nút chuột, ở bất kỳ đâu trên màn hình.
    pressed: bool,
}

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let mut last = None;
        // Lần gửi trước con trỏ có nằm trong vùng quanh pet không.
        let mut was_inside = true;
        // Lần đọc trước overlay có đang hiện không.
        let mut shown = false;
        loop {
            let overlay = app.state::<Overlay>();
            let interest = app.state::<CursorInterest>();
            if !overlay.is_visible() {
                shown = false;
                thread::sleep(HIDDEN_POLL_INTERVAL);
                continue;
            }
            thread::sleep(POLL_INTERVAL);
            let Some((x, y)) = cursor_position(&app) else {
                continue;
            };
            let pass_through = ctrl_pressed();
            let pressed = mouse_pressed();
            // Vùng quanh pet vừa đổi, hoặc overlay vừa hiện lại: gửi vị trí dù con trỏ đứng yên. Không gửi thì
            // vị trí cũ frontend đang giữ có thể lọt vào vùng mới, tưởng con trỏ nằm trên pet mà tắt
            // click-through.
            let refresh = interest.take_changed() || !shown;
            shown = true;
            if last == Some((x, y, pass_through, pressed)) && !refresh {
                continue;
            }
            let clicked = last.is_none_or(|(_, _, _, was_pressed)| was_pressed != pressed);
            last = Some((x, y, pass_through, pressed));
            let (x, y) = overlay.to_local(x, y);
            let inside = interest.contains(x, y);
            // Ngoài vùng quanh pet: chỉ gửi lúc vừa ra khỏi vùng (con trỏ không còn trên pet) và lúc bấm/nhả
            // chuột (click ở đâu pet đang ngủ cũng dậy).
            if !inside && !was_inside && !clicked && !refresh {
                continue;
            }
            was_inside = inside;
            let info = CursorInfo {
                x,
                y,
                pass_through,
                pressed,
            };
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

/// Chỉ đọc trạng thái phím Ctrl, không theo dõi phím bàn phím nào khác.
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

/// Nút chuột trái, phải hoặc giữa đang được giữ (nút vật lý, nên đổi tay chuột cũng không sao).
#[cfg(windows)]
fn mouse_pressed() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON,
    };

    [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON].into_iter().any(|key| {
        // SAFETY: GetAsyncKeyState chỉ đọc trạng thái nút.
        let state = unsafe { GetAsyncKeyState(i32::from(key)) };
        // Bit cao bật (số âm) nghĩa là đang giữ nút.
        state < 0
    })
}

#[cfg(not(windows))]
fn mouse_pressed() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chua_bao_vung_thi_gui_moi_cho_bao_roi_thi_chi_trong_vung() {
        let interest = CursorInterest::default();
        assert!(interest.contains(-5000.0, 3000.0));
        interest.set(Some(Rect {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 40.0,
        }));
        assert!(interest.take_changed());
        assert!(!interest.take_changed());
        assert!(interest.contains(100.0, 200.0));
        assert!(!interest.contains(150.0, 220.0));
        assert!(!interest.contains(-5000.0, 3000.0));
        interest.set(None);
        assert!(interest.contains(-5000.0, 3000.0));
    }
}
