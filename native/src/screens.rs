//! Các màn hình đang cắm (pixel vật lý, toạ độ desktop) và màn hình chính.

use tinyworld_backend::geometry::{Display, PhysicalRect};
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

fn physical(r: RECT) -> PhysicalRect {
    PhysicalRect {
        x: r.left,
        y: r.top,
        width: (r.right - r.left).max(0) as u32,
        height: (r.bottom - r.top).max(0) as u32,
    }
}

/// Mọi màn hình, màn hình chính đứng đầu.
pub fn displays() -> Vec<Display> {
    unsafe extern "system" fn each(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        // SAFETY: `data` là con trỏ tới Vec do `displays` truyền vào, còn sống suốt lúc liệt kê.
        let list = unsafe { &mut *(data.0 as *mut Vec<(Display, bool)>) };
        let mut info = MONITORINFOEXW::default();
        info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
        // SAFETY: `info` đủ chỗ cho MONITORINFOEXW như `cbSize` báo.
        if !unsafe { GetMonitorInfoW(monitor, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO) }.as_bool() {
            return true.into();
        }
        let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
        // SAFETY: chỉ đọc DPI của màn hình đang liệt kê.
        let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        let name = String::from_utf16_lossy(&info.szDevice).trim_end_matches('\0').to_owned();
        let display = Display {
            name,
            monitor: physical(info.monitorInfo.rcMonitor),
            work_area: physical(info.monitorInfo.rcWork),
            scale_factor: f64::from(dpi_x.max(1)) / 96.0,
        };
        list.push((display, info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0));
        true.into()
    }

    let mut list: Vec<(Display, bool)> = Vec::new();
    // SAFETY: `each` chỉ dùng `list` trong lúc hàm này chạy.
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(each), LPARAM(&mut list as *mut _ as isize));
    }
    list.sort_by_key(|(_, primary)| !primary);
    list.into_iter().map(|(display, _)| display).collect()
}
