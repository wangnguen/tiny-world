//! Ngôn ngữ của app: tiếng Việt hoặc tiếng Anh. Mọi chữ nằm trong `packages/core/src/i18n/vi.json` và
//! `en.json`, dùng chung với frontend; Rust nhúng hai file đó lúc build và chỉ lấy chữ do chính nó hiện (menu
//! khay, tiêu đề cửa sổ chat, câu báo lỗi trả về frontend, câu nhờ Gemini viết gợi ý).
//!
//! Người dùng chọn trong Cài đặt (`Language`); mặc định theo ngôn ngữ hiển thị của Windows: tiếng Việt thì
//! tiếng Việt, còn lại là tiếng Anh.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

static VI: LazyLock<Value> = LazyLock::new(|| parse(include_str!("../../../packages/core/src/i18n/vi.json")));
static EN: LazyLock<Value> = LazyLock::new(|| parse(include_str!("../../../packages/core/src/i18n/en.json")));

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("file chữ trong packages/core/src/i18n không phải JSON hợp lệ")
}

/// Ngôn ngữ chọn trong Cài đặt, khớp `Language` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// Theo ngôn ngữ hiển thị của Windows.
    #[default]
    Auto,
    Vi,
    En,
}

/// File sửa tay ghi giá trị lạ thì coi như theo Windows, không bỏ cả file cài đặt.
impl<'de> Deserialize<'de> for Language {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "vi" => Self::Vi,
            "en" => Self::En,
            _ => Self::Auto,
        })
    }
}

impl Language {
    /// Ngôn ngữ thật sự dùng: `Auto` thì theo Windows.
    pub fn resolve(self) -> Lang {
        match self {
            Self::Auto => system(),
            Self::Vi => Lang::Vi,
            Self::En => Lang::En,
        }
    }
}

/// Ngôn ngữ đang dùng, khớp `Lang` trong packages/core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Vi,
    En,
}

impl Lang {
    fn messages(self) -> &'static Value {
        match self {
            Self::Vi => &VI,
            Self::En => &EN,
        }
    }

    /// Chữ ở mục `key` (ví dụ "tray.pause"). Thiếu mục là lỗi của file chữ: bản debug dừng luôn để test bắt
    /// được, bản phát hành hiện tạm tên mục.
    pub fn t(self, key: &str) -> String {
        let node = key.split('.').fold(self.messages(), |node, part| &node[part]);
        match node.as_str() {
            Some(text) => text.to_owned(),
            None => {
                debug_assert!(false, "thiếu chữ {key} ({self:?})");
                key.to_owned()
            }
        }
    }

    /// Như `t`, thay `{tên}` trong chữ bằng giá trị trong `params`.
    pub fn tf(self, key: &str, params: &[(&str, &dyn Display)]) -> String {
        params
            .iter()
            .fold(self.t(key), |text, (name, value)| text.replace(&format!("{{{name}}}"), &value.to_string()))
    }
}

/// `true` là tiếng Anh. Đặt lúc mở app và mỗi lần đổi cài đặt (`set`).
static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set(lang: Lang) {
    ENGLISH.store(lang == Lang::En, Ordering::Relaxed);
}

pub fn current() -> Lang {
    if ENGLISH.load(Ordering::Relaxed) {
        Lang::En
    } else {
        Lang::Vi
    }
}

/// Chữ ở mục `key` theo ngôn ngữ đang dùng.
pub fn t(key: &str) -> String {
    current().t(key)
}

/// Như `t`, kèm giá trị cho các chỗ `{tên}`.
pub fn tf(key: &str, params: &[(&str, &dyn Display)]) -> String {
    current().tf(key, params)
}

/// Ngôn ngữ hiển thị của Windows: tiếng Việt hay không.
#[cfg(windows)]
fn system() -> Lang {
    use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;
    /// Mã ngôn ngữ chính của tiếng Việt (LANG_VIETNAMESE), 10 bit thấp của LANGID.
    const VIETNAMESE: u16 = 0x2a;
    // SAFETY: hàm không nhận tham số, chỉ đọc cài đặt của người dùng hiện tại.
    let id = unsafe { GetUserDefaultUILanguage() };
    if id & 0x3ff == VIETNAMESE {
        Lang::Vi
    } else {
        Lang::En
    }
}

#[cfg(not(windows))]
fn system() -> Lang {
    Lang::En
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gia_tri_la_thi_theo_windows() {
        let parse = |json: &str| serde_json::from_str::<Language>(json).unwrap();
        assert_eq!(parse(r#""vi""#), Language::Vi);
        assert_eq!(parse(r#""en""#), Language::En);
        assert_eq!(parse(r#""auto""#), Language::Auto);
        assert_eq!(parse(r#""fr""#), Language::Auto);
        assert_eq!(serde_json::to_string(&Language::En).unwrap(), r#""en""#);
        assert_eq!(Language::Vi.resolve(), Lang::Vi);
        assert_eq!(Language::En.resolve(), Lang::En);
    }

    #[test]
    fn lay_chu_theo_muc_va_dien_gia_tri() {
        assert_eq!(Lang::Vi.t("tray.quit"), "Thoát");
        assert_eq!(Lang::En.t("tray.quit"), "Quit");
        assert_eq!(Lang::En.tf("chat.title", &[("name", &"Momo")]), "Chat with Momo");
        assert_eq!(Lang::Vi.tf("time.seconds", &[("n", &3)]), "3 giây");
    }

    /// Hai file chữ có đúng các mục giống nhau, mảng dài bằng nhau (test này cũng có ở packages/core).
    #[test]
    fn hai_file_chu_co_du_cac_muc_giong_nhau() {
        fn same(vi: &Value, en: &Value, path: &str) {
            match (vi, en) {
                (Value::Object(a), Value::Object(b)) => {
                    let keys = |o: &serde_json::Map<String, Value>| o.keys().cloned().collect::<Vec<_>>();
                    assert_eq!(keys(a), keys(b), "các mục trong {path}");
                    for (key, value) in a {
                        same(value, &b[key], &format!("{path}.{key}"));
                    }
                }
                (Value::Array(a), Value::Array(b)) => {
                    assert_eq!(a.len(), b.len(), "số phần tử của {path}");
                    for (i, (x, y)) in a.iter().zip(b).enumerate() {
                        same(x, y, &format!("{path}[{i}]"));
                    }
                }
                (Value::String(_), Value::String(_)) => {}
                _ => panic!("{path}: hai file khác kiểu"),
            }
        }
        same(&VI, &EN, "");
    }
}
