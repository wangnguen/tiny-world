// Sinh từ desktop/src/skyIcons.ts bằng scripts/prepare-website.mjs, không sửa tay.
// Hình thời tiết nhỏ (SVG 16×16), dùng chung cho nhãn nhiệt độ cạnh pet và nút Xem thử trong Cài đặt.
// Chuỗi cố định, không có gì người dùng nhập.
const CLOUD = `<path d="M4.5 10h7.2a2.6 2.6 0 0 0 .3-5.2 3.6 3.6 0 0 0-6.9-.5A2.9 2.9 0 0 0 4.5 10z" fill="#e8eef9" stroke="#6b7fa8" stroke-width="1.1"/>`;
const ICONS = {
    sunny: `<circle cx="8" cy="8" r="3.2" fill="#ffb703" stroke="#e07b00"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" stroke="#e07b00" stroke-width="1.4" stroke-linecap="round"/>`,
    clear: `<path d="M10.5 2.5a5.5 5.5 0 1 0 3 9.6 4.6 4.6 0 0 1-3-9.6z" fill="#ffd36b" stroke="#c98a00"/>`,
    cloudy: `<path d="M4.5 12.5h7.2a2.8 2.8 0 0 0 .3-5.6 3.8 3.8 0 0 0-7.3-.6A3.1 3.1 0 0 0 4.5 12.5z" fill="#e8eef9" stroke="#6b7fa8" stroke-width="1.1"/>`,
    rain: `${CLOUD}<path d="M5.5 12l-.7 2M8.5 12l-.7 2M11.5 12l-.7 2" stroke="#3b82f6" stroke-width="1.3" stroke-linecap="round"/>`,
    storm: `${CLOUD}<path d="M8.8 10.2 6.8 13h2l-1 2.6 3-3.8h-2l.9-1.6z" fill="#ffcc33" stroke="#c98a00" stroke-width=".6"/>`,
    snow: `<path d="M8 2v12M2.8 5l10.4 6M2.8 11l10.4-6" stroke="#4a90e2" stroke-width="1.4" stroke-linecap="round"/>`,
    fog: `<path d="M2.5 5h11M1.5 8h13M3.5 11h9" stroke="#7d8fb3" stroke-width="1.6" stroke-linecap="round"/>`,
    petals: `<g fill="#ff9ec2" stroke="#d9668f" stroke-width=".7"><circle cx="8" cy="4.6" r="2.3"/><circle cx="11.3" cy="7" r="2.3"/><circle cx="10" cy="10.9" r="2.3"/><circle cx="6" cy="10.9" r="2.3"/><circle cx="4.7" cy="7" r="2.3"/></g><circle cx="8" cy="8" r="1.6" fill="#ffd36b"/>`,
};
/** Thẻ `<svg>` của hình thời tiết `sky`. */
export function skyIconSvg(sky) {
    return `<svg viewBox="0 0 16 16" aria-hidden="true">${ICONS[sky]}</svg>`;
}
