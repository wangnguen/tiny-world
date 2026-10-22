//! Cửa sổ overlay: phủ vùng làm việc của một màn hình, luôn nằm trên, không lấy focus, không hiện trên taskbar hay
//! Alt+Tab. `WS_EX_TOOLWINDOW` còn giữ cho Chrome, Edge không tính overlay là cửa sổ che kín trang (Google Meet thấy
//! trang bị che thì tắt video người khác chia sẻ). Chuột đi xuyên qua nhờ `WS_EX_LAYERED | WS_EX_TRANSPARENT`; bỏ
//! `WS_EX_TRANSPARENT` lúc con trỏ nằm trên pet để click và kéo được.
//!
//! Thủ tục cửa sổ chỉ ghi lại sự kiện (`take_input`); vòng lặp chính xử lý ở lượt sau.

use anyhow::{bail, Result};
use std::cell::RefCell;
use std::sync::atomic::{AtomicIsize, Ordering};
use tinyworld_backend::geometry::PhysicalRect;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, ReleaseCapture, SetCapture, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, FindWindowW, GetMessageTime, GetWindowLongPtrW, LoadCursorW, PostMessageW,
    PostQuitMessage, RegisterClassExW, RegisterWindowMessageW, SetLayeredWindowAttributes,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, IDC_ARROW, LWA_ALPHA, MA_NOACTIVATE,
    SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE, WM_APP, WM_CAPTURECHANGED,
    WM_DESTROY, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETTINGCHANGE, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

/// Bản dev và bản cài chạy song song được: khác tên lớp cửa sổ (và khác mutex, thư mục dữ liệu).
#[cfg(debug_assertions)]
pub const CLASS: PCWSTR = w!("TinyWorldOverlayDev");
#[cfg(not(debug_assertions))]
pub const CLASS: PCWSTR = w!("TinyWorldOverlay");

/// Luồng khác nhờ vòng lặp chính thức dậy (có event, có lệnh từ cửa sổ Cài đặt).
const WM_WAKE: u32 = WM_APP + 1;

/// Sự kiện chuột và hệ thống, toạ độ là pixel của cửa sổ (có thể âm hoặc vượt cửa sổ lúc đang giữ chuột).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Down { x: i32, y: i32, shift: bool, time: u32 },
    Move { x: i32, y: i32, time: u32 },
    Up { x: i32, y: i32, time: u32 },
    /// Mất quyền giữ chuột (cửa sổ khác lấy), như `pointercancel`.
    Cancel,
    /// Click chuột phải.
    Context { x: i32, y: i32 },
    /// Màn hình đổi độ phân giải, DPI, taskbar: đo lại ngay.
    Display,
    /// Mở app lần nữa: hiện lại pet nếu đang ẩn.
    Show,
}

thread_local! {
    static INPUT: RefCell<Vec<Input>> = const { RefCell::new(Vec::new()) };
}

static HWND_VALUE: AtomicIsize = AtomicIsize::new(0);

/// Các sự kiện từ lần gọi trước.
pub fn take_input() -> Vec<Input> {
    INPUT.with(|input| std::mem::take(&mut *input.borrow_mut()))
}

fn show_message() -> u32 {
    unsafe { RegisterWindowMessageW(w!("TinyWorld.Show")) }
}

/// Đánh thức vòng lặp chính từ luồng bất kỳ.
pub fn wake() {
    let hwnd = HWND_VALUE.load(Ordering::Relaxed);
    if hwnd != 0 {
        unsafe {
            let _ = PostMessageW(Some(HWND(hwnd as _)), WM_WAKE, WPARAM(0), LPARAM(0));
        }
    }
}

/// Có bản đang chạy thì nhờ nó hiện lại pet; trả `true` nếu đã gửi được.
pub fn signal_running() -> bool {
    unsafe {
        match FindWindowW(CLASS, PCWSTR::null()) {
            Ok(hwnd) if !hwnd.is_invalid() => PostMessageW(Some(hwnd), show_message(), WPARAM(0), LPARAM(0)).is_ok(),
            _ => false,
        }
    }
}

/// Tạo overlay (ẩn, chuột đi xuyên sẵn) ở `rect` (pixel vật lý, toạ độ desktop).
pub fn create(rect: PhysicalRect) -> Result<HWND> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: CLASS,
            ..Default::default()
        };
        if RegisterClassExW(&class) == 0 {
            bail!("RegisterClassExW: {}", windows::core::Error::from_thread());
        }
        let style =
            WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED | WS_EX_TRANSPARENT;
        let hwnd = CreateWindowExW(
            style,
            CLASS,
            w!("TinyWorld"),
            WS_POPUP,
            rect.x,
            rect.y,
            rect.width as i32,
            rect.height as i32,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        // Cửa sổ layered chưa đặt độ mờ thì không hiện gì.
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;
        HWND_VALUE.store(hwnd.0 as isize, Ordering::Relaxed);
        Ok(hwnd)
    }
}

/// Đặt cửa sổ đúng chỗ, đúng cỡ (pixel vật lý).
pub fn place(hwnd: HWND, rect: PhysicalRect) {
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            rect.x,
            rect.y,
            rect.width as i32,
            rect.height as i32,
            SWP_NOACTIVATE,
        );
    }
}

pub fn set_visible(hwnd: HWND, visible: bool) {
    unsafe {
        let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
    }
}

/// `true`: chuột đi xuyên overlay xuống app bên dưới.
pub fn set_click_through(hwnd: HWND, enabled: bool) {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let flag = WS_EX_TRANSPARENT.0 as isize;
        let next = if enabled { style | flag } else { style & !flag };
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
        }
    }
}

/// Giữ chuột (nhận cả khi con trỏ ra ngoài overlay) hoặc thả.
pub fn capture(hwnd: HWND, held: bool) {
    unsafe {
        if held {
            SetCapture(hwnd);
        } else {
            let _ = ReleaseCapture();
        }
    }
}

fn point(lparam: LPARAM) -> (i32, i32) {
    let value = lparam.0 as u32;
    (i32::from((value & 0xffff) as u16 as i16), i32::from((value >> 16) as u16 as i16))
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let push = |event| INPUT.with(|input| input.borrow_mut().push(event));
    let time = || unsafe { GetMessageTime() } as u32;
    match msg {
        // Bấm vào pet không cướp focus của app đang dùng.
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONDOWN => {
            let (x, y) = point(lparam);
            let shift = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
            push(Input::Down { x, y, shift, time: time() });
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = point(lparam);
            push(Input::Move { x, y, time: time() });
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let (x, y) = point(lparam);
            push(Input::Up { x, y, time: time() });
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            // Cửa sổ khác lấy mất quyền giữ chuột (tự thả bằng `capture(false)` thì `lparam` là 0).
            if lparam.0 != 0 && lparam.0 != hwnd.0 as isize {
                push(Input::Cancel);
            }
            LRESULT(0)
        }
        WM_RBUTTONDOWN => LRESULT(0),
        WM_RBUTTONUP => {
            let (x, y) = point(lparam);
            push(Input::Context { x, y });
            LRESULT(0)
        }
        WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
            push(Input::Display);
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_WAKE => LRESULT(0),
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ if msg == show_message() => {
            push(Input::Show);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
