//! Cài đặt người dùng chỉnh trong cửa sổ Settings, lưu ở `settings.json` cạnh `world.json`.
//! "Chạy cùng Windows" không nằm ở đây mà đọc thẳng từ registry (autostart.rs).

use crate::error::AppResult;
use crate::storage::write_atomic;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW_LABEL: &str = "settings";

const SIZE_RANGE: (f64, f64) = (0.5, 2.0);
const SPEED_RANGE: (f64, f64) = (0.5, 2.0);

/// Khớp `Settings` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Cỡ nhân vật so với cỡ gốc của sprite pack (`scale` trong pet.json).
    pub size: f64,
    /// Hệ số tốc độ đi/chạy.
    pub speed: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            size: 1.0,
            speed: 1.0,
        }
    }
}

impl Settings {
    /// Giá trị ngoài khoảng (file bị sửa tay) thì kẹp lại.
    fn sanitized(self) -> Self {
        let clamp = |value: f64, (min, max): (f64, f64), fallback: f64| {
            if value.is_finite() {
                value.clamp(min, max)
            } else {
                fallback
            }
        };
        let default = Self::default();
        Self {
            size: clamp(self.size, SIZE_RANGE, default.size),
            speed: clamp(self.speed, SPEED_RANGE, default.speed),
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    /// Chưa có file hoặc file hỏng thì dùng mặc định (file hỏng sẽ bị ghi đè ở lần lưu sau).
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("settings.json");
        let settings = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Settings>(&bytes).unwrap_or_else(|e| {
                eprintln!("settings.json hỏng, dùng cài đặt mặc định: {e}");
                Settings::default()
            }),
            Err(e) if e.kind() == ErrorKind::NotFound => Settings::default(),
            Err(e) => {
                eprintln!("Không đọc được settings.json, dùng cài đặt mặc định: {e}");
                Settings::default()
            }
        };
        Self {
            path,
            current: Mutex::new(settings.sanitized()),
        }
    }

    pub fn get(&self) -> Settings {
        *self.current.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Lưu và trả về giá trị thật sự được dùng (đã kẹp lại).
    pub fn set(&self, settings: Settings) -> AppResult<Settings> {
        let settings = settings.sanitized();
        write_atomic(&self.path, &serde_json::to_vec_pretty(&settings)?)?;
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = settings;
        Ok(settings)
    }
}

/// Mở cửa sổ Settings, đang mở thì đưa lên trước. Chỉ tạo khi cần và huỷ khi đóng để đỡ tốn RAM.
///
/// Tạo webview trong handler của tray trên Windows có thể treo (xem tài liệu
/// `WebviewWindowBuilder::build`), nên chạy ở luồng riêng.
pub fn open_window(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = show_or_create(&app) {
            eprintln!("Không mở được cửa sổ cài đặt: {e}");
        }
    });
}

fn show_or_create(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        return window.set_focus();
    }
    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("settings.html".into()))
        .title("Cài đặt TinyWorld")
        .inner_size(400.0, 360.0)
        .resizable(false)
        .maximizable(false)
        .center()
        .focused(true)
        .build()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tinyworld-settings-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn chua_co_file_thi_dung_mac_dinh_luu_roi_doc_lai() {
        let dir = temp_dir("roundtrip");
        let store = SettingsStore::load(&dir);
        assert_eq!(store.get(), Settings::default());
        let saved = store
            .set(Settings {
                size: 1.5,
                speed: 0.5,
            })
            .unwrap();
        assert_eq!(SettingsStore::load(&dir).get(), saved);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn gia_tri_ngoai_khoang_thi_kep_lai() {
        let dir = temp_dir("clamp");
        let saved = SettingsStore::load(&dir)
            .set(Settings {
                size: 10.0,
                speed: 0.0,
            })
            .unwrap();
        assert_eq!(
            saved,
            Settings {
                size: 2.0,
                speed: 0.5
            }
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn file_hong_hoac_thieu_truong_van_chay() {
        let dir = temp_dir("corrupt");
        fs::write(dir.join("settings.json"), b"{ not json").unwrap();
        assert_eq!(SettingsStore::load(&dir).get(), Settings::default());
        fs::write(dir.join("settings.json"), br#"{ "speed": 1.5, "moi": true }"#).unwrap();
        assert_eq!(
            SettingsStore::load(&dir).get(),
            Settings {
                size: 1.0,
                speed: 1.5
            }
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
