import type { Point } from "@tinyworld/core";
import type { Facing, Pet } from "@tinyworld/sim";
import { AuraEffect } from "./aura";
import { FRAME_ASPECT, FrameSet } from "./effectFrames";
import type { PetView } from "./petView";

export type LongAction = "transform" | "tail-swipe";
const ACTION_FPS = 7;
const ACTION_MS = 1000 / ACTION_FPS;

const urls = import.meta.glob<string>("../../../assets/effects/long-actions/*/frame-*.webp", {
  eager: true,
  query: "?url",
  import: "default",
});
const ACTION_FRAMES: Record<LongAction, FrameSet> = {
  transform: new FrameSet(urls, "/transform/"),
  "tail-swipe": new FrameSet(urls, "/tail-swipe/"),
};

/** Sprite cinematic của Long, vẽ thay sprite thường trong lúc biến hình hoặc quật đuôi. */
export class LongActionEffect {
  readonly element = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private frames: Record<LongAction, readonly HTMLImageElement[]> | null = {
    transform: ACTION_FRAMES.transform.acquire(),
    "tail-swipe": ACTION_FRAMES["tail-swipe"].acquire(),
  };
  private action: LongAction | null = null;
  /** Hướng đã chốt lúc `play`: frame vẽ quay sang phải, -1 thì lật. */
  private facing: Facing = 1;
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

  /**
   * Diễn `action` quay về phía `facing`. Hướng chốt luôn lúc này: trong lúc diễn sim có thể quay đầu Long
   * (đứng yên thì nhìn theo con trỏ), đòn vẫn phải đánh về phía mục tiêu.
   */
  play(action: LongAction, now: number, facing: Facing): void {
    this.action = action;
    this.facing = facing;
    this.started = now;
    this.last = -1;
    this.element.hidden = false;
  }

  stop(): void {
    this.action = null;
    this.element.hidden = true;
  }

  get playing(): boolean {
    return this.action !== null;
  }

  update(foot: Point, petWidth: number, petHeight: number, now: number): void {
    const { action, frames } = this;
    if (!action || !frames) return;
    const height = Math.round(Math.max(petHeight * 2, petWidth * 1.7));
    const width = Math.round(height * FRAME_ASPECT);
    if (width !== this.width || height !== this.height) this.resize(width, height);
    const left = Math.round(foot.x - width / 2);
    const top = Math.round(foot.y - height * 0.86);
    this.element.style.transform = `translate(${left}px, ${top}px)`;
    const images = frames[action];
    const frame = Math.min(images.length - 1, Math.floor((now - this.started) / ACTION_MS));
    if (frame !== this.last) {
      this.last = frame;
      this.draw(images[frame]);
    }
  }

  /** Long bị bỏ khỏi màn hình: thả ảnh cinematic. */
  dispose(): void {
    if (!this.frames) return;
    this.frames = null;
    this.stop();
    ACTION_FRAMES.transform.release();
    ACTION_FRAMES["tail-swipe"].release();
  }

  private resize(width: number, height: number): void {
    this.width = width;
    this.height = height;
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(width * dpr);
    this.element.height = Math.round(height * dpr);
    this.element.style.width = `${width}px`;
    this.element.style.height = `${height}px`;
    this.last = -1;
  }

  private draw(image: HTMLImageElement | undefined): void {
    const dpr = window.devicePixelRatio || 1;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.ctx.clearRect(0, 0, this.width, this.height);
    if (!image?.complete) return;
    if (this.facing < 0) this.ctx.setTransform(-dpr, 0, 0, dpr, this.width * dpr, 0);
    this.ctx.drawImage(image, 0, 0, this.width, this.height);
  }
}

/** Vệt cam ngắn lúc Long tốc biến; `reverse` lật hướng vệt khi tái xuất. */
export class TeleportStreakEffect {
  readonly element = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private until = 0;
  private started = 0;
  private direction: Facing = 1;
  private width = 0;
  private height = 0;

  constructor() {
    this.element.className = "teleport-streak";
    this.element.hidden = true;
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas tốc biến.");
    this.ctx = ctx;
  }

  /** Vệt chạy hết trong `duration` ms, đầu vệt tại điểm chân `point`. */
  play(
    point: Point,
    petWidth: number,
    petHeight: number,
    direction: Facing,
    reverse: boolean,
    now: number,
    duration: number,
  ): void {
    this.direction = reverse ? (direction === 1 ? -1 : 1) : direction;
    this.width = Math.round(petWidth * 1.8);
    this.height = Math.round(petHeight * 1.15);
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(this.width * dpr);
    this.element.height = Math.round(this.height * dpr);
    this.element.style.width = `${this.width}px`;
    this.element.style.height = `${this.height}px`;
    const left = Math.round(point.x - this.width / 2);
    const top = Math.round(point.y - this.height * 0.8);
    this.element.style.transform = `translate(${left}px, ${top}px)`;
    this.started = now;
    this.until = now + duration;
    this.element.hidden = false;
  }

  update(now: number): void {
    if (now >= this.until) {
      if (!this.element.hidden) this.element.hidden = true;
      return;
    }
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

/**
 * Đồ diễn riêng của Long: aura sau lưng, sprite cinematic (biến hình, quật đuôi), vệt tốc biến. Chỉ tạo
 * cho Long; ảnh effect nạp lúc tạo và thả lúc `dispose`.
 */
export class LongKit {
  readonly aura = new AuraEffect();
  readonly action = new LongActionEffect();
  readonly teleport = new TeleportStreakEffect();

  constructor(view: PetView) {
    view.attach(this.aura.element, true);
    view.attach(this.action.element);
    view.attach(this.teleport.element);
  }

  /** Mỗi lần vẽ: cinematic đi theo chân Long, vệt tốc biến chạy tiếp. Aura do `Ambience` đặt cùng thời tiết. */
  update(view: PetView, pet: Pet, now: number): void {
    this.action.update(view.foot ?? pet, view.width, view.height, now);
    this.teleport.update(now);
  }

  dispose(): void {
    this.aura.dispose();
    this.action.dispose();
  }
}
