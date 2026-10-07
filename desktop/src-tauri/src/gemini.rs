//! Gọi Gemini như trang gemini.google.com lúc chưa đăng nhập, cho chat với pet (Phase 6): không cần key,
//! cookie hay tài khoản, không chạy server, không mở cổng.
//!
//! Chọn model bằng header `x-goog-ext-525001261-jspb` (mã model + loại model) như trang Gemini. Build label
//! của trang và mã model Flash đọc từ chính trang (vài tiếng một lần, chỉ khi có người chat), nên Google lên
//! bản mới thì app tự theo, không phải cập nhật app. Chưa đăng nhập thì Google cho Flash mới nhất của gói
//! miễn phí (lúc viết là 3.6 Flash); Pro và Flash của gói trả phí thì Google tự đổi về bản nhẹ hơn.
//!
//! Mỗi lần `chat` gửi đúng một request, không tự thử lại: `chat.rs` lo giới hạn tần suất. Gửi dạng chat tạm
//! (không vào lịch sử Gemini).

use serde::Serialize;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const APP_URL: &str = "https://gemini.google.com/app";
const GENERATE_URL: &str =
    "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate";
/// Trang Gemini chỉ trả lời trình duyệt.
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0 Safari/537.36";
const MODEL_HEADER: &str = "x-goog-ext-525001261-jspb";
/// Đọc lại trang Gemini sau chừng này.
const PAGE_TTL: Duration = Duration::from_secs(6 * 3600);
/// Đọc trang lỗi (mất mạng...) thì chừng này sau mới đọc lại, trong lúc đó dùng giá trị dự phòng.
const PAGE_RETRY: Duration = Duration::from_secs(30 * 60);
const PAGE_TIMEOUT: Duration = Duration::from_secs(15);
/// Dự phòng khi chưa đọc được trang: build label và mã Flash lúc viết (10/2026).
const FALLBACK_BL: &str = "boq_assistant-bard-web-server_20260929.03_p0";
const FALLBACK_MODEL: &str = "fbb127bbb056c959";
/// Loại model trong JS của trang (MODE_CATEGORY): 1 là Flash, trả lời nhanh.
const MODE_FAST: u8 = 1;
/// Mức suy nghĩ của Flash như trang Gemini gửi.
const THINK_FAST: u8 = 4;
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

/// Build label và mã model đọc từ trang Gemini.
#[derive(Debug, Clone, PartialEq)]
struct Page {
    bl: String,
    model: String,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            bl: FALLBACK_BL.to_string(),
            model: FALLBACK_MODEL.to_string(),
        }
    }
}

struct Cached {
    page: Page,
    /// Đọc lại sau lúc này.
    until: Instant,
}

/// Câu trả lời, câu hỏi tiếp trang Gemini gợi ý và tên model Google đã dùng (như trang Gemini ghi, ví dụ
/// "3.6 Flash").
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    pub text: String,
    pub follow_ups: Vec<FollowUp>,
    pub model: Option<String>,
}

/// Câu hỏi tiếp trang Gemini gợi ý dưới câu trả lời (thẻ `<Elicitation>`): nút ghi `label`, bấm thì gửi
/// `query`. Khớp `FollowUp` trong packages/core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FollowUp {
    pub label: String,
    pub query: String,
}

pub struct Gemini {
    http: reqwest::Client,
    page: Mutex<Option<Cached>>,
}

impl Gemini {
    /// `timeout`: chờ Google trả lời tối đa chừng này.
    pub fn new(timeout: Duration) -> Self {
        let http = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent(USER_AGENT)
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Không tạo được HTTP client cho chat: {e}");
                reqwest::Client::new()
            });
        Self {
            http,
            page: Mutex::new(None),
        }
    }

    /// Gửi `prompt`, chờ cả câu trả lời.
    pub async fn chat(&self, prompt: &str) -> Result<Reply, GeminiError> {
        let page = self.page().await;
        let response = self
            .http
            .post(GENERATE_URL)
            .query(&[
                ("bl", page.bl.as_str()),
                ("hl", "en"),
                ("_reqid", &(now_secs() % 1_000_000).to_string()),
                ("rt", "c"),
            ])
            .header("Origin", "https://gemini.google.com")
            .header("Referer", "https://gemini.google.com/app")
            .header("X-Same-Domain", "1")
            .header(MODEL_HEADER, model_header(&page.model))
            .form(&[("f.req", request_body(prompt))])
            .send()
            .await?;
        let status = response.status();
        if status.as_u16() == 429 {
            return Err(GeminiError::RateLimited);
        }
        if !status.is_success() {
            // Hay gặp khi build label đã cũ: lần sau đọc lại trang.
            self.forget_page().await;
            return Err(GeminiError::Status(status.as_u16()));
        }
        let reply = parse_reply(&response.text().await?);
        if reply.text.is_empty() {
            self.forget_page().await;
            return Err(GeminiError::Empty);
        }
        Ok(reply)
    }

    /// Build label và mã model: đọc trang Gemini nếu chưa đọc hay đã lâu, đọc lỗi thì dùng dự phòng.
    async fn page(&self) -> Page {
        let mut cached = self.page.lock().await;
        if let Some(c) = cached.as_ref().filter(|c| Instant::now() < c.until) {
            return c.page.clone();
        }
        let (page, ttl) = match self.fetch_page().await {
            Some(page) => (page, PAGE_TTL),
            None => (Page::default(), PAGE_RETRY),
        };
        *cached = Some(Cached {
            page: page.clone(),
            until: Instant::now() + ttl,
        });
        page
    }

    async fn forget_page(&self) {
        *self.page.lock().await = None;
    }

    async fn fetch_page(&self) -> Option<Page> {
        let html = self
            .http
            .get(APP_URL)
            .header("Accept-Language", "en-US,en;q=0.9")
            .timeout(PAGE_TIMEOUT)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .ok()?
            .text()
            .await
            .ok()?;
        let page = parse_page(&html);
        if page.is_none() {
            eprintln!("Không đọc được build label trên trang Gemini, dùng giá trị dự phòng.");
        }
        page
    }
}

/// Header chọn model, như trang Gemini gửi khi chọn Flash.
fn model_header(model: &str) -> String {
    format!("[1,null,null,null,\"{model}\",null,null,0,[4,5,6,8],null,null,1,null,null,{MODE_FAST}]")
}

/// Giá trị `f.req`: mảng 102 phần tử như trang Gemini, chỉ điền những ô trang điền khi chat chữ.
fn request_body(prompt: &str) -> String {
    use serde_json::{json, Value};
    let mut inner = vec![Value::Null; 102];
    inner[0] = json!([prompt, 0, null, null, null, null, 0]);
    inner[1] = json!(["en"]);
    inner[2] = json!(["", "", "", null, null, null, null, null, null, ""]);
    inner[6] = json!([0]);
    inner[7] = json!(1);
    inner[10] = json!(1);
    inner[11] = json!(0);
    inner[17] = json!([[THINK_FAST]]);
    inner[18] = json!(0);
    inner[27] = json!(1);
    inner[30] = json!([4]);
    // Chat tạm: không vào lịch sử Gemini.
    inner[41] = json!([1]);
    inner[45] = json!(1);
    inner[53] = json!(0);
    inner[59] = json!(random_id());
    inner[61] = json!([]);
    inner[68] = json!(1);
    inner[79] = json!(MODE_FAST);
    json!([null, Value::Array(inner).to_string()]).to_string()
}

/// Build label (`boq_assistant-bard-web-server_20260929.03_p0`) và mã Flash (mã cuối trong danh sách
/// "fast=..." của trang) từ HTML của trang Gemini. Không có build label thì `None`; không có danh sách thì
/// dùng mã dự phòng.
fn parse_page(html: &str) -> Option<Page> {
    const BL: &str = "boq_assistant-bard-web-server_";
    let start = html.find(BL)?;
    let rest = &html[start + BL.len()..];
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_'))
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let bl = format!("{BL}{}", &rest[..end]);
    let model = fast_models(html).pop().unwrap_or_else(|| FALLBACK_MODEL.to_string());
    Some(Page { bl, model })
}

/// Các mã model trong danh sách "fast=id1,id2" của trang (dấu `=` có thể bị escape thành `=`).
fn fast_models(html: &str) -> Vec<String> {
    for (at, _) in html.match_indices("fast") {
        let rest = html[at + 4..].trim_start_matches('\\');
        let Some(list) = rest.strip_prefix("u003d").or_else(|| rest.strip_prefix('=')) else {
            continue;
        };
        let ids: Vec<String> = list
            .split(',')
            .map(|part| part.chars().take_while(char::is_ascii_hexdigit).collect::<String>())
            .take_while(|id| id.len() == 16)
            .collect();
        if !ids.is_empty() {
            return ids;
        }
    }
    Vec::new()
}

/// Câu trả lời trong body của StreamGenerate: các dòng `wrb.fr` gửi dần câu trả lời dài dần, lấy bản dài
/// nhất. Tên model Google đã dùng ở ô 42.
fn parse_reply(raw: &str) -> Reply {
    let mut text = String::new();
    let mut model = None;
    for line in raw.lines() {
        if !line.contains("\"wrb.fr\"") {
            continue;
        }
        let Some(inner) = serde_json::from_str::<serde_json::Value>(line)
            .ok()
            .and_then(|outer| outer.get(0)?.get(2)?.as_str().map(str::to_owned))
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        else {
            continue;
        };
        if let Some(name) = inner.get(42).and_then(|v| v.as_str()) {
            model = Some(name.to_string());
        }
        let candidates = inner.get(4).and_then(|v| v.as_array()).into_iter().flatten();
        for part in candidates.filter_map(|c| c.get(1)?.as_array()).flatten() {
            if let Some(s) = part.as_str().filter(|s| s.len() > text.len()) {
                text = s.to_string();
            }
        }
    }
    let (text, follow_ups) = clean(&text);
    Reply {
        text,
        follow_ups,
        model,
    }
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
    for line in text.split_inclusive('\n') {
        let is_fence = line.trim_start().starts_with("```");
        if fence || is_fence {
            out.push_str(&strip_prose(&prose, follow_ups));
            prose.clear();
            out.push_str(line);
            fence ^= is_fence;
        } else {
            prose.push_str(line);
        }
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

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Mã ngẫu nhiên dạng UUID v4 cho mỗi lần gửi, như trang Gemini.
fn random_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let mut bytes = [0u8; 16];
    for (i, chunk) in bytes.chunks_mut(8).enumerate() {
        // Mỗi RandomState có khoá ngẫu nhiên riêng.
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u128(nanos);
        hasher.write_usize(i);
        chunk.copy_from_slice(&hasher.finish().to_le_bytes());
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_build_label_va_ma_flash_tu_trang_gemini() {
        // Trích đúng kiểu escape trong HTML thật của trang.
        let html = r#"..."cfb2h":"boq_assistant-bard-web-server_20260929.03_p0","dLc0B":false...
[45734007,null,null,null,null,null,\"m3eQte\",[\"[[\\\"thinking\\\\u003de6fa609c3fa255c0,9d8ca3786ebdfbea\\\",\\\"fast\\\\u003d56fdd199312815e2,fbb127bbb056c959\\\"]]\"]]"#;
        assert_eq!(
            parse_page(html),
            Some(Page {
                bl: "boq_assistant-bard-web-server_20260929.03_p0".into(),
                model: "fbb127bbb056c959".into(),
            })
        );
        assert_eq!(fast_models("fast=0123456789abcdef"), vec!["0123456789abcdef".to_string()]);
        // Không thấy danh sách thì dùng mã dự phòng; không thấy build label thì coi như không đọc được.
        assert_eq!(
            parse_page("x boq_assistant-bard-web-server_20261101.01_p1\" y").map(|p| p.model),
            Some(FALLBACK_MODEL.to_string())
        );
        assert_eq!(parse_page("<html>lỗi</html>"), None);
    }

    fn line(model: &str, text: &str) -> String {
        let inner = serde_json::json!([null, ["c_1", "r_1"], null, null, [["rc_1", [text]]]]);
        let mut inner = inner.as_array().unwrap().clone();
        inner.resize(43, serde_json::Value::Null);
        inner[42] = serde_json::json!(model);
        let outer = serde_json::json!([["wrb.fr", null, serde_json::Value::Array(inner).to_string()]]);
        outer.to_string()
    }

    #[test]
    fn lay_ban_tra_loi_dai_nhat_va_ten_model() {
        let raw = format!(
            ")]}}'\n\n123\n{}\n456\n{}\n[[\"di\",80]]\n",
            line("3.6 Flash", "Xin"),
            line("3.6 Flash", "Xin chào! Xem http://googleusercontent.com/card_content/0 nhé")
        );
        assert_eq!(
            parse_reply(&raw),
            Reply {
                text: "Xin chào! Xem  nhé".into(),
                follow_ups: Vec::new(),
                model: Some("3.6 Flash".into()),
            }
        );
        assert_eq!(parse_reply("<html>").text, "");
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
    fn goi_dung_model_flash_chat_tam() {
        assert_eq!(
            model_header("fbb127bbb056c959"),
            r#"[1,null,null,null,"fbb127bbb056c959",null,null,0,[4,5,6,8],null,null,1,null,null,1]"#
        );
        let body: serde_json::Value = serde_json::from_str(&request_body("Chào")).unwrap();
        let inner: serde_json::Value = serde_json::from_str(body[1].as_str().unwrap()).unwrap();
        assert_eq!(inner.as_array().unwrap().len(), 102);
        assert_eq!(inner[0][0], "Chào");
        assert_eq!(inner[41], serde_json::json!([1]));
        assert_eq!(inner[79], 1);
        let id = random_id();
        assert_eq!(id.len(), 36);
        assert_ne!(id, random_id());
    }
}
