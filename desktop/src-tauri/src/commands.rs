//! Tauri commands gọi từ frontend qua `invoke()`.

use crate::activity::{Activity, StatsView};
use crate::autostart;
use crate::chat::{self, Chat, ChatTarget};
use crate::cursor::CursorInterest;
use crate::error::{AppError, AppResult};
use crate::events;
use crate::overlay::{self, Overlay, Rect, ScreenInfo};
use crate::settings::{Settings, SettingsStore};
use crate::storage::Storage;
use crate::weather::{self, CityResult, Report, Weather};
use crate::window_list::{WindowList, Windows};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

#[tauri::command]
pub fn screen_info(overlay: State<'_, Overlay>) -> ScreenInfo {
    overlay.geometry().screen
}

/// Pet bị kéo hoặc đi ra khỏi overlay tới điểm (`x`, `y`) (CSS pixel của overlay): điểm đó nằm trên
/// màn hình khác thì overlay sang màn hình đó, rồi báo qua event `screen-changed`. Là command async để
/// chạy ngoài main thread: đổi chỗ cửa sổ phải chờ main thread.
#[tauri::command]
pub async fn move_overlay(app: AppHandle, x: f64, y: f64) -> AppResult<()> {
    overlay::move_to(&app, x, y)
}

/// Các cửa sổ đang hiện (danh sách gửi lần gần nhất), để overlay vừa mở không phải chờ thay đổi.
#[tauri::command]
pub fn list_windows(windows: State<'_, Windows>) -> WindowList {
    windows.current()
}

/// `enabled = true`: chuột đi xuyên overlay xuống app bên dưới. Chỉ overlay được gọi.
#[tauri::command]
pub fn set_click_through(window: WebviewWindow, enabled: bool) -> AppResult<()> {
    if window.label() != overlay::LABEL {
        return Err(AppError::bad_request("Chỉ overlay được đổi click-through."));
    }
    window.set_ignore_cursor_events(enabled)?;
    Ok(())
}

/// Vùng quanh từng pet (CSS pixel của overlay) cần biết vị trí con trỏ; `null`: mọi chỗ (cursor.rs).
#[tauri::command]
pub fn set_cursor_interest(interest: State<'_, CursorInterest>, rects: Option<Vec<Rect>>) {
    interest.set(rects);
}

/// Pet ngủ, overlay dừng vòng lặp vẽ (`true`), hoặc thức dậy (`false`).
#[tauri::command]
pub fn set_resting(app: AppHandle, resting: bool) {
    overlay::set_resting(&app, resting);
}

/// Trạng thái thế giới pet đã lưu, `null` nếu chưa có.
#[tauri::command]
pub fn load_state(storage: State<'_, Storage>) -> AppResult<Option<Value>> {
    storage.load()
}

#[tauri::command]
pub fn save_state(storage: State<'_, Storage>, state: Value) -> AppResult<()> {
    storage.save(&state)
}

#[tauri::command]
pub fn get_settings(store: State<'_, SettingsStore>) -> Settings {
    store.get()
}

/// Lưu cài đặt rồi báo overlay áp dụng ngay. Trả về giá trị đã kẹp vào khoảng cho phép. Đổi thành
/// phố thì thời tiết cũ không còn đúng: báo overlay dùng thời tiết giả lập trong lúc hỏi thời tiết mới.
#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    store: State<'_, SettingsStore>,
    weather: State<'_, Weather>,
    activity: State<'_, Activity>,
    settings: Settings,
) -> AppResult<Settings> {
    let before = store.get().city;
    let settings = store.set(settings)?;
    activity.configure(&settings);
    // Tắt chat thì đóng luôn cửa sổ chat đang mở.
    if !settings.chat {
        if let Some(window) = app.get_webview_window(chat::WINDOW_LABEL) {
            window.close()?;
        }
    }
    app.emit_to(overlay::LABEL, events::SETTINGS_CHANGED, &settings)?;
    if settings.city != before {
        app.emit(events::WEATHER_CHANGED, weather.current(settings.city.as_ref()))?;
        weather.city_changed();
    }
    Ok(settings)
}

/// Thời tiết gần nhất của thành phố đang chọn; `null` nếu chưa chọn thành phố hoặc chưa lấy được lần nào.
#[tauri::command]
pub fn get_weather(store: State<'_, SettingsStore>, weather: State<'_, Weather>) -> Option<Report> {
    weather.current(store.get().city.as_ref())
}

/// Tìm thành phố theo tên cho ô Thành phố trong Cài đặt (Open-Meteo geocoding).
#[tauri::command]
pub async fn search_city(query: String) -> AppResult<Vec<CityResult>> {
    weather::search(&query).await
}

/// Click chuột phải vào pet: mở cửa sổ chat cạnh pet (`x`, `y`, `width`, `height`: khung của pet, CSS pixel
/// của overlay). Trả về phía của cửa sổ so với pet: -1 bên trái, 1 bên phải.
#[tauri::command]
pub fn open_chat(app: AppHandle, pet: String, name: String, x: f64, y: f64, width: f64, height: f64) -> AppResult<i8> {
    chat::open(&app, ChatTarget { pet, name }, chat::PetBox { x, y, width, height })
}

/// Câu gợi ý của hôm nay (`today` "2026-10-01" và `date` "Thứ Năm 01/10/2026" theo giờ máy); rỗng thì khung
/// chat dùng câu có sẵn.
#[tauri::command]
pub async fn chat_suggestions(
    chat: State<'_, Chat>,
    store: State<'_, SettingsStore>,
    today: String,
    date: String,
) -> AppResult<Vec<String>> {
    if !store.get().chat || today.len() > 10 || date.chars().count() > 40 {
        return Ok(Vec::new());
    }
    Ok(chat.suggestions(&today, &date).await)
}

/// Cửa sổ chat vừa mở: đang chat với con nào.
#[tauri::command]
pub fn chat_target(chat: State<'_, Chat>) -> Option<ChatTarget> {
    chat.target()
}

/// Gửi câu hỏi (đã kèm tính cách pet, giờ, vài lượt chat gần nhất), trả về câu trả lời.
#[tauri::command]
pub async fn send_chat(chat: State<'_, Chat>, store: State<'_, SettingsStore>, prompt: String) -> AppResult<String> {
    if !store.get().chat {
        return Err(AppError::bad_request("Chat với pet đang tắt trong Cài đặt."));
    }
    chat.send(&prompt).await
}

/// Mở link trong câu trả lời bằng trình duyệt.
#[tauri::command]
pub fn open_link(url: String) -> AppResult<()> {
    chat::open_link(&url)
}

/// Giờ ngồi máy đã lưu (tab Sức khoẻ trong Cài đặt).
#[tauri::command]
pub fn get_stats(activity: State<'_, Activity>) -> StatsView {
    activity.stats()
}

/// Xoá hết giờ ngồi máy đã lưu.
#[tauri::command]
pub fn clear_stats(activity: State<'_, Activity>) -> AppResult<()> {
    activity.clear()
}

#[tauri::command]
pub fn get_autostart() -> AppResult<bool> {
    autostart::is_enabled()
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> AppResult<()> {
    autostart::set_enabled(enabled)
}

/// Overlay gọi sau khi đã lưu trạng thái (xem `app::request_quit`).
#[tauri::command]
pub fn quit(app: AppHandle, activity: State<'_, Activity>) {
    activity.flush();
    app.exit(0);
}
