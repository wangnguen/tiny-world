/// Kẹp `value` vào [min, max]; khoảng rỗng (pet rộng hơn màn hình) thì lấy điểm giữa.
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if max < min {
        return (min + max) / 2.0;
    }
    value.max(min).min(max)
}
