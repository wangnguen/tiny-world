export interface Point {
  x: number;
  y: number;
}

/** Hình chữ nhật theo CSS pixel, gốc toạ độ là góc trên trái của overlay. */
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Màn hình overlay đang phủ (command `screen_info`), đã đổi sang CSS pixel. */
export interface ScreenInfo {
  /** Tỉ lệ DPI của màn hình: 1 là 100%, 1.5 là 150%. */
  scaleFactor: number;
  /** Toàn bộ màn hình, trùng với kích thước overlay. */
  bounds: Rect;
  /** Vùng làm việc (trừ taskbar). Mép dưới là chỗ pet đứng. */
  workArea: Rect;
}

export interface AppError {
  code: ErrorCode;
  message: string;
}

export type ErrorCode = "BAD_REQUEST" | "NO_WINDOW" | "INTERNAL";

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value
  );
}

export function errorMessage(value: unknown): string {
  if (isAppError(value)) return value.message;
  if (value instanceof Error) return value.message;
  return String(value);
}
