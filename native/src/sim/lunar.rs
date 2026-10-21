//! Âm lịch Việt Nam (giờ GMT+7), theo thuật toán của Hồ Ngọc Đức: tính ngày sóc (trăng mới) và trung khí theo thiên
//! văn, nên không cần bảng ngày Tết từng năm (`lunar.ts`). Tính bằng số thực như bản TS để ra đúng từng ngày.

use jiff::civil::Date;
use std::f64::consts::PI;

/// Ngày âm lịch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LunarDate {
    pub day: i32,
    pub month: i32,
    pub year: i32,
    /// Tháng nhuận.
    pub leap: bool,
}

/// Múi giờ của lịch Việt Nam.
const TIME_ZONE: f64 = 7.0;

/// Số ngày Julius của ngày dương lịch `dd/mm/yy`.
fn julian_day(dd: f64, mm: f64, yy: f64) -> f64 {
    let a = ((14.0 - mm) / 12.0).floor();
    let y = yy + 4800.0 - a;
    let m = mm + 12.0 * a - 3.0;
    let jd = dd + ((153.0 * m + 2.0) / 5.0).floor() + 365.0 * y + (y / 4.0).floor() - (y / 100.0).floor()
        + (y / 400.0).floor()
        - 32045.0;
    if jd < 2_299_161.0 {
        return dd + ((153.0 * m + 2.0) / 5.0).floor() + 365.0 * y + (y / 4.0).floor() - 32083.0;
    }
    jd
}

/// Ngày (số ngày Julius, theo giờ địa phương) có trăng mới thứ `k` kể từ 1/1/1900.
fn new_moon_day(k: f64) -> f64 {
    let t = k / 1236.85;
    let t2 = t * t;
    let t3 = t2 * t;
    let dr = PI / 180.0;
    let mut jd = 2_415_020.759_33 + 29.530_588_68 * k + 0.000_117_8 * t2 - 0.000_000_155 * t3;
    jd += 0.000_33 * ((166.56 + 132.87 * t - 0.009_173 * t2) * dr).sin();
    let m = 359.2242 + 29.105_356_08 * k - 0.000_033_3 * t2 - 0.000_003_47 * t3;
    let mpr = 306.0253 + 385.816_918_06 * k + 0.010_730_6 * t2 + 0.000_012_36 * t3;
    let f = 21.2964 + 390.670_506_46 * k - 0.001_652_8 * t2 - 0.000_002_39 * t3;
    let mut c = (0.1734 - 0.000_393 * t) * (m * dr).sin() + 0.0021 * (2.0 * dr * m).sin();
    c = c - 0.4068 * (mpr * dr).sin() + 0.0161 * (dr * 2.0 * mpr).sin();
    c -= 0.0004 * (dr * 3.0 * mpr).sin();
    c = c + 0.0104 * (dr * 2.0 * f).sin() - 0.0051 * (dr * (m + mpr)).sin();
    c = c - 0.0074 * (dr * (m - mpr)).sin() + 0.0004 * (dr * (2.0 * f + m)).sin();
    c = c - 0.0004 * (dr * (2.0 * f - m)).sin() - 0.0006 * (dr * (2.0 * f + mpr)).sin();
    c = c + 0.001 * (dr * (2.0 * f - mpr)).sin() + 0.0005 * (dr * (2.0 * mpr + m)).sin();
    let delta_t = if t < -11.0 {
        0.001 + 0.000_839 * t + 0.000_226_1 * t2 - 0.000_008_45 * t3 - 0.000_000_081 * t * t3
    } else {
        -0.000_278 + 0.000_265 * t + 0.000_262 * t2
    };
    (jd + c - delta_t + 0.5 + TIME_ZONE / 24.0).floor()
}

/// Kinh độ mặt trời lúc đầu ngày `jdn`, chia thành 12 cung (0–11).
fn sun_longitude(jdn: f64) -> f64 {
    let t = (jdn - 2_451_545.5 - TIME_ZONE / 24.0) / 36525.0;
    let t2 = t * t;
    let dr = PI / 180.0;
    let m = 357.5291 + 35999.0503 * t - 0.000_155_9 * t2 - 0.000_000_48 * t * t2;
    let l0 = 280.466_45 + 36_000.769_83 * t + 0.000_303_2 * t2;
    let mut dl = (1.9146 - 0.004_817 * t - 0.000_014 * t2) * (dr * m).sin();
    dl += (0.019_993 - 0.000_101 * t) * (dr * 2.0 * m).sin() + 0.000_29 * (dr * 3.0 * m).sin();
    let mut l = (l0 + dl) * dr;
    l -= PI * 2.0 * (l / (PI * 2.0)).floor();
    (l / PI * 6.0).floor()
}

/// Ngày bắt đầu tháng 11 âm lịch (tháng có đông chí) của năm dương lịch `yy`.
fn month11(yy: f64) -> f64 {
    let off = julian_day(31.0, 12.0, yy) - 2_415_021.0;
    let k = (off / 29.530_588_853).floor();
    let nm = new_moon_day(k);
    if sun_longitude(nm) >= 9.0 {
        new_moon_day(k - 1.0)
    } else {
        nm
    }
}

/// Tháng nhuận nằm sau tháng 11 bắt đầu ngày `a11` bao nhiêu tháng.
fn leap_month_offset(a11: f64) -> f64 {
    let k = ((a11 - 2_415_021.076_998_695) / 29.530_588_853 + 0.5).floor();
    let mut i = 1.0;
    let mut arc = sun_longitude(new_moon_day(k + i));
    loop {
        let last = arc;
        i += 1.0;
        arc = sun_longitude(new_moon_day(k + i));
        if arc == last || i >= 14.0 {
            break;
        }
    }
    i - 1.0
}

/// Ngày âm lịch của ngày dương lịch `date`.
pub fn to_lunar(date: Date) -> LunarDate {
    let dd = f64::from(date.day());
    let mm = f64::from(date.month());
    let yy = f64::from(date.year());
    let day = julian_day(dd, mm, yy);
    let k = ((day - 2_415_021.076_998_695) / 29.530_588_853).floor();
    let mut month_start = new_moon_day(k + 1.0);
    if month_start > day {
        month_start = new_moon_day(k);
    }
    let mut a11 = month11(yy);
    let mut b11 = a11;
    let mut year;
    if a11 >= month_start {
        year = yy;
        a11 = month11(yy - 1.0);
    } else {
        year = yy + 1.0;
        b11 = month11(yy + 1.0);
    }
    let diff = ((month_start - a11) / 29.0).floor();
    let mut leap = false;
    let mut month = diff + 11.0;
    if b11 - a11 > 365.0 {
        let leap_diff = leap_month_offset(a11);
        if diff >= leap_diff {
            month = diff + 10.0;
            leap = diff == leap_diff;
        }
    }
    if month > 12.0 {
        month -= 12.0;
    }
    if month >= 11.0 && diff < 4.0 {
        year -= 1.0;
    }
    LunarDate { day: (day - month_start + 1.0) as i32, month: month as i32, year: year as i32, leap }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn lunar(y: i16, m: i8, d: i8) -> LunarDate {
        to_lunar(date(y, m, d))
    }

    #[test]
    fn mung_1_tet_dung_cac_nam_da_biet() {
        let tet = [(2023, 1, 22), (2024, 2, 10), (2025, 1, 29), (2026, 2, 17), (2027, 2, 6), (2028, 1, 26)];
        for (y, m, d) in tet {
            assert_eq!(lunar(y, m, d), LunarDate { day: 1, month: 1, year: i32::from(y), leap: false }, "{d}/{m}/{y}");
        }
    }

    #[test]
    fn ngay_thuong_ram_trung_thu_va_thang_nhuan() {
        // Trung thu 2026 (rằm tháng 8) là 25/9/2026.
        assert_eq!(lunar(2026, 9, 25), LunarDate { day: 15, month: 8, year: 2026, leap: false });
        assert_eq!(lunar(2026, 10, 1), LunarDate { day: 21, month: 8, year: 2026, leap: false });
        // Năm 2025 nhuận tháng 6: mùng 1 tháng 6 nhuận là 25/7/2025.
        assert_eq!(lunar(2025, 7, 25), LunarDate { day: 1, month: 6, year: 2025, leap: true });
        // 31/12 dương lịch vẫn thuộc năm âm lịch trước Tết.
        assert_eq!(lunar(2026, 12, 31).year, 2026);
    }
}
