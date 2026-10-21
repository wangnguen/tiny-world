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

    /// Overlay giả: con trỏ ở `x` < 100 là con 1, < 200 là con 2, còn lại là chỗ trống.
    #[derive(Default)]
    struct Fake {
        foot: Option<Point>,
        positions: std::collections::HashMap<u32, (f64, f64)>,
        dragging: Option<u32>,
        holds: Vec<bool>,
        releases: Vec<(f64, f64)>,
        pokes: Vec<u32>,
        doubles: Vec<u32>,
        skills: Vec<u32>,
        /// Con nào có kỹ năng riêng (Shift + click).
        skilled: bool,
        raised: Vec<u32>,
        captured: bool,
        outside: Vec<Point>,
    }

    impl Host for Fake {
        fn pick(&self, point: Point) -> Option<u32> {
            if point.x < 100.0 {
                Some(1)
            } else if point.x < 200.0 {
                Some(2)
            } else {
                None
            }
        }
        fn foot(&self, _: u32) -> Option<Point> {
            self.foot.or(Some(Point { x: 100.0, y: 200.0 }))
        }
        fn grab_pointer(&mut self, pet: u32) {
            self.raised.push(pet);
            self.captured = true;
        }
        fn release_pointer(&mut self) {
            self.captured = false;
        }
        fn hold(&mut self, held: bool) {
            self.holds.push(held);
        }
        fn activity(&mut self) {}
        fn grab(&mut self, pet: u32) {
            self.dragging = Some(pet);
        }
        fn drag_to(&mut self, pet: u32, x: f64, y: f64) {
            self.positions.insert(pet, (x, y));
        }
        fn release(&mut self, _: u32, vx: f64, vy: f64) {
            self.dragging = None;
            self.releases.push((vx, vy));
        }
        fn poke(&mut self, pet: u32) {
            self.pokes.push(pet);
        }
        fn drag_outside(&mut self, point: Point) {
            self.outside.push(point);
        }
        fn inside(&self, point: Point) -> bool {
            point.x >= 0.0 && point.y >= 0.0
        }
        fn double_click(&mut self, pet: u32) {
            self.doubles.push(pet);
        }
        fn skill(&mut self, pet: u32) -> bool {
            self.skills.push(pet);
            self.skilled
        }
    }

    fn at(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    /// Bấm vào con 1 rồi kéo đi một đoạn.
    fn drag(interaction: &mut Interaction, host: &mut Fake) {
        interaction.down(host, at(50.0, 190.0), false, 0.0);
        interaction.moved(host, at(70.0, 100.0), 20.0);
        assert_eq!(host.dragging, Some(1));
    }

    #[test]
    fn mat_quyen_giu_chuot_thi_tha_pet_dang_keo_va_tra_lai_click_through() {
        let (mut interaction, mut host) = (Interaction::default(), Fake::default());
        drag(&mut interaction, &mut host);
        interaction.cancel(&mut host);
        interaction.cancel(&mut host);
        assert_eq!(host.dragging, None);
        assert!(!host.captured);
        assert_eq!(host.releases, [(0.0, 0.0)]);
        assert_eq!(host.holds, [true, false]);
        interaction.up(&mut host, 30.0);
        assert_eq!(host.releases.len(), 1);
    }

    #[test]
    fn click_thuong_van_cham_mot_lan() {
        let (mut interaction, mut host) = (Interaction::default(), Fake::default());
        interaction.down(&mut host, at(50.0, 190.0), false, 0.0);
        interaction.up(&mut host, 20.0);
        assert_eq!(host.pokes, [1]);
        assert!(!host.captured);
        assert_eq!(host.holds, [true, false]);
        assert!(host.releases.is_empty());
    }

    #[test]
    fn shift_click_goi_ky_nang_thay_vi_cham_hay_bam_dup() {
        let (mut interaction, mut host) = (Interaction::default(), Fake { skilled: true, ..Fake::default() });
        interaction.down(&mut host, at(50.0, 190.0), true, 100.0);
        interaction.up(&mut host, 120.0);
        assert_eq!(host.skills, [1]);
        assert!(host.pokes.is_empty());
    }

    #[test]
    fn shift_click_vao_con_khong_co_ky_nang_la_click_thuong() {
        let (mut interaction, mut host) = (Interaction::default(), Fake::default());
        interaction.down(&mut host, at(50.0, 190.0), true, 100.0);
        interaction.up(&mut host, 120.0);
        assert_eq!(host.skills, [1]);
        assert_eq!(host.pokes, [1]);
    }

    #[test]
    fn keo_theo_khoang_lech_luc_bam_va_nem_luc_buong() {
        let (mut interaction, mut host) = (Interaction::default(), Fake::default());
        drag(&mut interaction, &mut host);
        interaction.moved(&mut host, at(90.0, 80.0), 40.0);
        assert_eq!(host.positions[&1], (140.0, 90.0));
        interaction.up(&mut host, 45.0);
        assert_eq!(host.dragging, None);
        assert_eq!(host.releases, [(1000.0, -2750.0)]);
        assert_eq!(host.holds, [true, false]);
    }

    #[test]
    fn bat_pet_dang_bay_giu_dung_cho_dang_ve() {
        let (mut interaction, mut host) = (Interaction::default(), Fake { foot: Some(at(80.0, 170.0)), ..Fake::default() });
        interaction.down(&mut host, at(50.0, 190.0), false, 0.0);
        interaction.moved(&mut host, at(70.0, 100.0), 20.0);
        assert_eq!(host.positions[&1], (100.0, 80.0));
    }

    #[test]
    fn bam_vao_pet_thi_dua_no_len_tren_cac_con_khac() {
        let (mut interaction, mut host) = (Interaction::default(), Fake::default());
        interaction.down(&mut host, at(150.0, 50.0), false, 0.0);
        assert_eq!(host.raised, [2]);
        assert_eq!(interaction.held(), Some(2));
    }

    #[test]
    fn nhieu_pet_chi_bat_con_duoi_con_tro_cho_trong_thi_di_xuyen() {
        let (mut interaction, mut host) = (Interaction::default(), Fake::default());
        interaction.down(&mut host, at(150.0, 50.0), false, 0.0);
        interaction.up(&mut host, 10.0);
        assert_eq!(host.pokes, [2]);
        interaction.down(&mut host, at(300.0, 50.0), false, 20.0);
        assert_eq!(interaction.held(), None);
        interaction.up(&mut host, 30.0);
        assert_eq!(host.pokes, [2]);
    }

    #[test]
    fn hai_lan_click_nhanh_la_bam_dup_lan_ba_tinh_lai() {
        let (mut interaction, mut host) = (Interaction::default(), Fake::default());
        let mut click = |time: f64| {
            interaction.down(&mut host, at(10.0, 10.0), false, time);
            interaction.up(&mut host, time);
        };
        click(0.0);
        click(250.0);
        click(500.0);
        // Hai lần cách nhau lâu thì không phải bấm đúp.
        click(2000.0);
        assert_eq!(host.doubles, [1]);
        assert_eq!(host.pokes.len(), 4);
    }

    #[test]
    fn keo_ra_ngoai_overlay_thi_bao_de_sang_man_hinh_khac() {
        let (mut interaction, mut host) = (Interaction::default(), Fake::default());
        drag(&mut interaction, &mut host);
        interaction.moved(&mut host, at(-20.0, 100.0), 40.0);
        assert_eq!(host.outside, [at(-20.0, 100.0)]);
    }

    #[test]
    fn van_toc_nem_tu_doan_cuoi() {
        let samples: VecDeque<Sample> =
            [(0.0, 0.0, 0.0), (10.0, 0.0, 950.0), (60.0, 20.0, 1000.0)].map(|(x, y, t)| Sample { x, y, t }).into();
        assert_eq!(throw_velocity(&samples, 1000.0), (1000.0, 400.0));
        // Dừng tay rồi mới buông.
        assert_eq!(throw_velocity(&samples, 1200.0), (0.0, 0.0));
    }
}
