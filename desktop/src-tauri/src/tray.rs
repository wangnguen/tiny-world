//! Icon ở system tray. Overlay không có nút trên taskbar nên mọi điều khiển của app nằm ở đây.

use crate::overlay;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

const TOGGLE: &str = "toggle";
const QUIT: &str = "quit";
#[cfg(debug_assertions)]
const DEVTOOLS: &str = "devtools";

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, TOGGLE, "Ẩn / hiện pet", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Thoát", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;
    // Overlay để chuột đi xuyên nên không bấm F12 được, mở DevTools từ đây khi chạy dev.
    #[cfg(debug_assertions)]
    menu.insert(
        &MenuItem::with_id(app, DEVTOOLS, "Mở DevTools", true, None::<&str>)?,
        0,
    )?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("TinyWorld")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TOGGLE => overlay::toggle(app),
            QUIT => app.exit(0),
            #[cfg(debug_assertions)]
            DEVTOOLS => {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window(overlay::LABEL) {
                    window.open_devtools();
                }
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
