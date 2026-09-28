use crate::storage::Storage;
use crate::{commands, cursor, overlay, tray};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Mở app lần nữa khi đang chạy: hiện lại pet nếu đang ẩn.
            overlay::set_visible(app, true);
        }))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            app.manage(Storage::new(data_dir));
            overlay::setup(app.handle())?;
            tray::setup(app.handle())?;
            cursor::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::screen_info,
            commands::set_click_through,
            commands::load_state,
            commands::save_state,
        ])
        .run(tauri::generate_context!())
        .expect("không khởi động được ứng dụng");
}
