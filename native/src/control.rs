//! Cổng điều khiển trên loopback (std không có named pipe server): giữ app chỉ chạy một bản, và để thử tự động mà
//! không đụng chuột thật. Mỗi kết nối gửi một dòng lệnh; `stats` được trả lời rồi đóng kết nối.
//!
//!   quit | mode auto|idle|walk|run | drop <pet> <x> | stats

use crate::pet::Mode;
use std::io::{BufRead, BufReader};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::time::Duration;
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_APP};

/// Khác cổng 47811 của Look để chạy cạnh nhau được.
pub const PORT: u16 = 47832;

pub enum Command {
    Quit,
    Mode(Mode),
    /// Thả pet thứ `pet` rơi từ mép trên overlay, ở `x` (pixel của overlay).
    Drop { pet: usize, x: f32 },
    Stats(TcpStream),
}

/// Nhận lệnh trên luồng riêng, gửi qua `tx` rồi đánh thức vòng lặp của luồng `main_thread`. Đã có bản khác chạy
/// thì trả `AddrInUse`.
pub fn serve(tx: Sender<Command>, main_thread: u32) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", PORT))?;
    std::thread::Builder::new().name("control".into()).spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() {
                continue;
            }
            let words: Vec<&str> = line.split_whitespace().collect();
            let command = match words[..] {
                ["quit"] => Command::Quit,
                ["mode", mode] => match Mode::parse(mode) {
                    Some(mode) => Command::Mode(mode),
                    None => continue,
                },
                ["drop", pet, x] => match (pet.parse(), x.parse()) {
                    (Ok(pet), Ok(x)) => Command::Drop { pet, x },
                    _ => continue,
                },
                ["stats"] => Command::Stats(stream),
                _ => {
                    eprintln!("lệnh không hiểu: {}", line.trim());
                    continue;
                }
            };
            if tx.send(command).is_err() {
                break;
            }
            unsafe {
                let _ = PostThreadMessageW(main_thread, WM_APP, WPARAM(0), LPARAM(0));
            }
        }
    })?;
    Ok(())
}
