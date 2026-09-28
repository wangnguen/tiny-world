import { errorMessage } from "@tinyworld/core";

/** Bật/tắt chế độ chuột đi xuyên overlay, chỉ gọi xuống Rust khi trạng thái thật sự đổi. */
export class ClickThrough {
  /** Rust bật sẵn click-through lúc tạo overlay. */
  private enabled = true;
  private overPet = false;
  private held = false;

  constructor(private readonly apply: (enabled: boolean) => Promise<void>) {}

  /** Con trỏ có đang nằm trên phần có hình của pet không. */
  update(overPet: boolean): void {
    this.overPet = overPet;
    this.sync();
  }

  /** Đang kéo pet: giữ quyền nhận chuột kể cả khi con trỏ chạy ra khỏi pet. */
  hold(held: boolean): void {
    this.held = held;
    this.sync();
  }

  private sync(): void {
    const enabled = !this.overPet && !this.held;
    if (enabled === this.enabled) return;
    this.enabled = enabled;
    this.apply(enabled).catch((error) => {
      // Lần cập nhật sau sẽ thử lại.
      this.enabled = !enabled;
      console.error("Không đổi được click-through:", errorMessage(error));
    });
  }
}
