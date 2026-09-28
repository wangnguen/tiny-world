import "./overlay.css";
import { errorMessage, type Point, type ScreenInfo } from "@tinyworld/core";
import { FixedStep, World, type Floor } from "@tinyworld/sim";
import { api } from "./api";
import { ClickThrough } from "./overlay/clickThrough";
import { PetView } from "./overlay/petView";
import { loadSpriteSet } from "./overlay/sprites";

/** Tần số mô phỏng và vẽ tối đa: sprite thường chỉ 8–12 fps nên 30 là đủ mượt mà vẫn nhẹ. */
const FPS = 30;
/** Khoảng cách từ mép phải vùng làm việc tới pet lúc xuất hiện (CSS pixel). */
const SPAWN_MARGIN = 48;

/** Pet đứng trên mép dưới vùng làm việc, tức là mép trên taskbar. */
function floorOf(screen: ScreenInfo): Floor {
  const { x, y, width, height } = screen.workArea;
  return { left: x, right: x + width, y: y + height };
}

async function start(): Promise<void> {
  const container = document.getElementById("world");
  if (!container) throw new Error("Thiếu phần tử #world.");
  const [screen, sprite] = await Promise.all([api.screenInfo(), loadSpriteSet()]);

  const world = new World(floorOf(screen), Date.now());
  const width = sprite.frameWidth * sprite.scale;
  const height = sprite.frameHeight * sprite.scale;
  const pet = world.spawn({
    id: "pet-1",
    x: world.floor.right - SPAWN_MARGIN - width / 2,
    width,
    height,
  });
  const view = new PetView(sprite, container);

  // Overlay để chuột đi xuyên nên không tự nhận được sự kiện chuột: Rust gửi vị trí con trỏ sang,
  // con trỏ nằm trên phần có hình của pet thì overlay nhận chuột để click được.
  const clickThrough = new ClickThrough(api.setClickThrough);
  let cursor: Point | null = null;
  const refreshClickThrough = () => clickThrough.update(cursor !== null && view.hitTest(cursor));
  await api.onCursorMoved((position) => {
    cursor = position;
    refreshClickThrough();
  });

  window.addEventListener("pointerdown", (event) => {
    if (event.button === 0 && view.hitTest({ x: event.clientX, y: event.clientY })) pet.poke();
  });
  window.addEventListener("contextmenu", (event) => event.preventDefault());

  const step = new FixedStep(1 / FPS);
  let last = performance.now();
  const frame = (now: number) => {
    const elapsed = (now - last) / 1000;
    // rAF chạy theo tần số màn hình (60–144 Hz), bỏ bớt lượt để giữ tối đa FPS.
    if (elapsed >= 1 / FPS - 0.002) {
      last = now;
      for (let n = step.advance(elapsed); n > 0; n--) world.step(step.dt);
      view.update(pet);
      // Pet đổi frame hoặc đi dưới con trỏ đang đứng yên thì cũng phải tính lại.
      refreshClickThrough();
    }
    requestAnimationFrame(frame);
  };
  view.update(pet);
  requestAnimationFrame(frame);
}

start().catch((error) => console.error("Không khởi động được overlay:", errorMessage(error)));
