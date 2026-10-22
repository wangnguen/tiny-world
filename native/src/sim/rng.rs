//! Số ngẫu nhiên có seed (mulberry32), cùng thuật toán với `Rng` của packages/sim: cùng seed cho ra cùng một chuỗi
//! số, nên hành vi ngẫu nhiên của pet test được và tái hiện được.

#[derive(Debug, Clone)]
pub struct Rng {
    state: u32,
}

impl Rng {
    /// Như `seed >>> 0` của JavaScript: lấy 32 bit thấp.
    pub fn new(seed: u64) -> Self {
        Self { state: seed as u32 }
    }

    /// Số thực trong [0, 1). Trùng tên với `Rng.next` của bản TS, không phải iterator.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }

    /// Số thực trong [min, max).
    pub fn range(&mut self, min: f64, max: f64) -> f64 {
        min + (max - min) * self.next()
    }

    /// `true` với xác suất `p` (0–1).
    pub fn chance(&mut self, p: f64) -> bool {
        self.next() < p
    }

    /// Chỉ số ngẫu nhiên trong [0, `len`). `len` phải lớn hơn 0.
    pub fn index(&mut self, len: usize) -> usize {
        assert!(len > 0, "Rng::index: danh sách rỗng");
        ((self.next() * len as f64).floor() as usize).min(len - 1)
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.index(items.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cung_seed_cho_cung_chuoi_so() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        let seq: Vec<f64> = (0..5).map(|_| a.next()).collect();
        assert_eq!((0..5).map(|_| b.next()).collect::<Vec<_>>(), seq);
        assert_ne!(Rng::new(43).next(), seq[0]);
    }

    #[test]
    fn giong_het_ban_javascript() {
        // `new Rng(42)` trong packages/sim cho ra đúng các số này.
        let mut rng = Rng::new(42);
        assert_eq!([rng.next(), rng.next(), rng.next()], [0.6011037519201636, 0.44829055899754167, 0.8524657934904099]);
        // `seed >>> 0`: chỉ lấy 32 bit thấp, như `new Rng(Date.now())`.
        assert_eq!(Rng::new(1_760_000_000_123).state, 3_358_376_059);
    }

    #[test]
    fn nam_trong_khoang() {
        let mut rng = Rng::new(1);
        for _ in 0..1000 {
            let v = rng.range(-2.0, 3.0);
            assert!((-2.0..3.0).contains(&v));
        }
        assert!(["a", "b"].contains(rng.pick(&["a", "b"])));
    }

    #[test]
    #[should_panic]
    fn danh_sach_rong_thi_loi() {
        Rng::new(1).pick::<u8>(&[]);
    }
}
