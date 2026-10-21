//! Nối với tiến trình giao diện (Cài đặt, khung chat: Tauri + React, chạy `tinyworld.exe --ui` lúc cần, thoát khi đóng
//! hết cửa sổ). Overlay nghe ở cổng loopback ngẫu nhiên; tiến trình con nhận cổng và mã bí mật qua biến môi trường
//! `TINYWORLD_IPC` ("cổng mã"), kết nối rồi gửi mã trước tiên. Mỗi dòng một JSON:
//!
//! - giao diện gửi: `{"hello": mã}`, `{"id": n, "call": tên command, "args": {...}}`, `{"closed": "chat"}`
//! - overlay gửi: `{"id": n, "ok": giá trị}` hoặc `{"id": n, "err": AppError}`, `{"event": tên, "payload": ...,
//!   "to": cửa sổ | null}`, `{"open": "settings"}`, `{"open": "chat", ...}`, `{"close": "chat"}`, `{"title": ...}`,
//!   `{"quit": true}`
//!
//! Chỉ một tiến trình giao diện được nối mỗi lúc; mất kết nối thì lần mở sau chạy tiến trình mới.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tinyworld_backend::error::AppError;

/// Biến môi trường tiến trình giao diện đọc để nối về overlay.
pub const ENV: &str = "TINYWORLD_IPC";

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

pub struct Ipc {
    port: u16,
    token: String,
    writer: Option<Writer>,
    child: Option<Child>,
    /// Tin chờ gửi lúc tiến trình giao diện vừa nối (mở cửa sổ nào).
    pending: Vec<Value>,
}

impl Ipc {
    /// Mở cổng nghe; mỗi tin nhận được thì gửi vào `tx` rồi gọi `wake`. Bản dev ghi cổng và mã vào `debug` để
    /// script thử nối vào gọi command (`{"hello": mã, "debug": true}`, không thay kết nối của cửa sổ giao diện).
    pub fn start(tx: Sender<Message>, wake: fn(), debug: Option<&Path>) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let token = token();
        if let Some(path) = debug.filter(|_| cfg!(debug_assertions)) {
            std::fs::write(path, json!({ "port": port, "token": token }).to_string())?;
        }
        let expected = token.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (expected, tx) = (expected.clone(), tx.clone());
                std::thread::spawn(move || serve(stream, &expected, &tx, wake));
            }
        });
        Ok(Self { port, token, writer: None, child: None, pending: Vec::new() })
    }

    pub fn connected(&mut self, writer: Writer) {
        for message in self.pending.drain(..) {
            writer.send(&message);
        }
        self.writer = Some(writer);
    }

    pub fn disconnected(&mut self) {
        self.writer = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.try_wait();
        }
    }

    /// Gửi `message` cho tiến trình giao diện; chưa chạy thì chạy (`exe --ui`) rồi gửi lúc nó nối.
    pub fn open(&mut self, exe: Option<&Path>, message: Value) {
        if let Some(writer) = &self.writer {
            writer.send(&message);
            return;
        }
        self.pending.push(message);
        let running = self.child.as_mut().is_some_and(|child| matches!(child.try_wait(), Ok(None)));
        if running {
            return;
        }
        let Some(exe) = exe else {
            log!("Không có tiến trình giao diện để mở cửa sổ.");
            self.pending.clear();
            return;
        };
        match Command::new(exe).arg("--ui").env(ENV, format!("{} {}", self.port, self.token)).spawn() {
            Ok(child) => self.child = Some(child),
            Err(e) => {
                log!("Không chạy được tiến trình giao diện {}: {e}", exe.display());
                self.pending.clear();
            }
        }
    }

    /// Gửi tin cho tiến trình giao diện nếu đang chạy (không chạy thì thôi: chưa có cửa sổ nào cần biết).
    pub fn send(&self, message: Value) {
        if let Some(writer) = &self.writer {
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
        self.writer = None;
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
