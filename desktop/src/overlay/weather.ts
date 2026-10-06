import type { Point } from "@tinyworld/core";
import { ANIMATED_SKIES, type Sky } from "@tinyworld/sim";

/** Vẽ hiệu ứng thời tiết chừng này lần mỗi giây: hạt mưa, tuyết nhỏ, 12 fps là đủ, đỡ tốn CPU. */
export const EFFECT_FPS = 12;
const EFFECT_MS = 1000 / EFFECT_FPS;
/**
 * Vùng có thời tiết quanh pet: rộng gấp chừng này lần bề ngang, cao gấp chừng này lần chiều cao pet tính từ
 * mặt đất (chỗ chân pet) lên. Không vẽ gì dưới mặt đất: thò xuống thì thành vệt trên taskbar hay trên cửa
 * sổ nằm dưới.
 */
const AREA_WIDTH = 3;
const AREA_HEIGHT = 2;
/** Chớp lúc có sấm kéo dài chừng này ms. */
const FLASH_MS = 160;

/**
 * Các lớp sương mù, vẽ từ lớp sau ra lớp trước: đám sương bồng bềnh đáy nằm trên mặt đất, ngay chỗ pet cao
 * hơn mặt đất chừng `rise` lần chiều cao pet (mấp mô thêm `bumps` lần) rồi thấp dần ra hai bên, từng cụm to
 * chừng `puff` lần chiều cao pet, trôi ngang `drift` lần bề ngang pet mỗi giây (âm là sang trái), độ đậm
 * `density`.
 */
const FOG_LAYERS: { rise: number; bumps: number; puff: number; drift: number; density: number }[] = [
  { rise: 0.48, bumps: 0.1, puff: 0.2, drift: -0.05, density: 0.22 },
  { rise: 0.24, bumps: 0.08, puff: 0.15, drift: 0.08, density: 0.42 },
];
/** Ụ sương thấp dần ra hai bên: gần tới mép vùng vẽ còn chừng này phần độ cao ở giữa. */
const FOG_SPREAD = 0;
/** Màu sương và viền: trắng hơi xanh, viền xám xanh để thấy được trên cả nền sáng lẫn tối. */
const FOG_FILL = "#e6ecf7";
const FOG_EDGE = "#8a9cc0";

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
  /** Cỡ pet (CSS pixel). */
  private petWidth = 0;
  private petHeight = 0;
  private lastDraw = 0;
  private flashUntil = 0;
  private shown = false;
  /** Canvas phụ để vẽ từng lớp sương mù. */
  private fogLayer: HTMLCanvasElement | null = null;

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
    this.petWidth = petWidth;
    this.petHeight = petHeight;
    const left = Math.round(foot.x - width / 2);
    // Mép dưới vùng vẽ là mặt đất.
    const top = Math.round(foot.y) - height;
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
      y: anywhere ? Math.random() * this.height : -Math.random() * 20,
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
      if (p.y >= this.height) Object.assign(p, this.spawn(sky, false));
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
      const room = this.height - y;
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

  /**
   * Sương mù: vài lớp đám sương bồng bềnh (viền mảnh, đáy nằm trên mặt đất) trôi ngược chiều nhau quanh chân
   * pet, tan dần ra hai bên. Mỗi lớp vẽ đặc vào canvas phụ rồi mới phủ lên với độ đậm của lớp, để chỗ các cụm
   * chồng lên nhau không đậm hơn.
   */
  private drawFog(now: number): void {
    const layer = (this.fogLayer ??= document.createElement("canvas"));
    const dpr = window.devicePixelRatio || 1;
    if (layer.width !== this.element.width || layer.height !== this.element.height) {
      layer.width = this.element.width;
      layer.height = this.element.height;
    }
    const lctx = layer.getContext("2d");
    if (!lctx) return;
    const seconds = now / 1000;
    const { width, height, petWidth, petHeight } = this;
    FOG_LAYERS.forEach((fog, i) => {
      lctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      lctx.clearRect(0, 0, width, height);
      const radius = fog.puff * petHeight;
      const gap = radius * 1.3;
      const count = Math.ceil(width / gap) + 2;
      const period = count * gap;
      const offset = (((seconds * fog.drift * petWidth) % period) + period) % period;
      const puffs: [number, number, number][] = [];
      for (let k = 0; k < count; k++) {
        // Cụm thứ k luôn cao thấp, to nhỏ như nhau (không ngẫu nhiên mỗi lần vẽ), trôi vòng lại.
        const x = ((k * gap + offset) % period) - gap;
        const r = radius * (0.8 + 0.4 * (0.5 + 0.5 * Math.sin(k * 1.71 + i)));
        // Cao nhất ở giữa (chỗ pet), thấp dần ra hai bên.
        const across = (x - width / 2) / (width / 2);
        const mound = FOG_SPREAD + (1 - FOG_SPREAD) * Math.cos((Math.min(1, Math.abs(across) * 1.1) * Math.PI) / 2);
        const bump = (0.5 + 0.5 * Math.sin(k * 2.39 + i * 1.3)) * fog.bumps;
        const y = height - (fog.rise - bump) * petHeight * mound + r * 0.4;
        puffs.push([x, Math.min(y, height - r * 0.3), r]);
      }
      // Viền: hình sương nở thêm một chút, tô màu viền; rồi thân sương tô đè lên. Dưới mỗi cụm tô kín xuống
      // tới mặt đất.
      for (const [grow, color] of [[1.2, FOG_EDGE], [0, FOG_FILL]] as const) {
        lctx.fillStyle = color;
        lctx.beginPath();
        for (const [x, y, r] of puffs) {
          lctx.moveTo(x + r + grow, y);
          lctx.arc(x, y, r + grow, 0, Math.PI * 2);
          lctx.rect(x - r - grow, y, 2 * (r + grow), height - y);
        }
        lctx.fill();
      }
      // Tan dần ra hai bên.
      lctx.globalCompositeOperation = "destination-in";
      const fade = lctx.createLinearGradient(0, 0, width, 0);
      fade.addColorStop(0, "rgba(0, 0, 0, 0)");
      fade.addColorStop(0.15, "rgba(0, 0, 0, 1)");
      fade.addColorStop(0.85, "rgba(0, 0, 0, 1)");
      fade.addColorStop(1, "rgba(0, 0, 0, 0)");
      lctx.fillStyle = fade;
      lctx.fillRect(0, 0, width, height);
      lctx.globalCompositeOperation = "source-over";
      this.ctx.globalAlpha = fog.density;
      this.ctx.drawImage(layer, 0, 0, width, height);
      this.ctx.globalAlpha = 1;
    });
  }
}
