import { frameIndex, type AnimationName, type Point } from "@tinyworld/core";
import type { Pet } from "@tinyworld/sim";
import { DizzyStars, dizzyLean, ringRow } from "./dizzy";
import { headOf, type Animation, type Head, type SpriteSet } from "./spriteSet";

/**
 * Biểu tượng hiện trên đầu pet theo state (class `pet-effect--<state>` trong overlay.css).
 * Choáng không dùng biểu tượng mà có sao bay quanh đầu và lảo đảo, xem `dizzy.ts`.
 */
const EFFECTS: Partial<Record<AnimationName, string>> = { sleep: "💤" };
/** Cỡ biểu tượng và độ đè xuống đỉnh đầu (để trông như dính vào nhân vật), tính bằng pixel của frame. */
const EFFECT_SIZE = 7;
const EFFECT_OVERLAP = 1.5;

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
  private readonly stars: DizzyStars;
  /** Cỡ trong Settings (1 là cỡ gốc của pack). */
  private size = 1;
  /** Số CSS pixel cho một pixel của frame: `scale` của pack nhân với cỡ trong Settings. */
  private scale = 1;
  /** Kích thước khi vẽ (CSS pixel). */
  width = 0;
  height = 0;
  private drawnKey = "";
  private shownState: AnimationName | null = null;
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
  /** Độ nghiêng lúc lảo đảo (tan của góc skewX, quanh điểm chân); 0 là đứng thẳng. */
  private skew = 0;
  private originKey = "";

  /** `size`: cỡ trong Settings (1 là cỡ gốc của pack). */
  constructor(
    private sprite: SpriteSet,
    container: HTMLElement,
    size: number,
  ) {
    this.element = document.createElement("canvas");
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas cho pet.");
    this.ctx = ctx;
    this.effect = document.createElement("div");
    this.effect.className = "pet-effect";
    this.effect.hidden = true;
    this.effectSymbol = document.createElement("span");
    this.effect.append(this.effectSymbol);
    container.append(this.element, this.effect);
    this.stars = new DizzyStars(this.element);
    this.setSprite(sprite, size);
  }

  /**
   * Đổi nhân vật (chọn trong Settings) ngay tại chỗ; gọi `pet.resize(view.width, view.height)` sau đó
   * vì pack mới có thể to nhỏ khác.
   */
  setSprite(sprite: SpriteSet, size = this.size): void {
    this.sprite = sprite;
    this.element.className = sprite.pixelArt ? "pet pet--pixel" : "pet";
    this.stars.setFrameWidth(sprite.frameWidth);
    this.animation = null;
    this.shownState = null;
    this.setSize(size);
  }

  /** Đổi cỡ pet; gọi `pet.resize(view.width, view.height)` sau đó để sim biết cỡ mới. */
  setSize(size: number): void {
    this.size = size;
    this.scale = this.sprite.scale * size;
    this.width = this.sprite.frameWidth * this.scale;
    this.height = this.sprite.frameHeight * this.scale;
    this.element.style.width = `${this.width}px`;
    this.element.style.height = `${this.height}px`;
    this.resize();
    // Biểu tượng và sao to nhỏ theo pet.
    this.effect.style.fontSize = `${EFFECT_SIZE * this.scale}px`;
    this.measureEffect();
    this.stars.setScale(this.scale);
    this.left = Number.NaN;
    this.originKey = "";
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

    if (pet.state !== this.shownState) this.enterState(pet.state, animation);
    const { anchor, frameWidth } = this.sprite;
    const { scale } = this;
    const anchorX = flip ? frameWidth - anchor.x : anchor.x;
    const left = snap(pet.x - anchorX * scale);
    const top = snap(pet.y - anchor.y * scale);
    // Choáng: nghiêng qua lại quanh điểm chân, đỉnh đầu lệch `lean` pixel của frame.
    const height = Math.max(1, anchor.y - this.head.top);
    const lean = pet.state === "dizzy" ? dizzyLean(pet.stateTime, height) : 0;
    const skew = -lean / height;
    if (left !== this.left || top !== this.top || skew !== this.skew) {
      this.left = left;
      this.top = top;
      this.skew = skew;
      const origin = `${anchorX * scale}px ${anchor.y * scale}px`;
      if (origin !== this.originKey) {
        this.originKey = origin;
        this.element.style.transformOrigin = origin;
      }
      const tilt = skew === 0 ? "" : ` skewX(${Math.atan(skew)}rad)`;
      this.element.style.transform = `translate(${left}px, ${top}px)${tilt}`;
    }
    if (!this.effect.hidden) this.placeEffect(flip);
    if (pet.state === "dizzy") this.placeStars(pet.stateTime, flip);
  }

  /** Đo lại đầu nhân vật cho animation mới, bật/tắt biểu tượng và sao choáng. */
  private enterState(state: AnimationName, animation: Animation): void {
    this.shownState = state;
    this.head = headOf(animation.masks[0], this.sprite.frameWidth);
    if (state !== "dizzy") this.stars.hide();
    const symbol = EFFECTS[state];
    this.effect.hidden = !symbol;
    if (!symbol) return;
    this.effectSymbol.textContent = symbol;
    this.effect.className = `pet-effect pet-effect--${state}`;
    this.measureEffect();
  }

  private measureEffect(): void {
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
    const overlap = EFFECT_OVERLAP * scale;
    const y = Math.round(this.top + this.head.top * scale - this.effectHeight + overlap);
    const key = `${x},${y}`;
    if (key === this.effectKey) return;
    this.effectKey = key;
    this.effect.style.transform = `translate(${x}px, ${y}px)`;
  }

  /** Vòng sao quanh đầu, lệch theo đầu lúc lảo đảo. */
  private placeStars(time: number, flip: boolean): void {
    const { anchor, frameWidth } = this.sprite;
    const { scale } = this;
    const headX = flip ? frameWidth - this.head.centerX : this.head.centerX;
    const row = ringRow(this.head.top, anchor.y - this.head.top);
    const tilt = this.skew * (row - anchor.y) * scale;
    this.stars.update(time, this.left + headX * scale + tilt, this.top + row * scale);
  }

  /** Con trỏ (CSS pixel của overlay) có nằm trên phần có hình của pet không. */
  hitTest(point: Point): boolean {
    if (!this.animation) return false;
    const { anchor, frameWidth, frameHeight } = this.sprite;
    const { scale } = this;
    const localY = point.y - this.top;
    // Bỏ độ nghiêng lúc lảo đảo để về đúng pixel của frame.
    const localX = point.x - this.left - this.skew * (localY - anchor.y * scale);
    let x = Math.floor(localX / scale);
    const y = Math.floor(localY / scale);
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
