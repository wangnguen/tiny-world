// Ẩn cửa sổ console khi build release trên Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autostart;
mod commands;
mod cursor;
mod error;
mod events;
mod fullscreen;
mod overlay;
mod settings;
mod storage;
mod tray;

fn main() {
    app::run();
}
