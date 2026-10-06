import type { Point } from "@tinyworld/core";
import { FRAME_ASPECT, FrameSet } from "./effectFrames";

/** Aura chạy chậm hơn sprite một chút để vẫn sáng rõ nhưng không tốn CPU. */
const AURA_FPS = 10;
const AURA_FRAME_MS = 1000 / AURA_FPS;
const AURA_MIN_MS = 5_000;
const AURA_MAX_MS = 7_000;

const FRAMES = new FrameSet(
  import.meta.glob<string>("../../../assets/effects/long-aura/frame-*.webp", {
    eager: true,
    query: "?url",
    import: "default",
  }),
);

/** Aura riêng của Long; không liên quan tới dữ liệu hoặc trạng thái thời tiết. */
export class AuraEffect {
  readonly element: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private frames = FRAMES.acquire();
  private until = 0;
  private shown = false;
  private width = 0;
  private height = 0;
  private lastFrame = -1;

  constructor() {
    this.element = document.createElement("canvas");
    this.element.className = "aura";
    this.element.hidden = true;
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas aura.");
    this.ctx = ctx;
  }

  get animating(): boolean {
    return performance.now() < this.until;
  }

  /** Vòng lặp phải vẽ lại trong vòng chừng này ms; `Infinity` nếu aura đang tắt. */
  get frameMs(): number {
    return this.animating ? AURA_FRAME_MS : Number.POSITIVE_INFINITY;
  }

  /** Bật aura trong một khoảng ngẫu nhiên 5–7 giây; kích lại thì bắt đầu một lượt mới. */
  activate(now: number): void {
    this.until = now + AURA_MIN_MS + Math.random() * (AURA_MAX_MS - AURA_MIN_MS);
    this.lastFrame = -1;
  }

  /** Long bị bỏ khỏi màn hình: thả ảnh aura. */
  dispose(): void {
    if (this.frames.length === 0) return;
    this.frames = [];
    this.until = 0;
    FRAMES.release();
  }

  update(foot: Point, petWidth: number, petHeight: number, now: number, visible: boolean): void {
    const active = now < this.until;
    const show = active && visible;
    if (show !== this.shown) {
      this.shown = show;
      this.element.hidden = !show;
    }
    if (!show || this.frames.length === 0) return;

    const height = Math.round(petHeight * 1.8);
    // Long có thể dùng frame không vuông sau khi biến hình; giữ aura ít nhất phủ rộng thân.
    const width = Math.max(Math.round(height * FRAME_ASPECT), Math.round(petWidth * 1.35));
    if (width !== this.width || height !== this.height) this.resize(width, height);
    const left = Math.round(foot.x - width / 2);
    const top = Math.round(foot.y - height * 0.86);
    this.element.style.transform = `translate(${left}px, ${top}px)`;

    const frame = Math.floor(now / AURA_FRAME_MS) % this.frames.length;
    if (frame !== this.lastFrame) {
      this.lastFrame = frame;
      this.draw(frame);
    }
  }

  private resize(width: number, height: number): void {
    this.width = width;
    this.height = height;
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(width * dpr);
    this.element.height = Math.round(height * dpr);
    this.element.style.width = `${width}px`;
    this.element.style.height = `${height}px`;
    this.lastFrame = -1;
  }

  private draw(frame: number): void {
    const dpr = window.devicePixelRatio || 1;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.ctx.clearRect(0, 0, this.width, this.height);
    const image = this.frames[frame];
    if (image?.complete) this.ctx.drawImage(image, 0, 0, this.width, this.height);
  }
}
