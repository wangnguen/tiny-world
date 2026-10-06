//! Sức khoẻ & công việc (Phase 5): giờ ngồi máy, nhắc nghỉ, nhắc khuya, spam Ctrl+S, nhận biết đang gõ.
//!
//! Mỗi giây đọc thời điểm input cuối (`GetLastInputInfo`: chỉ biết lúc nào có phím hay chuột, không biết
//! phím gì) và vị trí con trỏ. Ngồi máy là khoảng giữa hai lần input liên tiếp cách nhau dưới 5 phút; xa
//! hơn là vắng, tính lại từ đầu lượt ngồi liền. Có input mà chuột đứng yên là đang gõ phím: overlay không
//! cho pet nói câu cho vui. Giờ ngồi máy, lượt ngồi liền lâu nhất và số lần nghỉ lưu theo ngày trong
//! `stats.json`.

use crate::error::AppResult;
use crate::events;
use crate::overlay;
use crate::settings::Settings;
use crate::storage::write_atomic;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

const TICK: Duration = Duration::from_secs(1);
/// Hai lần input cách nhau từ chừng này trở lên thì khoảng giữa là vắng, không tính ngồi máy.
const AWAY_MS: u64 = 5 * 60 * 1000;
/// Lần gõ phím gần nhất trong chừng này thì vẫn tính là đang gõ.
const TYPING_HOLD_MS: u64 = 5 * 1000;
/// Nhắc khuya chỉ khi vừa có input trong chừng này (đang ngồi máy thật, không phải để máy chạy).
const ACTIVE_NOW_MS: u64 = 60 * 1000;
const BEDTIME_REPEAT_MS: u64 = 30 * 60 * 1000;
/// Sau giờ đi ngủ tới chừng này (phút trong ngày, 5:00) vẫn là khuya.
const NIGHT_END: u16 = 5 * 60;
const SAVE_EVERY_MS: u64 = 60 * 1000;
/// Giữ số liệu của chừng này ngày gần nhất.
const KEEP_DAYS: usize = 30;
/// Bấm Ctrl+S chừng này lần trong chừng này ms là spam.
const SAVE_SPAM_COUNT: usize = 5;
const SAVE_SPAM_WINDOW_MS: u64 = 10 * 1000;

/// Phần cài đặt mà vòng đọc cần.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Config {
    pub screen_time: bool,
    pub break_reminder: bool,
    pub break_minutes: u16,
    pub water_reminder: bool,
    pub water_minutes: u16,
    pub bedtime_reminder: bool,
    pub bedtime: u16,
}

impl From<&Settings> for Config {
    fn from(s: &Settings) -> Self {
        Self {
            screen_time: s.screen_time,
            break_reminder: s.break_reminder,
            break_minutes: s.break_minutes,
            water_reminder: s.water_reminder,
            water_minutes: s.water_minutes,
            bedtime_reminder: s.bedtime_reminder,
            bedtime: s.bedtime,
        }
    }
}

/// Event `reminder`, khớp `Reminder` trong packages/core: pet nhắc một câu.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Reminder {
    /// Ngồi liền `minutes` phút.
    Break { minutes: u64 },
    Water,
    Bedtime,
    SaveSpam,
}

/// Số liệu một ngày (giờ máy), ms.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Day {
    pub active_ms: u64,
    /// Lượt ngồi liền lâu nhất trong ngày.
    pub longest_ms: u64,
    /// Số lần đứng dậy nghỉ (vắng từ `AWAY_MS`) sau một lượt ngồi.
    pub breaks: u32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct StatsFile {
    /// Theo ngày "2026-10-01", tăng dần.
    days: BTreeMap<String, Day>,
}

/// Một lần đọc của vòng lặp.
pub struct Sample<'a> {
    /// Đồng hồ của vòng đọc (ms, chỉ tăng).
    pub now: u64,
    /// Từ input cuối tới giờ (ms).
    pub idle: u64,
    /// Có input mới kể từ lần đọc trước.
    pub input: bool,
    /// Con trỏ có di chuyển kể từ lần đọc trước.
    pub cursor_moved: bool,
    /// Ngày theo giờ máy, "2026-10-01".
    pub date: &'a str,
    /// Phút trong ngày theo giờ máy.
    pub minute: u16,
}

/// Overlay cần biết sau một lần đọc.
#[derive(Debug, Default, PartialEq)]
pub struct Effects {
    /// Vừa bắt đầu hoặc thôi gõ phím.
    pub busy: Option<bool>,
    pub reminder: Option<Reminder>,
}

/// Logic đếm giờ và nhắc, không gọi Win32 nên test được.
#[derive(Default)]
pub struct Tracker {
    config: Config,
    days: BTreeMap<String, Day>,
    dirty: bool,
    last_input: Option<u64>,
    /// Ngồi liền (ms) từ lần vắng gần nhất, để nhắc nghỉ (đếm cả khi tắt đếm giờ).
    session: u64,
    /// Phần của `session` đã cộng vào giờ ngồi máy, để hiện trong tab Sức khoẻ: không lớn hơn số đã đếm.
    counted: u64,
    /// `session` lúc nhắc nghỉ gần nhất.
    reminded_at: u64,
    /// Ngồi máy (ms) từ lần nhắc uống nước gần nhất; vắng không tính lại.
    since_water: u64,
    last_bedtime: Option<u64>,
    last_typing: Option<u64>,
    busy: bool,
}

impl Tracker {
    pub fn configure(&mut self, config: Config) {
        self.config = config;
    }

    /// Ngồi liền để nhắc nghỉ (tab Sức khoẻ hiện `counted_session_ms`).
    #[cfg(test)]
    pub fn session_ms(&self) -> u64 {
        self.session
    }

    /// Ngồi liền đã đếm vào giờ ngồi máy (bật đếm giờ giữa chừng thì chỉ tính từ lúc bật).
    pub fn counted_session_ms(&self) -> u64 {
        self.counted
    }

    pub fn days(&self) -> &BTreeMap<String, Day> {
        &self.days
    }

    pub fn tick(&mut self, s: &Sample) -> Effects {
        let mut fx = Effects::default();
        if s.input {
            let at = s.now.saturating_sub(s.idle);
            if let Some(last) = self.last_input {
                let gap = at.saturating_sub(last);
                if gap < AWAY_MS {
                    self.credit(gap, s);
                } else {
                    self.rest(s.date);
                }
            }
            self.last_input = Some(at);
            if !s.cursor_moved {
                self.last_typing = Some(s.now);
            }
        } else if s.idle >= AWAY_MS {
            self.rest(s.date);
        }

        let busy = self
            .last_typing
            .is_some_and(|t| s.now.saturating_sub(t) < TYPING_HOLD_MS);
        if busy != self.busy {
            self.busy = busy;
            fx.busy = Some(busy);
        }

        let every = u64::from(self.config.break_minutes) * 60 * 1000;
        if self.config.break_reminder && every > 0 && self.session >= self.reminded_at + every {
            self.reminded_at = self.session;
            fx.reminder = Some(Reminder::Break {
                minutes: self.session / 60_000,
            });
        }
        let water = u64::from(self.config.water_minutes) * 60 * 1000;
        if fx.reminder.is_none() && self.config.water_reminder && water > 0 && self.since_water >= water {
            self.since_water = 0;
            fx.reminder = Some(Reminder::Water);
        }
        if fx.reminder.is_none()
            && self.config.bedtime_reminder
            && s.idle < ACTIVE_NOW_MS
            && is_late(s.minute, self.config.bedtime)
            && self
                .last_bedtime
                .is_none_or(|t| s.now.saturating_sub(t) >= BEDTIME_REPEAT_MS)
        {
            self.last_bedtime = Some(s.now);
            fx.reminder = Some(Reminder::Bedtime);
        }
        fx
    }

    /// Vắng đủ lâu (ngày `date`): hết lượt ngồi liền; vừa ngồi xong một lượt đã đếm thì tính là một lần nghỉ.
    fn rest(&mut self, date: &str) {
        if self.counted > 0 && self.config.screen_time {
            self.days.entry(date.to_string()).or_default().breaks += 1;
            self.dirty = true;
        }
        self.session = 0;
        self.counted = 0;
        self.reminded_at = 0;
    }

    fn credit(&mut self, ms: u64, s: &Sample) {
        self.session += ms;
        self.since_water += ms;
        if !self.config.screen_time {
            return;
        }
        self.counted += ms;
        let day = self.days.entry(s.date.to_string()).or_default();
        day.active_ms += ms;
        // Lượt ngồi qua nửa đêm: hôm nay chỉ tính phần của hôm nay.
        day.longest_ms = day.longest_ms.max(self.counted.min(day.active_ms));
        while self.days.len() > KEEP_DAYS {
            self.days.pop_first();
        }
        self.dirty = true;
    }

    pub fn clear(&mut self) {
        self.days.clear();
        self.dirty = true;
    }
}

/// Phút `minute` có sau giờ đi ngủ `bedtime` (và trước 5:00 sáng) không.
fn is_late(minute: u16, bedtime: u16) -> bool {
    if bedtime >= NIGHT_END {
        minute >= bedtime || minute < NIGHT_END
    } else {
        minute >= bedtime && minute < NIGHT_END
    }
}

/// Nhận biết bấm Ctrl+S dồn dập.
#[derive(Default)]
pub struct SaveSpam {
    presses: VecDeque<u64>,
}

impl SaveSpam {
    /// Vừa bấm Ctrl+S lúc `now` (ms): đủ `SAVE_SPAM_COUNT` lần trong `SAVE_SPAM_WINDOW_MS` thì `true` (rồi
    /// đếm lại từ đầu).
    pub fn press(&mut self, now: u64) -> bool {
        self.presses.push_back(now);
        while self
            .presses
            .front()
            .is_some_and(|&t| now.saturating_sub(t) > SAVE_SPAM_WINDOW_MS)
        {
            self.presses.pop_front();
        }
        if self.presses.len() >= SAVE_SPAM_COUNT {
            self.presses.clear();
            return true;
        }
        false
    }
}

/// State của app: logic đếm, file lưu, công tắc Ctrl+S.
pub struct Activity {
    path: PathBuf,
    tracker: Mutex<Tracker>,
    save_spam: AtomicBool,
    spam: Mutex<SaveSpam>,
}

/// Command `get_stats`, khớp `ScreenStats` trong packages/core.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsView {
    /// Hôm nay theo giờ máy.
    pub today: String,
    pub session_ms: u64,
    pub days: BTreeMap<String, Day>,
}

impl Activity {
    /// File hỏng thì bắt đầu lại (lần lưu sau ghi đè).
    pub fn load(data_dir: &Path, settings: &Settings) -> Self {
        let path = data_dir.join("stats.json");
        let file = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<StatsFile>(&bytes).ok())
            .unwrap_or_default();
        let tracker = Tracker {
            days: file.days,
            ..Tracker::default()
        };
        let activity = Self {
            path,
            tracker: Mutex::new(tracker),
            save_spam: AtomicBool::new(false),
            spam: Mutex::new(SaveSpam::default()),
        };
        activity.configure(settings);
        activity
    }

    pub fn configure(&self, settings: &Settings) {
        self.lock().configure(Config::from(settings));
        self.save_spam.store(settings.save_spam, Ordering::Relaxed);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Tracker> {
        self.tracker.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn save_spam_enabled(&self) -> bool {
        self.save_spam.load(Ordering::Relaxed)
    }

    /// Vừa bấm Ctrl+S (cursor.rs): `true` nếu là spam.
    pub fn save_pressed(&self, now: u64) -> bool {
        self.spam.lock().unwrap_or_else(|e| e.into_inner()).press(now)
    }

    pub fn stats(&self) -> StatsView {
        let tracker = self.lock();
        StatsView {
            today: local_time().0,
            session_ms: tracker.counted_session_ms(),
            days: tracker.days().clone(),
        }
    }

    /// Xoá hết số liệu đã lưu.
    pub fn clear(&self) -> AppResult<()> {
        let mut tracker = self.lock();
        tracker.clear();
        Self::write(&self.path, &tracker)?;
        tracker.dirty = false;
        Ok(())
    }

    /// Ghi `stats.json` nếu có thay đổi.
    pub fn flush(&self) {
        let mut tracker = self.lock();
        if !tracker.dirty {
            return;
        }
        match Self::write(&self.path, &tracker) {
            Ok(()) => tracker.dirty = false,
            Err(e) => eprintln!("Không lưu được stats.json: {e}"),
        }
    }

    fn write(path: &Path, tracker: &Tracker) -> AppResult<()> {
        let file = StatsFile {
            days: tracker.days.clone(),
        };
        write_atomic(path, &serde_json::to_vec_pretty(&file)?)
    }
}

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let start = Instant::now();
        let mut last_tick = None;
        let mut last_cursor = None;
        let mut last_save = 0;
        loop {
            thread::sleep(TICK);
            let Some((idle, tick)) = last_input() else {
                continue;
            };
            let now = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
            let cursor = cursor_position();
            let cursor_moved = cursor != last_cursor;
            last_cursor = cursor;
            let input = last_tick.is_some_and(|t| t != tick);
            last_tick = Some(tick);
            let activity = app.state::<Activity>();
            let (date, minute) = local_time();
            let fx = activity.lock().tick(&Sample {
                now,
                idle,
                input,
                cursor_moved,
                date: &date,
                minute,
            });
            if let Some(busy) = fx.busy {
                if let Err(e) = app.emit_to(overlay::LABEL, events::ACTIVITY_CHANGED, busy) {
                    eprintln!("Không báo được đang gõ phím: {e}");
                }
            }
            if let Some(reminder) = fx.reminder {
                if let Err(e) = app.emit_to(overlay::LABEL, events::REMINDER, reminder) {
                    eprintln!("Không gửi được lời nhắc: {e}");
                }
            }
            if now.saturating_sub(last_save) >= SAVE_EVERY_MS {
                last_save = now;
                activity.flush();
            }
        }
    });
}

/// (ms từ input cuối, mốc của input cuối). Mốc đổi là có input mới.
#[cfg(windows)]
fn last_input() -> Option<(u64, u32)> {
    use windows_sys::Win32::System::SystemInformation::GetTickCount;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut info = LASTINPUTINFO {
        cbSize: u32::try_from(std::mem::size_of::<LASTINPUTINFO>()).ok()?,
        dwTime: 0,
    };
    // SAFETY: GetLastInputInfo chỉ ghi vào `info` là biến cục bộ có cbSize đúng.
    if unsafe { GetLastInputInfo(&mut info) } == 0 {
        return None;
    }
    // SAFETY: GetTickCount không có tham số. Đồng hồ 32 bit quay vòng sau 49 ngày nên trừ kiểu quay vòng.
    let now = unsafe { GetTickCount() };
    Some((u64::from(now.wrapping_sub(info.dwTime)), info.dwTime))
}

#[cfg(not(windows))]
fn last_input() -> Option<(u64, u32)> {
    None
}

/// Số ms từ lần có phím hay chuột cuối cùng (ở bất kỳ app nào); `None` nếu không đọc được.
pub fn idle_ms() -> Option<u64> {
    last_input().map(|(idle, _)| idle)
}

#[cfg(windows)]
fn cursor_position() -> Option<(i32, i32)> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: GetCursorPos chỉ ghi vào `point` là biến cục bộ hợp lệ.
    (unsafe { GetCursorPos(&mut point) } != 0).then_some((point.x, point.y))
}

#[cfg(not(windows))]
fn cursor_position() -> Option<(i32, i32)> {
    None
}

/// (ngày "2026-10-01", phút trong ngày) theo giờ máy.
#[cfg(windows)]
fn local_time() -> (String, u16) {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;

    // SAFETY: SYSTEMTIME chỉ gồm số nguyên nên toàn 0 là hợp lệ; GetLocalTime chỉ ghi vào biến cục bộ này.
    let mut t: SYSTEMTIME = unsafe { std::mem::zeroed() };
    // SAFETY: như trên.
    unsafe { GetLocalTime(&mut t) };
    (
        format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay),
        t.wHour * 60 + t.wMinute,
    )
}

#[cfg(not(windows))]
fn local_time() -> (String, u16) {
    ("1970-01-01".to_string(), 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60 * 1000;

    fn config() -> Config {
        Config {
            screen_time: true,
            break_reminder: true,
            break_minutes: 50,
            water_reminder: false,
            water_minutes: 60,
            bedtime_reminder: true,
            bedtime: 23 * 60,
        }
    }

    /// Người dùng bấm phím hoặc chuột lúc `now` (ms), lúc `minute` trong ngày.
    fn input(now: u64, minute: u16, moved: bool) -> Sample<'static> {
        Sample {
            now,
            idle: 0,
            input: true,
            cursor_moved: moved,
            date: "2026-10-01",
            minute,
        }
    }

    #[test]
    fn ngoi_may_la_khoang_giua_hai_lan_input_gan_nhau_vang_thi_khong_tinh() {
        let mut t = Tracker::default();
        t.configure(config());
        t.tick(&input(0, 600, true));
        t.tick(&input(4 * MIN, 604, true));
        // Vắng 10 phút: không tính, lượt ngồi liền về 0, tính là một lần nghỉ.
        t.tick(&input(14 * MIN, 614, true));
        assert_eq!(t.session_ms(), 0);
        t.tick(&input(15 * MIN, 615, true));
        let day = &t.days()["2026-10-01"];
        assert_eq!(day.active_ms, 5 * MIN);
        assert_eq!(day.longest_ms, 4 * MIN);
        assert_eq!(day.breaks, 1);
        assert_eq!(t.session_ms(), MIN);
    }

    #[test]
    fn vang_du_lau_thi_het_luot_ngoi_lien_ca_khi_chua_quay_lai() {
        let mut t = Tracker::default();
        t.configure(config());
        t.tick(&input(0, 600, true));
        t.tick(&input(3 * MIN, 603, true));
        assert_eq!(t.session_ms(), 3 * MIN);
        t.tick(&Sample {
            idle: 5 * MIN,
            input: false,
            ..input(8 * MIN, 608, false)
        });
        assert_eq!(t.session_ms(), 0);
    }

    #[test]
    fn bat_dem_gio_giua_chung_thi_ngoi_lien_hien_ra_chi_tinh_tu_luc_bat() {
        let mut t = Tracker::default();
        t.configure(Config {
            screen_time: false,
            ..config()
        });
        t.tick(&input(0, 600, true));
        t.tick(&input(3 * MIN, 603, true));
        t.configure(config());
        t.tick(&input(5 * MIN, 605, true));
        assert_eq!(t.days()["2026-10-01"].active_ms, 2 * MIN);
        assert_eq!(t.counted_session_ms(), 2 * MIN);
        // Nhắc nghỉ vẫn tính cả lúc chưa bật.
        assert_eq!(t.session_ms(), 5 * MIN);
    }

    #[test]
    fn nhac_nghi_moi_50_phut_ngoi_lien_vang_thi_tinh_lai() {
        let mut t = Tracker::default();
        t.configure(config());
        let mut reminders = Vec::new();
        for m in 0..=101 {
            if let Some(r) = t.tick(&input(m * MIN, 600, true)).reminder {
                reminders.push((m, r));
            }
        }
        assert_eq!(
            reminders,
            vec![(50, Reminder::Break { minutes: 50 }), (100, Reminder::Break { minutes: 100 })]
        );
        // Vắng rồi quay lại: lại đủ 50 phút mới nhắc.
        t.tick(&input(120 * MIN, 600, true));
        assert_eq!(t.tick(&input(121 * MIN, 600, true)).reminder, None);
    }

    #[test]
    fn nhac_uong_nuoc_moi_60_phut_ngoi_may_vang_khong_tinh_lai() {
        let mut t = Tracker::default();
        t.configure(Config {
            break_reminder: false,
            water_reminder: true,
            ..config()
        });
        let mut reminders = Vec::new();
        for m in 0..=40 {
            if let Some(r) = t.tick(&input(m * MIN, 600, true)).reminder {
                reminders.push((m, r));
            }
        }
        // Vắng 10 phút rồi quay lại: phần đã ngồi vẫn tính, đủ 60 phút ngồi máy thì nhắc.
        for m in 50..=80 {
            if let Some(r) = t.tick(&input(m * MIN, 600, true)).reminder {
                reminders.push((m, r));
            }
        }
        assert_eq!(reminders, vec![(70, Reminder::Water)]);
        // Nhắc nghỉ cùng lúc thì nhắc nghỉ trước, uống nước giây sau.
        let mut both = Tracker::default();
        both.configure(Config {
            water_reminder: true,
            water_minutes: 50,
            ..config()
        });
        let mut at_50 = Vec::new();
        for m in 0..=50 {
            at_50.extend(both.tick(&input(m * MIN, 600, true)).reminder);
        }
        at_50.extend(both.tick(&input(50 * MIN + 1000, 600, true)).reminder);
        assert_eq!(at_50, vec![Reminder::Break { minutes: 50 }, Reminder::Water]);
    }

    #[test]
    fn tat_dem_gio_thi_khong_luu_gi_nhung_van_nhac_nghi() {
        let mut t = Tracker::default();
        t.configure(Config {
            screen_time: false,
            ..config()
        });
        let mut reminded = false;
        for m in 0..=50 {
            reminded |= t.tick(&input(m * MIN, 600, true)).reminder.is_some();
        }
        assert!(reminded);
        assert!(t.days().is_empty());
        assert!(!t.dirty);
    }

    #[test]
    fn nhac_khuya_sau_gio_di_ngu_toi_da_30_phut_mot_lan() {
        let mut t = Tracker::default();
        t.configure(Config {
            break_reminder: false,
            ..config()
        });
        assert_eq!(t.tick(&input(0, 22 * 60 + 59, true)).reminder, None);
        assert_eq!(t.tick(&input(MIN, 23 * 60, true)).reminder, Some(Reminder::Bedtime));
        assert_eq!(t.tick(&input(10 * MIN, 23 * 60 + 9, true)).reminder, None);
        assert_eq!(t.tick(&input(31 * MIN, 23 * 60 + 30, true)).reminder, Some(Reminder::Bedtime));
        // Qua nửa đêm vẫn là khuya, 5:00 thì thôi.
        assert!(is_late(2 * 60, 23 * 60));
        assert!(!is_late(5 * 60, 23 * 60));
        assert!(!is_late(12 * 60, 23 * 60));
        // Giờ đi ngủ sau nửa đêm.
        assert!(is_late(60, 30));
        assert!(!is_late(23 * 60, 30));
    }

    #[test]
    fn co_input_ma_chuot_dung_yen_la_dang_go_phim() {
        let mut t = Tracker::default();
        t.configure(config());
        assert_eq!(t.tick(&input(0, 600, true)).busy, None);
        assert_eq!(t.tick(&input(1000, 600, false)).busy, Some(true));
        assert_eq!(t.tick(&input(2000, 600, false)).busy, None);
        let quiet = Sample {
            idle: 5000,
            input: false,
            ..input(7000, 600, false)
        };
        assert_eq!(t.tick(&quiet).busy, Some(false));
    }

    #[test]
    fn chi_giu_30_ngay_gan_nhat() {
        let mut t = Tracker::default();
        t.configure(config());
        let dates: Vec<String> = (1..=35).map(|d| format!("2026-{:02}-{:02}", 8 + d / 31, d % 31 + 1)).collect();
        for (i, date) in dates.iter().enumerate() {
            let now = i as u64 * 2 * MIN;
            t.tick(&Sample { date, ..input(now, 600, true) });
            t.tick(&Sample { date, ..input(now + MIN, 600, true) });
        }
        assert_eq!(t.days().len(), KEEP_DAYS);
        assert!(!t.days().contains_key(&dates[0]));
    }

    #[test]
    fn ctrl_s_5_lan_trong_10_giay_la_spam() {
        let mut spam = SaveSpam::default();
        assert!(!(0..4).any(|i| spam.press(i * 1000)));
        assert!(spam.press(4000));
        // Đếm lại từ đầu sau khi kêu.
        assert!(!spam.press(4500));
        let mut slow = SaveSpam::default();
        assert!(!(0..10).any(|i| slow.press(i * 3000)));
    }

    #[test]
    fn luu_roi_doc_lai_va_xoa_duoc() {
        let dir = std::env::temp_dir().join(format!("tinyworld-activity-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut settings = Settings::default();
        settings.screen_time = true;
        let activity = Activity::load(&dir, &settings);
        {
            let mut t = activity.lock();
            t.tick(&input(0, 600, true));
            t.tick(&input(2 * MIN, 602, true));
        }
        activity.flush();
        let again = Activity::load(&dir, &settings);
        assert_eq!(again.lock().days()["2026-10-01"].active_ms, 2 * MIN);
        again.clear().unwrap();
        assert!(Activity::load(&dir, &settings).lock().days().is_empty());
        // File hỏng thì bắt đầu lại.
        fs::write(dir.join("stats.json"), b"{not json").unwrap();
        assert!(Activity::load(&dir, &settings).lock().days().is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
