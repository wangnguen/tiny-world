use serde::Serialize;
use std::fmt;

/// Lỗi trả về cho frontend dưới dạng `{ code, message }`.
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new("BAD_REQUEST", message)
    }

    pub fn no_window() -> Self {
        Self::new("NO_WINDOW", "Không tìm thấy cửa sổ overlay.")
    }

    pub fn internal(message: impl fmt::Display) -> Self {
        Self::new("INTERNAL", message.to_string())
    }

    /// Mất mạng hoặc máy chủ không trả lời (thời tiết, chat).
    pub fn offline(message: impl Into<String>) -> Self {
        Self::new("OFFLINE", message)
    }

    /// App tự chặn để không gửi dồn dập (chat): câu báo ghi lúc gửi lại được.
    pub fn busy(message: impl Into<String>) -> Self {
        Self::new("BUSY", message)
    }

    /// Dịch vụ bên ngoài lỗi hay đang chặn (chat): câu báo ghi lúc gửi lại được.
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new("UNAVAILABLE", message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::internal(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::internal(e)
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self::internal(e)
    }
}
