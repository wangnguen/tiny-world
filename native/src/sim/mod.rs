//! Mô phỏng pet, chuyển từ packages/sim (TypeScript) sang Rust: bước cố định, máy trạng thái, địa hình từ cửa sổ
//! thật, nhiều pet cùng sống, thời tiết, âm lịch, lịch sự kiện, câu pet nói. Không vẽ gì: renderer đọc `World` sau
//! mỗi bước để vẽ. Cùng seed thì chuỗi số ngẫu nhiên y như bản TS (`Rng`).

pub mod calendar;
pub mod lines;
pub mod lunar;
pub mod math;
pub mod moves;
pub mod pet;
pub mod rng;
pub mod snapshot;
pub mod step;
pub mod terrain;
pub mod tuning;
pub mod weather;
pub mod world;

#[cfg(test)]
mod tests;

pub use tinyworld_backend::geometry::{Rect, Remap};
pub use tinyworld_backend::i18n::Lang;
pub use tinyworld_backend::preview::Sky;
pub use tinyworld_backend::window_list::WindowInfo;

/// Điểm theo CSS pixel của overlay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Như `Math.round` của JavaScript: .5 làm tròn lên (cả số âm: -2.5 thành -2).
pub fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}
