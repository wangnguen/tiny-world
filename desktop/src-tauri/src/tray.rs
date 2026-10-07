//! Icon ở system tray. Overlay không có nút trên taskbar nên mọi điều khiển của app nằm ở đây.

use crate::i18n::{self, Lang};
use crate::{app, app_icon, overlay, settings};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry};

const PAUSE: &str = "pause";
const TOGGLE: &str = "toggle";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";
#[cfg(debug_assertions)]
const DEVTOOLS: &str = "devtools";

/// Các mục của menu, giữ lại để đổi chữ khi người dùng đổi ngôn ngữ (`retitle`).
struct Items {
    pause: CheckMenuItem<Wry>,
    toggle: MenuItem<Wry>,
    settings: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

/// Chữ của các mục: Tạm dừng, Ẩn / hiện pet, Cài đặt…, Thoát.
fn labels(lang: Lang) -> [String; 4] {
    ["tray.pause", "tray.toggle", "tray.settings", "tray.quit"].map(|key| lang.t(key))
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let [pause, toggle, settings, quit] = labels(i18n::current());
    let pause = CheckMenuItem::with_id(app, PAUSE, pause, true, false, None::<&str>)?;
    let toggle = MenuItem::with_id(app, TOGGLE, toggle, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, SETTINGS, settings, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, quit, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&pause, &toggle, &settings, &separator, &quit])?;
    // Overlay để chuột đi xuyên nên không bấm F12 được, mở DevTools từ đây khi chạy dev.
    #[cfg(debug_assertions)]
    menu.insert(
        &MenuItem::with_id(app, DEVTOOLS, "DevTools", true, None::<&str>)?,
        0,
    )?;
    app.manage(Items {
        pause: pause.clone(),
        toggle,
        settings,
        quit,
    });

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

/// Đổi chữ trong menu theo ngôn ngữ đang dùng.
pub fn retitle(app: &AppHandle) {
    let Some(items) = app.try_state::<Items>() else {
        return;
    };
    let [pause, toggle, settings, quit] = labels(i18n::current());
    let result = items
        .pause
        .set_text(pause)
        .and_then(|()| items.toggle.set_text(toggle))
        .and_then(|()| items.settings.set_text(settings))
        .and_then(|()| items.quit.set_text(quit));
    if let Err(e) = result {
        eprintln!("Không đổi được chữ trong menu khay: {e}");
    }
}
