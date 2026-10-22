//! Tiến trình giao diện (`tinyworld.exe --ui`): cửa sổ Cài đặt và khung chat (React). Overlay chạy tiến trình này lúc
//! cần mở hay dựng sẵn (`warm`) một trong hai cửa sổ. Đóng cửa sổ thì chỉ ẩn để lần mở sau hiện ngay; không còn cửa sổ
//! nào hiện trong `LINGER` thì tiến trình thoát để đỡ tốn RAM. Mọi command của frontend đi qua một command `call` rồi
//! gửi sang overlay (`native/src/ipc.rs`); event của overlay gửi lại cho đúng cửa sổ.

use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tauri::window::Color;
use tauri::{
    AppHandle, Emitter, LogicalPosition, Manager, State, Theme, Webview, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tinyworld_backend::chat::{self, ChatTarget, PetBox};
use tinyworld_backend::error::{AppError, AppResult};
use tinyworld_backend::events;
use tinyworld_backend::geometry::ScreenInfo;
use tokio::sync::oneshot;

const SETTINGS: &str = "settings";
const CHAT: &str = "chat";
/// Không còn cửa sổ nào hiện thì chờ chừng này mới thoát: mở lại Cài đặt hay khung chat trong lúc đó thì hiện ngay, khỏi
/// chờ WebView2 khởi động (gần một giây).
const LINGER: Duration = Duration::from_secs(60);
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

/// Cửa sổ nào đang ẩn, hiện: để biết lúc nào thoát.
#[derive(Default)]
struct Lifetime {
    /// Tăng mỗi lần ẩn, hiện, dựng sẵn cửa sổ: lần hẹn thoát (`exit_later`) chỉ còn hiệu lực nếu từ lúc hẹn không có gì
    /// đổi.
    changes: AtomicU64,
    /// Cửa sổ dựng sẵn chưa hiện lần nào.
    warm: Mutex<HashSet<String>>,
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
                app.manage(Lifetime::default());
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

/// Đọc tin của overlay: trả lời command, event, mở/dựng sẵn/đóng cửa sổ, thoát. Mất kết nối (overlay tắt) thì thoát.
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
        } else if let Some(label) = message["warm"].as_str() {
            if let Err(e) = warm(&app, label, &bridge) {
                log!("Không dựng sẵn được cửa sổ {label}: {e}");
            }
        } else if let Some(label) = message["cool"].as_str() {
            cool(&app, label);
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

/// Dựng sẵn cửa sổ `label` (ẩn) để lúc mở thì hiện ngay; đã có thì thôi. Không mở trong `LINGER` thì cũng thoát.
fn warm(app: &AppHandle, label: &str, bridge: &Arc<Bridge>) -> tauri::Result<()> {
    if app.get_webview_window(label).is_some() {
        return Ok(());
    }
    match label {
        SETTINGS => build_settings(app, false)?,
        CHAT => build_chat(app, bridge, "TinyWorld", None)?,
        _ => return Ok(()),
    };
    app.state::<Lifetime>().warm.lock().unwrap_or_else(PoisonError::into_inner).insert(label.to_owned());
    exit_later(app);
    Ok(())
}

/// Thôi dựng sẵn `label`: chưa hiện lần nào thì đóng hẳn. Đóng hết cửa sổ (hay chưa có cửa sổ nào) thì tiến trình thoát.
fn cool(app: &AppHandle, label: &str) {
    let warm = app.state::<Lifetime>().warm.lock().unwrap_or_else(PoisonError::into_inner).remove(label);
    match app.get_webview_window(label) {
        Some(window) if warm => {
            if let Err(e) = window.destroy() {
                log!("Không đóng được cửa sổ dựng sẵn {label}: {e}");
            }
        }
        Some(_) => {}
        None if app.webview_windows().is_empty() => app.exit(0),
        None => {}
    }
}

/// Không còn cửa sổ nào hiện thì chờ `LINGER` rồi đóng hẳn các cửa sổ ẩn (đóng hết thì tiến trình thoát). Có cửa sổ ẩn,
/// hiện hay dựng sẵn trong lúc chờ thì thôi lần hẹn này.
fn exit_later(app: &AppHandle) {
    let change = app.state::<Lifetime>().changes.fetch_add(1, Ordering::Relaxed) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(LINGER);
        let windows = app.webview_windows();
        let shown = windows.values().any(|window| window.is_visible().unwrap_or(false));
        if shown || app.state::<Lifetime>().changes.load(Ordering::Relaxed) != change {
            return;
        }
        for window in windows.values() {
            let _ = window.destroy();
        }
    });
}

/// Hiện cửa sổ đang ẩn hay dựng sẵn, đưa lên trước.
fn show(window: &WebviewWindow) -> tauri::Result<()> {
    let lifetime = window.state::<Lifetime>();
    lifetime.changes.fetch_add(1, Ordering::Relaxed);
    lifetime.warm.lock().unwrap_or_else(PoisonError::into_inner).remove(window.label());
    AsRef::<Webview>::as_ref(window).show()?;
    window.unminimize()?;
    window.show()?;
    window.set_focus()
}

/// Bấm đóng thì chỉ ẩn, cả webview (trang thôi vẽ, hẹn giờ chạy thưa lại), rồi tải lại trang để lần mở sau như mở mới:
/// Cài đặt về tab đầu, khung chat bắt đầu đoạn chat mới. `closed` chạy sau đó.
fn hide_on_close(window: &WebviewWindow, closed: impl Fn() + Send + 'static) {
    let handle = window.clone();
    window.on_window_event(move |event| {
        let WindowEvent::CloseRequested { api, .. } = event else {
            return;
        };
        api.prevent_close();
        let webview: &Webview = handle.as_ref();
        for result in [handle.hide(), webview.hide(), handle.reload()] {
            if let Err(e) = result {
                log!("Không ẩn được cửa sổ {}: {e}", handle.label());
            }
        }
        exit_later(handle.app_handle());
        closed();
    });
}

/// Mở cửa sổ Cài đặt, đang mở (hay đang ẩn, dựng sẵn) thì đưa lên trước.
fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    match app.get_webview_window(SETTINGS) {
        Some(window) => show(&window),
        None => build_settings(app, true).map(drop),
    }
}

/// Nền và thanh tiêu đề tối, trùng màu trang (settings.css) để lúc mở không bị loé trắng.
fn build_settings(app: &AppHandle, visible: bool) -> tauri::Result<WebviewWindow> {
    let window = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App("settings.html".into()))
        .title("TinyWorld")
        .inner_size(440.0, 740.0)
        .resizable(false)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .background_color(Color(13, 22, 40, 255))
        .center()
        .visible(visible)
        .focused(visible)
        .build()?;
    crate::app_icon::set_small_icon(&window);
    hide_on_close(&window, || {});
    Ok(window)
}

/// Cỡ cửa sổ cả viền và thanh tiêu đề (CSS pixel).
fn outer_size(window: &WebviewWindow) -> tauri::Result<(f64, f64)> {
    let size = window.outer_size()?.to_logical::<f64>(window.scale_factor()?);
    Ok((size.width, size.height))
}

/// Mở khung chat cạnh pet. Đang mở (hay đang ẩn, dựng sẵn) thì đổi sang pet đó, dời tới cạnh nó và đưa lên trước. Chỗ
/// đặt tính theo màn hình của overlay (`screen`, góc trên trái `origin` theo pixel vật lý) và khung của pet (`pet`, CSS
/// pixel của overlay).
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
        return show(&window);
    }
    let guess = (chat::WIDTH + chat::FRAME_GUESS.0, chat::HEIGHT + chat::FRAME_GUESS.1);
    let window = build_chat(app, bridge, &title, Some(to_screen(chat::place(&screen, pet, guess, side))))?;
    // Cỡ thật cả viền và thanh tiêu đề chỉ đo được sau khi tạo: đặt lại cho trọn trong vùng làm việc.
    window.set_position(to_screen(chat::place(&screen, pet, outer_size(&window)?, side)))?;
    show(&window)
}

/// Dựng khung chat ẩn ở `position` (chưa biết pet nào thì để Windows chọn chỗ).
fn build_chat(
    app: &AppHandle,
    bridge: &Arc<Bridge>,
    title: &str,
    position: Option<LogicalPosition<f64>>,
) -> tauri::Result<WebviewWindow> {
    let mut builder = WebviewWindowBuilder::new(app, CHAT, WebviewUrl::App("chat.html".into()))
        .title(title)
        .inner_size(chat::WIDTH, chat::HEIGHT)
        // Không hẹp hơn cảnh ở đầu khung chat (ChatScene.tsx rộng 360), để pet đứng đúng trên đồi.
        .min_inner_size(chat::WIDTH, 360.0)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .background_color(Color(13, 22, 40, 255))
        .visible(false);
    if let Some(start) = position {
        builder = builder.position(start.x, start.y);
    }
    let window = builder.build()?;
    crate::app_icon::set_small_icon(&window);
    let bridge = bridge.clone();
    // Pet đang chat lại đi lại như thường.
    hide_on_close(&window, move || {
        if let Err(e) = bridge.send(&json!({ "closed": CHAT })) {
            log!("Không báo được đã đóng chat: {}", e.message);
        }
    });
    Ok(window)
}
