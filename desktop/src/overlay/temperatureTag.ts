import type { Rect } from "@tinyworld/core";
import type { Bounds, Sky, Warmth } from "@tinyworld/sim";
import { skyIconSvg } from "../skyIcons";

/** Cách pet một chút (CSS pixel). */
const GAP = 4;

/**
 * Nhãn nhiệt độ nhỏ cạnh pet: hình thời tiết và số độ, ví dụ "☀ 27°C"; nóng thì chữ cam, lạnh thì chữ xanh.
 * Đứng ngang đầu, sau lưng pet (khói thở bay về phía trước không bị che; sát mép màn hình thì sang bên kia),
 * chuột đi xuyên.
 */
export class TemperatureTag {
  readonly element: HTMLDivElement;
  private readonly icon: HTMLSpanElement;
  private readonly text: HTMLSpanElement;
  private shown = false;
  private size = { width: 0, height: 0 };
  private key = "";
  private placed = "";

  constructor(container: HTMLElement) {
    this.element = document.createElement("div");
    this.element.className = "temperature";
    this.element.hidden = true;
    this.icon = document.createElement("span");
    this.icon.className = "temperature__icon";
    this.text = document.createElement("span");
    this.element.append(this.icon, this.text);
    container.append(this.element);
  }

  /** Đổi nội dung; `temperature` null là ẩn nhãn. Trả về `true` nếu có gì đổi (phải vẽ lại). */
  set(sky: Sky | null, temperature: number | null, warmth: Warmth | null): boolean {
    const key = temperature === null ? "" : `${sky}:${Math.round(temperature)}:${warmth}`;
    if (key === this.key) return false;
    this.key = key;
    if (temperature === null) return true;
    this.icon.innerHTML = sky ? skyIconSvg(sky) : "";
    this.text.textContent = `${Math.round(temperature)}°C`;
    this.element.dataset.warmth = warmth ?? "";
    this.size = { width: 0, height: 0 };
    return true;
  }

  /**
   * Đặt nhãn cạnh thân pet `body` (`PetView.body`), ngang đầu, sau lưng (pet nhìn về `facing`: 1 sang phải),
   * trong `bounds`; `body` null (không có pet nào đang hiện) hoặc chưa có nhiệt độ thì ẩn.
   */
  place(body: Rect | null, facing: 1 | -1, bounds: Bounds): void {
    const show = body !== null && this.key !== "";
    if (show !== this.shown) {
      this.shown = show;
      this.element.hidden = !show;
      this.placed = "";
    }
    if (!show || !body) return;
    if (this.size.width === 0) this.size = { width: this.element.offsetWidth, height: this.element.offsetHeight };
    const { width, height } = this.size;
    // Sau lưng; không đủ chỗ (sát mép màn hình) thì sang trước mặt.
    const behind = facing > 0 ? body.x - GAP - width : body.x + body.width + GAP;
    const ahead = facing > 0 ? body.x + body.width + GAP : body.x - GAP - width;
    const fits = (x: number) => x >= bounds.left + GAP && x + width <= bounds.right - GAP;
    const x = fits(behind) || !fits(ahead) ? behind : ahead;
    const y = Math.max(bounds.top + GAP, body.y + body.height * 0.2 - height / 2);
    const placed = `${Math.round(x)},${Math.round(y)}`;
    if (placed === this.placed) return;
    this.placed = placed;
    this.element.style.transform = `translate(${Math.round(x)}px, ${Math.round(y)}px)`;
  }
}
