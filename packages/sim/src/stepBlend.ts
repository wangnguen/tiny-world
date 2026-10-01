import type { Point } from "@tinyworld/core";

interface Move {
  from: Point;
  to: Point;
}

/**
 * Vị trí để vẽ giữa hai bước mô phỏng. Mô phỏng chỉ chạy 30 bước/giây: vẽ đúng vị trí bước cuối thì pet bị
 * ném (tới 3000 px/s) nhảy cóc cả trăm pixel, và khựng mỗi khi lượt vẽ lệch nhịp với bước (lượt 0 bước rồi
 * lượt 2 bước). Vẽ trễ một bước, nội suy theo phần bước đã trôi (`FixedStep.alpha`), thì pet đi đều theo giờ thật.
 */
export class StepBlend {
  private moves = new Map<Point, Move>();

  /** Chạy một bước mô phỏng (`run`), nhớ vị trí của từng `targets` (mọi pet) trước và sau bước đó. */
  step(targets: readonly Point[], run: () => void): void {
    const from = targets.map((t) => ({ x: t.x, y: t.y }));
    run();
    // Tạo lại mỗi bước: pet bị bỏ (bớt nhân vật trong Settings) không còn bị giữ lại ở đây.
    this.moves = new Map(targets.map((t, i) => [t, { from: from[i], to: { x: t.x, y: t.y } }]));
  }

  /**
   * Chỗ vẽ `target` khi đã trôi `alpha` (0–1) của bước kế tiếp. Bị dời ngoài mô phỏng (kéo bằng chuột, sang
   * màn hình khác, cửa sổ đang đứng bị kéo lúc tạm dừng) thì vẽ ngay chỗ mới, không trượt từ chỗ cũ tới.
   */
  at(target: Point, alpha: number): Point {
    const move = this.moves.get(target);
    if (!move || move.to.x !== target.x || move.to.y !== target.y) return { x: target.x, y: target.y };
    const { from, to } = move;
    return { x: from.x + (to.x - from.x) * alpha, y: from.y + (to.y - from.y) * alpha };
  }
}
