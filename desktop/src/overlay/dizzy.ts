import { DEFAULT_OUTLINE } from "@tinyworld/core";
import { TUNING } from "@tinyworld/sim";

/** Lảo đảo: đỉnh đầu lệch sang mỗi bên tối đa bấy nhiêu phần khoảng cách từ chân tới đỉnh đầu. */
const SWAY_LEAN = 0.12;
/** Số lần lảo đảo qua lại mỗi giây. */
const SWAY_HZ = 1.4;
/** Lảo đảo mạnh nhất trong phần đầu, từ đây tới lúc hết choáng thì dịu dần về đứng thẳng. */
const SWAY_STEADY = 0.4;
const STAR_COUNT = 3;
/** Số vòng sao bay quanh đầu mỗi giây. */
const STAR_TURNS = 1.2;
/** Vòng sao nằm dưới đỉnh đầu bấy nhiêu phần chiều cao, để vòng quanh đầu chứ không quanh tai/ăng-ten. */
const RING_DROP = 0.06;

/**
 * Đỉnh đầu lệch bao nhiêu pixel của frame lúc choáng (dương: sang phải màn hình).
 * `height`: từ chân tới đỉnh đầu, cũng tính bằng pixel của frame. Làm tròn theo pixel để pixel art
 * nghiêng thành bậc gọn chứ không lăn tăn.
 */
export function dizzyLean(time: number, height: number): number {
  const remaining = Math.max(0, 1 - time / TUNING.dizzyTime);
  const strength = Math.min(1, remaining / (1 - SWAY_STEADY));
  return Math.round(height * SWAY_LEAN * strength * Math.sin(2 * Math.PI * SWAY_HZ * time));
}

/** Hàng pixel (của frame) mà vòng sao bay quanh. */
export function ringRow(headTop: number, height: number): number {
  return headTop + Math.round(height * RING_DROP);
}

// Sao vẽ theo từng pixel: o viền, y vàng, w sáng, d vàng tối (sao phía sau, xa hơn).
const COLORS: Record<string, string> = { o: DEFAULT_OUTLINE, y: "#ffd84d", w: "#fff7d6", d: "#d4a53a" };
const NEAR = ["...o...", "..oyo..", "oooyooo", "oyywyyo", ".oyyyo.", ".oyoyo.", ".oo.oo."];
const FAR = ["..o..", ".odo.", "odddo", ".odo.", "..o.."];
const PAD = Math.floor(NEAR.length / 2);

/**
 * Sao bay vòng quanh đầu lúc choáng. Nửa vòng phía sau vẽ trên lớp nằm dưới pet (bị đầu che, sao nhỏ
 * và tối hơn), nửa phía trước vẽ trên lớp nằm trên pet. Vẽ theo pixel của frame, CSS phóng to kiểu
 * pixel art như chính pet.
 */
export class DizzyStars {
  private readonly back: HTMLCanvasElement;
  private readonly front: HTMLCanvasElement;
  /** Bán kính vòng sao theo chiều ngang / dọc, pixel của frame. */
  private rx = 0;
  private ry = 0;
  private scale = 1;
  private drawnKey = "";
  private placedKey = "";

  /** Chèn hai lớp ngay dưới và ngay trên canvas của pet; gọi `setFrameWidth` rồi `setScale` trước khi dùng. */
  constructor(pet: HTMLElement) {
    this.back = layer();
    this.front = layer();
    pet.before(this.back);
    pet.after(this.front);
  }

  /** Vòng sao rộng theo frame của pack; gọi `setScale` sau đó. */
  setFrameWidth(frameWidth: number): void {
    this.rx = Math.max(6, Math.round(frameWidth / 4));
    this.ry = Math.max(2, Math.round(this.rx / 4));
    for (const canvas of [this.back, this.front]) {
      canvas.width = 2 * this.rx + 2 * PAD + 1;
      canvas.height = 2 * this.ry + 2 * PAD + 1;
    }
    this.drawnKey = "";
  }

  setScale(scale: number): void {
    this.scale = scale;
    for (const canvas of [this.back, this.front]) {
      canvas.style.width = `${canvas.width * scale}px`;
      canvas.style.height = `${canvas.height * scale}px`;
    }
    this.placedKey = "";
  }

  hide(): void {
    this.back.hidden = true;
    this.front.hidden = true;
    this.drawnKey = "";
    this.placedKey = "";
  }

  /** `(x, y)`: tâm vòng sao, CSS pixel của overlay; `time`: số giây đã choáng. */
  update(time: number, x: number, y: number): void {
    this.back.hidden = false;
    this.front.hidden = false;
    const stars = orbit(time, this.rx, this.ry);
    const key = stars.map((s) => `${s.x},${s.y}`).join(";");
    if (key !== this.drawnKey) {
      this.drawnKey = key;
      this.draw(stars);
    }
    const left = snap(x - (this.rx + PAD) * this.scale);
    const top = snap(y - (this.ry + PAD) * this.scale);
    const placed = `${left},${top}`;
    if (placed === this.placedKey) return;
    this.placedKey = placed;
    const transform = `translate(${left}px, ${top}px)`;
    this.back.style.transform = transform;
    this.front.style.transform = transform;
  }

  private draw(stars: Star[]): void {
    const cx = this.rx + PAD;
    const cy = this.ry + PAD;
    const back = clear(this.back);
    const front = clear(this.front);
    for (const star of stars) {
      const art = star.near ? NEAR : FAR;
      paint(star.near ? front : back, art, cx + star.x, cy + star.y);
    }
  }
}

interface Star {
  /** Lệch khỏi tâm vòng, pixel của frame. */
  x: number;
  y: number;
  /** Đang ở nửa vòng phía trước (phía dưới trên màn hình, gần người xem). */
  near: boolean;
}

function orbit(time: number, rx: number, ry: number): Star[] {
  return Array.from({ length: STAR_COUNT }, (_, i) => {
    const angle = 2 * Math.PI * (STAR_TURNS * time + i / STAR_COUNT);
    const depth = Math.sin(angle);
    return { x: Math.round(rx * Math.cos(angle)), y: Math.round(ry * depth), near: depth >= 0 };
  });
}

function layer(): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.className = "pet-stars";
  canvas.hidden = true;
  return canvas;
}

function clear(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Không tạo được canvas cho hiệu ứng choáng.");
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  return ctx;
}

/** Vẽ hình sao `art` sao cho tâm hình nằm ở `(x, y)`. */
function paint(ctx: CanvasRenderingContext2D, art: string[], x: number, y: number): void {
  const half = Math.floor(art.length / 2);
  art.forEach((row, dy) => {
    [...row].forEach((cell, dx) => {
      const color = COLORS[cell];
      if (!color) return;
      ctx.fillStyle = color;
      ctx.fillRect(x - half + dx, y - half + dy, 1, 1);
    });
  });
}

/** Làm tròn theo pixel thật của màn hình để sao không bị nhoè khi di chuyển. */
function snap(value: number): number {
  const dpr = window.devicePixelRatio || 1;
  return Math.round(value * dpr) / dpr;
}
