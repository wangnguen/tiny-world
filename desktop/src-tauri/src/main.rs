// Ẩn cửa sổ console khi build release trên Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate tinyworld_backend;

mod app_icon;
mod ui;

/// Không tham số: overlay (pet, khay, các luồng nền), xem `tinyworld_native`. `--ui`: cửa sổ Cài đặt và khung chat,
/// do overlay chạy lúc cần.
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--ui") {
        ui::run();
        return;
    }
    let Some(data_dir) = tinyworld_native::default_data_dir() else {
        return;
    };
    let config = tinyworld_native::Config {
        version: env!("TINYWORLD_VERSION").to_owned(),
        data_dir,
        ui_exe: std::env::current_exe().ok(),
    };
    if let Err(e) = tinyworld_native::run(config) {
        log!("Overlay dừng vì lỗi: {e:#}");
    }
}
