//! Gọi Gemini qua endpoint mà trang gemini.google.com dùng lúc chưa đăng nhập, cho chat với pet (Phase 6):
//! không cần key, cookie hay tài khoản, không chạy server, không mở cổng.
//!
//! Chỉ gửi phần tối thiểu Google cần: một POST với `f.req` là câu hỏi, không header hay tham số nào khác,
//! không giả trình duyệt, không đọc trang Gemini. Trang thật gửi thêm nhiều thứ (build label, User-Agent,
//! header chọn model, body 102 ô...), và chính những thứ đó hay hỏng khi Google đổi trang: 13/10/2026 header
//! chọn 3.6 Flash làm mọi câu trả lời thành câu báo lỗi "Sorry, something went wrong", và build label đổi tên
//! server. Không chọn model thì Google dùng model mặc định cho người chưa đăng nhập (lúc viết là 3.5
//! Flash-Lite).
//!
//! Mỗi lần `chat` gửi đúng một request, không tự thử lại: `chat.rs` lo giới hạn tần suất. Câu trả lời có ảnh
//! Google tìm được thì `thumbnail` tải ảnh thu nhỏ từ máy chủ ảnh của Google, không tải từ trang có ảnh.

use serde::Serialize;
use std::time::Duration;

const GENERATE_URL: &str =
    "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate";
/// Thẻ HTML không có thẻ đóng.
const VOID_TAGS: [&str; 3] = ["br", "hr", "img"];
/// Thẻ HTML bỏ đi thì thay bằng xuống dòng, để chữ hai bên không dính nhau; thẻ khác bỏ hẳn.
const BREAK_TAGS: [&str; 6] = ["br", "hr", "p", "div", "li", "tr"];
/// Dài hơn chừng này byte thì không coi là thẻ.
const TAG_MAX: usize = 4096;
/// Câu hỏi tiếp dưới câu trả lời: tối đa chừng này nút; chữ trên nút và câu gửi đi tối đa chừng này ký tự
/// (câu gửi đi khớp `MESSAGE_MAX` trong desktop/src/chat/prompt.ts).
const FOLLOW_UP_MAX: usize = 4;
const LABEL_MAX_CHARS: usize = 80;
const QUERY_MAX_CHARS: usize = 1000;
/// Ảnh kèm câu trả lời: tối đa chừng này ảnh, tiêu đề tối đa chừng này ký tự; tải ảnh thu nhỏ chờ tối đa
/// chừng này, lớn tối đa chừng này byte, chỉ nhận các kiểu này.
const IMAGE_MAX: usize = 4;
const TITLE_MAX_CHARS: usize = 120;
const IMAGE_TIMEOUT: Duration = Duration::from_secs(8);
const IMAGE_MAX_BYTES: usize = 1024 * 1024;
const IMAGE_TYPES: [&str; 4] = ["image/jpeg", "image/png", "image/webp", "image/gif"];

/// Vì sao không có câu trả lời.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiError {
    /// Không tới được Google: mất mạng, DNS, kết nối bị ngắt.
    Offline,
    /// Google không trả lời kịp.
    Timeout,
    /// Google báo gửi nhiều quá (HTTP 429).
    RateLimited,
    /// Google trả mã lỗi HTTP khác.
    Status(u16),
    /// Google trả lời mà không có chữ (thường là bị chặn nhẹ hoặc trang đã đổi cách gửi).
    Empty,
}

impl std::fmt::Display for GeminiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeminiError::Offline => write!(f, "mất mạng"),
            GeminiError::Timeout => write!(f, "hết giờ chờ"),
            GeminiError::RateLimited => write!(f, "HTTP 429"),
            GeminiError::Status(code) => write!(f, "HTTP {code}"),
            GeminiError::Empty => write!(f, "trả lời rỗng"),
        }
    }
}

impl From<reqwest::Error> for GeminiError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            GeminiError::Timeout
        } else if let Some(status) = e.status() {
            GeminiError::Status(status.as_u16())
        } else {
            GeminiError::Offline
        }
    }
}

/// Câu trả lời, câu hỏi tiếp trang Gemini gợi ý, ảnh Google tìm được và tên model Google đã dùng (như trang
/// Gemini ghi, ví dụ "3.5 Flash-Lite").
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub text: String,
    pub follow_ups: Vec<FollowUp>,
    pub images: Vec<Image>,
    pub model: Option<String>,
}

/// Ảnh Google tìm được cho câu trả lời (trang Gemini vẽ ảnh vào chỗ link `lmdx_content` mà `clean` bỏ đi).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Ảnh thu nhỏ trên máy chủ ảnh của Google (`https://encrypted-tbn….gstatic.com/…`) và cỡ của nó.
    pub thumbnail: String,
    pub width: u32,
    pub height: u32,
    pub title: String,
    /// Trang có ảnh (http/https) và tên trang ("wikiHow"; không có thì là tên miền).
    pub page: String,
    pub site: String,
}

/// Câu hỏi tiếp trang Gemini gợi ý dưới câu trả lời (thẻ `<Elicitation>`): nút ghi `label`, bấm thì gửi
/// `query`. Khớp `FollowUp` trong packages/core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FollowUp {
    pub label: String,
    pub query: String,
}

#[derive(Clone)]
pub struct Gemini {
    http: reqwest::Client,
}

impl Gemini {
    /// `timeout`: chờ Google trả lời tối đa chừng này.
    pub fn new(timeout: Duration) -> Self {
        let http = reqwest::Client::builder().timeout(timeout).build().unwrap_or_else(|e| {
            log!("Không tạo được HTTP client cho chat: {e}");
            reqwest::Client::new()
        });
        Self { http }
    }

    /// Gửi `prompt`, chờ cả câu trả lời.
    pub async fn chat(&self, prompt: &str) -> Result<Reply, GeminiError> {
        let response = self
            .http
            .post(GENERATE_URL)
            .form(&[("f.req", request_body(prompt))])
            .send()
            .await?;
        let status = response.status();
        if status.as_u16() == 429 {
            return Err(GeminiError::RateLimited);
        }
        if !status.is_success() {
            return Err(GeminiError::Status(status.as_u16()));
        }
        let reply = parse_reply(&response.text().await?);
        if reply.text.is_empty() {
            return Err(GeminiError::Empty);
        }
        Ok(reply)
    }

    /// Tải ảnh thu nhỏ của `image`: trả về kiểu ảnh ("image/jpeg") và nội dung; lỗi, lớn quá hay không phải
    /// ảnh thì `None`.
    pub async fn thumbnail(&self, image: &Image) -> Option<(&'static str, Vec<u8>)> {
        let response = self.http.get(&image.thumbnail).timeout(IMAGE_TIMEOUT).send().await.ok()?;
        if !response.status().is_success() || response.content_length().is_some_and(|n| n > IMAGE_MAX_BYTES as u64) {
            return None;
        }
        let kind = {
            let header = response.headers().get(reqwest::header::CONTENT_TYPE)?.to_str().ok()?;
            let mime = header.split(';').next().unwrap_or_default().trim();
            IMAGE_TYPES.into_iter().find(|t| mime.eq_ignore_ascii_case(t))?
        };
        let bytes = response.bytes().await.ok()?;
        (bytes.len() <= IMAGE_MAX_BYTES).then(|| (kind, bytes.to_vec()))
    }
}

/// Giá trị `f.req`: `[null, "[[câu hỏi],null,null]"]`, kiểu gửi tối giản có từ thời Bard.
fn request_body(prompt: &str) -> String {
    use serde_json::json;
    json!([null, json!([[prompt], null, null]).to_string()]).to_string()
}

/// Câu trả lời trong body của StreamGenerate: các mục `wrb.fr` gửi dần câu trả lời dài dần, lấy bản dài
/// nhất. Ảnh ở ô 12 của câu trả lời (lấy danh sách gửi sau cùng), tên model Google đã dùng ở ô 42. Đọc mọi
/// mục trên mọi dòng, nên Google gửi mỗi dòng một mục (kiểu `rt=c`) hay cả mảng một dòng đều đọc được.
fn parse_reply(raw: &str) -> Reply {
    let mut text = String::new();
    let mut images = Vec::new();
    let mut model = None;
    let entries = raw
        .lines()
        .filter(|line| line.contains("\"wrb.fr\""))
        .filter_map(|line| serde_json::from_str::<Vec<serde_json::Value>>(line).ok())
        .flatten();
    for entry in entries {
        let Some(inner) = entry
            .get(2)
            .and_then(|v| v.as_str())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        else {
            continue;
        };
        if let Some(name) = inner.get(42).and_then(|v| v.as_str()) {
            model = Some(name.to_string());
        }
        for candidate in inner.get(4).and_then(|v| v.as_array()).into_iter().flatten() {
            for part in candidate.get(1).and_then(|v| v.as_array()).into_iter().flatten() {
                if let Some(s) = part.as_str().filter(|s| s.len() > text.len()) {
                    text = s.to_string();
                }
            }
            let found = parse_images(candidate.get(12));
            if !found.is_empty() {
                images = found;
            }
        }
    }
    let (text, follow_ups) = clean(&text);
    Reply {
        text,
        follow_ups,
        images,
        model,
    }
}

/// Ảnh ở ô 12 của câu trả lời: `[null, [ảnh, …]]`, mỗi ảnh `[[[link gốc], null, rộng, cao], [[trang], tiêu
/// đề, _, _, _, tên trang], null, [[link thu nhỏ], null, rộng, cao], _, _, _, [_, _, chữ thay ảnh]]`. Chỉ lấy
/// ảnh có ảnh thu nhỏ trên máy chủ ảnh của Google và trang http/https, bỏ ảnh trùng.
fn parse_images(cell: Option<&serde_json::Value>) -> Vec<Image> {
    fn link(v: Option<&serde_json::Value>) -> Option<&str> {
        v?.get(0)?.get(0)?.as_str()
    }
    fn text(v: Option<&serde_json::Value>) -> Option<&str> {
        v?.as_str().map(str::trim).filter(|s| !s.is_empty())
    }
    let mut out: Vec<Image> = Vec::new();
    for item in cell.and_then(|c| c.get(1)).and_then(|v| v.as_array()).into_iter().flatten() {
        let (thumb, source) = (item.get(3), item.get(1));
        let Some(thumbnail) = link(thumb).filter(|url| is_google_thumbnail(url)) else {
            continue;
        };
        let Some((page, host)) = link(source).and_then(|url| Some((url, web_host(url)?))) else {
            continue;
        };
        if out.iter().any(|known| known.thumbnail == thumbnail) {
            continue;
        }
        let size = |i: usize| thumb.and_then(|t| t.get(i)?.as_u64()).and_then(|n| u32::try_from(n).ok()).unwrap_or(0);
        let title = text(source.and_then(|s| s.get(1)))
            .or_else(|| text(item.get(7).and_then(|t| t.get(2))))
            .unwrap_or_default();
        out.push(Image {
            thumbnail: thumbnail.to_string(),
            width: size(2),
            height: size(3),
            title: title.chars().take(TITLE_MAX_CHARS).collect(),
            page: page.to_string(),
            site: text(source.and_then(|s| s.get(5)))
                .unwrap_or_else(|| host.strip_prefix("www.").unwrap_or(host))
                .to_string(),
        });
        if out.len() == IMAGE_MAX {
            break;
        }
    }
    out
}

/// Ảnh thu nhỏ trên máy chủ ảnh của Google: `https://encrypted-tbn0.gstatic.com/images?q=…`.
fn is_google_thumbnail(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let host = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    host.starts_with("encrypted-tbn")
        && host.ends_with(".gstatic.com")
        && host.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-')
}

/// Tên miền của link http/https ("www.wikihow.com"); không phải link http/https thì `None`.
fn web_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let host = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    let ok = !host.is_empty() && host.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-');
    ok.then_some(host)
}

/// Bỏ chỗ trang Gemini chèn link nội bộ thay cho thẻ (bản đồ, ảnh...): người dùng không mở được. Bỏ cả thẻ
/// kiểu HTML (`strip_tags`). Trả về chữ còn lại và các câu hỏi tiếp lấy từ thẻ.
fn clean(text: &str) -> (String, Vec<FollowUp>) {
    const CARD: &str = "http://googleusercontent.com/";
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(CARD) {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let end = tail.find(char::is_whitespace).unwrap_or(tail.len());
        rest = &tail[end..];
    }
    out.push_str(rest);
    let mut follow_ups = Vec::new();
    let text = strip_tags(&out, &mut follow_ups);
    (text.trim().to_string(), follow_ups)
}

/// Bỏ thẻ kiểu HTML khỏi câu trả lời, trừ trong code. Trang Gemini chèn `<ElicitationsGroup message=…>` bọc
/// các `<Elicitation label=… query=…/>` để vẽ nút hỏi tiếp: câu `message` giữ lại làm một đoạn, mỗi
/// `<Elicitation>` thành một `FollowUp`. Thẻ HTML như `<br>`, `<b>…</b>` thì bỏ thẻ, giữ chữ. Chỉ đụng tới chỗ
/// chắc là thẻ: thẻ đóng, thẻ tự đóng, thẻ có thuộc tính, `<br>`, hoặc thẻ mở có thẻ đóng phía sau; nên
/// `Vec<String>`, `git clone <url>` giữ nguyên.
fn strip_tags(text: &str, follow_ups: &mut Vec<FollowUp>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prose = String::new();
    let mut fence = false;
    let mut hidden = false;
    for line in text.split_inclusive('\n') {
        let is_fence = line.trim_start().starts_with("```");
        if is_fence && !fence {
            // Code trang Gemini tự chạy để tính (```python?code_reference…) và kết quả (```text?code_stdout…):
            // không phải câu trả lời, bỏ cả khối.
            hidden = line.contains("code_event_index");
        }
        if fence || is_fence {
            if !hidden {
                out.push_str(&strip_prose(&prose, follow_ups));
                prose.clear();
                out.push_str(line);
            }
        } else {
            prose.push_str(line);
        }
        fence ^= is_fence;
        hidden &= fence;
    }
    out.push_str(&strip_prose(&prose, follow_ups));
    out
}

/// `strip_tags` cho đoạn ngoài khối code: giữ `code` trong dòng, thay thẻ, rồi gộp các dòng trống liền nhau
/// (thẻ bỏ đi để lại dòng trống).
fn strip_prose(text: &str, follow_ups: &mut Vec<FollowUp>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(['<', '`']) {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let keep = if rest.starts_with('`') {
            rest[1..]
                .find(['`', '\n'])
                .filter(|&end| rest[1 + end..].starts_with('`'))
                .map_or(1, |end| end + 2)
        } else if let Some(tag) = tag_at(rest) {
            out.push_str(&replace_tag(&tag, follow_ups));
            rest = &rest[tag.len..];
            continue;
        } else {
            1
        };
        out.push_str(&rest[..keep]);
        rest = &rest[keep..];
    }
    out.push_str(rest);
    let mut tidy = String::with_capacity(out.len());
    let mut blank = false;
    for line in out.split_inclusive('\n') {
        let empty = line.trim().is_empty();
        if !(empty && blank) {
            tidy.push_str(if empty { "\n" } else { line });
        }
        blank = empty;
    }
    tidy
}

/// Một thẻ kiểu HTML trong câu trả lời.
struct Tag<'a> {
    /// Tên viết thường: "br", "elicitation".
    name: String,
    /// Thẻ đóng `</tên>`.
    closing: bool,
    /// Phần sau tên, trước `>`: ` label="…" query="…"/`.
    attrs: &'a str,
    /// Độ dài cả thẻ (byte).
    len: usize,
}

/// Thẻ ở đầu `s` (bắt đầu bằng `<`) nếu chắc là thẻ.
fn tag_at(s: &str) -> Option<Tag<'_>> {
    let b = s.as_bytes();
    let closing = b.get(1) == Some(&b'/');
    let start = if closing { 2 } else { 1 };
    if !b.get(start)?.is_ascii_alphabetic() {
        return None;
    }
    let mut i = start;
    while b.get(i).is_some_and(|&c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') {
        i += 1;
    }
    let name_end = i;
    let name = s[start..name_end].to_ascii_lowercase();
    if !b.get(i).is_some_and(|&c| c == b'>' || c == b'/' || c.is_ascii_whitespace()) {
        return None;
    }
    let mut has_attrs = false;
    let mut quote = None;
    let end = loop {
        let c = *b.get(i).filter(|_| i < TAG_MAX)?;
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == b'>' => break i,
            None if c == b'<' => return None,
            None if (c == b'"' || c == b'\'') && b[i - 1] == b'=' => {
                quote = Some(c);
                has_attrs = true;
            }
            None => {}
        }
        i += 1;
    };
    let self_closing = b[end - 1] == b'/';
    let sure = closing
        || self_closing
        || has_attrs
        || VOID_TAGS.contains(&name.as_str())
        || s[end..].to_ascii_lowercase().contains(&format!("</{name}>"));
    sure.then(|| Tag {
        name,
        closing,
        attrs: &s[name_end..end],
        len: end + 1,
    })
}

/// Chữ thay cho thẻ: câu hỏi chung của `<ElicitationsGroup>` thành một đoạn, `<Elicitation>` thành câu hỏi
/// tiếp (không để lại chữ), thẻ xuống dòng thành xuống dòng, thẻ khác bỏ.
fn replace_tag(tag: &Tag, follow_ups: &mut Vec<FollowUp>) -> String {
    let line_break = if BREAK_TAGS.contains(&tag.name.as_str()) { "\n" } else { "" };
    if tag.closing {
        return line_break.to_string();
    }
    match tag.name.as_str() {
        "elicitationsgroup" => attr(tag.attrs, "message").map_or_else(String::new, |message| format!("\n\n{message}\n\n")),
        "elicitation" => {
            let query = attr(tag.attrs, "query").unwrap_or_default();
            let label = attr(tag.attrs, "label").filter(|l| !l.is_empty()).unwrap_or_else(|| query.clone());
            let query = if query.is_empty() { label.clone() } else { query };
            let follow_up = FollowUp {
                label: label.chars().take(LABEL_MAX_CHARS).collect(),
                query: query.chars().take(QUERY_MAX_CHARS).collect(),
            };
            let known = follow_ups.iter().any(|f| f.label == follow_up.label);
            if !follow_up.label.is_empty() && !known && follow_ups.len() < FOLLOW_UP_MAX {
                follow_ups.push(follow_up);
            }
            String::new()
        }
        _ => line_break.to_string(),
    }
}

/// Giá trị thuộc tính `key="…"` (hay `'…'`) trong phần thuộc tính của thẻ, đã đổi `&quot;`... về chữ và gộp
/// khoảng trắng.
fn attr(attrs: &str, key: &str) -> Option<String> {
    let mut rest = attrs;
    loop {
        rest = &rest[rest.find(|c: char| c.is_ascii_alphabetic())?..];
        let name_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(rest.len());
        let name = &rest[..name_end];
        rest = rest[name_end..].trim_start();
        let Some(value) = rest.strip_prefix('=').map(str::trim_start) else {
            continue;
        };
        let Some(quote) = value.chars().next().filter(|&c| c == '"' || c == '\'') else {
            rest = value;
            continue;
        };
        let close = value[1..].find(quote)?;
        if name.eq_ignore_ascii_case(key) {
            let value = unescape(&value[1..1 + close]);
            return Some(value.split_whitespace().collect::<Vec<_>>().join(" "));
        }
        rest = &value[close + 2..];
    }
}

/// `&quot;`, `&#39;`, `&lt;`, `&gt;`, `&amp;` trong giá trị thuộc tính.
fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Một mục `wrb.fr` mang câu trả lời `text` của `model`.
    fn entry(model: &str, text: &str) -> serde_json::Value {
        let inner = serde_json::json!([null, ["c_1", "r_1"], null, null, [["rc_1", [text]]]]);
        let mut inner = inner.as_array().unwrap().clone();
        inner.resize(43, serde_json::Value::Null);
        inner[42] = serde_json::json!(model);
        serde_json::json!(["wrb.fr", null, serde_json::Value::Array(inner).to_string()])
    }

    fn line(model: &str, text: &str) -> String {
        serde_json::json!([entry(model, text)]).to_string()
    }

    #[test]
    fn lay_ban_tra_loi_dai_nhat_va_ten_model() {
        let raw = format!(
            ")]}}'\n\n123\n{}\n456\n{}\n[[\"di\",80]]\n",
            line("3.5 Flash-Lite", "Xin"),
            line("3.5 Flash-Lite", "Xin chào! Xem http://googleusercontent.com/card_content/0 nhé")
        );
        let reply = Reply {
            text: "Xin chào! Xem  nhé".into(),
            follow_ups: Vec::new(),
            images: Vec::new(),
            model: Some("3.5 Flash-Lite".into()),
        };
        assert_eq!(parse_reply(&raw), reply);
        // Không có `rt=c` thì Google gửi cả mảng trên một dòng.
        let one_line = serde_json::json!([
            entry("3.5 Flash-Lite", "Xin"),
            entry("3.5 Flash-Lite", "Xin chào! Xem http://googleusercontent.com/card_content/0 nhé"),
            ["di", 80]
        ]);
        assert_eq!(parse_reply(&format!(")]}}'\n\n{one_line}\n")), reply);
        assert_eq!(parse_reply("<html>").text, "");
    }

    #[test]
    fn lay_anh_thu_nho_cua_google_kem_trang_co_anh() {
        use serde_json::json;
        // Một ảnh trong ô 12, như trang Gemini nhận (14/10/2026).
        let image = |thumb: &str, page: &str, site: serde_json::Value| {
            json!([
                [["https://www.wikihow.com/images/3/33/Chiikawa.jpg"], null, 3200, 3073],
                [[page], "Chiikawa Characters Guide", 7, null, null, site],
                null,
                [[thumb], null, 565, 543],
                null, null, null,
                ["http://googleusercontent.com/image_agent_tag_1", null, "Chiikawa and friends"]
            ])
        };
        let tbn = "https://encrypted-tbn0.gstatic.com/images?q=tbn:abc&s=10";
        let mut inner = json!([null, ["c_1", "r_1"], null, null, [["rc_1", ["Đây nè:\n\nhttp://googleusercontent.com/lmdx_content/x"]]]]);
        let candidate = &mut inner[4][0];
        candidate.as_array_mut().unwrap().resize(13, serde_json::Value::Null);
        candidate[12] = json!([null, [
            image(tbn, "https://www.wikihow.com/Chiikawa-Characters", json!("wikiHow")),
            // Trùng ảnh thu nhỏ, ảnh thu nhỏ không phải của Google, trang không phải http/https: bỏ.
            image(tbn, "https://example.com/a", json!("Khác")),
            image("https://evil.com@encrypted-tbn0.gstatic.com/x", "https://example.com/b", json!(null)),
            image("https://encrypted-tbn1.gstatic.com/images?q=2", "javascript:alert(1)", json!(null)),
            // Không có tên trang thì lấy tên miền.
            image("https://encrypted-tbn1.gstatic.com/images?q=3", "https://www.example.com/c", json!(null)),
        ]]);
        let raw = serde_json::json!([["wrb.fr", null, inner.to_string()]]).to_string();
        let reply = parse_reply(&raw);
        assert_eq!(reply.text, "Đây nè:");
        let images: Vec<_> = reply.images.iter().map(|i| (i.thumbnail.as_str(), i.page.as_str(), i.site.as_str())).collect();
        assert_eq!(
            images,
            [
                (tbn, "https://www.wikihow.com/Chiikawa-Characters", "wikiHow"),
                ("https://encrypted-tbn1.gstatic.com/images?q=3", "https://www.example.com/c", "example.com"),
            ]
        );
        assert_eq!((reply.images[0].width, reply.images[0].height), (565, 543));
        assert_eq!(reply.images[0].title, "Chiikawa Characters Guide");
        assert!(!is_google_thumbnail("http://encrypted-tbn0.gstatic.com/images?q=1"));
        assert!(!is_google_thumbnail("https://encrypted-tbn0.gstatic.com.evil.com/x"));
    }

    #[test]
    fn tach_nut_hoi_tiep_cua_trang_gemini() {
        let reply = concat!(
            "Có vài hướng nè.\n\n",
            "<ElicitationsGroup message=\"Bạn muốn tớ gợi ý sâu hơn theo hướng nào?\">\n",
            "<Elicitation label=\"Gợi ý phim\" query=\"Gợi ý vài bộ phim &quot;dài&quot; > 2 tiếng.\"/>\n",
            "<Elicitation label='Kênh YouTube' query=\"Tìm kênh\n  YouTube hay.\" />\n",
            "<Elicitation label=\"Gợi ý phim\" query=\"Trùng chữ trên nút thì bỏ.\"/>\n",
            "<Elicitation query=\"Chỉ có câu hỏi\"/>\n",
            "</ElicitationsGroup>"
        );
        let follow_up = |label: &str, query: &str| FollowUp {
            label: label.into(),
            query: query.into(),
        };
        assert_eq!(
            clean(reply),
            (
                "Có vài hướng nè.\n\nBạn muốn tớ gợi ý sâu hơn theo hướng nào?".to_string(),
                vec![
                    follow_up("Gợi ý phim", "Gợi ý vài bộ phim \"dài\" > 2 tiếng."),
                    follow_up("Kênh YouTube", "Tìm kênh YouTube hay."),
                    follow_up("Chỉ có câu hỏi", "Chỉ có câu hỏi"),
                ]
            )
        );
        assert_eq!(
            clean("<ElicitationsGroup><Elicitation label=\"x\" query=\"y\"/></ElicitationsGroup>"),
            (String::new(), vec![follow_up("x", "y")])
        );
        let many: String = (0..9).map(|i| format!("<Elicitation label=\"Nút {i}\" query=\"Câu {i}\"/>")).collect();
        assert_eq!(clean(&many).1.len(), FOLLOW_UP_MAX);
    }

    #[test]
    fn bo_the_html_giu_chu() {
        let text = |s: &str| clean(s).0;
        assert_eq!(text("A\n\n<Group>\n<Item x='1' />\n</Group>\n\nB"), "A\n\nB");
        assert_eq!(
            text("Dòng 1<br>Dòng 2<BR/>Có <b>đậm</b>, <a href=\"https://x.vn\">link</a> và<sup>1</sup>"),
            "Dòng 1\nDòng 2\nCó đậm, link và1"
        );
    }

    #[test]
    fn giu_nguyen_code_va_chu_trong_ngoac_nhon() {
        let code = concat!(
            "Nút nè:\n",
            "```html\n<button class=\"x\">Bấm</button>\n\n\n<Elicitation label=\"x\" query=\"y\"/>\n```\n",
            "Dùng `<br>` để xuống dòng, `<b>` để in đậm."
        );
        assert_eq!(clean(code), (code.to_string(), Vec::new()));
        let ran = concat!(
            "Tính nè:\n\n",
            "```python?code_reference&code_event_index=1\nprint(2 + 3)\n\n```\n\n",
            "```text?code_stdout&code_event_index=1\n5\n\n```\n\n",
            "Là **5**.\n```py\nprint(5)\n```"
        );
        assert_eq!(clean(ran).0, "Tính nè:\n\nLà **5**.\n```py\nprint(5)\n```");
        for text in [
            "Vec<String> và HashMap<String, i32>",
            "Chạy git clone <url> nhé",
            "a < b > c, x<5 và y>3",
            "Đặt tên <tên file>.txt",
            "Link <https://example.com/>",
        ] {
            assert_eq!(clean(text).0, text);
        }
    }

    #[test]
    fn chi_gui_cau_hoi() {
        assert_eq!(request_body("Chào \"pet\""), r#"[null,"[[\"Chào \\\"pet\\\"\"],null,null]"]"#);
    }

    #[test]
    #[ignore = "cần mạng, gọi Gemini thật"]
    fn gemini_that() {
        tauri::async_runtime::block_on(async {
            let gemini = Gemini::new(Duration::from_secs(60));
            let reply = gemini.chat("What is 2 + 3? Answer with just the number.").await.unwrap();
            println!("{:?} | {}", reply.model, reply.text);
            assert!(reply.text.contains('5') && !reply.text.contains("code_event_index"), "{}", reply.text);
            // Câu cần tra mạng.
            let reply = gemini.chat("Tỷ giá USD/VND hôm nay bao nhiêu? Trả lời một câu.").await.unwrap();
            println!("{:?} | {} | {:?}", reply.model, reply.text, reply.follow_ups);
            // Câu xin ảnh: Google thường kèm ảnh tìm được.
            let reply = gemini.chat("Show me a picture of a red panda.").await.unwrap();
            println!("{} | {:?}", reply.text, reply.images);
            for image in &reply.images {
                let (kind, bytes) = gemini.thumbnail(image).await.unwrap();
                println!("{kind} {} byte", bytes.len());
            }
        });
    }
}
