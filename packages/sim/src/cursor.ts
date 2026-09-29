/** Con trỏ chuột như pet thấy, CSS pixel của overlay. */
export interface CursorView {
  readonly x: number;
  readonly y: number;
  /** Chỗ con trỏ ở lúc bước mô phỏng trước: đoạn từ đó tới (x, y) là chỗ con trỏ vừa quét qua. */
  readonly fromX: number;
  readonly fromY: number;
  /** Vận tốc đã làm mượt (px/s), về 0 khi con trỏ dừng. */
  readonly vx: number;
  readonly vy: number;
  /** Tốc độ lớn nhất giữa hai lần báo vị trí trong bước vừa rồi (px/s): giật chuột thì vọt lên. */
  readonly peak: number;
  /** Số giây con trỏ đứng yên. */
  readonly still: number;
  /** Đang giữ nút chuột (kéo cửa sổ, bôi đen chữ): người dùng đang bận, pet không đuổi theo. */
  readonly pressed: boolean;
}

/** Vận tốc lấy trung bình trong khoảng chừng này (giây). */
const SMOOTHING = 0.08;
/** Hai lần báo vị trí cách nhau lâu hơn chừng này thì không tính vận tốc (con trỏ vừa đứng yên một lúc). */
const MAX_GAP = 0.25;
/** Không có lần báo nào trong chừng này thì coi như con trỏ đã dừng. */
const STOP_AFTER = 0.1;

/** Theo dõi con trỏ từ các lần Rust báo vị trí (khoảng 60 lần/giây, chỉ khi có thay đổi). */
export class Pointer implements CursorView {
  x: number;
  y: number;
  fromX: number;
  fromY: number;
  vx = 0;
  vy = 0;
  peak = 0;
  still = 0;
  pressed: boolean;
  /** Thời điểm lần báo gần nhất (giây, đồng hồ của người gọi). */
  private time: number;
  private moved = false;
  private quiet = 0;

  constructor(x: number, y: number, pressed: boolean, time: number) {
    this.x = this.fromX = x;
    this.y = this.fromY = y;
    this.pressed = pressed;
    this.time = time;
  }

  move(x: number, y: number, pressed: boolean, time: number): void {
    const dt = time - this.time;
    const dx = x - this.x;
    const dy = y - this.y;
    if (dt > 0 && dt <= MAX_GAP) {
      const k = Math.min(1, dt / SMOOTHING);
      this.vx += (dx / dt - this.vx) * k;
      this.vy += (dy / dt - this.vy) * k;
      this.peak = Math.max(this.peak, Math.hypot(dx, dy) / dt);
    } else if (dt > MAX_GAP) {
      // Lâu rồi mới báo lại (vòng lặp vừa chạy lại): vận tốc cũ không còn đúng.
      this.vx = 0;
      this.vy = 0;
    }
    // Vừa đổi chỗ thì không còn là đứng yên, kể cả trong bước mô phỏng đang tới.
    if (dx !== 0 || dy !== 0) {
      this.moved = true;
      this.still = 0;
    }
    this.x = x;
    this.y = y;
    this.pressed = pressed;
    this.time = time;
    this.quiet = 0;
  }

  /** Sau mỗi bước mô phỏng: bắt đầu đoạn quét mới, đếm thời gian đứng yên. */
  settle(dt: number): void {
    this.fromX = this.x;
    this.fromY = this.y;
    this.peak = 0;
    this.still = this.moved ? 0 : this.still + dt;
    this.moved = false;
    this.quiet += dt;
    if (this.quiet >= STOP_AFTER) {
      this.vx = 0;
      this.vy = 0;
    }
  }

  /** Đổi sang toạ độ overlay mới (đổi màn hình): `p * scale + (x, y)`. */
  remap(scale: number, x: number, y: number): void {
    this.x = this.x * scale + x;
    this.y = this.y * scale + y;
    this.fromX = this.x;
    this.fromY = this.y;
    this.vx = 0;
    this.vy = 0;
    this.peak = 0;
  }
}

/** Khoảng cách từ đoạn thẳng (ax, ay)–(bx, by) tới hình chữ nhật [left, right] × [top, bottom]. */
export function segmentGap(
  ax: number,
  ay: number,
  bx: number,
  by: number,
  box: { left: number; right: number; top: number; bottom: number },
): number {
  const gapAt = (x: number, y: number) =>
    Math.hypot(Math.max(0, box.left - x, x - box.right), Math.max(0, box.top - y, y - box.bottom));
  // Đoạn ngắn (một lần báo vị trí đi được vài chục pixel): lấy mẫu dày là đủ chính xác.
  const steps = Math.max(1, Math.ceil(Math.hypot(bx - ax, by - ay) / 4));
  let best = Number.POSITIVE_INFINITY;
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    best = Math.min(best, gapAt(ax + (bx - ax) * t, ay + (by - ay) * t));
    if (best === 0) break;
  }
  return best;
}
