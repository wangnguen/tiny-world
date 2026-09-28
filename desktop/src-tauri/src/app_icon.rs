//! Icon cỡ nhỏ (tray, thanh tiêu đề) lấy đúng cỡ có sẵn trong icon.ico.
//!
//! icon.ico có bản 16/24/32 px vẽ tay theo lưới pixel (scripts/small-icons.mjs). Mặc định Tauri dùng
//! icon 32 px cho mọi chỗ rồi Windows tự thu nhỏ xuống 16 px, mắt và miệng của pet nhoè thành vệt.

use tauri::image::Image;
use tauri::{AppHandle, WebviewWindow};

/// Cỡ icon nhỏ của hệ thống: 16 px ở 100%, 24 px ở 150%...
#[cfg(windows)]
fn small_size() -> i32 {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};
    // SAFETY: GetSystemMetrics chỉ đọc số đo của hệ thống.
    match unsafe { GetSystemMetrics(SM_CXSMICON) } {
        n if n > 0 => n,
        _ => 16,
    }
}

/// Icon cho tray, đúng cỡ tray vẽ.
pub fn tray(app: &AppHandle) -> Option<Image<'static>> {
    #[cfg(windows)]
    if let Ok(icon) = Image::from_app_icon_resource(small_size() as u32) {
        return Some(icon);
    }
    app.default_window_icon().map(|icon| icon.clone().to_owned())
}

/// Đặt icon nhỏ (thanh tiêu đề) đúng cỡ cho cửa sổ; icon lớn (taskbar, Alt+Tab) giữ nguyên.
#[cfg(windows)]
pub fn set_small_icon(window: &WebviewWindow) {
    use tauri::utils::platform::WINDOWS_APP_ICON_RESOURCE_ID;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        LoadImageW, SendMessageW, ICON_SMALL, IMAGE_ICON, LR_SHARED, WM_SETICON,
    };

    let Ok(hwnd) = window.hwnd() else {
        return;
    };
    let size = small_size();
    // SAFETY: icon nằm trong exe đang chạy (MAKEINTRESOURCE theo id); LR_SHARED để Windows tự giữ và
    // giải phóng handle, gọi nhiều lần cũng không rò. `hwnd` là cửa sổ còn sống của app.
    unsafe {
        let resource = WINDOWS_APP_ICON_RESOURCE_ID as usize as *const u16;
        let icon = LoadImageW(GetModuleHandleW(std::ptr::null()), resource, IMAGE_ICON, size, size, LR_SHARED);
        if !icon.is_null() {
            SendMessageW(hwnd.0 as _, WM_SETICON, ICON_SMALL as usize, icon as isize);
        }
    }
}

#[cfg(not(windows))]
pub fn set_small_icon(_window: &WebviewWindow) {}
