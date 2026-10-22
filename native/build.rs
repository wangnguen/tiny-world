//! Nhúng sprite pack (`assets/sprites`) và effect của Long (`assets/effects`) vào exe: tạo `assets.rs` trong
//! `OUT_DIR`, mỗi file một dòng `(đường dẫn, include_bytes!)`. Thêm hay sửa pack thì build lại là có.

use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../assets");
    let mut files = Vec::new();
    for dir in ["sprites", "effects"] {
        let path = root.join(dir);
        println!("cargo:rerun-if-changed={}", path.display());
        collect(&path, dir, &mut files);
    }
    files.sort();
    let mut out = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for (name, path) in &files {
        out.push_str(&format!("    ({name:?}, include_bytes!({:?})),\n", path.display().to_string()));
    }
    out.push_str("];\n");
    let target = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("assets.rs");
    fs::write(target, out).unwrap();
}

/// Mọi file trong `dir`, tên tính từ `assets/` (chữ thường: Windows không phân biệt hoa thường, pet.json ghi
/// "idle.png" cho file "Idle.png" vẫn phải chạy).
fn collect(dir: &Path, name: &str, files: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let child = format!("{name}/{}", entry.file_name().to_string_lossy()).to_lowercase();
        if path.is_dir() {
            println!("cargo:rerun-if-changed={}", path.display());
            collect(&path, &child, files);
        } else if is_asset(&path) {
            files.push((child, path));
        }
    }
}

fn is_asset(path: &Path) -> bool {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or_default().to_lowercase();
    matches!(extension.as_str(), "json" | "png" | "webp")
}
