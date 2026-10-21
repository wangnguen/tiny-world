//! Những câu đã nói hôm nay (câu của dịp lễ, con ma) và bản mới nhất pet đã báo, để mở lại app không nói lại. Lưu ở
//! `said.json` trong thư mục dữ liệu (bản web lưu trong localStorage của overlay); mất thì cùng lắm nói lại một lần.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::PathBuf;
use tinyworld_backend::storage::write_atomic;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct File {
    /// Ngày của `keys`, dạng "2026-10-01".
    day: String,
    keys: BTreeSet<String>,
    /// Bản mới nhất đã báo (update.rs).
    update_said: Option<String>,
}

pub struct Said {
    path: PathBuf,
    file: File,
}

impl Said {
    pub fn load(path: PathBuf) -> Self {
        let file = std::fs::read(&path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default();
        Self { path, file }
    }

    pub fn has(&self, day: &str, key: &str) -> bool {
        self.file.day == day && self.file.keys.contains(key)
    }

    pub fn add(&mut self, day: &str, key: &str) {
        if day != self.file.day {
            self.file.day = day.to_owned();
            self.file.keys.clear();
        }
        self.file.keys.insert(key.to_owned());
        self.save();
    }

    pub fn update_said(&self) -> Option<&str> {
        self.file.update_said.as_deref()
    }

    pub fn set_update_said(&mut self, version: &str) {
        self.file.update_said = Some(version.to_owned());
        self.save();
    }

    fn save(&self) {
        let result = serde_json::to_vec(&self.file).map_err(|e| e.to_string()).and_then(|bytes| {
            write_atomic(&self.path, &bytes).map_err(|e| e.message)
        });
        if let Err(e) = result {
            log!("Không lưu được said.json: {e}");
        }
    }
}
