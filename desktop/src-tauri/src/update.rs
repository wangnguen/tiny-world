//! Command cập nhật cho Cài đặt. Hỏi GitHub, tải và kiểm tra bộ cài ở `tinyworld_backend::update`.

pub use tinyworld_backend::update::*;

use crate::app;
use crate::error::AppResult;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

/// Cài đặt vừa mở: có bản mới không (theo lần hỏi gần nhất).
#[tauri::command]
pub fn get_update(updater: State<'_, Arc<Updater>>) -> Option<UpdateInfo> {
    updater.latest()
}

/// Nút Kiểm tra bản mới: hỏi ngay; `None` là đang dùng bản mới nhất.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<Option<UpdateInfo>> {
    let updater = app.state::<Arc<Updater>>().inner().clone();
    updater.check_now(&app::host(&app)).await
}

/// Nút Cập nhật ngay: tải và kiểm tra bộ cài rồi thoát app; bộ cài chạy lúc app thoát hẳn (`launch_pending`).
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    let updater = app.state::<Arc<Updater>>().inner().clone();
    updater.install(&app::host(&app)).await?;
    app::request_quit(&app);
    Ok(())
}

/// App sắp thoát hẳn: có bộ cài đã tải thì chạy nó (cài đè, xong tự mở lại app).
pub fn launch_pending(app: &AppHandle) {
    if let Some(updater) = app.try_state::<Arc<Updater>>() {
        updater.launch_pending();
    }
}
