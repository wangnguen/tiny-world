//! Overlay TinyWorld vẽ bằng DirectComposition (không WebView2): mô phỏng pet (`sim`, chuyển từ packages/sim), vẽ
//! pet và hiệu ứng, chuột, khay, các luồng nền của `tinyworld-backend`. Cửa sổ Cài đặt và khung chat vẫn là Tauri +
//! React, chạy ở tiến trình riêng lúc cần (`ipc.rs`).

#[macro_use]
extern crate tinyworld_backend;

pub mod sim;

mod ambience;
mod app;
mod assets;
mod bubble;
mod clock;
mod commands;
mod dizzy;
mod ghost;
mod gfx;
mod interaction;
mod ipc;
mod long;
mod member;
mod random;
mod said;
mod screens;
mod sprites;
mod tag;
mod tray;
mod view;
mod weather;
mod window;

pub use app::{default_data_dir, run, Config};
pub use ipc::ENV as IPC_ENV;
