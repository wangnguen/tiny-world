//! Lưu trạng thái thế giới pet dạng JSON trong thư mục dữ liệu của app
//! (`%APPDATA%\com.tinyworld.app\world.json`).

use crate::error::{AppError, AppResult};
use serde_json::Value;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

/// Giới hạn kích thước trạng thái nhận từ frontend.
const MAX_STATE_BYTES: usize = 1024 * 1024;

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            path: data_dir.join("world.json"),
        }
    }

    /// `None` khi chưa lưu lần nào.
    pub fn load(&self) -> AppResult<Option<Value>> {
        match fs::read(&self.path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Ghi ra file tạm rồi đổi tên, để tắt máy ngang lúc đang ghi cũng không hỏng file cũ.
    pub fn save(&self, state: &Value) -> AppResult<()> {
        let bytes = serde_json::to_vec_pretty(state)?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err(AppError::bad_request("Trạng thái cần lưu quá lớn."));
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_storage(name: &str) -> (Storage, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "tinyworld-storage-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        (Storage::new(dir.clone()), dir)
    }

    #[test]
    fn chua_luu_thi_tra_ve_none() {
        let (storage, dir) = temp_storage("empty");
        assert_eq!(storage.load().unwrap(), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn luu_roi_doc_lai_va_ghi_de() {
        let (storage, dir) = temp_storage("roundtrip");
        storage.save(&json!({ "pets": [{ "x": 1 }] })).unwrap();
        storage.save(&json!({ "pets": [{ "x": 2 }] })).unwrap();
        assert_eq!(storage.load().unwrap(), Some(json!({ "pets": [{ "x": 2 }] })));
        assert!(!dir.join("world.json.tmp").exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tu_choi_trang_thai_qua_lon() {
        let (storage, dir) = temp_storage("too-large");
        let big = json!({ "data": "x".repeat(MAX_STATE_BYTES) });
        assert_eq!(storage.save(&big).unwrap_err().code, "BAD_REQUEST");
        assert_eq!(storage.load().unwrap(), None);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn file_hong_thi_bao_loi() {
        let (storage, dir) = temp_storage("corrupt");
        fs::write(dir.join("world.json"), b"{ not json").unwrap();
        assert_eq!(storage.load().unwrap_err().code, "INTERNAL");
        fs::remove_dir_all(dir).unwrap();
    }
}
