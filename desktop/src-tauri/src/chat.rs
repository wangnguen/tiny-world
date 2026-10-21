//! Cửa sổ chat nhỏ cạnh pet (Phase 6). Hỏi Gemini, giới hạn tần suất, câu gợi ý ở `tinyworld_backend::chat`.

pub use tinyworld_backend::chat::*;

use crate::error::AppResult;
use crate::events;
use crate::overlay::{self, Overlay};
use crate::settings::SettingsStore;
use std::sync::Arc;
use tauri::window::Color;
use tauri::{
    AppHandle, Emitter, LogicalPosition, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub const WINDOW_LABEL: &str = "chat";

/// Mở cửa sổ chat với `target` cạnh `pet`. Đang mở thì đổi sang pet đó, dời tới cạnh nó và đưa lên trước.
/// Trả về phía của cửa sổ so với pet (-1 bên trái, 1 bên phải) để pet quay mặt về phía đó.
pub fn open(app: &AppHandle, target: ChatTarget, pet: PetBox) -> AppResult<i8> {
    if !app.state::<Arc<SettingsStore>>().get().chat {
        return Err(chat_off());
    }
    let target = checked_target(target)?;
    app.state::<Chat>().set_target(Some(target.clone()));
    let side = side_of(&app.state::<Arc<Overlay>>().geometry().screen, pet, WIDTH + FRAME_GUESS.0);
    let app = app.clone();
    // Tạo webview trong command trên Windows có thể treo (như cửa sổ Cài đặt), nên chạy ở luồng riêng.
    std::thread::spawn(move || {
        if let Err(e) = show_or_create(&app, &target, pet, side) {
            log!("Không mở được cửa sổ chat: {e}");
        }
    });
    Ok(side)
}

fn show_or_create(app: &AppHandle, target: &ChatTarget, pet: PetBox, side: i8) -> tauri::Result<()> {
    let overlay = app.state::<Arc<Overlay>>().geometry();
    // Toạ độ overlay đổi sang toạ độ màn hình (logical) theo vị trí và DPI của cửa sổ overlay.
    let scale = overlay.screen.scale_factor;
    let to_screen = |(x, y): (f64, f64)| {
        LogicalPosition::new(f64::from(overlay.window.x) / scale + x, f64::from(overlay.window.y) / scale + y)
    };
    let title = title(&target.name);
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.set_title(&title)?;
        app.emit_to(WINDOW_LABEL, events::CHAT_TARGET, target)?;
        window.unminimize()?;
        window.set_position(to_screen(place(&overlay.screen, pet, outer_size(&window)?, side)))?;
        window.show()?;
        return window.set_focus();
    }
    let guess = (WIDTH + FRAME_GUESS.0, HEIGHT + FRAME_GUESS.1);
    let start = to_screen(place(&overlay.screen, pet, guess, side));
    let window = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("chat.html".into()))
        .title(title)
        .inner_size(WIDTH, HEIGHT)
        // Không hẹp hơn cảnh ở đầu khung chat (ChatScene.tsx rộng 360), để pet đứng đúng trên đồi.
        .min_inner_size(WIDTH, 360.0)
        .position(start.x, start.y)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .background_color(Color(13, 22, 40, 255))
        .visible(false)
        .build()?;
    crate::app_icon::set_small_icon(&window);
    // Cỡ thật cả viền và thanh tiêu đề chỉ đo được sau khi tạo: đặt lại cho trọn trong vùng làm việc.
    window.set_position(to_screen(place(&overlay.screen, pet, outer_size(&window)?, side)))?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            handle.state::<Chat>().set_target(None);
            // Pet đang chat lại đi lại như thường.
            if let Err(e) = handle.emit_to(overlay::LABEL, events::CHAT_CLOSED, ()) {
                log!("Không báo được đã đóng chat: {e}");
            }
        }
    });
    window.show()?;
    window.set_focus()
}

/// Đổi tiêu đề cửa sổ chat đang mở theo ngôn ngữ đang dùng.
pub fn retitle(app: &AppHandle) {
    let (Some(window), Some(target)) = (app.get_webview_window(WINDOW_LABEL), app.state::<Chat>().target()) else {
        return;
    };
    if let Err(e) = window.set_title(&title(&target.name)) {
        log!("Không đổi được tiêu đề cửa sổ chat: {e}");
    }
}

/// Cỡ cửa sổ cả viền và thanh tiêu đề (CSS pixel).
fn outer_size(window: &WebviewWindow) -> tauri::Result<(f64, f64)> {
    let size = window.outer_size()?.to_logical::<f64>(window.scale_factor()?);
    Ok((size.width, size.height))
}
