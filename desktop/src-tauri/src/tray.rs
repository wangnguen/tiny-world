//! Icon ở system tray. Overlay không có nút trên taskbar nên mọi điều khiển của app nằm ở đây.

use crate::{app, app_icon, overlay, settings};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::AppHandle;

const PAUSE: &str = "pause";
const TOGGLE: &str = "toggle";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";
#[cfg(debug_assertions)]
const DEVTOOLS: &str = "devtools";

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let pause = CheckMenuItem::with_id(app, PAUSE, "Tạm dừng", true, false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, TOGGLE, "Ẩn / hiện pet", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, SETTINGS, "Cài đặt…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Thoát", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&pause, &toggle, &settings, &separator, &quit])?;
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
        .on_menu_event(move |app, event| match event.id().as_ref() {
            PAUSE => {
                // Menu tự đảo dấu tick khi bấm; đặt lại theo trạng thái thật cho chắc.
                let paused = overlay::toggle_paused(app);
                if let Err(e) = pause.set_checked(paused) {
                    eprintln!("Không cập nhật được mục Tạm dừng: {e}");
                }
            }
            TOGGLE => overlay::toggle(app),
            SETTINGS => settings::open_window(app),
            QUIT => app::request_quit(app),
            #[cfg(debug_assertions)]
            DEVTOOLS => {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window(overlay::LABEL) {
                    window.open_devtools();
                }
            }
            _ => {}
        });
    if let Some(icon) = app_icon::tray(app) {
        tray = tray.icon(icon);
    }
    tray.build(app)?;
    Ok(())
}
