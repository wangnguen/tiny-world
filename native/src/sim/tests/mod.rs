//! Test hành vi của pet, chuyển từ các file `*.test.ts` của packages/sim. Bước mô phỏng 1/30 giây và cách đếm thời
//! gian như bản TS, nên cùng seed ra cùng kết quả.

mod blink;
mod cursor;
mod group;
mod pet;
mod play;
mod windows;

use super::pet::{PetOptions, PetState};
use super::terrain::Bounds;
use super::world::World;
use super::{Rect, WindowInfo};

pub const DT: f64 = 1.0 / 30.0;
pub const BOUNDS: Bounds = Bounds { left: 0.0, right: 1000.0, top: 0.0, floor: 700.0 };

pub fn options(id: &str, x: f64, size: f64) -> PetOptions {
    PetOptions { id: id.into(), x, width: size, height: size, reach: None }
}

/// Thế giới `bounds` với các pet 60 px đứng ở các chỗ `xs` (id "p0", "p1"...).
pub fn group(xs: &[f64], seed: u64, bounds: Bounds) -> World {
    let mut world = World::new(bounds, seed);
    for (i, &x) in xs.iter().enumerate() {
        world.spawn(options(&format!("p{i}"), x, 60.0));
    }
    world
}

pub fn win(id: isize, x: f64, y: f64, width: f64, height: f64) -> WindowInfo {
    WindowInfo { id, rect: Rect { x, y, width, height } }
}

/// Chạy `seconds` giây, sau mỗi bước gọi `each` cho từng pet. Trả về các state theo thứ tự (bỏ lặp liên tiếp).
pub fn simulate(world: &mut World, seconds: f64, mut each: impl FnMut(&World, usize)) -> Vec<PetState> {
    let mut seen: Vec<PetState> = Vec::new();
    let mut t = 0.0;
    while t < seconds {
        world.step(DT);
        for i in 0..world.pets.len() {
            each(world, i);
            let state = world.pets[i].state();
            if seen.last() != Some(&state) {
                seen.push(state);
            }
        }
        t += DT;
    }
    seen
}

/// Chạy `seconds` giây không làm gì thêm.
pub fn run(world: &mut World, seconds: f64) {
    simulate(world, seconds, |_, _| {});
}

/// Chạy tối đa `seconds` giây; `until` đúng thì dừng sớm, trả về `true`.
pub fn run_until(world: &mut World, seconds: f64, mut until: impl FnMut(&World) -> bool) -> bool {
    let mut t = 0.0;
    while t < seconds {
        world.step(DT);
        if until(world) {
            return true;
        }
        t += DT;
    }
    false
}

/// Nhấc pet `i` lên tới điểm chân (x, y) rồi thả với vận tốc (vx, vy).
pub fn drop_at(world: &mut World, i: usize, x: f64, y: f64, vx: f64, vy: f64) {
    let mut pet = world.pet(i);
    pet.grab();
    pet.drag_to(x, y);
    pet.release(vx, vy);
}

/// Thả pet `i` lên mép cửa sổ rồi chờ nó đứng yên.
pub fn stand_on(world: &mut World, i: usize, x: f64, y: f64) {
    drop_at(world, i, x, y - 40.0, 0.0, 0.0);
    run(world, 1.0);
    assert!(world.pets[i].mount.is_some());
}

/// Thử lần lượt từng seed, chạy tới khi `until` đúng rồi trả về thế giới lúc đó. Hành vi ngẫu nhiên nên không phải
/// seed nào cũng xảy ra trong 40 giây; 20 seed không cái nào xảy ra thì test lỗi.
pub fn first_world(make: impl Fn(u64) -> World, mut until: impl FnMut(&World) -> bool) -> World {
    for seed in 1..=20 {
        let mut world = make(seed);
        if run_until(&mut world, 40.0, &mut until) {
            return world;
        }
    }
    panic!("Không seed nào xảy ra.");
}

/// Như `toBeCloseTo` của vitest: lệch dưới nửa đơn vị ở chữ số thập phân thứ `digits`.
pub fn close_to(actual: f64, expected: f64, digits: i32) -> bool {
    (actual - expected).abs() < 10f64.powi(-digits) / 2.0
}

pub fn close(actual: f64, expected: f64) -> bool {
    close_to(actual, expected, 2)
}
