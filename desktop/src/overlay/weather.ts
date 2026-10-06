import type { Point } from "@tinyworld/core";
import { ANIMATED_SKIES, type Sky } from "@tinyworld/sim";

/** Vẽ hiệu ứng thời tiết chừng này lần mỗi giây: hạt mưa, tuyết nhỏ, 12 fps là đủ, đỡ tốn CPU. */
export const EFFECT_FPS = 12;
const EFFECT_MS = 1000 / EFFECT_FPS;
/** Vùng có thời tiết quanh pet: rộng gấp chừng này lần bề ngang, cao gấp chừng này lần chiều cao pet. */
const AREA_WIDTH = 3;
const AREA_HEIGHT = 2.2;
/**
 * Vùng còn chừa xuống dưới chân pet chừng này lần chiều cao pet, cho sương mù quanh chân. Mưa, tuyết, cánh
 * hoa thì dừng ở mặt đất (chỗ chân pet): rơi tiếp xuống phần này thì thành vệt trên taskbar hay trên cửa sổ
 * nằm dưới.
 */
const BELOW_FEET = 0.25;
/** Chớp lúc có sấm kéo dài chừng này ms. */
const FLASH_MS = 160;

interface Particle {
  x: number;
  y: number;
  /** Tốc độ rơi (px/s). */
  speed: number;
  /** Cỡ: chiều dài hạt mưa, cạnh bông tuyết/cánh hoa (CSS pixel). */
  size: number;
  /** Pha đung đưa (tuyết, cánh hoa). */
  phase: number;
}

/** Số hạt, tốc độ rơi [min, max], cỡ [min, max] của từng kiểu thời tiết. */
const KINDS: Partial<Record<Sky, { count: number; speed: [number, number]; size: [number, number] }>> = {
  rain: { count: 16, speed: [380, 460], size: [7, 10] },
  storm: { count: 24, speed: [460, 560], size: [8, 12] },
  snow: { count: 14, speed: [25, 45], size: [2, 3] },
  petals: { count: 7, speed: [22, 38], size: [3, 4] },
};

const PETAL_COLORS = ["#ffb7d0", "#ff9ec2", "#ffd1e0"];

/**
 * Thời tiết chỉ quanh một pet (mưa, tuyết, sương mù, sấm, cánh hoa): canvas nhỏ đi theo pet, hạt mờ dần ra
 * hai bên và lên trên nên trông như chỉ có quanh pet. Chỉ vẽ `EFFECT_FPS` lần mỗi giây; trời quang, nhiều
 * mây, pet đang ngủ hay tắt hiệu ứng thì ẩn hẳn, không vẽ gì.
 */
export class WeatherEffect {
  readonly element: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private sky: Sky | null = null;
  private particles: Particle[] = [];
  /** Cỡ vùng đang vẽ (CSS pixel). */
  private width = 0;
  private height = 0;
  /** Mặt đất (chỗ chân pet), tính từ mép trên vùng vẽ (CSS pixel). */
  private ground = 0;
  private lastDraw = 0;
  private flashUntil = 0;
  private shown = false;

  constructor() {
    this.element = document.createElement("canvas");
    this.element.className = "weather";
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas thời tiết.");
    this.ctx = ctx;
    this.element.hidden = true;
  }

  /** Có đang vẽ (vòng lặp phải thức dậy đủ `EFFECT_FPS` lần mỗi giây). */
  get animating(): boolean {
    return this.shown;
  }

  /** Đổi thời tiết; `null` là tắt hiệu ứng. */
  setSky(sky: Sky | null): void {
    if (sky === this.sky) return;
    this.sky = sky;
    this.particles = [];
  }

  /** Sấm: chớp sáng một cái. */
  flash(now: number): void {
    this.flashUntil = now + FLASH_MS;
  }

  /**
   * Đặt vùng thời tiết quanh pet (điểm chân `foot`, cỡ pet `petWidth` × `petHeight`), vẽ lại nếu tới lượt.
   * `visible`: pet đang thức (ngủ thì không vẽ, vòng lặp được dừng).
   */
  update(foot: Point, petWidth: number, petHeight: number, now: number, visible: boolean): void {
    const sky = this.sky;
    const show = visible && sky !== null && ANIMATED_SKIES.has(sky);
    if (show !== this.shown) {
      this.shown = show;
      this.element.hidden = !show;
    }
    if (!show || !sky) return;
    const width = Math.round(petWidth * AREA_WIDTH);
    const height = Math.round(petHeight * AREA_HEIGHT);
    if (width !== this.width || height !== this.height) this.resize(width, height);
    const left = Math.round(foot.x - width / 2);
    const top = Math.round(foot.y - height + petHeight * BELOW_FEET);
    this.ground = foot.y - top;
    this.element.style.transform = `translate(${left}px, ${top}px)`;
    if (now - this.lastDraw < EFFECT_MS) return;
    const dt = Math.min(0.2, (now - this.lastDraw) / 1000);
    this.lastDraw = now;
    this.step(sky, dt, now);
    this.draw(sky, now);
  }

  private resize(width: number, height: number): void {
    this.width = width;
    this.height = height;
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(width * dpr);
    this.element.height = Math.round(height * dpr);
    this.element.style.width = `${width}px`;
    this.element.style.height = `${height}px`;
    this.particles = [];
  }

  private spawn(sky: Sky, anywhere: boolean): Particle {
    const kind = KINDS[sky];
    const [s0, s1] = kind?.speed ?? [0, 0];
    const [z0, z1] = kind?.size ?? [0, 0];
    return {
      x: Math.random() * this.width,
      y: anywhere ? Math.random() * this.ground : -Math.random() * 20,
      speed: s0 + Math.random() * (s1 - s0),
      size: z0 + Math.random() * (z1 - z0),
      phase: Math.random() * Math.PI * 2,
    };
  }

  private step(sky: Sky, dt: number, now: number): void {
    const kind = KINDS[sky];
    if (!kind) return;
    while (this.particles.length < kind.count) this.particles.push(this.spawn(sky, true));
    const sway = sky === "snow" ? 10 : sky === "petals" ? 18 : 0;
    for (const p of this.particles) {
      p.y += p.speed * dt;
      if (sway) p.x += Math.sin(now / 700 + p.phase) * sway * dt;
      // Chạm mặt đất thì hết, rơi lại từ trên.
      if (p.y >= this.ground) Object.assign(p, this.spawn(sky, false));
    }
  }

  /** Hạt ở giữa vùng (trên đầu pet) rõ nhất, mờ dần ra hai bên và lên trên. */
  private fade(x: number, y: number): number {
    const dx = (x - this.width / 2) / (this.width / 2);
    const across = Math.max(0, 1 - dx * dx);
    const down = Math.min(1, (y / this.height) * 2.5);
    return across * down;
  }

  private draw(sky: Sky, now: number): void {
    const { ctx } = this;
    const dpr = window.devicePixelRatio || 1;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, this.width, this.height);
    if (sky === "fog") {
      this.drawFog(now);
      return;
    }
    for (const p of this.particles) {
      const alpha = this.fade(p.x, p.y);
      if (alpha <= 0.02) continue;
      const x = Math.round(p.x);
      const y = Math.round(p.y);
      // Phần chạm xuống dưới mặt đất thì cắt bỏ.
      const room = this.ground - y;
      if (room <= 0) continue;
      if (sky === "rain" || sky === "storm") {
        // Xanh vừa phải để thấy được trên cả hình nền sáng lẫn tối.
        ctx.fillStyle = `rgba(110, 160, 235, ${0.85 * alpha})`;
        ctx.fillRect(x, y, 1.5, Math.min(p.size, room));
      } else if (sky === "snow") {
        ctx.fillStyle = `rgba(255, 255, 255, ${0.9 * alpha})`;
        ctx.fillRect(x, y, p.size, Math.min(p.size, room));
      } else {
        ctx.globalAlpha = alpha;
        ctx.fillStyle = PETAL_COLORS[Math.floor(p.phase) % PETAL_COLORS.length];
        ctx.fillRect(x, y, p.size, Math.min(p.size * 0.6, room));
        ctx.globalAlpha = 1;
      }
    }
    if (sky === "storm" && now < this.flashUntil) {
      const left = (this.flashUntil - now) / FLASH_MS;
      const glow = ctx.createRadialGradient(this.width / 2, this.height * 0.6, 0, this.width / 2, this.height * 0.6, this.width / 2);
      glow.addColorStop(0, `rgba(255, 255, 235, ${0.55 * left})`);
      glow.addColorStop(1, "rgba(255, 255, 235, 0)");
      ctx.fillStyle = glow;
      ctx.fillRect(0, 0, this.width, this.height);
    }
  }

  /** Sương mù: vài dải mờ trôi chậm quanh chân pet. */
  private drawFog(now: number): void {
    const { ctx } = this;
    const base = this.height * 0.82;
    for (let i = 0; i < 3; i++) {
      const drift = Math.sin(now / 4000 + i * 2.1) * this.width * 0.12;
      const x = this.width * (0.3 + 0.2 * i) + drift;
      const y = base - i * this.height * 0.08;
      const rx = this.width * 0.32;
      const ry = this.height * 0.07;
      const glow = ctx.createRadialGradient(x, y, 0, x, y, rx);
      glow.addColorStop(0, "rgba(225, 230, 240, 0.32)");
      glow.addColorStop(1, "rgba(225, 230, 240, 0)");
      ctx.fillStyle = glow;
      ctx.beginPath();
      ctx.ellipse(x, y, rx, ry, 0, 0, Math.PI * 2);
      ctx.fill();
    }
  }
}
