//! Icon ở system tray. Overlay không có nút trên taskbar nên mọi điều khiển của app nằm ở đây: Tạm dừng, Im lặng,
//! Ẩn / hiện pet, Cài đặt…, Thoát. Bấm một mục thì `muda` gửi event; vòng lặp chính đọc ở `take_events`.
//!
//! Icon và menu sống trên một luồng riêng: menu khay chạy vòng lặp modal (`TrackPopupMenu`) trên luồng tạo icon tới
//! khi đóng, để trên luồng chính thì pet đứng hình suốt lúc menu mở.

use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use tinyworld_backend::i18n::Lang;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, PeekMessageW, PostMessageW, MSG, PM_REMOVE, WM_CANCELMODE, WM_NULL,
};

/// Các lựa chọn Im lặng: id mục trong menu, khoá chữ, thời gian (`None`: tới khi tắt).
pub const QUIET: [(&str, &str, Option<Duration>); 3] = [
    ("quiet-30", "tray.quiet30", Some(Duration::from_secs(30 * 60))),
    ("quiet-60", "tray.quiet60", Some(Duration::from_secs(60 * 60))),
    ("quiet-on", "tray.quietOn", None),
];

/// Lúc thoát chờ luồng khay gỡ icon tối đa chừng này.
const STOP_WAIT: Duration = Duration::from_secs(1);

/// Một mục vừa được bấm, hay menu vừa đóng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Pause,
    /// Chỉ số trong `QUIET`.
    Quiet(usize),
    Toggle,
    Settings,
    Quit,
    /// Menu vừa đóng; có chọn mục nào thì mục đó tới trước.
    Closed,
}

impl Action {
    fn from_id(id: &str) -> Option<Self> {
        match id {
            "pause" => Some(Self::Pause),
            "toggle" => Some(Self::Toggle),
            "settings" => Some(Self::Settings),
            "quit" => Some(Self::Quit),
            id => QUIET.iter().position(|(quiet, ..)| *quiet == id).map(Self::Quiet),
        }
    }
}

/// Vòng lặp chính nhờ luồng khay đổi menu.
enum Command {
    Paused(bool),
    Quiet(Option<usize>),
    Retitle(Lang),
    /// App thoát: gỡ icon rồi dừng luồng.
    Stop,
}

/// Icon của app trong exe (tài nguyên Tauri tạo lúc build); bản chạy riêng `native` không có thì dùng ảnh nhúng sẵn.
const ICON_RESOURCE: u16 = 32512;

/// Cỡ icon nhỏ của hệ thống: 16 px ở 100%, 24 px ở 150%... icon.ico có bản 16/24/32 px vẽ tay theo lưới pixel.
fn small_size() -> u32 {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSMICON};
    match unsafe { GetSystemMetrics(SM_CXSMICON) } {
        n if n > 0 => n as u32,
        _ => 16,
    }
}

fn icon() -> Option<Icon> {
    let size = small_size();
    if let Ok(icon) = Icon::from_resource(ICON_RESOURCE, Some((size, size))) {
        return Some(icon);
    }
    let image = image::load_from_memory(include_bytes!("../../desktop/src-tauri/icons/32x32.png")).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

/// Đầu bên vòng lặp chính của icon khay.
pub struct Tray {
    /// Cửa sổ ẩn của `tray-icon` trên luồng khay: post message vào đó để luồng khay đọc `commands`.
    hwnd: isize,
    commands: Sender<Command>,
    events: Receiver<Action>,
    /// Luồng khay dừng (đã gỡ icon) thì đầu gửi bị bỏ.
    done: Receiver<()>,
}

/// Icon và các mục menu. `muda` không cho đưa mục menu sang luồng khác nên chỉ dùng trên luồng khay.
struct Items {
    icon: TrayIcon,
    pause: CheckMenuItem,
    quiet: Submenu,
    quiet_items: Vec<CheckMenuItem>,
    toggle: MenuItem,
    settings: MenuItem,
    quit: MenuItem,
}

/// Chữ của các mục: Tạm dừng, Im lặng, Ẩn / hiện pet, Cài đặt…, Thoát.
fn labels(lang: Lang) -> [String; 5] {
    ["tray.pause", "tray.quiet", "tray.toggle", "tray.settings", "tray.quit"].map(|key| lang.t(key))
}

impl Tray {
    /// Tạo icon khay trên luồng riêng; `wake` đánh thức vòng lặp chính mỗi lần có mục được bấm hay menu đóng.
    /// `on_open` chạy trên luồng khay lúc bấm icon, ngay trước khi menu hiện.
    pub fn new(lang: Lang, wake: fn(), on_open: impl Fn() + Send + Sync + 'static) -> Result<Self> {
        let (tx, events) = mpsc::channel();
        let menu_tx = tx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some(action) = Action::from_id(event.id.as_ref()) {
                let _ = menu_tx.send(action);
                wake();
            }
        }));
        let opened = Arc::new(AtomicBool::new(false));
        let clicked = opened.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, .. } = event {
                if matches!(button, MouseButton::Left | MouseButton::Right) {
                    clicked.store(true, Ordering::Relaxed);
                    on_open();
                }
            }
        }));
        let (commands, received) = mpsc::channel();
        let (ready_tx, ready) = mpsc::channel();
        let (done_tx, done) = mpsc::channel::<()>();
        std::thread::spawn(move || {
            let _done = done_tx;
            serve(lang, wake, &opened, &tx, &received, &ready_tx);
        });
        let hwnd = ready.recv().context("luồng khay dừng trước khi tạo icon")??;
        Ok(Self { hwnd, commands, events, done })
    }

    /// Các mục vừa được bấm (và lần menu đóng) từ lần gọi trước.
    pub fn take_events(&self) -> Vec<Action> {
        self.events.try_iter().collect()
    }

    /// Menu tự đảo dấu tick khi bấm; đặt lại theo trạng thái thật.
    pub fn set_paused(&self, paused: bool) {
        self.send(Command::Paused(paused));
    }

    pub fn set_quiet(&self, choice: Option<usize>) {
        self.send(Command::Quiet(choice));
    }

    /// Đổi chữ trong menu theo ngôn ngữ `lang`.
    pub fn retitle(&self, lang: Lang) {
        self.send(Command::Retitle(lang));
    }

    /// Gửi lệnh rồi đánh thức luồng khay; menu đang mở thì lệnh chờ tới lúc menu đóng.
    fn send(&self, command: Command) {
        let _ = self.commands.send(command);
        self.post(WM_NULL);
    }

    fn post(&self, message: u32) {
        unsafe {
            let _ = PostMessageW(Some(HWND(self.hwnd as _)), message, WPARAM(0), LPARAM(0));
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        // Thoát lúc menu đang mở (từ Cài đặt, lúc cài bản mới) thì đóng menu để luồng khay đọc được lệnh dừng.
        self.post(WM_CANCELMODE);
        // Chờ luồng khay gỡ icon, không thì icon còn trên khay tới lúc rê chuột qua.
        let _ = self.done.recv_timeout(STOP_WAIT);
    }
}

/// Luồng khay: tạo icon, báo `ready` (cửa sổ ẩn của icon hoặc lỗi), rồi xử lý message tới khi `Command::Stop`.
fn serve(
    lang: Lang,
    wake: fn(),
    opened: &AtomicBool,
    events: &Sender<Action>,
    commands: &Receiver<Command>,
    ready: &Sender<Result<isize>>,
) {
    let items = match Items::new(lang) {
        Ok(items) => items,
        Err(e) => {
            let _ = ready.send(Err(e));
            return;
        }
    };
    let _ = ready.send(Ok(items.icon.window_handle() as isize));
    let mut msg = MSG::default();
    // GetMessageW trả 0 lúc gặp WM_QUIT, -1 lúc lỗi.
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
        unsafe { DispatchMessageW(&msg) };
        // Bấm icon thì menu hiện rồi đóng ngay trong lần dispatch vừa rồi. Mục được chọn còn nằm trong hàng đợi
        // (WM_COMMAND): xử lý nốt rồi mới báo menu đóng.
        if opened.swap(false, Ordering::Relaxed) {
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    DispatchMessageW(&msg);
                }
            }
            let _ = events.send(Action::Closed);
            wake();
        }
        for command in commands.try_iter() {
            match command {
                Command::Paused(paused) => items.pause.set_checked(paused),
                Command::Quiet(choice) => items.set_quiet(choice),
                Command::Retitle(lang) => items.retitle(lang),
                Command::Stop => return,
            }
        }
    }
}

impl Items {
    fn new(lang: Lang) -> Result<Self> {
        let [pause, quiet, toggle, settings, quit] = labels(lang);
        let pause = CheckMenuItem::with_id("pause", pause, true, false, None);
        let quiet_items: Vec<CheckMenuItem> =
            QUIET.iter().map(|(id, key, _)| CheckMenuItem::with_id(*id, lang.t(key), true, false, None)).collect();
        let refs: Vec<&dyn tray_icon::menu::IsMenuItem> =
            quiet_items.iter().map(|item| item as &dyn tray_icon::menu::IsMenuItem).collect();
        let quiet = Submenu::with_items(quiet, true, &refs)?;
        let toggle = MenuItem::with_id("toggle", toggle, true, None);
        let settings = MenuItem::with_id("settings", settings, true, None);
        let quit = MenuItem::with_id("quit", quit, true, None);
        let menu = Menu::with_items(&[&pause, &quiet, &toggle, &settings, &PredefinedMenuItem::separator(), &quit])?;
        let mut builder =
            TrayIconBuilder::new().with_tooltip("TinyWorld").with_menu(Box::new(menu)).with_menu_on_left_click(true);
        if let Some(icon) = icon() {
            builder = builder.with_icon(icon);
        }
        let icon = builder.build()?;
        Ok(Self { icon, pause, quiet, quiet_items, toggle, settings, quit })
    }

    fn set_quiet(&self, choice: Option<usize>) {
        for (i, item) in self.quiet_items.iter().enumerate() {
            item.set_checked(choice == Some(i));
        }
    }

    fn retitle(&self, lang: Lang) {
        let [pause, quiet, toggle, settings, quit] = labels(lang);
        self.pause.set_text(pause);
        self.quiet.set_text(quiet);
        for (item, (_, key, _)) in self.quiet_items.iter().zip(QUIET) {
            item.set_text(lang.t(key));
        }
        self.toggle.set_text(toggle);
        self.settings.set_text(settings);
        self.quit.set_text(quit);
    }
}
