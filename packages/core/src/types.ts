export interface Point {
  x: number;
  y: number;
}

/** Event `cursor-moved`: vị trí con trỏ theo CSS pixel của overlay. */
export interface CursorInfo extends Point {
  /** Đang giữ Ctrl: click xuyên qua pet xuống app bên dưới. */
  passThrough: boolean;
  /** Đang giữ một nút chuột, ở bất kỳ đâu trên màn hình (click ra ngoài cũng làm pet đang ngủ thức dậy). */
  pressed: boolean;
}

/** Hình chữ nhật theo CSS pixel, gốc toạ độ là góc trên trái của overlay. */
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/**
 * Màn hình overlay đang nằm (command `screen_info`), đã đổi sang CSS pixel. Overlay phủ vùng làm
 * việc chứ không phủ cả màn hình, nên gốc toạ độ là góc trên trái vùng làm việc.
 */
export interface ScreenInfo {
  /** Tỉ lệ DPI của màn hình: 1 là 100%, 1.5 là 150%. */
  scaleFactor: number;
  /** Toàn bộ màn hình, kể cả taskbar (taskbar ở trên/trái thì `x`/`y` âm). */
  bounds: Rect;
  /** Vùng làm việc (trừ taskbar), trùng với kích thước overlay. Mép dưới là chỗ pet đứng. */
  workArea: Rect;
  /**
   * Vùng làm việc của các màn hình khác, tính theo toạ độ của overlay này. Màn hình nào giáp mép trái/phải
   * của `workArea` thì pet đi hoặc bay sang được bên đó.
   */
  neighbors: Rect[];
}

/** Đổi toạ độ CSS pixel của overlay cũ sang overlay mới: `p * scale + (x, y)`. */
export interface Remap {
  scale: number;
  x: number;
  y: number;
}

/**
 * Event `screen-changed`: overlay vừa sang màn hình khác, hoặc màn hình đang ở đổi độ phân giải, DPI,
 * taskbar. `remap` đổi toạ độ cũ (pet, cửa sổ) sang toạ độ mới.
 */
export interface ScreenChange {
  screen: ScreenInfo;
  remap: Remap;
}

/** Một cửa sổ thật trên màn hình, khớp `WindowInfo` trong windows.rs. */
export interface WindowInfo {
  /** HWND, không đổi trong suốt đời cửa sổ. */
  id: number;
  /** Khung nhìn thấy (không tính viền kéo giãn trong suốt), CSS pixel; có thể thò ra ngoài overlay. */
  rect: Rect;
}

/**
 * Event `windows-changed` và command `list_windows`: các cửa sổ đang hiện (không tính cửa sổ thu
 * nhỏ, ẩn, ở desktop ảo khác), xếp từ trên xuống dưới theo thứ tự chồng.
 */
export interface WindowList {
  windows: WindowInfo[];
  /** Cửa sổ vừa bị đóng hẳn (không phải thu nhỏ hay ẩn) kể từ lần gửi trước, kèm khung lúc còn hiện. */
  closed: WindowInfo[];
}

/** Cài đặt người dùng (command `get_settings` / `set_settings`), khớp `Settings` trong settings.rs. */
export interface Settings {
  /** Cỡ nhân vật so với cỡ gốc của sprite pack, 0.5–2. */
  size: number;
  /** Hệ số tốc độ đi/chạy, 0.5–2. */
  speed: number;
  /**
   * Nhân vật đang chọn: tên thư mục sprite pack trong `assets/sprites/`. `null` (hoặc pack không còn)
   * là pack đầu tiên theo tên thư mục.
   */
  pet: string | null;
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
