/** Một state của máy trạng thái. `C` là đối tượng được điều khiển (ví dụ `Pet`). */
export interface StateDef<S extends string, C> {
  enter?(ctx: C): void;
  /**
   * Gọi mỗi bước mô phỏng. `time`: số giây đã ở state này, tính cả bước hiện tại.
   * Trả về state tiếp theo để chuyển (trả về chính state này thì chạy lại từ đầu),
   * hoặc `undefined` để ở lại.
   */
  update?(ctx: C, time: number, dt: number): S | undefined;
  exit?(ctx: C): void;
}

export type StateTable<S extends string, C> = { readonly [K in S]: StateDef<S, C> };

/** Máy trạng thái hữu hạn. `enter` của state ban đầu không được gọi. */
export class StateMachine<S extends string, C> {
  private current: S;
  private elapsed = 0;

  constructor(
    private readonly table: StateTable<S, C>,
    initial: S,
  ) {
    this.current = initial;
  }

  get state(): S {
    return this.current;
  }

  /** Số giây đã ở state hiện tại. */
  get time(): number {
    return this.elapsed;
  }

  update(ctx: C, dt: number): void {
    this.elapsed += dt;
    const next = this.table[this.current].update?.(ctx, this.elapsed, dt);
    if (next !== undefined) this.go(ctx, next);
  }

  /**
   * Chuyển state ngay. Chuyển sang chính state đang ở thì chạy lại state đó từ đầu. `prepare` chạy sau
   * `exit` của state cũ, trước `enter` của state mới: đặt dữ liệu mà `exit` sẽ xoá còn `enter` cần đọc.
   */
  go(ctx: C, next: S, prepare?: () => void): void {
    this.table[this.current].exit?.(ctx);
    prepare?.();
    this.current = next;
    this.elapsed = 0;
    this.table[next].enter?.(ctx);
  }
}
