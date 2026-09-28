import { frameIndex, type AnimationName, type Point } from "@tinyworld/core";
import type { Bounds, Pet } from "@tinyworld/sim";
import { DizzyStars, dizzyLean, dizzyReach, leanShift, ringRow } from "./dizzy";
import { headOf, type Animation, type Head, type SpriteSet } from "./spriteSet";

/**
 * Một pet trên màn hình: canvas nhỏ bằng một frame (chừa thêm vài cột mỗi bên cho lúc lảo đảo),
 * di chuyển bằng CSS transform. Chỉ vẽ lại canvas khi đổi frame, không vẽ lại cả overlay.
 */
export class PetView {
  readonly element: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;
  private readonly stars: DizzyStars;
  /** Cỡ trong Settings (1 là cỡ gốc của pack). */
  private size = 1;
  /** Số CSS pixel cho một pixel của frame: `scale` của pack nhân với cỡ trong Settings. */
  private scale = 1;
  /** Kích thước một frame khi vẽ (CSS pixel), không tính phần chừa cho lúc lảo đảo. */
  width = 0;
  height = 0;
  /** Số cột pixel (của frame) chừa thêm mỗi bên canvas để phần đầu nghiêng ra lúc lảo đảo không bị cắt. */
  private pad = 0;
  private drawnKey = "";
  private shownState: AnimationName | null = null;
  /** Đầu nhân vật trong animation hiện tại, đo lại khi đổi state. */
  private head: Head = { top: 0, centerX: 0 };
  private animation: Animation | null = null;
  private frame = 0;
  private flip = false;
  /** Góc trên bên trái của frame (không tính phần chừa), CSS pixel của overlay. */
  private left = Number.NaN;
  private top = Number.NaN;
  /** Đỉnh đầu đang lệch bao nhiêu pixel của frame lúc lảo đảo (dương: sang phải màn hình); 0 là đứng thẳng. */
  private lean = 0;

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
    container.append(this.element);
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
    this.pad = dizzyReach(this.sprite.frameHeight);
    this.width = this.sprite.frameWidth * this.scale;
    this.height = this.sprite.frameHeight * this.scale;
    this.element.style.width = `${this.canvasWidth}px`;
    this.element.style.height = `${this.height}px`;
    this.resize();
    // Sao choáng to nhỏ theo pet.
    this.stars.setScale(this.scale);
    this.left = Number.NaN;
  }

  /** Đặt lại độ phân giải canvas theo DPI hiện tại để pixel art không bị nhoè. */
  resize(): void {
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(this.canvasWidth * dpr);
    this.element.height = Math.round(this.height * dpr);
    this.drawnKey = "";
  }

  /** Bề ngang canvas (CSS pixel): một frame cộng phần chừa hai bên. */
  private get canvasWidth(): number {
    return this.width + 2 * this.pad * this.scale;
  }

  update(pet: Pet): void {
    const animation = this.sprite.animations[pet.state];
    // Đi/chạy nhanh hơn (Settings) thì chân cũng bước nhanh hơn, không trượt.
    const moving = pet.state === "walk" || pet.state === "run";
    const fps = moving ? animation.fps * pet.env.speed : animation.fps;
    const frame = frameIndex(animation.frames.length, fps, animation.loop, pet.stateTime);
    const flip = (pet.facing === 1) !== (this.sprite.facing === "right");
    if (pet.state !== this.shownState) this.enterState(pet.state, animation);

    const { anchor, frameWidth } = this.sprite;
    const { scale } = this;
    const anchorX = flip ? frameWidth - anchor.x : anchor.x;
    const left = snap(pet.x - anchorX * scale);
    const top = snap(pet.y - anchor.y * scale);
    // Choáng: lảo đảo qua lại quanh điểm chân, đỉnh đầu lệch `lean` pixel của frame.
    const lean =
      pet.state === "dizzy"
        ? this.fitLean(
            dizzyLean(pet.stateTime, Math.max(1, anchor.y - this.head.top)),
            animation.masks[frame],
            flip,
            left,
            pet.env.bounds,
          )
        : 0;
    const key = `${pet.state}:${frame}:${flip}:${lean}`;
    if (key !== this.drawnKey) {
      this.animation = animation;
      this.frame = frame;
      this.flip = flip;
      this.lean = lean;
      this.draw(animation, frame, flip);
      this.drawnKey = key;
    }
    if (left !== this.left || top !== this.top) {
      this.left = left;
      this.top = top;
      this.element.style.transform = `translate(${snap(left - this.pad * scale)}px, ${top}px)`;
    }
    if (pet.state === "dizzy") this.placeStars(pet.stateTime, flip);
  }

  /** Đo lại đầu nhân vật cho animation mới, tắt sao choáng khi hết choáng. */
  private enterState(state: AnimationName, animation: Animation): void {
    this.shownState = state;
    this.head = headOf(animation.masks[0], this.sprite.frameWidth);
    if (state !== "dizzy") this.stars.hide();
  }

  /** Vòng sao quanh đầu, lệch theo đầu lúc lảo đảo. */
  private placeStars(time: number, flip: boolean): void {
    const { anchor, frameWidth } = this.sprite;
    const { scale } = this;
    const headX = flip ? frameWidth - this.head.centerX : this.head.centerX;
    const row = ringRow(this.head.top, anchor.y - this.head.top);
    this.stars.update(time, this.left + (headX + this.shift(row)) * scale, this.top + row * scale);
  }

  /** Con trỏ (CSS pixel của overlay) có nằm trên phần có hình của pet không. */
  hitTest(point: Point): boolean {
    if (!this.animation) return false;
    const { frameWidth, frameHeight } = this.sprite;
    const { scale } = this;
    const y = Math.floor((point.y - this.top) / scale);
    if (y < 0 || y >= frameHeight) return false;
    // Bỏ độ lệch lúc lảo đảo để về đúng pixel của frame.
    let x = Math.floor((point.x - this.left) / scale) - this.shift(y);
    if (x < 0 || x >= frameWidth) return false;
    if (this.flip) x = frameWidth - 1 - x;
    return this.animation.masks[this.frame][y * frameWidth + x] === 1;
  }

  /** Hàng `y` của frame đang bị đẩy ngang bao nhiêu pixel của frame (lảo đảo). */
  private shift(y: number): number {
    return leanShift(this.lean, y, this.sprite.anchor.y, this.head.top);
  }

  /**
   * Giữ phần bị đẩy ngang lúc lảo đảo trong phần canvas chừa sẵn và trong màn hình: pet đứng sát mép thì
   * nghiêng ít lại chứ không để mất một phần hình.
   */
  private fitLean(lean: number, mask: Uint8Array, flip: boolean, left: number, bounds: Bounds): number {
    if (lean === 0) return 0;
    const { frameWidth } = this.sprite;
    let min = frameWidth;
    let max = -1;
    for (let i = 0; i < mask.length; i++) {
      if (mask[i] === 0) continue;
      min = Math.min(min, i % frameWidth);
      max = Math.max(max, i % frameWidth);
    }
    if (max < 0) return 0;
    // Cột có hình ngoài cùng bên trái / phải, theo chiều trên màn hình.
    const first = flip ? frameWidth - 1 - max : min;
    const last = flip ? frameWidth - 1 - min : max;
    const lowest = Math.max(-this.pad - first, Math.ceil((bounds.left - left) / this.scale) - first);
    const highest = Math.min(
      frameWidth - 1 + this.pad - last,
      Math.floor((bounds.right - left) / this.scale) - last - 1,
    );
    if (lowest > highest) return 0;
    return Math.min(highest, Math.max(lowest, lean));
  }

  private draw(animation: Animation, frame: number, flip: boolean): void {
    const { ctx, element } = this;
    const { frameWidth, frameHeight } = this.sprite;
    const source = animation.frames[frame];
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, element.width, element.height);
    ctx.imageSmoothingEnabled = !this.sprite.pixelArt;
    // Số pixel canvas cho một pixel của frame.
    const kx = element.width / (frameWidth + 2 * this.pad);
    const ky = element.height / frameHeight;
    const width = frameWidth * kx;
    // Vẽ theo dải hàng, lúc lảo đảo mỗi dải lệch một số nguyên pixel của frame: pixel art nghiêng thành
    // bậc gọn. Nghiêng cả canvas bằng CSS thì trình duyệt nội suy, pet to lên là thấy nhoè.
    for (let y = 0; y < frameHeight; ) {
      const shift = this.shift(y);
      let end = y + 1;
      while (end < frameHeight && this.shift(end) === shift) end++;
      const x = (this.pad + shift) * kx;
      if (flip) ctx.setTransform(-1, 0, 0, 1, x + width, 0);
      else ctx.setTransform(1, 0, 0, 1, x, 0);
      ctx.drawImage(
        animation.image,
        source.x,
        source.y + y,
        source.width,
        end - y,
        0,
        y * ky,
        width,
        (end - y) * ky,
      );
      y = end;
    }
  }
}

/** Làm tròn theo pixel thật của màn hình để pixel art không bị nhoè khi di chuyển. */
function snap(value: number): number {
  const dpr = window.devicePixelRatio || 1;
  return Math.round(value * dpr) / dpr;
}
