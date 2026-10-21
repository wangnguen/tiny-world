//! Chạy riêng overlay để thử nhanh, không phải build cả shell Tauri. Có `tinyworld.exe` đã build cạnh exe này (cùng
//! thư mục `target/<profile>`) thì Cài đặt và khung chat mở bằng nó.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let Some(data_dir) = tinyworld_native::default_data_dir() else {
        eprintln!("tinyworld-native: không có thư mục APPDATA");
        std::process::exit(1);
    };
    let ui_exe = std::env::current_exe().ok().map(|exe| exe.with_file_name("tinyworld.exe")).filter(|exe| exe.exists());
    let config = tinyworld_native::Config { version: env!("CARGO_PKG_VERSION").to_owned(), data_dir, ui_exe };
    if let Err(err) = tinyworld_native::run(config) {
        eprintln!("tinyworld-native: {err:#}");
        std::process::exit(1);
    }
}
