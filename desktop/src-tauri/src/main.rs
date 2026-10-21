// Ẩn cửa sổ console khi build release trên Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
extern crate tinyworld_backend;

// Phần không dính tới Tauri nằm ở desktop/backend; đặt tên ở đây để các module dùng `crate::weather`... như cũ.
use tinyworld_backend::{
    activity, autostart, error, events, fullscreen, geometry, i18n, log, storage, weather, window_list,
};

mod app;
mod app_icon;
mod chat;
mod commands;
mod cursor;
mod overlay;
mod preview;
mod settings;
mod tray;
mod update;

fn main() {
    app::run();
}
