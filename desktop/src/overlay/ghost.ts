import type { Bounds } from "@tinyworld/sim";

/** Con ma pixel art (sự kiện hiếm lúc 2 giờ sáng): chữ là ô màu, chấm là trong suốt. */
const SPRITE = [
  "....KKKKKK....",
  "...KWWWWWWK...",
  "..KWWWWWWWWK..",
  ".KWWWWWWWWWWK.",
  ".KWWKKWWKKWWK.",
  ".KWWKKWWKKWWK.",
  ".KWWWWWWWWWWK.",
  ".KWWWWBBWWWWK.",
  ".KWWWWWWWWWWK.",
  ".KWWWWWWWWWWK.",
  ".KWWWWWWWWWWK.",
  ".KWKWWKWWKWWK.",
  "..K.KK.KK.KK..",
];
const COLORS: Record<string, string> = { K: "#2a2340", W: "#eef2ff", B: "#ff9ec2" };
/** Mỗi ô của con ma to chừng này CSS pixel. */
const CELL = 3;
/** Bay ngang hết vùng làm việc trong chừng này giây, nhấp nhô lên xuống chừng này px. */
const CROSS_SECONDS = 12;
const BOB = 14;

/** Con ma bay ngang qua màn hình, ngay trên chỗ các pet đứng, rồi biến mất. */
export class Ghost {
  readonly element: HTMLCanvasElement;
  private start = 0;
  /** -1: bay từ phải sang trái, 1: từ trái sang phải. */
  private dir: 1 | -1 = 1;
  private y = 0;
  private bounds: Bounds | null = null;

  constructor(container: HTMLElement) {
    this.element = document.createElement("canvas");
    this.element.className = "ghost";
    this.element.hidden = true;
    const width = SPRITE[0].length * CELL;
    const height = SPRITE.length * CELL;
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(width * dpr);
    this.element.height = Math.round(height * dpr);
    this.element.style.width = `${width}px`;
    this.element.style.height = `${height}px`;
    const ctx = this.element.getContext("2d");
    if (ctx) {
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      SPRITE.forEach((row, r) => {
        for (let c = 0; c < row.length; c++) {
          const color = COLORS[row[c]];
          if (!color) continue;
          ctx.fillStyle = color;
          ctx.fillRect(c * CELL, r * CELL, CELL, CELL);
        }
      });
    }
    container.append(this.element);
  }

  /** Đang bay. */
  get flying(): boolean {
    return !this.element.hidden;
  }

  /** Bắt đầu bay ngang `bounds`, cao hơn mặt đất chừng `above` px (ngang tầm đầu pet). */
  fly(bounds: Bounds, above: number, now: number): void {
    this.bounds = bounds;
    this.start = now;
    this.dir = Math.random() < 0.5 ? 1 : -1;
    this.y = bounds.floor - above - SPRITE.length * CELL;
    this.element.hidden = false;
  }

  /** Vị trí giữa thân con ma theo chiều ngang (CSS pixel của overlay), `null` nếu không bay. */
  update(now: number): number | null {
    const bounds = this.bounds;
    if (!this.flying || !bounds) return null;
    const t = (now - this.start) / 1000 / CROSS_SECONDS;
    if (t >= 1) {
      this.element.hidden = true;
      return null;
    }
    const width = SPRITE[0].length * CELL;
    const span = bounds.right - bounds.left + 2 * width;
    const x = this.dir > 0 ? bounds.left - width + span * t : bounds.right + width - span * t - width;
    const y = this.y + Math.sin(t * Math.PI * 6) * BOB;
    this.element.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px) scaleX(${-this.dir})`;
    return x + width / 2;
  }
}
