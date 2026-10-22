//! Giờ theo thành phố đã chọn và lịch sự kiện (`calendar.ts`).

use super::lunar::to_lunar;
use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};
use tinyworld_backend::settings::Occasion;

/// Giờ đồng hồ lúc `now` ở múi giờ `time_zone` (IANA, của thành phố đã chọn). Không có hoặc sai múi giờ thì dùng
/// giờ máy.
pub fn wall_clock(now: Timestamp, time_zone: Option<&str>) -> DateTime {
    let zone = time_zone.and_then(|name| TimeZone::get(name).ok()).unwrap_or_else(TimeZone::system);
    now.to_zoned(zone).datetime()
}

/// Ngày bắt đầu của dịp `occasion` có nằm trong `occasion.days` ngày gần nhất tính tới `today` không.
fn happening(occasion: &Occasion, today: Date) -> bool {
    (0..i64::from(occasion.days)).any(|back| {
        let Ok(day) = today.checked_sub(back.days()) else {
            return false;
        };
        if occasion.lunar {
            let lunar = to_lunar(day);
            !lunar.leap && lunar.day == i32::from(occasion.day) && lunar.month == i32::from(occasion.month)
        } else {
            i32::from(day.day()) == i32::from(occasion.day) && i32::from(day.month()) == i32::from(occasion.month)
        }
    })
}

/// Các dịp đang diễn ra hôm nay (`today` theo giờ ở thành phố đã chọn, `wall_clock`), theo thứ tự trong lịch. Ngày
/// không có trong năm đó (30 âm lịch tháng thiếu, 29/2 năm không nhuận) thì năm đó không có.
pub fn active_occasions(occasions: &[Occasion], today: Date) -> Vec<&Occasion> {
    occasions.iter().filter(|occasion| occasion.enabled && happening(occasion, today)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    fn occasion(name: &str, day: u8, month: u8, lunar: bool, days: u8) -> Occasion {
        Occasion {
            name: name.into(),
            day,
            month,
            lunar,
            days,
            message: String::new(),
            enabled: true,
            preset: None,
        }
    }

    fn names(today: Date, list: &[Occasion]) -> Vec<String> {
        active_occasions(list, today).into_iter().map(|o| o.name.clone()).collect()
    }

    #[test]
    fn doi_sang_gio_o_mui_gio_cua_thanh_pho() {
        // 1/10/2026 08:30 UTC: Hà Nội 15:30, Tokyo 17:30, New York 04:30.
        let now: Timestamp = "2026-10-01T08:30:00Z".parse().unwrap();
        let clock = |tz: &str| {
            let d = wall_clock(now, Some(tz));
            (d.day(), d.hour(), d.minute())
        };
        assert_eq!(clock("Asia/Bangkok"), (1, 15, 30));
        assert_eq!(clock("Asia/Tokyo"), (1, 17, 30));
        assert_eq!(clock("America/New_York"), (1, 4, 30));
    }

    #[test]
    fn chua_co_hoac_sai_mui_gio_thi_dung_gio_may() {
        let now = Timestamp::now();
        let local = now.to_zoned(TimeZone::system()).datetime();
        assert_eq!(wall_clock(now, None), local);
        assert_eq!(wall_clock(now, Some("Hành tinh/Sao Hoả")), local);
    }

    #[test]
    fn tet_theo_am_lich_keo_dai_tu_mung_1_toi_mung_5() {
        let tet = [occasion("Tết Nguyên Đán", 1, 1, true, 5)];
        assert_eq!(names(date(2026, 2, 17), &tet), ["Tết Nguyên Đán"]);
        assert_eq!(names(date(2026, 2, 21), &tet), ["Tết Nguyên Đán"]);
        assert!(names(date(2026, 2, 22), &tet).is_empty());
        assert!(names(date(2026, 2, 16), &tet).is_empty());
        // Năm sau Tết sang ngày khác.
        assert_eq!(names(date(2027, 2, 6), &tet), ["Tết Nguyên Đán"]);
    }

    #[test]
    fn duong_lich_keo_dai_qua_ngay_am_lich_nhu_ram_trung_thu() {
        let noel = [occasion("Giáng sinh", 24, 12, false, 2)];
        assert_eq!(names(date(2026, 12, 25), &noel), ["Giáng sinh"]);
        assert!(names(date(2026, 12, 26), &noel).is_empty());
        let mid_autumn = [occasion("Trung thu", 15, 8, true, 1)];
        assert_eq!(names(date(2026, 9, 25), &mid_autumn), ["Trung thu"]);
    }

    #[test]
    fn nhieu_dip_cung_ngay_theo_thu_tu_trong_lich_dip_tat_thi_khong_tinh() {
        let birthday = Occasion { message: "Chúc mừng sinh nhật :)))".into(), ..occasion("Sinh nhật", 17, 2, false, 1) };
        let tet = occasion("Tết Nguyên Đán", 1, 1, true, 5);
        let list = [occasion("Trung thu", 15, 8, true, 1), birthday.clone(), tet.clone()];
        let today = date(2026, 2, 17);
        assert_eq!(names(today, &list), ["Sinh nhật", "Tết Nguyên Đán"]);
        assert_eq!(names(today, &[Occasion { enabled: false, ..birthday }, tet]), ["Tết Nguyên Đán"]);
        assert!(names(date(2026, 5, 5), &list).is_empty());
    }
}
