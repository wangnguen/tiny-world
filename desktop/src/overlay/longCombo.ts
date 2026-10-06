import type { Facing, Pet } from "@tinyworld/sim";
import type { LongActionEffect, TeleportStreakEffect } from "./longAction";
import type { PetView } from "./petView";

/** Pack của Long: Shift + click vào Long thì bật aura, có con khác thì diễn combo quật đuôi. */
export const LONG_PACK = "c-long";

/** Thời lượng từng đoạn của combo (ms). */
export const TRANSFORM_MS = 1_100;
export const BLINK_MS = 180;
export const SETTLE_MS = 100;
export const SWIPE_MS = 650;
export const STUN_MS = 550;
/** Mục tiêu đang bận (bị kéo, đang rơi, leo, nhảy, chat) thì chờ tối đa chừng này rồi bỏ qua nó. */
export const WAIT_MS = 2_000;
/** Bị quật trúng thì biến mất chừng này giây. */
export const VANISH_SECONDS = 5;
/** Long hiện ra cách mục tiêu chừng này lần tổng bề ngang hai con (giữa hai điểm chân). */
const REACH = 0.42;
/** Mục tiêu đứng yên tới lúc trúng đòn, dư nửa giây cho vòng lặp vẽ trễ. */
const TARGET_HOLD = (2 * BLINK_MS + SETTLE_MS + SWIPE_MS) / 1000 + 0.5;
/** Long đứng yên từ lúc hiện ra cạnh mục tiêu tới hết lúc mục tiêu choáng. */
const LONG_HOLD = (BLINK_MS + SETTLE_MS + SWIPE_MS + STUN_MS) / 1000 + 0.5;

/** Một con tham gia combo: phần mô phỏng và phần vẽ. */
export interface Fighter {
  readonly pet: Pet;
  readonly view: Pick<PetView, "width" | "height" | "setCovered">;
}

/** Đồ diễn của Long (`LongKit`) mà combo dùng tới. */
export interface ComboKit {
  readonly action: Pick<LongActionEffect, "play" | "stop">;
  readonly teleport: Pick<TeleportStreakEffect, "play">;
}

type Phase = "transform" | "seek" | "blink-out" | "blink-in" | "settle" | "swipe" | "stun" | "done";

/**
 * Combo của Long: biến hình, rồi lần lượt tốc biến tới cạnh từng con khác quật đuôi một cái; con bị trúng
 * giật mình rồi biến mất `VANISH_SECONDS` giây. Chỉ đổi chỗ đứng và state trong mô phỏng (`Pet.hold`,
 * `blinkTo`, `vanish`), không đụng tới cài đặt hay dữ liệu lưu. Gọi `update` mỗi lần vẽ.
 */
export class LongCombo {
  private phase: Phase = "transform";
  private until: number;
  private target: Fighter | null = null;
  /** Phía Long đánh tới mục tiêu hiện tại (1: mục tiêu ở bên phải). */
  private facing: Facing;
  private readonly queue: Fighter[];

  constructor(
    private readonly long: Fighter,
    private readonly kit: ComboKit,
    targets: readonly Fighter[],
    now: number,
  ) {
    this.queue = [...targets];
    this.facing = long.pet.facing;
    // Đang rảnh thì đứng yên biến hình tại chỗ; đang rơi, leo, nhảy thì cứ thế, lúc tốc biến sẽ đứng hẳn lên.
    long.pet.hold(this.facing, TRANSFORM_MS / 1000 + 0.5);
    long.view.setCovered(true);
    kit.action.play("transform", now, this.facing);
    this.until = now + TRANSFORM_MS;
  }

  get done(): boolean {
    return this.phase === "done";
  }

  /**
   * `fighter` bị bỏ khỏi màn hình (đổi nhân vật trong Settings): bỏ chính Long thì dừng hẳn, bỏ mục tiêu thì
   * bỏ qua nó.
   */
  drop(fighter: Fighter): void {
    if (fighter === this.long) {
      this.stop();
      return;
    }
    this.forget(fighter);
  }

  /** Dừng hẳn: Long hiện lại như thường. */
  stop(): void {
    if (this.done) return;
    this.kit.action.stop();
    this.long.view.setCovered(false);
    this.target = null;
    this.phase = "done";
  }

  update(now: number): void {
    if (this.done) return;
    const { long, kit } = this;
    // Người dùng nhấc Long lên giữa chừng: thôi diễn, để Long bị kéo như thường.
    if (long.pet.state === "dragged") {
      this.stop();
      return;
    }
    const target = this.target;
    if (this.phase !== "transform" && this.phase !== "seek") {
      // Mục tiêu bị bỏ khỏi màn hình hoặc bị nhấc lên giữa chừng: bỏ qua, sang con tiếp theo.
      if (!target || target.pet.state === "dragged") {
        this.miss(now);
        return;
      }
      // Hai con luôn quay mặt vào nhau tới hết đòn: đứng yên thì sim quay đầu nhìn theo con trỏ.
      long.pet.facing = this.facing;
      target.pet.facing = this.facing === 1 ? -1 : 1;
    }
    if (this.phase === "seek") {
      this.seek(now);
      return;
    }
    if (now < this.until) return;
    if (this.phase === "transform") {
      kit.action.stop();
      long.view.setCovered(false);
      this.wait(now);
      this.seek(now);
      return;
    }
    if (!target) return;
    switch (this.phase) {
      case "blink-out": {
        const reach = (long.view.width + target.view.width) * REACH;
        long.pet.blinkTo(target.pet, target.pet.x - this.facing * reach, LONG_HOLD);
        // Kẹp vào chỗ đứng (sát mép màn hình, mép cửa sổ hẹp) có khi phải đứng phía bên kia.
        this.facing = long.pet.facing;
        const at = { x: long.pet.x, y: long.pet.y };
        kit.teleport.play(at, long.view.width, long.view.height, this.facing, true, now, BLINK_MS);
        this.next("blink-in", now + BLINK_MS);
        return;
      }
      case "blink-in":
        // Vệt xuất hiện chạy hết rồi Long mới hiện rõ một chút trước khi quật.
        long.view.setCovered(false);
        this.next("settle", now + SETTLE_MS);
        return;
      case "settle":
        long.view.setCovered(true);
        kit.action.play("tail-swipe", now, this.facing);
        this.next("swipe", now + SWIPE_MS);
        return;
      case "swipe":
        kit.action.stop();
        long.view.setCovered(false);
        target.pet.startle();
        this.next("stun", now + STUN_MS);
        return;
      case "stun":
        target.pet.vanish(VANISH_SECONDS);
        this.forget(target);
        this.wait(now);
        this.seek(now);
        return;
    }
  }

  /**
   * Tìm mục tiêu: con đầu hàng đứng yên được thì tốc biến tới nó; đang bận thì chờ, chờ quá `WAIT_MS` thì
   * bỏ qua con đó. Hết con thì xong.
   */
  private seek(now: number): void {
    const { long, kit } = this;
    for (;;) {
      const target = this.queue[0];
      if (!target) {
        this.stop();
        return;
      }
      const facing: Facing = target.pet.x >= long.pet.x ? 1 : -1;
      if (target.pet.hold(facing === 1 ? -1 : 1, TARGET_HOLD)) {
        this.target = target;
        this.facing = facing;
        // Đứng yên ở chỗ cũ trong lúc vệt biến đi chạy.
        long.pet.hold(facing, BLINK_MS / 1000 + 0.5);
        long.view.setCovered(true);
        const at = { x: long.pet.x, y: long.pet.y };
        kit.teleport.play(at, long.view.width, long.view.height, facing, false, now, BLINK_MS);
        this.next("blink-out", now + BLINK_MS);
        return;
      }
      if (now < this.until) return;
      this.queue.shift();
      this.until = now + WAIT_MS;
    }
  }

  /** Đòn đang dở mà mục tiêu không còn: Long hiện lại, tìm con tiếp theo. */
  private miss(now: number): void {
    this.kit.action.stop();
    this.long.view.setCovered(false);
    if (this.target) this.forget(this.target);
    this.wait(now);
    this.seek(now);
  }

  /** Bỏ `fighter` khỏi hàng chờ, thôi nhắm vào nó. */
  private forget(fighter: Fighter): void {
    const index = this.queue.indexOf(fighter);
    if (index >= 0) this.queue.splice(index, 1);
    if (fighter === this.target) this.target = null;
  }

  /** Bắt đầu chờ mục tiêu tiếp theo rảnh. */
  private wait(now: number): void {
    this.phase = "seek";
    this.until = now + WAIT_MS;
  }

  private next(phase: Phase, until: number): void {
    this.phase = phase;
    this.until = until;
  }
}
