import { starPath } from "../settings/scenery";

/** Cỡ cảnh (CSS pixel), khớp `.chat__head` trong chat.css. */
export const SCENE = { width: 360, height: 112 };
/** Chỗ pet đứng trên đồi trong `ChatScene` (CSS pixel, tính từ góc trên phải của cảnh). */
export const HILL_FOOT = { right: 62, y: 92 };

/** [x, y, bán kính, độ sáng] */
const DOTS = [
  [118, 16, 1.1, 0.9],
  [146, 46, 0.8, 0.7],
  [172, 22, 1.3, 1],
  [204, 58, 0.8, 0.6],
  [226, 12, 1, 0.85],
  [262, 34, 0.9, 0.75],
  [290, 10, 1.2, 0.9],
  [318, 44, 0.8, 0.7],
  [346, 22, 1.1, 0.9],
  [134, 70, 0.7, 0.5],
  [244, 64, 0.7, 0.55],
  [96, 40, 0.8, 0.6],
];

/** Đom đóm trên đồi: [x, y]. */
const FIREFLIES = [
  [214, 84],
  [338, 74],
  [278, 70],
];

/**
 * Đầu khung chat: trời đêm có trăng và sao, làng nhỏ sáng đèn sau đồi, đồi cỏ bên phải cho pet đứng, cùng
 * kiểu với đầu cửa sổ Cài đặt. Cảnh bám mép phải, cửa sổ rộng ra thì phần trời bên trái dài thêm.
 */
export function ChatScene() {
  return (
    <svg
      className="chat__scene"
      viewBox={`0 0 ${SCENE.width} ${SCENE.height}`}
      preserveAspectRatio="xMaxYMax meet"
      aria-hidden="true"
    >
      <defs>
        <radialGradient id="chat-glow">
          <stop offset="0" stopColor="#4a76d1" stopOpacity="0.55" />
          <stop offset="1" stopColor="#3a63b8" stopOpacity="0" />
        </radialGradient>
        <radialGradient id="chat-moon-glow">
          <stop offset="0" stopColor="#fff3c9" stopOpacity="0.45" />
          <stop offset="1" stopColor="#fff3c9" stopOpacity="0" />
        </radialGradient>
        <radialGradient id="chat-warm">
          <stop offset="0" stopColor="#ffbf66" stopOpacity="0.7" />
          <stop offset="0.45" stopColor="#ffb05a" stopOpacity="0.22" />
          <stop offset="1" stopColor="#ffb05a" stopOpacity="0" />
        </radialGradient>
        <linearGradient id="chat-hill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#3a6d72" />
          <stop offset="0.5" stopColor="#21405a" />
          <stop offset="1" stopColor="#13213a" />
        </linearGradient>
        <linearGradient id="chat-back-hill" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#223a66" />
          <stop offset="1" stopColor="#152442" />
        </linearGradient>
        <linearGradient id="chat-fade" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#fff" stopOpacity="0" />
          <stop offset="0.32" stopColor="#fff" stopOpacity="1" />
        </linearGradient>
        <mask id="chat-scene-fade">
          <rect width={SCENE.width} height={SCENE.height} fill="url(#chat-fade)" />
        </mask>
      </defs>
      <circle cx="290" cy="20" r="120" fill="url(#chat-glow)" />
      <g mask="url(#chat-scene-fade)">
        {DOTS.map(([x, y, r, o]) => (
          <circle key={`${x}-${y}`} cx={x} cy={y} r={r * 1.3} fill="#f2f5ff" opacity={o} />
        ))}
        <path d={starPath(196, 34, 6)} fill="#ffcc62" />
        <path d={starPath(352, 52, 4)} fill="#ffcc62" />

        {/* Trăng khuyết. */}
        <circle cx="258" cy="26" r="26" fill="url(#chat-moon-glow)" />
        <path d="M262 14a12 12 0 1 0 8 21a10 10 0 1 1-8-21z" fill="#fff1c2" />

        {/* Đồi sau và làng nhỏ sáng đèn. */}
        <path d="M110 112C150 98 190 92 232 94S318 86 360 88V112Z" fill="url(#chat-back-hill)" />
        <g fill="#2c4272">
          <path d="M168 104V90l9-8 9 8v14z" />
          <path d="M190 104V80l7-14 7 14v24z" />
          <path d="M196.2 66v-8h1.6v8zM194 60.5h6v1.4h-6z" />
          <circle cx="215" cy="98" r="8" />
        </g>
        <g fill="url(#chat-warm)">
          <circle cx="177" cy="95" r="9" />
          <circle cx="197" cy="86" r="8" />
        </g>
        <g fill="#ffc768">
          <rect x="174.5" y="92" width="5" height="5" rx="1" />
          <rect x="195.5" y="83" width="3.5" height="6" rx="1.5" />
        </g>

        {/* Đồi trước, viền cỏ sáng dưới trăng; pet đứng ở đây. */}
        <path d="M190 112C236 108 262 96 290 93S340 90 360 91V112Z" fill="url(#chat-hill)" />
        <path
          d="M190 112C236 108 262 96 290 93S340 90 360 91"
          fill="none"
          stroke="#8fd4ae"
          strokeOpacity="0.85"
          strokeWidth="1.5"
        />
        <g fill="none" stroke="#74ba94" strokeWidth="1.2" strokeLinecap="round">
          <path d="M252 101l2-5 1 5 2-4" />
          <path d="M322 92l2-5 1 5 2-4 1 4" />
          <path d="M346 92l2-4 1 4" />
        </g>

        {/* Đèn lồng sau lưng pet. */}
        <circle cx="266" cy="88" r="30" fill="url(#chat-warm)" />
        <path d="M261.5 77q4.5-7 9 0" fill="none" stroke="#3b2d24" strokeWidth="1.5" />
        <rect x="259" y="76" width="14" height="3" rx="1.2" fill="#3b2d24" />
        <rect x="260" y="79" width="12" height="13" rx="2" fill="#ffcd72" stroke="#4a3424" strokeWidth="1.3" />
        <ellipse cx="266" cy="85.5" rx="1.8" ry="3" fill="#fff3c6" />
        <rect x="258.5" y="92" width="15" height="3" rx="1.2" fill="#3b2d24" />

        {FIREFLIES.map(([x, y]) => (
          <g key={`${x}-${y}`}>
            <circle cx={x} cy={y} r="4" fill="url(#chat-warm)" />
            <circle cx={x} cy={y} r="1.1" fill="#ffe08a" />
          </g>
        ))}
      </g>
    </svg>
  );
}
