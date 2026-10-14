//! Icon ở system tray. Overlay không có nút trên taskbar nên mọi điều khiển của app nằm ở đây.

use crate::i18n::{self, Lang};
use crate::{app, app_icon, events, overlay, settings};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};

const PAUSE: &str = "pause";
const TOGGLE: &str = "toggle";
const SETTINGS: &str = "settings";
const QUIT: &str = "quit";
#[cfg(debug_assertions)]
const DEVTOOLS: &str = "devtools";
/// Các lựa chọn Im lặng: id mục trong menu, khoá chữ, thời gian (`None`: tới khi tắt).
const QUIET: [(&str, &str, Option<Duration>); 3] = [
    ("quiet-30", "tray.quiet30", Some(Duration::from_secs(30 * 60))),
    ("quiet-60", "tray.quiet60", Some(Duration::from_secs(60 * 60))),
    ("quiet-on", "tray.quietOn", None),
];

/// Các mục của menu, giữ lại để đổi chữ khi người dùng đổi ngôn ngữ (`retitle`).
struct Items {
    pause: CheckMenuItem<Wry>,
    quiet: Submenu<Wry>,
    /// Theo thứ tự của `QUIET`.
    quiet_items: Vec<CheckMenuItem<Wry>>,
    toggle: MenuItem<Wry>,
    settings: MenuItem<Wry>,
    quit: MenuItem<Wry>,
}

/// Im lặng đang bật (chỉ số trong `QUIET`). `generation` tăng mỗi lần đổi, để bộ hẹn giờ của lựa chọn cũ
/// biết mình hết hiệu lực.
#[derive(Default)]
struct Quiet {
    choice: Mutex<Option<usize>>,
    generation: AtomicU64,
}

/// Chữ của các mục: Tạm dừng, Im lặng, Ẩn / hiện pet, Cài đặt…, Thoát.
fn labels(lang: Lang) -> [String; 5] {
    ["tray.pause", "tray.quiet", "tray.toggle", "tray.settings", "tray.quit"].map(|key| lang.t(key))
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let lang = i18n::current();
    let [pause, quiet, toggle, settings, quit] = labels(lang);
    let pause = CheckMenuItem::with_id(app, PAUSE, pause, true, false, None::<&str>)?;
    let quiet_items = QUIET
        .iter()
        .map(|(id, key, _)| CheckMenuItem::with_id(app, *id, lang.t(key), true, false, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let quiet_refs: Vec<&dyn tauri::menu::IsMenuItem<Wry>> =
        quiet_items.iter().map(|item| item as &dyn tauri::menu::IsMenuItem<Wry>).collect();
    let quiet = Submenu::with_items(app, quiet, true, &quiet_refs)?;
    let toggle = MenuItem::with_id(app, TOGGLE, toggle, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, SETTINGS, settings, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, quit, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&pause, &quiet, &toggle, &settings, &separator, &quit])?;
    // Overlay để chuột đi xuyên nên không bấm F12 được, mở DevTools từ đây khi chạy dev.
    #[cfg(debug_assertions)]
    menu.insert(
        &MenuItem::with_id(app, DEVTOOLS, "DevTools", true, None::<&str>)?,
        0,
    )?;
    app.manage(Items {
        pause: pause.clone(),
        quiet,
        quiet_items,
        toggle,
        settings,
        quit,
    });
    app.manage(Quiet::default());

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("TinyWorld")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            id if QUIET.iter().any(|(quiet, ..)| *quiet == id) => choose_quiet(app, id),
            PAUSE => {
                // Menu tự đảo dấu tick khi bấm; đặt lại theo trạng thái thật cho chắc.
                let paused = overlay::toggle_paused(app);
                if let Err(e) = pause.set_checked(paused) {
                    log!("Không cập nhật được mục Tạm dừng: {e}");
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
    let lang = i18n::current();
    let [pause, quiet, toggle, settings, quit] = labels(lang);
    let result = items
        .pause
        .set_text(pause)
        .and_then(|()| items.quiet.set_text(quiet))
        .and_then(|()| {
            items
                .quiet_items
                .iter()
                .zip(QUIET)
                .try_for_each(|(item, (_, key, _))| item.set_text(lang.t(key)))
        })
        .and_then(|()| items.toggle.set_text(toggle))
        .and_then(|()| items.settings.set_text(settings))
        .and_then(|()| items.quit.set_text(quit));
    if let Err(e) = result {
        log!("Không đổi được chữ trong menu khay: {e}");
    }
}

/// Bấm một lựa chọn Im lặng: đang bật đúng lựa chọn đó thì tắt, không thì bật nó (tính giờ lại từ đầu).
fn choose_quiet(app: &AppHandle, id: &str) {
    let Some(index) = QUIET.iter().position(|(quiet, ..)| *quiet == id) else {
        return;
    };
    let current = *app.state::<Quiet>().choice.lock().unwrap_or_else(PoisonError::into_inner);
    set_quiet(app, (current != Some(index)).then_some(index));
}

/// Bật Im lặng theo lựa chọn `choice` (`None` là tắt): đánh dấu menu, báo overlay, hẹn giờ tự tắt.
fn set_quiet(app: &AppHandle, choice: Option<usize>) {
    let state = app.state::<Quiet>();
    *state.choice.lock().unwrap_or_else(PoisonError::into_inner) = choice;
    let generation = state.generation.fetch_add(1, Ordering::SeqCst) + 1;
    if let Some(items) = app.try_state::<Items>() {
        for (i, item) in items.quiet_items.iter().enumerate() {
            // Menu tự đảo dấu tick khi bấm; đặt lại theo trạng thái thật.
            if let Err(e) = item.set_checked(choice == Some(i)) {
                log!("Không cập nhật được mục Im lặng: {e}");
            }
        }
    }
    if let Err(e) = app.emit_to(overlay::LABEL, events::QUIET, choice.is_some()) {
        log!("Không báo được Im lặng cho overlay: {e}");
    }
    if let Some(duration) = choice.and_then(|i| QUIET[i].2) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(duration).await;
            if app.state::<Quiet>().generation.load(Ordering::SeqCst) == generation {
                set_quiet(&app, None);
            }
        });
    }
}
