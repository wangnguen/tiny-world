//! Gợi ý cập nhật: hỏi GitHub bản phát hành mới nhất của TinyWorld (repo public, không cần token), mới hơn
//! bản đang chạy thì báo Cài đặt và overlay (một con nói một câu, mỗi bản một lần). Người dùng bấm Cập nhật
//! ngay thì tải bộ cài về thư mục tạm, kiểm tra đúng cỡ file, rồi thoát app như bấm Thoát (overlay lưu trạng
//! thái); lúc app thoát hẳn (`launch_pending`) chạy bộ cài ở chế độ tự động: cài đè lên bản cũ, xong tự mở
//! lại. Bản portable chạy bộ cài thì thành bản cài đặt. Bộ cài chưa ký số nên chỉ tải từ đúng trang release
//! của repo.
//!
//! Bản dev (debug) có version 0.1.0 nên luôn thấy "có bản mới": không tự hỏi, trừ khi đặt biến môi trường
//! `TINYWORLD_CHECK_UPDATE`. Nút Kiểm tra bản mới trong Cài đặt thì vẫn hỏi.

use crate::app;
use crate::error::{AppError, AppResult};
use crate::{events, i18n};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Notify;

const LATEST_URL: &str = "https://api.github.com/repos/wangnguen/tiny-world/releases/latest";
/// Bộ cài chỉ được tải từ đây (trang release của repo).
const DOWNLOAD_PREFIX: &str = "https://github.com/wangnguen/tiny-world/releases/download/";
/// Trang release chỉ được mở nếu nằm ở đây.
const PAGE_PREFIX: &str = "https://github.com/wangnguen/tiny-world/releases/";
/// Mở app chừng này lâu mới hỏi lần đầu (không tranh mạng lúc vừa mở), rồi chừng này lâu hỏi lại.
const FIRST_CHECK: Duration = Duration::from_secs(20);
const RECHECK: Duration = Duration::from_secs(6 * 3600);
/// Hỏi lỗi (mất mạng...) thì chừng này lâu hỏi lại.
const RETRY: Duration = Duration::from_secs(30 * 60);
const TIMEOUT: Duration = Duration::from_secs(20);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);
/// Bộ cài to hơn chừng này thì không phải bộ cài TinyWorld.
const MAX_SETUP_BYTES: u64 = 200 * 1024 * 1024;
/// Cờ của bộ cài NSIS Tauri: `/P` tự chạy chỉ hiện tiến trình, `/UPDATE` cài đè không gỡ bản cũ, `/R` mở lại
/// app sau khi cài xong.
const INSTALLER_ARGS: [&str; 3] = ["/P", "/UPDATE", "/R"];
/// Báo tiến độ tải mỗi lần được thêm chừng này phần trăm.
const PROGRESS_STEP: u64 = 2;
/// Bộ cài tải về nằm trong thư mục này của thư mục tạm; lần tải sau xoá bộ cài cũ trong đó.
const DOWNLOAD_DIR: &str = "TinyWorld-update";

/// Một bản phát hành mới hơn bản đang chạy.
#[derive(Debug, Clone, PartialEq)]
struct Release {
    version: String,
    page: String,
    setup_url: String,
    setup_name: String,
    setup_size: u64,
}

/// Event `update-available` và command `get_update`, khớp `UpdateInfo` trong packages/core.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// Bản mới, ví dụ "1.6.0".
    pub version: String,
    /// Bản đang chạy.
    pub current: String,
    /// Trang release (có gì mới).
    pub page: String,
    /// Cỡ bộ cài (byte).
    pub size: u64,
}

/// Event `update-progress`, khớp `UpdateProgress` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub received: u64,
    pub total: u64,
}

pub struct Updater {
    latest: Mutex<Option<Release>>,
    /// Bộ cài đã tải xong, chạy lúc app thoát hẳn.
    pending: Mutex<Option<PathBuf>>,
    /// Đang tải bộ cài (bấm Cập nhật ngay hai lần thì lần sau bị từ chối).
    installing: AtomicBool,
    /// Bấm Kiểm tra bản mới: vòng hỏi chạy lại lịch từ đầu.
    wake: Notify,
}

impl Default for Updater {
    fn default() -> Self {
        Self {
            latest: Mutex::new(None),
            pending: Mutex::new(None),
            installing: AtomicBool::new(false),
            wake: Notify::new(),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn current_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

fn info(release: &Release, current: &str) -> UpdateInfo {
    UpdateInfo {
        version: release.version.clone(),
        current: current.to_owned(),
        page: release.page.clone(),
        size: release.setup_size,
    }
}

/// "v1.6.0" hay "1.6.0" thành (1, 6, 0); dạng khác (có hậu tố...) thì `None`.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((major, minor, patch))
}

#[derive(Deserialize)]
struct ReleaseBody {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<AssetBody>,
}

#[derive(Deserialize)]
struct AssetBody {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// Đọc bản phát hành mới nhất từ GitHub; `None` nếu không mới hơn `current` hay không có bộ cài dùng được.
fn parse_release(body: &str, current: &str) -> AppResult<Option<Release>> {
    let body: ReleaseBody = serde_json::from_str(body)
        .map_err(|e| AppError::internal(i18n::tf("errors.updateOdd", &[("error", &e)])))?;
    let (Some(latest), Some(now)) = (parse_version(&body.tag_name), parse_version(current)) else {
        return Ok(None);
    };
    if latest <= now || !body.html_url.starts_with(PAGE_PREFIX) {
        return Ok(None);
    }
    let setup = body.assets.into_iter().find(|a| {
        a.name.ends_with("-setup.exe")
            && a.browser_download_url.starts_with(DOWNLOAD_PREFIX)
            && a.size > 0
            && a.size <= MAX_SETUP_BYTES
    });
    Ok(setup.map(|setup| Release {
        version: format!("{}.{}.{}", latest.0, latest.1, latest.2),
        page: body.html_url,
        setup_url: setup.browser_download_url,
        setup_name: setup.name,
        setup_size: setup.size,
    }))
}

fn client(timeout: Duration) -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(timeout)
        .user_agent(concat!("TinyWorld/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::internal(i18n::tf("errors.httpClient", &[("error", &e)])))
}

fn network_error(e: reqwest::Error) -> AppError {
    if e.is_connect() || e.is_timeout() {
        AppError::offline(i18n::t("errors.updateOffline"))
    } else if matches!(e.status().map(|s| s.as_u16()), Some(403 | 429)) {
        // Không có token thì GitHub cho mỗi IP 60 lượt hỏi một giờ (mạng công ty dùng chung IP dễ hết).
        AppError::unavailable(i18n::t("errors.updateLimited"))
    } else {
        AppError::internal(i18n::tf("errors.updateFailed", &[("error", &e)]))
    }
}

/// Hỏi GitHub bản phát hành mới nhất; `None` nếu không mới hơn `current`.
async fn fetch_latest(current: &str) -> AppResult<Option<Release>> {
    let body = client(TIMEOUT)?
        .get(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(network_error)?
        .text()
        .await
        .map_err(network_error)?;
    parse_release(&body, current)
}

/// Hỏi GitHub, lưu kết quả; có bản mới thì báo mọi cửa sổ (Cài đặt hiện thẻ cập nhật, pet nói một câu).
async fn check(app: &AppHandle) -> AppResult<Option<UpdateInfo>> {
    let current = current_version(app);
    let release = fetch_latest(&current).await?;
    let found = release.as_ref().map(|r| info(r, &current));
    *lock(&app.state::<Updater>().latest) = release;
    if let Some(found) = &found {
        if let Err(e) = app.emit(events::UPDATE_AVAILABLE, found) {
            eprintln!("Không báo được bản mới: {e}");
        }
    }
    Ok(found)
}

/// Bản build thật thì tự hỏi; bản dev chỉ khi đặt `TINYWORLD_CHECK_UPDATE`.
fn auto_check() -> bool {
    !cfg!(debug_assertions) || std::env::var_os("TINYWORLD_CHECK_UPDATE").is_some()
}

/// Vòng hỏi bản mới chạy nền suốt đời app.
pub fn spawn(app: AppHandle) {
    if !auto_check() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK).await;
        loop {
            let wait = match check(&app).await {
                Ok(_) => RECHECK,
                Err(e) => {
                    eprintln!("Không hỏi được bản mới, thử lại sau: {e}");
                    RETRY
                }
            };
            let updater = app.state::<Updater>();
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                _ = updater.wake.notified() => {}
            }
        }
    });
}

/// Cài đặt vừa mở: có bản mới không (theo lần hỏi gần nhất).
#[tauri::command]
pub fn get_update(app: AppHandle, updater: State<'_, Updater>) -> Option<UpdateInfo> {
    let current = current_version(&app);
    lock(&updater.latest).as_ref().map(|r| info(r, &current))
}

/// Nút Kiểm tra bản mới: hỏi ngay; `None` là đang dùng bản mới nhất.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<Option<UpdateInfo>> {
    let found = check(&app).await?;
    // Vòng hỏi nền tính lại lịch từ lần hỏi này.
    app.state::<Updater>().wake.notify_one();
    Ok(found)
}

/// Nút Cập nhật ngay: tải bộ cài (báo tiến độ qua `update-progress`), kiểm tra cỡ file, rồi thoát app; bộ
/// cài chạy lúc app thoát hẳn (`launch_pending`).
///
/// Bản dev không cài: bộ cài ghi đè bản TinyWorld đã cài thật trong máy chứ không phải bản đang chạy thử.
/// Muốn thử cả luồng thì đặt biến môi trường `TINYWORLD_INSTALL_UPDATE`.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    if cfg!(debug_assertions) && std::env::var_os("TINYWORLD_INSTALL_UPDATE").is_none() {
        return Err(AppError::bad_request(i18n::t("errors.updateDev")));
    }
    let release = lock(&app.state::<Updater>().latest)
        .clone()
        .ok_or_else(|| AppError::bad_request(i18n::t("errors.updateNone")))?;
    let updater = app.state::<Updater>();
    if updater.installing.swap(true, Ordering::SeqCst) {
        return Err(AppError::busy(i18n::t("errors.updateBusy")));
    }
    let path = match download(&app, &release).await {
        Ok(path) => path,
        Err(e) => {
            updater.installing.store(false, Ordering::SeqCst);
            return Err(e);
        }
    };
    // Tải xong thì giữ `installing`: app đang thoát, không tải thêm lần nữa.
    *lock(&updater.pending) = Some(path);
    app::request_quit(&app);
    Ok(())
}

/// Tải bộ cài về thư mục tạm, báo tiến độ cho mọi cửa sổ.
async fn download(app: &AppHandle, release: &Release) -> AppResult<PathBuf> {
    let dir = std::env::temp_dir().join(DOWNLOAD_DIR);
    download_to(release, &dir, |progress| {
        if let Err(e) = app.emit(events::UPDATE_PROGRESS, progress) {
            eprintln!("Không báo được tiến độ tải: {e}");
        }
    })
    .await
}

/// Tải bộ cài vào `dir` (bộ cài cũ trong đó bị xoá trước), đúng cỡ GitHub báo thì giữ. `progress` được gọi
/// mỗi `PROGRESS_STEP` phần trăm và lúc xong.
async fn download_to(
    release: &Release,
    dir: &Path,
    mut progress: impl FnMut(UpdateProgress),
) -> AppResult<PathBuf> {
    if !release.setup_url.starts_with(DOWNLOAD_PREFIX) {
        return Err(AppError::bad_request(i18n::t("errors.updateBadLink")));
    }
    let mut response = client(DOWNLOAD_TIMEOUT)?
        .get(&release.setup_url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(network_error)?;
    let total = release.setup_size;
    let mut data = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
    let mut reported = 0;
    while let Some(chunk) = response.chunk().await.map_err(network_error)? {
        data.extend_from_slice(&chunk);
        if data.len() as u64 > total {
            return Err(AppError::internal(i18n::t("errors.updateTooBig")));
        }
        let percent = data.len() as u64 * 100 / total;
        if percent >= reported + PROGRESS_STEP || data.len() as u64 == total {
            reported = percent;
            progress(UpdateProgress { received: data.len() as u64, total });
        }
    }
    if data.len() as u64 != total {
        return Err(AppError::offline(i18n::t("errors.updateCut")));
    }
    // Bộ cài của lần cập nhật trước (đã chạy xong) không cần nữa.
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir)
        .map_err(|e| AppError::internal(i18n::tf("errors.updateFolder", &[("error", &e)])))?;
    let path = dir.join(installer_name(&release.setup_name, &release.version));
    std::fs::write(&path, &data)
        .map_err(|e| AppError::internal(i18n::tf("errors.updateSave", &[("error", &e)])))?;
    Ok(path)
}

/// Tên file bộ cài lưu trong thư mục tạm: chỉ giữ chữ, số, `._-` cho chắc.
fn installer_name(name: &str, version: &str) -> String {
    let clean: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    if clean.ends_with(".exe") && clean.len() > 4 {
        clean
    } else {
        format!("TinyWorld_{version}_x64-setup.exe")
    }
}

/// App sắp thoát hẳn: có bộ cài đã tải thì chạy nó (cài đè, xong tự mở lại app).
pub fn launch_pending(app: &AppHandle) {
    let Some(updater) = app.try_state::<Updater>() else {
        return;
    };
    let Some(path) = lock(&updater.pending).take() else {
        return;
    };
    if let Err(e) = std::process::Command::new(&path).args(INSTALLER_ARGS).spawn() {
        eprintln!("Không chạy được bộ cài {}: {e}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, assets: &str) -> String {
        format!(
            r#"{{"tag_name":"{tag}","html_url":"https://github.com/wangnguen/tiny-world/releases/tag/{tag}","draft":false,"assets":[{assets}]}}"#
        )
    }

    const SETUP: &str = r#"{"name":"TinyWorld_1.6.0_x64-setup.exe","size":5000000,"browser_download_url":"https://github.com/wangnguen/tiny-world/releases/download/v1.6.0/TinyWorld_1.6.0_x64-setup.exe"}"#;
    const PORTABLE: &str = r#"{"name":"TinyWorld_1.6.0_x64-portable.exe","size":9000000,"browser_download_url":"https://github.com/wangnguen/tiny-world/releases/download/v1.6.0/TinyWorld_1.6.0_x64-portable.exe"}"#;

    #[test]
    fn doc_version() {
        assert_eq!(parse_version("v1.6.0"), Some((1, 6, 0)));
        assert_eq!(parse_version("1.10.2"), Some((1, 10, 2)));
        assert_eq!(parse_version("v1.6"), None);
        assert_eq!(parse_version("v1.6.0-beta"), None);
        assert_eq!(parse_version("v1.6.0.1"), None);
        // So theo số chứ không theo chữ: 1.10 mới hơn 1.9.
        assert!(parse_version("1.10.0") > parse_version("1.9.9"));
    }

    #[test]
    fn co_ban_moi_thi_lay_bo_cai() {
        let body = release("v1.6.0", &format!("{PORTABLE},{SETUP}"));
        let found = parse_release(&body, "1.5.0").unwrap().unwrap();
        assert_eq!(found.version, "1.6.0");
        assert_eq!(found.setup_name, "TinyWorld_1.6.0_x64-setup.exe");
        assert_eq!(found.setup_size, 5_000_000);
        assert!(found.page.starts_with(PAGE_PREFIX));
    }

    #[test]
    fn khong_moi_hon_hay_khong_co_bo_cai_thi_thoi() {
        let body = release("v1.5.0", SETUP);
        assert_eq!(parse_release(&body, "1.5.0").unwrap(), None);
        assert_eq!(parse_release(&body, "1.6.0").unwrap(), None);
        // Chỉ có bản portable: không cập nhật tự động được.
        assert_eq!(parse_release(&release("v1.6.0", PORTABLE), "1.5.0").unwrap(), None);
        // Bộ cài nằm ở chỗ khác trang release của repo: bỏ.
        let elsewhere = SETUP.replace("github.com/wangnguen", "example.com/wangnguen");
        assert_eq!(parse_release(&release("v1.6.0", &elsewhere), "1.5.0").unwrap(), None);
        assert!(parse_release("{}", "1.5.0").is_err());
    }

    #[test]
    fn ten_bo_cai_trong_thu_muc_tam_an_toan() {
        let setup = "TinyWorld_1.6.0_x64-setup.exe";
        assert_eq!(installer_name(setup, "1.6.0"), setup);
        // Không có dấu gạch chéo nào lọt qua: file luôn nằm ngay trong thư mục tạm.
        assert_eq!(installer_name("..\\..\\evil.exe", "1.6.0"), "....evil.exe");
        assert_eq!(installer_name("setup.msi", "1.6.0"), setup);
    }

    /// Hỏi GitHub thật rồi tải bộ cài của bản mới nhất (vài MB). Cần mạng nên bỏ qua mặc định:
    /// `cargo test -- --ignored github_that`.
    #[test]
    #[ignore = "cần mạng, gọi GitHub thật"]
    fn github_that() {
        tauri::async_runtime::block_on(async {
            let release = fetch_latest("0.0.1").await.unwrap().expect("repo chưa có release kèm bộ cài");
            assert!(release.setup_url.starts_with(DOWNLOAD_PREFIX), "{}", release.setup_url);
            assert!(release.page.starts_with(PAGE_PREFIX), "{}", release.page);
            assert!(parse_version(&release.version).is_some());

            let dir = std::env::temp_dir().join("TinyWorld-update-test");
            let mut last = None;
            let path = download_to(&release, &dir, |p| last = Some(p)).await.unwrap();
            let data = std::fs::read(&path).unwrap();
            let _ = std::fs::remove_dir_all(&dir);
            assert_eq!(data.len() as u64, release.setup_size);
            assert_eq!(last, Some(UpdateProgress { received: release.setup_size, total: release.setup_size }));
            assert_eq!(&data[..2], b"MZ", "không phải file .exe");
            assert_eq!(path.file_name().unwrap().to_str().unwrap(), release.setup_name);
        });
    }
}
