/**
 * Chia thời gian thực thành các bước cố định `dt` (giây) để mô phỏng không phụ thuộc fps.
 * Máy bị lag hoặc overlay bị treo lâu thì chỉ chạy bù tối đa `maxSteps` bước, tránh pet "bay" xa.
 */
export class FixedStep {
  private accumulated = 0;

  constructor(
    readonly dt: number,
    private readonly maxSteps = 5,
  ) {}

  /** Cộng thêm thời gian đã trôi (giây), trả về số bước cần chạy. */
  advance(elapsed: number): number {
    this.accumulated += Math.max(0, elapsed);
    // Bù sai số làm tròn để 3 × dt vẫn ra đúng 3 bước.
    const steps = Math.floor(this.accumulated / this.dt + 1e-9);
    if (steps > this.maxSteps) {
      this.accumulated = 0;
      return this.maxSteps;
    }
    this.accumulated = Math.max(0, this.accumulated - steps * this.dt);
    return steps;
  }
}
