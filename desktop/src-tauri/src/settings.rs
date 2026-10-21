//! Cửa sổ Cài đặt. Cài đặt và cách lưu ở `tinyworld_backend::settings`.

pub use tinyworld_backend::settings::*;

use crate::app_icon;
use tauri::window::Color;
use tauri::{AppHandle, Manager, Theme, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW_LABEL: &str = "settings";

/// Mở cửa sổ Settings, đang mở thì đưa lên trước. Chỉ tạo khi cần và huỷ khi đóng để đỡ tốn RAM.
///
/// Tạo webview trong handler của tray trên Windows có thể treo (xem tài liệu
/// `WebviewWindowBuilder::build`), nên chạy ở luồng riêng.
pub fn open_window(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = show_or_create(&app) {
            log!("Không mở được cửa sổ cài đặt: {e}");
        }
    });
}

fn show_or_create(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    // Nền và thanh tiêu đề tối, trùng màu trang (settings.css) để lúc mở không bị loé trắng.
    let window = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("settings.html".into()))
        .title("TinyWorld")
        .inner_size(440.0, 740.0)
        .resizable(false)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .background_color(Color(13, 22, 40, 255))
        .center()
        .focused(true)
        .build()?;
    app_icon::set_small_icon(&window);
    Ok(())
}
