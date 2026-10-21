//! Con ma pixel art (sự kiện hiếm lúc 2 giờ sáng, `ghost.ts`): bay ngang qua màn hình, lượn lên xuống khắp vùng làm
//! việc, sà xuống ngang tầm từng pet (để còn dọa) rồi biến mất ở mép bên kia.

use crate::gfx::{rotation, scale, Color, Gfx, Layer};
use crate::random::random;
use crate::sim::terrain::Bounds;
use crate::sim::Point;
use anyhow::Result;
use std::f64::consts::PI;
use windows_numerics::Matrix3x2;

const SPRITE: [&str; 13] = [
    "....KKKKKK....",
    "...KWWWWWWK...",
    "..KWWWWWWWWK..",
    ".KWWWWWWWWWWK.",
    ".KWWKKWWKKWWK.",
    ".KWWKKWWKKWWK.",
    ".KWWWWWWWWWWK.",
    ".KWWWWBBWWWWK.",
    ".KWWWWWWWWWWK.",
    ".KWWWWWWWWWWK.",
    ".KWWWWWWWWWWK.",
    ".KWKWWKWWKWWK.",
    "..K.KK.KK.KK..",
];
/// Mỗi ô của con ma to chừng này CSS pixel.
const CELL: f64 = 3.0;
pub const WIDTH: f64 = 14.0 * CELL;
pub const HEIGHT: f64 = 13.0 * CELL;
/// Bay hết bề ngang vùng làm việc trong chừng này giây.
const CROSS_SECONDS: f64 = 14.0;
/// Lượn lên xuống chừng này lượt trong một lần bay ngang, khắp chiều cao vùng làm việc.
const WAVES: f64 = 1.6;
/// Nhấp nhô thêm chừng này px, chừng này lượt mỗi lần bay ngang.
const BOB: f64 = 10.0;
const BOBS: f64 = 9.0;
/// Sà xuống ngang tầm từng pet trong khoảng chừng này lần bề ngang vùng làm việc quanh con đó.
const SWOOP: f64 = 0.1;
/// Cách mép trên, mép dưới vùng làm việc ít nhất chừng này px.
const MARGIN: f64 = 16.0;
/// Nghiêng theo hướng bay, tối đa chừng này radian.
const MAX_TILT: f64 = 0.45;
/// Hơi trong (`opacity: 0.85` của bản web).
const OPACITY: f32 = 0.85;

/// Đường bay của một lượt: từ mép này sang mép kia, lượn khắp chiều cao, sà xuống ngang tầm từng pet.
#[derive(Debug, Clone, Copy)]
pub struct Flight {
    pub bounds: Bounds,
    /// -1: bay từ phải sang trái, 1: từ trái sang phải.
    pub dir: f64,
    /// Pha lượn lên xuống (0–2π): mỗi lượt bắt đầu ở một độ cao khác.
    pub phase: f64,
}

/// Giữa thân con ma và độ nghiêng lúc đã bay được phần `t` (0–1) của đường bay; `targets`: chỗ sà qua (giữa thân
/// con ma), mỗi pet một chỗ.
pub fn ghost_at(flight: &Flight, targets: &[Point], t: f64) -> (f64, f64, f64) {
    let at = |u: f64| {
        let Flight { bounds, dir, phase } = *flight;
        let from = if dir > 0.0 { bounds.left - WIDTH / 2.0 } else { bounds.right + WIDTH / 2.0 };
        let to = if dir > 0.0 { bounds.right + WIDTH / 2.0 } else { bounds.left - WIDTH / 2.0 };
        let x = from + (to - from) * u;
        let high = bounds.top + MARGIN + HEIGHT / 2.0;
        let low = bounds.floor - MARGIN - HEIGHT / 2.0;
        let mut y = high + (low - high) * (0.5 + 0.5 * (2.0 * PI * WAVES * u + phase).sin());
        y += (2.0 * PI * BOBS * u).sin() * BOB;
        // Gần pet nào thì sà xuống ngang tầm con đó; hai con gần nhau thì lấy chỗ giữa, không bị giật.
        let radius = (SWOOP * (bounds.right - bounds.left)).max(1.0);
        let (mut pull, mut sum, mut weighted) = (0.0f64, 0.0, 0.0);
        for target in targets {
            let w = (-((x - target.x) / radius).powi(2)).exp();
            pull = pull.max(w);
            sum += w;
            weighted += w * target.y;
        }
        if sum > 0.0 {
            y += (weighted / sum - y) * pull;
        }
        (x, y.max(high).min(low))
    };
    let here = at(t);
    let ahead = at(t + 0.004);
    let dx = (ahead.0 - here.0).abs();
    let tilt = if dx > 0.0 { flight.dir * ((ahead.1 - here.1) / dx).atan() * 0.6 } else { 0.0 };
    (here.0, here.1, tilt.clamp(-MAX_TILT, MAX_TILT))
}

pub struct Ghost {
    layer: Layer,
    start: f64,
    flight: Option<Flight>,
    drawn: Option<f64>,
}

impl Ghost {
    pub fn new(mut layer: Layer) -> Result<Self> {
        layer.set_visible(false)?;
        layer.set_opacity(OPACITY)?;
        Ok(Self { layer, start: 0.0, flight: None, drawn: None })
    }

    pub fn flying(&self) -> bool {
        self.flight.is_some()
    }

    /// Bắt đầu bay ngang `bounds`.
    pub fn fly(&mut self, bounds: Bounds, now: f64) -> Result<()> {
        self.flight = Some(Flight { bounds, dir: if random() < 0.5 { 1.0 } else { -1.0 }, phase: random() * PI * 2.0 });
        self.start = now;
        self.layer.set_visible(true)
    }

    /// Bay tiếp, sà qua từng chỗ trong `targets` (giữa thân con ma, mỗi pet một chỗ). Trả về giữa thân con ma (CSS
    /// pixel của overlay), `None` nếu không bay.
    pub fn update(&mut self, gfx: &Gfx, now: f64, targets: &[Point], px: f64) -> Result<Option<Point>> {
        let Some(flight) = self.flight else {
            return Ok(None);
        };
        let t = (now - self.start) / 1000.0 / CROSS_SECONDS;
        if t >= 1.0 {
            self.flight = None;
            self.layer.set_visible(false)?;
            return Ok(None);
        }
        let (w, h) = ((WIDTH * px).round() as u32, (HEIGHT * px).round() as u32);
        if self.layer.resize(gfx, w, h)? || self.drawn != Some(px) {
            self.drawn = Some(px);
            let cell = (CELL * px) as f32;
            self.layer.draw(|canvas| {
                for (r, row) in SPRITE.iter().enumerate() {
                    for (c, ch) in row.bytes().enumerate() {
                        let color = match ch {
                            b'K' => 0x2a2340,
                            b'W' => 0xeef2ff,
                            b'B' => 0xff9ec2,
                            _ => continue,
                        };
                        let brush = canvas.brush(Color::hex(color, 1.0))?;
                        canvas.fill_rect(c as f32 * cell, r as f32 * cell, cell, cell, &brush);
                    }
                }
                Ok(())
            })?;
        }
        let (x, y, tilt) = ghost_at(&flight, targets, t);
        let left = (x - WIDTH / 2.0).round();
        let top = (y - HEIGHT / 2.0).round();
        self.layer.move_to((left * px).round() as f32, (top * px).round() as f32)?;
        // Như `rotate(tilt) scaleX(-dir)` của CSS quanh tâm con ma.
        let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
        let transform = Matrix3x2::translation(-cx, -cy)
            * scale(-flight.dir as f32, 1.0)
            * rotation(tilt as f32, 0.0, 0.0)
            * Matrix3x2::translation(cx, cy);
        self.layer.set_transform(&transform)?;
        Ok(Some(Point { x, y }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Bounds = Bounds { left: 0.0, right: 1920.0, top: 0.0, floor: 1040.0 };

    fn samples(flight: &Flight, targets: &[Point], steps: usize) -> Vec<(f64, f64, f64)> {
        (0..=steps).map(|i| ghost_at(flight, targets, i as f64 / steps as f64)).collect()
    }

    fn phases() -> impl Iterator<Item = f64> {
        (0..9).map(|i| f64::from(i) * 0.7)
    }

    #[test]
    fn bay_tu_ngoai_mep_nay_sang_ngoai_mep_kia() {
        for dir in [1.0, -1.0] {
            let flight = Flight { bounds: BOUNDS, dir, phase: 1.0 };
            let (first, last) = (ghost_at(&flight, &[], 0.0), ghost_at(&flight, &[], 1.0));
            let outside = |x: f64| x + WIDTH / 2.0 <= BOUNDS.left || x - WIDTH / 2.0 >= BOUNDS.right;
            assert!(outside(first.0) && outside(last.0));
            assert_eq!((last.0 - first.0).signum(), dir);
        }
    }

    #[test]
    fn luon_khap_chieu_cao_ma_khong_ra_ngoai_mep_tren_mep_duoi() {
        let half = HEIGHT / 2.0;
        for phase in phases() {
            let ys: Vec<f64> =
                samples(&Flight { bounds: BOUNDS, dir: 1.0, phase }, &[], 2000).iter().map(|p| p.1).collect();
            let min = ys.iter().copied().fold(f64::INFINITY, f64::min);
            let max = ys.iter().copied().fold(0.0, f64::max);
            assert!(min - half >= BOUNDS.top, "phase {phase}: thò lên trên");
            assert!(max + half <= BOUNDS.floor, "phase {phase}: thò xuống dưới");
            assert!(min < BOUNDS.floor * 0.25, "phase {phase}: không lên cao");
            assert!(max > BOUNDS.floor * 0.75, "phase {phase}: không xuống thấp");
        }
    }

    #[test]
    fn sa_qua_dung_cho_tung_pet_ke_ca_pet_dung_tren_cua_so() {
        let targets = [Point { x: 300.0, y: 980.0 }, Point { x: 1500.0, y: 420.0 }];
        for phase in phases() {
            for dir in [1.0, -1.0] {
                let path = samples(&Flight { bounds: BOUNDS, dir, phase }, &targets, 2000);
                for target in &targets {
                    let near = (path.iter().copied())
                        .reduce(|a, b| if (b.0 - target.x).abs() < (a.0 - target.x).abs() { b } else { a })
                        .unwrap();
                    let off = near.1 - target.y;
                    assert!(off.abs() < 12.0, "phase {phase} dir {dir}: cách pet {off}px");
                }
            }
        }
    }

    #[test]
    fn bay_muot_khong_giat_nghieng_vua_phai() {
        let targets = [Point { x: 900.0, y: 980.0 }, Point { x: 960.0, y: 500.0 }];
        let path = samples(&Flight { bounds: BOUNDS, dir: 1.0, phase: 2.0 }, &targets, 14 * 30);
        for pair in path.windows(2) {
            assert!((pair[1].1 - pair[0].1).abs() < 40.0);
            assert!(pair[1].2.abs() <= 0.45 + 1e-9);
        }
    }
}
