//! Command của cửa sổ Cài đặt và khung chat, gửi qua cổng giao diện (`ipc.rs`): cùng tên, cùng tham số với các Tauri
//! command cũ (`desktop/src/api.ts`). Command chờ mạng (tìm thành phố, chat, cập nhật) chạy trên runtime tokio và trả
//! lời khi xong; còn lại trả lời ngay trên vòng lặp chính.

use crate::app::{App, Message};
use crate::ipc::Writer;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::future::Future;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tinyworld_backend::activity::Activity;
use tinyworld_backend::chat::{self, Chat};
use tinyworld_backend::error::{AppError, AppResult};
use tinyworld_backend::events::{Event, Host};
use tinyworld_backend::settings::SettingsStore;
use tinyworld_backend::update::Updater;
use tinyworld_backend::weather::{self, Weather};
use tinyworld_backend::{autostart, i18n};

/// Dịch vụ dùng chung giữa vòng lặp chính và các tác vụ trả lời chậm.
pub struct Services {
    pub settings: Arc<SettingsStore>,
    pub weather: Arc<Weather>,
    pub activity: Arc<Activity>,
    pub updater: Arc<Updater>,
    pub chat: Arc<Chat>,
    pub runtime: tokio::runtime::Handle,
    pub tx: Sender<Message>,
}

/// Event của lúc tải bộ cài (tiến độ) gửi vào vòng lặp chính như event của luồng nền.
struct Forward(Sender<Message>);

impl Host for Forward {
    fn emit(&self, event: Event) {
        let _ = self.0.send(Message::Backend(event));
        crate::window::wake();
    }
}

fn arg<T: DeserializeOwned>(args: &Value, name: &str) -> AppResult<T> {
    serde_json::from_value(args.get(name).cloned().unwrap_or(Value::Null))
        .map_err(|e| AppError::bad_request(format!("{name}: {e}")))
}

fn ok(value: impl serde::Serialize) -> AppResult<Value> {
    serde_json::to_value(value).map_err(|e| AppError::internal(e.to_string()))
}

/// Chạy `task` trên runtime rồi trả lời.
fn later<F>(services: &Services, writer: Writer, id: u64, task: F)
where
    F: Future<Output = AppResult<Value>> + Send + 'static,
{
    services.runtime.spawn(async move {
        writer.reply(id, task.await);
    });
}

pub fn call(app: &mut App, id: u64, name: &str, args: Value, writer: Writer) {
    let services = app.services();
    let result: AppResult<Value> = match name {
        "get_settings" => ok(app.settings_snapshot()),
        "set_settings" => arg(&args, "settings").and_then(|s| app.set_settings(s)).and_then(ok),
        "get_autostart" => autostart::is_enabled().and_then(ok),
        "set_autostart" => arg(&args, "enabled").and_then(autostart::set_enabled).and_then(ok),
        "get_weather" => ok(services.weather.current(services.settings.get().city.as_ref())),
        "get_stats" => ok(services.activity.stats()),
        "clear_stats" => services.activity.clear().and_then(ok),
        "idle_ms" => ok(tinyworld_backend::activity::idle_ms()),
        "preview_weather" => arg(&args, "preview").and_then(|p| app.preview_weather(p)).and_then(ok),
        "preview_ghost" => app.preview_ghost().and_then(ok),
        "get_preview" => ok(app.preview_state()),
        "chat_target" => ok(app.chat_target()),
        "open_link" => arg::<String>(&args, "url").and_then(|url| chat::open_link(&url)).and_then(ok),
        "open_log_folder" => {
            let opened = tinyworld_backend::log::dir().is_some_and(|dir| chat::shell_open(&dir.to_string_lossy()));
            if opened {
                ok(())
            } else {
                Err(AppError::internal(i18n::t("errors.logFolder")))
            }
        }
        "get_update" => ok(services.updater.latest()),
        "quit" => {
            let _ = services.tx.send(Message::Quit);
            crate::window::wake();
            ok(())
        }
        "search_city" => {
            let query: String = match arg(&args, "query") {
                Ok(query) => query,
                Err(e) => return writer.reply(id, Err(e)),
            };
            return later(&services, writer, id, async move { weather::search(&query).await.and_then(ok) });
        }
        // Câu gợi ý của hôm nay (`today` "2026-10-01" và `date` "Thứ Năm 01/10/2026" theo giờ máy); rỗng thì khung chat
        // dùng câu có sẵn.
        "chat_suggestions" => {
            let (today, date): (String, String) = match (arg(&args, "today"), arg(&args, "date")) {
                (Ok(today), Ok(date)) => (today, date),
                (Err(e), _) | (_, Err(e)) => return writer.reply(id, Err(e)),
            };
            let (chat, on) = (services.chat.clone(), services.settings.get().chat);
            return later(&services, writer, id, async move {
                if !on || today.len() > 10 || date.chars().count() > 40 {
                    return ok(Vec::<String>::new());
                }
                ok(chat.suggestions(&today, &date).await)
            });
        }
        "send_chat" => {
            let prompt: String = match arg(&args, "prompt") {
                Ok(prompt) => prompt,
                Err(e) => return writer.reply(id, Err(e)),
            };
            let (chat, on) = (services.chat.clone(), services.settings.get().chat);
            return later(&services, writer, id, async move {
                if !on {
                    return Err(chat::chat_off());
                }
                chat.send(&prompt).await.and_then(ok)
            });
        }
        "check_update" => {
            let (updater, host) = (services.updater.clone(), Forward(services.tx.clone()));
            return later(&services, writer, id, async move { updater.check_now(&host).await.and_then(ok) });
        }
        // Tải và kiểm tra bộ cài rồi thoát app; bộ cài chạy lúc app thoát hẳn (`Updater::launch_pending`).
        "install_update" => {
            let (updater, tx) = (services.updater.clone(), services.tx.clone());
            let host = Forward(tx.clone());
            return later(&services, writer, id, async move {
                updater.install(&host).await?;
                let _ = tx.send(Message::Quit);
                crate::window::wake();
                ok(())
            });
        }
        #[cfg(debug_assertions)]
        "debug_state" => Ok(app.debug_state()),
        #[cfg(debug_assertions)]
        "debug_pet" => app.debug_pet(&args),
        #[cfg(debug_assertions)]
        "debug_open" => app.debug_open(&args),
        other => Err(AppError::bad_request(format!("Không có command {other}."))),
    };
    writer.reply(id, result);
}
