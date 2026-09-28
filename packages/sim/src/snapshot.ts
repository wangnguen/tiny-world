import type { Facing } from "./pet";

/**
 * Những gì của một pet cần giữ lại khi tắt/mở app. Chỉ giữ thứ còn đúng khi mở lại: pet luôn được
 * đặt lại trên mặt đất, đang rơi hay đang bị kéo thì coi như đứng.
 */
export interface PetSnapshot {
  id: string;
  /** Điểm chân theo chiều ngang (CSS pixel của overlay). */
  x: number;
  facing: Facing;
  asleep: boolean;
  /** Số giây chưa được click/kéo, để lâu không đụng tới thì mở lại vẫn buồn ngủ. */
  sinceInteraction: number;
}

/** Nội dung `world.json`. */
export interface WorldSnapshot {
  version: typeof SNAPSHOT_VERSION;
  pets: PetSnapshot[];
}

/** Tăng khi đổi định dạng; file cũ khác version thì bỏ qua, bắt đầu lại từ đầu. */
export const SNAPSHOT_VERSION = 1;

/** Đọc `world.json`. File sai version hoặc sai kiểu thì trả `null`; pet nào sai thì bỏ riêng pet đó. */
export function parseWorldSnapshot(value: unknown): WorldSnapshot | null {
  if (!isObject(value) || value.version !== SNAPSHOT_VERSION || !Array.isArray(value.pets)) {
    return null;
  }
  const pets = value.pets.flatMap((item) => {
    const pet = parsePet(item);
    return pet ? [pet] : [];
  });
  return { version: SNAPSHOT_VERSION, pets };
}

function parsePet(value: unknown): PetSnapshot | null {
  if (!isObject(value)) return null;
  const { id, x, facing, asleep, sinceInteraction } = value;
  if (
    typeof id !== "string" ||
    !isFiniteNumber(x) ||
    (facing !== 1 && facing !== -1) ||
    typeof asleep !== "boolean" ||
    !isFiniteNumber(sinceInteraction) ||
    sinceInteraction < 0
  ) {
    return null;
  }
  return { id, x, facing, asleep, sinceInteraction };
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}
