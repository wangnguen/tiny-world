import type { Point } from "@tinyworld/core";

/**
 * Vị trí để vẽ giữa hai bước mô phỏng. Mô phỏng chỉ chạy 30 bước/giây: vẽ đúng vị trí bước cuối thì pet bị
 * ném (tới 3000 px/s) nhảy cóc cả trăm pixel, và khựng mỗi khi lượt vẽ lệch nhịp với bước (lượt 0 bước rồi
 * lượt 2 bước). Vẽ trễ một bước, nội suy theo phần bước đã trôi (`FixedStep.alpha`), thì pet đi đều theo giờ thật.
 */
export class StepBlend {
  private from: Point | null = null;
  private to: Point | null = null;

  /** Chạy một bước mô phỏng (`run`), nhớ vị trí của `target` trước và sau bước đó. */
  step(target: Point, run: () => void): void {
    const from = { x: target.x, y: target.y };
    run();
    this.from = from;
    this.to = { x: target.x, y: target.y };
  }

  /**
   * Chỗ vẽ `target` khi đã trôi `alpha` (0–1) của bước kế tiếp. Bị dời ngoài mô phỏng (kéo bằng chuột, sang
   * màn hình khác, cửa sổ đang đứng bị kéo lúc tạm dừng) thì vẽ ngay chỗ mới, không trượt từ chỗ cũ tới.
   */
  at(target: Point, alpha: number): Point {
    const { from, to } = this;
    if (!from || !to || to.x !== target.x || to.y !== target.y) return { x: target.x, y: target.y };
    return { x: from.x + (to.x - from.x) * alpha, y: from.y + (to.y - from.y) * alpha };
  }
}
