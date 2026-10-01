import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import ts from "typescript";

// Chạy đúng code của khung chat; hai file này không import gì.
async function load(file) {
  const code = ts.transpileModule(readFileSync(new URL(`../desktop/src/chat/${file}`, import.meta.url), "utf8"), {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);
}
const { parseInline, parseReply } = await load("markdown.ts");
const { buildPrompt, dayKey, dayText, personaOf, pickSuggestions, HISTORY_TURNS, PROMPT_MAX, SUGGESTION_POOL } =
  await load("prompt.ts");

test("chữ đậm, code, link markdown và link trần (bỏ dấu câu dính sau)", () => {
  assert.deepEqual(parseInline("Xem **tỉ giá** ở [Vietcombank](https://vcb.com.vn/ty-gia) hoặc https://sbv.gov.vn. Gõ `npm i`."), [
    { kind: "text", text: "Xem " },
    { kind: "bold", text: "tỉ giá" },
    { kind: "text", text: " ở " },
    { kind: "link", text: "Vietcombank", url: "https://vcb.com.vn/ty-gia" },
    { kind: "text", text: " hoặc " },
    { kind: "link", text: "https://sbv.gov.vn", url: "https://sbv.gov.vn" },
    { kind: "text", text: ". Gõ " },
    { kind: "code", text: "npm i" },
    { kind: "text", text: "." },
  ]);
});

test("chữ nghiêng sát chữ hai đầu; dấu nhân có cách thì không phải chữ nghiêng", () => {
  assert.deepEqual(parseInline("*Lưu ý:* giá có thể chênh"), [
    { kind: "italic", text: "Lưu ý:" },
    { kind: "text", text: " giá có thể chênh" },
  ]);
  assert.deepEqual(parseInline("5 * 3 = 15, 2 * 4 = 8"), [{ kind: "text", text: "5 * 3 = 15, 2 * 4 = 8" }]);
});

test("bỏ biểu tượng cảm xúc trong câu trả lời, giữ dấu ngoặc của chữ", () => {
  assert.deepEqual(parseInline("Tỉ giá khoảng 25.982 :)))"), [{ kind: "text", text: "Tỉ giá khoảng 25.982" }]);
  assert.deepEqual(parseInline("Chịu thôi :(( mai hỏi lại nhé ;)"), [{ kind: "text", text: "Chịu thôi mai hỏi lại nhé" }]);
  assert.deepEqual(parseInline("f(x) = (a:b)"), [{ kind: "text", text: "f(x) = (a:b)" }]);
});

test("không nhận link javascript:, HTML giữ nguyên là chữ", () => {
  assert.deepEqual(parseInline("[bấm](javascript:alert(1)) <b>x</b>"), [
    { kind: "text", text: "[bấm](javascript:alert(1)) <b>x</b>" },
  ]);
});

test("đoạn, tiêu đề, gạch đầu dòng, danh sách số, khối code", () => {
  const blocks = parseReply("## Tỉ giá\nUSD hôm nay:\n- Mua: 25.000\n- Bán: 25.400\n\n1. Một\n2. Hai\n```\nlet a = 1;\n```\nHết.");
  assert.deepEqual(blocks.map((b) => b.kind), ["heading", "paragraph", "list", "list", "code", "paragraph"]);
  assert.equal(blocks[2].ordered, false);
  assert.equal(blocks[2].items.length, 2);
  assert.equal(blocks[3].ordered, true);
  assert.equal(blocks[4].text, "let a = 1;");
});

test("câu gửi đi có tính cách, giờ, chỉ vài lượt gần nhất, không quá dài", () => {
  const persona = personaOf("c-byte", "Byte");
  const history = Array.from({ length: 10 }, (_, i) => ({ from: i % 2 ? "pet" : "user", text: `lượt ${i}` }));
  const prompt = buildPrompt(persona, history, "Tỉ giá USD?", new Date(2026, 9, 1, 22, 40));
  assert.match(prompt, /Bạn là Byte, chim cánh cụt coder/);
  assert.match(prompt, /Thứ Năm 01\/10\/2026, 22:40/);
  assert.ok(!prompt.includes(`lượt ${9 - HISTORY_TURNS}`));
  assert.ok(prompt.includes("lượt 9"));
  assert.ok(prompt.endsWith("Người dùng: Tỉ giá USD?\nByte:"));
  const long = buildPrompt(persona, [{ from: "user", text: "x".repeat(10_000) }], "Hỏi", new Date());
  assert.ok(long.length <= PROMPT_MAX);
  assert.ok(long.endsWith("Byte:"));
  assert.equal(personaOf("z-moi", "Mới").name, "Mới");
});

test("câu gợi ý: bốc ngẫu nhiên không trùng, ngày gửi Rust theo giờ máy", () => {
  const picked = pickSuggestions(SUGGESTION_POOL);
  assert.equal(picked.length, 4);
  assert.equal(new Set(picked).size, 4);
  assert.ok(picked.every((s) => SUGGESTION_POOL.includes(s)));
  // Câu trùng chỉ tính một lần; ít câu hơn số cần thì lấy hết.
  assert.deepEqual(pickSuggestions(["a", "a", "b"], 4, () => 0), ["a", "b"]);
  assert.deepEqual(pickSuggestions(["a", "b", "c"], 2, () => 0.99), ["c", "b"]);
  const day = new Date(2026, 9, 1, 23, 50);
  assert.equal(dayKey(day), "2026-10-01");
  assert.equal(dayText(day), "Thứ Năm 01/10/2026");
});
