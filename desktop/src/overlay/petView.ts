import { frameIndex, type Point } from "@tinyworld/core";
import type { Pet } from "@tinyworld/sim";
import type { Animation, SpriteSet } from "./spriteSet";

/**
 * Một pet trên màn hình: canvas nhỏ đúng bằng một frame, di chuyển bằng CSS transform.
 * Chỉ vẽ lại canvas khi đổi frame, không vẽ lại cả overlay.
 */
export class PetView {
  readonly element: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly width: number;
  private readonly height: number;
  private drawnKey = "";
  private animation: Animation | null = null;
  private frame = 0;
  private flip = false;
  private left = Number.NaN;
  private top = Number.NaN;

  constructor(
    private readonly sprite: SpriteSet,
    container: HTMLElement,
  ) {
    this.width = sprite.frameWidth * sprite.scale;
    this.height = sprite.frameHeight * sprite.scale;
    this.element = document.createElement("canvas");
    this.element.className = sprite.pixelArt ? "pet pet--pixel" : "pet";
    this.element.style.width = `${this.width}px`;
    this.element.style.height = `${this.height}px`;
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas cho pet.");
    this.ctx = ctx;
    container.append(this.element);
    this.resize();
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
    const frame = frameIndex(animation.frames.length, animation.fps, animation.loop, pet.stateTime);
    const flip = (pet.facing === 1) !== (this.sprite.facing === "right");
    const key = `${pet.state}:${frame}:${flip}`;
    if (key !== this.drawnKey) {
      this.draw(animation, frame, flip);
      this.drawnKey = key;
      this.animation = animation;
      this.frame = frame;
      this.flip = flip;
    }

    const { anchor, frameWidth, scale } = this.sprite;
    const anchorX = flip ? frameWidth - anchor.x : anchor.x;
    const left = snap(pet.x - anchorX * scale);
    const top = snap(pet.y - anchor.y * scale);
    if (left !== this.left || top !== this.top) {
      this.left = left;
      this.top = top;
      this.element.style.transform = `translate(${left}px, ${top}px)`;
    }
  }

  /** Con trỏ (CSS pixel của overlay) có nằm trên phần có hình của pet không. */
  hitTest(point: Point): boolean {
    if (!this.animation) return false;
    const { frameWidth, frameHeight, scale } = this.sprite;
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
