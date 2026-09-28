import { frameIndex, type AnimationName, type Point } from "@tinyworld/core";
import type { Pet } from "@tinyworld/sim";
import { headOf, type Animation, type Head, type SpriteSet } from "./spriteSet";

/** Biểu tượng hiện trên đầu pet theo state (class `pet-effect--<state>` trong overlay.css). */
const EFFECTS: Partial<Record<AnimationName, string>> = { sleep: "💤", dizzy: "💫" };
/** Hiệu ứng đè xuống đỉnh đầu bao nhiêu (CSS pixel) để trông như dính vào nhân vật. */
const EFFECT_OVERLAP = 3;

/**
 * Một pet trên màn hình: canvas nhỏ đúng bằng một frame, di chuyển bằng CSS transform.
 * Chỉ vẽ lại canvas khi đổi frame, không vẽ lại cả overlay.
 */
export class PetView {
  readonly element: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  /** Khung đặt vị trí hiệu ứng; phần tử con chạy animation để không làm lệch vị trí. */
  private readonly effect: HTMLDivElement;
  private readonly effectSymbol: HTMLSpanElement;
  /** Số CSS pixel cho một pixel của frame: `scale` của pack nhân với cỡ trong Settings. */
  private scale = 1;
  /** Kích thước khi vẽ (CSS pixel). */
  width = 0;
  height = 0;
  private drawnKey = "";
  private effectState: AnimationName | null = null;
  /** Đầu nhân vật trong animation hiện tại và kích thước hiệu ứng, đo lại khi đổi state. */
  private head: Head = { top: 0, centerX: 0 };
  private effectWidth = 0;
  private effectHeight = 0;
  private effectKey = "";
  private animation: Animation | null = null;
  private frame = 0;
  private flip = false;
  private left = Number.NaN;
  private top = Number.NaN;

  /** `size`: cỡ trong Settings (1 là cỡ gốc của pack). */
  constructor(
    private readonly sprite: SpriteSet,
    container: HTMLElement,
    size: number,
  ) {
    this.element = document.createElement("canvas");
    this.element.className = sprite.pixelArt ? "pet pet--pixel" : "pet";
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas cho pet.");
    this.ctx = ctx;
    this.effect = document.createElement("div");
    this.effect.className = "pet-effect";
    this.effect.hidden = true;
    this.effectSymbol = document.createElement("span");
    this.effect.append(this.effectSymbol);
    container.append(this.element, this.effect);
    this.setSize(size);
  }

  /** Đổi cỡ pet; gọi `pet.resize(view.width, view.height)` sau đó để sim biết cỡ mới. */
  setSize(size: number): void {
    this.scale = this.sprite.scale * size;
    this.width = this.sprite.frameWidth * this.scale;
    this.height = this.sprite.frameHeight * this.scale;
    this.element.style.width = `${this.width}px`;
    this.element.style.height = `${this.height}px`;
    this.resize();
    this.left = Number.NaN;
    this.effectKey = "";
  }

  /** Đặt lại độ phân giải canvas theo DPI hiện tại để pixel art không bị nhoè. */
  resize(): void {
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(this.width * dpr);
    this.element.height = Math.round(this.height * dpr);
    this.drawnKey = "";
  }

  update(pet: Pet): void {
    const animation = this.sprite.animations[pet.state];
    // Đi/chạy nhanh hơn (Settings) thì chân cũng bước nhanh hơn, không trượt.
    const moving = pet.state === "walk" || pet.state === "run";
    const fps = moving ? animation.fps * pet.env.speed : animation.fps;
    const frame = frameIndex(animation.frames.length, fps, animation.loop, pet.stateTime);
    const flip = (pet.facing === 1) !== (this.sprite.facing === "right");
    const key = `${pet.state}:${frame}:${flip}`;
    if (key !== this.drawnKey) {
      this.draw(animation, frame, flip);
      this.drawnKey = key;
      this.animation = animation;
      this.frame = frame;
      this.flip = flip;
    }

    const { anchor, frameWidth } = this.sprite;
    const { scale } = this;
    const anchorX = flip ? frameWidth - anchor.x : anchor.x;
    const left = snap(pet.x - anchorX * scale);
    const top = snap(pet.y - anchor.y * scale);
    if (left !== this.left || top !== this.top) {
      this.left = left;
      this.top = top;
      this.element.style.transform = `translate(${left}px, ${top}px)`;
    }
    if (pet.state !== this.effectState) this.showEffect(pet.state, animation);
    if (!this.effect.hidden) this.placeEffect(flip);
  }

  private showEffect(state: AnimationName, animation: Animation): void {
    this.effectState = state;
    const symbol = EFFECTS[state];
    this.effect.hidden = !symbol;
    if (!symbol) return;
    this.effectSymbol.textContent = symbol;
    this.effect.className = `pet-effect pet-effect--${state}`;
    this.head = headOf(animation.masks[0], this.sprite.frameWidth);
    this.effectWidth = this.effect.offsetWidth;
    this.effectHeight = this.effect.offsetHeight;
    this.effectKey = "";
  }

  /** Đặt hiệu ứng ngay trên đỉnh đầu, giữa đầu theo chiều ngang (tính cả khi pet quay mặt). */
  private placeEffect(flip: boolean): void {
    const { frameWidth } = this.sprite;
    const { scale } = this;
    const headX = flip ? frameWidth - this.head.centerX : this.head.centerX;
    const x = Math.round(this.left + headX * scale - this.effectWidth / 2);
    const y = Math.round(this.top + this.head.top * scale - this.effectHeight + EFFECT_OVERLAP);
    const key = `${x},${y}`;
    if (key === this.effectKey) return;
    this.effectKey = key;
    this.effect.style.transform = `translate(${x}px, ${y}px)`;
  }

  /** Con trỏ (CSS pixel của overlay) có nằm trên phần có hình của pet không. */
  hitTest(point: Point): boolean {
    if (!this.animation) return false;
    const { frameWidth, frameHeight } = this.sprite;
    const { scale } = this;
    let x = Math.floor((point.x - this.left) / scale);
    const y = Math.floor((point.y - this.top) / scale);
    if (x < 0 || y < 0 || x >= frameWidth || y >= frameHeight) return false;
    if (this.flip) x = frameWidth - 1 - x;
    return this.animation.masks[this.frame][y * frameWidth + x] === 1;
  }

  private draw(animation: Animation, frame: number, flip: boolean): void {
    const { ctx, element } = this;
    const source = animation.frames[frame];
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, element.width, element.height);
    ctx.imageSmoothingEnabled = !this.sprite.pixelArt;
    if (flip) ctx.setTransform(-1, 0, 0, 1, element.width, 0);
    ctx.drawImage(
      animation.image,
      source.x,
      source.y,
      source.width,
      source.height,
      0,
      0,
      element.width,
      element.height,
    );
  }
}

/** Làm tròn theo pixel thật của màn hình để pixel art không bị nhoè khi di chuyển. */
function snap(value: number): number {
  const dpr = window.devicePixelRatio || 1;
  return Math.round(value * dpr) / dpr;
}
