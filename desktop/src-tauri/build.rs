fn main() {
    // Version ghi trong tauri.conf.json (workflow Build và Release ghi vào trước khi build): overlay chạy trước khi có
    // Tauri nên không đọc được từ context của Tauri.
    let config = std::fs::read_to_string("tauri.conf.json").expect("đọc tauri.conf.json");
    let config: serde_json::Value = serde_json::from_str(&config).expect("tauri.conf.json không phải JSON");
    let version = config["version"].as_str().unwrap_or("0.0.0");
    println!("cargo:rustc-env=TINYWORLD_VERSION={version}");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    tauri_build::build()
}
