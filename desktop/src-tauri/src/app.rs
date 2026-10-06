use crate::activity::{self, Activity};
use crate::chat::Chat;
use crate::cursor::CursorInterest;
use crate::preview::{self, Preview};
use crate::update::{self, Updater};
use crate::settings::SettingsStore;
use crate::storage::Storage;
use crate::weather::{self, Weather};
use crate::window_list::{self, Windows};
use crate::{commands, cursor, events, fullscreen, overlay, tray};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// Overlay có chừng này thời gian để lưu trạng thái trước khi app tự thoát.
const QUIT_TIMEOUT: Duration = Duration::from_millis(1500);

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Mở app lần nữa khi đang chạy: hiện lại pet nếu đang ẩn.
            overlay::set_user_hidden(app, false);
        }))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let settings = SettingsStore::load(&data_dir);
            app.manage(Activity::load(&data_dir, &settings.get()));
            app.manage(settings);
            app.manage(Weather::load(&data_dir));
            app.manage(Storage::new(data_dir));
            app.manage(Windows::default());
            app.manage(CursorInterest::default());
            app.manage(Chat::default());
            app.manage(Preview::default());
            app.manage(Updater::default());
            overlay::setup(app.handle())?;
            overlay::watch(app.handle().clone());
            tray::setup(app.handle())?;
            cursor::spawn(app.handle().clone());
            fullscreen::spawn(app.handle().clone());
            window_list::spawn(app.handle().clone());
            weather::spawn(app.handle().clone());
            activity::spawn(app.handle().clone());
            update::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::screen_info,
            commands::move_overlay,
            commands::list_windows,
            commands::set_click_through,
            commands::set_cursor_interest,
            commands::set_resting,
            commands::load_state,
            commands::save_state,
            commands::get_settings,
            commands::set_settings,
            commands::get_weather,
            commands::search_city,
            commands::get_stats,
            commands::clear_stats,
            commands::idle_ms,
            preview::preview_weather,
            preview::preview_ghost,
            preview::report_preview,
            preview::get_preview,
            update::get_update,
            update::check_update,
            update::install_update,
            commands::open_chat,
            commands::chat_suggestions,
            commands::chat_target,
            commands::send_chat,
            commands::open_link,
            commands::get_autostart,
            commands::set_autostart,
            commands::quit,
        ])
        .build(tauri::generate_context!())
        .expect("không khởi động được ứng dụng")
        .run(|app, event| {
            // Thoát theo đường nào cũng vậy: có bộ cài bản mới đã tải thì chạy nó.
            if let tauri::RunEvent::Exit = event {
                update::launch_pending(app);
            }
        });
}

/// Tray bấm Thoát: nhờ overlay lưu trạng thái, overlay lưu xong thì gọi command `quit`. Overlay không
/// trả lời (trang lỗi...) thì vẫn thoát sau `QUIT_TIMEOUT`.
pub fn request_quit(app: &AppHandle) {
    app.state::<Activity>().flush();
    if app.emit_to(overlay::LABEL, events::QUIT_REQUESTED, ()).is_err() {
        app.exit(0);
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(QUIT_TIMEOUT);
        app.exit(0);
    });
}
