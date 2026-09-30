import { clamp } from "./math";
import type { Pet } from "./pet";
import { openSpanAt, type Ledge, type Wall } from "./terrain";
import { TUNING } from "./tuning";

/** Chỗ pet đang đứng: mặt đất hoặc mép trên một cửa sổ. */
export interface Ground {
  y: number;
  /** `null`: mặt đất (mép trên taskbar). */
  ledge: Ledge | null;
  /** Khoảng điểm chân đi lại được. */
  from: number;
  to: number;
}

/** Chỗ nhảy tới: điểm chân lúc đáp, `id` là cửa sổ đáp lên (`null`: mặt đất). */
export interface JumpTarget {
  id: number | null;
  x: number;
  y: number;
}

export function groundOf(pet: Pet): Ground {
  const { terrain, bounds } = pet.env;
  const half = pet.width / 2;
  const ledge = pet.mount && pet.state !== "climb" ? terrain.ledge(pet.mount.id) : undefined;
  if (!ledge) return { y: bounds.floor, ledge: null, from: bounds.left + half, to: bounds.right - half };
  const margin = pet.width * TUNING.ledgeMargin;
  return {
    y: ledge.y,
    ledge,
    from: Math.max(ledge.from + margin, bounds.left + half),
    to: Math.min(ledge.to - margin, bounds.right - half),
  };
}

/** Mép đủ xa trần để pet đứng không bị cắt đầu. */
export function roomy(pet: Pet, ledge: Ledge): boolean {
  return ledge.y - pet.env.bounds.top >= pet.height * TUNING.headroom;
}

/** Chỗ đáp lên mép cửa sổ: `x` là điểm chân cắt ngang mép. */
export interface Landing {
  ledge: Ledge;
  x: number;
}

/**
 * Mép cao nhất pet rơi qua trong bước vừa rồi (điểm chân từ (`prevX`, `prevY`) tới vị trí hiện tại) mà
 * đáp được. Xét đúng chỗ chân cắt ngang mép chứ không phải chỗ sau bước: bay nhanh thì mỗi bước lệch
 * cả chục pixel, đáp sát đầu mép sẽ trượt.
 */
export function landingLedge(pet: Pet, prevX: number, prevY: number): Landing | null {
  let best: Landing | null = null;
  for (const ledge of pet.env.terrain.ledges) {
    if (ledge.id === pet.skipLedge || ledge.y < prevY || ledge.y > pet.y) continue;
    if (!roomy(pet, ledge)) continue;
    const t = pet.y > prevY ? (ledge.y - prevY) / (pet.y - prevY) : 0;
    const x = prevX + t * (pet.x - prevX);
    if (!openSpanAt(ledge.open, x)) continue;
    if (!best || ledge.y < best.ledge.y) best = { ledge, x };
  }
  return best;
}

/** Pet đứng trên cửa sổ mà bị cửa sổ khác che gần hết. */
export function hidden(pet: Pet): boolean {
  const mount = pet.mount;
  if (!mount) return false;
  const { terrain } = pet.env;
  return (
    terrain.covered(mount.id, pet.x, pet.y - 1) &&
    terrain.covered(mount.id, pet.x, pet.y - pet.height / 2)
  );
}

/** Chỗ gần nhất trên mép đang đứng mà không bị che, `null` nếu bị che hết. */
export function nearestOpenX(pet: Pet, ground: Ground): number | null {
  if (!ground.ledge) return null;
  let best: number | null = null;
  for (const span of ground.ledge.open) {
    const from = Math.max(span.from, ground.from);
    const to = Math.min(span.to, ground.to);
    if (to - from < 8) continue;
    // Ra hẳn khỏi chỗ bị che nếu đoạn đủ rộng.
    const pad = Math.min(pet.width / 2, (to - from) / 2);
    const x = clamp(pet.x, from + pad, to - pad);
    if (best === null || Math.abs(x - pet.x) < Math.abs(best - pet.x)) best = x;
  }
  return best;
}

/**
 * Pet bị che mà không còn chỗ nào để ra: bị che gần hết trên mép bị che hết (bấm vào cửa sổ phóng to
 * nằm dưới cửa sổ đang đứng, kéo cửa sổ khác đè lên), hoặc chỗ đang bám trên cạnh cửa sổ bị che.
 */
export function buried(pet: Pet): boolean {
  const { mount, climbing } = pet;
  if (!mount) return false;
  if (pet.state === "climb") {
    // Xét cột pixel ngoài cạnh như lúc chọn cạnh để leo (`openWall`), không xét thân pet: cửa sổ nằm trên
    // sát cạnh mà không che cạnh thì pet leo được, không bám vào rồi buông ra mãi.
    const wall = climbing && pet.env.terrain.wall(mount.id, climbing.side);
    if (!wall) return false;
    return !openSpanAt(wall.open, clamp(pet.y - pet.height / 2, wall.from, wall.to));
  }
  return hidden(pet) && nearestOpenX(pet, groundOf(pet)) === null;
}

/** Cạnh `wall` không bị che từ mép trên xuống tới `bottom`: leo trên đó không bị khuất. */
export function openWall(wall: Wall, bottom = wall.to): boolean {
  const to = Math.min(bottom, wall.to);
  return wall.open.some((span) => span.from <= wall.from + 1 && span.to >= to - 1);
}

/** Các chỗ nhảy tới được: mép cửa sổ khác trong tầm, và mặt đất nếu đang ở trên cửa sổ không quá cao. */
export function jumpTargets(pet: Pet): JumpTarget[] {
  const { terrain, bounds } = pet.env;
  const y0 = groundOf(pet).y;
  const here = pet.mount?.id;
  const half = pet.width / 2;
  const margin = pet.width * TUNING.ledgeMargin;
  const targets: JumpTarget[] = [];
  for (const ledge of terrain.ledges) {
    if (ledge.id === here || !roomy(pet, ledge)) continue;
    const dy = ledge.y - y0;
    if (dy < -TUNING.maxJumpUp || dy > TUNING.maxJumpDown) continue;
    const lo = Math.max(ledge.from + margin, bounds.left + half);
    const hi = Math.min(ledge.to - margin, bounds.right - half);
    for (const span of ledge.open) {
      // Đáp hẳn vào phần nhìn thấy, không đáp sát chỗ bị che.
      const from = Math.max(span.from + 8, lo);
      const to = Math.min(span.to - 8, hi);
      if (to < from) continue;
      const x = clamp(pet.x, from, to);
      const dx = x - pet.x;
      if (Math.abs(dx) > TUNING.maxJumpX || (Math.abs(dx) < 16 && Math.abs(dy) < 16)) continue;
      targets.push({ id: ledge.id, x, y: ledge.y });
    }
  }
  if (here !== undefined && bounds.floor - y0 <= TUNING.maxJumpDown) {
    const x = clamp(pet.x + pet.facing * 60, bounds.left + half, bounds.right - half);
    targets.push({ id: null, x, y: bounds.floor });
  }
  return targets;
}

/**
 * Cạnh cửa sổ leo lên được từ chỗ đang đứng: đủ cao, đáy chạm tới chân hoặc trong tầm nhảy lên bám, không
 * bị che, trên đỉnh đứng được.
 */
export function climbTargets(pet: Pet, ground: Ground): Wall[] {
  const { terrain } = pet.env;
  const margin = pet.width * TUNING.ledgeMargin;
  return terrain.walls.filter((wall) => {
    if (wall.id === pet.mount?.id) return false;
    const ledge = terrain.ledge(wall.id);
    const rect = terrain.window(wall.id);
    if (!ledge || !rect || !roomy(pet, ledge)) return false;
    if (ground.y - ledge.y < TUNING.minClimb) return false;
    // Đáy cửa sổ cao quá tầm nhảy lên bám thì chịu.
    if (rect.y + rect.height + pet.height * TUNING.grabDepth < ground.y - TUNING.maxGrabJump) return false;
    const contact = wall.x + wall.side * pet.reach;
    if (contact < ground.from || contact > ground.to) return false;
    if (Math.abs(contact - pet.x) > TUNING.climbSearch) return false;
    if (!openWall(wall, ground.y)) return false;
    const top = wall.side < 0 ? ledge.from + margin : ledge.to - margin;
    return openSpanAt(ledge.open, top) !== undefined;
  });
}
