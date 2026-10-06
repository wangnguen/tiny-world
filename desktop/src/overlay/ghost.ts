import type { Point } from "@tinyworld/core";
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
const WIDTH = SPRITE[0].length * CELL;
const HEIGHT = SPRITE.length * CELL;
/** Cỡ con ma (CSS pixel). */
export const GHOST_SIZE = { width: WIDTH, height: HEIGHT } as const;
/** Bay hết bề ngang vùng làm việc trong chừng này giây. */
const CROSS_SECONDS = 14;
/** Lượn lên xuống chừng này lượt trong một lần bay ngang, khắp chiều cao vùng làm việc. */
const WAVES = 1.6;
/** Nhấp nhô thêm chừng này px, chừng này lượt mỗi lần bay ngang. */
const BOB = 10;
const BOBS = 9;
/** Sà xuống ngang tầm từng pet trong khoảng chừng này lần bề ngang vùng làm việc quanh con đó. */
const SWOOP = 0.1;
/** Cách mép trên, mép dưới vùng làm việc ít nhất chừng này px. */
const MARGIN = 16;
/** Nghiêng theo hướng bay, tối đa chừng này radian. */
const MAX_TILT = 0.45;

/** Đường bay của một lượt: từ mép này sang mép kia, lượn khắp chiều cao, sà xuống ngang tầm từng pet. */
export interface GhostFlight {
  bounds: Bounds;
  /** -1: bay từ phải sang trái, 1: từ trái sang phải. */
  dir: 1 | -1;
  /** Pha lượn lên xuống (0–2π): mỗi lượt bắt đầu ở một độ cao khác. */
  phase: number;
}

/**
 * Giữa thân con ma và độ nghiêng lúc đã bay được phần `t` (0–1) của đường bay `flight`; `targets`: chỗ sà
 * qua (giữa thân con ma), mỗi pet một chỗ.
 */
export function ghostAt(flight: GhostFlight, targets: readonly Point[], t: number): { x: number; y: number; tilt: number } {
  const at = (u: number) => {
    const { bounds, dir, phase } = flight;
    const from = dir > 0 ? bounds.left - WIDTH / 2 : bounds.right + WIDTH / 2;
    const to = dir > 0 ? bounds.right + WIDTH / 2 : bounds.left - WIDTH / 2;
    const x = from + (to - from) * u;
    const high = bounds.top + MARGIN + HEIGHT / 2;
    const low = bounds.floor - MARGIN - HEIGHT / 2;
    let y = high + (low - high) * (0.5 + 0.5 * Math.sin(2 * Math.PI * WAVES * u + phase));
    y += Math.sin(2 * Math.PI * BOBS * u) * BOB;
    // Gần pet nào thì sà xuống ngang tầm con đó; hai con gần nhau thì lấy chỗ giữa, không bị giật.
    const radius = Math.max(1, SWOOP * (bounds.right - bounds.left));
    let pull = 0;
    let sum = 0;
    let weighted = 0;
    for (const target of targets) {
      const w = Math.exp(-(((x - target.x) / radius) ** 2));
      pull = Math.max(pull, w);
      sum += w;
      weighted += w * target.y;
    }
    if (sum > 0) y += (weighted / sum - y) * pull;
    return { x, y: Math.min(Math.max(y, high), low) };
  };
  const here = at(t);
  const ahead = at(t + 0.004);
  const dx = Math.abs(ahead.x - here.x);
  const tilt = dx > 0 ? flight.dir * Math.atan((ahead.y - here.y) / dx) * 0.6 : 0;
  return { ...here, tilt: Math.min(Math.max(tilt, -MAX_TILT), MAX_TILT) };
}

/**
 * Con ma bay ngang qua màn hình, lượn lên xuống khắp vùng làm việc, sà xuống ngang tầm từng pet (để còn
 * dọa) rồi biến mất ở mép bên kia.
 */
export class Ghost {
  readonly element: HTMLCanvasElement;
  private start = 0;
  private flight: GhostFlight | null = null;

  constructor(container: HTMLElement) {
    this.element = document.createElement("canvas");
    this.element.className = "ghost";
    this.element.hidden = true;
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(WIDTH * dpr);
    this.element.height = Math.round(HEIGHT * dpr);
    this.element.style.width = `${WIDTH}px`;
    this.element.style.height = `${HEIGHT}px`;
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

  /** Bắt đầu bay ngang `bounds`. */
  fly(bounds: Bounds, now: number): void {
    this.flight = {
      bounds,
      dir: Math.random() < 0.5 ? 1 : -1,
      phase: Math.random() * Math.PI * 2,
    };
    this.start = now;
    this.element.hidden = false;
  }

  /**
   * Bay tiếp, sà qua từng chỗ trong `targets` (giữa thân con ma, mỗi pet một chỗ, theo chỗ pet đang đứng).
   * Trả về giữa thân con ma (CSS pixel của overlay), `null` nếu không bay.
   */
  update(now: number, targets: readonly Point[]): Point | null {
    const flight = this.flight;
    if (!this.flying || !flight) return null;
    const t = (now - this.start) / 1000 / CROSS_SECONDS;
    if (t >= 1) {
      this.element.hidden = true;
      this.flight = null;
      return null;
    }
    const { x, y, tilt } = ghostAt(flight, targets, t);
    const left = Math.round(x - WIDTH / 2);
    const top = Math.round(y - HEIGHT / 2);
    this.element.style.transform = `translate(${left}px, ${top}px) rotate(${tilt.toFixed(3)}rad) scaleX(${-flight.dir})`;
    return { x, y };
  }
}
