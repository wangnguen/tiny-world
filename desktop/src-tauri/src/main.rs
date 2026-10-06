// Ẩn cửa sổ console khi build release trên Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod app;
mod app_icon;
mod autostart;
mod chat;
mod commands;
mod cursor;
mod error;
mod events;
mod fullscreen;
mod gemini;
mod overlay;
mod preview;
mod settings;
mod storage;
mod tray;
mod update;
mod weather;
mod window_list;

fn main() {
    app::run();
}
