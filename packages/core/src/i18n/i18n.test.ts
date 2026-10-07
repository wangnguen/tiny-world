import { describe, expect, it } from "vitest";
import en from "./en.json";
import vi from "./vi.json";
import { fill, messages } from ".";

/** Danh sách mục (đường dẫn) và độ dài mảng của một file chữ. */
function shape(value: unknown, path = ""): string[] {
  if (Array.isArray(value)) return [`${path}[${value.length}]`, ...value.flatMap((v, i) => shape(v, `${path}[${i}]`))];
  if (value && typeof value === "object") {
    return Object.entries(value).flatMap(([key, v]) => [path ? `${path}.${key}` : key, ...shape(v, path ? `${path}.${key}` : key)]);
  }
  return [];
}

describe("file chữ", () => {
  it("vi.json và en.json có đúng các mục giống nhau, mảng dài bằng nhau", () => {
    expect(shape(en).sort()).toEqual(shape(vi).sort());
  });

  it("chữ nào cũng không rỗng", () => {
    const texts = (value: unknown): string[] =>
      typeof value === "string" ? [value] : value && typeof value === "object" ? Object.values(value).flatMap(texts) : [];
    for (const file of [vi, en]) expect(texts(file).filter((text) => !text.trim())).toEqual([]);
  });

  it("điền giá trị vào chỗ {tên}, tên không có thì giữ nguyên", () => {
    expect(fill(messages("en").chat.title, { name: "Momo" })).toBe("Chat with Momo");
    expect(fill(messages("vi").pet.break, { minutes: 50 })).toBe("Ngồi liền 50 phút rồi, đứng dậy nghỉ mắt chút đi :)))");
    expect(fill("{a} {b}", { a: 1 })).toBe("1 {b}");
  });
});
