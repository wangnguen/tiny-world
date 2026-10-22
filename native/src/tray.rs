//! Icon ở system tray. Overlay không có nút trên taskbar nên mọi điều khiển của app nằm ở đây: Tạm dừng, Im lặng,
//! Ẩn / hiện pet, Cài đặt…, Thoát. Bấm một mục thì `muda` gửi event; vòng lặp chính đọc ở `take_events`.

use anyhow::Result;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;
use tinyworld_backend::i18n::Lang;
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Các lựa chọn Im lặng: id mục trong menu, khoá chữ, thời gian (`None`: tới khi tắt).
pub const QUIET: [(&str, &str, Option<Duration>); 3] = [
    ("quiet-30", "tray.quiet30", Some(Duration::from_secs(30 * 60))),
    ("quiet-60", "tray.quiet60", Some(Duration::from_secs(60 * 60))),
    ("quiet-on", "tray.quietOn", None),
];

/// Một mục vừa được bấm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Pause,
    /// Chỉ số trong `QUIET`.
    Quiet(usize),
    Toggle,
    Settings,
    Quit,
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

pub struct Tray {
    _icon: TrayIcon,
    pause: CheckMenuItem,
    quiet: Submenu,
    quiet_items: Vec<CheckMenuItem>,
    toggle: MenuItem,
    settings: MenuItem,
    quit: MenuItem,
    events: Receiver<MenuEvent>,
}

/// Chữ của các mục: Tạm dừng, Im lặng, Ẩn / hiện pet, Cài đặt…, Thoát.
fn labels(lang: Lang) -> [String; 5] {
    ["tray.pause", "tray.quiet", "tray.toggle", "tray.settings", "tray.quit"].map(|key| lang.t(key))
}

impl Tray {
    /// Tạo icon khay; `wake` đánh thức vòng lặp chính mỗi lần có mục được bấm. `on_open` chạy lúc bấm icon, ngay trước
    /// khi menu hiện: vòng lặp chính bị menu chặn tới khi chọn xong.
    pub fn new(lang: Lang, wake: fn(), on_open: impl Fn() + Send + Sync + 'static) -> Result<Self> {
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
        let (tx, events) = mpsc::channel();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = tx.send(event);
            wake();
        }));
        TrayIconEvent::set_event_handler(Some(move |event| {
            if let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, .. } = event {
                if matches!(button, MouseButton::Left | MouseButton::Right) {
                    on_open();
                }
            }
        }));
        let mut builder =
            TrayIconBuilder::new().with_tooltip("TinyWorld").with_menu(Box::new(menu)).with_menu_on_left_click(true);
        if let Some(icon) = icon() {
            builder = builder.with_icon(icon);
        }
        let icon = builder.build()?;
        Ok(Self { _icon: icon, pause, quiet, quiet_items, toggle, settings, quit, events })
    }

    /// Các mục vừa được bấm từ lần gọi trước.
    pub fn take_events(&self) -> Vec<Action> {
        self.events
            .try_iter()
            .filter_map(|event| match event.id.as_ref() {
                "pause" => Some(Action::Pause),
                "toggle" => Some(Action::Toggle),
                "settings" => Some(Action::Settings),
                "quit" => Some(Action::Quit),
                id => QUIET.iter().position(|(quiet, ..)| *quiet == id).map(Action::Quiet),
            })
            .collect()
    }

    /// Menu tự đảo dấu tick khi bấm; đặt lại theo trạng thái thật.
    pub fn set_paused(&self, paused: bool) {
        self.pause.set_checked(paused);
    }

    pub fn set_quiet(&self, choice: Option<usize>) {
        for (i, item) in self.quiet_items.iter().enumerate() {
            item.set_checked(choice == Some(i));
        }
    }

    /// Đổi chữ trong menu theo ngôn ngữ `lang`.
    pub fn retitle(&self, lang: Lang) {
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
