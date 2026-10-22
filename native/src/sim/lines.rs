//! Mọi câu pet nói (speech bubble), theo ngôn ngữ `lang` (`lines.ts`). Câu chữ nằm ở mục `pet` (và `calendar`,
//! `occasions`) trong `packages/core/src/i18n/vi.json`, `en.json`: giọng thân mật, dùng ":)))" thay emoji.

use super::lunar::LunarDate;
use super::weather::Warmth;
use super::{js_round, Lang, Sky};
use jiff::civil::DateTime;
use tinyworld_backend::i18n::fill;
use tinyworld_backend::settings::Occasion;

/// Câu chào khi gặp nhau (chọn ngẫu nhiên một câu; hai ngôn ngữ cùng số câu nên cùng seed ra cùng lượt).
pub fn greetings(lang: Lang) -> Vec<String> {
    lang.list("pet.greetings")
}

fn degrees(temperature: f64) -> String {
    format!("{}°C", js_round(temperature) as i64)
}

/// Câu khi trời vừa đổi, kèm nhiệt độ nếu biết, ví dụ "Mưa rồi, 24°C :(". Nắng mà nóng thì kêu nóng.
pub fn sky_line(lang: Lang, sky: Sky, temperature: Option<f64>, warmth: Option<Warmth>) -> String {
    let key = if sky == Sky::Sunny && warmth == Some(Warmth::Hot) {
        "pet.sunnyHot".to_owned()
    } else {
        format!("pet.sky.{}", sky.key())
    };
    let parts = lang.list(&key);
    let (line, mood) = (parts.first().cloned().unwrap_or_default(), parts.get(1).cloned().unwrap_or_default());
    match temperature {
        None => lang.tf("pet.skyAlone", &[("text", &line), ("mood", &mood)]),
        Some(t) => lang.tf("pet.skyWithTemperature", &[("text", &line), ("temperature", &degrees(t)), ("mood", &mood)]),
    }
}

/// Câu khi trời vừa chuyển nóng hay lạnh (thời tiết không đổi), ví dụ "Nóng quá, 36°C :(((".
pub fn warmth_line(lang: Lang, warmth: Warmth, temperature: f64) -> String {
    lang.tf(&format!("pet.{}", warmth.key()), &[("temperature", &degrees(temperature))])
}

/// Không lấy được thời tiết: mất mạng.
pub fn no_network(lang: Lang) -> String {
    lang.t("pet.noNetwork")
}

/// Không lấy được thời tiết: máy chủ thời tiết lỗi.
pub fn no_weather(lang: Lang) -> String {
    lang.t("pet.noWeather")
}

/// Con ma bay qua lúc 2 giờ sáng: con đầu tiên bị giật mình nói câu này.
pub fn ghost_line(lang: Lang) -> String {
    lang.t("pet.ghost")
}

/// Nhắc nghỉ sau `minutes` phút ngồi liền.
pub fn break_line(lang: Lang, minutes: u32) -> String {
    lang.tf("pet.break", &[("minutes", &minutes)])
}

/// Nhắc uống nước sau một lúc ngồi máy.
pub fn water_line(lang: Lang) -> String {
    lang.t("pet.water")
}

/// Nhắc đi ngủ lúc khuya, `time` ví dụ "23:40".
pub fn bedtime_line(lang: Lang, time: &str) -> String {
    lang.tf("pet.bedtime", &[("time", &time)])
}

/// Bấm Ctrl+S dồn dập.
pub fn save_spam_line(lang: Lang) -> String {
    lang.t("pet.saveSpam")
}

/// Click chuột phải vào pet mà chưa bật chat.
pub fn chat_off_line(lang: Lang) -> String {
    lang.t("pet.chatOff")
}

/// GitHub có bản mới: một con báo, mỗi bản một lần.
pub fn update_line(lang: Lang, version: &str) -> String {
    lang.tf("pet.update", &[("version", &version)])
}

/// Thời tiết khi bấm đúp vào pet, ví dụ "mưa" (trời nắng mùa xuân thì là `Petals`, vẫn là "trời nắng").
pub fn sky_name(lang: Lang, sky: Sky) -> String {
    lang.t(&format!("pet.skyNames.{}", sky.key()))
}

/// Tên dịp và câu nói của dịp: dịp có sẵn chưa sửa thì theo ngôn ngữ `lang`, còn lại như người dùng ghi.
pub fn occasion_text(occasion: &Occasion, lang: Lang) -> (String, String) {
    let preset = occasion.preset.as_ref().map(|preset| lang.value(&format!("occasions.{preset}")));
    match preset.and_then(|p| Some((p["name"].as_str()?, p["message"].as_str()?))) {
        Some((name, message)) => (name.to_owned(), message.to_owned()),
        None => (occasion.name.clone(), occasion.message.clone()),
    }
}

/// Thời tiết ở thành phố đã chọn để nói khi bấm đúp; `sky` `None` là chưa có thời tiết.
#[derive(Debug, Clone, PartialEq)]
pub struct PlaceWeather {
    /// Tên ngắn của thành phố, ví dụ "Hà Nội".
    pub place: String,
    pub sky: Option<Sky>,
    pub temperature: Option<f64>,
    /// Lần lấy thời tiết gần nhất bị lỗi: nói câu này (`no_network`, `no_weather`) thay cho thời tiết.
    pub problem: Option<String>,
}

/// Một dòng thời tiết, ví dụ "Hà Nội: mưa, 27°C"; lấy không được thì là câu báo lỗi (`problem`).
pub fn weather_line(lang: Lang, weather: &PlaceWeather) -> String {
    if let (Some(sky), Some(temperature)) = (weather.sky, weather.temperature) {
        let sky = sky_name(lang, sky);
        return lang.tf(
            "pet.place",
            &[("place", &weather.place), ("sky", &sky), ("temperature", &degrees(temperature))],
        );
    }
    weather.problem.clone().unwrap_or_else(|| format!("{}: {}", weather.place, lang.t("pet.weatherUnknown")))
}

/// Câu khi bấm đúp vào pet: giờ, thứ, ngày dương lịch, ngày âm lịch, và thời tiết nếu đã chọn thành phố. Ví dụ
/// "15:42 · Thứ Tư 01/10", "Âm lịch 21/8", "Hà Nội: mưa, 27°C", mỗi phần một dòng.
pub fn now_text(lang: Lang, date: DateTime, lunar: LunarDate, weather: Option<&PlaceWeather>) -> String {
    let weekday = usize::from(date.weekday().to_sunday_zero_offset().unsigned_abs());
    let month = usize::from((date.month() - 1).unsigned_abs());
    let pick = |key: &str, index: usize| lang.value(key)[index].as_str().unwrap_or_default().to_owned();
    let clock = fill(
        lang.t("pet.clock"),
        &[
            ("hh", &format!("{:02}", date.hour())),
            ("min", &format!("{:02}", date.minute())),
            ("weekday", &pick("calendar.weekdays", weekday)),
            ("dd", &format!("{:02}", date.day())),
            ("mm", &format!("{:02}", date.month())),
            ("day", &date.day()),
            ("shortMonth", &pick("calendar.shortMonths", month)),
        ],
    );
    let leap = if lunar.leap { lang.t("pet.leap") } else { String::new() };
    let lunar = lang.tf("pet.lunar", &[("day", &lunar.day), ("month", &lunar.month), ("leap", &leap)]);
    let mut lines = vec![clock, lunar];
    if let Some(weather) = weather {
        lines.push(weather_line(lang, weather));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::lunar::to_lunar;
    use jiff::civil::date;

    #[test]
    fn cau_khi_troi_doi_kem_nhiet_do_neu_biet_nang_ma_nong_thi_keu_nong() {
        let vi = Lang::Vi;
        assert_eq!(sky_line(vi, Sky::Rain, Some(24.4), None), "Mưa rồi, 24°C :(");
        assert_eq!(sky_line(vi, Sky::Rain, None, None), "Mưa rồi :(");
        assert_eq!(sky_line(vi, Sky::Sunny, Some(36.2), Some(Warmth::Hot)), "Nắng to quá, 36°C :(((");
        assert_eq!(sky_line(vi, Sky::Sunny, Some(28.0), None), "Nắng đẹp ghê, 28°C :)))");
        assert_eq!(warmth_line(vi, Warmth::Cold, 9.6), "Lạnh ghê, 10°C :(((");
        assert_eq!(warmth_line(vi, Warmth::Hot, 35.0), "Nóng quá, 35°C :(((");
    }

    #[test]
    fn cau_khi_troi_doi_tieng_anh() {
        assert_eq!(sky_line(Lang::En, Sky::Rain, Some(24.4), None), "It's raining, 24°C :(");
        assert_eq!(warmth_line(Lang::En, Warmth::Cold, 9.6), "Brr, it's cold, 10°C :(((");
    }

    #[test]
    fn bam_dup_du_gio_thu_ngay_duong_ngay_am_va_thoi_tiet() {
        let now = date(2026, 10, 1).at(15, 4, 0, 0);
        let weather =
            PlaceWeather { place: "Hà Nội".into(), sky: Some(Sky::Rain), temperature: Some(26.6), problem: None };
        assert_eq!(
            now_text(Lang::Vi, now, to_lunar(now.date()), Some(&weather)),
            "15:04 · Thứ Năm 01/10\nÂm lịch 21/8\nHà Nội: mưa, 27°C"
        );
    }

    #[test]
    fn chua_chon_thanh_pho_thi_khong_noi_thoi_tiet_chua_co_thoi_tiet_thi_noi_ro() {
        // Mùng 1 Tết 2026.
        let now = date(2026, 2, 17).at(8, 30, 0, 0);
        let lunar = to_lunar(now.date());
        assert_eq!(now_text(Lang::Vi, now, lunar, None), "08:30 · Thứ Ba 17/02\nÂm lịch 1/1");
        let hue = PlaceWeather { place: "Huế".into(), sky: None, temperature: None, problem: None };
        assert!(now_text(Lang::Vi, now, lunar, Some(&hue)).contains("Huế: chưa có thời tiết"));
        let offline = PlaceWeather { problem: Some(no_network(Lang::Vi)), ..hue };
        assert_eq!(now_text(Lang::Vi, now, lunar, Some(&offline)), "08:30 · Thứ Ba 17/02\nÂm lịch 1/1\nKhông có mạng :))");
    }

    #[test]
    fn thang_nhuan_ghi_ro() {
        let now = date(2025, 7, 25).at(8, 30, 0, 0);
        assert!(now_text(Lang::Vi, now, to_lunar(now.date()), None).contains("Âm lịch 1/6 nhuận"));
    }

    #[test]
    fn tieng_anh_ten_thu_thang_va_thoi_tiet_bang_tieng_anh() {
        let now = date(2026, 10, 1).at(15, 4, 0, 0);
        let weather =
            PlaceWeather { place: "Hanoi".into(), sky: Some(Sky::Rain), temperature: Some(26.6), problem: None };
        assert_eq!(
            now_text(Lang::En, now, to_lunar(now.date()), Some(&weather)),
            "15:04 · Thursday, Oct 1\nLunar date 8/21\nHanoi: rainy, 27°C"
        );
        let hue = PlaceWeather { place: "Hue".into(), sky: None, temperature: None, problem: Some(no_network(Lang::En)) };
        assert_eq!(weather_line(Lang::En, &hue), "No internet :))");
    }

    #[test]
    fn dip_co_san_theo_ngon_ngu_dip_da_sua_thi_nhu_nguoi_dung_ghi() {
        let custom = Occasion {
            name: "Sinh nhật".into(),
            day: 17,
            month: 2,
            lunar: false,
            days: 1,
            message: "Chúc mừng :)))".into(),
            enabled: true,
            preset: None,
        };
        assert_eq!(occasion_text(&custom, Lang::En), ("Sinh nhật".into(), "Chúc mừng :)))".into()));
        let missing = Occasion { preset: Some("khongco".into()), ..custom };
        assert_eq!(occasion_text(&missing, Lang::En).0, "Sinh nhật");
    }
}
