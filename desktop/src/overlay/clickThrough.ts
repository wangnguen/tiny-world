import { errorMessage } from "@tinyworld/core";

/** Bật/tắt chế độ chuột đi xuyên overlay, chỉ gọi xuống Rust khi trạng thái thật sự đổi. */
export class ClickThrough {
  /** Rust bật sẵn click-through lúc tạo overlay. */
  private enabled = true;

  constructor(private readonly apply: (enabled: boolean) => Promise<void>) {}

  update(overPet: boolean): void {
    const enabled = !overPet;
    if (enabled === this.enabled) return;
    this.enabled = enabled;
    this.apply(enabled).catch((error) => {
      // Lần cập nhật sau sẽ thử lại.
      this.enabled = !enabled;
      console.error("Không đổi được click-through:", errorMessage(error));
    });
  }
}
