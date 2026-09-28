// Ẩn cửa sổ console khi build release trên Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod commands;
mod cursor;
mod error;
mod overlay;
mod storage;
mod tray;

fn main() {
    app::run();
}
