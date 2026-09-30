import type { Point, Remap } from "@tinyworld/core";
import type { Pet } from "@tinyworld/sim";
import type { PetView } from "./petView";

/** Giữ chuột rồi di quá khoảng này (CSS pixel) thì tính là kéo, không phải click. */
const DRAG_THRESHOLD = 4;
/** Vận tốc ném tính từ chuyển động chuột trong khoảng này trước lúc buông (ms). */
const THROW_WINDOW = 100;

interface Sample {
  x: number;
  y: number;
  t: number;
}

interface Press {
  id: number;
  start: Point;
  /** Khoảng cách từ con trỏ tới điểm chân pet, giữ nguyên trong lúc kéo. */
  offset: Point;
  dragging: boolean;
  samples: Sample[];
}

export interface InteractionHooks {
  /** Bắt đầu / kết thúc giữ chuột trên pet. */
  onHold(held: boolean): void;
  /** Pet vừa được click hoặc kéo, để chạy lại vòng lặp vẽ nếu đang dừng. */
  onActivity(): void;
  /** Đang kéo pet mà con trỏ ra ngoài overlay (CSS pixel của overlay): có thể là sang màn hình khác. */
  onDragOutside?(point: Point): void;
}

/** Click, kéo thả và ném pet bằng chuột trái. */
export class PetInteraction {
  private press: Press | null = null;

  constructor(
    private readonly pet: Pet,
    private readonly view: PetView,
    private readonly hooks: InteractionHooks,
  ) {
    window.addEventListener("pointerdown", this.onDown);
    window.addEventListener("pointermove", this.onMove);
    window.addEventListener("pointerup", this.onUp);
    window.addEventListener("pointercancel", this.onCancel);
    window.addEventListener("lostpointercapture", this.onCancel);
    window.addEventListener("blur", this.cancel);
  }

  /** Ẩn/tạm dừng overlay hoặc mất capture thì thả nhẹ, tránh kẹt ở state bị kéo. */
  readonly cancel = (): void => {
    const press = this.press;
    if (!press) return;
    this.finish(press.id);
    if (press.dragging) this.pet.release(0, 0);
  };

  /**
   * Overlay vừa đổi chỗ (kéo pet sang màn hình khác): đổi các điểm đã ghi sang toạ độ mới, để lúc buông
   * vận tốc ném không bị tính từ toạ độ cũ. Khoảng từ con trỏ tới chân pet giữ nguyên: pet vẫn to bằng
   * chừng ấy CSS pixel.
   */
  remap({ scale, x, y }: Remap): void {
    const press = this.press;
    if (!press) return;
    press.start = { x: press.start.x * scale + x, y: press.start.y * scale + y };
    press.samples = press.samples.map((s) => ({ x: s.x * scale + x, y: s.y * scale + y, t: s.t }));
  }

  private readonly onDown = (event: PointerEvent) => {
    if (event.button !== 0 || this.press) return;
    const point = { x: event.clientX, y: event.clientY };
    if (!this.view.hitTest(point)) return;
    // Vẫn nhận chuột khi con trỏ chạy ra ngoài overlay, ví dụ kéo sang màn hình khác.
    this.view.element.setPointerCapture(event.pointerId);
    // Tính theo chỗ pet đang hiện (trễ `pet.x/y` tới một bước lúc bay), để bắt pet giữa không trung nó không nhảy.
    const foot = this.view.foot ?? this.pet;
    this.press = {
      id: event.pointerId,
      start: point,
      offset: { x: foot.x - point.x, y: foot.y - point.y },
      dragging: false,
      samples: [{ ...point, t: event.timeStamp }],
    };
    this.hooks.onHold(true);
  };

  private readonly onMove = (event: PointerEvent) => {
    const press = this.press;
    if (!press || event.pointerId !== press.id) return;
    const { clientX: x, clientY: y, timeStamp: t } = event;
    press.samples.push({ x, y, t });
    while (press.samples.length > 2 && t - press.samples[0].t > THROW_WINDOW) press.samples.shift();
    if (!press.dragging) {
      if (Math.hypot(x - press.start.x, y - press.start.y) < DRAG_THRESHOLD) return;
      press.dragging = true;
      this.pet.grab();
    }
    this.pet.dragTo(x + press.offset.x, y + press.offset.y);
    this.hooks.onActivity();
    if (x < 0 || y < 0 || x >= window.innerWidth || y >= window.innerHeight) this.hooks.onDragOutside?.({ x, y });
  };

  private readonly onUp = (event: PointerEvent) => {
    const press = this.finish(event.pointerId);
    if (!press) return;
    if (press.dragging) this.pet.release(...throwVelocity(press.samples, event.timeStamp));
    else this.pet.poke();
  };

  private readonly onCancel = (event: PointerEvent) => {
    if (event.pointerId === this.press?.id) this.cancel();
  };

  private finish(pointerId: number): Press | null {
    const press = this.press;
    if (!press || pointerId !== press.id) return null;
    this.press = null;
    if (this.view.element.hasPointerCapture(press.id)) this.view.element.releasePointerCapture(press.id);
    this.hooks.onHold(false);
    this.hooks.onActivity();
    return press;
  }
}

/** Vận tốc chuột (px/s) ngay trước lúc buông; dừng tay rồi mới buông thì coi như thả nhẹ. */
function throwVelocity(samples: Sample[], now: number): [number, number] {
  const recent = samples.filter((s) => now - s.t <= THROW_WINDOW);
  if (recent.length < 2) return [0, 0];
  const first = recent[0];
  const last = recent[recent.length - 1];
  const dt = (last.t - first.t) / 1000;
  if (dt < 0.01) return [0, 0];
  return [(last.x - first.x) / dt, (last.y - first.y) / dt];
}
