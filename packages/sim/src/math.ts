/** Kẹp `value` vào [min, max]; khoảng rỗng (pet rộng hơn màn hình) thì lấy điểm giữa. */
export function clamp(value: number, min: number, max: number): number {
  if (max < min) return (min + max) / 2;
  return Math.min(Math.max(value, min), max);
}
