import type { Point } from "@tinyworld/core";

export type LongAction = "transform" | "tail-swipe";
const ACTION_FPS = 7;
const ACTION_MS = 1000 / ACTION_FPS;
const aspect = 512 / 683;
const urls = import.meta.glob<string>("../../../assets/effects/long-actions/*/frame-*.webp", { eager: true, query: "?url", import: "default" });

function frames(action: LongAction): HTMLImageElement[] {
  return Object.entries(urls).filter(([path]) => path.includes(`/${action}/`)).sort(([a], [b]) => a.localeCompare(b)).map(([, url]) => {
    const image = new Image(); image.src = url; return image;
  });
}
const ACTION_FRAMES: Record<LongAction, HTMLImageElement[]> = { transform: frames("transform"), "tail-swipe": frames("tail-swipe") };

/** Sprite cinematic của Long, vẽ thay sprite thường trong lúc biến hình hoặc quật đuôi. */
export class LongActionEffect {
  readonly element = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private action: LongAction | null = null;
  private started = 0;
  private width = 0;
  private height = 0;
  private last = -1;

  constructor() {
    this.element.className = "long-action";
    this.element.hidden = true;
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas cinematic của Long.");
    this.ctx = ctx;
  }

  play(action: LongAction, now: number): void { this.action = action; this.started = now; this.last = -1; this.element.hidden = false; }
  stop(): void { this.action = null; this.element.hidden = true; }
  get playing(): boolean { return this.action !== null; }

  update(foot: Point, petWidth: number, petHeight: number, now: number): void {
    const action = this.action;
    if (!action) return;
    const height = Math.round(Math.max(petHeight * 2, petWidth * 1.7));
    const width = Math.round(height * aspect);
    if (width !== this.width || height !== this.height) this.resize(width, height);
    this.element.style.transform = `translate(${Math.round(foot.x - width / 2)}px, ${Math.round(foot.y - height * 0.86)}px)`;
    const images = ACTION_FRAMES[action];
    const frame = Math.min(images.length - 1, Math.floor((now - this.started) / ACTION_MS));
    if (frame !== this.last) { this.last = frame; this.draw(images[frame]); }
  }

  private resize(width: number, height: number): void {
    this.width = width; this.height = height;
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(width * dpr); this.element.height = Math.round(height * dpr);
    this.element.style.width = `${width}px`; this.element.style.height = `${height}px`; this.last = -1;
  }
  private draw(image: HTMLImageElement | undefined): void {
    const dpr = window.devicePixelRatio || 1;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0); this.ctx.clearRect(0, 0, this.width, this.height);
    if (image?.complete) this.ctx.drawImage(image, 0, 0, this.width, this.height);
  }
}

/** Vệt cam ngắn lúc Long tốc biến; `reverse` lật hướng vệt khi tái xuất. */
export class TeleportStreakEffect {
  readonly element = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private until = 0;
  private started = 0;
  private direction = 1;
  private width = 0;
  private height = 0;

  constructor() {
    this.element.className = "teleport-streak";
    this.element.hidden = true;
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas tốc biến.");
    this.ctx = ctx;
  }

  get playing(): boolean { return performance.now() < this.until; }

  play(point: Point, petWidth: number, petHeight: number, direction: number, reverse: boolean, now: number): void {
    this.direction = (reverse ? -direction : direction) || 1;
    this.width = Math.round(petWidth * 1.8);
    this.height = Math.round(petHeight * 1.15);
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(this.width * dpr);
    this.element.height = Math.round(this.height * dpr);
    this.element.style.width = `${this.width}px`;
    this.element.style.height = `${this.height}px`;
    this.element.style.transform = `translate(${Math.round(point.x - this.width / 2)}px, ${Math.round(point.y - this.height * 0.8)}px)`;
    this.started = now;
    this.until = now + 180;
    this.element.hidden = false;
  }

  update(now: number): void {
    if (now >= this.until) { this.element.hidden = true; return; }
    const dpr = window.devicePixelRatio || 1;
    const t = (now - this.started) / (this.until - this.started);
    const { ctx } = this;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, this.width, this.height);
    ctx.lineCap = "round";
    for (let i = 0; i < 6; i++) {
      const y = this.height * (0.18 + i * 0.12);
      const length = this.width * (0.3 + i * 0.09);
      const head = this.width * 0.5 + this.direction * this.width * (0.14 + t * 0.2);
      ctx.strokeStyle = `rgba(255, ${150 + i * 12}, 35, ${(1 - t) * (0.8 - i * 0.07)})`;
      ctx.lineWidth = Math.max(1, this.height * (0.03 - i * 0.002));
      ctx.beginPath();
      ctx.moveTo(head - this.direction * length, y);
      ctx.lineTo(head, y - this.height * 0.05);
      ctx.stroke();
    }
  }
}
