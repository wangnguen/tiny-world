//! Cửa sổ overlay: phủ vùng làm việc của màn hình chính, luôn nằm trên, không lấy focus, không hiện trên taskbar
//! hay Alt+Tab. `WS_EX_TOOLWINDOW` cũng là thứ giữ cho Google Meet không đen video khi con trỏ nằm trên pet (xem
//! desktop/src-tauri/src/overlay.rs). Chuột đi xuyên qua nhờ `WS_EX_LAYERED | WS_EX_TRANSPARENT`; bỏ
//! `WS_EX_TRANSPARENT` lúc con trỏ nằm trên pet để click và kéo được.

use anyhow::{bail, Result};
use std::cell::RefCell;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTOPRIMARY};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetWindowLongPtrW, LoadCursorW, PostQuitMessage, RegisterClassExW,
    SetLayeredWindowAttributes, SetWindowDisplayAffinity, SetWindowLongPtrW, ShowWindow, GWL_EXSTYLE, IDC_ARROW,
    LWA_ALPHA, MA_NOACTIVATE, SW_SHOWNOACTIVATE, WDA_EXCLUDEFROMCAPTURE, WM_CAPTURECHANGED, WM_DESTROY,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_NOREDIRECTIONBITMAP, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};

/// Hình chữ nhật theo pixel vật lý, toạ độ desktop.
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Nút trái chuột trên overlay (chỉ tới được lúc overlay không cho chuột đi xuyên).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Press,
    Release,
}

thread_local! {
    static INPUT: RefCell<Vec<Input>> = const { RefCell::new(Vec::new()) };
}

/// Các lần bấm/nhả chuột từ lần gọi trước.
pub fn take_input() -> Vec<Input> {
    INPUT.with(|input| std::mem::take(&mut *input.borrow_mut()))
}

/// Vùng làm việc của màn hình chính và tỉ lệ DPI. Taskbar tự ẩn thì vùng làm việc trùng cả màn hình: bớt 1 pixel
/// để Windows không coi overlay là app fullscreen (như `Geometry::new` của shell Tauri).
pub fn work_area() -> Option<(Rect, f32)> {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let (mut dpi_x, mut dpi_y) = (96, 96);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);
        let (work, whole) = (info.rcWork, info.rcMonitor);
        let mut rect =
            Rect { x: work.left, y: work.top, width: work.right - work.left, height: work.bottom - work.top };
        if work == whole {
            rect.height -= 1;
        }
        Some((rect, dpi_x as f32 / 96.0))
    }
}

/// Tạo và hiện overlay (chuột đi xuyên sẵn). `layered: false` bỏ `WS_EX_LAYERED | WS_EX_TRANSPARENT`, chỉ để thử
/// DirectComposition có vẽ được lên cửa sổ layered không.
pub fn create(rect: Rect, layered: bool) -> Result<HWND> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: w!("TinyWorldNativeOverlay"),
            ..Default::default()
        };
        if RegisterClassExW(&class) == 0 {
            bail!("RegisterClassExW: {}", windows::core::Error::from_thread());
        }
        let mut style = WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;
        if layered {
            style |= WS_EX_LAYERED | WS_EX_TRANSPARENT;
        }
        let hwnd = CreateWindowExW(
            style,
            w!("TinyWorldNativeOverlay"),
            w!("TinyWorld"),
            WS_POPUP,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        if layered {
            // Cửa sổ layered chưa đặt độ mờ thì không hiện gì.
            SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;
        }
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        Ok(hwnd)
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

/// Ẩn overlay khỏi ảnh chụp và chia sẻ màn hình. Overlay WebView2 gọi hàm này luôn báo lỗi 8 (PLAN.md, "Chưa làm").
pub fn exclude_from_capture(hwnd: HWND) -> Result<()> {
    unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)? };
    Ok(())
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let push = |event| INPUT.with(|input| input.borrow_mut().push(event));
    match msg {
        // Bấm vào pet không cướp focus của app đang dùng.
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_LBUTTONDOWN => {
            push(Input::Press);
            LRESULT(0)
        }
        WM_LBUTTONUP | WM_CAPTURECHANGED => {
            push(Input::Release);
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
