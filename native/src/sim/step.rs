//! Chia thời gian thực thành bước mô phỏng cố định (`FixedStep`), và vị trí để vẽ giữa hai bước (`StepBlend`).

use super::Point;

/// Chia thời gian thực thành các bước cố định `dt` (giây) để mô phỏng không phụ thuộc fps. Máy bị lag hoặc overlay
/// bị treo lâu thì chỉ chạy bù tối đa `max_steps` bước, tránh pet "bay" xa.
#[derive(Debug, Clone)]
pub struct FixedStep {
    pub dt: f64,
    max_steps: u32,
    accumulated: f64,
}

impl FixedStep {
    pub fn new(dt: f64, max_steps: u32) -> Self {
        Self { dt, max_steps, accumulated: 0.0 }
    }

    /// Cộng thêm thời gian đã trôi (giây), trả về số bước cần chạy.
    pub fn advance(&mut self, elapsed: f64) -> u32 {
        self.accumulated += elapsed.max(0.0);
        // Bù sai số làm tròn để 3 × dt vẫn ra đúng 3 bước.
        let steps = (self.accumulated / self.dt + 1e-9).floor();
        if steps > f64::from(self.max_steps) {
            self.accumulated = 0.0;
            return self.max_steps;
        }
        self.accumulated = (self.accumulated - steps * self.dt).max(0.0);
        steps as u32
    }

    /// Phần bước kế tiếp đã trôi (0–1), để vẽ vị trí nằm giữa hai bước (`StepBlend`).
    pub fn alpha(&self) -> f64 {
        (self.accumulated / self.dt).min(1.0)
    }
}

/// Vị trí để vẽ giữa hai bước mô phỏng. Mô phỏng chỉ chạy 30 bước/giây: vẽ đúng vị trí bước cuối thì pet bị ném
/// (tới 3000 px/s) nhảy cóc cả trăm pixel, và khựng mỗi khi lượt vẽ lệch nhịp với bước (lượt 0 bước rồi lượt 2
/// bước). Vẽ trễ một bước, nội suy theo phần bước đã trôi (`FixedStep::alpha`), thì pet đi đều theo giờ thật.
#[derive(Debug, Clone, Default)]
pub struct StepBlend {
    /// Mỗi pet (theo `uid`): chỗ trước và sau bước gần nhất.
    moves: Vec<(u32, Point, Point)>,
}

impl StepBlend {
    /// Một bước mô phỏng vừa chạy: chỗ của từng pet (`uid`, chỗ) trước (`from`) và sau (`to`) bước đó. Tạo lại mỗi
    /// bước: pet bị bỏ (bớt nhân vật trong Cài đặt) không còn bị giữ lại ở đây.
    pub fn record(&mut self, from: &[(u32, Point)], to: &[(u32, Point)]) {
        self.moves = from
            .iter()
            .filter_map(|&(uid, a)| to.iter().find(|(id, _)| *id == uid).map(|&(_, b)| (uid, a, b)))
            .collect();
    }

    /// Chỗ vẽ pet `uid` đang ở `at` khi đã trôi `alpha` (0–1) của bước kế tiếp. Bị dời ngoài mô phỏng (kéo bằng chuột,
    /// sang màn hình khác, cửa sổ đang đứng bị kéo lúc tạm dừng) thì vẽ ngay chỗ mới, không trượt từ chỗ cũ tới.
    pub fn at(&self, uid: u32, at: Point, alpha: f64) -> Point {
        match self.moves.iter().find(|(id, ..)| *id == uid) {
            Some((_, from, to)) if *to == at => {
                Point { x: from.x + (to.x - from.x) * alpha, y: from.y + (to.y - from.y) * alpha }
            }
            _ => at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn don_thoi_gian_le_sang_lan_sau() {
        let mut step = FixedStep::new(1.0 / 30.0, 5);
        assert_eq!(step.advance(1.0 / 60.0), 0);
        assert_eq!(step.advance(1.0 / 60.0), 1);
        assert_eq!(step.advance(3.0 / 30.0), 3);
    }

    #[test]
    fn chi_chay_bu_toi_da_max_steps_buoc() {
        let mut step = FixedStep::new(1.0 / 30.0, 5);
        assert_eq!(step.advance(10.0), 5);
        assert_eq!(step.advance(1.0 / 30.0), 1);
    }

    #[test]
    fn bo_qua_thoi_gian_am() {
        assert_eq!(FixedStep::new(1.0 / 30.0, 5).advance(-1.0), 0);
    }

    #[test]
    fn alpha_la_phan_buoc_ke_tiep_da_troi() {
        let mut step = FixedStep::new(1.0 / 30.0, 5);
        assert_eq!(step.alpha(), 0.0);
        step.advance(1.0 / 60.0);
        assert!((step.alpha() - 0.5).abs() < 1e-9);
        step.advance(1.0 / 30.0);
        assert!((step.alpha() - 0.5).abs() < 1e-9);
    }

    fn p(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn ve_giua_hai_buoc_theo_alpha() {
        let mut blend = StepBlend::default();
        blend.record(&[(1, p(0.0, 100.0)), (2, p(500.0, 100.0))], &[(1, p(50.0, 80.0)), (2, p(400.0, 100.0))]);
        assert_eq!(blend.at(1, p(50.0, 80.0), 0.0), p(0.0, 100.0));
        assert_eq!(blend.at(1, p(50.0, 80.0), 0.5), p(25.0, 90.0));
        assert_eq!(blend.at(1, p(50.0, 80.0), 1.0), p(50.0, 80.0));
        assert_eq!(blend.at(2, p(400.0, 100.0), 0.5), p(450.0, 100.0));
    }

    #[test]
    fn chua_co_buoc_nao_bi_doi_ngoai_mo_phong_hoac_khong_con_trong_buoc_thi_ve_dung_cho() {
        let mut blend = StepBlend::default();
        assert_eq!(blend.at(1, p(10.0, 20.0), 0.5), p(10.0, 20.0));
        blend.record(&[(1, p(0.0, 0.0))], &[(1, p(10.0, 0.0))]);
        // Bị kéo sang chỗ khác sau bước: vẽ đúng chỗ mới.
        assert_eq!(blend.at(1, p(300.0, 40.0), 0.5), p(300.0, 40.0));
        // Pet không có trong bước gần nhất.
        assert_eq!(blend.at(9, p(5.0, 5.0), 0.5), p(5.0, 5.0));
    }
}
