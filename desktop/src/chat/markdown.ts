/**
 * Đọc phần markdown thường gặp trong câu trả lời của Gemini (đoạn, gạch đầu dòng, danh sách số, tiêu đề,
 * chữ đậm, `code`, khối code, link) thành khối để React vẽ, không dùng HTML thô nên không chèn được mã.
 */

export type Inline =
  | { kind: "text"; text: string }
  | { kind: "bold"; text: string }
  | { kind: "italic"; text: string }
  | { kind: "code"; text: string }
  | { kind: "link"; text: string; url: string };

export type Block =
  | { kind: "paragraph"; lines: Inline[][] }
  | { kind: "heading"; content: Inline[] }
  | { kind: "list"; ordered: boolean; items: Inline[][] }
  | { kind: "code"; text: string };

/**
 * `[chữ](link)`, `**đậm**`, `` `code` ``, link trần http(s), hoặc `*nghiêng*` (sát chữ hai đầu, nên "5 * 3"
 * không bị coi là chữ nghiêng).
 */
const INLINE =
  /\[([^\]\n]+)\]\((https?:\/\/[^\s)]+)\)|\*\*([^*\n]+)\*\*|`([^`\n]+)`|(https?:\/\/[^\s<>()]+)|\*(?=\S)([^*\n]*?\S)\*/g;
/** Dấu câu dính sau link trần không thuộc link. */
const TRAILING = /[.,;:!?'"’”]+$/;

/** Biểu tượng cảm xúc kiểu ":)))", ":((", ";)" đứng riêng: câu trả lời của pet trong khung chat không dùng. */
const EMOTICON = /(^|\s)[:;]-?[)(]+(?=\s|$|[.,!?])/g;

/** Bỏ biểu tượng cảm xúc khỏi một dòng chữ (không đụng tới code). */
export function dropEmoticons(text: string): string {
  return text.replace(EMOTICON, "$1").replace(/ {2,}/g, " ").trimEnd();
}

export function parseInline(text: string): Inline[] {
  text = dropEmoticons(text);
  const out: Inline[] = [];
  const push = (part: Inline) => {
    const last = out[out.length - 1];
    if (part.kind === "text" && last?.kind === "text") last.text += part.text;
    else if (part.kind !== "text" || part.text) out.push(part);
  };
  let at = 0;
  for (const match of text.matchAll(INLINE)) {
    const start = match.index ?? 0;
    push({ kind: "text", text: text.slice(at, start) });
    const [whole, label, href, bold, code, bare, italic] = match;
    if (label && href) push({ kind: "link", text: label, url: href });
    else if (bold) push({ kind: "bold", text: bold });
    else if (italic) push({ kind: "italic", text: italic });
    else if (code) push({ kind: "code", text: code });
    else if (bare) {
      const tail = TRAILING.exec(bare)?.[0] ?? "";
      const url = bare.slice(0, bare.length - tail.length);
      push({ kind: "link", text: url, url });
      push({ kind: "text", text: tail });
    } else push({ kind: "text", text: whole });
    at = start + whole.length;
  }
  push({ kind: "text", text: text.slice(at) });
  return out;
}

const BULLET = /^\s*[-*•]\s+(.*)$/;
const NUMBER = /^\s*\d+[.)]\s+(.*)$/;
const HEADING = /^\s*#{1,6}\s+(.*)$/;

export function parseReply(text: string): Block[] {
  const blocks: Block[] = [];
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (line.trim().startsWith("```")) {
      const code: string[] = [];
      i++;
      while (i < lines.length && !lines[i].trim().startsWith("```")) code.push(lines[i++]);
      i++;
      blocks.push({ kind: "code", text: code.join("\n") });
      continue;
    }
    if (!line.trim()) {
      i++;
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading) {
      blocks.push({ kind: "heading", content: parseInline(heading[1]) });
      i++;
      continue;
    }
    const ordered = NUMBER.test(line);
    if (ordered || BULLET.test(line)) {
      const pattern = ordered ? NUMBER : BULLET;
      const items: Inline[][] = [];
      while (i < lines.length && pattern.test(lines[i])) {
        items.push(parseInline(pattern.exec(lines[i])?.[1] ?? ""));
        i++;
      }
      blocks.push({ kind: "list", ordered, items });
      continue;
    }
    const paragraph: Inline[][] = [];
    while (
      i < lines.length &&
      lines[i].trim() &&
      !lines[i].trim().startsWith("```") &&
      !HEADING.test(lines[i]) &&
      !BULLET.test(lines[i]) &&
      !NUMBER.test(lines[i])
    ) {
      paragraph.push(parseInline(lines[i]));
      i++;
    }
    blocks.push({ kind: "paragraph", lines: paragraph });
  }
  return blocks;
}
