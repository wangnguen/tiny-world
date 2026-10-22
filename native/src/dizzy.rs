//! Choáng: lảo đảo quanh điểm chân và sao bay vòng quanh đầu (`dizzy.ts`).

use crate::gfx::{Color, Gfx, Layer};
use crate::sim::tuning as t;
use anyhow::Result;
use std::f64::consts::PI;

/// Lảo đảo: đỉnh đầu lệch sang mỗi bên tối đa bấy nhiêu phần khoảng cách từ chân tới đỉnh đầu.
const SWAY_LEAN: f64 = 0.12;
/// Số lần lảo đảo qua lại mỗi giây.
const SWAY_HZ: f64 = 1.4;
/// Lảo đảo mạnh nhất trong phần đầu, từ đây tới lúc hết choáng thì dịu dần về đứng thẳng.
const SWAY_STEADY: f64 = 0.4;
const STAR_COUNT: usize = 3;
/// Số vòng sao bay quanh đầu mỗi giây.
const STAR_TURNS: f64 = 1.2;

/// Đỉnh đầu lệch bao nhiêu pixel của frame lúc choáng (dương: sang phải màn hình). `height`: từ chân tới đỉnh đầu,
/// pixel của frame. Làm tròn theo pixel để pixel art nghiêng thành bậc gọn chứ không lăn tăn.
pub fn dizzy_lean(time: f64, height: f64) -> f64 {
    let remaining = (1.0 - time / t::DIZZY_TIME).max(0.0);
    let strength = (remaining / (1.0 - SWAY_STEADY)).min(1.0);
    crate::sim::js_round(height * SWAY_LEAN * strength * (2.0 * PI * SWAY_HZ * time).sin())
}

/// `dizzy_lean` lệch tối đa bấy nhiêu pixel với frame cao `frame_height`: số cột chừa sẵn mỗi bên canvas.
pub fn dizzy_reach(frame_height: u32) -> f64 {
    (f64::from(frame_height) * SWAY_LEAN).ceil()
}

/// Hàng `y` của frame bị đẩy ngang bao nhiêu pixel của frame lúc lảo đảo: chân (`foot_y`) đứng yên, từ đỉnh đầu trở
/// lên lệch đúng `lean`, ở giữa lệch dần. Làm tròn theo pixel màn hình (`density` pixel màn hình cho một pixel của
/// frame): dời nửa pixel thì hàng đó lấy mẫu lại lệch pha với hàng bên cạnh, vân và viền nhấp nháy như nhiễu.
pub fn lean_shift(lean: f64, y: f64, foot_y: f64, head_top: f64, density: f64) -> f64 {
    if lean == 0.0 {
        return 0.0;
    }
    let t = (foot_y - y) / (foot_y - head_top).max(1.0);
    crate::sim::js_round(lean * density * t.clamp(0.0, 1.0)) / density
}

// Sao vẽ theo từng pixel: o viền, y vàng, w sáng, d vàng tối (sao phía sau, xa hơn).
fn star_color(c: u8) -> Option<Color> {
    Some(match c {
        b'o' => Color::hex(0x1b1622, 1.0),
        b'y' => Color::hex(0xffd84d, 1.0),
        b'w' => Color::hex(0xfff7d6, 1.0),
        b'd' => Color::hex(0xd4a53a, 1.0),
        _ => return None,
    })
}
const NEAR: [&str; 7] = ["...o...", "..oyo..", "oooyooo", "oyywyyo", ".oyyyo.", ".oyoyo.", ".oo.oo."];
const FAR: [&str; 5] = ["..o..", ".odo.", "odddo", ".odo.", "..o.."];
const PAD: i32 = NEAR.len() as i32 / 2;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Star {
    /// Lệch khỏi tâm vòng, pixel của frame.
    x: i32,
    y: i32,
    /// Đang ở nửa vòng phía trước (phía dưới trên màn hình, gần người xem).
    near: bool,
}

fn orbit(time: f64, rx: i32, ry: i32) -> Vec<Star> {
    (0..STAR_COUNT)
        .map(|i| {
            let angle = 2.0 * PI * (STAR_TURNS * time + i as f64 / STAR_COUNT as f64);
            let depth = angle.sin();
            Star {
                x: crate::sim::js_round(f64::from(rx) * angle.cos()) as i32,
                y: crate::sim::js_round(f64::from(ry) * depth) as i32,
                near: depth >= 0.0,
            }
        })
        .collect()
}

/// Sao bay vòng quanh đầu lúc choáng. Nửa vòng phía sau vẽ trên lớp nằm dưới pet (bị đầu che, sao nhỏ và tối hơn),
/// nửa phía trước trên lớp nằm trên pet. Vẽ theo pixel của frame, phóng to kiểu pixel art như chính pet.
pub struct DizzyStars {
    back: Layer,
    front: Layer,
    /// Bán kính vòng sao theo chiều ngang / dọc, pixel của frame.
    rx: i32,
    ry: i32,
    /// Số CSS pixel cho một pixel của sao.
    scale: f64,
    drawn: Vec<Star>,
    shown: bool,
}

impl DizzyStars {
    /// `back`, `front`: hai lớp ngay dưới và ngay trên sprite của pet.
    pub fn new(back: Layer, front: Layer) -> Result<Self> {
        let mut stars = Self { back, front, rx: 6, ry: 2, scale: 1.0, drawn: Vec::new(), shown: true };
        stars.hide()?;
        Ok(stars)
    }

    /// Vòng sao rộng theo frame của pack, mỗi pixel của sao to `scale` CSS pixel.
    pub fn configure(&mut self, frame_width: u32, scale: f64) {
        self.rx = (crate::sim::js_round(f64::from(frame_width) / 4.0) as i32).max(6);
        self.ry = (crate::sim::js_round(f64::from(self.rx) / 4.0) as i32).max(2);
        self.scale = scale;
        self.drawn.clear();
    }

    pub fn hide(&mut self) -> Result<()> {
        if !self.shown {
            return Ok(());
        }
        self.shown = false;
        self.back.set_visible(false)?;
        self.front.set_visible(false)?;
        self.drawn.clear();
        Ok(())
    }

    /// `(x, y)`: giữa đỉnh đầu, CSS pixel của overlay; `time`: số giây đã choáng; `px`: pixel màn hình cho một CSS
    /// pixel. Vòng sao nằm ngay trên đỉnh đầu: sao bay qua phía trước chỉ chạm mép trên của đầu, không che mắt.
    pub fn update(&mut self, gfx: &Gfx, time: f64, x: f64, y: f64, px: f64) -> Result<()> {
        let y = y - f64::from(self.ry) * self.scale;
        if !self.shown {
            self.shown = true;
            self.back.set_visible(true)?;
            self.front.set_visible(true)?;
        }
        let cell = (self.scale * px).max(1.0);
        let width = ((2 * self.rx + 2 * PAD + 1) as f64 * cell).round() as u32;
        let height = ((2 * self.ry + 2 * PAD + 1) as f64 * cell).round() as u32;
        let resized = self.back.resize(gfx, width, height)? | self.front.resize(gfx, width, height)?;
        let stars = orbit(time, self.rx, self.ry);
        if resized || stars != self.drawn {
            let (cx, cy) = (self.rx + PAD, self.ry + PAD);
            for (layer, near) in [(&self.back, false), (&self.front, true)] {
                layer.draw(|canvas| {
                    let brushes = (*b"oywd")
                        .map(|c| star_color(c).map(|color| canvas.brush(color)).transpose());
                    for star in stars.iter().filter(|s| s.near == near) {
                        let art: &[&str] = if star.near { &NEAR } else { &FAR };
                        let half = art.len() as i32 / 2;
                        for (dy, row) in art.iter().enumerate() {
                            for (dx, c) in row.bytes().enumerate() {
                                let Some(slot) = b"oywd".iter().position(|&k| k == c) else { continue };
                                let Ok(Some(brush)) = &brushes[slot] else { continue };
                                let px_x = cx + star.x - half + dx as i32;
                                let px_y = cy + star.y - half + dy as i32;
                                canvas.fill_rect(
                                    (f64::from(px_x) * cell) as f32,
                                    (f64::from(px_y) * cell) as f32,
                                    cell as f32,
                                    cell as f32,
                                    brush,
                                );
                            }
                        }
                    }
                    Ok(())
                })?;
            }
            self.drawn = stars;
        }
        let left = ((x - f64::from(self.rx + PAD) * self.scale) * px).round() as f32;
        let top = ((y - f64::from(self.ry + PAD) * self.scale) * px).round() as f32;
        self.back.move_to(left, top)?;
        self.front.move_to(left, top)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lao_dao_diu_dan_ve_dung_thang() {
        assert_eq!(dizzy_lean(0.0, 100.0), 0.0);
        let peak = (0..100).map(|i| dizzy_lean(f64::from(i) / 100.0, 100.0).abs()).fold(0.0, f64::max);
        assert_eq!(peak, 12.0);
        assert_eq!(dizzy_lean(t::DIZZY_TIME, 100.0), 0.0);
        assert_eq!(dizzy_reach(192), 24.0);
    }

    #[test]
    fn chan_dung_yen_dau_lech_dung_lean() {
        assert_eq!(lean_shift(6.0, 180.0, 180.0, 40.0, 1.0), 0.0);
        assert_eq!(lean_shift(6.0, 40.0, 180.0, 40.0, 1.0), 6.0);
        assert_eq!(lean_shift(6.0, 10.0, 180.0, 40.0, 1.0), 6.0);
        assert_eq!(lean_shift(6.0, 110.0, 180.0, 40.0, 0.5), 4.0);
    }
}
