/**
 * Câu gửi Gemini khi chat với pet: tính cách ngắn của nhân vật, ngày giờ hiện tại, vài lượt chat gần nhất
 * và câu hỏi. Không gửi gì khác (không gửi tên người dùng, thành phố, cửa sổ đang mở...). Câu chữ theo ngôn
 * ngữ đang dùng, lấy ở mục `prompt`, `personas`, `calendar` của file chữ (packages/core/src/i18n).
 *
 * File này không import giá trị nào (chỉ kiểu) để test chạy thẳng được (scripts/chat.test.mjs).
 */
import type { Messages } from "@tinyworld/core";

/** Phần file chữ mà câu gửi Gemini cần. */
export type PromptText = Pick<Messages, "prompt" | "personas" | "calendar">;

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

const pad = (n: number) => String(n).padStart(2, "0");

/** Thay `{tên}` trong `template` (như `fill` của packages/core, chép lại để file này không import gì). */
function format(template: string, params: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match));
}

/** Tính cách theo tên thư mục pack (mục `personas`, khớp bảng nhân vật trong assets/sprite-sources/README.md); pack mới chưa có thì chỉ có tên. */
export function personaOf(pet: string, name: string, text: PromptText): Persona {
  const personas: Record<string, string | undefined> = text.personas;
  return { name, about: personas[pet] ?? text.prompt.defaultAbout };
}

/** "Thứ Năm 01/10/2026" ("Thursday, October 1, 2026") theo giờ máy. */
export function dayText(now: Date, text: PromptText): string {
  return format(text.prompt.date, {
    weekday: text.calendar.weekdays[now.getDay()],
    dd: pad(now.getDate()),
    mm: pad(now.getMonth() + 1),
    yyyy: now.getFullYear(),
    d: now.getDate(),
    monthName: text.calendar.months[now.getMonth()],
  });
}

/** "2026-10-01" theo giờ máy. */
export function dayKey(now: Date): string {
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}

function timeText(now: Date, text: PromptText): string {
  return format(text.prompt.time, { date: dayText(now, text), hh: pad(now.getHours()), min: pad(now.getMinutes()) });
}

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
export function buildPrompt(
  persona: Persona,
  history: readonly Turn[],
  message: string,
  now: Date,
  text: PromptText,
): string {
  const { prompt } = text;
  const head = [
    format(prompt.you, { name: persona.name, about: persona.about }),
    prompt.language,
    prompt.style,
    format(prompt.now, { time: timeText(now, text) }),
  ].join("\n");
  const line = (turn: Turn) => `${turn.from === "user" ? prompt.user : persona.name}: ${turn.text.trim()}`;
  const ask = `${prompt.user}: ${message.trim()}\n${persona.name}:`;
  const recent = history.slice(-HISTORY_TURNS).map(line);
  // Dài quá thì bỏ dần lượt cũ nhất.
  while (recent.length > 0 && [head, ...recent, ask].join("\n\n").length > PROMPT_MAX) recent.shift();
  const parts = recent.length > 0 ? [head, prompt.recent, recent.join("\n"), ask] : [head, ask];
  return parts.join("\n\n").slice(0, PROMPT_MAX);
}
