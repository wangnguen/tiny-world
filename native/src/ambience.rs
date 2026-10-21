//! Những thứ quanh nhóm pet theo giờ ở thành phố đã chọn (`ambience.ts`): thời tiết (thật hoặc giả lập), ban đêm, câu
//! của dịp lễ, con ma lúc 2 giờ sáng, câu giờ/ngày/thời tiết khi bấm đúp, nhắc nghỉ, báo bản mới, mục Xem thử.

use crate::ghost::{self, Ghost};
use crate::gfx::Gfx;
use crate::member::Member;
use crate::random::{between, random};
use crate::said::Said;
use crate::sim::calendar::{active_occasions, wall_clock};
use crate::sim::lines::{self, PlaceWeather};
use crate::sim::lunar::to_lunar;
use crate::sim::pet::PetState;
use crate::sim::weather::{day_sky, simulated_sky, sky_of, warmth_of, Warmth, REPORT_MAX_AGE};
use crate::sim::world::World;
use crate::sim::{Point, Sky};
use crate::tag::TemperatureTag;
use anyhow::Result;
use jiff::civil::DateTime;
use jiff::Timestamp;
use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};
use tinyworld_backend::activity::Reminder;
use tinyworld_backend::preview::{PreviewState, WeatherPreview, PREVIEW_SECONDS};
use tinyworld_backend::settings::{City, Settings};
use tinyworld_backend::weather::{Failure, Report};

/// Chưa có thời tiết thật thì ban đêm theo giờ ở thành phố đã chọn: từ 19 giờ tới trước 6 giờ sáng.
const NIGHT_FROM: i8 = 19;
const NIGHT_UNTIL: i8 = 6;
/// Hai tiếng sấm cách nhau ngẫu nhiên trong khoảng này (ms).
const THUNDER_GAP: (f64, f64) = (15_000.0, 45_000.0);
/// Xem thử giông: tiếng sấm đầu tiên sau chừng này ms, các tiếng sau cách nhau chừng này (ms).
const PREVIEW_FIRST_THUNDER_MS: f64 = 1_500.0;
const PREVIEW_THUNDER_GAP: (f64, f64) = (5_000.0, 9_000.0);
/// Con ma chỉ bay trong khoảng 2:00–2:30 (giờ ở thành phố đã chọn), mỗi đêm một lần.
const GHOST_HOUR: i8 = 2;
const GHOST_MINUTES: i8 = 30;
/// Con ma sà xuống ngang tầm chừng này lần chiều cao pet tính từ chân (bụng con ma ở đó).
const GHOST_HEIGHT: f64 = 0.6;
/// Vừa có phím hay chuột trong chừng này ms là người dùng đang ngồi máy (con ma lúc 2 giờ sáng).
const PRESENT_MS: u64 = 60_000;
/// Bấm đúp: câu giờ, ngày, thời tiết hiện chừng này giây.
const INFO_SECONDS: f64 = 6.0;
/// Câu báo (lỗi thời tiết, câu của dịp lễ) hiện chừng này giây.
const NOTICE_SECONDS: f64 = 5.0;
/// Nhắc nghỉ, nhắc khuya hiện chừng này giây, bấm vào pet thì tắt sớm.
const REMINDER_SECONDS: f64 = 30.0;
/// Tính lại thời tiết, ban đêm, dịp lễ, con ma mỗi chừng này.
const REFRESH: Duration = Duration::from_secs(60);

/// Đang chạy, leo, nhảy thì thở dồn (1), đang đi thì vừa (0.5), còn lại thở đều (0).
fn effort(state: PetState) -> f64 {
    match state {
        PetState::Run | PetState::Climb | PetState::Jump => 1.0,
        PetState::Walk => 0.5,
        _ => 0.0,
    }
}

/// Câu bắt buộc phải nói (không qua giới hạn câu nói cho vui); `key`: đánh dấu đã nói hôm nay.
struct Notice {
    text: String,
    day: Option<(String, String)>,
    seconds: f64,
    /// Lời nhắc: con nói nhảy lên một cái cho dễ thấy, cả nhóm đang ngủ thì đánh thức một con dậy nói.
    urgent: bool,
    /// Bản mới vừa báo (lưu lại lúc đã nói thật, không phải lúc xếp hàng).
    update: Option<String>,
}

impl Notice {
    fn new(text: String) -> Self {
        Self { text, day: None, seconds: NOTICE_SECONDS, urgent: false, update: None }
    }
}

/// Mấy việc cần ở overlay (pet đang hiện, chạy lại vòng lặp vẽ).
pub struct Ctx<'a> {
    pub gfx: &'a Gfx,
    pub world: &'a mut World,
    pub members: &'a mut [Member],
    /// Overlay đang hiện và không tạm dừng.
    pub live: bool,
    pub px: f64,
}

pub struct Ambience {
    settings: Settings,
    report: Option<Report>,
    failure: Option<Failure>,
    /// `None`: chưa tính lần nào, lần đầu không nói câu thời tiết.
    sky: Option<Option<Sky>>,
    /// Nhiệt độ ở thành phố đã chọn (°C, `None`: chưa biết) và nóng hay lạnh.
    temperature: Option<f64>,
    warmth: Option<Warmth>,
    /// Nóng lạnh đang vẽ quanh pet (tắt hiệu ứng thời tiết thì không vẽ).
    warmth_shown: Option<Warmth>,
    pub tag: TemperatureTag,
    /// Câu thời tiết đang chờ tới lượt nói cho vui (`World::chat`).
    sky_line: Option<String>,
    notices: VecDeque<Notice>,
    /// Khoá đã xếp hàng, để mỗi phút tính lại không xếp thêm lần nữa.
    queued: HashSet<String>,
    said: Said,
    next_thunder: f64,
    pub ghost: Ghost,
    /// Các con (`uid`) đã bị con ma đang bay làm giật mình.
    scared: HashSet<u32>,
    /// Thời tiết, nhiệt độ đang xem thử (mục Xem thử trong Cài đặt), thay trời thật tới lúc `until` (ms từ 1970).
    preview: Option<(WeatherPreview, f64)>,
    preview_ends: Option<Instant>,
    /// Lần báo Cài đặt gần nhất con ma có đang bay không.
    ghost_reported: bool,
    /// Vừa chọn thành phố mới: lấy được thời tiết ở đó thì một con báo luôn, để biết là đã chạy.
    announce: bool,
    /// Im lặng (tray): thông báo chờ tới lúc hết Im lặng, lời nhắc thì bỏ.
    quiet: bool,
    next_refresh: Instant,
    /// Có gì đổi, vòng lặp vẽ phải chạy lại (`take_wake`).
    wake: bool,
    /// Trạng thái xem thử cần báo Cài đặt (`take_report`).
    report_out: Option<PreviewState>,
}

impl Ambience {
    pub fn new(settings: Settings, said: Said, tag: TemperatureTag, ghost: Ghost) -> Self {
        Self {
            settings,
            report: None,
            failure: None,
            sky: None,
            temperature: None,
            warmth: None,
            warmth_shown: None,
            tag,
            sky_line: None,
            notices: VecDeque::new(),
            queued: HashSet::new(),
            said,
            next_thunder: 0.0,
            ghost,
            scared: HashSet::new(),
            preview: None,
            preview_ends: None,
            ghost_reported: false,
            announce: false,
            quiet: false,
            next_refresh: Instant::now(),
            wake: false,
            // Overlay vừa mở: không xem thử gì, Cài đặt bỏ sáng nút cũ.
            report_out: Some(PreviewState::default()),
        }
    }

    /// Có gì đổi từ lần trước, vòng lặp vẽ phải chạy lại.
    pub fn take_wake(&mut self) -> bool {
        std::mem::take(&mut self.wake)
    }

    /// Trạng thái xem thử vừa đổi, cần báo Cài đặt.
    pub fn take_report(&mut self) -> Option<PreviewState> {
        self.report_out.take()
    }

    /// Con ma đang bay: vòng lặp phải chạy tiếp kể cả khi cả nhóm đã ngủ.
    pub fn flying(&self) -> bool {
        self.ghost.flying()
    }

    /// Vòng lặp phải vẽ lại trong vòng chừng này ms (hiệu ứng thời tiết, aura, con ma); vô cùng nếu không cần.
    pub fn frame_ms(&self, members: &[Member], now: f64) -> f64 {
        if self.ghost.flying() {
            return 0.0;
        }
        let mut ms = f64::INFINITY;
        for member in members {
            ms = ms.min(member.effect.frame_ms());
            if let Some(kit) = &member.kit {
                ms = ms.min(kit.frame_ms(now));
            }
        }
        ms
    }

    /// Lúc nào phải tính lại (mỗi phút, hết xem thử).
    pub fn next_due(&self) -> Instant {
        self.preview_ends.map_or(self.next_refresh, |end| end.min(self.next_refresh))
    }

    /// Tới giờ tính lại thì tính.
    pub fn tick(&mut self, ctx: &mut Ctx) {
        let now = Instant::now();
        if self.preview_ends.is_some_and(|end| now >= end) {
            self.preview_ends = None;
            self.preview = None;
            self.refresh(ctx, true);
            self.report_preview();
        }
        if now >= self.next_refresh {
            self.refresh(ctx, false);
        }
    }

    pub fn set_settings(&mut self, ctx: &mut Ctx, next: Settings) {
        // Đổi thành phố: lỗi của thành phố cũ không còn đúng, thời tiết mới tới sau qua `set_report`.
        if !same_city(next.city.as_ref(), self.settings.city.as_ref()) {
            self.failure = None;
            self.announce = next.city.is_some();
        }
        self.settings = next;
        ctx.world.chatter = self.settings.chatter;
        self.refresh(ctx, false);
    }

    /// Thời tiết mới của thành phố đang chọn (`None`: chưa có).
    pub fn set_report(&mut self, ctx: &mut Ctx, report: Option<Report>) {
        self.report = report;
        if self.report.is_some() {
            self.failure = None;
        }
        self.refresh(ctx, false);
        if self.announce && self.fresh_report().is_some() {
            self.announce = false;
            let wall = self.wall();
            if let Some(weather) = self.place_weather(&wall) {
                self.notices.push_back(Notice::new(lines::weather_line(ctx.world.lang, &weather)));
            }
            self.wake = true;
        }
    }

    /// Lấy thời tiết bị lỗi (chỉ báo ở lần lỗi đầu của mỗi đợt): một con nói là không có mạng hay không có thời tiết.
    pub fn fail(&mut self, ctx: &mut Ctx, failure: Failure) {
        self.failure = Some(failure);
        self.announce = false;
        if self.settings.city.is_none() {
            return;
        }
        let lang = ctx.world.lang;
        let text = if failure.offline { lines::no_network(lang) } else { lines::no_weather(lang) };
        self.notices.push_back(Notice::new(text));
        self.refresh(ctx, false);
        self.wake = true;
    }

    /// Con mới hiện ra (đổi nhân vật): theo thời tiết như cả nhóm.
    pub fn adopt(&self, member: &mut Member) {
        member.effect.set_sky(self.sky.flatten());
        member.effect.set_warmth(self.warmth_shown);
    }

    fn wall(&self) -> DateTime {
        wall_clock(Timestamp::now(), self.settings.city.as_ref().and_then(|c| c.timezone.as_deref()))
    }

    /// Bấm đúp vào pet `i`: nói giờ, thứ, ngày dương lịch, âm lịch và thời tiết ở thành phố đã chọn.
    pub fn tell_time(&self, world: &mut World, i: usize) {
        let wall = self.wall();
        let text = lines::now_text(world.lang, wall, to_lunar(wall.date()), self.place_weather(&wall).as_ref());
        world.pets[i].say(text, INFO_SECONDS);
    }

    /// Mỗi lần vẽ: đặt hiệu ứng thời tiết theo thân thật của từng pet (khói thở ra đúng miệng con đó), nhãn nhiệt độ
    /// cạnh con đầu tiên đang hiện.
    pub fn place(&mut self, ctx: &mut Ctx, now: f64) -> Result<()> {
        let mut tagged = false;
        let bounds = ctx.world.bounds();
        for member in ctx.members.iter_mut() {
            let Some(i) = ctx.world.index_of(member.uid) else {
                continue;
            };
            let pet = &ctx.world.pets[i];
            let visible = pet.state() != PetState::Sleep && !member.view.sprite_hidden();
            let foot = member.view.foot.unwrap_or(Point { x: pet.x, y: pet.y });
            let size = (member.view.width, member.view.height);
            let mut aura_on = false;
            if let Some(kit) = &mut member.kit {
                kit.aura.update(ctx.gfx, foot, size, now, visible, ctx.px)?;
                aura_on = kit.aura.animating(now);
            }
            let Some(body) = member.view.body() else {
                continue;
            };
            let mouth = member.view.mouth_at();
            // Aura của Long thay thế thời tiết quanh chính Long; pet khác vẫn theo thời tiết chung.
            member.effect.update(ctx.gfx, body, now, visible && !aura_on, mouth, effort(pet.state()), ctx.px)?;
            if !tagged && !member.view.sprite_hidden() {
                self.tag.place(ctx.gfx, Some(body), pet.facing, &bounds, ctx.px)?;
                tagged = true;
            }
        }
        if !tagged {
            self.tag.place(ctx.gfx, None, 1, &bounds, ctx.px)?;
        }
        Ok(())
    }

    /// Mỗi lượt của vòng lặp: sấm, con ma làm giật mình, câu báo đang chờ.
    pub fn act(&mut self, ctx: &mut Ctx, now: f64) -> Result<()> {
        let animating = ctx.members.iter().any(|m| m.effect.animating());
        if self.sky == Some(Some(Sky::Storm)) && animating {
            let previewing = self.preview.is_some_and(|(p, _)| p.sky == Some(Sky::Storm));
            if self.next_thunder == 0.0 {
                self.next_thunder =
                    now + if previewing { PREVIEW_FIRST_THUNDER_MS } else { between(THUNDER_GAP) };
            } else if now >= self.next_thunder {
                self.next_thunder = now + between(if previewing { PREVIEW_THUNDER_GAP } else { THUNDER_GAP });
                for member in ctx.members.iter_mut() {
                    member.effect.flash(now);
                    if let Some(i) = ctx.world.index_of(member.uid) {
                        ctx.world.pet(i).startle();
                    }
                }
            }
        } else {
            self.next_thunder = 0.0;
        }

        let targets: Vec<Point> = (ctx.members.iter())
            .filter_map(|m| {
                let pet = &ctx.world.pets[ctx.world.index_of(m.uid)?];
                (pet.vanished == 0.0).then(|| Point {
                    x: pet.x,
                    y: pet.y - m.view.height * GHOST_HEIGHT - ghost::HEIGHT / 2.0,
                })
            })
            .collect();
        let ghost = self.ghost.update(ctx.gfx, now, &targets, ctx.px)?;
        if ghost.is_some() != self.ghost_reported {
            self.ghost_reported = ghost.is_some();
            self.report_preview();
        }
        match ghost {
            None => self.scared.clear(),
            Some(ghost) => {
                for member in ctx.members.iter() {
                    let Some(i) = ctx.world.index_of(member.uid) else { continue };
                    let pet = &ctx.world.pets[i];
                    if self.scared.contains(&member.uid) || pet.vanished > 0.0 {
                        continue;
                    }
                    // Con ma bay ngang qua ngay trên pet (từ ngang thân tới hơn đầu một chút) thì mới giật mình.
                    if (pet.x - ghost.x).abs() > member.view.width / 2.0 {
                        continue;
                    }
                    if ghost.y > pet.y || ghost.y < pet.y - member.view.height * 1.5 - ghost::HEIGHT {
                        continue;
                    }
                    // Đang ngủ thì giật mình tỉnh dậy, đang thức thì nhảy dựng lên.
                    if pet.state() == PetState::Sleep {
                        ctx.world.pet(i).wake();
                    } else {
                        ctx.world.pet(i).startle();
                    }
                    if self.scared.is_empty() {
                        let line = lines::ghost_line(ctx.world.lang);
                        ctx.world.pets[i].say(line, crate::sim::tuning::SPEECH_TIME);
                    }
                    self.scared.insert(member.uid);
                }
            }
        }

        if let Some(line) = self.sky_line.clone() {
            if let Some(i) = pick_awake(ctx) {
                if ctx.world.chat(i, &line) {
                    self.sky_line = None;
                }
            }
        }

        let speaking = ctx.world.pets.iter().any(|p| p.speech.is_some());
        if !self.notices.is_empty() && !self.quiet && !speaking {
            let urgent = self.notices[0].urgent;
            let speaker = pick_awake(ctx).or_else(|| {
                urgent.then(|| ctx.members.first().and_then(|m| ctx.world.index_of(m.uid))).flatten()
            });
            if let Some(i) = speaker {
                let notice = self.notices.pop_front().expect("có câu chờ");
                if notice.urgent {
                    // Đang ngủ thì giật mình dậy, đang rảnh thì nhảy lên.
                    if ctx.world.pets[i].state() == PetState::Sleep {
                        ctx.world.pet(i).wake();
                    } else {
                        ctx.world.pet(i).startle();
                    }
                }
                ctx.world.pets[i].say(notice.text, notice.seconds);
                if let Some((day, key)) = &notice.day {
                    self.said.add(day, key);
                }
                if let Some(version) = &notice.update {
                    self.said.set_update_said(version);
                }
            }
        }
        Ok(())
    }

    /// Mục Xem thử trong Cài đặt: thời tiết, nhiệt độ `next` quanh pet `PREVIEW_SECONDS` giây (bỏ qua công tắc và trời
    /// thật; một con nói ngay câu của thứ vừa bật; giông thì sấm ngay), cả hai `None` thì về trời thật. Mỗi lần bấm
    /// tính lại từ đầu `PREVIEW_SECONDS` giây.
    pub fn preview_weather(&mut self, ctx: &mut Ctx, next: WeatherPreview) {
        let before = self.preview.map(|(p, _)| p);
        let active = !next.is_stop();
        self.preview = active.then(|| (next, epoch_ms() + f64::from(PREVIEW_SECONDS) * 1000.0));
        self.preview_ends = active.then(|| Instant::now() + Duration::from_secs(PREVIEW_SECONDS.into()));
        self.refresh(ctx, true);
        if next.sky == Some(Sky::Storm) && before.and_then(|p| p.sky) != Some(Sky::Storm) {
            self.next_thunder = 0.0;
        }
        // Câu của thứ vừa bật nói luôn (thay câu đang nói), không chờ tới lượt câu nói cho vui.
        let lang = ctx.world.lang;
        let line = match (next.sky, next.temperature) {
            (Some(sky), _) if Some(sky) != before.and_then(|p| p.sky) => {
                Some(lines::sky_line(lang, sky, self.temperature, self.warmth))
            }
            (_, Some(t)) if Some(t) != before.and_then(|p| p.temperature) => {
                self.warmth.map(|w| lines::warmth_line(lang, w, t))
            }
            _ => None,
        };
        if let (Some(line), Some(i)) = (line, pick_awake(ctx)) {
            ctx.world.pets[i].say(line, NOTICE_SECONDS);
        }
        self.report_preview();
        self.wake = true;
    }

    /// Mục Xem thử: con ma bay qua ngay (không tính vào lượt mỗi đêm).
    pub fn preview_ghost(&mut self, ctx: &mut Ctx, now: f64) -> Result<()> {
        if !self.ghost.flying() && !ctx.members.is_empty() {
            self.ghost.fly(ctx.world.bounds(), now)?;
        }
        self.wake = true;
        Ok(())
    }

    /// Báo Cài đặt đang xem thử gì.
    fn report_preview(&mut self) {
        let (preview, until) = self.preview.map_or((None, None), |(p, until)| (Some(p), Some(until)));
        self.report_out = Some(PreviewState {
            sky: preview.and_then(|p| p.sky),
            temperature: preview.and_then(|p| p.temperature),
            until,
            ghost: self.ghost.flying(),
        });
    }

    /// Im lặng (tray): pet không nói câu cho vui (`World::quiet`), thông báo (thời tiết, dịp, bản mới) chờ tới lúc hết
    /// Im lặng, lời nhắc tới trong lúc đó thì bỏ. Câu người dùng tự hỏi (bấm đúp) vẫn nói.
    pub fn set_quiet(&mut self, world: &mut World, quiet: bool) {
        self.quiet = quiet;
        world.quiet = quiet;
        if !quiet {
            self.wake = true;
        }
    }

    /// Nhắc nghỉ, uống nước, nhắc khuya, spam Ctrl+S (activity.rs). Không qua giới hạn câu nói cho vui.
    pub fn remind(&mut self, world: &World, reminder: Reminder) {
        if self.quiet {
            return;
        }
        let lang = world.lang;
        let notice = match reminder {
            Reminder::SaveSpam => Notice { urgent: true, ..Notice::new(lines::save_spam_line(lang)) },
            other => {
                let now = jiff::Zoned::now();
                let time = format!("{:02}:{:02}", now.hour(), now.minute());
                let text = match other {
                    Reminder::Break { minutes } => lines::break_line(lang, minutes as u32),
                    Reminder::Water => lines::water_line(lang),
                    _ => lines::bedtime_line(lang, &time),
                };
                Notice { seconds: REMINDER_SECONDS, urgent: true, ..Notice::new(text) }
            }
        };
        self.notices.push_back(notice);
        self.wake = true;
    }

    /// GitHub có bản mới (update.rs, hỏi lại mỗi vài giờ): con thức đầu tiên báo một câu, mỗi bản một lần kể cả khi
    /// mở lại app. Không đánh thức ai, không nhảy lên: không gấp.
    pub fn announce_update(&mut self, world: &World, version: &str) {
        let key = format!("update:{version}");
        if self.queued.contains(&key) || self.said.update_said() == Some(version) {
            return;
        }
        self.queued.insert(key);
        let text = lines::update_line(world.lang, version);
        self.notices.push_back(Notice { seconds: REMINDER_SECONDS, update: Some(version.to_owned()), ..Notice::new(text) });
        self.wake = true;
    }

    /// Tính lại thời tiết, ban đêm, câu của dịp lễ và con ma theo giờ hiện tại; có gì đổi thì chạy lại vòng lặp vẽ.
    /// `quiet`: trời đổi vì bật, tắt xem thử, không nói câu thời tiết cho vui.
    fn refresh(&mut self, ctx: &mut Ctx, quiet: bool) {
        self.next_refresh = Instant::now() + REFRESH;
        let mut changed = false;
        let wall = self.wall();
        let report = self.fresh_report();
        let hour = wall.hour();
        ctx.world.night = report.as_ref().map_or(!(NIGHT_UNTIL..NIGHT_FROM).contains(&hour), |r| !r.is_day);

        // Đang xem thử thì thay trời thật, kể cả khi tắt hiệu ứng thời tiết.
        let preview = self.preview.map(|(p, _)| p);
        let sky = preview.and_then(|p| p.sky).or_else(|| self.sky_at(&wall, report.as_ref(), ctx.world.night));
        let temperature = preview.and_then(|p| p.temperature).or(report.as_ref().map(|r| r.temperature));
        let warmth = warmth_of(temperature);
        let shown = if self.settings.weather || preview.is_some_and(|p| p.temperature.is_some()) { warmth } else { None };
        // Câu nói: trời đổi từ kiểu này sang kiểu khác thì nói câu của trời mới (kèm nhiệt độ); trời không đổi mà chuyển
        // nóng, lạnh thì kêu nóng, lạnh. Lúc mới mở app, mới có thời tiết, hay đang xem thử thì không nói.
        let known = matches!(self.sky, Some(Some(_))) && !quiet && preview.is_none();
        let lang = ctx.world.lang;
        let before = self.sky.flatten();
        if let Some(next) = sky.filter(|_| known && sky != before) {
            self.sky_line = Some(lines::sky_line(lang, next, temperature, warmth));
        } else if let (true, Some(shown), Some(t)) = (known && sky == before && warmth != self.warmth, shown, temperature) {
            self.sky_line = Some(lines::warmth_line(lang, shown, t));
        } else if sky.is_none() {
            self.sky_line = None;
        }
        if Some(sky) != self.sky {
            self.sky = Some(sky);
            for member in ctx.members.iter_mut() {
                member.effect.set_sky(sky);
            }
            changed = true;
        }
        if shown != self.warmth_shown {
            self.warmth_shown = shown;
            for member in ctx.members.iter_mut() {
                member.effect.set_warmth(shown);
            }
            changed = true;
        }
        self.temperature = temperature;
        self.warmth = warmth;
        // Hình trên nhãn nhiệt độ theo trời thật, kể cả khi tắt hiệu ứng thời tiết.
        let icon = preview
            .and_then(|p| p.sky)
            .or_else(|| report.as_ref().map(|r| day_sky(sky_of(r.code), wall.month(), r.is_day)));
        let tagged =
            preview.and_then(|p| p.temperature).or(if self.settings.temperature_tag { temperature } else { None });
        if self.tag.set(icon, tagged, warmth) {
            changed = true;
        }

        let day = day_key(&wall);
        let occasions =
            if self.settings.events { active_occasions(&self.settings.occasions, wall.date()) } else { Vec::new() };
        for occasion in occasions {
            let key = format!("occasion:{}", occasion.name);
            // Dịp có sẵn nói theo ngôn ngữ đang dùng; `name` lưu trong file không đổi theo ngôn ngữ nên khoá vẫn đúng.
            let (_, message) = lines::occasion_text(occasion, lang);
            let queued = format!("{day}/{key}");
            if message.is_empty() || self.said.has(&day, &key) || self.queued.contains(&queued) {
                continue;
            }
            self.queued.insert(queued);
            self.notices.push_back(Notice { day: Some((day.clone(), key)), ..Notice::new(message) });
            changed = true;
        }

        let ghost_time = hour == GHOST_HOUR && wall.minute() < GHOST_MINUTES;
        if self.settings.ghost && ghost_time && !self.ghost.flying() && !self.said.has(&day, "ghost") {
            self.summon_ghost(ctx, &day);
        }
        if changed {
            self.wake = true;
        }
    }

    /// Con ma bay qua nếu người dùng đang ngồi máy để còn thấy, kể cả khi cả nhóm đã ngủ: 3 phút không đụng tới là pet
    /// ngủ, nên chờ có con thức thì gần như không bao giờ thấy ma. Mỗi đêm một lần.
    fn summon_ghost(&mut self, ctx: &mut Ctx, day: &str) {
        if !ctx.live || ctx.members.is_empty() {
            return;
        }
        let present = tinyworld_backend::activity::idle_ms().is_some_and(|idle| idle < PRESENT_MS);
        if !present {
            return;
        }
        self.said.add(day, "ghost");
        let now = crate::clock::now_ms();
        if let Err(e) = self.ghost.fly(ctx.world.bounds(), now) {
            log!("Không cho con ma bay được: {e}");
        }
        self.wake = true;
    }

    /// Thời tiết quanh pet: thành phố đã chọn thì theo thời tiết thật (chưa có thì không có gì), chưa chọn thì giả lập.
    fn sky_at(&self, wall: &DateTime, report: Option<&Report>, night: bool) -> Option<Sky> {
        if !self.settings.weather {
            return None;
        }
        if self.settings.city.is_some() {
            return report.map(|r| day_sky(sky_of(r.code), wall.month(), r.is_day));
        }
        Some(day_sky(simulated_sky(*wall), wall.month(), !night))
    }

    /// Thời tiết thật của đúng thành phố đang chọn, chưa quá cũ; `None` nếu không có.
    fn fresh_report(&self) -> Option<Report> {
        let report = self.report.as_ref()?;
        let city = self.settings.city.as_ref()?;
        if (report.latitude - city.latitude).abs() > 1e-6 || (report.longitude - city.longitude).abs() > 1e-6 {
            return None;
        }
        let now = (epoch_ms() / 1000.0) as u64;
        (now.saturating_sub(report.fetched_at) < REPORT_MAX_AGE).then(|| report.clone())
    }

    fn place_weather(&self, wall: &DateTime) -> Option<PlaceWeather> {
        let city = self.settings.city.as_ref()?;
        let short = city.name.split(',').next().unwrap_or_default().trim();
        let place = if short.is_empty() { city.name.clone() } else { short.to_owned() };
        let lang = tinyworld_backend::i18n::current();
        let Some(report) = self.fresh_report() else {
            let problem = self.failure.map(|f| if f.offline { lines::no_network(lang) } else { lines::no_weather(lang) });
            return Some(PlaceWeather { place, sky: None, temperature: None, problem });
        };
        Some(PlaceWeather {
            place,
            sky: Some(day_sky(sky_of(report.code), wall.month(), report.is_day)),
            temperature: Some(report.temperature),
            problem: None,
        })
    }
}

fn same_city(a: Option<&City>, b: Option<&City>) -> bool {
    a.map(|c| (c.latitude, c.longitude)) == b.map(|c| (c.latitude, c.longitude))
}

/// Một con đang thức (và đang hiện) bất kỳ, `None` nếu cả nhóm ngủ.
fn pick_awake(ctx: &Ctx) -> Option<usize> {
    let awake: Vec<usize> = (ctx.members.iter())
        .filter_map(|m| ctx.world.index_of(m.uid))
        .filter(|&i| ctx.world.pets[i].state() != PetState::Sleep && ctx.world.pets[i].vanished == 0.0)
        .collect();
    (!awake.is_empty()).then(|| awake[(random() * awake.len() as f64).floor() as usize % awake.len()])
}

/// Ngày `wall` dạng "2026-10-01".
fn day_key(wall: &DateTime) -> String {
    format!("{:04}-{:02}-{:02}", wall.year(), wall.month(), wall.day())
}

/// Số ms từ 1970 (UTC), như `Date.now()`.
fn epoch_ms() -> f64 {
    Timestamp::now().as_millisecond() as f64
}
