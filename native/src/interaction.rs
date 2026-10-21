//! Click, kéo thả và ném pet bằng chuột trái (`interaction.ts`). Một lúc chỉ giữ một con: `Host::pick` trả về con nằm
//! trên cùng ở chỗ bấm (theo phần có hình của sprite), không có thì chuột không thuộc về pet nào. Pet gọi theo `uid`
//! (`Pet::uid`) để bỏ bớt nhân vật giữa lúc kéo cũng không nhầm con.

use crate::sim::{Point, Remap};
use std::collections::VecDeque;

/// Giữ chuột rồi di quá khoảng này (CSS pixel) thì tính là kéo, không phải click.
const DRAG_THRESHOLD: f64 = 4.0;
/// Vận tốc ném tính từ chuyển động chuột trong khoảng này trước lúc buông (ms).
const THROW_WINDOW: f64 = 100.0;
/// Hai lần click vào cùng một con cách nhau dưới chừng này (ms) là bấm đúp.
const DOUBLE_CLICK_MS: f64 = 400.0;

#[derive(Debug, Clone, Copy)]
struct Sample {
    x: f64,
    y: f64,
    /// ms, theo đồng hồ của message (`GetMessageTime`).
    t: f64,
}

#[derive(Debug, Clone)]
struct Press {
    pet: u32,
    start: Point,
    /// Khoảng cách từ con trỏ tới điểm chân pet, giữ nguyên trong lúc kéo.
    offset: Point,
    dragging: bool,
    samples: VecDeque<Sample>,
    /// Giữ Shift lúc bấm: dùng kỹ năng riêng của nhân vật (`Host::skill`).
    skill: bool,
}

/// Những gì tương tác cần ở overlay.
pub trait Host {
    /// Con nằm trên cùng có phần hình dưới `point`, `None` nếu không có.
    fn pick(&self, point: Point) -> Option<u32>;
    /// Điểm chân đang vẽ của con `pet` (trễ `pet.x/y` tới một bước lúc bay).
    fn foot(&self, pet: u32) -> Option<Point>;
    /// Đưa con `pet` lên trên các con khác và giữ chuột (nhận cả khi con trỏ ra ngoài overlay).
    fn grab_pointer(&mut self, pet: u32);
    /// Thả chuột ra.
    fn release_pointer(&mut self);
    /// Bắt đầu / kết thúc giữ chuột trên pet (click-through).
    fn hold(&mut self, held: bool);
    /// Pet vừa được click hoặc kéo: chạy lại vòng lặp vẽ nếu đang dừng.
    fn activity(&mut self);
    fn grab(&mut self, pet: u32);
    fn drag_to(&mut self, pet: u32, x: f64, y: f64);
    fn release(&mut self, pet: u32, vx: f64, vy: f64);
    fn poke(&mut self, pet: u32);
    /// Đang kéo pet mà con trỏ ra ngoài overlay (CSS pixel của overlay): có thể là sang màn hình khác.
    fn drag_outside(&mut self, point: Point);
    /// `point` nằm trong overlay không.
    fn inside(&self, point: Point) -> bool;
    /// Bấm đúp vào `pet` (lần click thứ hai vẫn làm pet nhảy như click thường).
    fn double_click(&mut self, pet: u32);
    /// Shift + click trái vào `pet`, không kéo: kỹ năng riêng của nhân vật. `false` nếu con đó không có, khi đó tính
    /// như click thường.
    fn skill(&mut self, pet: u32) -> bool;
}

#[derive(Debug, Default)]
pub struct Interaction {
    press: Option<Press>,
    /// Lần click (không kéo) gần nhất, để nhận ra bấm đúp.
    last_click: Option<(u32, f64)>,
}

impl Interaction {
    /// Pet đang được giữ chuột, `None` nếu không có.
    pub fn held(&self) -> Option<u32> {
        self.press.as_ref().map(|p| p.pet)
    }

    /// Ẩn/tạm dừng overlay hoặc mất quyền giữ chuột thì thả nhẹ, tránh kẹt ở state bị kéo.
    pub fn cancel(&mut self, host: &mut impl Host) {
        let Some(press) = self.finish(host) else {
            return;
        };
        if press.dragging {
            host.release(press.pet, 0.0, 0.0);
        }
    }

    /// Overlay vừa đổi chỗ (kéo pet sang màn hình khác): đổi các điểm đã ghi sang toạ độ mới, để lúc buông vận tốc
    /// ném không bị tính từ toạ độ cũ. Khoảng từ con trỏ tới chân pet giữ nguyên.
    pub fn remap(&mut self, remap: &Remap) {
        let Some(press) = &mut self.press else {
            return;
        };
        let map = |x: f64, y: f64| (x * remap.scale + remap.x, y * remap.scale + remap.y);
        let (x, y) = map(press.start.x, press.start.y);
        press.start = Point { x, y };
        for sample in &mut press.samples {
            (sample.x, sample.y) = map(sample.x, sample.y);
        }
    }

    pub fn down(&mut self, host: &mut impl Host, point: Point, shift: bool, time: f64) {
        if self.press.is_some() {
            return;
        }
        let Some(pet) = host.pick(point) else {
            return;
        };
        // Con đang bị kéo nằm trên các con khác; vẫn nhận chuột khi con trỏ chạy ra ngoài overlay.
        host.grab_pointer(pet);
        let foot = host.foot(pet).unwrap_or(point);
        self.press = Some(Press {
            pet,
            start: point,
            offset: Point { x: foot.x - point.x, y: foot.y - point.y },
            dragging: false,
            samples: VecDeque::from([Sample { x: point.x, y: point.y, t: time }]),
            skill: shift,
        });
        host.hold(true);
    }

    pub fn moved(&mut self, host: &mut impl Host, point: Point, time: f64) {
        let Some(press) = &mut self.press else {
            return;
        };
        press.samples.push_back(Sample { x: point.x, y: point.y, t: time });
        while press.samples.len() > 2 && time - press.samples[0].t > THROW_WINDOW {
            press.samples.pop_front();
        }
        if !press.dragging {
            if (point.x - press.start.x).hypot(point.y - press.start.y) < DRAG_THRESHOLD {
                return;
            }
            press.dragging = true;
            host.grab(press.pet);
        }
        host.drag_to(press.pet, point.x + press.offset.x, point.y + press.offset.y);
        host.activity();
        if !host.inside(point) {
            host.drag_outside(point);
        }
    }

    pub fn up(&mut self, host: &mut impl Host, time: f64) {
        let Some(press) = self.finish(host) else {
            return;
        };
        let pet = press.pet;
        if press.dragging {
            self.last_click = None;
            let (vx, vy) = throw_velocity(&press.samples, time);
            host.release(pet, vx, vy);
            return;
        }
        if press.skill && host.skill(pet) {
            self.last_click = None;
            return;
        }
        host.poke(pet);
        let double = self.last_click.is_some_and(|(last, at)| last == pet && time - at < DOUBLE_CLICK_MS);
        // Bấm đúp xong thì lần click sau tính lại từ đầu, không thành bấm ba.
        self.last_click = if double { None } else { Some((pet, time)) };
        if double {
            host.double_click(pet);
        }
    }

    fn finish(&mut self, host: &mut impl Host) -> Option<Press> {
        let press = self.press.take()?;
        host.release_pointer();
        host.hold(false);
        host.activity();
        Some(press)
    }
}

/// Vận tốc chuột (px/s) ngay trước lúc buông; dừng tay rồi mới buông thì coi như thả nhẹ.
fn throw_velocity(samples: &VecDeque<Sample>, now: f64) -> (f64, f64) {
    let recent: Vec<&Sample> = samples.iter().filter(|s| now - s.t <= THROW_WINDOW).collect();
    let (Some(first), Some(last)) = (recent.first(), recent.last()) else {
        return (0.0, 0.0);
    };
    if recent.len() < 2 {
        return (0.0, 0.0);
    }
    let dt = (last.t - first.t) / 1000.0;
    if dt < 0.01 {
        return (0.0, 0.0);
    }
    ((last.x - first.x) / dt, (last.y - first.y) / dt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn van_toc_nem_tu_doan_cuoi() {
        let samples: VecDeque<Sample> =
            [(0.0, 0.0, 0.0), (10.0, 0.0, 950.0), (60.0, 20.0, 1000.0)].map(|(x, y, t)| Sample { x, y, t }).into();
        assert_eq!(throw_velocity(&samples, 1000.0), (1000.0, 400.0));
        // Dừng tay rồi mới buông.
        assert_eq!(throw_velocity(&samples, 1200.0), (0.0, 0.0));
    }
}
