import type { Rect, WindowInfo } from "@tinyworld/core";

/** Vùng pet được ở, theo CSS pixel của overlay: tường trái, tường phải, trần, mặt đất. */
export interface Bounds {
  left: number;
  right: number;
  top: number;
  floor: number;
}

/** Đoạn [from, to] trên một trục. */
export interface Span {
  from: number;
  to: number;
}

/** Mép trên của một cửa sổ: pet đứng, đi trên đó. */
export interface Ledge {
  /** Cửa sổ có mép này. */
  id: number;
  y: number;
  /** Cả mép, đã cắt theo màn hình. */
  from: number;
  to: number;
  /** Phần không bị cửa sổ nằm trên che, từ trái sang phải. */
  open: Span[];
}

/** Cạnh bên của một cửa sổ: pet leo từ phía ngoài. */
export interface Wall {
  id: number;
  /** -1: cạnh trái, pet bám từ bên trái (mặt quay sang phải); 1: cạnh phải. */
  side: -1 | 1;
  x: number;
  /** Từ mép trên tới mép dưới cửa sổ, đã cắt theo màn hình. */
  from: number;
  to: number;
  /** Phần không bị cửa sổ nằm trên che, từ trên xuống dưới. */
  open: Span[];
}

interface Entry {
  rect: Rect;
  /** Vị trí trong thứ tự chồng, 0 là cửa sổ trên cùng. */
  z: number;
  ledge: Ledge | null;
  walls: [Wall | null, Wall | null];
}

/**
 * Địa hình pet sống: mặt đất (mép trên taskbar) cộng mép trên và cạnh bên của các cửa sổ thật.
 * Pet đứng trên mặt đất hoặc đang bay thì nằm trước mọi cửa sổ; đứng hay leo trên cửa sổ nào thì bị
 * các cửa sổ nằm trên cửa sổ đó che.
 */
export class Terrain {
  readonly ledges: Ledge[] = [];
  private readonly entries = new Map<number, Entry>();
  /** Khung cửa sổ theo thứ tự chồng, trên cùng trước. */
  private readonly stack: Rect[];

  /** `windows` xếp từ trên xuống dưới theo thứ tự chồng. */
  constructor(
    readonly bounds: Bounds,
    readonly windows: readonly WindowInfo[] = [],
  ) {
    this.stack = windows.map((w) => w.rect);
    windows.forEach(({ id, rect }, z) => {
      if (this.entries.has(id)) return;
      const above = this.stack.slice(0, z);
      const ledge = ledgeOf(id, rect, above, bounds);
      if (ledge) this.ledges.push(ledge);
      this.entries.set(id, {
        rect,
        z,
        ledge,
        walls: [wallOf(id, rect, -1, above, bounds), wallOf(id, rect, 1, above, bounds)],
      });
    });
  }

  window(id: number): Rect | undefined {
    return this.entries.get(id)?.rect;
  }

  ledge(id: number): Ledge | undefined {
    return this.entries.get(id)?.ledge ?? undefined;
  }

  wall(id: number, side: -1 | 1): Wall | undefined {
    return this.entries.get(id)?.walls[side < 0 ? 0 : 1] ?? undefined;
  }

  get walls(): Wall[] {
    return [...this.entries.values()].flatMap((entry) => entry.walls.filter((w) => w !== null));
  }

  /** Các cửa sổ nằm trên cửa sổ `id` trong thứ tự chồng: che mất pet đứng hoặc leo trên `id`. */
  above(id: number): Rect[] {
    const z = this.entries.get(id)?.z;
    return z === undefined ? [] : this.stack.slice(0, z);
  }

  /** Điểm (x, y) có bị cửa sổ nằm trên cửa sổ `id` che không. */
  covered(id: number, x: number, y: number): boolean {
    return this.above(id).some((rect) => contains(rect, x, y));
  }
}

/** Đoạn mở chứa `x`, hoặc `undefined` nếu chỗ đó bị che. */
export function openSpanAt(spans: readonly Span[], x: number): Span | undefined {
  return spans.find((span) => x >= span.from && x <= span.to);
}

export function contains(rect: Rect, x: number, y: number): boolean {
  return x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height;
}

/** Bỏ các đoạn `cuts` ra khỏi `spans` (đều xếp tăng dần hoặc không, kết quả xếp tăng dần). */
export function subtract(spans: readonly Span[], cuts: readonly Span[]): Span[] {
  let result = [...spans];
  for (const cut of cuts) {
    if (cut.to <= cut.from) continue;
    result = result.flatMap((span) => {
      if (cut.to <= span.from || cut.from >= span.to) return [span];
      const parts: Span[] = [];
      if (cut.from > span.from) parts.push({ from: span.from, to: cut.from });
      if (cut.to < span.to) parts.push({ from: cut.to, to: span.to });
      return parts;
    });
  }
  return result.sort((a, b) => a.from - b.from);
}

/** Mép trên nằm trong màn hình, dưới trần và trên mặt đất, thì mới là chỗ đứng. */
function ledgeOf(id: number, rect: Rect, above: readonly Rect[], bounds: Bounds): Ledge | null {
  const y = rect.y;
  const from = Math.max(rect.x, bounds.left);
  const to = Math.min(rect.x + rect.width, bounds.right);
  if (to <= from || y <= bounds.top || y >= bounds.floor) return null;
  // Hàng pixel ngay trên mép bị che thì chỗ đó không đứng được.
  const cuts = above
    .filter((r) => r.y <= y - 1 && r.y + r.height > y - 1)
    .map((r) => ({ from: r.x, to: r.x + r.width }));
  return { id, y, from, to, open: subtract([{ from, to }], cuts) };
}

function wallOf(
  id: number,
  rect: Rect,
  side: -1 | 1,
  above: readonly Rect[],
  bounds: Bounds,
): Wall | null {
  const x = side < 0 ? rect.x : rect.x + rect.width;
  const from = Math.max(rect.y, bounds.top);
  const to = Math.min(rect.y + rect.height, bounds.floor);
  if (to <= from || x <= bounds.left || x >= bounds.right) return null;
  // Cột pixel ngay bên ngoài cạnh, phía pet bám.
  const column = side < 0 ? x - 1 : x;
  const cuts = above
    .filter((r) => r.x <= column && r.x + r.width > column)
    .map((r) => ({ from: r.y, to: r.y + r.height }));
  return { id, side, x, from, to, open: subtract([{ from, to }], cuts) };
}
