//! Phần của TinyWorld không dính tới cách vẽ: cài đặt, log, chữ hai ngôn ngữ, thời tiết, giờ ngồi máy, chat,
//! cập nhật, theo dõi cửa sổ và màn hình của Windows. Shell Tauri (Cài đặt, Chat) và overlay cùng dùng.
//!
//! Các luồng nền gửi event qua `events::Host` chứ không gọi thẳng cửa sổ nào.

#[macro_use]
pub mod log;

pub mod activity;
pub mod autostart;
pub mod chat;
pub mod error;
pub mod events;
pub mod fullscreen;
pub mod gemini;
pub mod geometry;
pub mod i18n;
pub mod preview;
pub mod settings;
pub mod storage;
pub mod update;
pub mod weather;
pub mod window_list;
