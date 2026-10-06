//! Cài đặt người dùng chỉnh trong cửa sổ Settings, lưu ở `settings.json` cạnh `world.json`.
//! "Chạy cùng Windows" không nằm ở đây mà đọc thẳng từ registry (autostart.rs).

use crate::app_icon;
use crate::error::AppResult;
use crate::storage::write_atomic;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::window::Color;
use tauri::{AppHandle, Manager, Theme, WebviewUrl, WebviewWindowBuilder};

pub const WINDOW_LABEL: &str = "settings";

const SIZE_RANGE: (f64, f64) = (0.5, 2.0);
const SPEED_RANGE: (f64, f64) = (0.5, 2.0);
const PET_ID_MAX_LEN: usize = 64;
/// Số nhân vật tối đa cùng sống trên màn hình, khớp `MAX_PETS` trong packages/core.
const MAX_PETS: usize = 3;
const CITY_NAME_MAX_LEN: usize = 120;
const TIMEZONE_MAX_LEN: usize = 64;
/// Tối đa chừng này sự kiện trong lịch; tên, câu nói dài tối đa chừng này ký tự; kéo dài 1–10 ngày.
const MAX_OCCASIONS: usize = 30;
const OCCASION_NAME_MAX_LEN: usize = 40;
const OCCASION_MESSAGE_MAX_LEN: usize = 80;
const OCCASION_DAYS: (u8, u8) = (1, 10);
/// Nhắc nghỉ sau chừng này phút ngồi liền (mặc định 50).
const BREAK_MINUTES: (u16, u16) = (15, 120);
/// Nhắc uống nước sau chừng này phút ngồi máy (mặc định 60).
const WATER_MINUTES: (u16, u16) = (20, 180);
/// Phút trong ngày, 0–1439.
const MINUTES_PER_DAY: u16 = 24 * 60;

/// Thành phố người dùng chọn để lấy thời tiết thật (weather.rs), khớp `City` trong packages/core. Chỉ
/// lưu tên, toạ độ và múi giờ.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct City {
    /// Tên để hiện, ví dụ "Hà Nội, Việt Nam".
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    /// Múi giờ IANA, ví dụ "Asia/Ho_Chi_Minh": giờ của pet (ngày/đêm, sự kiện, bấm đúp) theo múi giờ này.
    /// `None` là giờ máy.
    #[serde(default)]
    pub timezone: Option<String>,
}

impl City {
    fn is_valid(&self) -> bool {
        let name = self.name.trim();
        !name.is_empty()
            && name.chars().count() <= CITY_NAME_MAX_LEN
            && self.latitude.is_finite()
            && self.longitude.is_finite()
            && (-90.0..=90.0).contains(&self.latitude)
            && (-180.0..=180.0).contains(&self.longitude)
    }
}

/// Tên múi giờ IANA: chữ, số, `/`, `_`, `+`, `-`.
fn is_timezone(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= TIMEZONE_MAX_LEN
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '+' | '-'))
}

/// Một dịp trong lịch sự kiện (Tết, Noel, sinh nhật...), khớp `Occasion` trong packages/core. Lặp lại
/// mỗi năm vào ngày `day/month` (dương lịch, hoặc âm lịch Việt Nam nếu `lunar`) trong `days` ngày.
/// File của bản cũ còn trường `hat` (mũ, đã bỏ) thì bỏ qua lúc đọc, lần lưu sau không ghi nữa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Occasion {
    pub name: String,
    pub day: u8,
    pub month: u8,
    #[serde(default)]
    pub lunar: bool,
    #[serde(default = "one_day")]
    pub days: u8,
    /// Câu một con nói (mỗi ngày một lần) trong dịp đó; rỗng là không nói.
    #[serde(default)]
    pub message: String,
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn one_day() -> u8 {
    1
}

fn enabled() -> bool {
    true
}

impl Occasion {
    fn new(name: &str, day: u8, month: u8, lunar: bool, days: u8, message: &str) -> Self {
        Self {
            name: name.into(),
            day,
            month,
            lunar,
            days,
            message: message.into(),
            enabled: true,
        }
    }

    /// Ngày lễ Việt Nam có sẵn trong lịch lúc mới cài; người dùng tắt, sửa, xoá hay thêm dịp riêng được.
    pub fn presets() -> Vec<Self> {
        vec![
            Self::new("Tết Nguyên Đán", 1, 1, true, 5, "Chúc mừng năm mới :)))"),
            Self::new("Tết Dương lịch", 1, 1, false, 1, "Năm mới vui vẻ nha :)))"),
            Self::new("Giỗ Tổ Hùng Vương", 10, 3, true, 1, "Hôm nay Giỗ Tổ Hùng Vương đó :)"),
            Self::new("Ngày Thống nhất", 30, 4, false, 1, "30/4 rồi, nghỉ lễ chưa :)))"),
            Self::new("Quốc tế Lao động", 1, 5, false, 1, "1/5 nghỉ ngơi chút đi :)))"),
            Self::new("Quốc khánh", 2, 9, false, 1, "Mừng Quốc khánh 2/9 :)"),
            Self::new("Trung thu", 15, 8, true, 1, "Trung thu ăn bánh chưa :)))"),
            Self::new("Giáng sinh", 24, 12, false, 2, "Giáng sinh vui vẻ :)))"),
        ]
    }

    /// Ngày có thật (âm lịch tối đa 30 ngày), tên không rỗng; cắt bớt chữ quá dài, kẹp số ngày kéo dài.
    /// Sai hẳn thì bỏ.
    fn sanitized(self) -> Option<Self> {
        let name: String = self.name.trim().chars().take(OCCASION_NAME_MAX_LEN).collect();
        let max_day = if self.lunar {
            30
        } else {
            match self.month {
                2 => 29,
                4 | 6 | 9 | 11 => 30,
                _ => 31,
            }
        };
        let valid = !name.is_empty()
            && (1..=12).contains(&self.month)
            && (1..=max_day).contains(&self.day);
        valid.then(|| Self {
            name,
            message: self.message.trim().chars().take(OCCASION_MESSAGE_MAX_LEN).collect(),
            days: self.days.clamp(OCCASION_DAYS.0, OCCASION_DAYS.1),
            ..self
        })
    }
}

/// Khớp `Settings` trong packages/core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Cỡ nhân vật so với cỡ gốc của sprite pack (`scale` trong pet.json).
    pub size: f64,
    /// Hệ số tốc độ đi/chạy.
    pub speed: f64,
    /// Các nhân vật đang hiện (tối đa `MAX_PETS`, không trùng), theo thứ tự chọn: tên thư mục sprite
    /// pack trong `assets/sprites/`. Rỗng là pack đầu tiên. Pack không còn trong bản build thì overlay tự
    /// bỏ qua, nên ở đây chỉ kiểm tra dạng tên.
    pub pets: Vec<String>,
    /// Bản cũ chỉ có một nhân vật: đọc vào `pets` khi `pets` rỗng (`sanitized`). Vẫn ghi ra (con đầu tiên)
    /// để mở lại bản cũ không bị mất nhân vật đã chọn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pet: Option<String>,
    /// Thành phố để lấy thời tiết thật; `None` là thời tiết giả lập, không gọi mạng.
    pub city: Option<City>,
    /// Hiệu ứng thời tiết quanh pet (nắng, mây, sao, mưa, tuyết, sương mù, sấm, cánh hoa; nóng, lạnh).
    pub weather: bool,
    /// Nhãn nhiệt độ cạnh pet (cần chọn thành phố).
    pub temperature_tag: bool,
    /// Pet nói câu cho vui (chào nhau, thời tiết).
    pub chatter: bool,
    /// Lịch sự kiện: đúng dịp thì một con nói câu của dịp đó.
    pub events: bool,
    /// Các dịp trong lịch sự kiện (mặc định: ngày lễ Việt Nam, `Occasion::presets`).
    pub occasions: Vec<Occasion>,
    /// Sự kiện hiếm: ma bay qua lúc 2 giờ sáng.
    pub ghost: bool,
    /// Phase 5, mặc định tắt hết. Đếm giờ ngồi máy theo ngày (`stats.json`, activity.rs).
    pub screen_time: bool,
    /// Ngồi liền `break_minutes` phút thì pet nhắc nghỉ.
    pub break_reminder: bool,
    pub break_minutes: u16,
    /// Cứ ngồi máy đủ `water_minutes` phút thì pet nhắc uống nước.
    pub water_reminder: bool,
    pub water_minutes: u16,
    /// Còn ngồi máy sau `bedtime` (phút trong ngày, giờ máy) thì pet nhắc đi ngủ, tối đa 30 phút một lần.
    pub bedtime_reminder: bool,
    pub bedtime: u16,
    /// Bấm Ctrl+S dồn dập thì pet kêu. Chỉ đọc thêm phím S lúc đang giữ Ctrl.
    pub save_spam: bool,
    /// Phase 6, mặc định tắt: click chuột phải vào pet để chat (gửi câu hỏi tới Gemini, chat.rs).
    pub chat: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            size: 1.0,
            speed: 1.0,
            pets: Vec::new(),
            pet: None,
            city: None,
            weather: true,
            temperature_tag: true,
            chatter: true,
            events: true,
            occasions: Occasion::presets(),
            ghost: true,
            screen_time: false,
            break_reminder: false,
            break_minutes: 50,
            water_reminder: false,
            water_minutes: 60,
            bedtime_reminder: false,
            bedtime: 23 * 60,
            save_spam: false,
            chat: false,
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
        let mut pets = self.pets;
        if pets.is_empty() {
            pets.extend(self.pet);
        }
        let mut seen = Vec::new();
        for id in pets {
            if is_pet_id(&id) && !seen.contains(&id) && seen.len() < MAX_PETS {
                seen.push(id);
            }
        }
        Self {
            size: clamp(self.size, SIZE_RANGE, default.size),
            speed: clamp(self.speed, SPEED_RANGE, default.speed),
            pet: seen.first().cloned(),
            pets: seen,
            city: self.city.filter(City::is_valid).map(|c| City {
                name: c.name.trim().to_string(),
                timezone: c.timezone.filter(|tz| is_timezone(tz)),
                ..c
            }),
            occasions: self
                .occasions
                .into_iter()
                .filter_map(Occasion::sanitized)
                .take(MAX_OCCASIONS)
                .collect(),
            break_minutes: self.break_minutes.clamp(BREAK_MINUTES.0, BREAK_MINUTES.1),
            water_minutes: self.water_minutes.clamp(WATER_MINUTES.0, WATER_MINUTES.1),
            bedtime: if self.bedtime < MINUTES_PER_DAY {
                self.bedtime
            } else {
                default.bedtime
            },
            ..self
        }
    }
}

/// Tên thư mục pack: chữ, số, `-`, `_`, `.`, không bắt đầu bằng `.` (không chứa đường dẫn).
fn is_pet_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= PET_ID_MAX_LEN
        && !id.starts_with('.')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
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
        self.current.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Lưu và trả về giá trị thật sự được dùng (đã kẹp lại).
    pub fn set(&self, settings: Settings) -> AppResult<Settings> {
        let settings = settings.sanitized();
        write_atomic(&self.path, &serde_json::to_vec_pretty(&settings)?)?;
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = settings.clone();
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
    // Nền và thanh tiêu đề tối, trùng màu trang (settings.css) để lúc mở không bị loé trắng.
    let window = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("settings.html".into()))
        .title("TinyWorld")
        .inner_size(440.0, 740.0)
        .resizable(false)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .background_color(Color(13, 22, 40, 255))
        .center()
        .focused(true)
        .build()?;
    app_icon::set_small_icon(&window);
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
                pets: vec!["b-kitsu".into(), "a-momo".into()],
                ..Settings::default()
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
                ..Settings::default()
            })
            .unwrap();
        assert_eq!(
            saved,
            Settings {
                size: 2.0,
                speed: 0.5,
                ..Settings::default()
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
                speed: 1.5,
                ..Settings::default()
            }
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn file_ban_cu_mot_nhan_vat_thanh_danh_sach() {
        let dir = temp_dir("legacy");
        fs::write(dir.join("settings.json"), br#"{ "size": 1.5, "pet": "b-kitsu" }"#).unwrap();
        let store = SettingsStore::load(&dir);
        assert_eq!(store.get().pets, vec!["b-kitsu".to_string()]);
        // Chọn thêm rồi lưu: có `pets`, `pet` là con đầu tiên để bản cũ vẫn đọc được.
        store
            .set(Settings {
                pets: vec!["c-lumi".into(), "b-kitsu".into()],
                ..store.get()
            })
            .unwrap();
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(dir.join("settings.json")).unwrap()).unwrap();
        assert_eq!(json["pets"], serde_json::json!(["c-lumi", "b-kitsu"]));
        assert_eq!(json["pet"], "c-lumi");
        // `pets` có rồi thì `pet` không ghi đè lên.
        assert_eq!(SettingsStore::load(&dir).get().pets, ["c-lumi", "b-kitsu"]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn danh_sach_nhan_vat_bo_trung_bo_ten_sai_toi_da_3() {
        let pets = |ids: &[&str]| {
            Settings {
                pets: ids.iter().map(|id| id.to_string()).collect(),
                ..Settings::default()
            }
            .sanitized()
            .pets
        };
        assert_eq!(pets(&["c-lumi", "a/b", "c-lumi", "b-bong"]), ["c-lumi", "b-bong"]);
        assert_eq!(pets(&["a", "b", "c", "d", "e"]), ["a", "b", "c"]);
        assert!(pets(&[]).is_empty());
    }

    #[test]
    fn thanh_pho_sai_thi_bo_mui_gio_sai_thi_dung_gio_may() {
        let city = |name: &str, latitude: f64, longitude: f64, timezone: Option<&str>| {
            Settings {
                city: Some(City {
                    name: name.into(),
                    latitude,
                    longitude,
                    timezone: timezone.map(Into::into),
                }),
                ..Settings::default()
            }
            .sanitized()
            .city
        };
        let hanoi = city("  Hà Nội, Việt Nam ", 21.02, 105.84, Some("Asia/Bangkok")).unwrap();
        assert_eq!(hanoi.name, "Hà Nội, Việt Nam");
        assert_eq!(hanoi.timezone.as_deref(), Some("Asia/Bangkok"));
        assert_eq!(city("X", 0.0, 0.0, Some("../etc")).unwrap().timezone, None);
        assert_eq!(city("X", 0.0, 0.0, Some("")).unwrap().timezone, None);
        assert_eq!(city("", 21.0, 105.0, None), None);
        assert_eq!(city("X", 91.0, 0.0, None), None);
        assert_eq!(city("X", 0.0, f64::NAN, None), None);
        assert_eq!(city(&"x".repeat(CITY_NAME_MAX_LEN + 1), 0.0, 0.0, None), None);
        // File bản trước chưa có múi giờ vẫn đọc được.
        let old: City = serde_json::from_str(r#"{"name":"Huế","latitude":16.46,"longitude":107.59}"#).unwrap();
        assert_eq!(old.timezone, None);
    }

    #[test]
    fn lich_su_kien_bo_dip_sai_kep_so_ngay_cat_chu_dai() {
        let one = |o: Occasion| {
            Settings {
                occasions: vec![o],
                ..Settings::default()
            }
            .sanitized()
            .occasions
            .pop()
        };
        let birthday = Occasion::new(" Sinh nhật ", 29, 2, false, 0, "Chúc mừng sinh nhật :)))");
        let saved = one(birthday).unwrap();
        assert_eq!(saved.name, "Sinh nhật");
        assert_eq!(saved.days, 1);
        // Ngày không có thật, tên rỗng thì bỏ.
        assert_eq!(one(Occasion::new("A", 31, 4, false, 1, "")), None);
        assert_eq!(one(Occasion::new("A", 31, 1, true, 1, "")), None);
        assert_eq!(one(Occasion::new("A", 1, 13, false, 1, "")), None);
        assert_eq!(one(Occasion::new("  ", 1, 1, false, 1, "")), None);
        // Âm lịch có ngày 30; kéo dài quá thì kẹp lại; chữ dài thì cắt.
        let long = one(Occasion::new(&"x".repeat(60), 30, 12, true, 99, &"y".repeat(200))).unwrap();
        assert_eq!(long.days, 10);
        assert_eq!(long.name.chars().count(), OCCASION_NAME_MAX_LEN);
        assert_eq!(long.message.chars().count(), OCCASION_MESSAGE_MAX_LEN);
        // Tối đa 30 dịp.
        let many = Settings {
            occasions: (0..40).map(|i| Occasion::new(&format!("D{i}"), 1, 1, false, 1, "")).collect(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(many.occasions.len(), MAX_OCCASIONS);
    }

    #[test]
    fn file_cu_con_mu_trong_lich_su_kien_van_doc_duoc_va_khong_ghi_lai() {
        let settings: Settings = serde_json::from_str(
            r#"{ "occasions": [{ "name": "Tết", "day": 1, "month": 1, "lunar": true, "hat": "tet" }] }"#,
        )
        .unwrap();
        let settings = settings.sanitized();
        assert_eq!(settings.occasions.len(), 1);
        assert!(!serde_json::to_string(&settings).unwrap().contains(r#""hat""#));
    }

    #[test]
    fn file_cu_chua_co_cong_tac_thi_bat_san_va_co_san_ngay_le() {
        let settings: Settings = serde_json::from_str(r#"{ "pet": "a-momo" }"#).unwrap();
        let settings = settings.sanitized();
        assert!(settings.weather && settings.temperature_tag && settings.chatter);
        assert!(settings.events && settings.ghost);
        assert_eq!(settings.city, None);
        assert_eq!(settings.occasions, Occasion::presets());
        // Ngày lễ có sẵn đều hợp lệ.
        assert_eq!(settings.occasions.len(), 8);
        // Người dùng xoá hết dịp thì giữ nguyên là rỗng, không tự thêm lại.
        let empty: Settings = serde_json::from_str(r#"{ "occasions": [] }"#).unwrap();
        assert!(empty.sanitized().occasions.is_empty());
    }

    #[test]
    fn suc_khoe_mac_dinh_tat_het_gia_tri_sai_thi_kep_lai() {
        // File cũ còn "appTime" (đã bỏ đếm theo từng app) vẫn đọc được.
        let settings: Settings = serde_json::from_str(r#"{ "pet": "a-momo", "appTime": true }"#).unwrap();
        let settings = settings.sanitized();
        assert!(
            !(settings.screen_time
                || settings.break_reminder
                || settings.water_reminder
                || settings.bedtime_reminder
                || settings.save_spam)
        );
        assert_eq!((settings.break_minutes, settings.water_minutes, settings.bedtime), (50, 60, 23 * 60));
        let odd: Settings =
            serde_json::from_str(r#"{ "breakMinutes": 2, "waterMinutes": 999, "bedtime": 5000 }"#).unwrap();
        let odd = odd.sanitized();
        assert_eq!(
            (odd.break_minutes, odd.water_minutes, odd.bedtime),
            (BREAK_MINUTES.0, WATER_MINUTES.1, 23 * 60)
        );
    }

    #[test]
    fn ten_pack_khong_hop_le_thi_bo() {
        let valid = |id: &str| {
            Settings {
                pets: vec![id.into()],
                ..Settings::default()
            }
            .sanitized()
            .pets
            .len()
                == 1
        };
        assert!(valid("c-lumi"));
        assert!(valid("Cat_v2.1"));
        let long = "x".repeat(PET_ID_MAX_LEN + 1);
        let bad: [&str; 7] = ["", "../world", "a/b", "a\\b", ".hidden", "mèo", &long];
        for id in bad {
            assert!(!valid(id), "{id}");
        }
    }
}
