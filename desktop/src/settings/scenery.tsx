// Cảnh trang trí của cửa sổ cài đặt: làng nhỏ ban đêm trên đồi (đầu trang), bụi cỏ ở góc dưới,
// cây thông trong ô "Chạy cùng Windows". Toạ độ tính theo cửa sổ rộng 440 px (không đổi cỡ được).

/** Chỗ pet đứng trên đồi trong `NightScene` (CSS pixel, gốc là góc trên trái của cảnh). */
export const HILL_SPOT = { x: 384, y: 112 };

/** Ngôi sao 5 cánh tâm (cx, cy), bán kính r. */
export function starPath(cx: number, cy: number, r: number): string {
  const points = Array.from({ length: 10 }, (_, i) => {
    const angle = (Math.PI / 5) * i - Math.PI / 2;
    const radius = i % 2 === 0 ? r : r * 0.45;
    return `${(cx + radius * Math.cos(angle)).toFixed(1)},${(cy + radius * Math.sin(angle)).toFixed(1)}`;
  });
  return `M${points.join("L")}Z`;
}

const DOTS = [
  [196, 34, 1],
  [232, 64, 0.8],
  [262, 20, 1.2],
  [334, 34, 0.9],
  [352, 70, 0.8],
  [396, 52, 1.1],
  [432, 80, 0.8],
  [248, 88, 0.9],
  [410, 16, 1],
  [178, 74, 0.8],
];

export function NightScene() {
  return (
    <svg className="hero__scene" viewBox="0 0 440 150" aria-hidden="true">
      <defs>
        <radialGradient id="sky-glow">
          <stop offset="0" stopColor="#3a63b8" stopOpacity="0.45" />
          <stop offset="1" stopColor="#3a63b8" stopOpacity="0" />
        </radialGradient>
        <radialGradient id="lantern-glow">
          <stop offset="0" stopColor="#ffbf66" stopOpacity="0.75" />
          <stop offset="0.45" stopColor="#ffb05a" stopOpacity="0.25" />
          <stop offset="1" stopColor="#ffb05a" stopOpacity="0" />
        </radialGradient>
        <radialGradient id="window-glow">
          <stop offset="0" stopColor="#ffc46b" stopOpacity="0.6" />
          <stop offset="1" stopColor="#ffc46b" stopOpacity="0" />
        </radialGradient>
        <linearGradient id="hill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#315c66" />
          <stop offset="0.5" stopColor="#1f3a4f" />
          <stop offset="1" stopColor="#13213a" />
        </linearGradient>
        <linearGradient id="fade" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#fff" stopOpacity="0" />
          <stop offset="0.3" stopColor="#fff" stopOpacity="1" />
        </linearGradient>
        <mask id="scene-fade">
          <rect width="440" height="150" fill="url(#fade)" />
        </mask>
      </defs>
      <circle cx="360" cy="40" r="130" fill="url(#sky-glow)" />
      <g mask="url(#scene-fade)">
        {DOTS.map(([x, y, r]) => (
          <circle key={`${x}-${y}`} cx={x} cy={y} r={r * 1.3} fill="#eef2ff" opacity="0.9" />
        ))}
        <path d={starPath(300, 50, 10)} fill="#ffcc62" />
        <path d={starPath(424, 30, 6.5)} fill="#ffcc62" />

        {/* Làng phía sau đồi. */}
        <g fill="#25375f">
          <circle cx="196" cy="130" r="13" />
          <circle cx="214" cy="126" r="11" />
          <path d="M290 150v-24l10-9 10 9v24z" />
          <circle cx="314" cy="130" r="8" />
        </g>
        <g fill="#2f4575">
          <path d="M232 150V102l9-18 9 18v48z" />
          <path d="M240.2 84V73h1.6v11zM237.5 76.5h7v1.6h-7z" />
          <path d="M252 150v-34l16-14 16 14v34z" />
        </g>
        <g fill="url(#window-glow)">
          <circle cx="241" cy="110" r="9" />
          <circle cx="269" cy="127" r="11" />
          <circle cx="299" cy="132" r="7" />
        </g>
        <g fill="#ffc768">
          <rect x="239" y="106" width="4" height="7" rx="2" />
          <rect x="262" y="124" width="5" height="6" rx="1" />
          <rect x="272" y="124" width="5" height="6" rx="1" opacity="0.8" />
          <rect x="297" y="130" width="4" height="5" rx="1" />
        </g>

        {/* Đồi, viền cỏ sáng dưới trăng. */}
        <path d="M160 150C226 146 268 124 320 118S402 110 440 112V150Z" fill="url(#hill)" />
        <path d="M160 150C226 146 268 124 320 118S402 110 440 112" fill="none" stroke="#86c8a4" strokeOpacity="0.75" strokeWidth="1.6" />
        <g fill="none" stroke="#6fb28f" strokeWidth="1.3" strokeLinecap="round">
          <path d="M296 121l2-6 1 6 2-5 1 5" />
          <path d="M340 116l2-5 1 5 2-4" />
          <path d="M408 111l2-5 1 5 2-4 1 4" />
        </g>

        {/* Hàng rào bên phải. */}
        <g fill="#454a63" stroke="#454a63" strokeWidth="2" strokeLinecap="round">
          <rect x="412" y="92" width="3.5" height="20" rx="1" stroke="none" />
          <rect x="428" y="90" width="3.5" height="22" rx="1" stroke="none" />
          <path d="M406 98l34-3M406 105l34-3" fill="none" />
        </g>
        <g fill="#2f6150">
          <ellipse cx="430" cy="78" rx="7" ry="14" transform="rotate(20 430 78)" />
          <ellipse cx="438" cy="70" rx="6" ry="13" transform="rotate(-15 438 70)" />
        </g>

        {/* Đèn lồng đặt trên đồi, hắt sáng vàng xuống cỏ. */}
        <circle cx="318" cy="108" r="44" fill="url(#lantern-glow)" />
        <path d="M312 94q6-9 12 0" fill="none" stroke="#3b2d24" strokeWidth="1.8" />
        <rect x="310.5" y="93" width="15" height="4" rx="1.5" fill="#3b2d24" />
        <rect x="311.5" y="97" width="13" height="16" rx="2" fill="#ffcd72" stroke="#4a3424" strokeWidth="1.6" />
        <path d="M318 97v16" stroke="#d8923c" strokeWidth="0.9" />
        <ellipse cx="318" cy="106" rx="2" ry="3.4" fill="#fff3c6" />
        <rect x="309.5" y="113" width="17" height="4" rx="1.2" fill="#3b2d24" />
      </g>
    </svg>
  );
}

/** Bụi cỏ, hòn đá ở góc dưới bên phải cửa sổ. */
export function CornerDecor() {
  return (
    <svg className="page__decor" viewBox="0 0 150 90" aria-hidden="true">
      <ellipse cx="92" cy="84" rx="30" ry="12" fill="#1e2e4f" />
      <g fill="#2a4a6e">
        <path d="M134 90c-4-18-1-34 8-46-3 16-2 31 2 46z" />
        <path d="M122 90c-9-12-12-25-9-38 5 12 9 24 14 38z" />
        <path d="M146 90c2-14 1-27-4-40 9 10 10 25 8 40z" />
      </g>
      <g fill="#36608a">
        <ellipse cx="119" cy="62" rx="4" ry="9" transform="rotate(-25 119 62)" />
        <ellipse cx="141" cy="52" rx="4" ry="9" transform="rotate(20 141 52)" />
      </g>
      <circle cx="70" cy="48" r="1" fill="#d6ddff" opacity="0.6" />
      <circle cx="104" cy="30" r="0.8" fill="#d6ddff" opacity="0.6" />
    </svg>
  );
}

/** Cây thông nhỏ và vài ngôi sao trong ô "Chạy cùng Windows". */
export function PineDecor() {
  return (
    <svg className="startup__decor" viewBox="0 0 80 80" aria-hidden="true">
      <circle cx="18" cy="14" r="1" fill="#9fb4ff" opacity="0.8" />
      <circle cx="6" cy="30" r="0.8" fill="#9fb4ff" opacity="0.6" />
      <path d="M0 80c14-8 40-12 80-10v10z" fill="#1a2a4a" />
      <g fill="#24365e">
        <path d="M54 22l14 22H40z" />
        <path d="M54 34l17 26H37z" />
        <path d="M52 58h4v12h-4z" />
      </g>
    </svg>
  );
}
