// Sinh bởi scripts/prepare-website.mjs, không sửa tay. Thay @tinyworld/core cho lines.js trên trang giới
// thiệu: chữ của app (câu pet nói, tên thứ, tháng) theo ngôn ngữ của trang nằm sẵn trong #tw-app-text.
const TEXT = JSON.parse(document.getElementById("tw-app-text")?.textContent ?? "{}");

export function messages() {
  return TEXT;
}

export function fill(template, params) {
  return template.replace(/\{(\w+)\}/g, (match, name) => (name in params ? String(params[name]) : match));
}
