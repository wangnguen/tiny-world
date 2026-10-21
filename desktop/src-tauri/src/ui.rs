//! Tiến trình giao diện (`tinyworld.exe --ui`): cửa sổ Cài đặt và khung chat (React). Overlay chạy tiến trình này lúc
//! cần mở một trong hai cửa sổ, đóng hết cửa sổ thì tiến trình thoát để đỡ tốn RAM. Mọi command của frontend đi qua
//! một command `call` rồi gửi sang overlay (`native/src/ipc.rs`); event của overlay gửi lại cho đúng cửa sổ.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use tauri::window::Color;
use tauri::{
    AppHandle, Emitter, LogicalPosition, Manager, State, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tinyworld_backend::chat::{self, ChatTarget, PetBox};
use tinyworld_backend::error::{AppError, AppResult};
use tinyworld_backend::events;
use tinyworld_backend::geometry::ScreenInfo;
use tokio::sync::oneshot;

const SETTINGS: &str = "settings";
const CHAT: &str = "chat";
/// Mã lỗi của `AppError` (khớp `ErrorCode` trong packages/core).
const CODES: [&str; 7] = ["BAD_REQUEST", "NO_WINDOW", "INTERNAL", "OFFLINE", "BUSY", "UNAVAILABLE", "TIMEOUT"];

/// Kết nối tới overlay: gửi command, chờ trả lời theo `id`.
struct Bridge {
    stream: Mutex<TcpStream>,
    next: AtomicU64,
    pending: Mutex<HashMap<u64, oneshot::Sender<AppResult<Value>>>>,
}

impl Bridge {
    fn send(&self, message: &Value) -> AppResult<()> {
        let mut line = message.to_string();
        line.push('\n');
        let mut stream = self.stream.lock().unwrap_or_else(PoisonError::into_inner);
        stream.write_all(line.as_bytes()).map_err(|e| AppError::internal(format!("Không gửi được cho overlay: {e}")))
    }

    async fn call(&self, name: String, args: Value) -> AppResult<Value> {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap_or_else(PoisonError::into_inner).insert(id, tx);
        self.send(&json!({ "id": id, "call": name, "args": args }))?;
        rx.await.map_err(|_| AppError::internal("Overlay đã tắt."))?
    }

    fn resolve(&self, id: u64, result: AppResult<Value>) {
        if let Some(tx) = self.pending.lock().unwrap_or_else(PoisonError::into_inner).remove(&id) {
            let _ = tx.send(result);
        }
    }
}

/// Mọi command của frontend (`api.ts`): cùng tên, cùng tham số như trước, overlay làm và trả lời.
#[tauri::command]
async fn call(bridge: State<'_, Arc<Bridge>>, name: String, args: Option<Value>) -> AppResult<Value> {
    bridge.call(name, args.unwrap_or(Value::Null)).await
}

/// Nối về overlay theo `TINYWORLD_IPC` rồi chạy tới khi đóng hết cửa sổ (hoặc overlay tắt).
pub fn run() {
    let Some(stream) = connect() else {
        log!("Không nối được về overlay, thoát.");
        return;
    };
    let Ok(read) = stream.try_clone() else {
        return;
    };
    let bridge = Arc::new(Bridge { stream: Mutex::new(stream), next: AtomicU64::new(1), pending: Mutex::default() });
    tauri::Builder::default()
        .setup({
            let bridge = bridge.clone();
            move |app| {
                app.manage(bridge.clone());
                let handle = app.handle().clone();
                std::thread::spawn(move || listen(handle, read, bridge));
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![call])
        .build(tauri::generate_context!())
        .expect("không khởi động được cửa sổ giao diện")
        .run(|_app, _event| {});
}

fn connect() -> Option<TcpStream> {
    let value = std::env::var(tinyworld_native::IPC_ENV).ok()?;
    let (port, token) = value.split_once(' ')?;
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port.parse::<u16>().ok()?)).ok()?;
    let hello = format!("{}\n", json!({ "hello": token }));
    stream.write_all(hello.as_bytes()).ok()?;
    Some(stream)
}

/// Đọc tin của overlay: trả lời command, event, mở/đóng cửa sổ, thoát. Mất kết nối (overlay tắt) thì thoát luôn.
fn listen(app: AppHandle, read: TcpStream, bridge: Arc<Bridge>) {
    for line in BufReader::new(read).lines() {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(id) = message["id"].as_u64() {
            let result = if message.get("err").is_some() { Err(error(&message["err"])) } else { Ok(message["ok"].clone()) };
            bridge.resolve(id, result);
        } else if let Some(name) = message["event"].as_str() {
            let payload = message["payload"].clone();
            let result = match message["to"].as_str() {
                Some(label) => app.emit_to(label, name, payload),
                None => app.emit(name, payload),
            };
            if let Err(e) = result {
                log!("Không gửi được event {name}: {e}");
            }
        } else if message["open"] == SETTINGS {
            if let Err(e) = open_settings(&app) {
                log!("Không mở được cửa sổ cài đặt: {e}");
            }
        } else if message["open"] == CHAT {
            if let Err(e) = open_chat(&app, &message, &bridge) {
                log!("Không mở được cửa sổ chat: {e}");
            }
        } else if message["close"] == CHAT {
            if let Some(window) = app.get_webview_window(CHAT) {
                let _ = window.close();
            }
        } else if let Some(title) = message["title"].as_str() {
            if let Some(window) = app.get_webview_window(CHAT) {
                let _ = window.set_title(title);
            }
        } else if message["quit"] == true {
            break;
        }
    }
    app.exit(0);
}

fn error(value: &Value) -> AppError {
    let code = value["code"].as_str().unwrap_or_default();
    let code = CODES.iter().find(|c| **c == code).copied().unwrap_or("INTERNAL");
    AppError::new(code, value["message"].as_str().unwrap_or_default())
}

/// Mở cửa sổ Cài đặt, đang mở thì đưa lên trước. Nền và thanh tiêu đề tối, trùng màu trang (settings.css) để lúc mở
/// không bị loé trắng.
fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    let window = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings.html".into()))
        .title("TinyWorld")
        .inner_size(440.0, 740.0)
        .resizable(false)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .background_color(Color(13, 22, 40, 255))
        .center()
        .focused(true)
        .build()?;
    crate::app_icon::set_small_icon(&window);
    Ok(())
}

/// Cỡ cửa sổ cả viền và thanh tiêu đề (CSS pixel).
fn outer_size(window: &WebviewWindow) -> tauri::Result<(f64, f64)> {
    let size = window.outer_size()?.to_logical::<f64>(window.scale_factor()?);
    Ok((size.width, size.height))
}

/// Mở khung chat cạnh pet. Đang mở thì đổi sang pet đó, dời tới cạnh nó và đưa lên trước. Chỗ đặt tính theo màn hình
/// của overlay (`screen`, góc trên trái `origin` theo pixel vật lý) và khung của pet (`pet`, CSS pixel của overlay).
fn open_chat(app: &AppHandle, message: &Value, bridge: &Arc<Bridge>) -> tauri::Result<()> {
    let parse = |key: &str| message.get(key).cloned().unwrap_or(Value::Null);
    let (Ok(target), Ok(screen), Ok(pet), Ok(origin)) = (
        serde_json::from_value::<ChatTarget>(parse("target")),
        serde_json::from_value::<ScreenInfo>(parse("screen")),
        serde_json::from_value::<PetBox>(parse("pet")),
        serde_json::from_value::<(i32, i32)>(parse("origin")),
    ) else {
        log!("Lệnh mở chat sai dạng.");
        return Ok(());
    };
    let side = message["side"].as_i64().unwrap_or(1) as i8;
    let title = message["title"].as_str().unwrap_or("TinyWorld").to_owned();
    // Toạ độ overlay đổi sang toạ độ màn hình (logical) theo vị trí và DPI của cửa sổ overlay.
    let scale = screen.scale_factor;
    let to_screen =
        |(x, y): (f64, f64)| LogicalPosition::new(f64::from(origin.0) / scale + x, f64::from(origin.1) / scale + y);
    if let Some(window) = app.get_webview_window(CHAT) {
        window.set_title(&title)?;
        app.emit_to(CHAT, events::CHAT_TARGET, &target)?;
        window.unminimize()?;
        window.set_position(to_screen(chat::place(&screen, pet, outer_size(&window)?, side)))?;
        window.show()?;
        return window.set_focus();
    }
    let guess = (chat::WIDTH + chat::FRAME_GUESS.0, chat::HEIGHT + chat::FRAME_GUESS.1);
    let start = to_screen(chat::place(&screen, pet, guess, side));
    let window = WebviewWindowBuilder::new(app, CHAT, WebviewUrl::App("chat.html".into()))
        .title(title)
        .inner_size(chat::WIDTH, chat::HEIGHT)
        // Không hẹp hơn cảnh ở đầu khung chat (ChatScene.tsx rộng 360), để pet đứng đúng trên đồi.
        .min_inner_size(chat::WIDTH, 360.0)
        .position(start.x, start.y)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .background_color(Color(13, 22, 40, 255))
        .visible(false)
        .build()?;
    crate::app_icon::set_small_icon(&window);
    // Cỡ thật cả viền và thanh tiêu đề chỉ đo được sau khi tạo: đặt lại cho trọn trong vùng làm việc.
    window.set_position(to_screen(chat::place(&screen, pet, outer_size(&window)?, side)))?;
    let bridge = bridge.clone();
    window.on_window_event(move |event| {
        // Pet đang chat lại đi lại như thường.
        if matches!(event, WindowEvent::Destroyed) {
            if let Err(e) = bridge.send(&json!({ "closed": CHAT })) {
                log!("Không báo được đã đóng chat: {}", e.message);
            }
        }
    });
    window.show()?;
    window.set_focus()
}
