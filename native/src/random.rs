//! Số ngẫu nhiên cho phần vẽ (hạt mưa, sấm, con ma), như `Math.random()` của bản web: không cần tái hiện được nên
//! không dùng chung `World::rng` (đổi lượt bốc số của mô phỏng thì pet sống khác đi).

use crate::sim::rng::Rng;
use std::cell::RefCell;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static RNG: RefCell<Rng> = RefCell::new(Rng::new(
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64 ^ 0x9e37_79b9),
    ));
}

/// Số thực trong [0, 1).
pub fn random() -> f64 {
    RNG.with(|rng| rng.borrow_mut().next())
}

/// Số thực trong [`min`, `max`).
pub fn between((min, max): (f64, f64)) -> f64 {
    min + random() * (max - min)
}
