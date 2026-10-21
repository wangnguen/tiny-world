//! Thời tiết quanh pet (`weather.ts`): mã WMO thành kiểu trời, nắng/cánh hoa ban ngày, nóng/lạnh, thời tiết giả lập
//! khi chưa chọn thành phố.

use super::rng::Rng;
use super::Sky;
use jiff::civil::DateTime;

/// Từ nhiệt độ này trở lên là nóng (hơi nóng bốc lên quanh pet), °C.
pub const HOT_FROM: f64 = 33.0;
/// Từ nhiệt độ này trở xuống là lạnh (pet thở ra khói), °C.
pub const COLD_UNTIL: f64 = 15.0;

/// Kết quả thật (`Report`) quá chừng này giây thì coi như không có.
pub const REPORT_MAX_AGE: u64 = 3 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warmth {
    Hot,
    Cold,
}

impl Warmth {
    /// Tên mục trong file chữ ("pet.hot").
    pub fn key(self) -> &'static str {
        match self {
            Self::Hot => "hot",
            Self::Cold => "cold",
        }
    }
}

/// Nóng, lạnh hay bình thường (`None`, kể cả khi chưa biết nhiệt độ).
pub fn warmth_of(temperature: Option<f64>) -> Option<Warmth> {
    let temperature = temperature?;
    if temperature >= HOT_FROM {
        Some(Warmth::Hot)
    } else if temperature <= COLD_UNTIL {
        Some(Warmth::Cold)
    } else {
        None
    }
}

/// Mã thời tiết WMO (Open-Meteo) thành thời tiết quanh pet: 0–3 quang/nhiều mây, 45/48 sương mù, 51–67 và 80–82 mưa,
/// 71–77 và 85–86 tuyết, 95–99 sấm. Mã lạ thì coi như nhiều mây. Trời quang ban ngày: gọi `day_sky` sau đó.
pub fn sky_of(code: u16) -> Sky {
    match code {
        0 | 1 => Sky::Clear,
        2 | 3 => Sky::Cloudy,
        45 | 48 => Sky::Fog,
        51..=67 | 80..=82 => Sky::Rain,
        71..=77 | 85 | 86 => Sky::Snow,
        95..=99 => Sky::Storm,
        _ => Sky::Cloudy,
    }
}

/// Trời quang ban ngày là nắng; mùa xuân (tháng 2–4, quanh Tết tới hết tháng 4) thì cánh hoa rơi. Ban đêm vẫn là trời
/// quang (sao). `month`: 1–12.
pub fn day_sky(sky: Sky, month: i8, day: bool) -> Sky {
    if sky != Sky::Clear || !day {
        return sky;
    }
    if (2..=4).contains(&month) {
        Sky::Petals
    } else {
        Sky::Sunny
    }
}

/// Tỉ lệ từng kiểu thời tiết giả lập theo mùa (cộng lại bằng 1). `month`: 1–12.
fn season_of(month: i8) -> &'static [(Sky, f64)] {
    match month {
        2..=4 => &[(Sky::Clear, 0.45), (Sky::Cloudy, 0.3), (Sky::Rain, 0.15), (Sky::Fog, 0.1)],
        5..=8 => &[(Sky::Clear, 0.4), (Sky::Cloudy, 0.25), (Sky::Rain, 0.25), (Sky::Storm, 0.1)],
        9..=11 => &[(Sky::Clear, 0.45), (Sky::Cloudy, 0.3), (Sky::Rain, 0.2), (Sky::Fog, 0.05)],
        _ => &[(Sky::Clear, 0.35), (Sky::Cloudy, 0.35), (Sky::Fog, 0.15), (Sky::Rain, 0.1), (Sky::Snow, 0.05)],
    }
}

/// Mỗi khung giờ thời tiết giả lập dài chừng này giờ.
const SIMULATED_BLOCK_HOURS: i8 = 3;

/// Thời tiết giả lập (chưa chọn thành phố): đổi theo mùa mỗi 3 tiếng. Cùng ngày, cùng khung giờ thì luôn ra cùng một
/// thời tiết, mở lại app không bị đổi. Chưa phân nắng, cánh hoa: gọi `day_sky` sau đó.
pub fn simulated_sky(now: DateTime) -> Sky {
    let block = i64::from(now.hour() / SIMULATED_BLOCK_HOURS);
    let day = i64::from(now.year()) * 10_000 + i64::from(now.month()) * 100 + i64::from(now.day());
    let roll = Rng::new((day * 8 + block) as u64).next();
    let mut sum = 0.0;
    for &(sky, share) in season_of(now.month()) {
        sum += share;
        if roll < sum {
            return sky;
        }
    }
    Sky::Clear
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn ma_wmo_doi_dung_thanh_hieu_ung() {
        let cases = [
            (0, Sky::Clear),
            (1, Sky::Clear),
            (2, Sky::Cloudy),
            (3, Sky::Cloudy),
            (45, Sky::Fog),
            (48, Sky::Fog),
            (51, Sky::Rain),
            (61, Sky::Rain),
            (67, Sky::Rain),
            (80, Sky::Rain),
            (82, Sky::Rain),
            (71, Sky::Snow),
            (77, Sky::Snow),
            (85, Sky::Snow),
            (86, Sky::Snow),
            (95, Sky::Storm),
            (99, Sky::Storm),
            (42, Sky::Cloudy),
        ];
        for (code, sky) in cases {
            assert_eq!(sky_of(code), sky, "{code}");
        }
    }

    #[test]
    fn troi_quang_ban_ngay_nang_mua_xuan_canh_hoa_ban_dem_van_quang() {
        assert_eq!(day_sky(Sky::Clear, 3, true), Sky::Petals);
        assert_eq!(day_sky(Sky::Clear, 3, false), Sky::Clear);
        assert_eq!(day_sky(Sky::Rain, 3, true), Sky::Rain);
        assert_eq!(day_sky(Sky::Clear, 8, true), Sky::Sunny);
        assert_eq!(day_sky(Sky::Clear, 8, false), Sky::Clear);
        assert_eq!(day_sky(Sky::Cloudy, 8, true), Sky::Cloudy);
    }

    #[test]
    fn nong_tu_33_lanh_tu_15_tro_xuong_chua_biet_thi_binh_thuong() {
        assert_eq!(warmth_of(Some(33.0)), Some(Warmth::Hot));
        assert_eq!(warmth_of(Some(32.9)), None);
        assert_eq!(warmth_of(Some(15.0)), Some(Warmth::Cold));
        assert_eq!(warmth_of(Some(15.1)), None);
        assert_eq!(warmth_of(Some(-5.0)), Some(Warmth::Cold));
        assert_eq!(warmth_of(None), None);
    }

    #[test]
    fn gia_lap_cung_khung_3_tieng_thi_nhu_nhau() {
        let a = simulated_sky(date(2026, 10, 1).at(13, 5, 0, 0));
        assert_eq!(simulated_sky(date(2026, 10, 1).at(14, 59, 0, 0)), a);
    }

    #[test]
    fn gia_lap_mua_he_khong_co_tuyet_co_luc_mua_hay_sam() {
        let mut summer = Vec::new();
        for day in 1..=30 {
            for hour in (0..24).step_by(3) {
                summer.push(simulated_sky(date(2026, 6, day).at(hour, 0, 0, 0)));
            }
        }
        assert!(!summer.contains(&Sky::Snow));
        assert!(summer.contains(&Sky::Rain) || summer.contains(&Sky::Storm));
        assert!(summer.contains(&Sky::Clear));
    }

    #[test]
    fn gia_lap_giong_ban_ts() {
        // Seed của 1/10/2026 khung 13–15 giờ: 20261001 * 8 + 4, như `simulatedSky` trong packages/sim.
        let roll = Rng::new(20_261_001 * 8 + 4).next();
        let sky = simulated_sky(date(2026, 10, 1).at(13, 0, 0, 0));
        let expected = if roll < 0.45 { Sky::Clear } else if roll < 0.75 { Sky::Cloudy } else if roll < 0.95 { Sky::Rain } else { Sky::Fog };
        assert_eq!(sky, expected);
    }
}
