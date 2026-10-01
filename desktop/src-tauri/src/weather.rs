//! Thời tiết thật từ Open-Meteo (miễn phí, không cần key hay tài khoản) theo thành phố người dùng tự
//! nhập trong Cài đặt. Hỏi lúc mở app, lúc đổi thành phố, rồi 30 phút một lần; lỗi thì chờ lâu dần mới
//! hỏi lại, và báo overlay một lần mỗi đợt lỗi (pet nói là không có mạng / không có thời tiết). Kết quả
//! gần nhất lưu ở `weather.json`, mở app lúc mất mạng vẫn có thời tiết. Chưa chọn thành phố thì không gọi
//! mạng, overlay dùng thời tiết giả lập. Webview không gọi mạng: mọi request ở đây.
//!
//! Hạn mức miễn phí dưới 10.000 lần/ngày; app chỉ gọi khoảng 50 lần/ngày.

use crate::error::{AppError, AppResult};
use crate::events;
use crate::settings::{City, SettingsStore};
use crate::storage::write_atomic;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";
const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
/// Hỏi lại thời tiết chừng này một lần.
const REFRESH: Duration = Duration::from_secs(30 * 60);
/// Lỗi lần đầu thì chờ chừng này rồi hỏi lại, mỗi lần lỗi tiếp gấp đôi, tối đa `REFRESH`.
const RETRY_FIRST: Duration = Duration::from_secs(60);
const TIMEOUT: Duration = Duration::from_secs(15);
/// Tìm thành phố: tối đa chừng này kết quả, từ khoá dài 2–80 ký tự.
const SEARCH_COUNT: &str = "6";
const QUERY_LEN: (usize, usize) = (2, 80);

/// Thời tiết hiện tại ở một chỗ, khớp `WeatherReport` trong packages/core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub latitude: f64,
    pub longitude: f64,
    /// Mã thời tiết WMO (0 quang, 61 mưa, 71 tuyết, 95 sấm...).
    pub code: u16,
    /// Mặt trời đang mọc (giờ mọc/lặn thật ở chỗ đó).
    pub is_day: bool,
    /// °C.
    pub temperature: f64,
    /// km/h.
    pub wind_speed: f64,
    /// Lúc lấy về, số giây từ 1970 (UTC).
    pub fetched_at: u64,
}

impl Report {
    /// Kết quả này của đúng thành phố `city` (đổi thành phố thì kết quả cũ không còn dùng được).
    fn is_for(&self, city: &City) -> bool {
        (self.latitude - city.latitude).abs() < 1e-6 && (self.longitude - city.longitude).abs() < 1e-6
    }
}

/// Event `weather-failed`, khớp `WeatherFailure` trong packages/core: pet nói là không có mạng hay không
/// có thời tiết.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    /// Mất mạng hoặc máy chủ không trả lời; `false` là máy chủ trả lỗi hoặc dữ liệu lạ.
    pub offline: bool,
}

/// Một kết quả tìm thành phố, khớp `CityResult` trong packages/core.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CityResult {
    /// Tên để hiện, ví dụ "Hà Nội, Việt Nam": tên, tỉnh/bang (nếu khác tên), nước.
    pub label: String,
    pub latitude: f64,
    pub longitude: f64,
    /// Múi giờ IANA của chỗ đó, ví dụ "Asia/Bangkok".
    pub timezone: Option<String>,
}

pub struct Weather {
    path: PathBuf,
    report: Mutex<Option<Report>>,
    /// Đổi thành phố: đánh thức vòng hỏi thời tiết để hỏi ngay.
    wake: Notify,
}

impl Weather {
    /// Đọc kết quả đã lưu; chưa có hoặc file hỏng thì coi như chưa có.
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("weather.json");
        let report = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Report>(&bytes).ok());
        Self {
            path,
            report: Mutex::new(report),
            wake: Notify::new(),
        }
    }

    /// Kết quả gần nhất của thành phố `city`; `None` nếu chưa có hoặc là của thành phố khác.
    pub fn current(&self, city: Option<&City>) -> Option<Report> {
        let city = city?;
        let report = self.report.lock().ok()?.clone()?;
        report.is_for(city).then_some(report)
    }

    /// Thành phố vừa đổi: hỏi lại ngay.
    pub fn city_changed(&self) {
        self.wake.notify_one();
    }

    fn set(&self, report: Report) {
        match serde_json::to_vec_pretty(&report) {
            Ok(bytes) => {
                if let Err(e) = write_atomic(&self.path, &bytes) {
                    eprintln!("Không lưu được weather.json: {e}");
                }
            }
            Err(e) => eprintln!("Không lưu được weather.json: {e}"),
        }
        if let Ok(mut current) = self.report.lock() {
            *current = Some(report);
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!("TinyWorld/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::internal(format!("Không tạo được HTTP client: {e}")))
}

/// Chờ trước lần hỏi tiếp theo: lần trước lỗi thì gấp đôi lần chờ trước, tối đa `REFRESH`.
fn next_retry(previous: Option<Duration>) -> Duration {
    previous.map_or(RETRY_FIRST, |d| (d * 2).min(REFRESH))
}

/// Vòng hỏi thời tiết chạy nền suốt đời app.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let client = match client() {
            Ok(client) => client,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };
        let mut retry: Option<Duration> = None;
        loop {
            let weather = app.state::<Weather>();
            let city = app.state::<SettingsStore>().get().city;
            let wait = match &city {
                // Chưa chọn thành phố: không gọi mạng, chờ người dùng chọn.
                None => None,
                Some(city) => {
                    let age = weather
                        .current(Some(city))
                        .map(|r| Duration::from_secs(now_secs().saturating_sub(r.fetched_at)));
                    match age {
                        Some(age) if age < REFRESH && retry.is_none() => Some(REFRESH - age),
                        _ => match fetch(&client, city).await {
                            Ok(report) => {
                                retry = None;
                                weather.set(report.clone());
                                if let Err(e) = app.emit(events::WEATHER_CHANGED, Some(&report)) {
                                    eprintln!("Không gửi được thời tiết: {e}");
                                }
                                Some(REFRESH)
                            }
                            Err(e) => {
                                eprintln!("Không lấy được thời tiết, thử lại sau: {e}");
                                // Chỉ báo ở lần lỗi đầu của mỗi đợt, không báo lại mỗi lần thử.
                                if retry.is_none() {
                                    let failure = Failure {
                                        offline: e.code == "OFFLINE",
                                    };
                                    if let Err(e) = app.emit(events::WEATHER_FAILED, failure) {
                                        eprintln!("Không báo được lỗi thời tiết: {e}");
                                    }
                                }
                                let wait = next_retry(retry);
                                retry = Some(wait);
                                Some(wait)
                            }
                        },
                    }
                }
            };
            match wait {
                Some(wait) => {
                    tokio::select! {
                        _ = tokio::time::sleep(wait) => {}
                        _ = weather.wake.notified() => retry = None,
                    }
                }
                None => {
                    weather.wake.notified().await;
                    retry = None;
                }
            }
        }
    });
}

/// Hỏi thời tiết hiện tại ở `city`.
async fn fetch(client: &reqwest::Client, city: &City) -> AppResult<Report> {
    let body = client
        .get(FORECAST_URL)
        .query(&[
            ("latitude", city.latitude.to_string()),
            ("longitude", city.longitude.to_string()),
            ("current", "weather_code,is_day,temperature_2m,wind_speed_10m".into()),
        ])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(network_error)?
        .text()
        .await
        .map_err(network_error)?;
    parse_forecast(&body, city, now_secs())
}

/// Tìm thành phố theo tên (command `search_city`), tên tiếng Việt nếu có.
pub async fn search(query: &str) -> AppResult<Vec<CityResult>> {
    let query = query.trim();
    let len = query.chars().count();
    if len < QUERY_LEN.0 || len > QUERY_LEN.1 {
        return Ok(Vec::new());
    }
    let body = client()?
        .get(GEOCODING_URL)
        .query(&[("name", query), ("count", SEARCH_COUNT), ("language", "vi"), ("format", "json")])
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(network_error)?
        .text()
        .await
        .map_err(network_error)?;
    parse_search(&body)
}

/// Lỗi mạng: mất mạng, hết giờ chờ, máy chủ trả lỗi.
fn network_error(e: reqwest::Error) -> AppError {
    if e.is_connect() || e.is_timeout() {
        AppError::offline("Mất mạng hoặc máy chủ thời tiết không trả lời.")
    } else {
        AppError::internal(format!("Lỗi khi hỏi thời tiết: {e}"))
    }
}

#[derive(Deserialize)]
struct ForecastBody {
    current: CurrentBody,
}

#[derive(Deserialize)]
struct CurrentBody {
    weather_code: u16,
    is_day: u8,
    temperature_2m: f64,
    wind_speed_10m: f64,
}

fn parse_forecast(body: &str, city: &City, fetched_at: u64) -> AppResult<Report> {
    let body: ForecastBody = serde_json::from_str(body)
        .map_err(|e| AppError::internal(format!("Open-Meteo trả về dữ liệu lạ: {e}")))?;
    let current = body.current;
    Ok(Report {
        latitude: city.latitude,
        longitude: city.longitude,
        code: current.weather_code,
        is_day: current.is_day != 0,
        temperature: current.temperature_2m,
        wind_speed: current.wind_speed_10m,
        fetched_at,
    })
}

#[derive(Deserialize)]
struct SearchBody {
    #[serde(default)]
    results: Vec<SearchItem>,
}

#[derive(Deserialize)]
struct SearchItem {
    name: String,
    latitude: f64,
    longitude: f64,
    #[serde(default)]
    admin1: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    timezone: Option<String>,
}

fn parse_search(body: &str) -> AppResult<Vec<CityResult>> {
    let body: SearchBody = serde_json::from_str(body)
        .map_err(|e| AppError::internal(format!("Open-Meteo trả về dữ liệu lạ: {e}")))?;
    Ok(body
        .results
        .into_iter()
        .map(|item| {
            let mut parts = vec![item.name.clone()];
            parts.extend(item.admin1.filter(|a| !a.is_empty() && *a != item.name));
            parts.extend(item.country.filter(|c| !c.is_empty()));
            CityResult {
                label: parts.join(", "),
                latitude: item.latitude,
                longitude: item.longitude,
                timezone: item.timezone,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hanoi() -> City {
        City {
            name: "Hà Nội, Việt Nam".into(),
            latitude: 21.0245,
            longitude: 105.84117,
            timezone: Some("Asia/Bangkok".into()),
        }
    }

    #[test]
    fn doc_thoi_tiet_hien_tai() {
        let body = r#"{"latitude":21.0,"longitude":105.875,"current_units":{},"current":{"time":"2026-10-01T15:00","interval":900,"weather_code":61,"is_day":1,"temperature_2m":27.4,"wind_speed_10m":6.1}}"#;
        let report = parse_forecast(body, &hanoi(), 1000).unwrap();
        assert_eq!(report.code, 61);
        assert!(report.is_day);
        assert_eq!(report.temperature, 27.4);
        // Lưu theo toạ độ người dùng chọn chứ không theo toạ độ ô lưới Open-Meteo trả về.
        assert!(report.is_for(&hanoi()));
        assert!(parse_forecast("{}", &hanoi(), 0).is_err());
    }

    #[test]
    fn doc_ket_qua_tim_thanh_pho() {
        let body = r#"{"results":[
            {"name":"Hà Nội","latitude":21.0245,"longitude":105.84117,"country":"Việt Nam","admin1":"Hà Nội","timezone":"Asia/Bangkok"},
            {"name":"Hanoi","latitude":44.1,"longitude":-83.2,"country":"Hoa Kỳ","admin1":"Michigan"}
        ],"generationtime_ms":0.5}"#;
        let results = parse_search(body).unwrap();
        assert_eq!(results[0].label, "Hà Nội, Việt Nam");
        assert_eq!(results[0].timezone.as_deref(), Some("Asia/Bangkok"));
        assert_eq!(results[1].label, "Hanoi, Michigan, Hoa Kỳ");
        assert_eq!(results[1].timezone, None);
        // Không tìm thấy gì thì Open-Meteo bỏ hẳn `results`.
        assert!(parse_search(r#"{"generationtime_ms":0.2}"#).unwrap().is_empty());
    }

    #[test]
    fn ket_qua_chi_dung_cho_dung_thanh_pho() {
        let dir = std::env::temp_dir().join(format!("tinyworld-weather-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let weather = Weather::load(&dir);
        assert_eq!(weather.current(Some(&hanoi())), None);
        let report = parse_forecast(
            r#"{"current":{"weather_code":0,"is_day":0,"temperature_2m":20.0,"wind_speed_10m":1.0}}"#,
            &hanoi(),
            5,
        )
        .unwrap();
        weather.set(report.clone());
        assert_eq!(weather.current(Some(&hanoi())), Some(report.clone()));
        assert_eq!(weather.current(None), None);
        let hue = City {
            name: "Huế".into(),
            latitude: 16.46,
            longitude: 107.59,
            timezone: None,
        };
        assert_eq!(weather.current(Some(&hue)), None);
        // Mở lại app: đọc lại từ weather.json.
        assert_eq!(Weather::load(&dir).current(Some(&hanoi())), Some(report));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn loi_thi_cho_lau_dan() {
        let mut wait = None;
        let mut seen = Vec::new();
        for _ in 0..7 {
            let next = next_retry(wait);
            seen.push(next.as_secs());
            wait = Some(next);
        }
        assert_eq!(seen, [60, 120, 240, 480, 960, 1800, 1800]);
    }
}
