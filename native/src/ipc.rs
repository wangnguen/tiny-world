//! Nối với tiến trình giao diện (Cài đặt, khung chat: Tauri + React, chạy `tinyworld.exe --ui` lúc cần). Đóng cửa sổ thì
//! tiến trình đó chỉ ẩn cửa sổ, mấy phút không mở lại mới thoát; sắp mở (bấm icon khay, con trỏ dừng trên pet lúc đã bật
//! chat) thì overlay bảo nó dựng sẵn cửa sổ ẩn, khỏi chờ WebView2 khởi động lúc mở. Overlay nghe ở cổng loopback ngẫu
//! nhiên; tiến trình con nhận cổng và mã bí mật qua biến môi trường `TINYWORLD_IPC` ("cổng mã"), kết nối rồi gửi mã trước
//! tiên. Mỗi dòng một JSON:
//!
//! - giao diện gửi: `{"hello": mã}`, `{"id": n, "call": tên command, "args": {...}}`, `{"closed": "chat"}`
//! - overlay gửi: `{"id": n, "ok": giá trị}` hoặc `{"id": n, "err": AppError}`, `{"event": tên, "payload": ...,
//!   "to": cửa sổ | null}`, `{"open": "settings"}`, `{"open": "chat", ...}`, `{"warm": cửa sổ}` (dựng sẵn),
//!   `{"cool": cửa sổ}` (thôi dựng sẵn), `{"close": "chat"}`, `{"title": ...}`, `{"quit": true}`
//!
//! Chỉ một tiến trình giao diện được nối mỗi lúc; mất kết nối thì lần mở sau chạy tiến trình mới.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard};
use tinyworld_backend::error::AppError;
use windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

/// Biến môi trường tiến trình giao diện đọc để nối về overlay.
pub const ENV: &str = "TINYWORLD_IPC";
/// Tên các cửa sổ của tiến trình giao diện.
pub const SETTINGS: &str = "settings";
pub const CHAT: &str = "chat";

/// Đầu ghi của kết nối, dùng chung giữa vòng lặp chính và các tác vụ trả lời chậm (chat, tìm thành phố).
#[derive(Clone)]
pub struct Writer(Arc<Mutex<TcpStream>>);

impl Writer {
    pub fn send(&self, message: &Value) {
        let mut line = message.to_string();
        line.push('\n');
        let mut stream = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = stream.write_all(line.as_bytes()) {
            log!("Không gửi được cho cửa sổ giao diện: {e}");
        }
    }

    pub fn reply(&self, id: u64, result: Result<Value, AppError>) {
        self.send(&match result {
            Ok(value) => json!({ "id": id, "ok": value }),
            Err(e) => json!({ "id": id, "err": e }),
        });
    }
}

/// Tin từ tiến trình giao diện, đưa vào vòng lặp chính.
pub enum Message {
    Connected(Writer),
    /// Gọi command; trả lời vào `writer`.
    Call { id: u64, name: String, args: Value, writer: Writer },
    /// Cửa sổ chat vừa đóng.
    ChatClosed,
    Disconnected,
}

pub struct Ipc(Arc<Mutex<Shared>>);

/// Dùng chung giữa vòng lặp chính và khay (`Launcher`).
struct Shared {
    /// Exe chạy được `--ui`; `None` thì không mở được cửa sổ nào.
    exe: Option<PathBuf>,
    /// Giá trị `ENV` cho tiến trình con.
    env: String,
    writer: Option<Writer>,
    child: Option<Child>,
    /// Tin chờ gửi lúc tiến trình giao diện vừa nối (mở cửa sổ nào).
    pending: Vec<Value>,
    /// Cửa sổ đang dựng sẵn mà chưa mở.
    warm: Vec<&'static str>,
}

impl Shared {
    fn running(&mut self) -> bool {
        self.child.as_mut().is_some_and(|child| matches!(child.try_wait(), Ok(None)))
    }

    /// Gửi ngay nếu tiến trình giao diện đang nối, chưa nối thì chờ lúc nối; chưa chạy thì chạy nếu `launch`.
    fn send(&mut self, message: Value, launch: bool) {
        if let Some(writer) = &self.writer {
            writer.send(&message);
            return;
        }
        if self.running() {
            self.pending.push(message);
            return;
        }
        let Some(exe) = self.exe.as_ref().filter(|_| launch) else {
            return;
        };
        match Command::new(exe).arg("--ui").env(ENV, &self.env).spawn() {
            Ok(child) => {
                self.child = Some(child);
                // Tin chờ của tiến trình trước (thoát trước khi kịp nối) không còn đúng.
                self.pending = vec![message];
                self.warm.clear();
            }
            Err(e) => log!("Không chạy được tiến trình giao diện {}: {e}", exe.display()),
        }
    }

    fn warm(&mut self, window: &'static str) {
        if self.exe.is_none() || self.warm.contains(&window) {
            return;
        }
        self.send(json!({ "warm": window }), true);
        self.warm.push(window);
    }
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

impl Ipc {
    /// Mở cổng nghe; mỗi tin nhận được thì gửi vào `tx` rồi gọi `wake`. Bản dev ghi cổng và mã vào `debug` để
    /// script thử nối vào gọi command (`{"hello": mã, "debug": true}`, không thay kết nối của cửa sổ giao diện).
    pub fn start(tx: Sender<Message>, wake: fn(), exe: Option<PathBuf>, debug: Option<&Path>) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let token = token();
        if let Some(path) = debug.filter(|_| cfg!(debug_assertions)) {
            std::fs::write(path, json!({ "port": port, "token": token }).to_string())?;
        }
        let env = format!("{port} {token}");
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (expected, tx) = (token.clone(), tx.clone());
                std::thread::spawn(move || serve(stream, &expected, &tx, wake));
            }
        });
        let shared = Shared { exe, env, writer: None, child: None, pending: Vec::new(), warm: Vec::new() };
        Ok(Self(Arc::new(Mutex::new(shared))))
    }

    /// Cho khay dựng sẵn Cài đặt.
    pub fn launcher(&self) -> Launcher {
        Launcher(self.0.clone())
    }

    pub fn connected(&mut self, writer: Writer) {
        let mut shared = lock(&self.0);
        for message in shared.pending.drain(..) {
            writer.send(&message);
        }
        shared.writer = Some(writer);
    }

    pub fn disconnected(&mut self) {
        let mut shared = lock(&self.0);
        shared.writer = None;
        shared.pending.clear();
        shared.warm.clear();
        if let Some(mut child) = shared.child.take() {
            let _ = child.try_wait();
        }
    }

    /// Mở cửa sổ `message["open"]`: gửi cho tiến trình giao diện, chưa chạy thì chạy rồi gửi lúc nó nối.
    pub fn open(&mut self, message: Value) {
        let mut shared = lock(&self.0);
        shared.warm.retain(|window| message["open"] != *window);
        shared.send(message, true);
        if shared.exe.is_none() {
            log!("Không có tiến trình giao diện để mở cửa sổ.");
        }
        // Tiến trình giao diện chạy từ trước không tự đưa cửa sổ lên trước được; overlay vừa được bấm (khay, pet) nên
        // cho phép nó.
        if let Some(child) = &shared.child {
            unsafe {
                let _ = AllowSetForegroundWindow(child.id());
            }
        }
    }

    /// Sắp mở `window`: dựng sẵn cửa sổ ẩn, chưa chạy tiến trình giao diện thì chạy. Đang dựng sẵn rồi thì thôi.
    pub fn warm(&mut self, window: &'static str) {
        lock(&self.0).warm(window);
    }

    /// Thôi dựng sẵn `window` (chưa mở): tiến trình giao diện đóng cửa sổ đó, không còn cửa sổ nào thì thoát.
    pub fn cool(&mut self, window: &str) {
        let mut shared = lock(&self.0);
        let Some(i) = shared.warm.iter().position(|w| *w == window) else {
            return;
        };
        shared.warm.remove(i);
        // Chưa kịp gửi thì bỏ luôn tin dựng sẵn.
        let warm = json!({ "warm": window });
        shared.pending.retain(|message| *message != warm);
        shared.send(json!({ "cool": window }), false);
    }

    /// Gửi tin cho tiến trình giao diện nếu đang chạy (không chạy thì thôi: chưa có cửa sổ nào cần biết).
    pub fn send(&self, message: Value) {
        if let Some(writer) = &lock(&self.0).writer {
            writer.send(&message);
        }
    }

    /// Gửi event cho cửa sổ `to` (hoặc mọi cửa sổ).
    pub fn event(&self, name: &str, payload: Value, to: Option<&str>) {
        self.send(json!({ "event": name, "payload": payload, "to": to }));
    }

    /// App thoát: đóng luôn các cửa sổ giao diện.
    pub fn quit(&mut self) {
        self.send(json!({ "quit": true }));
        lock(&self.0).writer = None;
    }
}

/// Luồng khay gọi lúc bấm icon, ngay trước khi menu hiện, để khỏi chờ vòng lặp chính.
#[derive(Clone)]
pub struct Launcher(Arc<Mutex<Shared>>);

impl Launcher {
    /// Dựng sẵn cửa sổ Cài đặt (ẩn) để chọn Cài đặt… thì hiện ngay; menu đóng mà không chọn thì vòng lặp chính `cool`.
    pub fn warm_settings(&self) {
        lock(&self.0).warm(SETTINGS);
    }
}

/// Mã bí mật cho một lần chạy: tiến trình khác trên máy không đoán được để gọi command.
fn token() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    (0..4)
        .map(|i| {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(i);
            hasher.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
            format!("{:016x}", hasher.finish())
        })
        .collect()
}

/// Đọc một kết nối tới khi đóng. Dòng đầu phải là mã bí mật, sai thì bỏ kết nối.
fn serve(stream: TcpStream, token: &str, tx: &Sender<Message>, wake: fn()) {
    let Ok(read) = stream.try_clone() else {
        return;
    };
    let mut lines = BufReader::new(read).lines();
    let hello = lines.next().and_then(Result::ok).and_then(|line| serde_json::from_str::<Value>(&line).ok());
    if hello.as_ref().and_then(|h| h["hello"].as_str()) != Some(token) {
        log!("Kết nối giao diện sai mã, bỏ qua.");
        return;
    }
    let send = |message| {
        let _ = tx.send(message);
        wake();
    };
    let writer = Writer(Arc::new(Mutex::new(stream)));
    let debug = cfg!(debug_assertions) && hello.as_ref().is_some_and(|h| h["debug"] == true);
    if !debug {
        send(Message::Connected(writer.clone()));
    }
    for line in lines {
        let Ok(line) = line else { break };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let (Some(id), Some(name)) = (message["id"].as_u64(), message["call"].as_str()) {
            send(Message::Call { id, name: name.to_owned(), args: message["args"].clone(), writer: writer.clone() });
        } else if message["closed"] == "chat" {
            send(Message::ChatClosed);
        }
    }
    if !debug {
        send(Message::Disconnected);
    }
}
