//! Chat với pet (Phase 6): cửa sổ chat nhỏ cạnh pet, hỏi Gemini (`gemini.rs`: không cần key hay đăng
//! nhập, không chạy server, không mở cổng). Chỉ Rust gọi mạng. TinyWorld không lưu đoạn chat, đóng cửa sổ
//! là mất.
//!
//! Tránh bị Google chặn IP: chỉ gọi khi người dùng bấm gửi, mỗi lúc một câu, hai câu cách nhau ít nhất
//! `MIN_GAP`, tối đa `PER_HOUR` câu mỗi giờ; Google trả lỗi hay trả rỗng thì chờ lâu dần mới cho gửi tiếp.
//! App tự chặn trước, khung chat ghi lý do và lúc gửi lại được. Câu gợi ý trong khung chat do Gemini viết,
//! mỗi ngày một lần (giữ trong bộ nhớ), ngoài giới hạn của người dùng.

use crate::error::{AppError, AppResult};
use crate::events;
use crate::gemini::{Gemini, GeminiError};
use crate::overlay::{self, Overlay, ScreenInfo};
use crate::settings::SettingsStore;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::window::Color;
use tauri::{
    AppHandle, Emitter, LogicalPosition, Manager, Theme, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub const WINDOW_LABEL: &str = "chat";
/// Cỡ phần trong cửa sổ chat (CSS pixel), cách pet chừng này.
const WIDTH: f64 = 360.0;
const HEIGHT: f64 = 500.0;
const GAP: f64 = 16.0;
/// Viền và thanh tiêu đề (CSS pixel) trước khi đo được cửa sổ thật: Windows 10/11 ở 100%.
const FRAME_GUESS: (f64, f64) = (16.0, 39.0);
/// Câu hỏi kèm vài lượt chat gần nhất, tối đa chừng này ký tự.
const PROMPT_MAX_CHARS: usize = 8000;
const NAME_MAX_CHARS: usize = 40;
const MIN_GAP: Duration = Duration::from_secs(3);
const PER_HOUR: usize = 30;
const HOUR: Duration = Duration::from_secs(3600);
const BACKOFF_START: Duration = Duration::from_secs(30);
const BACKOFF_MAX: Duration = Duration::from_secs(30 * 60);
/// Chờ Google trả lời tối đa chừng này.
const TIMEOUT: Duration = Duration::from_secs(60);
/// Câu gợi ý: hỏi lỗi thì chừng này sau mới hỏi lại; giữ tối đa chừng này câu, mỗi câu tối đa chừng này
/// ký tự.
const SUGGEST_RETRY: Duration = Duration::from_secs(3600);
const SUGGEST_COUNT: usize = 8;
const SUGGEST_MAX_CHARS: usize = 60;

/// Pet được click chuột phải (CSS pixel của overlay).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Đang chat với con nào, khớp `ChatTarget` trong packages/core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatTarget {
    /// Tên thư mục sprite pack ("placeholder" là pet tạm).
    pub pet: String,
    /// Tên ngắn để hiện, ví dụ "Momo".
    pub name: String,
}

/// Vì sao chưa cho gửi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Wait {
    /// Câu trước chưa trả lời xong.
    Busy,
    /// Vừa gửi xong, còn chừng này nữa.
    TooFast(Duration),
    /// Đủ số câu trong một giờ, còn chừng này nữa.
    Hourly(Duration),
    /// Google vừa lỗi hay chặn, còn chừng này nữa.
    Backoff(Duration),
}

/// Kết quả một lần gửi, để tính lần sau.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    Ok,
    /// Mất mạng, hết giờ chờ: không phải lỗi của Google, không chờ lâu dần.
    Offline,
    /// Google trả lỗi, chặn hay trả rỗng.
    Failed,
}

/// Giới hạn tần suất gửi, không gọi mạng nên test được.
#[derive(Debug, Default)]
pub struct Limiter {
    sent: VecDeque<Instant>,
    in_flight: bool,
    blocked_until: Option<Instant>,
    backoff: Option<Duration>,
}

impl Limiter {
    /// Cho gửi lúc `now` không; cho thì tính là đã bắt đầu gửi.
    pub fn begin(&mut self, now: Instant) -> Result<(), Wait> {
        if self.in_flight {
            return Err(Wait::Busy);
        }
        if let Some(until) = self.blocked_until.filter(|&until| until > now) {
            return Err(Wait::Backoff(until - now));
        }
        while self.sent.front().is_some_and(|&t| now.duration_since(t) >= HOUR) {
            self.sent.pop_front();
        }
        if let Some(&last) = self.sent.back() {
            let since = now.duration_since(last);
            if since < MIN_GAP {
                return Err(Wait::TooFast(MIN_GAP - since));
            }
        }
        if self.sent.len() >= PER_HOUR {
            let oldest = self.sent[0];
            return Err(Wait::Hourly(HOUR - now.duration_since(oldest)));
        }
        self.sent.push_back(now);
        self.in_flight = true;
        Ok(())
    }

    /// Lần gửi vừa xong lúc `now`.
    pub fn finish(&mut self, now: Instant, outcome: Outcome) {
        self.in_flight = false;
        match outcome {
            Outcome::Ok => {
                self.backoff = None;
                self.blocked_until = None;
            }
            Outcome::Offline => {}
            Outcome::Failed => {
                let wait = self.backoff.map_or(BACKOFF_START, |b| (b * 2).min(BACKOFF_MAX));
                self.backoff = Some(wait);
                self.blocked_until = Some(now + wait);
            }
        }
    }
}

/// "3 giây", "12 phút".
fn duration_text(d: Duration) -> String {
    let secs = d.as_secs_f64().ceil() as u64;
    if secs < 60 {
        format!("{} giây", secs.max(1))
    } else {
        format!("{} phút", secs.div_ceil(60))
    }
}

/// Câu ngắn cho khung chat, không lộ lỗi kỹ thuật.
fn wait_error(wait: Wait) -> AppError {
    match wait {
        Wait::Busy => AppError::busy("Đang trả lời câu trước, đợi chút nhé :)"),
        Wait::TooFast(d) => AppError::busy(format!("Từ từ thôi, gửi lại sau {} nhé :)", duration_text(d))),
        Wait::Hourly(d) => AppError::busy(format!(
            "Hỏi nhiều quá rồi, nghỉ chút đã, gửi lại được sau {} :)",
            duration_text(d)
        )),
        Wait::Backoff(d) => AppError::unavailable(format!(
            "Google đang bận, gửi lại được sau {}",
            duration_text(d)
        )),
    }
}

/// Câu gợi ý của ngày, do Gemini viết.
#[derive(Default)]
struct Suggestions {
    /// Ngày "2026-10-01" và các câu của ngày đó.
    day: Option<(String, Vec<String>)>,
    /// Lần hỏi gần nhất bị lỗi.
    failed_at: Option<Instant>,
    in_flight: bool,
}

pub struct Chat {
    gemini: Gemini,
    limiter: Mutex<Limiter>,
    target: Mutex<Option<ChatTarget>>,
    suggestions: Mutex<Suggestions>,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            gemini: Gemini::new(TIMEOUT),
            limiter: Mutex::new(Limiter::default()),
            target: Mutex::new(None),
            suggestions: Mutex::new(Suggestions::default()),
        }
    }
}

impl Chat {
    pub fn target(&self) -> Option<ChatTarget> {
        self.target.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set_target(&self, target: Option<ChatTarget>) {
        *self.target.lock().unwrap_or_else(|e| e.into_inner()) = target;
    }

    /// Câu gợi ý của hôm nay (`today` "2026-10-01", `date` để ghi vào câu hỏi). Chưa có thì hỏi Gemini một
    /// lần; đang hỏi, vừa hỏi lỗi hay Google đang bận thì trả danh sách rỗng (khung chat dùng câu có sẵn).
    pub async fn suggestions(&self, today: &str, date: &str) -> Vec<String> {
        {
            let mut s = self.suggestions.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((day, list)) = &s.day {
                if day == today {
                    return list.clone();
                }
            }
            let recent_failure = s.failed_at.is_some_and(|t| t.elapsed() < SUGGEST_RETRY);
            let blocked = self
                .limiter
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .blocked_until
                .is_some_and(|until| until > Instant::now());
            if s.in_flight || recent_failure || blocked {
                return Vec::new();
            }
            s.in_flight = true;
        }
        let result = self.gemini.chat(&suggest_prompt(date)).await;
        let mut s = self.suggestions.lock().unwrap_or_else(|e| e.into_inner());
        s.in_flight = false;
        let list = result.map(|reply| parse_suggestions(&reply.text)).unwrap_or_default();
        if list.is_empty() {
            s.failed_at = Some(Instant::now());
        } else {
            s.day = Some((today.to_string(), list.clone()));
        }
        list
    }

    /// Gửi `prompt` (đã kèm tính cách pet, giờ, vài lượt chat gần nhất) và chờ câu trả lời.
    pub async fn send(&self, prompt: &str) -> AppResult<String> {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err(AppError::bad_request("Chưa có gì để gửi."));
        }
        if prompt.chars().count() > PROMPT_MAX_CHARS {
            return Err(AppError::bad_request("Câu hỏi dài quá, cắt bớt nhé."));
        }
        self.limiter
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .begin(Instant::now())
            .map_err(wait_error)?;
        let result = self.gemini.chat(prompt).await;
        let outcome = match &result {
            Ok(_) => Outcome::Ok,
            Err(GeminiError::Offline | GeminiError::Timeout) => Outcome::Offline,
            Err(_) => Outcome::Failed,
        };
        let mut limiter = self.limiter.lock().unwrap_or_else(|e| e.into_inner());
        limiter.finish(Instant::now(), outcome);
        match result {
            Ok(reply) => Ok(reply.text),
            Err(GeminiError::Offline) => Err(AppError::offline("Mất mạng")),
            Err(GeminiError::Timeout) => Err(AppError::unavailable("Lâu quá Google chưa trả lời, gửi lại thử nhé")),
            Err(e) => {
                eprintln!("Chat lỗi: {e}");
                let wait = limiter.blocked_until.map(|until| until.saturating_duration_since(Instant::now()));
                Err(wait_error(Wait::Backoff(wait.unwrap_or(BACKOFF_START))))
            }
        }
    }
}

/// Câu nhờ Gemini viết câu gợi ý cho ngày `date` ("Thứ Năm 01/10/2026").
fn suggest_prompt(date: &str) -> String {
    format!(
        "Hôm nay là {date}. Gợi ý {SUGGEST_COUNT} câu hỏi ngắn (mỗi câu dưới 45 ký tự) mà dân văn phòng ở Việt \
         Nam hay muốn hỏi nhanh hôm nay: tỉ giá, giá vàng, tin tức, thời tiết, đổi đơn vị, mẹo dùng máy tính, \
         viết giúp một câu... Đa dạng, hợp với ngày hôm nay (ngày lễ, dịp gần đây nếu có). Mỗi câu một dòng, \
         không đánh số, không giải thích."
    )
}

/// Tách câu trả lời thành các câu gợi ý: bỏ gạch đầu dòng, số thứ tự, ngoặc kép, dòng tiêu đề, câu quá
/// ngắn hay quá dài, câu trùng.
fn parse_suggestions(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines() {
        let mut s = line.trim().trim_start_matches(['-', '*', '•']).trim_start();
        let digits = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        // "1. ", "2) " là số thứ tự; "1 inch", "1.5 lít" là chữ của câu.
        let numbered = s[digits..].starts_with(['.', ')']) && s[digits + 1..].starts_with(char::is_whitespace);
        if digits > 0 && numbered {
            s = s[digits + 1..].trim_start();
        }
        let s = s.trim_matches(['"', '“', '”', '*']).trim();
        let len = s.chars().count();
        if (4..=SUGGEST_MAX_CHARS).contains(&len) && !s.ends_with(':') && !out.iter().any(|known| known == s) {
            out.push(s.to_string());
        }
        if out.len() == SUGGEST_COUNT {
            break;
        }
    }
    out
}

/// Mở cửa sổ chat với `target` cạnh `pet`. Đang mở thì đổi sang pet đó, dời tới cạnh nó và đưa lên trước.
/// Trả về phía của cửa sổ so với pet (-1 bên trái, 1 bên phải) để pet quay mặt về phía đó.
pub fn open(app: &AppHandle, mut target: ChatTarget, pet: PetBox) -> AppResult<i8> {
    if !app.state::<SettingsStore>().get().chat {
        return Err(AppError::bad_request("Chat với pet đang tắt trong Cài đặt."));
    }
    target.name = target.name.trim().chars().take(NAME_MAX_CHARS).collect();
    if target.pet.is_empty() || target.name.is_empty() {
        return Err(AppError::bad_request("Thiếu tên pet."));
    }
    app.state::<Chat>().set_target(Some(target.clone()));
    let side = side_of(&app.state::<Overlay>().geometry().screen, pet, WIDTH + FRAME_GUESS.0);
    let app = app.clone();
    // Tạo webview trong command trên Windows có thể treo (như cửa sổ Cài đặt), nên chạy ở luồng riêng.
    std::thread::spawn(move || {
        if let Err(e) = show_or_create(&app, &target, pet, side) {
            eprintln!("Không mở được cửa sổ chat: {e}");
        }
    });
    Ok(side)
}

/// Phía đặt cửa sổ rộng `width` (cả viền) cạnh `pet`: bên trái nếu đủ chỗ (-1), không thì bên phải (1).
fn side_of(screen: &ScreenInfo, pet: PetBox, width: f64) -> i8 {
    if pet.x - GAP - width >= screen.work_area.x {
        -1
    } else {
        1
    }
}

/// Góc trên trái (CSS pixel của overlay) của cửa sổ cỡ `size` (cả viền và thanh tiêu đề) ở phía `side` của
/// `pet`, không đè lên pet, ngang tầm giữa pet, luôn nằm trọn trong vùng làm việc (không lấn xuống taskbar).
fn place(screen: &ScreenInfo, pet: PetBox, (width, height): (f64, f64), side: i8) -> (f64, f64) {
    let area = &screen.work_area;
    let left = if side < 0 { pet.x - GAP - width } else { pet.x + pet.width + GAP };
    let left = left.clamp(area.x, (area.x + area.width - width).max(area.x));
    let top = (pet.y + pet.height / 2.0 - height / 2.0).clamp(area.y, (area.y + area.height - height).max(area.y));
    (left, top)
}

fn show_or_create(app: &AppHandle, target: &ChatTarget, pet: PetBox, side: i8) -> tauri::Result<()> {
    let overlay = app.state::<Overlay>().geometry();
    // Toạ độ overlay đổi sang toạ độ màn hình (logical) theo vị trí và DPI của cửa sổ overlay.
    let scale = overlay.screen.scale_factor;
    let to_screen = |(x, y): (f64, f64)| {
        LogicalPosition::new(f64::from(overlay.window.x) / scale + x, f64::from(overlay.window.y) / scale + y)
    };
    let title = format!("Chat với {}", target.name);
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.set_title(&title)?;
        app.emit_to(WINDOW_LABEL, events::CHAT_TARGET, target)?;
        window.unminimize()?;
        window.set_position(to_screen(place(&overlay.screen, pet, outer_size(&window)?, side)))?;
        window.show()?;
        return window.set_focus();
    }
    let guess = (WIDTH + FRAME_GUESS.0, HEIGHT + FRAME_GUESS.1);
    let start = to_screen(place(&overlay.screen, pet, guess, side));
    let window = WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("chat.html".into()))
        .title(title)
        .inner_size(WIDTH, HEIGHT)
        // Không hẹp hơn cảnh ở đầu khung chat (ChatScene.tsx rộng 360), để pet đứng đúng trên đồi.
        .min_inner_size(WIDTH, 360.0)
        .position(start.x, start.y)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .background_color(Color(13, 22, 40, 255))
        .visible(false)
        .build()?;
    crate::app_icon::set_small_icon(&window);
    // Cỡ thật cả viền và thanh tiêu đề chỉ đo được sau khi tạo: đặt lại cho trọn trong vùng làm việc.
    window.set_position(to_screen(place(&overlay.screen, pet, outer_size(&window)?, side)))?;
    let handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            handle.state::<Chat>().set_target(None);
            // Pet đang chat lại đi lại như thường.
            if let Err(e) = handle.emit_to(overlay::LABEL, events::CHAT_CLOSED, ()) {
                eprintln!("Không báo được đã đóng chat: {e}");
            }
        }
    });
    window.show()?;
    window.set_focus()
}

/// Cỡ cửa sổ cả viền và thanh tiêu đề (CSS pixel).
fn outer_size(window: &WebviewWindow) -> tauri::Result<(f64, f64)> {
    let size = window.outer_size()?.to_logical::<f64>(window.scale_factor()?);
    Ok((size.width, size.height))
}

/// Mở link trong câu trả lời bằng trình duyệt mặc định. Chỉ nhận http/https.
pub fn open_link(url: &str) -> AppResult<()> {
    let url = url.trim();
    let ok = (url.starts_with("https://") || url.starts_with("http://"))
        && url.len() <= 2048
        && !url.chars().any(|c| c.is_control() || c == '"');
    if !ok {
        return Err(AppError::bad_request("Link không hợp lệ."));
    }
    shell_open(url)
}

#[cfg(windows)]
fn shell_open(url: &str) -> AppResult<()> {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (verb, target) = (wide("open"), wide(url));
    // SAFETY: các chuỗi kết thúc bằng 0 và sống tới hết lời gọi; không cần cửa sổ cha.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    // Theo tài liệu ShellExecute: giá trị > 32 là thành công.
    if result as isize > 32 {
        Ok(())
    } else {
        Err(AppError::internal("Không mở được trình duyệt."))
    }
}

#[cfg(not(windows))]
fn shell_open(_url: &str) -> AppResult<()> {
    Err(AppError::internal("Chỉ hỗ trợ Windows."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn moi_luc_mot_cau_hai_cau_cach_nhau_it_nhat_3_giay() {
        let start = Instant::now();
        let mut l = Limiter::default();
        assert_eq!(l.begin(start), Ok(()));
        assert_eq!(l.begin(start + secs(1)), Err(Wait::Busy));
        l.finish(start + secs(1), Outcome::Ok);
        assert_eq!(l.begin(start + secs(2)), Err(Wait::TooFast(secs(1))));
        assert_eq!(l.begin(start + secs(3)), Ok(()));
    }

    #[test]
    fn toi_da_30_cau_moi_gio() {
        let start = Instant::now();
        let mut l = Limiter::default();
        for i in 0..PER_HOUR as u64 {
            assert_eq!(l.begin(start + secs(i * 10)), Ok(()));
            l.finish(start + secs(i * 10 + 1), Outcome::Ok);
        }
        let now = start + secs(PER_HOUR as u64 * 10);
        assert_eq!(l.begin(now), Err(Wait::Hourly(HOUR - secs(PER_HOUR as u64 * 10))));
        // Câu đầu tiên đủ một giờ thì gửi tiếp được.
        assert_eq!(l.begin(start + HOUR), Ok(()));
    }

    #[test]
    fn google_loi_thi_cho_lau_dan_thanh_cong_thi_het_cho() {
        let start = Instant::now();
        let mut l = Limiter::default();
        l.begin(start).unwrap();
        l.finish(start, Outcome::Failed);
        assert_eq!(l.begin(start + secs(10)), Err(Wait::Backoff(secs(20))));
        l.begin(start + secs(30)).unwrap();
        l.finish(start + secs(30), Outcome::Failed);
        assert_eq!(l.begin(start + secs(31)), Err(Wait::Backoff(secs(59))));
        l.begin(start + secs(90)).unwrap();
        l.finish(start + secs(90), Outcome::Ok);
        assert_eq!(l.begin(start + secs(93)), Ok(()));
    }

    #[test]
    fn mat_mang_khong_tinh_la_google_chan() {
        let start = Instant::now();
        let mut l = Limiter::default();
        l.begin(start).unwrap();
        l.finish(start, Outcome::Offline);
        assert_eq!(l.begin(start + secs(3)), Ok(()));
    }

    #[test]
    fn cho_toi_da_30_phut() {
        let start = Instant::now();
        let mut l = Limiter::default();
        let mut now = start;
        for _ in 0..10 {
            l.finish(now, Outcome::Failed);
            now += BACKOFF_MAX;
        }
        assert_eq!(l.backoff, Some(BACKOFF_MAX));
    }

    #[test]
    fn cau_bao_cho_ghi_thoi_gian_de_doc() {
        assert_eq!(duration_text(Duration::from_millis(1200)), "2 giây");
        assert_eq!(duration_text(secs(90)), "2 phút");
        assert_eq!(wait_error(Wait::TooFast(secs(2))).code, "BUSY");
        assert_eq!(wait_error(Wait::Backoff(secs(30))).code, "UNAVAILABLE");
    }

    #[test]
    fn cua_so_chat_canh_pet_khong_de_len_pet_khong_lan_xuong_taskbar() {
        use crate::overlay::Rect;
        let screen = ScreenInfo {
            scale_factor: 1.0,
            bounds: Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1080.0 },
            work_area: Rect { x: 0.0, y: 0.0, width: 1920.0, height: 1040.0 },
            neighbors: Vec::new(),
        };
        let size = (376.0, 539.0);
        // Pet đứng trên taskbar: cửa sổ bên trái pet, mép dưới cả viền đúng mép trên taskbar.
        let pet = PetBox { x: 1480.0, y: 980.0, width: 60.0, height: 60.0 };
        assert_eq!(side_of(&screen, pet, size.0), -1);
        assert_eq!(place(&screen, pet, size, -1), (1480.0 - GAP - 376.0, 1040.0 - 539.0));
        // Sát mép trái màn hình: sang bên phải, cách mép phải của pet.
        let pet = PetBox { x: 100.0, y: 20.0, width: 60.0, height: 60.0 };
        assert_eq!(side_of(&screen, pet, size.0), 1);
        assert_eq!(place(&screen, pet, size, 1), (160.0 + GAP, 0.0));
    }

    #[test]
    fn tach_cau_goi_y_bo_so_thu_tu_va_dong_thua() {
        let text = "Đây là vài câu gợi ý:\n1. Tỉ giá USD hôm nay?\n- **Giá vàng SJC?**\n\n2) 1 inch bằng bao nhiêu cm?\n\"Tỉ giá USD hôm nay?\"\nOK\n1.5 lít là bao nhiêu ml?";
        assert_eq!(
            parse_suggestions(text),
            vec!["Tỉ giá USD hôm nay?", "Giá vàng SJC?", "1 inch bằng bao nhiêu cm?", "1.5 lít là bao nhiêu ml?"]
        );
        assert!(suggest_prompt("Thứ Năm 01/10/2026").starts_with("Hôm nay là Thứ Năm 01/10/2026."));
    }

    #[test]
    fn chi_mo_link_http() {
        assert!(open_link("file:///C:/Windows").is_err());
        assert!(open_link("javascript:alert(1)").is_err());
        assert!(open_link("https://exa\nmple.com").is_err());
    }
}
