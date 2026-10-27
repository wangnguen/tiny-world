//! Tiến trình overlay (`main.ts` của bản web cộng phần overlay của shell Tauri): một cửa sổ DirectComposition phủ
//! vùng làm việc của một màn hình, các luồng nền (thời tiết, giờ ngồi máy, cập nhật, danh sách cửa sổ, app
//! fullscreen), icon khay, và cổng nối với tiến trình giao diện (Cài đặt, khung chat).
//!
//! Vòng lặp chính chạy trên luồng tạo cửa sổ: xử lý message, đọc con trỏ khoảng 60 lần mỗi giây (overlay để chuột đi
//! xuyên nên không tự nhận sự kiện chuột), bước mô phỏng 30 lần mỗi giây và vẽ lúc có gì đổi. Cả nhóm đứng yên thì chỉ
//! thức dậy lúc có con đổi frame; cả nhóm ngủ thì không vẽ gì, chỉ còn đọc con trỏ.

use crate::ambience::{self, Ambience};
use crate::clock::now_ms;
use crate::ghost::Ghost;
use crate::gfx::Gfx;
use crate::interaction::{self, Interaction};
use crate::ipc::{self, Ipc, Writer};
use crate::long::{Action, LongCombo, Stage, LONG_PACK};
use crate::member::Member;
use crate::said::Said;
use crate::screens;
use crate::sim::pet::{Facing, PetOptions, PetState};
use crate::sim::snapshot::{parse_world_snapshot, PetSnapshot};
use crate::sim::step::{FixedStep, StepBlend};
use crate::sim::terrain::Bounds;
use crate::sim::tuning as t;
use crate::sim::world::World;
use crate::sim::{lines, Point, Rect};
use crate::sprites;
use crate::tag::TemperatureTag;
use crate::tray::{self, Tray};
use crate::view;
use crate::window::{self, Input};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tinyworld_backend::activity::{self, Activity};
use tinyworld_backend::chat::{self, Chat, ChatTarget, PetBox};
use tinyworld_backend::events::{Event, Host};
use tinyworld_backend::geometry::{Geometry, Overlay, ScreenChange, ScreenInfo};
use tinyworld_backend::i18n;
use tinyworld_backend::preview::Preview;
use tinyworld_backend::settings::{Settings, SettingsStore};
use tinyworld_backend::storage::Storage;
use tinyworld_backend::update::Updater;
use tinyworld_backend::weather::Weather;
use tinyworld_backend::window_list::{self, WindowList, Windows};
use tinyworld_backend::{fullscreen, update, weather};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, POINT};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::System::Threading::{
    CreateMutexW, CreateWaitableTimerExW, SetWaitableTimer, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, INFINITE,
    TIMER_ALL_ACCESS,
};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, DispatchMessageW, GetCursorPos, MsgWaitForMultipleObjects, PeekMessageW, MSG, PM_REMOVE,
    QS_ALLINPUT, WM_QUIT,
};

/// Tần số mô phỏng. Có con chuyển động mà chọn 30 fps trong Cài đặt thì cũng là tần số vẽ; chọn cao hơn thì vẽ theo
/// nhịp màn hình (`App::sleep`).
const FPS: f64 = 30.0;
const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 30);
/// Cả nhóm đứng yên thì chỉ cần thức dậy lúc có con đổi frame, nhưng tối đa chừng này một lần để mô phỏng không dồn
/// quá `MAX_STEPS` bước một lượt.
const CALM_MAX: Duration = Duration::from_millis(250);
const MAX_STEPS: u32 = 10;
/// Đọc con trỏ chừng này một lần lúc overlay đang hiện, và lúc đang ẩn (chỉ để kịp biết lúc hiện lại).
const POLL: Duration = Duration::from_millis(16);
const HIDDEN_POLL: Duration = Duration::from_millis(250);
/// Đo lại màn hình chừng này một lần.
const WATCH: Duration = Duration::from_secs(1);
/// Chu kỳ lưu world.json (chỉ ghi khi có thay đổi).
const SAVE_INTERVAL: Duration = Duration::from_secs(30);
/// Hai lần cho overlay sang màn hình khác cách nhau ít nhất chừng này.
const MOVE_INTERVAL: Duration = Duration::from_millis(200);
/// `DwmFlush` trả về nhanh hơn chừng này là không có gì chờ ghép hình: hẹn giờ như thường để khỏi quay vòng.
const FLUSH_MIN: Duration = Duration::from_millis(2);
/// Màn hình làm tươi sớm hơn lần vẽ kế (`App::smooth_due`) chừng này vẫn vẽ: nhịp làm tươi xê dịch đôi chút.
const FLUSH_SLACK: Duration = Duration::from_millis(2);
/// Đã bật chat mà con trỏ dừng trên một con chừng này thì dựng sẵn khung chat; con trỏ rời pet chừng này mà không mở
/// thì thôi.
const CHAT_HOVER: Duration = Duration::from_millis(250);
const CHAT_COOL: Duration = Duration::from_secs(15);
/// Khoảng cách từ mép phải vùng làm việc tới con đầu tiên lúc xuất hiện lần đầu (CSS pixel).
const SPAWN_MARGIN: f64 = 48.0;
/// Các con xuất hiện lần đầu đứng cách nhau chừng này lần bề ngang pet.
const SPAWN_GAP: f64 = 1.4;
/// Id của pet tạm vẽ bằng code, khi bản build chưa có sprite pack nào.
const PLACEHOLDER_ID: &str = "placeholder";
/// Bản cũ chỉ có một pet, lưu trong world.json với id này: lấy làm chỗ của con đầu tiên.
const LEGACY_ID: &str = "pet-1";

/// Những gì tiến trình overlay cần biết lúc chạy.
pub struct Config {
    /// Version của app (Cài đặt, log, cập nhật).
    pub version: String,
    /// Thư mục dữ liệu (`%APPDATA%\com.tinyworld.app`).
    pub data_dir: PathBuf,
    /// Exe chạy được `--ui` (tiến trình giao diện); `None` thì không mở được Cài đặt, khung chat.
    pub ui_exe: Option<PathBuf>,
}

/// Thư mục dữ liệu mặc định. Bản dev dùng thư mục riêng để chạy song song bản cài mà không đụng dữ liệu của nó.
pub fn default_data_dir() -> Option<PathBuf> {
    let name = if cfg!(debug_assertions) { "com.tinyworld.app.dev" } else { "com.tinyworld.app" };
    Some(PathBuf::from(std::env::var_os("APPDATA")?).join(name))
}

/// Tin đưa vào vòng lặp chính từ các luồng khác.
pub enum Message {
    Backend(Event),
    Fullscreen(bool),
    Ui(ipc::Message),
    /// Frame cho cỡ mới đã làm xong ở luồng nền (`App::resize_members`), theo `uid` của từng con.
    Resized { generation: u64, views: Vec<(u32, view::Prepared)> },
    /// Pack của các nhân vật mới chọn đã nạp xong ở luồng nền (`App::sync_pets_later`).
    PetsLoaded { generation: u64, packs: Vec<Option<String>>, loaded: Vec<(Option<String>, view::Prepared)> },
    /// Tải xong bộ cài bản mới: thoát để chạy nó.
    Quit,
}

/// Luồng nền gửi event vào vòng lặp chính.
struct NativeHost(Sender<Message>);

impl Host for NativeHost {
    fn emit(&self, event: Event) {
        let _ = self.0.send(Message::Backend(event));
        window::wake();
    }
}

/// Con trỏ theo CSS pixel của overlay, kèm phím Ctrl và nút chuột (`CursorInfo` của bản web).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cursor {
    x: f64,
    y: f64,
    /// Đang giữ Ctrl: click xuyên qua pet xuống app bên dưới.
    pass_through: bool,
    /// Đang giữ một nút chuột, ở bất kỳ đâu trên màn hình.
    pressed: bool,
}

/// Chạy overlay tới khi thoát. Đã có một bản đang chạy thì nhờ nó hiện lại pet rồi thoát luôn.
pub fn run(config: Config) -> Result<()> {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let mutex_name: Vec<u16> =
        format!("Local\\{}\0", if cfg!(debug_assertions) { "TinyWorld.Dev" } else { "TinyWorld" }).encode_utf16().collect();
    let _mutex = unsafe { CreateMutexW(None, true, PCWSTR(mutex_name.as_ptr()))? };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        window::signal_running();
        return Ok(());
    }
    std::fs::create_dir_all(&config.data_dir).context("tạo thư mục dữ liệu")?;
    tinyworld_backend::log::init(&config.data_dir, &config.version);
    App::new(config)?.run()
}

pub struct App {
    hwnd: HWND,
    gfx: Gfx,
    world: World,
    members: Vec<Member>,
    ambience: Ambience,
    interaction: Interaction,
    /// Combo quật đuôi Long đang diễn (Shift + click vào Long), `None` nếu không có.
    combo: Option<LongCombo>,
    step: FixedStep,
    blend: StepBlend,
    settings: Arc<SettingsStore>,
    activity: Arc<Activity>,
    weather: Arc<Weather>,
    updater: Arc<Updater>,
    windows: Arc<Windows>,
    overlay: Arc<Overlay>,
    storage: Storage,
    chat: Arc<Chat>,
    preview: Preview,
    runtime: tokio::runtime::Handle,
    tx: Sender<Message>,
    rx: Receiver<Message>,
    tray: Tray,
    ipc: Ipc,
    /// Màn hình overlay đang phủ, và mép trên taskbar tự ẩn đang trồi lên (`WindowList::taskbar_top`).
    current: ScreenInfo,
    taskbar_top: Option<f64>,
    /// Cỡ nhân vật trong Settings.
    size: f64,
    /// Lần đổi cỡ (`resize_members`) và lần đổi nhân vật (`sync_pets_later`) gần nhất, để bỏ kết quả nạp xong muộn của
    /// lần đổi cũ hơn.
    resizing: u64,
    loading: u64,
    /// Nhịp vẽ lúc có con chuyển động (`Settings::fps`) và lúc tới lần vẽ kế khi vẽ theo nhịp màn hình (`sleep`).
    fps: u16,
    smooth_due: Instant,
    /// Chat với pet đang bật trong Cài đặt (Phase 6).
    chat_on: bool,
    /// Con trỏ nằm trên pet từ lúc nào, và đang dựng sẵn khung chat tới lúc nào (`prepare_chat`).
    hover_since: Option<Instant>,
    chat_warm_until: Option<Instant>,
    /// Con đang chat (`uid`): đứng yên cạnh khung chat tới khi đóng khung chat.
    listening: Option<u32>,
    paused: bool,
    hidden: bool,
    /// Vòng lặp vẽ đang chạy (cả nhóm ngủ thì dừng), lần vẽ sau lúc nào và có phải là chờ lâu lúc đứng yên không.
    running: bool,
    next_frame: Instant,
    calm_wait: bool,
    last_frame: Instant,
    /// Đã báo cả nhóm đang ngủ (`Overlay::set_resting`).
    resting: bool,
    cursor: Option<Cursor>,
    /// Lần đọc con trỏ trước (pixel vật lý, Ctrl, nút chuột) và có đang giữ Ctrl+S không.
    last_cursor: Option<(i32, i32, bool, bool)>,
    saving: bool,
    click_through: bool,
    held: bool,
    last_move: Option<Instant>,
    /// Trạng thái đã lưu, chỉ dùng cho các con hiện ra lúc mở app (`sync_pets` lần đầu).
    restoring: HashMap<String, PetSnapshot>,
    saved: String,
    next_save: Instant,
    next_poll: Instant,
    next_watch: Instant,
    /// Im lặng đang bật (chỉ số trong `tray::QUIET`) và lúc tự tắt.
    quiet: Option<usize>,
    quiet_until: Option<Instant>,
    depth: u64,
    depths: HashMap<u32, u64>,
    /// Overlay vừa đổi chỗ: đổi toạ độ các điểm chuột đã ghi (`Interaction::remap`).
    remaps: Vec<tinyworld_backend::geometry::Remap>,
    timer: HANDLE,
    quitting: bool,
    /// Lỗi lần trước của vòng lặp, để không ghi log lặp lại mỗi lượt.
    last_error: Option<String>,
}

impl App {
    fn new(config: Config) -> Result<Self> {
        let data_dir = config.data_dir.clone();
        let settings = Arc::new(SettingsStore::load(&data_dir));
        let current_settings = settings.get();
        i18n::set(current_settings.lang);
        let activity = Arc::new(Activity::load(&data_dir, &current_settings));
        let weather = Arc::new(Weather::load(&data_dir));
        let updater = Arc::new(Updater::new(config.version.clone()));
        let windows = Arc::new(Windows::default());
        let storage = Storage::new(data_dir.clone());

        let all = screens::displays();
        let primary = all.first().cloned().context("không tìm thấy màn hình")?;
        let geometry = Geometry::new(&primary, &all);
        let overlay = Arc::new(Overlay::new(geometry.clone()));
        let hwnd = window::create(geometry.window)?;
        let gfx = Gfx::new(hwnd)?;

        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        let handle = runtime.handle().clone();
        std::thread::spawn(move || runtime.block_on(std::future::pending::<()>()));

        let (tx, rx) = mpsc::channel();
        let host: Arc<dyn Host> = Arc::new(NativeHost(tx.clone()));
        let fullscreen_tx = tx.clone();
        fullscreen::spawn(move |busy| {
            let _ = fullscreen_tx.send(Message::Fullscreen(busy));
            window::wake();
        });
        window_list::spawn(overlay.clone(), windows.clone(), hwnd.0 as isize, host.clone());
        weather::spawn(&handle, weather.clone(), settings.clone(), host.clone());
        activity::spawn(activity.clone(), host.clone());
        update::spawn(&handle, updater.clone(), host);

        let (ui_tx, ui_rx) = mpsc::channel::<ipc::Message>();
        let forward = tx.clone();
        std::thread::spawn(move || {
            for message in ui_rx {
                if forward.send(Message::Ui(message)).is_err() {
                    break;
                }
            }
        });
        let debug = data_dir.join("ipc.json");
        let ipc = Ipc::start(ui_tx, window::wake, config.ui_exe, Some(&debug))
            .context("mở cổng cho cửa sổ giao diện")?;
        let launcher = ipc.launcher();
        let tray = Tray::new(current_settings.lang, window::wake, move || launcher.warm_settings())?;

        let screen = geometry.screen.clone();
        let bounds = bounds_of(&screen, None);
        let seed = jiff::Timestamp::now().as_millisecond() as u64;
        let mut world = World::new(bounds, seed);
        world.set_screen(bounds, screen.neighbors.clone(), None);
        world.speed = current_settings.speed;
        world.lang = current_settings.lang;
        world.chatter = current_settings.chatter;
        let saved = match storage.load() {
            Ok(saved) => saved,
            Err(e) => {
                log!("Không đọc được trạng thái đã lưu, bắt đầu lại: {}", e.message);
                None
            }
        };
        let restoring = saved
            .as_ref()
            .and_then(parse_world_snapshot)
            .map(|snapshot| snapshot.pets.into_iter().map(|p| (p.id.clone(), p)).collect())
            .unwrap_or_default();
        let ghost = Ghost::new(gfx.layer(None)?)?;
        let tag = TemperatureTag::new(gfx.layer(None)?)?;
        let said = Said::load(data_dir.join("said.json"));
        let ambience = Ambience::new(current_settings.clone(), said, tag, ghost);
        let timer = unsafe {
            CreateWaitableTimerExW(None, PCWSTR::null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS.0)?
        };
        let now = Instant::now();
        let mut app = Self {
            hwnd,
            gfx,
            world,
            members: Vec::new(),
            ambience,
            interaction: Interaction::default(),
            combo: None,
            step: FixedStep::new(1.0 / FPS, MAX_STEPS),
            blend: StepBlend::default(),
            settings,
            activity,
            weather,
            updater,
            windows,
            overlay,
            storage,
            chat: Arc::new(Chat::default()),
            preview: Preview::default(),
            runtime: handle,
            tx,
            rx,
            tray,
            ipc,
            current: screen,
            taskbar_top: None,
            size: current_settings.size,
            resizing: 0,
            loading: 0,
            fps: current_settings.fps,
            smooth_due: now,
            chat_on: current_settings.chat,
            hover_since: None,
            chat_warm_until: None,
            listening: None,
            paused: false,
            hidden: false,
            running: false,
            next_frame: now,
            calm_wait: false,
            last_frame: now,
            resting: false,
            cursor: None,
            last_cursor: None,
            saving: false,
            click_through: true,
            held: false,
            last_move: None,
            restoring,
            saved: String::new(),
            next_save: now + SAVE_INTERVAL,
            next_poll: now,
            next_watch: now + WATCH,
            quiet: None,
            quiet_until: None,
            depth: 0,
            depths: HashMap::new(),
            remaps: Vec::new(),
            timer,
            quitting: false,
            last_error: None,
        };
        app.sync_pets(&sprites::resolve_packs(&current_settings.pets))?;
        app.restoring.clear();
        app.saved = serde_json::to_string(&app.world.snapshot())?;
        let windows = app.windows.current();
        app.apply_windows(&windows);
        let report = app.weather.current(current_settings.city.as_ref());
        app.with_ambience(|ambience, ctx| ambience.set_report(ctx, report));
        window::set_visible(hwnd, true);
        app.redraw()?;
        app.wake();
        Ok(app)
    }

    fn px(&self) -> f64 {
        self.current.scale_factor
    }

    fn with_ambience<R>(&mut self, f: impl FnOnce(&mut Ambience, &mut ambience::Ctx) -> R) -> R {
        let live = !self.hidden && !self.paused;
        let px = self.px();
        let mut ctx = ambience::Ctx { gfx: &self.gfx, world: &mut self.world, members: &mut self.members, live, px };
        f(&mut self.ambience, &mut ctx)
    }

    fn run(mut self) -> Result<()> {
        loop {
            if !pump() {
                break;
            }
            // Lỗi vẽ (driver GPU, thiếu bộ nhớ) không làm tắt app: ghi log một lần, lượt sau thử lại.
            if let Err(e) = self.tick() {
                let message = format!("{e:#}");
                if self.last_error.as_deref() != Some(message.as_str()) {
                    log!("Overlay lỗi: {message}");
                    self.last_error = Some(message);
                }
                if crate::gfx::is_device_lost(&e) {
                    if let Err(e) = self.recreate_graphics() {
                        log!("Không tạo lại được thiết bị GPU: {e:#}");
                    }
                }
                self.next_frame = Instant::now() + FRAME;
            }
            if self.quitting {
                unsafe {
                    let _ = DestroyWindow(self.hwnd);
                }
                continue;
            }
            self.sleep();
        }
        self.ipc.quit();
        self.updater.launch_pending();
        Ok(())
    }

    /// Một lượt của vòng lặp: tin từ luồng khác, chuột, khay, con trỏ, màn hình, vẽ, lưu.
    fn tick(&mut self) -> Result<()> {
        while let Ok(message) = self.rx.try_recv() {
            self.message(message)?;
        }
        for input in window::take_input() {
            self.input(input)?;
        }
        for action in self.tray.take_events() {
            self.tray_action(action)?;
        }
        for remap in std::mem::take(&mut self.remaps) {
            self.interaction.remap(&remap);
        }
        let now = Instant::now();
        if self.quiet_until.is_some_and(|until| now >= until) {
            self.set_quiet(None);
        }
        if now >= self.next_poll {
            self.poll_cursor()?;
            self.next_poll = now + if self.overlay.is_visible() { POLL } else { HIDDEN_POLL };
            self.prepare_chat(now);
        }
        if now >= self.next_watch {
            self.next_watch = now + WATCH;
            if let Err(e) = self.refit() {
                log!("Không đặt lại được overlay theo màn hình: {e}");
            }
        }
        if now >= self.ambience.next_due() {
            self.with_ambience(|ambience, ctx| ambience.tick(ctx));
        }
        self.flush_ambience();
        if now >= self.next_save {
            self.next_save = now + SAVE_INTERVAL;
            self.autosave();
        }
        if self.running && Instant::now() >= self.next_frame {
            self.frame()?;
        }
        Ok(())
    }

    /// Ngủ tới việc gần nhất phải làm; có message (chuột, tin từ luồng khác) thì thức sớm.
    fn sleep(&mut self) {
        // Có gì đang chuyển động mà chọn trên 30 fps: vẽ theo nhịp màn hình, ở vị trí nội suy giữa hai bước mô phỏng
        // (`StepBlend`); màn hình làm tươi nhanh hơn nhịp đã chọn thì bỏ bớt lần, trung bình vẫn đúng nhịp. Vẽ 30 fps
        // thì pet đi lại hơi giật, bị ném nhanh thì nhảy cóc từng đoạn dài.
        if self.running && (self.fps == 0 || f64::from(self.fps) > FPS) && self.fast() {
            let start = Instant::now();
            let flushed = unsafe { DwmFlush() }.is_ok();
            let now = Instant::now();
            if flushed && now - start >= FLUSH_MIN {
                if now + FLUSH_SLACK >= self.smooth_due {
                    let gap = if self.fps == 0 { Duration::ZERO } else { Duration::from_secs(1) / u32::from(self.fps) };
                    // Không hẹn xa hơn một nhịp kể từ lần vẽ này (màn hình nhanh hơn nhịp đã chọn một chút thì vẫn vẽ mọi
                    // lần), cũng không hẹn về trước (vừa đứng yên một lúc thì không vẽ bù).
                    self.smooth_due = (self.smooth_due + gap).clamp(now, now + gap);
                    self.next_frame = now;
                }
                return;
            }
        }
        let mut due = self.next_poll.min(self.next_watch).min(self.next_save).min(self.ambience.next_due());
        if self.running {
            due = due.min(self.next_frame);
        }
        if let Some(until) = self.quiet_until {
            due = due.min(until);
        }
        let wait = due.saturating_duration_since(Instant::now());
        if wait.is_zero() {
            return;
        }
        let ticks = -((wait.as_nanos() / 100).max(1) as i64);
        unsafe {
            if SetWaitableTimer(self.timer, &ticks, 0, None, None, false).is_ok() {
                MsgWaitForMultipleObjects(Some(&[self.timer]), false, INFINITE, QS_ALLINPUT);
            } else {
                std::thread::sleep(wait);
            }
        }
    }

    /// Có gì đang chuyển động: con nào không đứng yên (đi, chạy, leo, bay, bị kéo, lảo đảo...), Long đang diễn combo,
    /// hay con ma đang bay. Đứng yên, ngồi mép, ngủ thì chỉ đổi frame vài lần mỗi giây (`next_delay`).
    fn fast(&self) -> bool {
        self.combo.is_some()
            || self.ambience.flying()
            || (self.world.pets.iter())
                .any(|p| p.vanished == 0.0 && !matches!(p.state(), PetState::Idle | PetState::Perch | PetState::Sleep))
    }

    fn message(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Backend(event) => self.backend(event)?,
            Message::Fullscreen(busy) => {
                self.overlay.set_auto_hidden(busy);
                self.apply_visibility();
            }
            Message::Ui(message) => self.ui(message)?,
            Message::Resized { generation, views } => self.resized(generation, views)?,
            Message::PetsLoaded { generation, packs, loaded } => self.pets_loaded(generation, &packs, loaded)?,
            Message::Quit => self.request_quit(),
        }
        Ok(())
    }

    fn backend(&mut self, event: Event) -> Result<()> {
        if !event.overlay_only() {
            self.ipc.event(event.name(), event.payload(), None);
        }
        match event {
            Event::WindowsChanged(list) => self.apply_windows(&list),
            Event::WeatherChanged(report) => self.with_ambience(|ambience, ctx| ambience.set_report(ctx, report)),
            Event::WeatherFailed(failure) => self.with_ambience(|ambience, ctx| ambience.fail(ctx, failure)),
            Event::ActivityChanged(busy) => self.world.busy = busy,
            Event::Reminder(reminder) => {
                if !self.hidden && !self.paused {
                    self.ambience.remind(&self.world, reminder);
                }
            }
            Event::UpdateAvailable(info) => self.ambience.announce_update(&self.world, &info.version),
            Event::UpdateProgress(_) => {}
        }
        self.flush_ambience();
        Ok(())
    }

    /// Báo Cài đặt thứ đang xem thử, chạy lại vòng lặp nếu ambience có gì đổi.
    fn flush_ambience(&mut self) {
        if let Some(state) = self.ambience.take_report() {
            self.preview.set(state);
            self.ipc.event(tinyworld_backend::events::PREVIEW_CHANGED, json!(state), Some("settings"));
        }
        if self.ambience.take_wake() {
            self.wake();
        }
    }

    fn ui(&mut self, message: ipc::Message) -> Result<()> {
        match message {
            ipc::Message::Connected(writer) => self.ipc.connected(writer),
            ipc::Message::Disconnected => {
                self.ipc.disconnected();
                self.chat_closed();
            }
            ipc::Message::ChatClosed => self.chat_closed(),
            ipc::Message::Call { id, name, args, writer } => self.call(id, &name, args, writer)?,
        }
        Ok(())
    }

    /// Đóng khung chat (hay tắt chat trong Cài đặt) thì con đang chat lại đi lại.
    fn chat_closed(&mut self) {
        self.chat.set_target(None);
        self.stop_listening();
        self.wake();
    }

    fn stop_listening(&mut self) {
        if let Some(i) = self.listening.take().and_then(|uid| self.world.index_of(uid)) {
            self.world.pets[i].stop_listening();
        }
    }

    // Mô phỏng và vẽ.

    /// Chạy lại vòng lặp; đang chờ lâu lúc cả nhóm đứng yên mà có chuyện (click, cửa sổ đổi) thì vẽ ngay lần sau.
    fn wake(&mut self) {
        if self.hidden || self.paused {
            return;
        }
        if self.running {
            if self.calm_wait {
                self.next_frame = Instant::now();
                self.calm_wait = false;
            }
            return;
        }
        self.running = true;
        self.set_resting(false);
        self.last_frame = Instant::now();
        self.next_frame = self.last_frame;
    }

    fn set_resting(&mut self, resting: bool) {
        if resting == self.resting {
            return;
        }
        self.resting = resting;
        // Luồng theo dõi cửa sổ đổi cách theo dõi (pet ngủ thì chỉ cần taskbar); thức dậy thì đọc lại ngay.
        if self.overlay.set_resting(resting) {
            window_list::refresh();
        }
    }

    fn frame(&mut self) -> Result<()> {
        if self.hidden || self.paused {
            self.running = false;
            return Ok(());
        }
        let now = Instant::now();
        let elapsed = (now - self.last_frame).as_secs_f64();
        self.last_frame = now;
        for _ in 0..self.step.advance(elapsed) {
            let from = self.positions();
            self.world.step(self.step.dt);
            let to = self.positions();
            self.blend.record(&from, &to);
        }
        let ms = now_ms();
        self.with_ambience(|ambience, ctx| ambience.act(ctx, ms))?;
        // Có con đang đi hoặc bay ra khỏi mép giáp màn hình khác: overlay sang bên đó, cả nhóm theo sang.
        let bounds = self.world.bounds();
        if let Some(leaving) = self.world.pets.iter().find_map(|p| p.leaving(&bounds)) {
            self.move_overlay(leaving)?;
        }
        if let Some(mut combo) = self.combo.take() {
            combo.update(&mut self.stage(), ms);
            if !combo.done() {
                self.combo = Some(combo);
            }
        }
        self.redraw()?;
        self.flush_ambience();
        // Cả nhóm ngủ thì dừng hẳn vòng lặp (trừ lúc con ma còn đang bay); click hoặc kéo sẽ chạy lại.
        if self.world.resting() && !self.ambience.flying() {
            self.running = false;
            self.set_resting(true);
            return Ok(());
        }
        let delay = self.next_delay();
        self.calm_wait = delay > FRAME;
        self.next_frame = now + delay;
        Ok(())
    }

    fn positions(&self) -> Vec<(u32, Point)> {
        self.world.pets.iter().map(|p| (p.uid(), Point { x: p.x, y: p.y })).collect()
    }

    /// Cả nhóm đứng yên thì chờ tới lúc có con đổi frame; có con đi lại thì vẽ 30 lần mỗi giây (chọn nhịp cao hơn thì
    /// `sleep` vẽ sớm hơn, theo nhịp màn hình).
    fn next_delay(&self) -> Duration {
        // Long đang diễn combo (cinematic, vệt tốc biến): vẽ đủ nhịp.
        if self.combo.is_some() {
            return FRAME;
        }
        let ms = now_ms();
        // Đang mưa, tuyết quanh pet hay con ma đang bay thì vẽ đủ nhịp của hiệu ứng.
        let mut wait = self.ambience.frame_ms(&self.members, ms) / 1000.0;
        let pace = self.world.pace();
        for member in &self.members {
            let Some(i) = self.world.index_of(member.uid) else { continue };
            let pet = &self.world.pets[i];
            // Con đang biến mất đứng nguyên một chỗ, không cần vẽ; chờ lâu nhất `CALM_MAX` nên hiện lại kịp.
            if pet.vanished > 0.0 {
                continue;
            }
            if !matches!(pet.state(), PetState::Idle | PetState::Perch | PetState::Sleep) {
                return FRAME;
            }
            wait = wait.min(member.view.next_frame_in(pet, pace));
        }
        Duration::from_secs_f64(wait.clamp(FRAME.as_secs_f64(), CALM_MAX.as_secs_f64()))
    }

    /// Vẽ lại cả nhóm; con đổi frame, bị cửa sổ che, hoặc đi dưới con trỏ đứng yên thì cũng tính lại click-through.
    fn redraw(&mut self) -> Result<()> {
        let ms = now_ms();
        let bounds = self.world.bounds();
        let pace = self.world.pace();
        let alpha = self.step.alpha();
        // Vẽ trễ mô phỏng một bước, nội suy theo `alpha` (`StepBlend`).
        let lag = (1.0 - alpha) * self.step.dt;
        let px = self.px();
        for member in &mut self.members {
            let Some(i) = self.world.index_of(member.uid) else { continue };
            let pet = &self.world.pets[i];
            let occluders = self.world.occluders(i);
            let at = self.blend.at(member.uid, Point { x: pet.x, y: pet.y }, alpha);
            member.view.update(&self.gfx, pet, &bounds, &occluders, at, lag, pace)?;
            if let Some(kit) = &mut member.kit {
                let foot = member.view.foot.unwrap_or(at);
                kit.update(&self.gfx, foot, (member.view.width, member.view.height), ms, px)?;
            }
        }
        self.with_ambience(|ambience, ctx| ambience.place(ctx, ms))?;
        self.gfx.commit()?;
        self.refresh_click_through();
        Ok(())
    }

    // Chuột.

    /// Con nằm trên cùng có phần hình dưới `point`.
    fn pick(&self, point: Point) -> Option<u32> {
        self.members
            .iter()
            .filter(|m| !m.view.sprite_hidden() && m.view.hit_test(point))
            .max_by_key(|m| self.depths.get(&m.uid).copied().unwrap_or(0))
            .map(|m| m.uid)
    }

    /// Con trỏ nằm trên phần có hình của một con (và không giữ Ctrl), hoặc đang kéo: overlay nhận chuột. Trả về con trỏ có
    /// nằm trên pet không.
    fn refresh_click_through(&mut self) -> bool {
        let over = !self.paused
            && self.cursor.is_some_and(|c| !c.pass_through && self.pick(Point { x: c.x, y: c.y }).is_some());
        if !over {
            self.hover_since = None;
        }
        let enabled = !over && !self.held;
        if enabled != self.click_through {
            self.click_through = enabled;
            window::set_click_through(self.hwnd, enabled);
        }
        over
    }

    /// Đọc con trỏ, phím Ctrl, nút chuột (và phím S khi bật "Spam Ctrl+S").
    fn poll_cursor(&mut self) -> Result<()> {
        if !self.overlay.is_visible() {
            self.last_cursor = None;
            self.hover_since = None;
            return Ok(());
        }
        let mut point = POINT::default();
        if unsafe { GetCursorPos(&mut point) }.is_err() {
            return Ok(());
        }
        let down = |key: u16| unsafe { GetAsyncKeyState(i32::from(key)) } < 0;
        let pass_through = down(VK_CONTROL.0);
        let pressed = [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON].into_iter().any(|key| down(key.0));
        if self.activity.save_spam_enabled() {
            let saving = pass_through && down(u16::from(b'S'));
            if saving && !self.saving && self.activity.save_pressed(now_ms() as u64) {
                self.ambience.remind(&self.world, tinyworld_backend::activity::Reminder::SaveSpam);
                self.flush_ambience();
            }
            self.saving = saving;
        }
        let raw = (point.x, point.y, pass_through, pressed);
        if self.last_cursor == Some(raw) {
            return Ok(());
        }
        self.last_cursor = Some(raw);
        let (x, y) = self.overlay.to_local(f64::from(point.x), f64::from(point.y));
        let info = Cursor { x, y, pass_through, pressed };
        // Vừa bấm chuột ở bất kỳ đâu (kể cả ngoài pet): các con đang ngủ đều thức dậy.
        let was_pressed = self.cursor.is_some_and(|c| c.pressed);
        if pressed && !was_pressed && !self.paused && self.world.pets.iter().any(|p| p.state() == PetState::Sleep) {
            self.world.wake_all();
            self.wake();
        }
        self.cursor = Some(info);
        // Con đứng yên thì quay đầu nhìn theo con trỏ. Con đang ngủ không để ý con trỏ.
        if self.running {
            self.world.move_cursor(x, y);
        }
        // Chỉ tính lúc người dùng đưa chuột vào pet (`prepare_chat`), không tính lúc pet đi tới dưới con trỏ đứng yên.
        if self.refresh_click_through() && !self.held && self.hover_since.is_none() {
            self.hover_since = Some(Instant::now());
        }
        Ok(())
    }

    fn input(&mut self, input: Input) -> Result<()> {
        let px = self.px();
        let css = |x: i32, y: i32| Point { x: f64::from(x) / px, y: f64::from(y) / px };
        let mut interaction = std::mem::take(&mut self.interaction);
        match input {
            Input::Down { x, y, shift, time } => {
                if !self.paused && !self.hidden {
                    interaction.down(self, css(x, y), shift, f64::from(time));
                }
            }
            Input::Move { x, y, time } => interaction.moved(self, css(x, y), f64::from(time)),
            Input::Up { time, .. } => interaction.up(self, f64::from(time)),
            Input::Cancel => interaction.cancel(self),
            Input::Context { x, y } => self.context_menu(css(x, y))?,
            Input::Display => self.next_watch = Instant::now(),
            Input::Show => {
                self.overlay.set_user_hidden(false);
                self.apply_visibility();
            }
        }
        for remap in self.remaps.drain(..) {
            interaction.remap(&remap);
        }
        self.interaction = interaction;
        Ok(())
    }

    /// Click chuột phải vào pet: mở cửa sổ chat cạnh con đó; chưa bật chat thì con đó nhắc bật trong Cài đặt.
    fn context_menu(&mut self, point: Point) -> Result<()> {
        if self.paused {
            return Ok(());
        }
        let Some(uid) = self.pick(point) else {
            return Ok(());
        };
        let Some(member) = self.members.iter().find(|m| m.uid == uid) else {
            return Ok(());
        };
        let Some(i) = self.world.index_of(uid) else {
            return Ok(());
        };
        if !self.chat_on {
            let line = lines::chat_off_line(self.world.lang);
            self.world.pets[i].say(line, t::SPEECH_TIME);
            self.wake();
            return Ok(());
        }
        let Some(rect) = member.view.bounds() else {
            return Ok(());
        };
        let name = member
            .pack
            .as_deref()
            .and_then(sprites::pack)
            .map_or("Pet".to_owned(), |p| p.manifest.name.split(" — ").next().unwrap_or("Pet").to_owned());
        let target = ChatTarget { pet: self.world.pets[i].id.clone(), name };
        let pet = PetBox { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
        let side = match self.open_chat(target, pet) {
            Ok(side) => side,
            Err(e) => {
                log!("Không mở được cửa sổ chat: {}", e.message);
                return Ok(());
            }
        };
        if self.listening != Some(uid) {
            self.stop_listening();
        }
        self.listening = Some(uid);
        self.world.pet(i).listen(if side == 1 { 1 } else { -1 });
        self.wake();
        Ok(())
    }

    /// Mở khung chat với `target` cạnh `pet` (khung của pet, CSS pixel của overlay). Đang mở thì đổi sang pet đó, dời
    /// tới cạnh nó. Trả về phía của cửa sổ so với pet (-1 bên trái, 1 bên phải) để pet quay mặt về phía đó.
    fn open_chat(&mut self, target: ChatTarget, pet: PetBox) -> tinyworld_backend::error::AppResult<i8> {
        if !self.settings.get().chat {
            return Err(chat::chat_off());
        }
        let target = chat::checked_target(target)?;
        self.chat.set_target(Some(target.clone()));
        let geometry = self.overlay.geometry();
        let side = chat::side_of(&geometry.screen, pet, chat::WIDTH + chat::FRAME_GUESS.0);
        let message = json!({
            "open": "chat",
            "target": target,
            "title": chat::title(&target.name),
            "screen": geometry.screen,
            "origin": [geometry.window.x, geometry.window.y],
            "pet": pet,
            "side": side,
        });
        self.ipc.open(message);
        Ok(side)
    }

    fn open_settings(&mut self) {
        self.ipc.open(json!({ "open": ipc::SETTINGS }));
    }

    /// Đã bật chat mà con trỏ dừng trên một con chừng `CHAT_HOVER`: dựng sẵn khung chat (ẩn) để click chuột phải là hiện
    /// ngay. Con trỏ rời pet `CHAT_COOL` mà không mở thì thôi.
    fn prepare_chat(&mut self, now: Instant) {
        // Đang chat thì khung chat đã có sẵn.
        let hovering = self.chat_on
            && self.listening.is_none()
            && !self.held
            && self.hover_since.is_some_and(|since| now - since >= CHAT_HOVER);
        if hovering {
            if self.chat_warm_until.is_none() {
                self.ipc.warm(ipc::CHAT);
            }
            self.chat_warm_until = Some(now + CHAT_COOL);
        } else if self.chat_warm_until.is_some_and(|until| now >= until) {
            self.chat_warm_until = None;
            self.ipc.cool(ipc::CHAT);
        }
    }

    // Khay.

    fn tray_action(&mut self, action: tray::Action) -> Result<()> {
        match action {
            tray::Action::Pause => {
                let paused = self.overlay.toggle_paused();
                self.tray.set_paused(paused);
                self.set_paused(paused);
            }
            tray::Action::Quiet(index) => {
                // Đang bật đúng lựa chọn đó thì tắt, không thì bật nó (tính giờ lại từ đầu).
                self.set_quiet((self.quiet != Some(index)).then_some(index));
            }
            tray::Action::Toggle => {
                self.overlay.set_user_hidden(!self.overlay.is_user_hidden());
                self.apply_visibility();
            }
            tray::Action::Settings => self.open_settings(),
            tray::Action::Quit => self.request_quit(),
            // Bấm icon khay thì đã dựng sẵn Cài đặt (`Launcher`); menu đóng mà không chọn Cài đặt… thì thôi. Đã chọn
            // thì `open_settings` đã lấy cửa sổ dựng sẵn, `cool` không làm gì.
            tray::Action::Closed => self.ipc.cool(ipc::SETTINGS),
        }
        Ok(())
    }

    /// Tạm dừng: pet đứng yên và chuột đi xuyên qua pet, bỏ tạm dừng thì sống tiếp.
    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        if paused {
            self.cancel_interaction();
        }
        self.refresh_click_through();
        if !paused {
            self.wake();
        }
    }

    /// Im lặng theo lựa chọn `choice` (`None` là tắt): đánh dấu menu, báo ambience, hẹn giờ tự tắt.
    fn set_quiet(&mut self, choice: Option<usize>) {
        self.quiet = choice;
        self.quiet_until = choice.and_then(|i| tray::QUIET[i].2).map(|d| Instant::now() + d);
        self.tray.set_quiet(choice);
        self.ambience.set_quiet(&mut self.world, choice.is_some());
        self.flush_ambience();
    }

    fn cancel_interaction(&mut self) {
        let mut interaction = std::mem::take(&mut self.interaction);
        interaction.cancel(self);
        self.interaction = interaction;
    }

    /// Overlay ẩn (tray, app fullscreen) thì dừng hẳn, pet đứng nguyên chỗ cũ; hiện lại thì chạy tiếp.
    fn apply_visibility(&mut self) {
        let visible = self.overlay.is_visible();
        window::set_visible(self.hwnd, visible);
        self.hidden = !visible;
        if self.hidden {
            self.cancel_interaction();
        } else {
            // Lúc ẩn luồng theo dõi cửa sổ gỡ hook; hiện lại thì đánh thức để nó gắn lại và đọc ngay.
            window_list::refresh();
            self.wake();
        }
    }

    /// Tray bấm Thoát: lưu trạng thái rồi thoát.
    fn request_quit(&mut self) {
        self.autosave();
        self.activity.flush();
        self.quitting = true;
    }

    /// Lưu world.json nếu có gì đổi từ lần lưu trước.
    fn autosave(&mut self) {
        let snapshot = self.world.snapshot();
        let Ok(json) = serde_json::to_string(&snapshot) else {
            return;
        };
        if json == self.saved {
            return;
        }
        match serde_json::to_value(&snapshot).map_err(|e| e.to_string()).and_then(|v| {
            self.storage.save(&v).map_err(|e| e.message)
        }) {
            // Lần sau thử lại.
            Err(e) => log!("Không lưu được trạng thái pet: {e}"),
            Ok(()) => self.saved = json,
        }
    }

    // Màn hình và cửa sổ.

    /// Cửa sổ mở, đóng, di chuyển: dựng lại địa hình.
    fn apply_windows(&mut self, list: &WindowList) {
        self.world.set_windows(list.windows.clone(), &list.closed);
        // Taskbar tự ẩn trồi lên hoặc thụt xuống: pet đứng lên mép trên của nó, hoặc rơi xuống lại đáy màn hình.
        let floor_moved = list.taskbar_top != self.taskbar_top;
        if floor_moved {
            self.taskbar_top = list.taskbar_top;
            let bounds = bounds_of(&self.current, self.taskbar_top);
            self.world.set_screen(bounds, self.current.neighbors.clone(), None);
        }
        // Cả nhóm ngủ trên taskbar không bị cửa sổ ảnh hưởng: không chạy lại vòng lặp, mặt đất đổi thì chỉ vẽ lại.
        if self.world.resting() {
            if floor_moved {
                self.redraw_logged();
            }
            return;
        }
        // Tạm dừng thì không có vòng lặp, pet đứng trên cửa sổ vẫn phải đi theo.
        if self.paused {
            self.redraw_logged();
        } else {
            self.wake();
        }
    }

    fn redraw_logged(&mut self) {
        if let Err(e) = self.redraw() {
            log!("Không vẽ được overlay: {e}");
        }
    }

    /// Màn hình đổi độ phân giải, DPI, taskbar, hoặc màn hình khác được cắm/rút: đặt lại overlay trên màn hình đang ở.
    /// Màn hình đó bị rút ra thì sang màn hình chính.
    fn refit(&mut self) -> Result<()> {
        let all = screens::displays();
        let current = self.overlay.geometry();
        let (cx, cy) = current.window.center();
        let target = all
            .iter()
            .find(|d| !d.name.is_empty() && d.name == current.display)
            .or_else(|| all.iter().find(|d| d.monitor.contains(cx, cy)))
            .or_else(|| all.first());
        let Some(target) = target else {
            return Ok(());
        };
        let next = Geometry::new(target, &all);
        if next == current {
            return Ok(());
        }
        self.apply_geometry(&current, next)
    }

    /// Pet bị kéo hoặc đi ra khỏi overlay tới `point` (CSS pixel của overlay): điểm đó nằm trên màn hình khác thì
    /// overlay sang màn hình đó.
    fn move_overlay(&mut self, point: Point) -> Result<()> {
        let now = Instant::now();
        if self.last_move.is_some_and(|last| now - last < MOVE_INTERVAL) {
            return Ok(());
        }
        self.last_move = Some(now);
        let current = self.overlay.geometry();
        let (px, py) = current.to_physical(point.x, point.y);
        let all = screens::displays();
        let Some(target) = all.iter().find(|d| d.monitor.contains(px, py)) else {
            return Ok(());
        };
        let (cx, cy) = current.window.center();
        if target.monitor.contains(cx, cy) {
            return Ok(());
        }
        let next = Geometry::new(target, &all);
        self.apply_geometry(&current, next)
    }

    /// Đổi chỗ overlay, đổi toạ độ pet và cửa sổ sang overlay mới, rồi đọc lại danh sách cửa sổ theo toạ độ mới.
    fn apply_geometry(&mut self, current: &Geometry, next: Geometry) -> Result<()> {
        let _moving = self.overlay.moving();
        if next.window != current.window {
            window::place(self.hwnd, next.window);
        }
        let change = ScreenChange { remap: current.remap_to(&next), screen: next.screen.clone() };
        self.overlay.set_geometry(next);
        window_list::refresh();
        drop(_moving);
        self.screen_changed(change)
    }

    /// Overlay sang màn hình khác (pet bị kéo, ném, tự đi sang), hoặc màn hình đổi độ phân giải, DPI, taskbar.
    fn screen_changed(&mut self, change: ScreenChange) -> Result<()> {
        // Mép trên taskbar tự ẩn tính theo overlay cũ: bỏ, danh sách cửa sổ tới ngay sau đó sẽ báo lại.
        self.current = change.screen;
        self.taskbar_top = None;
        let bounds = bounds_of(&self.current, None);
        self.world.set_screen(bounds, self.current.neighbors.clone(), Some(change.remap));
        // Đang giữ chuột thì `Interaction` đang nằm ngoài `self` (`input`): đổi toạ độ sau.
        self.remaps.push(change.remap);
        let px = self.px();
        for member in &mut self.members {
            member.view.set_px(&self.gfx, px)?;
        }
        self.redraw()?;
        if !self.paused {
            self.wake();
        }
        Ok(())
    }

    /// Mất thiết bị GPU (driver cập nhật hay khởi động lại): tạo thiết bị mới và vẽ lại mọi con ở đúng chỗ cũ. Mô
    /// phỏng không đổi gì.
    fn recreate_graphics(&mut self) -> Result<()> {
        if let Some(mut combo) = self.combo.take() {
            combo.stop(&mut self.stage());
        }
        // Gỡ hết visual cũ khỏi cây rồi mới bỏ: còn treo trong cây thì cả thiết bị cũ không được giải phóng.
        let old: Vec<(Option<String>, u32)> = (self.members.drain(..))
            .map(|m| {
                m.destroy(&self.gfx);
                (m.pack.clone(), m.uid)
            })
            .collect();
        self.gfx.release();
        self.gfx = Gfx::new(self.hwnd)?;
        let ghost = Ghost::new(self.gfx.layer(None)?)?;
        let tag = TemperatureTag::new(self.gfx.layer(None)?)?;
        let packs: Vec<Option<String>> = old.iter().map(|(pack, _)| pack.clone()).collect();
        let prepared = view::prepare_all(&packs, self.size, self.px());
        for ((pack, uid), prepared) in old.into_iter().zip(prepared) {
            let mut member = Member::new(&self.gfx, pack, prepared)?;
            member.uid = uid;
            self.ambience.adopt(&mut member);
            self.members.push(member);
        }
        self.with_ambience(|ambience, ctx| ambience.rebuild(ctx, ghost, tag));
        self.last_error = None;
        self.redraw()?;
        self.wake();
        Ok(())
    }

    // Nhân vật.

    /// Chỗ đứng cho con mới: từ góc phải dọc theo mặt đất, chỗ đầu tiên không đứng sát con nào.
    fn spawn_x(&self, width: f64) -> f64 {
        let Bounds { left, right, .. } = self.world.bounds();
        let mut x = right - SPAWN_MARGIN - width / 2.0;
        while x >= left + width / 2.0 {
            if self.world.pets.iter().all(|p| (p.x - x).abs() >= width) {
                return x;
            }
            x -= width * SPAWN_GAP;
        }
        right - SPAWN_MARGIN - width / 2.0
    }

    /// Hiện đúng các con trong `packs` (`resolve_packs`), nạp pack mới ngay (lúc mở app).
    fn sync_pets(&mut self, packs: &[Option<String>]) -> Result<()> {
        let missing = self.missing_packs(packs);
        let prepared = view::prepare_all(&missing, self.size, self.px());
        self.place_pets(packs, missing.into_iter().zip(prepared).collect())
    }

    /// Như `sync_pets` nhưng nạp pack mới ở luồng nền (đổi nhân vật trong Cài đặt): trong lúc chờ các con đang có vẫn
    /// đi lại như thường, nạp xong mới thay (`pets_loaded`).
    fn sync_pets_later(&mut self, packs: Vec<Option<String>>) -> Result<()> {
        self.loading += 1;
        let missing = self.missing_packs(&packs);
        if missing.is_empty() {
            return self.place_pets(&packs, Vec::new());
        }
        let (generation, size, px, tx) = (self.loading, self.size, self.px(), self.tx.clone());
        std::thread::spawn(move || {
            let prepared = view::prepare_all(&missing, size, px);
            let loaded = missing.into_iter().zip(prepared).collect();
            let _ = tx.send(Message::PetsLoaded { generation, packs, loaded });
            window::wake();
        });
        Ok(())
    }

    /// Pack mới đã nạp xong (`sync_pets_later`).
    fn pets_loaded(
        &mut self,
        generation: u64,
        packs: &[Option<String>],
        loaded: Vec<(Option<String>, view::Prepared)>,
    ) -> Result<()> {
        // Trong lúc chờ đã đổi nhân vật lần nữa: bỏ kết quả của lần cũ.
        if generation != self.loading {
            return Ok(());
        }
        // Trong lúc chờ đã đổi cỡ: con mới hiện ở cỡ cũ rồi đổi cỡ cùng cả nhóm.
        let stale = loaded.iter().any(|(_, prepared)| !prepared.fits(self.size, self.px()));
        self.place_pets(packs, loaded)?;
        if stale {
            self.resize_members();
        }
        Ok(())
    }

    /// Các pack trong `packs` chưa có con nào trên màn hình.
    fn missing_packs(&self, packs: &[Option<String>]) -> Vec<Option<String>> {
        packs.iter().filter(|pack| !self.members.iter().any(|m| &m.pack == *pack)).cloned().collect()
    }

    /// Bỏ các con không còn trong `packs` rồi thêm các con đã nạp (`loaded`): pack mới nạp xong rồi mới bỏ con cũ, để
    /// lúc đổi nhân vật không có lúc trống; con mới đứng đúng chỗ con vừa bị bỏ.
    fn place_pets(&mut self, packs: &[Option<String>], loaded: Vec<(Option<String>, view::Prepared)>) -> Result<()> {
        let mut spots = Vec::new();
        let mut i = 0;
        while i < self.members.len() {
            if packs.contains(&self.members[i].pack) {
                i += 1;
                continue;
            }
            let member = self.members.remove(i);
            spots.push(self.remove_member(member));
        }
        for (pack, prepared) in loaded {
            let spot = if spots.is_empty() { None } else { Some(spots.remove(0)) };
            self.add_member(pack, prepared, spot)?;
        }
        self.redraw()?;
        self.wake();
        Ok(())
    }

    fn add_member(&mut self, pack: Option<String>, prepared: view::Prepared, spot: Option<(f64, Facing)>) -> Result<()> {
        let mut member = Member::new(&self.gfx, pack.clone(), prepared)?;
        let view = &member.view;
        let x = spot.map_or_else(|| self.spawn_x(view.width), |(x, _)| x);
        let id = pack.clone().unwrap_or_else(|| PLACEHOLDER_ID.to_owned());
        let i = self.world.spawn(PetOptions {
            id: id.clone(),
            x,
            width: view.width,
            height: view.height,
            reach: Some(view.reach),
        });
        if let Some((_, facing)) = spot {
            self.world.pets[i].facing = facing;
        }
        // Mở app: con nào đã lưu thì về chỗ cũ (đang ngủ thì ngủ tiếp). Bản cũ chỉ có một pet: chỗ của nó là chỗ của
        // con đầu tiên.
        let previous = self
            .restoring
            .get(&id)
            .or_else(|| if self.members.is_empty() { self.restoring.get(LEGACY_ID) } else { None })
            .cloned();
        if let Some(previous) = previous {
            self.world.pet(i).restore(&previous);
        }
        member.uid = self.world.pets[i].uid();
        self.ambience.adopt(&mut member);
        self.depth += 1;
        self.depths.insert(member.uid, self.depth);
        self.members.push(member);
        Ok(())
    }

    /// Bỏ một con (đổi nhân vật trong Settings); trả về chỗ của nó để con mới đứng vào.
    fn remove_member(&mut self, member: Member) -> (f64, Facing) {
        if self.interaction.held() == Some(member.uid) {
            self.cancel_interaction();
        }
        // Bỏ Long giữa combo thì dừng combo; bỏ con đang bị nhắm thì Long sang con khác.
        if let Some(mut combo) = self.combo.take() {
            combo.drop(&mut self.stage(), member.uid);
            if !combo.done() {
                self.combo = Some(combo);
            }
        }
        member.destroy(&self.gfx);
        self.depths.remove(&member.uid);
        if self.listening == Some(member.uid) {
            self.listening = None;
        }
        let spot = self
            .world
            .index_of(member.uid)
            .map(|i| (self.world.pets[i].x, self.world.pets[i].facing))
            .unwrap_or((0.0, 1));
        let id = self.world.index_of(member.uid).map(|i| self.world.pets[i].id.clone());
        if let Some(id) = id {
            self.world.remove(&id);
        }
        spot
    }

    /// Lưu cài đặt rồi áp dụng ngay (`set_settings` của Cài đặt). Đổi thành phố thì thời tiết cũ không còn đúng; đổi
    /// ngôn ngữ thì đổi luôn chữ trong menu khay và tiêu đề khung chat.
    pub fn set_settings(&mut self, settings: Settings) -> tinyworld_backend::error::AppResult<Settings> {
        let before = self.settings.get();
        let settings = self.settings.set(settings)?;
        self.activity.configure(&settings);
        if settings.lang != before.lang {
            i18n::set(settings.lang);
            self.tray.retitle(settings.lang);
            if let Some(target) = self.chat.target() {
                self.ipc.send(json!({ "title": chat::title(&target.name) }));
            }
        }
        // Tắt chat thì đóng luôn cửa sổ chat đang mở.
        if !settings.chat {
            self.ipc.send(json!({ "close": "chat" }));
        }
        self.ipc.event(tinyworld_backend::events::SETTINGS_CHANGED, json!(settings), None);
        if settings.city != before.city {
            let report = self.weather.current(settings.city.as_ref());
            self.ipc.event(tinyworld_backend::events::WEATHER_CHANGED, json!(report), None);
            self.weather.city_changed();
            self.apply_settings(&settings);
            self.with_ambience(|ambience, ctx| ambience.set_report(ctx, report));
        } else {
            self.apply_settings(&settings);
        }
        self.flush_ambience();
        Ok(settings)
    }

    fn apply_settings(&mut self, next: &Settings) {
        self.world.lang = next.lang;
        self.world.speed = next.speed;
        self.fps = next.fps;
        self.chat_on = next.chat;
        if !next.chat {
            self.chat_closed();
        }
        let next_settings = next.clone();
        self.with_ambience(|ambience, ctx| ambience.set_settings(ctx, next_settings));
        if next.size != self.size {
            self.size = next.size;
            self.resize_members();
        }
        if let Err(e) = self.sync_pets_later(sprites::resolve_packs(&next.pets)) {
            log!("Không đổi được nhân vật: {e}");
        }
    }

    /// Đổi cỡ cả nhóm theo `self.size`. Nạp lại và thu nhỏ frame ở luồng nền (bản debug mất cả giây mỗi con), xong mới
    /// đổi cỡ trên màn hình (`resized`); trong lúc chờ các con vẫn đi lại ở cỡ cũ.
    fn resize_members(&mut self) {
        self.resizing += 1;
        let generation = self.resizing;
        let (uids, ids): (Vec<u32>, Vec<Option<String>>) =
            self.members.iter().map(|m| (m.uid, m.view.sprite.id.clone())).unzip();
        let (size, px, tx) = (self.size, self.px(), self.tx.clone());
        std::thread::spawn(move || {
            let views = uids.into_iter().zip(view::prepare_all(&ids, size, px)).collect();
            let _ = tx.send(Message::Resized { generation, views });
            window::wake();
        });
    }

    /// Frame cho cỡ mới đã làm xong (`resize_members`): đổi cỡ từng con trên màn hình và trong mô phỏng.
    fn resized(&mut self, generation: u64, views: Vec<(u32, view::Prepared)>) -> Result<()> {
        // Trong lúc chờ đã đổi cỡ lần nữa: bỏ kết quả của lần cũ.
        if generation != self.resizing {
            return Ok(());
        }
        for (uid, prepared) in views {
            let Some(member) = self.members.iter_mut().find(|m| m.uid == uid) else { continue };
            member.view.apply(&self.gfx, prepared)?;
            if let Some(i) = self.world.index_of(uid) {
                let view = &member.view;
                self.world.pet(i).resize(view.width, view.height, Some(view.reach));
            }
        }
        self.redraw()?;
        self.wake();
        Ok(())
    }

    /// Shift + click vào Long: aura; có con khác thì diễn combo quật đuôi. Con khác chưa có kỹ năng riêng.
    fn skill(&mut self, uid: u32) -> bool {
        let ms = now_ms();
        let Some(long) = self.members.iter_mut().find(|m| m.uid == uid && m.pack.as_deref() == Some(LONG_PACK)) else {
            return false;
        };
        let Some(kit) = &mut long.kit else {
            return false;
        };
        kit.aura.activate(ms);
        // Tính là có đụng tới như click: cả nhóm không lăn ra ngủ ngay sau màn diễn.
        if let Some(i) = self.world.index_of(uid) {
            self.world.pets[i].since_interaction = 0.0;
        }
        let targets: Vec<u32> = (self.members.iter())
            .filter(|m| m.uid != uid)
            .filter(|m| self.world.index_of(m.uid).is_some_and(|i| self.world.pets[i].vanished == 0.0))
            .map(|m| m.uid)
            .collect();
        if self.combo.is_none() && !targets.is_empty() {
            let combo = LongCombo::start(&mut self.stage(), uid, targets, ms);
            self.combo = Some(combo);
        }
        self.wake();
        true
    }

    fn stage(&mut self) -> ComboStage<'_> {
        ComboStage { gfx: &self.gfx, world: &mut self.world, members: &mut self.members, px: self.current.scale_factor }
    }

    // Command của cửa sổ giao diện.

    fn call(&mut self, id: u64, name: &str, args: Value, writer: Writer) -> Result<()> {
        crate::commands::call(self, id, name, args, writer);
        Ok(())
    }

    pub(crate) fn services(&self) -> crate::commands::Services {
        crate::commands::Services {
            settings: self.settings.clone(),
            weather: self.weather.clone(),
            activity: self.activity.clone(),
            updater: self.updater.clone(),
            chat: self.chat.clone(),
            runtime: self.runtime.clone(),
            tx: self.tx.clone(),
        }
    }

    pub(crate) fn preview_state(&self) -> tinyworld_backend::preview::PreviewState {
        self.preview.get()
    }

    /// Mục Xem thử trong Cài đặt: cả nhóm dậy để thấy hiệu ứng. Thôi xem thử thì lúc nào cũng được.
    pub(crate) fn preview_weather(
        &mut self,
        preview: tinyworld_backend::preview::WeatherPreview,
    ) -> tinyworld_backend::error::AppResult<()> {
        preview.check()?;
        if !preview.is_stop() {
            tinyworld_backend::preview::ready(&self.overlay)?;
            self.world.wake_all();
        }
        self.with_ambience(|ambience, ctx| ambience.preview_weather(ctx, preview));
        self.flush_ambience();
        self.wake();
        Ok(())
    }

    pub(crate) fn preview_ghost(&mut self) -> tinyworld_backend::error::AppResult<()> {
        tinyworld_backend::preview::ready(&self.overlay)?;
        self.world.wake_all();
        let ms = now_ms();
        if let Err(e) = self.with_ambience(|ambience, ctx| ambience.preview_ghost(ctx, ms)) {
            log!("Không cho con ma bay được: {e}");
        }
        self.flush_ambience();
        self.wake();
        Ok(())
    }

    pub(crate) fn chat_target(&self) -> Option<ChatTarget> {
        self.chat.target()
    }

    pub(crate) fn settings_snapshot(&self) -> Settings {
        self.settings.get()
    }

    /// Bản dev: trạng thái overlay cho script thử (`debug_state`).
    #[cfg(debug_assertions)]
    pub(crate) fn debug_state(&self) -> Value {
        let pets: Vec<Value> = (self.world.pets.iter())
            .map(|p| {
                let member = self.members.iter().find(|m| m.uid == p.uid());
                json!({
                    "uid": p.uid(),
                    "id": p.id,
                    "state": p.state().name(),
                    "x": p.x,
                    "y": p.y,
                    "facing": p.facing,
                    "mount": p.mount.map(|m| m.id),
                    "speech": p.speech.as_ref().map(|s| s.text.clone()),
                    "vanished": p.vanished,
                    "sinceInteraction": p.since_interaction,
                    "bounds": member.and_then(|m| m.view.bounds()),
                    "body": member.and_then(|m| m.view.body()),
                    "hidden": member.is_some_and(|m| m.view.sprite_hidden()),
                })
            })
            .collect();
        json!({
            "running": self.running,
            "hidden": self.hidden,
            "paused": self.paused,
            "resting": self.resting,
            "clickThrough": self.click_through,
            "bounds": [self.world.bounds().left, self.world.bounds().top, self.world.bounds().right, self.world.bounds().floor],
            "screen": self.current,
            "cursor": self.cursor.map(|c| [c.x, c.y]),
            "night": self.world.night,
            "quiet": self.world.quiet,
            "combo": self.combo.is_some(),
            "ghost": self.ambience.flying(),
            "pets": pets,
        })
    }

    /// Bản dev: mở cửa sổ Cài đặt (`{"window": "settings"}`) hoặc khung chat với con thứ `index`, như bấm khay hay click
    /// chuột phải.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_open(&mut self, args: &Value) -> tinyworld_backend::error::AppResult<Value> {
        if args["window"] == "settings" {
            self.open_settings();
            return Ok(Value::Null);
        }
        // Dựng sẵn hay thôi dựng sẵn cửa sổ (`{"warm": "settings"}`, `{"cool": "chat"}`), như bấm khay, trỏ vào pet.
        for (key, warm) in [("warm", true), ("cool", false)] {
            if let Some(window) = [ipc::SETTINGS, ipc::CHAT].into_iter().find(|w| args[key] == *w) {
                if warm {
                    self.ipc.warm(window);
                } else {
                    self.ipc.cool(window);
                }
                return Ok(Value::Null);
            }
        }
        let index = args["index"].as_u64().unwrap_or(0) as usize;
        let uid = self.world.pets.get(index).map(|p| p.uid()).unwrap_or_default();
        let center = (self.members.iter())
            .find(|m| m.uid == uid)
            .and_then(|m| m.view.body())
            .map(|b| Point { x: b.x + b.width / 2.0, y: b.y + b.height / 2.0 });
        if let Some(point) = center {
            self.context_menu(point).map_err(tinyworld_backend::error::AppError::internal)?;
        }
        Ok(self.debug_state())
    }

    /// Bản dev: tạo lại thiết bị GPU như lúc mất thiết bị.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_rebuild(&mut self) -> tinyworld_backend::error::AppResult<Value> {
        self.recreate_graphics().map_err(tinyworld_backend::error::AppError::internal)?;
        Ok(self.debug_state())
    }

    /// Bản dev: làm gì đó với con thứ `index` (`debug_pet`): click, nhấc lên, kéo, thả, nói, buồn ngủ, kỹ năng, bấm đúp.
    #[cfg(debug_assertions)]
    pub(crate) fn debug_pet(&mut self, args: &Value) -> tinyworld_backend::error::AppResult<Value> {
        use tinyworld_backend::error::AppError;
        let index = args["index"].as_u64().unwrap_or(0) as usize;
        let Some(uid) = self.world.pets.get(index).map(|p| p.uid()) else {
            return Err(AppError::bad_request("không có con đó"));
        };
        let num = |key: &str| args[key].as_f64().unwrap_or(0.0);
        match args["action"].as_str().unwrap_or_default() {
            "poke" => self.world.pet(index).poke(),
            "grab" => self.world.pet(index).grab(),
            "drag" => self.world.pet(index).drag_to(num("x"), num("y")),
            "release" => self.world.pet(index).release(num("vx"), num("vy")),
            "say" => self.world.pets[index].say(args["text"].as_str().unwrap_or_default(), num("seconds").max(1.0)),
            "sleepy" => self.world.pets[index].since_interaction = 1e6,
            "skill" => {
                self.skill(uid);
            }
            "double" => interaction::Host::double_click(self, uid),
            other => return Err(AppError::bad_request(format!("không hiểu {other}"))),
        }
        self.wake();
        Ok(self.debug_state())
    }
}

/// Pet sống trong vùng làm việc; mặt đất là mép dưới, tức là mép trên taskbar. Taskbar tự ẩn đang trồi lên
/// (`taskbar_top`) thì mặt đất là mép trên của nó.
fn bounds_of(screen: &ScreenInfo, taskbar_top: Option<f64>) -> Bounds {
    let Rect { x, y, width, height } = screen.work_area;
    let floor = taskbar_top.map_or(y + height, |top| (y + height).min(top));
    Bounds { left: x, right: x + width, top: y, floor }
}

/// Xử lý hết message đang chờ. `false` khi app phải thoát.
fn pump() -> bool {
    let mut msg = MSG::default();
    unsafe {
        while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
            if msg.message == WM_QUIT {
                return false;
            }
            DispatchMessageW(&msg);
        }
    }
    true
}

impl interaction::Host for App {
    fn pick(&self, point: Point) -> Option<u32> {
        App::pick(self, point)
    }

    fn foot(&self, pet: u32) -> Option<Point> {
        let member = self.members.iter().find(|m| m.uid == pet)?;
        member.view.foot.or_else(|| self.world.index_of(pet).map(|i| Point { x: self.world.pets[i].x, y: self.world.pets[i].y }))
    }

    fn grab_pointer(&mut self, pet: u32) {
        if let Some(member) = self.members.iter().find(|m| m.uid == pet) {
            if let Err(e) = member.raise(&self.gfx) {
                log!("Không đưa được pet lên trên: {e}");
            }
            self.depth += 1;
            self.depths.insert(pet, self.depth);
        }
        window::capture(self.hwnd, true);
    }

    fn release_pointer(&mut self) {
        window::capture(self.hwnd, false);
    }

    fn hold(&mut self, held: bool) {
        self.held = held;
        self.refresh_click_through();
    }

    fn activity(&mut self) {
        self.wake();
    }

    fn grab(&mut self, pet: u32) {
        if let Some(i) = self.world.index_of(pet) {
            self.world.pet(i).grab();
        }
    }

    fn drag_to(&mut self, pet: u32, x: f64, y: f64) {
        if let Some(i) = self.world.index_of(pet) {
            self.world.pet(i).drag_to(x, y);
        }
    }

    fn release(&mut self, pet: u32, vx: f64, vy: f64) {
        if let Some(i) = self.world.index_of(pet) {
            self.world.pet(i).release(vx, vy);
        }
    }

    fn poke(&mut self, pet: u32) {
        if let Some(i) = self.world.index_of(pet) {
            self.world.pet(i).poke();
        }
    }

    fn drag_outside(&mut self, point: Point) {
        if let Err(e) = self.move_overlay(point) {
            log!("Không đưa được overlay sang màn hình khác: {e}");
        }
    }

    fn inside(&self, point: Point) -> bool {
        let window = self.overlay.geometry().window;
        let px = self.px();
        point.x >= 0.0 && point.y >= 0.0 && point.x < f64::from(window.width) / px && point.y < f64::from(window.height) / px
    }

    fn double_click(&mut self, pet: u32) {
        // Bấm đúp: nói giờ, ngày, âm lịch, thời tiết.
        if let Some(i) = self.world.index_of(pet) {
            self.ambience.tell_time(&mut self.world, i);
            self.wake();
        }
    }

    fn skill(&mut self, pet: u32) -> bool {
        App::skill(self, pet)
    }
}

/// Phần overlay combo của Long dùng tới.
struct ComboStage<'a> {
    gfx: &'a Gfx,
    world: &'a mut World,
    members: &'a mut [Member],
    px: f64,
}

impl ComboStage<'_> {
    fn member(&mut self, pet: u32) -> Option<&mut Member> {
        self.members.iter_mut().find(|m| m.uid == pet)
    }

    fn kit(&mut self) -> Option<&mut crate::long::LongKit> {
        self.members.iter_mut().find_map(|m| m.kit.as_mut())
    }
}

impl Stage for ComboStage<'_> {
    fn state(&self, pet: u32) -> Option<PetState> {
        self.world.index_of(pet).map(|i| self.world.pets[i].state())
    }

    fn position(&self, pet: u32) -> Option<Point> {
        self.world.index_of(pet).map(|i| Point { x: self.world.pets[i].x, y: self.world.pets[i].y })
    }

    fn facing(&self, pet: u32) -> Facing {
        self.world.index_of(pet).map_or(1, |i| self.world.pets[i].facing)
    }

    fn set_facing(&mut self, pet: u32, facing: Facing) {
        if let Some(i) = self.world.index_of(pet) {
            self.world.pets[i].facing = facing;
        }
    }

    fn hold(&mut self, pet: u32, facing: Facing, seconds: f64) -> bool {
        self.world.index_of(pet).is_some_and(|i| self.world.pet(i).hold(facing, seconds))
    }

    fn blink_to(&mut self, pet: u32, target: u32, x: f64, seconds: f64) {
        if let (Some(i), Some(j)) = (self.world.index_of(pet), self.world.index_of(target)) {
            self.world.pet(i).blink_to(j, x, seconds);
        }
    }

    fn startle(&mut self, pet: u32) {
        if let Some(i) = self.world.index_of(pet) {
            self.world.pet(i).startle();
        }
    }

    fn vanish(&mut self, pet: u32, seconds: f64) {
        if let Some(i) = self.world.index_of(pet) {
            self.world.pets[i].vanish(seconds);
        }
    }

    fn size(&self, pet: u32) -> (f64, f64) {
        self.members.iter().find(|m| m.uid == pet).map_or((0.0, 0.0), |m| (m.view.width, m.view.height))
    }

    fn set_covered(&mut self, pet: u32, covered: bool) {
        if let Some(member) = self.member(pet) {
            if let Err(e) = member.view.set_covered(covered) {
                log!("Không ẩn được sprite: {e}");
            }
        }
    }

    fn play(&mut self, action: Action, now: f64, facing: Facing) {
        if let Some(kit) = self.kit() {
            if let Err(e) = kit.action.play(action, now, facing) {
                log!("Không diễn được cinematic của Long: {e}");
            }
        }
    }

    fn stop_action(&mut self) {
        if let Some(kit) = self.kit() {
            if let Err(e) = kit.action.stop() {
                log!("Không dừng được cinematic của Long: {e}");
            }
        }
    }

    fn teleport(&mut self, at: Point, size: (f64, f64), direction: Facing, reverse: bool, now: f64, duration: f64) {
        let (gfx, px) = (self.gfx, self.px);
        if let Some(kit) = self.kit() {
            if let Err(e) = kit.teleport.play(gfx, at, size, direction, reverse, now, duration, px) {
                log!("Không vẽ được vệt tốc biến: {e}");
            }
        }
    }
}
