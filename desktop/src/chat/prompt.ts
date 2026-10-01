/**
 * Câu gửi Gemini khi chat với pet: tính cách ngắn của nhân vật, ngày giờ hiện tại, vài lượt chat gần nhất
 * và câu hỏi. Không gửi gì khác (không gửi tên người dùng, thành phố, cửa sổ đang mở...).
 */

/** Một lượt trong khung chat. */
export interface Turn {
  from: "user" | "pet";
  text: string;
}

/** Nhân vật: tên ngắn và vài chữ mô tả, để Gemini trả lời theo giọng nhân vật. */
export interface Persona {
  name: string;
  about: string;
}

/** Chỉ gửi kèm chừng này lượt gần nhất. */
export const HISTORY_TURNS = 6;
/** Cả câu gửi đi tối đa chừng này ký tự (Rust chặn ở 8000); lượt cũ bị cắt trước. */
export const PROMPT_MAX = 6000;
/** Một câu hỏi tối đa chừng này ký tự (ô nhập). */
export const MESSAGE_MAX = 1000;

const WEEKDAYS = ["Chủ nhật", "Thứ Hai", "Thứ Ba", "Thứ Tư", "Thứ Năm", "Thứ Sáu", "Thứ Bảy"];
const pad = (n: number) => String(n).padStart(2, "0");

/** Tính cách theo tên thư mục pack, khớp bảng nhân vật trong assets/sprite-sources/README.md. */
const PERSONAS: Record<string, Persona> = {
  "a-momo": { name: "Momo", about: "axolotl hồng hiền lành, mang san hô, hay tò mò" },
  "b-bong": { name: "Bông", about: "thỏ kem tai hồng quàng khăn xanh, nhanh nhảu" },
  "b-kitsu": { name: "Kitsu", about: "cáo cam đuôi trắng, lanh lợi" },
  "c-mam": { name: "Mầm", about: "sinh vật rừng màu kem có hai lá trên đầu, dịu dàng" },
  "c-bip": { name: "Bíp", about: "robot xanh mắt LED vàng, thích số liệu chính xác" },
  "c-lumi": { name: "Lumi", about: "tinh linh đầu sao vàng, lạc quan" },
  "c-nam": { name: "Nấm", about: "bé nấm mũ đỏ đốm trắng, chậm rãi" },
  "c-may": { name: "Mây", about: "tinh linh mây xanh nhạt, bay bổng" },
  "c-tan": { name: "Tàn", about: "bé lửa cam nhiệt tình" },
  "c-reu": { name: "Rêu", about: "rồng lá xanh ngọc, hiểu biết" },
  "c-cuc": { name: "Cục", about: "golem đá tím có tinh thể trên đầu, ít nói mà chắc" },
  "c-muc": { name: "Mực", about: "bạch tuộc tím tròn, khéo tay" },
  "c-dua": { name: "Dứa", about: "bé dứa vàng vui tính" },
  "c-su": { name: "Su", about: "phi hành gia nhỏ, mê khám phá" },
  "c-bap": { name: "Bắp", about: "ong vàng tròn chăm chỉ" },
  "c-boggo": { name: "Boggo", about: "ếch coder mắt lúc nào cũng mệt, ôm ly cà phê" },
  "c-gloop": { name: "Gloop", about: "ếch nghịch ngợm quàng khăn cam" },
  "c-bep": { name: "Bẹp", about: "cóc lùn mặt chán đời nhưng tốt bụng" },
  "c-frobu": { name: "Frobu", about: "ếch cú đêm mặc hoodie đeo tai nghe" },
  "c-byte": { name: "Byte", about: "chim cánh cụt coder đeo kính tròn" },
  "c-patch": { name: "Patch", about: "gấu trúc đỏ coder đeo tai nghe tím" },
};

/** Tính cách của pack `pet`; pack mới chưa có trong bảng thì chỉ có tên. */
export function personaOf(pet: string, name: string): Persona {
  return PERSONAS[pet] ?? { name, about: "pet nhỏ dễ thương" };
}

/** "Thứ Năm 01/10/2026" theo giờ máy. */
export function dayText(now: Date): string {
  return `${WEEKDAYS[now.getDay()]} ${pad(now.getDate())}/${pad(now.getMonth() + 1)}/${now.getFullYear()}`;
}

/** "2026-10-01" theo giờ máy. */
export function dayKey(now: Date): string {
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

function timeText(now: Date): string {
  return `${dayText(now)}, ${pad(now.getHours())}:${pad(now.getMinutes())}`;
}

/** Câu gợi ý có sẵn: dùng lúc chưa có câu Gemini viết cho hôm nay (đang hỏi, mất mạng). */
export const SUGGESTION_POOL = [
  "Tỉ giá USD hôm nay?",
  "Giá vàng SJC hôm nay?",
  "Tin công nghệ nổi bật hôm nay",
  "1 inch bằng bao nhiêu cm?",
  "Giá xăng hôm nay bao nhiêu?",
  "Viết lời chúc sinh nhật đồng nghiệp",
  "Phím tắt chụp màn hình trên Windows?",
  "Trưa nay ăn gì cho nhanh?",
  "Đổi 100 đô ra tiền Việt",
  "Mẹo tập trung khi làm việc",
  "REST API là gì, nói ngắn thôi",
  "Còn bao nhiêu ngày nữa tới Tết?",
];

/** Khung chat hiện chừng này câu gợi ý. */
export const SHOWN_SUGGESTIONS = 4;

/** Bốc ngẫu nhiên `count` câu khác nhau trong `list`; `random` trả số trong [0, 1). */
export function pickSuggestions(list: readonly string[], count = SHOWN_SUGGESTIONS, random = Math.random): string[] {
  const rest = [...new Set(list)];
  const out: string[] = [];
  while (out.length < count && rest.length > 0) out.push(rest.splice(Math.floor(random() * rest.length), 1)[0]);
  return out;
}

/** Ghép câu gửi Gemini; `now` theo giờ máy. */
export function buildPrompt(persona: Persona, history: readonly Turn[], message: string, now: Date): string {
  const head = [
    `Bạn là ${persona.name}, ${persona.about}, một pet nhỏ sống trên màn hình máy tính của người dùng (app TinyWorld).`,
    "Trả lời bằng tiếng Việt (người dùng viết tiếng khác thì theo tiếng đó), thân thiện, không dùng emoji hay biểu tượng cảm xúc kiểu \":)))\".",
    "Ưu tiên đúng và gọn: vài câu hoặc vài gạch đầu dòng. Thông tin mới (tỉ giá, giá vàng, tin tức, thời tiết) thì tra cứu rồi ghi nguồn hoặc link nếu có. Không chắc thì nói không chắc, không bịa.",
    `Bây giờ là ${timeText(now)} (giờ máy người dùng).`,
  ].join("\n");
  const line = (turn: Turn) => `${turn.from === "user" ? "Người dùng" : persona.name}: ${turn.text.trim()}`;
  const ask = `Người dùng: ${message.trim()}\n${persona.name}:`;
  const recent = history.slice(-HISTORY_TURNS).map(line);
  // Dài quá thì bỏ dần lượt cũ nhất.
  while (recent.length > 0 && [head, ...recent, ask].join("\n\n").length > PROMPT_MAX) recent.shift();
  const parts = recent.length > 0 ? [head, "Đoạn chat gần đây:", recent.join("\n"), ask] : [head, ask];
  return parts.join("\n\n").slice(0, PROMPT_MAX);
}
