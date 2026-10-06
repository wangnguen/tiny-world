import type { Point, Rect } from "@tinyworld/core";
import type { Sky, Warmth } from "@tinyworld/sim";

/**
 * Mưa, tuyết, cánh hoa rơi nhanh, khói thở bay nhanh: vẽ 12 lần mỗi giây. Nắng, sao, mây, sương, hơi nóng
 * trôi chậm: 8 lần là đủ, đỡ tốn CPU.
 */
const FAST_FPS = 12;
const SLOW_FPS = 8;
const FAST_SKIES: ReadonlySet<Sky> = new Set(["rain", "storm", "snow", "petals"]);
/**
 * Vùng có thời tiết quanh pet, theo thân thật của con đó (`PetView.body`, không theo khung ảnh): rộng gấp
 * chừng này lần cạnh lớn của thân, phía trên đầu chừa chừng này lần (cho mây, sao). Mép dưới là mặt đất,
 * không vẽ gì dưới đó: thò xuống thì thành vệt trên taskbar hay trên cửa sổ nằm dưới.
 */
const AREA_WIDTH = 3;
const ABOVE_HEAD = 1;
/** Chớp lúc có sấm kéo dài chừng này ms. */
const FLASH_MS = 160;
/** Trời quang ban đêm: sao băng cách nhau ngẫu nhiên trong khoảng này (ms), bay qua trong chừng này ms. */
const METEOR_GAP: [number, number] = [7_000, 16_000];
const METEOR_MS = 900;

/** Cách hạt di chuyển: rơi từ trên xuống, trôi ngang từ trái sang phải, đứng yên lấp lánh rồi hiện chỗ khác. */
type Motion = "fall" | "drift" | "twinkle";
/**
 * Chỗ hạt hiện: cả vùng, quanh chân (sương), trên đầu (sao, mây), quanh thân (lấp lánh lúc nắng). Tính theo
 * thân thật của pet nên con cao con thấp đều vừa.
 */
type Zone = "all" | "feet" | "sky" | "around";

interface Particle {
  x: number;
  y: number;
  /** Tốc độ rơi hay trôi ngang (px/s). */
  speed: number;
  /** Cỡ: chiều dài hạt mưa, vệt sương, cạnh bông tuyết/cánh hoa, tia lấp lánh, ô pixel của mây (CSS pixel). */
  size: number;
  /** Pha đung đưa (tuyết, cánh hoa), chọn màu cánh hoa, hình đám mây. */
  phase: number;
  /** Hạt lấp lánh: đã hiện bao lâu, hiện tổng cộng bao lâu (giây). */
  age: number;
  life: number;
}

/**
 * Từng kiểu thời tiết: cách di chuyển, chỗ hiện, số hạt, tốc độ [min, max] (px/s), cỡ [min, max] (CSS pixel),
 * thời gian sống [min, max] (giây, hạt lấp lánh).
 */
const KINDS: Record<
  Sky,
  { motion: Motion; zone: Zone; count: number; speed: [number, number]; size: [number, number]; life?: [number, number] }
> = {
  rain: { motion: "fall", zone: "all", count: 16, speed: [380, 460], size: [7, 10] },
  storm: { motion: "fall", zone: "all", count: 24, speed: [460, 560], size: [8, 12] },
  snow: { motion: "fall", zone: "all", count: 16, speed: [22, 42], size: [1.5, 3.5] },
  petals: { motion: "fall", zone: "all", count: 7, speed: [22, 38], size: [3, 5] },
  fog: { motion: "drift", zone: "feet", count: 14, speed: [8, 18], size: [8, 18] },
  cloudy: { motion: "drift", zone: "sky", count: 3, speed: [5, 9], size: [0, 0] },
  sunny: { motion: "twinkle", zone: "around", count: 7, speed: [0, 0], size: [2, 4], life: [1.2, 2.2] },
  clear: { motion: "twinkle", zone: "sky", count: 8, speed: [0, 0], size: [1, 3], life: [2.5, 4.5] },
};
/** Gió thổi mưa xiên: đi xuống 1 px thì sang phải chừng này px. */
const WIND: Partial<Record<Sky, number>> = { rain: 0.08, storm: 0.28 };

const PETAL_COLORS = ["#ffb7d0", "#ff9ec2", "#ffd1e0"];

/** Mây pixel: `X` thân, `S` bóng phía dưới, chấm là trong suốt; vẽ thêm viền quanh. */
const CLOUD_SHAPES = [
  ["....XXX.....", "..XXXXXXX.X.", ".XXXXXXXXXXX", "XXXXXXXXXXXX", ".SSSSSSSSSS."],
  ["...XX..XXX....", "..XXXXXXXXXX..", ".XXXXXXXXXXXX.", "XXXXXXXXXXXXXX", ".SSSSSSSSSSSS."],
  ["..XXX....", ".XXXXXXX.", "XXXXXXXXX", ".SSSSSSS."],
];
const CLOUD_COLORS: Record<string, string> = { X: "#f4f7fd", S: "#c3cfe6" };
const CLOUD_EDGE = "#7b8db3";
/** Mỗi ô pixel của mây to bằng chừng này phần cạnh lớn của thân pet (ít nhất 2 px). */
const CLOUD_CELL = 1 / 32;

/** Tia nắng xiên từ trên trái xuống: đi xuống 1 px thì sang phải chừng này px. */
const SUN_SLANT = 0.45;

/**
 * Hơi nóng bốc lên hai bên thân: số vệt, tốc độ bốc lên [min, max] (px/s), dài [min, max] (phần chiều cao
 * thân).
 */
const HEAT = { count: 6, speed: [16, 26] as [number, number], size: [0.17, 0.25] as [number, number] };

/** Một giọt mưa bắn lên lúc chạm đất. */
interface Splash {
  x: number;
  y: number;
  vx: number;
  vy: number;
}
const SPLASH_GRAVITY = 420;

/**
 * Hơi thở lúc lạnh: mỗi lần thở ra phả hơi trong chừng này ms; thở cách nhau chừng này ms lúc đứng yên, ngắn
 * dần khi đi, chạy (thở dồn).
 */
const EXHALE_MS = 600;
const BREATH_REST_MS = 3_400;
const BREATH_RUN_MS = 1_400;
/** Mỗi giây thở ra chừng này hạt hơi nước. */
const VAPOR_RATE = 24;

/** Một hạt hơi nước thở ra, toạ độ trong vùng vẽ. */
interface Vapor {
  x: number;
  y: number;
  vx: number;
  vy: number;
  age: number;
  life: number;
  /** Bán kính lúc mới thở ra và lúc tan hẳn (CSS pixel). */
  from: number;
  to: number;
  seed: number;
}

/** Miệng pet (chỗ thở ra khói), CSS pixel của overlay, và hướng mặt (1: sang phải màn hình). */
export type Mouth = Point & { dir: 1 | -1 };

/** Một đốm hơi nước mềm (trắng giữa, tan dần ra mép), vẽ sẵn một lần rồi phóng to nhỏ. */
let vaporSprite: HTMLCanvasElement | null = null;
function softPuff(): HTMLCanvasElement {
  if (vaporSprite) return vaporSprite;
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext("2d");
  if (ctx) {
    const glow = ctx.createRadialGradient(32, 32, 0, 32, 32, 32);
    // Giữa trắng, mép hơi xám xanh để thấy được cả trên nền sáng.
    glow.addColorStop(0, "rgba(240, 244, 251, 1)");
    glow.addColorStop(0.45, "rgba(212, 222, 238, 0.65)");
    glow.addColorStop(1, "rgba(200, 212, 232, 0)");
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, 64, 64);
  }
  vaporSprite = canvas;
  return canvas;
}

/**
 * Thời tiết chỉ quanh một pet: canvas nhỏ đi theo pet, vừa với thân thật của con đó; hạt mờ dần ra hai bên và
 * lên trên nên trông như chỉ có quanh pet. Mưa (giọt bắn lên lúc chạm đất), tuyết, cánh hoa, sương mù, sấm;
 * nắng (tia nắng, lấp lánh), trời quang ban đêm (sao, sao băng), nhiều mây (mây pixel trôi trên đầu); nóng thì
 * hơi nóng bốc lên hai bên thân, lạnh thì pet thở ra hơi nước từ miệng. Pet đang ngủ hay tắt hiệu ứng thì ẩn
 * hẳn, không vẽ gì.
 */
export class WeatherEffect {
  readonly element: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private sky: Sky | null = null;
  private warmth: Warmth | null = null;
  private particles: Particle[] = [];
  private splashes: Splash[] = [];
  private heat: Particle[] = [];
  private vapor: Vapor[] = [];
  /** Cỡ vùng đang vẽ (CSS pixel). */
  private width = 0;
  private height = 0;
  /** Góc trên trái vùng vẽ (CSS pixel của overlay). */
  private left = 0;
  private top = 0;
  /** Thân pet trong vùng vẽ: đỉnh đầu, hai mép, chiều cao (CSS pixel). */
  private headTop = 0;
  private bodyLeft = 0;
  private bodyRight = 0;
  private bodyHeight = 0;
  /** Cạnh lớn của thân pet (CSS pixel). */
  private unit = 0;
  private lastDraw = 0;
  private flashUntil = 0;
  /** Sao băng đang bay từ lúc này (ms), lần tới lúc nào; 0 là chưa hẹn. */
  private meteorAt = Number.NEGATIVE_INFINITY;
  private meteorNext = 0;
  private meteorFrom: Point = { x: 0, y: 0 };
  private meteorDir: 1 | -1 = 1;
  /** Thở: lần thở ra tới lúc nào, lần đang thở ra từ lúc nào tới lúc nào (ms), số hạt hơi nước còn nợ. */
  private nextBreath = 0;
  private exhaleFrom = 0;
  private exhaleUntil = 0;
  private vaporDue = 0;
  private shown = false;
  /** Mây pixel đã vẽ sẵn theo hình và cỡ ô. */
  private readonly clouds = new Map<string, HTMLCanvasElement>();

  constructor() {
    this.element = document.createElement("canvas");
    this.element.className = "weather";
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas thời tiết.");
    this.ctx = ctx;
    this.element.hidden = true;
  }

  /** Có đang vẽ (vòng lặp phải thức dậy đủ `frameMs`). */
  get animating(): boolean {
    return this.shown;
  }

  /** Vòng lặp phải vẽ lại trong vòng chừng này ms; `Infinity` nếu đang ẩn. */
  get frameMs(): number {
    if (!this.shown) return Number.POSITIVE_INFINITY;
    const fast = (this.sky !== null && FAST_SKIES.has(this.sky)) || this.warmth === "cold";
    return 1000 / (fast ? FAST_FPS : SLOW_FPS);
  }

  /** Đổi thời tiết; `null` là tắt hiệu ứng. */
  setSky(sky: Sky | null): void {
    if (sky === this.sky) return;
    this.sky = sky;
    this.particles = [];
    this.splashes = [];
    this.meteorNext = 0;
  }

  /** Nóng (hơi nóng bốc lên), lạnh (thở ra hơi nước) hay bình thường. */
  setWarmth(warmth: Warmth | null): void {
    if (warmth === this.warmth) return;
    this.warmth = warmth;
    this.heat = [];
    this.vapor = [];
    this.nextBreath = 0;
    this.exhaleUntil = 0;
  }

  /** Sấm: chớp sáng một cái. */
  flash(now: number): void {
    this.flashUntil = now + FLASH_MS;
  }

  /**
   * Đặt vùng thời tiết quanh thân pet `body` (`PetView.body`), vẽ lại nếu tới lượt. `visible`: pet đang thức
   * (ngủ thì không vẽ, vòng lặp được dừng). `mouth`: miệng ở frame đang vẽ. `effort`: 0 đứng yên, 1 chạy (thở
   * dồn hơn).
   */
  update(body: Rect, now: number, visible: boolean, mouth: Mouth | null = null, effort = 0): void {
    const show = visible && (this.sky !== null || this.warmth !== null);
    if (show !== this.shown) {
      this.shown = show;
      this.element.hidden = !show;
    }
    if (!show) return;
    const unit = Math.max(body.width, body.height);
    const width = Math.round(unit * AREA_WIDTH);
    const height = Math.round(body.height + unit * ABOVE_HEAD);
    if (width !== this.width || height !== this.height) this.resize(width, height);
    this.unit = unit;
    this.bodyHeight = body.height;
    this.left = Math.round(body.x + body.width / 2 - width / 2);
    // Mép dưới vùng vẽ là mặt đất (chân pet).
    this.top = Math.round(body.y + body.height) - height;
    this.headTop = body.y - this.top;
    this.bodyLeft = body.x - this.left;
    this.bodyRight = this.bodyLeft + body.width;
    this.element.style.transform = `translate(${this.left}px, ${this.top}px)`;
    if (now - this.lastDraw < this.frameMs) return;
    const dt = Math.min(0.2, (now - this.lastDraw) / 1000);
    this.lastDraw = now;
    this.step(dt, now, mouth, effort);
    this.draw(now);
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
    this.splashes = [];
    this.heat = [];
    this.vapor = [];
  }

  /** Khoảng [trên, dưới] theo chiều dọc của chỗ hạt hiện. */
  private zone(zone: Zone): [number, number] {
    const head = this.headTop;
    switch (zone) {
      case "feet":
        return [this.height - this.bodyHeight * 0.5, this.height - 2];
      case "sky":
        return [head * 0.06, head * 0.8];
      case "around":
        return [head * 0.6, this.height - 4];
      default:
        return [0, this.height];
    }
  }

  /** Hạt mới; `anywhere`: ở chỗ bất kỳ (lúc mới bật), không thì ở chỗ bắt đầu (trên cùng, mép trái). */
  private spawn(sky: Sky, anywhere: boolean): Particle {
    const kind = KINDS[sky];
    const [s0, s1] = kind.speed;
    const [z0, z1] = kind.size;
    const [l0, l1] = kind.life ?? [0, 0];
    const life = l0 + Math.random() * (l1 - l0);
    const p: Particle = {
      x: Math.random() * this.width,
      y: 0,
      speed: s0 + Math.random() * (s1 - s0),
      size: sky === "cloudy" ? Math.max(2, Math.round(this.unit * CLOUD_CELL)) : z0 + Math.random() * (z1 - z0),
      phase: Math.random() * Math.PI * 2,
      age: anywhere ? Math.random() * life : 0,
      life,
    };
    const [top, bottom] = this.zone(kind.zone);
    // Mây nằm hẳn trên đầu, không đè xuống đầu pet.
    const lowest = sky === "cloudy" ? Math.max(top, this.headTop - this.driftSize(sky, p).height - 2) : bottom;
    p.y = top + Math.random() * (lowest - top);
    if (kind.motion === "fall") {
      if (!anywhere) p.y = -Math.random() * 20;
      // Mưa xiên: hạt bắt đầu lệch sang trái để rơi xiên vào vùng vẽ.
      p.x -= (WIND[sky] ?? 0) * this.height * Math.random();
    }
    if (kind.motion === "drift" && !anywhere) p.x = -this.driftSize(sky, p).width - Math.random() * 20;
    return p;
  }

  /** Cỡ của hạt trôi ngang: đám mây thì theo hình mây, vệt sương thì dài `size`. */
  private driftSize(sky: Sky, p: Particle): { width: number; height: number } {
    if (sky !== "cloudy") return { width: p.size, height: 2 };
    const rows = CLOUD_SHAPES[Math.floor(p.phase) % CLOUD_SHAPES.length];
    return { width: (rows[0].length + 2) * p.size, height: (rows.length + 2) * p.size };
  }

  private step(dt: number, now: number, mouth: Mouth | null, effort: number): void {
    const sky = this.sky;
    if (sky) {
      const kind = KINDS[sky];
      while (this.particles.length < kind.count) this.particles.push(this.spawn(sky, true));
      const sway = sky === "snow" ? 10 : sky === "petals" ? 18 : 0;
      const wind = WIND[sky] ?? 0;
      for (const p of this.particles) {
        if (kind.motion === "fall") {
          p.y += p.speed * dt;
          p.x += p.speed * wind * dt;
          if (sway) p.x += Math.sin(now / 700 + p.phase) * sway * dt;
          // Chạm mặt đất thì hết (mưa thì bắn lên vài giọt), rơi lại từ trên.
          if (p.y >= this.height) {
            if (wind > 0 && this.fade(p.x, this.height) > 0.2) this.splash(p.x, sky === "storm" ? 3 : 2);
            Object.assign(p, this.spawn(sky, false));
          }
        } else if (kind.motion === "drift") {
          p.x += p.speed * dt;
          // Trôi hết sang mép phải thì hiện lại bên trái.
          if (p.x >= this.width) Object.assign(p, this.spawn(sky, false));
        } else {
          p.age += dt;
          // Lấp lánh xong thì hiện ở chỗ khác.
          if (p.age >= p.life) Object.assign(p, this.spawn(sky, false));
        }
      }
      for (const s of this.splashes) {
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        s.vy += SPLASH_GRAVITY * dt;
      }
      this.splashes = this.splashes.filter((s) => s.y < this.height);
      if (sky === "clear") {
        if (this.meteorNext === 0) this.meteorNext = now + between(METEOR_GAP);
        else if (now >= this.meteorNext) {
          this.meteorNext = now + between(METEOR_GAP);
          this.meteorAt = now;
          this.meteorDir = Math.random() < 0.5 ? 1 : -1;
          this.meteorFrom = {
            x: this.width * (this.meteorDir > 0 ? 0.1 + Math.random() * 0.3 : 0.6 + Math.random() * 0.3),
            y: this.headTop * (0.05 + Math.random() * 0.25),
          };
        }
      }
    }

    if (this.warmth === "hot") {
      while (this.heat.length < HEAT.count) this.heat.push(this.spawnHeat(true));
      for (const p of this.heat) {
        p.y -= p.speed * dt;
        // Bốc lên tới ngang đầu pet thì tan, bốc lại từ mặt đất.
        if (p.y < this.headTop) Object.assign(p, this.spawnHeat(false));
      }
    }

    if (this.warmth === "cold") this.breathe(dt, now, mouth, effort);
  }

  /** Giọt mưa chạm đất ở `x`: bắn lên `count` giọt nhỏ rồi rơi lại. */
  private splash(x: number, count: number): void {
    for (let i = 0; i < count; i++) {
      const side = i % 2 === 0 ? -1 : 1;
      this.splashes.push({ x, y: this.height - 1, vx: side * (15 + Math.random() * 25), vy: -(45 + Math.random() * 40) });
    }
  }

  /**
   * Thở ra hơi nước như người lúc trời lạnh: mỗi lần thở phả ra một luồng hạt hơi nước từ miệng, bay về phía
   * trước rồi chậm dần, bốc lên, nở to và tan. Đi, chạy thì thở dồn hơn.
   */
  private breathe(dt: number, now: number, mouth: Mouth | null, effort: number): void {
    // Vóc to thì hơi thở to, bay xa.
    const scale = this.bodyHeight / 90;
    for (const v of this.vapor) {
      v.age += dt;
      const drag = Math.exp(-1.9 * dt);
      v.vx *= drag;
      v.vy = v.vy * drag - 7 * scale * dt;
      v.x += (v.vx + Math.sin(v.age * 5 + v.seed) * 4 * scale) * dt;
      v.y += v.vy * dt;
    }
    this.vapor = this.vapor.filter((v) => v.age < v.life);
    if (!mouth) return;
    if (now >= this.nextBreath) {
      const gap = BREATH_REST_MS - (BREATH_REST_MS - BREATH_RUN_MS) * effort;
      this.nextBreath = now + gap * (0.88 + Math.random() * 0.24);
      this.exhaleFrom = now;
      this.exhaleUntil = now + EXHALE_MS * (1 - 0.35 * effort);
    }
    if (now >= this.exhaleUntil) {
      this.vaporDue = 0;
      return;
    }
    // Đầu hơi thở mạnh, cuối hơi thở yếu dần.
    const progress = (now - this.exhaleFrom) / (this.exhaleUntil - this.exhaleFrom);
    this.vaporDue += VAPOR_RATE * dt;
    const x = mouth.x - this.left;
    const y = mouth.y - this.top;
    for (; this.vaporDue >= 1; this.vaporDue--) {
      const angle = ((-8 + Math.random() * 30) * Math.PI) / 180;
      const speed = (52 + Math.random() * 20) * scale * (1 - 0.5 * progress);
      this.vapor.push({
        x: x + (Math.random() - 0.5) * 2,
        y: y + (Math.random() - 0.5) * 2,
        vx: mouth.dir * speed * Math.cos(angle),
        vy: -speed * Math.sin(angle) - 3 * scale,
        age: 0,
        life: 1.3 + Math.random() * 0.6,
        from: 1 + 0.015 * this.bodyHeight,
        to: (0.1 + Math.random() * 0.05) * this.bodyHeight,
        seed: Math.random() * Math.PI * 2,
      });
    }
  }

  /** Vệt hơi nóng mới: hai bên thân pet (không đè lên thân), từ mặt đất bốc lên. */
  private spawnHeat(anywhere: boolean): Particle {
    const reach = (this.bodyRight - this.bodyLeft) * 0.5 + 4;
    const x = Math.random() < 0.5 ? this.bodyLeft - 3 - Math.random() * reach : this.bodyRight + 3 + Math.random() * reach;
    const [s0, s1] = HEAT.speed;
    const [z0, z1] = HEAT.size;
    return {
      x,
      y: anywhere ? this.headTop + Math.random() * (this.height - this.headTop) : this.height + Math.random() * 12,
      speed: s0 + Math.random() * (s1 - s0),
      size: Math.max(10, (z0 + Math.random() * (z1 - z0)) * this.bodyHeight),
      phase: Math.random() * Math.PI * 2,
      age: 0,
      life: 0,
    };
  }

  /** Hạt ở giữa vùng (quanh pet) rõ nhất, mờ dần ra hai bên; `fromTop`: mờ dần cả lên trên. */
  private fade(x: number, y: number, fromTop = true): number {
    const dx = (x - this.width / 2) / (this.width / 2);
    const across = Math.max(0, 1 - dx * dx);
    const down = fromTop ? Math.min(1, (y / this.height) * 2.5) : 1;
    return across * down;
  }

  private draw(now: number): void {
    const { ctx } = this;
    const dpr = window.devicePixelRatio || 1;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, this.width, this.height);
    const sky = this.sky;
    if (sky === "sunny") this.drawRays(now);
    if (sky === "clear") this.drawMeteor(now);
    if (sky) for (const p of this.particles) this.drawParticle(sky, p, now);
    for (const s of this.splashes) {
      const alpha = 0.8 * this.fade(s.x, this.height);
      if (alpha <= 0.02) continue;
      ctx.fillStyle = `rgba(110, 160, 235, ${alpha})`;
      ctx.fillRect(Math.round(s.x), Math.round(s.y), 1.5, 1.5);
    }
    if (sky === "storm" && now < this.flashUntil) {
      const left = (this.flashUntil - now) / FLASH_MS;
      const cy = this.height - this.bodyHeight * 0.5;
      const glow = ctx.createRadialGradient(this.width / 2, cy, 0, this.width / 2, cy, this.width / 2);
      glow.addColorStop(0, `rgba(255, 255, 235, ${0.55 * left})`);
      glow.addColorStop(1, "rgba(255, 255, 235, 0)");
      ctx.fillStyle = glow;
      ctx.fillRect(0, 0, this.width, this.height);
    }
    if (this.warmth === "hot") this.drawHeat(now);
    if (this.warmth === "cold") this.drawVapor();
  }

  private drawParticle(sky: Sky, p: Particle, now: number): void {
    const { ctx } = this;
    const alpha = this.fade(p.x, p.y, sky !== "cloudy" && sky !== "clear");
    if (alpha <= 0.02) return;
    const x = Math.round(p.x);
    const y = Math.round(p.y);
    // Phần chạm xuống dưới mặt đất thì cắt bỏ.
    const room = this.height - y;
    if (room <= 0) return;
    switch (sky) {
      case "rain":
      case "storm": {
        // Xanh vừa phải để thấy được trên cả hình nền sáng lẫn tối; xiên theo gió, đuôi ở phía trên.
        const length = Math.min(p.size, room);
        const slant = (WIND[sky] ?? 0) * length;
        ctx.strokeStyle = `rgba(110, 160, 235, ${0.85 * alpha})`;
        ctx.lineWidth = 1.5;
        ctx.beginPath();
        ctx.moveTo(p.x - slant, y);
        ctx.lineTo(p.x, y + length);
        ctx.stroke();
        break;
      }
      case "snow":
        ctx.fillStyle = `rgba(255, 255, 255, ${0.9 * alpha})`;
        ctx.fillRect(x, y, Math.round(p.size), Math.min(Math.round(p.size), room));
        break;
      case "petals": {
        // Cánh hoa lật qua lật lại khi rơi: bề ngang co giãn.
        const turn = 0.3 + 0.7 * Math.abs(Math.cos(now / 280 + p.phase * 3));
        ctx.globalAlpha = alpha;
        ctx.fillStyle = PETAL_COLORS[Math.floor(p.phase) % PETAL_COLORS.length];
        ctx.fillRect(x, y, Math.max(1, Math.round(p.size * turn)), Math.min(Math.round(p.size * 0.6), room));
        ctx.globalAlpha = 1;
        break;
      }
      case "fog": {
        // Vệt sương hơi nhấp nhô.
        const bob = Math.round(Math.sin(now / 900 + p.phase) * 1.5);
        ctx.fillStyle = `rgba(200, 210, 228, ${0.8 * alpha})`;
        ctx.fillRect(x, y + bob, Math.round(p.size), Math.min(2, room));
        break;
      }
      case "cloudy": {
        const cloud = this.cloud(Math.floor(p.phase) % CLOUD_SHAPES.length, p.size);
        const { width, height } = this.driftSize(sky, p);
        const bob = Math.round(Math.sin(now / 1500 + p.phase));
        ctx.globalAlpha = 0.95 * alpha;
        ctx.drawImage(cloud, x, y + bob, width, height);
        ctx.globalAlpha = 1;
        break;
      }
      case "sunny":
      case "clear": {
        // Lấp lánh: tia dài ra rồi ngắn lại, sáng lên rồi tắt.
        const glow = Math.sin((p.age / p.life) * Math.PI);
        const arm = Math.round(p.size * glow);
        ctx.fillStyle =
          sky === "sunny" ? `rgba(255, 196, 70, ${alpha * glow})` : `rgba(255, 244, 196, ${alpha * (0.35 + 0.65 * glow)})`;
        ctx.fillRect(x - arm, y, 2 * arm + 1, 1);
        ctx.fillRect(x, y - arm, 1, 2 * arm + 1);
        break;
      }
    }
  }

  /** Nắng: vài tia nắng vàng nhạt xiên từ trên trái xuống rọi vào pet, sáng lên tối đi chậm. */
  private drawRays(now: number): void {
    const { ctx } = this;
    const bodyWidth = this.bodyRight - this.bodyLeft;
    for (let i = 0; i < 3; i++) {
      // Chân tia ở quanh thân pet, đầu tia ở mép trên.
      const x1 = this.width / 2 + (i - 1) * bodyWidth * 0.6;
      const x0 = x1 - this.height * SUN_SLANT;
      const pulse = 0.5 + 0.5 * Math.sin(now / 1800 + i * 2.1);
      const alpha = 0.12 + 0.16 * pulse;
      const ray = ctx.createLinearGradient(x0, 0, x1, this.height);
      ray.addColorStop(0, "rgba(255, 206, 96, 0)");
      ray.addColorStop(0.45, `rgba(255, 206, 96, ${alpha})`);
      ray.addColorStop(1, "rgba(255, 206, 96, 0)");
      ctx.strokeStyle = ray;
      ctx.lineWidth = 3 + i;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(x0, 0);
      ctx.lineTo(x1, this.height);
      ctx.stroke();
    }
  }

  /** Sao băng: một vệt sáng ngắn bay chéo xuống phía trên đầu pet, đuôi mờ dần. */
  private drawMeteor(now: number): void {
    const t = (now - this.meteorAt) / METEOR_MS;
    if (t < 0 || t >= 1) return;
    const { ctx } = this;
    const head = {
      x: this.meteorFrom.x + this.meteorDir * this.width * 0.35 * t,
      y: this.meteorFrom.y + this.headTop * 0.4 * t,
    };
    const tail = 22;
    const tx = head.x - this.meteorDir * tail * 0.87;
    const ty = head.y - tail * 0.5;
    const alpha = Math.sin(t * Math.PI);
    const trail = ctx.createLinearGradient(tx, ty, head.x, head.y);
    trail.addColorStop(0, "rgba(255, 244, 196, 0)");
    trail.addColorStop(1, `rgba(255, 248, 220, ${0.9 * alpha})`);
    ctx.strokeStyle = trail;
    ctx.lineWidth = 1.5;
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.moveTo(tx, ty);
    ctx.lineTo(head.x, head.y);
    ctx.stroke();
  }

  /** Nóng: vệt hơi nóng lượn sóng màu cam bốc lên hai bên thân pet, nhạt dần khi lên cao. */
  private drawHeat(now: number): void {
    const { ctx } = this;
    ctx.lineWidth = 2;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    const rise = this.height - this.headTop;
    for (const p of this.heat) {
      const left = Math.min(1, (p.y - this.headTop) / (rise * 0.5));
      const alpha = 0.9 * left * this.fade(p.x, this.height);
      if (alpha <= 0.02) continue;
      ctx.strokeStyle = `rgba(255, 122, 48, ${alpha})`;
      ctx.beginPath();
      for (let k = 0; k <= p.size; k += 1.5) {
        const y = p.y + k;
        if (y > this.height) break;
        const x = p.x + Math.sin(y / 4 + p.phase + now / 260) * 2.6;
        if (k === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();
    }
  }

  /** Lạnh: hơi nước thở ra, mỗi hạt là một đốm mờ nở to dần rồi tan, chồng lên nhau thành luồng hơi. */
  private drawVapor(): void {
    const { ctx } = this;
    const puff = softPuff();
    for (const v of this.vapor) {
      const t = v.age / v.life;
      const r = v.from + (v.to - v.from) * Math.sqrt(t);
      // Hiện nhanh rồi tan dần.
      const alpha = 0.4 * Math.min(1, t * 6) * (1 - t) ** 1.4;
      if (alpha <= 0.01) continue;
      ctx.globalAlpha = alpha;
      ctx.drawImage(puff, v.x - r, v.y - r, 2 * r, 2 * r);
    }
    ctx.globalAlpha = 1;
  }

  /** Đám mây pixel hình `shape`, mỗi ô `cell` CSS pixel, có viền; vẽ sẵn một lần. */
  private cloud(shape: number, cell: number): HTMLCanvasElement {
    const dpr = window.devicePixelRatio || 1;
    const key = `${shape}:${cell}:${dpr}`;
    const cached = this.clouds.get(key);
    if (cached) return cached;
    const rows = CLOUD_SHAPES[shape];
    const canvas = document.createElement("canvas");
    canvas.width = Math.round((rows[0].length + 2) * cell * dpr);
    canvas.height = Math.round((rows.length + 2) * cell * dpr);
    const ctx = canvas.getContext("2d");
    if (ctx) {
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      const cells: [number, number, string][] = [];
      rows.forEach((row, r) => {
        for (let c = 0; c < row.length; c++) if (CLOUD_COLORS[row[c]]) cells.push([c + 1, r + 1, CLOUD_COLORS[row[c]]]);
      });
      // Viền: tô các ô sát bên mỗi ô có hình, rồi tô thân mây đè lên.
      ctx.fillStyle = CLOUD_EDGE;
      for (const [c, r] of cells) {
        for (const [dc, dr] of [[-1, 0], [1, 0], [0, -1], [0, 1]]) ctx.fillRect((c + dc) * cell, (r + dr) * cell, cell, cell);
      }
      for (const [c, r, color] of cells) {
        ctx.fillStyle = color;
        ctx.fillRect(c * cell, r * cell, cell, cell);
      }
    }
    this.clouds.set(key, canvas);
    return canvas;
  }
}

function between([min, max]: [number, number]): number {
  return min + Math.random() * (max - min);
}
