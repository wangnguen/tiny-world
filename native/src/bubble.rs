//! Speech bubble trên đầu pet, vẽ như `.bubble` trong overlay.css: chữ Segoe UI 600 13px, dòng cao 1.35, khung kem
//! viền tím đậm bo góc, bóng đổ xuống 2px, đuôi chỉ vào giữa đầu. Câu nhiều dòng (bấm đúp: giờ, ngày, thời tiết) xuống
//! dòng đúng chỗ; câu dài tự xuống dòng ở bề ngang 200px rồi co khung lại vừa dòng dài nhất.

use crate::gfx::{rotation, Color, Gfx, Layer};
use anyhow::Result;
use windows::Win32::Graphics::Direct2D::D2D1_ROUNDED_RECT;
use windows::Win32::Graphics::DirectWrite::IDWriteTextLayout;

const FONT_SIZE: f32 = 13.0;
const LINE: f32 = FONT_SIZE * 1.35;
const MAX_WIDTH: f32 = 200.0;
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 5.0;
const BORDER: f32 = 2.0;
const RADIUS: f32 = 10.0;
const SHADOW: f32 = 2.0;
/// Đuôi: hình vuông 10px (cả viền 12px) xoay 45 độ, tâm ở dưới mép trong của khung.
const TAIL: f32 = 12.0;
/// Phần dưới khung còn chừa cho đuôi và bóng (CSS pixel).
const BELOW: f32 = 10.0;
const INK: Color = Color::hex(0x2a2340, 1.0);
const PAPER: Color = Color::hex(0xfffaf0, 1.0);
const FADE_SECONDS: f32 = 0.15;

pub struct Bubble {
    layer: Layer,
    text: String,
    layout: Option<IDWriteTextLayout>,
    /// Cỡ khung cả viền (CSS pixel), không tính đuôi và bóng.
    pub width: f64,
    pub height: f64,
    /// Lần vẽ trước: chỗ đuôi và pixel màn hình cho một CSS pixel.
    drawn: Option<(i64, f64)>,
}

/// Như `offsetWidth/offsetHeight` của trình duyệt: cỡ khung làm tròn tới pixel.
fn outer(width: f32, height: f32) -> (f64, f64) {
    (f64::from(width.ceil() + 2.0 * (PAD_X + BORDER)), f64::from((height + 2.0 * (PAD_Y + BORDER)).round()))
}

impl Bubble {
    pub fn new(mut layer: Layer) -> Result<Self> {
        layer.set_visible(false)?;
        Ok(Self { layer, text: String::new(), layout: None, width: 0.0, height: 0.0, drawn: None })
    }

    /// Đổi câu; rỗng là cất bubble. Câu mới thì đo lại cỡ và hiện dần lên.
    pub fn set_text(&mut self, gfx: &Gfx, text: &str, px: f64) -> Result<()> {
        if text == self.text {
            return Ok(());
        }
        self.text = text.to_owned();
        self.drawn = None;
        if text.is_empty() {
            self.layout = None;
            return self.layer.set_visible(false);
        }
        let px = px as f32;
        let format = gfx.text_format(FONT_SIZE * px, 600, LINE * px)?;
        let (layout, width, height) = gfx.layout(&pre_line(text), &format, MAX_WIDTH * px)?;
        // Co khung lại vừa dòng dài nhất (câu dài bị xuống dòng thì khung vẫn rộng bằng max-width).
        unsafe { layout.SetMaxWidth((width / px).ceil() * px + 0.5)? };
        (self.width, self.height) = outer(width / px, height / px);
        self.layout = Some(layout);
        self.layer.set_visible(true)?;
        gfx.fade_in(&self.layer, FADE_SECONDS)?;
        Ok(())
    }

    /// Đặt khung ở (`x`, `y`) (góc trên trái, CSS pixel của overlay), đuôi ở `tail` CSS pixel tính từ mép trong bên
    /// trái của khung.
    pub fn place(&mut self, gfx: &Gfx, x: f64, y: f64, tail: f64, px: f64) -> Result<()> {
        let Some(layout) = &self.layout else {
            return Ok(());
        };
        let width = (self.width * px).ceil() as u32;
        let height = ((self.height + f64::from(BELOW)) * px).ceil() as u32;
        let resized = self.layer.resize(gfx, width, height)?;
        let key = ((tail * 4.0).round() as i64, px);
        if resized || self.drawn != Some(key) {
            self.drawn = Some(key);
            let (w, h, k) = (self.width as f32, self.height as f32, px as f32);
            let tail = tail as f32;
            let layout = layout.clone();
            self.layer.draw(|canvas| {
                let scale = crate::gfx::scale(k, k);
                canvas.transform(&scale);
                let ink = canvas.brush(INK)?;
                let paper = canvas.brush(PAPER)?;
                let shadow = canvas.brush(Color::rgba(42, 35, 64, 0.35))?;
                let rounded = |x: f32, y: f32, w: f32, h: f32, r: f32| D2D1_ROUNDED_RECT {
                    rect: crate::gfx::rect(x, y, w, h),
                    radiusX: r,
                    radiusY: r,
                };
                unsafe {
                    canvas.dc.FillRoundedRectangle(&rounded(0.0, SHADOW, w, h, RADIUS), &shadow);
                    canvas.dc.FillRoundedRectangle(&rounded(0.0, 0.0, w, h, RADIUS), &ink);
                    let inner = RADIUS - BORDER;
                    canvas.dc.FillRoundedRectangle(
                        &rounded(BORDER, BORDER, w - 2.0 * BORDER, h - 2.0 * BORDER, inner),
                        &paper,
                    );
                }
                // Đuôi: vuông nền kem có viền phải và dưới, xoay 45 độ quanh tâm.
                let (cx, cy) = (BORDER + tail, h - BORDER + 1.0);
                canvas.transform(&(rotation(std::f32::consts::FRAC_PI_4, cx, cy) * scale));
                let left = cx - TAIL / 2.0;
                let top = cy - TAIL / 2.0;
                canvas.fill_rect(left, top, TAIL, TAIL, &paper);
                canvas.fill_rect(left + TAIL - BORDER, top, BORDER, TAIL, &ink);
                canvas.fill_rect(left, top + TAIL - BORDER, TAIL, BORDER, &ink);
                canvas.reset();
                canvas.text(&layout, (BORDER + PAD_X) * k, (BORDER + PAD_Y) * k, &ink);
                Ok(())
            })?;
        }
        self.layer.move_to((x * px).round() as f32, (y * px).round() as f32)
    }
}

/// Như `white-space: pre-line`: giữ xuống dòng, gộp khoảng trắng liền nhau, bỏ khoảng trắng đầu và cuối dòng.
fn pre_line(text: &str) -> String {
    text.lines().map(|line| line.split_whitespace().collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn giu_xuong_dong_gop_khoang_trang() {
        assert_eq!(pre_line("  15:04 ·  Thứ Năm\nÂm lịch 21/8 "), "15:04 · Thứ Năm\nÂm lịch 21/8");
    }

    #[test]
    fn co_khung_nhu_trinh_duyet() {
        assert_eq!(outer(57.3, 17.55), (82.0, 32.0));
    }
}
