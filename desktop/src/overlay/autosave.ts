import { errorMessage } from "@tinyworld/core";
import type { World, WorldSnapshot } from "@tinyworld/sim";

/**
 * Lưu thế giới pet vào world.json định kỳ và khi thoát app. Chỉ ghi khi có gì đổi, nên pet đang
 * ngủ thì không ghi đĩa. Tắt máy ngang thì mất tối đa một chu kỳ.
 */
export class AutoSave {
  private saved = "";

  constructor(
    private readonly world: World,
    private readonly save: (snapshot: WorldSnapshot) => Promise<void>,
  ) {}

  /** Lấy trạng thái vừa nạp làm mốc, để lần lưu đầu không ghi lại y hệt. */
  markSaved(): void {
    this.saved = JSON.stringify(this.world.snapshot());
  }

  start(intervalMs: number): void {
    setInterval(() => void this.flush(), intervalMs);
  }

  async flush(): Promise<void> {
    const snapshot = this.world.snapshot();
    const json = JSON.stringify(snapshot);
    if (json === this.saved) return;
    try {
      await this.save(snapshot);
      this.saved = json;
    } catch (error) {
      // Lần sau thử lại.
      console.error("Không lưu được trạng thái pet:", errorMessage(error));
    }
  }
}
