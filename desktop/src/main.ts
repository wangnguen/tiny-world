import "./overlay.css";
import { errorMessage, type CursorInfo, type ScreenInfo, type Settings } from "@tinyworld/core";
import { FixedStep, World, parseWorldSnapshot, type Bounds } from "@tinyworld/sim";
import { api } from "./api";
import { AutoSave } from "./overlay/autosave";
import { ClickThrough } from "./overlay/clickThrough";
import { PetInteraction } from "./overlay/interaction";
import { PetView } from "./overlay/petView";
import { loadSpriteSet, resolvePack } from "./overlay/sprites";

/** Tần số mô phỏng và vẽ tối đa: sprite thường chỉ 8–12 fps nên 30 là đủ mượt mà vẫn nhẹ. */
const FPS = 30;
/** Khoảng cách từ mép phải vùng làm việc tới pet lúc xuất hiện lần đầu (CSS pixel). */
const SPAWN_MARGIN = 48;
/** Chu kỳ lưu world.json (chỉ ghi khi có thay đổi). */
const SAVE_INTERVAL_MS = 30_000;

/** Pet sống trong vùng làm việc; mặt đất là mép dưới, tức là mép trên taskbar. */
function boundsOf(screen: ScreenInfo): Bounds {
  const { x, y, width, height } = screen.workArea;
  return { left: x, right: x + width, top: y, floor: y + height };
}

/** File hỏng hay đọc lỗi thì bắt đầu lại từ đầu, lần lưu sau sẽ ghi đè. */
async function loadSaved(): Promise<unknown> {
  try {
    return await api.loadState();
  } catch (error) {
    console.warn("Không đọc được trạng thái đã lưu, bắt đầu lại:", errorMessage(error));
    return null;
  }
}

async function start(): Promise<void> {
  const container = document.getElementById("world");
  if (!container) throw new Error("Thiếu phần tử #world.");
  const [screen, settings, saved] = await Promise.all([
    api.screenInfo(),
    api.getSettings(),
    loadSaved(),
  ]);
  /** Pack đang hiện (nhân vật chọn trong Settings, pack đó không còn thì pack đầu tiên). */
  let petId = resolvePack(settings.pet);
  const sprite = await loadSpriteSet(petId);

  const world = new World(boundsOf(screen), Date.now());
  world.speed = settings.speed;
  const view = new PetView(sprite, container, settings.size);
  const pet = world.spawn({
    id: "pet-1",
    x: world.bounds.right - SPAWN_MARGIN - view.width / 2,
    width: view.width,
    height: view.height,
  });
  const previous = parseWorldSnapshot(saved)?.pets.find((p) => p.id === pet.id);
  if (previous) pet.restore(previous);

  const autosave = new AutoSave(world, api.saveState);
  autosave.markSaved();
  autosave.start(SAVE_INTERVAL_MS);

  let paused = false;
  // Overlay để chuột đi xuyên nên không tự nhận được sự kiện chuột: Rust gửi vị trí con trỏ sang,
  // con trỏ nằm trên phần có hình của pet (và không giữ Ctrl) thì overlay nhận chuột.
  const clickThrough = new ClickThrough(api.setClickThrough);
  let cursor: CursorInfo | null = null;
  const refreshClickThrough = () =>
    clickThrough.update(!paused && cursor !== null && !cursor.passThrough && view.hitTest(cursor));
  await api.onCursorMoved((info) => {
    cursor = info;
    refreshClickThrough();
  });
  window.addEventListener("contextmenu", (event) => event.preventDefault());

  const step = new FixedStep(1 / FPS);
  let last = 0;
  let running = false;
  let hidden = false;
  const frame = (now: number) => {
    if (hidden || paused) {
      running = false;
      return;
    }
    const elapsed = (now - last) / 1000;
    // rAF chạy theo tần số màn hình (60–144 Hz), bỏ bớt lượt để giữ tối đa FPS.
    if (elapsed >= 1 / FPS - 0.002) {
      last = now;
      for (let n = step.advance(elapsed); n > 0; n--) world.step(step.dt);
      view.update(pet);
      // Pet đổi frame hoặc đi dưới con trỏ đang đứng yên thì cũng phải tính lại.
      refreshClickThrough();
      // Pet ngủ thì dừng hẳn vòng lặp; click hoặc kéo sẽ chạy lại.
      if (world.resting) {
        running = false;
        return;
      }
    }
    requestAnimationFrame(frame);
  };
  const wake = () => {
    if (running || hidden || paused) return;
    running = true;
    last = performance.now();
    requestAnimationFrame(frame);
  };

  // Overlay ẩn (tray, app fullscreen) thì dừng hẳn, pet đứng nguyên chỗ cũ; hiện lại thì chạy tiếp.
  await api.onVisibilityChanged((visible) => {
    hidden = !visible;
    if (visible) wake();
  });
  // Tạm dừng: pet đứng yên và chuột đi xuyên qua pet, bỏ tạm dừng thì sống tiếp.
  await api.onPaused((value) => {
    paused = value;
    refreshClickThrough();
    if (!paused) wake();
  });
  let latest = settings;
  const applySettings = async (next: Settings) => {
    latest = next;
    const id = resolvePack(next.pet);
    if (id !== petId) {
      petId = id;
      const sprite = await loadSpriteSet(id);
      // Trong lúc nạp đã chọn nhân vật khác: lần gọi sau sẽ áp dụng.
      if (petId !== id) return;
      view.setSprite(sprite);
    }
    // Có thể đã đổi cỡ, tốc độ trong lúc nạp pack, nên dùng giá trị mới nhất.
    world.speed = latest.speed;
    view.setSize(latest.size);
    pet.resize(view.width, view.height);
    view.update(pet);
    refreshClickThrough();
  };
  await api.onSettingsChanged(applySettings);
  await api.onQuitRequested(async () => {
    await autosave.flush();
    await api.quit();
  });

  new PetInteraction(pet, view, {
    onHold: (held) => clickThrough.hold(held),
    onActivity: wake,
  });
  view.update(pet);
  wake();

  // Chỉ khi chạy dev: xem và chỉnh pet từ DevTools (tray → Mở DevTools), ví dụ
  // `__tinyworld.pet.sinceInteraction = 1e6` để pet đi ngủ ngay. Bản build không có dòng này.
  if (import.meta.env.DEV) {
    Object.assign(window, { __tinyworld: { world, pet, view, wake, autosave } });
  }
}

start().catch((error) => console.error("Không khởi động được overlay:", errorMessage(error)));
