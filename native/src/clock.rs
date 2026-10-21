//! Đồng hồ của vòng lặp vẽ: số ms từ lúc app chạy, như `performance.now()` của bản web.

use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();

pub fn now_ms() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}
