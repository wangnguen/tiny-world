//! Ghi lỗi ra `tinyworld.log` trong thư mục dữ liệu (`%APPDATA%\com.tinyworld.app\`), mỗi dòng kèm giờ máy.
//! Bản release không có console nên `eprintln!` không đi đâu cả: lỗi chat, thời tiết, cập nhật, panic đều
//! mất. File chỉ ở trên máy, không gửi đi đâu; Cài đặt có nút mở thư mục để người dùng tự gửi khi cần.
//!
//! Dùng `log!` như `eprintln!`. Chưa gọi `init` (lúc mở app, trong test) thì chỉ in ra console. File quá
//! `MAX_BYTES` thì đổi tên thành `tinyworld.old.log` (đè bản cũ hơn) rồi ghi file mới.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

const FILE_NAME: &str = "tinyworld.log";
const OLD_NAME: &str = "tinyworld.old.log";
const MAX_BYTES: u64 = 1024 * 1024;

struct LogFile {
    path: PathBuf,
    file: File,
    size: u64,
}

static LOG: Mutex<Option<LogFile>> = Mutex::new(None);

/// Ghi một dòng như `eprintln!`, kèm giờ máy.
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(format_args!($($arg)*))
    };
}

/// Mở file log trong `data_dir`, ghi dòng mở app (`version`), ghi lại mọi panic từ đây về sau.
pub fn init(data_dir: &Path, version: &str) {
    let path = data_dir.join(FILE_NAME);
    match open(&path) {
        Ok(file) => *LOG.lock().unwrap_or_else(PoisonError::into_inner) = Some(file),
        Err(e) => eprintln!("Không mở được file log {}: {e}", path.display()),
    }
    write(format_args!("TinyWorld {version} mở"));
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        write(format_args!("Panic ở luồng {}: {info}", thread.name().unwrap_or("?")));
        previous(info);
    }));
}

/// Thư mục chứa file log (để nút trong Cài đặt mở ra); chưa `init` thì `None`.
pub fn dir() -> Option<PathBuf> {
    let log = LOG.lock().unwrap_or_else(PoisonError::into_inner);
    log.as_ref().and_then(|log| log.path.parent().map(Path::to_path_buf))
}

/// Mở `path` để ghi tiếp; quá `MAX_BYTES` thì đổi sang file mới.
fn open(path: &Path) -> std::io::Result<LogFile> {
    if fs::metadata(path).is_ok_and(|m| m.len() > MAX_BYTES) {
        let _ = fs::rename(path, path.with_file_name(OLD_NAME));
    }
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    let size = file.metadata().map_or(0, |m| m.len());
    Ok(LogFile {
        path: path.to_path_buf(),
        file,
        size,
    })
}

/// Ghi `message` thành một dòng có giờ (`log!`).
pub fn write(message: fmt::Arguments) {
    // Định dạng xong trước khi giữ khoá: lỗi lúc định dạng (panic) không làm kẹt khoá của chính panic hook.
    let line = format!("[{}] {message}\n", now());
    eprint!("{line}");
    let mut log = LOG.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(path) = log.as_ref().filter(|l| l.size + line.len() as u64 > MAX_BYTES).map(|l| l.path.clone()) else {
        return append(log.as_mut(), &line);
    };
    // Đóng file trước: Windows không cho đổi tên file đang mở.
    *log = None;
    *log = open(&path).ok();
    append(log.as_mut(), &line);
}

fn append(log: Option<&mut LogFile>, line: &str) {
    let Some(current) = log else {
        return;
    };
    if current.file.write_all(line.as_bytes()).is_ok() {
        current.size += line.len() as u64;
    }
}

/// "2026-10-14 11:20:01" theo giờ máy.
#[cfg(windows)]
fn now() -> String {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    // SAFETY: SYSTEMTIME chỉ gồm số nguyên nên toàn 0 là hợp lệ; GetLocalTime chỉ ghi vào biến cục bộ này.
    let mut t: SYSTEMTIME = unsafe { std::mem::zeroed() };
    // SAFETY: như trên.
    unsafe { GetLocalTime(&mut t) };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond
    )
}

#[cfg(not(windows))]
fn now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!("unix {secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_qua_lon_thi_doi_sang_file_moi() {
        let dir = std::env::temp_dir().join(format!("tinyworld-log-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE_NAME);
        fs::write(&path, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        let log = open(&path).unwrap();
        assert_eq!(log.size, 0);
        assert_eq!(fs::metadata(dir.join(OLD_NAME)).unwrap().len(), MAX_BYTES + 1);
        // File nhỏ thì ghi tiếp vào cuối.
        fs::write(&path, b"abc").unwrap();
        assert_eq!(open(&path).unwrap().size, 3);
        let _ = fs::remove_dir_all(&dir);
        #[cfg(windows)]
        assert_eq!(now().len(), "2026-10-14 11:20:01".len());
    }
}
