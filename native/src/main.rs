//! PoC overlay TinyWorld vẽ bằng DirectComposition, không có WebView2 (xem README.md).
//!
//!   cargo run --release -- [--pets a,b] [--size 1] [--speed 1] [--mode auto|idle|walk|run] [--trace]
//!                          [--exclude-capture] [--no-layered]
//!
//! Mặc định lấy nhân vật, cỡ và tốc độ trong Cài đặt của TinyWorld (chỉ đọc, không ghi gì vào máy). Cổng điều
//! khiển: control.rs.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod control;
mod gfx;
mod member;
mod overlay;
mod pet;
mod sprites;

use anyhow::{bail, Context, Result};
use control::Command;
use gfx::Gfx;
use member::Member;
use overlay::Input;
use pet::{Mode, Pet, World};
use std::collections::VecDeque;
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, HWND, POINT};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::System::Threading::{
    CreateWaitableTimerExW, GetCurrentThreadId, SetWaitableTimer, CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, INFINITE,
    TIMER_ALL_ACCESS,
};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, DispatchMessageW, GetCursorPos, MsgWaitForMultipleObjects, PeekMessageW, MSG, PM_REMOVE,
    QS_ALLINPUT, WM_QUIT,
};

/// Nhịp bước và vẽ lúc không có gì chuyển động nhanh (`FPS` trong desktop/src/main.ts).
const STEP: Duration = Duration::from_nanos(1_000_000_000 / 30);
/// Thức dậy sớm hơn hạn bước chừng này vẫn bước luôn, khỏi lỡ nhịp vì hẹn giờ lệch.
const STEP_SLACK: Duration = Duration::from_millis(4);
/// Nhịp đọc con trỏ để bật tắt chuột đi xuyên (cursor.rs của shell Tauri cũng khoảng 60 lần/giây).
const POLL: Duration = Duration::from_nanos(1_000_000_000 / 60);
/// `DwmFlush` trả về nhanh hơn chừng này là không có gì chờ ghép hình: hẹn giờ như thường để khỏi quay vòng.
const FLUSH_MIN: Duration = Duration::from_millis(2);
/// Vận tốc lúc buông tay tính trên đoạn kéo cuối dài chừng này.
const THROW_WINDOW: Duration = Duration::from_millis(100);
/// Đếm số lần thức, bước, vẽ, commit trong mỗi khoảng này (`--trace`, lệnh `stats`).
const STATS_EVERY: Duration = Duration::from_secs(5);

struct Options {
    pets: Vec<String>,
    size: f32,
    speed: f32,
    mode: Mode,
    trace: bool,
    exclude_capture: bool,
    layered: bool,
    assets: PathBuf,
}

fn options() -> Result<Options> {
    let mut options = Options {
        pets: Vec::new(),
        size: 1.0,
        speed: 1.0,
        mode: Mode::Auto,
        trace: false,
        exclude_capture: false,
        layered: true,
        assets: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../assets/sprites"),
    };
    if let Some(settings) = user_settings() {
        if let Some(pets) = settings["pets"].as_array() {
            options.pets = pets.iter().filter_map(|p| p.as_str().map(str::to_owned)).collect();
        }
        options.size = settings["size"].as_f64().unwrap_or(1.0) as f32;
        options.speed = settings["speed"].as_f64().unwrap_or(1.0) as f32;
    }
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().with_context(|| format!("{arg} thiếu giá trị"));
        match arg.as_str() {
            "--pets" => options.pets = value()?.split(',').map(str::to_owned).collect(),
            "--size" => options.size = value()?.parse()?,
            "--speed" => options.speed = value()?.parse()?,
            "--mode" => options.mode = Mode::parse(&value()?).context("--mode: auto, idle, walk hoặc run")?,
            "--assets" => options.assets = PathBuf::from(value()?),
            "--trace" => options.trace = true,
            "--exclude-capture" => options.exclude_capture = true,
            "--no-layered" => options.layered = false,
            other => bail!("không hiểu tham số {other}"),
        }
    }
    if options.pets.is_empty() {
        options.pets.push("a-momo".into());
    }
    Ok(options)
}

/// Cài đặt của TinyWorld (shell Tauri), nếu có.
fn user_settings() -> Option<serde_json::Value> {
    let path = PathBuf::from(std::env::var_os("APPDATA")?).join("com.tinyworld.app").join("settings.json");
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn main() {
    if let Err(err) = run() {
        eprintln!("tinyworld-native: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let options = options()?;
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let (tx, rx) = mpsc::channel();
    match control::serve(tx, unsafe { GetCurrentThreadId() }) {
        Err(err) if err.kind() == ErrorKind::AddrInUse => bail!("đã có một bản đang chạy (cổng {})", control::PORT),
        result => result.context("mở cổng điều khiển")?,
    }
    let (rect, px) = overlay::work_area().context("không đọc được màn hình chính")?;
    let hwnd = overlay::create(rect, options.layered)?;
    let capture = options.exclude_capture.then(|| match overlay::exclude_from_capture(hwnd) {
        Ok(()) => "ok".to_owned(),
        Err(err) => format!("{err:#}"),
    });
    if let Some(capture) = &capture {
        eprintln!("WDA_EXCLUDEFROMCAPTURE: {capture}");
    }
    let gfx = Gfx::new(hwnd)?;
    let seed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    let mut members = Vec::new();
    for (i, name) in options.pets.iter().enumerate() {
        let x = rect.width as f32 * (i + 1) as f32 / (options.pets.len() + 1) as f32;
        let pet = Pet::new(x, rect.height as f32, seed ^ ((i as u64 + 1) * 7919));
        let member = Member::load(&gfx, &options.assets.join(name), options.size, px, pet)
            .with_context(|| format!("nạp nhân vật {name}"))?;
        members.push(member);
    }
    let app = App {
        hwnd,
        origin: (rect.x, rect.y),
        area: (rect.width as f32, rect.height as f32),
        px,
        pace: options.speed,
        mode: options.mode,
        layered: options.layered,
        click_through: true,
        gfx,
        members,
        drag: None,
        trace: options.trace,
        capture,
        stats: Stats::new(),
    };
    app.run(rx)
}

struct Drag {
    member: usize,
    /// Từ con trỏ tới điểm giữa chân.
    offset: (f32, f32),
    /// Vị trí con trỏ gần đây, để tính vận tốc lúc buông.
    trail: VecDeque<(Instant, f32, f32)>,
}

#[derive(Default, Clone, Copy)]
struct Counters {
    wakes: u32,
    steps: u32,
    draws: u32,
    commits: u32,
}

struct Stats {
    since: Instant,
    now: Counters,
    last: Counters,
    last_secs: f32,
}

impl Stats {
    fn new() -> Self {
        Stats { since: Instant::now(), now: Counters::default(), last: Counters::default(), last_secs: 0.0 }
    }

    /// Hết một khoảng đếm thì chốt số liệu, trả `true`.
    fn roll(&mut self) -> bool {
        let elapsed = self.since.elapsed();
        if elapsed < STATS_EVERY {
            return false;
        }
        self.last = std::mem::take(&mut self.now);
        self.last_secs = elapsed.as_secs_f32();
        self.since = Instant::now();
        true
    }

    fn per_second(&self) -> [f32; 4] {
        let Counters { wakes, steps, draws, commits } = self.last;
        let secs = self.last_secs.max(f32::EPSILON);
        [wakes, steps, draws, commits].map(|n| n as f32 / secs)
    }
}

struct App {
    hwnd: HWND,
    /// Góc trên trái của overlay, toạ độ desktop.
    origin: (i32, i32),
    area: (f32, f32),
    px: f32,
    pace: f32,
    mode: Mode,
    layered: bool,
    click_through: bool,
    gfx: Gfx,
    members: Vec<Member>,
    drag: Option<Drag>,
    trace: bool,
    capture: Option<String>,
    stats: Stats,
}

impl App {
    fn run(mut self, commands: Receiver<Command>) -> Result<()> {
        let timer = unsafe {
            CreateWaitableTimerExW(None, PCWSTR::null(), CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, TIMER_ALL_ACCESS.0)?
        };
        let mut last = Instant::now();
        let mut since_step = STEP;
        loop {
            if !pump() {
                return Ok(());
            }
            for command in commands.try_iter() {
                self.command(command);
            }
            let now = Instant::now();
            since_step += now - last;
            last = now;
            self.stats.now.wakes += 1;
            let cursor = self.cursor();
            for input in overlay::take_input() {
                self.input(input, cursor, now);
            }
            let fast = self.drag.is_some() || self.members.iter().any(|m| m.pet.state.fast());
            let mut committed = false;
            if fast || since_step + STEP_SLACK >= STEP {
                let dt = since_step.as_secs_f32().min(0.1);
                since_step = Duration::ZERO;
                committed = self.step(dt, cursor, now)?;
            }
            self.update_click_through(cursor);
            if self.stats.roll() && self.trace {
                let [wakes, steps, draws, commits] = self.stats.per_second();
                eprintln!("mỗi giây: thức {wakes:.1}, bước {steps:.1}, vẽ {draws:.1}, commit {commits:.1}");
            }
            // Đang chuyển động nhanh: chờ DWM ghép xong lần commit này rồi bước tiếp, tức là đi theo nhịp màn hình.
            if fast && committed {
                let start = Instant::now();
                let flushed = unsafe { DwmFlush() }.is_ok();
                if !flushed || start.elapsed() < FLUSH_MIN {
                    wait(timer, POLL);
                }
            } else {
                wait(timer, POLL);
            }
        }
    }

    fn world(&self, member: &Member) -> World {
        let half = member.half_width();
        World { left: half, right: self.area.0 - half, ground: self.area.1, px: self.px, pace: self.pace }
    }

    /// Con trỏ theo pixel của overlay.
    fn cursor(&self) -> (f32, f32) {
        let mut point = POINT::default();
        let _ = unsafe { GetCursorPos(&mut point) };
        ((point.x - self.origin.0) as f32, (point.y - self.origin.1) as f32)
    }

    /// Bước mọi pet `dt` giây, vẽ lại pet đổi frame, dời pet đổi chỗ, rồi commit một lần. `true` nếu có commit.
    fn step(&mut self, dt: f32, cursor: (f32, f32), now: Instant) -> Result<bool> {
        self.stats.now.steps += 1;
        if let Some(drag) = &mut self.drag {
            self.members[drag.member].pet.drag_to(cursor.0 + drag.offset.0, cursor.1 + drag.offset.1);
            drag.trail.push_back((now, cursor.0, cursor.1));
            while drag.trail.len() > 2 && now - drag.trail[0].0 > THROW_WINDOW {
                drag.trail.pop_front();
            }
        }
        let mut changed = false;
        for i in 0..self.members.len() {
            let world = self.world(&self.members[i]);
            let mode = self.mode;
            let member = &mut self.members[i];
            member.pet.step(dt, &world, mode);
            let (drew, moved) = member.render()?;
            if drew {
                self.stats.now.draws += 1;
            }
            changed |= drew || moved;
        }
        if changed {
            self.gfx.commit()?;
            self.stats.now.commits += 1;
        }
        Ok(changed)
    }

    fn input(&mut self, input: Input, cursor: (f32, f32), now: Instant) {
        match input {
            Input::Press => {
                if self.drag.is_some() {
                    return;
                }
                // Pet thêm sau nằm trên.
                let Some(member) = self.members.iter().rposition(|m| m.hit(cursor.0, cursor.1)) else { return };
                let pet = &mut self.members[member].pet;
                pet.grab();
                let offset = (pet.x - cursor.0, pet.y - cursor.1);
                self.drag = Some(Drag { member, offset, trail: VecDeque::from([(now, cursor.0, cursor.1)]) });
                unsafe { SetCapture(self.hwnd) };
            }
            Input::Release => {
                let Some(drag) = self.drag.take() else { return };
                let (vx, vy) = match (drag.trail.front(), drag.trail.back()) {
                    (Some(first), Some(last)) if last.0 > first.0 => {
                        let dt = (last.0 - first.0).as_secs_f32();
                        ((last.1 - first.1) / dt, (last.2 - first.2) / dt)
                    }
                    _ => (0.0, 0.0),
                };
                let world = self.world(&self.members[drag.member]);
                self.members[drag.member].pet.release(vx, vy, &world);
                unsafe {
                    let _ = ReleaseCapture();
                }
            }
        }
    }

    /// Con trỏ nằm trên phần có hình của pet (hoặc đang kéo) thì overlay nhận chuột, còn lại cho đi xuyên.
    fn update_click_through(&mut self, cursor: (f32, f32)) {
        if !self.layered {
            return;
        }
        let over = self.drag.is_some() || self.members.iter().any(|m| m.hit(cursor.0, cursor.1));
        if over == self.click_through {
            self.click_through = !over;
            overlay::set_click_through(self.hwnd, self.click_through);
        }
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::Quit => unsafe {
                let _ = DestroyWindow(self.hwnd);
            },
            Command::Mode(mode) => self.mode = mode,
            Command::Drop { pet, x } => {
                if let Some(member) = self.members.get_mut(pet) {
                    let top = member.height as f32;
                    member.pet.drop_at(x, top);
                }
            }
            Command::Stats(mut stream) => {
                let [wakes, steps, draws, commits] = self.stats.per_second();
                let pets: Vec<_> = self
                    .members
                    .iter()
                    .map(|m| serde_json::json!({ "state": format!("{:?}", m.pet.state), "x": m.pet.x, "y": m.pet.y }))
                    .collect();
                let reply = serde_json::json!({
                    "seconds": self.stats.last_secs,
                    "wakes": wakes,
                    "steps": steps,
                    "draws": draws,
                    "commits": commits,
                    "mode": format!("{:?}", self.mode),
                    "clickThrough": self.click_through,
                    "excludeCapture": self.capture,
                    "pets": pets,
                });
                let _ = writeln!(stream, "{reply}");
            }
        }
    }
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

/// Ngủ `duration`, thức sớm nếu có message (chuột, lệnh điều khiển).
fn wait(timer: HANDLE, duration: Duration) {
    let due = -((duration.as_nanos() / 100) as i64);
    unsafe {
        if SetWaitableTimer(timer, &due, 0, None, None, false).is_ok() {
            MsgWaitForMultipleObjects(Some(&[timer]), false, INFINITE, QS_ALLINPUT);
        } else {
            std::thread::sleep(duration);
        }
    }
}
