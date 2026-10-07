/**
 * Chữ của app theo ngôn ngữ: mọi câu chữ nằm trong `vi.json` và `en.json` cạnh file này, dùng chung cho
 * frontend (Cài đặt, khung chat, câu pet nói) và Rust (menu khay, câu báo lỗi; i18n.rs nhúng hai file lúc
 * build). Thêm hay sửa câu thì sửa cả hai file, giữ đúng các mục như nhau (`en.json` thiếu mục nào thì
 * typecheck báo lỗi, test so cả hai chiều). Chỗ `{tên}` được thay bằng giá trị lúc dùng (`fill`).
 */
import type { Lang } from "../types";
import en from "./en.json";
import vi from "./vi.json";

/** Chữ của một ngôn ngữ, theo cấu trúc của `vi.json`. */
export type Messages = typeof vi;

const english: Messages = en;

export const MESSAGES: Record<Lang, Messages> = { vi, en: english };

/** Chữ của ngôn ngữ `lang`. */
export function messages(lang: Lang): Messages {
  return MESSAGES[lang];
}

/** Thay các chỗ `{tên}` trong `template` bằng giá trị trong `params`; tên không có thì giữ nguyên. */
export function fill(template: string, params: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (match, name: string) => (name in params ? String(params[name]) : match));
}
