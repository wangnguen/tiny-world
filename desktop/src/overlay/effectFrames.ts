/**
 * Tỉ lệ ngang/dọc của mọi frame effect của Long (sheet nguồn 4 frame 543×724 xếp ngang, tách bằng
 * `scripts/prepare-aura.mjs`).
 */
export const FRAME_ASPECT = 3 / 4;

/**
 * Bộ ảnh frame của một effect, dùng chung giữa các con: nạp khi có con đầu tiên cần (`acquire`), bỏ khi con
 * cuối cùng thôi dùng (`release`). Không chọn Long thì không tốn RAM cho ảnh của Long.
 */
export class FrameSet {
  private images: HTMLImageElement[] | null = null;
  private users = 0;

  /** `urls`: kết quả `import.meta.glob` (đường dẫn → url); `only`: chỉ lấy đường dẫn có chuỗi này. */
  constructor(
    private readonly urls: Record<string, string>,
    private readonly only = "",
  ) {}

  acquire(): readonly HTMLImageElement[] {
    this.users++;
    this.images ??= Object.entries(this.urls)
      .filter(([path]) => path.includes(this.only))
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([, url]) => {
        const image = new Image();
        image.src = url;
        return image;
      });
    return this.images;
  }

  release(): void {
    this.users = Math.max(0, this.users - 1);
    if (this.users === 0) this.images = null;
  }
}
