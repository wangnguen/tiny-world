//! Nhãn nhiệt độ nhỏ cạnh pet (`temperatureTag.ts`): hình thời tiết và số độ, ví dụ "☀ 27°C"; nóng thì chữ cam, lạnh
//! thì chữ xanh. Đứng ngang đầu, sau lưng pet (khói thở bay về phía trước không bị che; sát mép màn hình thì sang bên
//! kia). Vẽ như `.temperature` trong overlay.css.

use crate::gfx::{scale, Color, Gfx, Layer};
use crate::sim::terrain::Bounds;
use crate::sim::weather::Warmth;
use crate::sim::{js_round, Rect, Sky};
use anyhow::Result;
use windows::Win32::Graphics::Direct2D::D2D1_ROUNDED_RECT;
use windows::Win32::Graphics::DirectWrite::IDWriteTextLayout;

/// Cách pet một chút (CSS pixel).
const GAP: f64 = 4.0;
const FONT_SIZE: f32 = 11.0;
const LINE: f32 = FONT_SIZE * 1.35;
const ICON: f32 = 13.0;
const ICON_GAP: f32 = 3.0;
const PAD_LEFT: f32 = 4.0;
const PAD_RIGHT: f32 = 7.0;
const PAD_Y: f32 = 1.0;
const BORDER: f32 = 2.0;
const RADIUS: f32 = 9.0;
const INK: Color = Color::hex(0x2a2340, 1.0);
const PAPER: Color = Color::hex(0xfffaf0, 1.0);

/// Hình thời tiết nhỏ (SVG 16×16), giống `skyIcons.ts` của Cài đặt. Chuỗi cố định, không có gì người dùng nhập.
fn icon_svg(sky: Sky) -> String {
    const CLOUD: &str = r##"<path d="M4.5 10h7.2a2.6 2.6 0 0 0 .3-5.2 3.6 3.6 0 0 0-6.9-.5A2.9 2.9 0 0 0 4.5 10z" fill="#e8eef9" stroke="#6b7fa8" stroke-width="1.1"/>"##;
    let body = match sky {
        Sky::Sunny => r##"<circle cx="8" cy="8" r="3.2" fill="#ffb703" stroke="#e07b00"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" stroke="#e07b00" stroke-width="1.4" stroke-linecap="round"/>"##.to_owned(),
        Sky::Clear => r##"<path d="M10.5 2.5a5.5 5.5 0 1 0 3 9.6 4.6 4.6 0 0 1-3-9.6z" fill="#ffd36b" stroke="#c98a00"/>"##.to_owned(),
        Sky::Cloudy => r##"<path d="M4.5 12.5h7.2a2.8 2.8 0 0 0 .3-5.6 3.8 3.8 0 0 0-7.3-.6A3.1 3.1 0 0 0 4.5 12.5z" fill="#e8eef9" stroke="#6b7fa8" stroke-width="1.1"/>"##.to_owned(),
        Sky::Rain => format!(r##"{CLOUD}<path d="M5.5 12l-.7 2M8.5 12l-.7 2M11.5 12l-.7 2" stroke="#3b82f6" stroke-width="1.3" stroke-linecap="round"/>"##),
        Sky::Storm => format!(r##"{CLOUD}<path d="M8.8 10.2 6.8 13h2l-1 2.6 3-3.8h-2l.9-1.6z" fill="#ffcc33" stroke="#c98a00" stroke-width=".6"/>"##),
        Sky::Snow => r##"<path d="M8 2v12M2.8 5l10.4 6M2.8 11l10.4-6" stroke="#4a90e2" stroke-width="1.4" stroke-linecap="round"/>"##.to_owned(),
        Sky::Fog => r##"<path d="M2.5 5h11M1.5 8h13M3.5 11h9" stroke="#7d8fb3" stroke-width="1.6" stroke-linecap="round"/>"##.to_owned(),
        Sky::Petals => r##"<g fill="#ff9ec2" stroke="#d9668f" stroke-width=".7"><circle cx="8" cy="4.6" r="2.3"/><circle cx="11.3" cy="7" r="2.3"/><circle cx="10" cy="10.9" r="2.3"/><circle cx="6" cy="10.9" r="2.3"/><circle cx="4.7" cy="7" r="2.3"/></g><circle cx="8" cy="8" r="1.6" fill="#ffd36b"/>"##.to_owned(),
    };
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">{body}</svg>"#)
}

pub struct TemperatureTag {
    layer: Layer,
    shown: bool,
    /// Nội dung đang hiện: hình, số độ (đã làm tròn), nóng lạnh; `None` là không có nhiệt độ (ẩn).
    content: Option<(Option<Sky>, i64, Option<Warmth>)>,
    layout: Option<IDWriteTextLayout>,
    text_width: f32,
    /// Cỡ khung (CSS pixel).
    width: f64,
    height: f64,
    drawn: Option<f64>,
}

impl TemperatureTag {
    pub fn new(mut layer: Layer) -> Result<Self> {
        layer.set_visible(false)?;
        Ok(Self { layer, shown: false, content: None, layout: None, text_width: 0.0, width: 0.0, height: 0.0, drawn: None })
    }

    /// Đổi nội dung; `temperature` `None` là ẩn nhãn. Trả về `true` nếu có gì đổi (phải vẽ lại).
    pub fn set(&mut self, sky: Option<Sky>, temperature: Option<f64>, warmth: Option<Warmth>) -> bool {
        let content = temperature.map(|t| (sky, js_round(t) as i64, warmth));
        if content == self.content {
            return false;
        }
        self.content = content;
        self.layout = None;
        self.drawn = None;
        true
    }

    fn measure(&mut self, gfx: &Gfx, px: f64) -> Result<()> {
        let Some((sky, degrees, _)) = self.content else {
            return Ok(());
        };
        let k = px as f32;
        let format = gfx.text_format(FONT_SIZE * k, 700, LINE * k)?;
        let (layout, width, _) = gfx.layout(&format!("{degrees}°C"), &format, f32::MAX)?;
        self.text_width = width / k;
        self.layout = Some(layout);
        let icon = if sky.is_some() { ICON + ICON_GAP } else { 0.0 };
        self.width = f64::from((PAD_LEFT + icon + self.text_width + PAD_RIGHT + 2.0 * BORDER).ceil());
        self.height = f64::from((LINE.max(ICON) + 2.0 * PAD_Y + 2.0 * BORDER).round());
        Ok(())
    }

    /// Đặt nhãn cạnh thân pet `body` (`PetView::body`), ngang đầu, sau lưng (pet nhìn về `facing`: 1 sang phải), trong
    /// `bounds`; `body` `None` (không có pet nào đang hiện) hoặc chưa có nhiệt độ thì ẩn.
    pub fn place(&mut self, gfx: &Gfx, body: Option<Rect>, facing: i8, bounds: &Bounds, px: f64) -> Result<()> {
        let show = body.is_some() && self.content.is_some();
        if show != self.shown {
            self.shown = show;
            self.layer.set_visible(show)?;
        }
        let (Some(body), Some((sky, _, warmth))) = (body, self.content) else {
            return Ok(());
        };
        if self.layout.is_none() {
            self.measure(gfx, px)?;
        }
        let (width, height) = (self.width, self.height);
        // Sau lưng; không đủ chỗ (sát mép màn hình) thì sang trước mặt.
        let behind = if facing > 0 { body.x - GAP - width } else { body.x + body.width + GAP };
        let ahead = if facing > 0 { body.x + body.width + GAP } else { body.x - GAP - width };
        let fits = |x: f64| x >= bounds.left + GAP && x + width <= bounds.right - GAP;
        let x = if fits(behind) || !fits(ahead) { behind } else { ahead };
        let y = (bounds.top + GAP).max(body.y + body.height * 0.2 - height / 2.0);
        let resized = self.layer.resize(gfx, (width * px).ceil() as u32, ((height + 2.0) * px).ceil() as u32)?;
        if resized || self.drawn != Some(px) {
            self.drawn = Some(px);
            self.draw(sky, warmth, px)?;
        }
        self.layer.move_to((js_round(x) * px).round() as f32, (js_round(y) * px).round() as f32)
    }

    fn draw(&self, sky: Option<Sky>, warmth: Option<Warmth>, px: f64) -> Result<()> {
        let Some(layout) = &self.layout else {
            return Ok(());
        };
        let (w, h, k) = (self.width as f32, self.height as f32, px as f32);
        let color = match warmth {
            Some(Warmth::Hot) => Color::hex(0xd9480f, 1.0),
            Some(Warmth::Cold) => Color::hex(0x1c64d8, 1.0),
            None => INK,
        };
        self.layer.draw(|canvas| {
            canvas.transform(&scale(k, k));
            let rounded = |x: f32, y: f32, w: f32, h: f32, r: f32| D2D1_ROUNDED_RECT {
                rect: crate::gfx::rect(x, y, w, h),
                radiusX: r,
                radiusY: r,
            };
            let ink = canvas.brush(INK)?;
            let paper = canvas.brush(PAPER)?;
            let shadow = canvas.brush(Color::rgba(42, 35, 64, 0.35))?;
            unsafe {
                canvas.dc.FillRoundedRectangle(&rounded(0.0, 2.0, w, h, RADIUS), &shadow);
                canvas.dc.FillRoundedRectangle(&rounded(0.0, 0.0, w, h, RADIUS), &ink);
                canvas.dc.FillRoundedRectangle(
                    &rounded(BORDER, BORDER, w - 2.0 * BORDER, h - 2.0 * BORDER, RADIUS - BORDER),
                    &paper,
                );
            }
            let mut x = BORDER + PAD_LEFT;
            if let Some(sky) = sky {
                // Hình vẽ theo pixel màn hình cho nét: bỏ phép phóng to, tính chỗ bằng tay.
                canvas.reset();
                let top = (h - ICON) / 2.0;
                canvas.svg(&icon_svg(sky), x * k, top * k, ICON * k)?;
                x += ICON + ICON_GAP;
            }
            canvas.reset();
            let brush = canvas.brush(color)?;
            canvas.text(layout, x * k, ((h - LINE) / 2.0) * k, &brush);
            Ok(())
        })
    }
}
