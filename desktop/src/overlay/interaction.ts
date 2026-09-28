import type { Point } from "@tinyworld/core";
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
  }

  private readonly onDown = (event: PointerEvent) => {
    if (event.button !== 0 || this.press) return;
    const point = { x: event.clientX, y: event.clientY };
    if (!this.view.hitTest(point)) return;
    // Vẫn nhận chuột khi con trỏ chạy ra ngoài overlay, ví dụ kéo sang màn hình khác.
    this.view.element.setPointerCapture(event.pointerId);
    this.press = {
      id: event.pointerId,
      start: point,
      offset: { x: this.pet.x - point.x, y: this.pet.y - point.y },
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
  };

  private readonly onUp = (event: PointerEvent) => {
    const press = this.finish(event);
    if (!press) return;
    if (press.dragging) this.pet.release(...throwVelocity(press.samples, event.timeStamp));
    else this.pet.poke();
  };

  private readonly onCancel = (event: PointerEvent) => {
    const press = this.finish(event);
    if (press?.dragging) this.pet.release(0, 0);
  };

  private finish(event: PointerEvent): Press | null {
    const press = this.press;
    if (!press || event.pointerId !== press.id) return null;
    this.press = null;
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
