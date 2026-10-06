import { frameIndex, type AnimationName, type Point, type Rect } from "@tinyworld/core";
import { contains, type Bounds, type Pet } from "@tinyworld/sim";
import { DizzyStars, dizzyLean, dizzyReach, leanShift, ringRow } from "./dizzy";
import { headOf, type Animation, type Head, type Mask, type SpriteSet } from "./spriteSet";

/** Đáy speech bubble (cả đuôi) cách đỉnh đầu pet chừng này CSS pixel. */
const BUBBLE_LIFT = 12;

/**
 * Một pet trên màn hình: canvas nhỏ bằng một frame (chừa thêm vài cột mỗi bên cho lúc lảo đảo),
 * di chuyển bằng CSS transform. Chỉ vẽ lại canvas khi đổi frame, không vẽ lại cả overlay.
 */
export class PetView {
  readonly element: HTMLCanvasElement;
  /** Lớp chứa canvas và sao choáng của pet này, để đưa cả con lên trên các con khác (`raise`). */
  private readonly layer: HTMLDivElement;
  private static topDepth = 0;
  /** Con có `depth` lớn hơn nằm trên: tạo sau hoặc vừa được đưa lên (`raise`). */
  depth = ++PetView.topDepth;
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
  /** Số pixel canvas (pixel màn hình) cho một pixel của frame. */
  private density = 1;
  /** Frame đứng thẳng vẽ sẵn ở đây lúc lảo đảo, rồi chép từng dải hàng sang canvas chính. */
  private readonly upright = document.createElement("canvas");
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
  /** Điểm chân lúc vẽ lần cuối (CSS pixel của overlay), có thể trễ hơn `pet.x/y` một bước mô phỏng. */
  foot: Point | null = null;
  /** Đỉnh đầu đang lệch bao nhiêu pixel của frame lúc lảo đảo (dương: sang phải màn hình); 0 là đứng thẳng. */
  private lean = 0;
  /** Cửa sổ đang che pet (CSS pixel của overlay): không vẽ, không bắt chuột ở đó. */
  private occluders: readonly Rect[] = [];
  /** Khoảng từ điểm chân tới tường lúc leo (CSS pixel), đo theo tay nhân vật trong animation `climb`. */
  reach = 0;
  /** Speech bubble đang hiện (`pet.speech`), câu đang hiện và cỡ của nó (đo một lần khi đổi câu). */
  private bubble: HTMLDivElement | null = null;
  private bubbleText = "";
  private bubbleSize = { width: 0, height: 0 };
  /** Có cinematic đang vẽ thay sprite (`setCovered`). */
  private covered = false;
  /** Pet đang biến mất (`Pet.vanished`), theo lần `update` gần nhất. */
  private gone = false;

  /** `size`: cỡ trong Settings (1 là cỡ gốc của pack). */
  constructor(
    private sprite: SpriteSet,
    private readonly container: HTMLElement,
    size: number,
  ) {
    this.layer = document.createElement("div");
    this.layer.className = "pet-layer";
    this.element = document.createElement("canvas");
    const ctx = this.element.getContext("2d");
    if (!ctx) throw new Error("Không tạo được canvas cho pet.");
    this.ctx = ctx;
    this.layer.append(this.element);
    container.append(this.layer);
    this.stars = new DizzyStars(this.element);
    this.setSprite(sprite, size);
  }

  /** Vẽ pet này lên trên các con khác (vừa bị bấm, bị kéo). */
  raise(): void {
    if (this.container.lastElementChild === this.layer) return;
    this.container.append(this.layer);
    this.depth = ++PetView.topDepth;
  }

  /** Bỏ pet khỏi màn hình (bớt nhân vật trong Settings). */
  destroy(): void {
    this.layer.remove();
  }

  /** Một cinematic vẽ thay sprite (Long biến hình, quật đuôi): ẩn riêng sprite, các lớp effect vẫn vẽ. */
  setCovered(covered: boolean): void {
    this.covered = covered;
    this.syncHidden();
  }

  /** Sprite đang ẩn (bị cinematic thay, hoặc pet đang biến mất): không bắt chuột, không hiện bubble. */
  get spriteHidden(): boolean {
    return this.element.hidden;
  }

  private syncHidden(): void {
    const hidden = this.covered || this.gone;
    if (this.element.hidden !== hidden) this.element.hidden = hidden;
  }

  /** Thêm một lớp vẽ đi cùng pet (hiệu ứng thời tiết/aura), chung lớp với pet. */
  attach(element: HTMLElement, behind = false): void {
    if (behind) this.layer.insertBefore(element, this.element);
    else this.layer.append(element);
  }

  /**
   * Đổi nhân vật (chọn trong Settings) ngay tại chỗ; gọi `pet.resize(view.width, view.height)` sau đó
   * vì pack mới có thể to nhỏ khác.
   */
  setSprite(sprite: SpriteSet, size = this.size): void {
    this.sprite = sprite;
    this.element.className = sprite.pixelArt ? "pet pet--pixel" : "pet";
    // Sao dùng lưới 48px riêng, không nhỏ đi khi pack tăng độ phân giải ảnh.
    this.stars.setFrameWidth(Math.min(48, sprite.frameWidth));
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
    this.reach = climbReach(this.sprite) * this.scale;
    this.element.style.width = `${this.canvasWidth}px`;
    this.element.style.height = `${this.height}px`;
    this.resize();
    // Sao choáng to nhỏ theo pet.
    this.stars.setScale(this.scale * Math.max(1, this.sprite.frameWidth / 48));
    this.left = Number.NaN;
  }

  /** Đặt lại độ phân giải canvas theo DPI hiện tại để pixel art không bị nhoè. */
  resize(): void {
    const dpr = window.devicePixelRatio || 1;
    this.element.width = Math.round(this.canvasWidth * dpr);
    this.element.height = Math.round(this.height * dpr);
    this.upright.width = this.element.width;
    this.upright.height = this.element.height;
    this.density = this.element.width / (this.sprite.frameWidth + 2 * this.pad);
    this.drawnKey = "";
  }

  /** Bề ngang canvas (CSS pixel): một frame cộng phần chừa hai bên. */
  private get canvasWidth(): number {
    return this.width + 2 * this.pad * this.scale;
  }

  /** Khung canvas đang vẽ (CSS pixel của overlay); `null` khi chưa vẽ lần nào. */
  get bounds(): Rect | null {
    if (Number.isNaN(this.left)) return null;
    return { x: this.left - this.pad * this.scale, y: this.top, width: this.canvasWidth, height: this.height };
  }

  /** Còn bao nhiêu giây nữa thì animation của `pet` đổi sang frame khác; `Infinity` nếu không đổi nữa. */
  nextFrameIn(pet: Pet): number {
    const animation = this.sprite.animations[pet.state];
    const count = animation.frames.length;
    if (pet.pose !== undefined || count < 2) return Number.POSITIVE_INFINITY;
    const fps = animationFps(pet, animation);
    const time = Math.max(0, pet.stateTime);
    const index = Math.floor(time * fps);
    if (!animation.loop && index >= count - 1) return Number.POSITIVE_INFINITY;
    return (index + 1) / fps - time;
  }

  /**
   * `occluders`: cửa sổ đang che pet (`World.occluders`), phần bị che không vẽ. `at`: điểm chân để vẽ, mặc định
   * đúng chỗ của pet; vòng lặp truyền vị trí nội suy giữa hai bước mô phỏng (`StepBlend`).
   */
  update(pet: Pet, occluders: readonly Rect[] = [], at: Point = pet): void {
    this.gone = pet.vanished > 0;
    this.syncHidden();
    const animation = this.sprite.animations[pet.state];
    const fps = animationFps(pet, animation);
    const count = animation.frames.length;
    const frame =
      pet.pose === undefined
        ? frameIndex(count, fps, animation.loop, pet.stateTime)
        : Math.min(pet.pose, count - 1);
    const flip = (pet.facing === 1) !== (this.sprite.facing === "right");
    if (pet.state !== this.shownState) this.enterState(pet.state, animation);

    const { anchor, frameWidth } = this.sprite;
    const { scale } = this;
    const anchorX = flip ? frameWidth - anchor.x : anchor.x;
    // Làm tròn gốc canvas một lần; vùng bắt chuột dùng đúng gốc frame đã vẽ.
    // Làm tròn riêng frame rồi canvas sẽ lệch tới nửa pixel ở DPI 125%.
    const canvasLeft = snap(at.x - (anchorX + this.pad) * scale);
    const left = canvasLeft + this.pad * scale;
    const top = snap(at.y - anchor.y * scale);
    this.foot = { x: at.x, y: at.y };
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
    // Chỉ giữ cửa sổ chạm vào canvas, toạ độ tính từ góc canvas để pet đi mà cửa sổ đứng yên cũng vẽ lại.
    const box = { x: canvasLeft, y: top, width: this.canvasWidth, height: this.height };
    this.occluders = occluders.filter((r) => overlaps(r, box));
    const cuts = this.occluders.map((r) => ({ ...r, x: r.x - canvasLeft, y: r.y - top }));
    const key = `${pet.state}:${frame}:${flip}:${lean}:${cuts.map(rectKey).join(";")}`;
    if (key !== this.drawnKey) {
      this.animation = animation;
      this.frame = frame;
      this.flip = flip;
      this.lean = lean;
      this.draw(animation, frame, flip);
      this.cut(cuts);
      this.drawnKey = key;
    }
    if (left !== this.left || top !== this.top) {
      this.left = left;
      this.top = top;
      this.element.style.transform = `translate(${canvasLeft}px, ${top}px)`;
    }
    if (pet.state === "dizzy") this.placeStars(pet.stateTime, flip);
    this.placeBubble(pet, flip);
  }

  /** Speech bubble ngay trên đầu, luôn nằm trong màn hình; đuôi bubble chỉ vào giữa đầu. */
  private placeBubble(pet: Pet, flip: boolean): void {
    // Sprite đang ẩn thì cất bubble; câu chưa hết giờ thì hiện lại cùng pet.
    const text = this.element.hidden ? "" : (pet.speech?.text ?? "");
    if (text !== this.bubbleText) {
      this.bubbleText = text;
      if (!text) {
        this.bubble?.remove();
        this.bubble = null;
        return;
      }
      if (!this.bubble) {
        this.bubble = document.createElement("div");
        this.bubble.className = "bubble";
        this.layer.append(this.bubble);
      }
      this.bubble.textContent = text;
      this.bubbleSize = { width: this.bubble.offsetWidth, height: this.bubble.offsetHeight };
    }
    const bubble = this.bubble;
    if (!bubble) return;
    const { frameWidth } = this.sprite;
    const { left, right, top } = pet.env.bounds;
    const { width, height } = this.bubbleSize;
    const headX = this.left + (flip ? frameWidth - this.head.centerX : this.head.centerX) * this.scale;
    const x = Math.round(Math.min(Math.max(headX - width / 2, left + 4), right - width - 4));
    const y = Math.round(Math.max(this.top + this.head.top * this.scale - BUBBLE_LIFT - height, top + 4));
    bubble.style.transform = `translate(${x}px, ${y}px)`;
    bubble.style.setProperty("--tail", `${Math.min(Math.max(headX - x, 12), width - 12)}px`);
  }

  /** Đo lại đầu nhân vật cho animation mới, tắt sao choáng khi hết choáng. */
  private enterState(state: AnimationName, animation: Animation): void {
    this.shownState = state;
    this.head = headOf(animation.masks[0]);
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
    // Chỗ bị cửa sổ khác che thì click thuộc về cửa sổ đó.
    if (this.occluders.some((r) => contains(r, point.x, point.y))) return false;
    const { frameWidth, frameHeight } = this.sprite;
    const { scale } = this;
    const y = Math.floor((point.y - this.top) / scale);
    if (y < 0 || y >= frameHeight) return false;
    // Bỏ độ lệch lúc lảo đảo để về đúng pixel của frame.
    let x = Math.floor((point.x - this.left) / scale - this.shift((point.y - this.top) / scale));
    if (x < 0 || x >= frameWidth) return false;
    if (this.flip) x = frameWidth - 1 - x;
    return this.animation.masks[this.frame].has(x, y);
  }

  /** Hàng `y` của frame (có thể lẻ) đang bị đẩy ngang bao nhiêu pixel của frame (lảo đảo). */
  private shift(y: number): number {
    return leanShift(this.lean, y, this.sprite.anchor.y, this.head.top, this.density);
  }

  /** Hàng `row` của canvas đang bị đẩy ngang bao nhiêu pixel canvas (lảo đảo), luôn là số nguyên. */
  private rowShift(row: number): number {
    const ky = this.element.height / this.sprite.frameHeight;
    return Math.round(this.shift((row + 0.5) / ky) * this.density);
  }

  /**
   * Giữ phần bị đẩy ngang lúc lảo đảo trong phần canvas chừa sẵn và trong màn hình: pet đứng sát mép thì
   * nghiêng ít lại chứ không để mất một phần hình.
   */
  private fitLean(lean: number, mask: Mask, flip: boolean, left: number, bounds: Bounds): number {
    if (lean === 0) return 0;
    const { frameWidth } = this.sprite;
    const { left: min, right: max } = mask;
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

  /** Xoá phần canvas bị cửa sổ che; `cuts` tính theo CSS pixel từ góc trên trái canvas. */
  private cut(cuts: readonly Rect[]): void {
    if (cuts.length === 0) return;
    const { ctx, element } = this;
    const kx = element.width / this.canvasWidth;
    const ky = element.height / this.height;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    for (const r of cuts) ctx.clearRect(r.x * kx, r.y * ky, r.width * kx, r.height * ky);
  }

  private draw(animation: Animation, frame: number, flip: boolean): void {
    const { ctx, element } = this;
    const source = animation.frames[frame];
    // Lảo đảo: vẽ frame đứng thẳng (thu nhỏ y như lúc không choáng) lên canvas phụ, rồi chép từng dải
    // hàng sang, mỗi dải dời một số nguyên pixel canvas: pixel art nghiêng thành bậc gọn mà vân không đổi.
    // Nghiêng cả canvas bằng CSS thì trình duyệt nội suy, pet to lên là thấy nhoè.
    const target = this.lean === 0 ? ctx : context(this.upright);
    target.setTransform(1, 0, 0, 1, 0, 0);
    target.clearRect(0, 0, element.width, element.height);
    target.imageSmoothingEnabled = !this.sprite.pixelArt;
    const x = this.pad * this.density;
    const width = this.sprite.frameWidth * this.density;
    if (flip) target.setTransform(-1, 0, 0, 1, x + width, 0);
    else target.setTransform(1, 0, 0, 1, x, 0);
    target.drawImage(animation.image, source.x, source.y, source.width, source.height, 0, 0, width, element.height);
    if (target === ctx) return;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, element.width, element.height);
    for (let row = 0; row < element.height; ) {
      const shift = this.rowShift(row);
      let end = row + 1;
      while (end < element.height && this.rowShift(end) === shift) end++;
      ctx.drawImage(this.upright, 0, row, element.width, end - row, shift, row, element.width, end - row);
      row = end;
    }
  }
}

/** Đi/chạy/leo nhanh hơn (Settings) hay chậm hơn (ban đêm) thì chân tay cũng theo, không trượt. */
function animationFps(pet: Pet, animation: Animation): number {
  const moving = pet.state === "walk" || pet.state === "run" || pet.state === "climb";
  return moving ? animation.fps * pet.pace : animation.fps;
}

/** Khoảng từ điểm chân tới mép tay xa nhất trong animation `climb` (pixel của frame): chỗ tay chạm tường. */
function climbReach(sprite: SpriteSet): number {
  const { frameWidth, anchor } = sprite;
  let reach = 0;
  for (const mask of sprite.animations.climb.masks) {
    if (mask.right < 0) continue;
    reach = Math.max(reach, sprite.facing === "right" ? mask.right + 1 - anchor.x : anchor.x - mask.left);
  }
  return Math.max(reach, frameWidth * 0.1);
}

function context(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Không tạo được canvas cho pet.");
  return ctx;
}

function overlaps(a: Rect, b: Rect): boolean {
  return a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
}

function rectKey(r: Rect): string {
  return [r.x, r.y, r.width, r.height].map((v) => Math.round(v * 4) / 4).join(",");
}

/** Làm tròn theo pixel thật của màn hình để pixel art không bị nhoè khi di chuyển. */
function snap(value: number): number {
  const dpr = window.devicePixelRatio || 1;
  return Math.round(value * dpr) / dpr;
}
