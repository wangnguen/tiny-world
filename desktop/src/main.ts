import "./overlay.css";
import {
  errorMessage,
  type CursorInfo,
  type Point,
  type ScreenInfo,
  type Settings,
  type WindowList,
} from "@tinyworld/core";
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
/** Hai lần xin Rust cho overlay sang màn hình khác cách nhau ít nhất chừng này (ms). */
const MOVE_INTERVAL_MS = 200;

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
  world.setScreen(boundsOf(screen), screen.neighbors);
  world.speed = settings.speed;
  const view = new PetView(sprite, container, settings.size);
  const pet = world.spawn({
    id: "pet-1",
    x: world.bounds.right - SPAWN_MARGIN - view.width / 2,
    width: view.width,
    height: view.height,
    reach: view.reach,
  });
  const previous = parseWorldSnapshot(saved)?.pets.find((p) => p.id === pet.id);
  if (previous) pet.restore(previous);

  const autosave = new AutoSave(world, api.saveState);
  autosave.markSaved();
  autosave.start(SAVE_INTERVAL_MS);

  let paused = false;
  // Khai báo trước khi đăng ký event: event có thể tới ngay khi vừa đăng ký xong.
  const step = new FixedStep(1 / FPS);
  let last = 0;
  let running = false;
  let hidden = false;
  // Overlay để chuột đi xuyên nên không tự nhận được sự kiện chuột: Rust gửi vị trí con trỏ sang,
  // con trỏ nằm trên phần có hình của pet (và không giữ Ctrl) thì overlay nhận chuột.
  const clickThrough = new ClickThrough(api.setClickThrough);
  let cursor: CursorInfo | null = null;
  const refreshClickThrough = () =>
    clickThrough.update(!paused && cursor !== null && !cursor.passThrough && view.hitTest(cursor));
  await api.onCursorMoved((info) => {
    // Vừa bấm chuột ở bất kỳ đâu (kể cả ngoài pet): pet đang ngủ thì thức dậy.
    if (info.pressed && !cursor?.pressed && !paused && pet.state === "sleep") {
      pet.wake();
      wake();
    }
    cursor = info;
    // Con trỏ là một thứ sống trên màn hình với pet. Chỉ báo lúc vòng lặp đang chạy: pet đang ngủ không
    // để ý con trỏ, và lúc chạy lại không bị tính một cú giật chuột từ chỗ cũ tới chỗ mới.
    if (running) world.moveCursor(info.x, info.y, info.pressed, performance.now() / 1000);
    refreshClickThrough();
  });
  window.addEventListener("contextmenu", (event) => event.preventDefault());

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
      // Pet đang đi hoặc bay ra khỏi mép giáp màn hình khác: overlay sang bên đó.
      const leaving = pet.leaving;
      if (leaving) moveOverlay(leaving);
      redraw();
      // Pet ngủ thì dừng hẳn vòng lặp; click hoặc kéo sẽ chạy lại.
      if (world.resting) {
        running = false;
        return;
      }
    }
    requestAnimationFrame(frame);
  };
  /** Vẽ lại pet; pet đổi frame, bị cửa sổ che, hoặc đi dưới con trỏ đang đứng yên thì cũng tính lại click-through. */
  const redraw = () => {
    view.update(pet, world.occluders(pet));
    refreshClickThrough();
  };
  const wake = () => {
    if (running || hidden || paused) return;
    running = true;
    last = performance.now();
    requestAnimationFrame(frame);
  };

  let moving = false;
  let lastMove = Number.NEGATIVE_INFINITY;
  /** Điểm `point` (CSS pixel của overlay) nằm trên màn hình khác thì Rust cho overlay sang đó (`onScreenChanged`). */
  const moveOverlay = (point: Point) => {
    const now = performance.now();
    if (moving || now - lastMove < MOVE_INTERVAL_MS) return;
    moving = true;
    lastMove = now;
    api
      .moveOverlay(point.x, point.y)
      .catch((error: unknown) => console.warn("Không đưa được overlay sang màn hình khác:", errorMessage(error)))
      .finally(() => {
        moving = false;
      });
  };

  const interaction = new PetInteraction(pet, view, {
    onHold: (held) => clickThrough.hold(held),
    onActivity: wake,
    // Kéo pet sang màn hình khác.
    onDragOutside: moveOverlay,
  });

  // Overlay ẩn (tray, app fullscreen) thì dừng hẳn, pet đứng nguyên chỗ cũ; hiện lại thì chạy tiếp.
  await api.onVisibilityChanged((visible) => {
    hidden = !visible;
    if (hidden) interaction.cancel();
    if (visible) wake();
  });
  // Overlay sang màn hình khác (pet bị kéo, ném, tự đi sang), hoặc màn hình đổi độ phân giải, DPI, taskbar:
  // đổi toạ độ pet và cửa sổ sang overlay mới. Danh sách cửa sổ theo toạ độ mới tới ngay sau đó.
  await api.onScreenChanged(({ screen, remap }) => {
    world.setScreen(boundsOf(screen), screen.neighbors, remap);
    interaction.remap(remap);
    view.resize();
    redraw();
    if (!paused) wake();
  });
  // Tạm dừng: pet đứng yên và chuột đi xuyên qua pet, bỏ tạm dừng thì sống tiếp.
  await api.onPaused((value) => {
    paused = value;
    if (paused) interaction.cancel();
    refreshClickThrough();
    if (!paused) wake();
  });
  // Cửa sổ mở, đóng, di chuyển: dựng lại địa hình. Đăng ký trước rồi mới lấy danh sách hiện tại, để
  // không lỡ thay đổi nào ở giữa; đã nhận event thì danh sách lấy về (cũ hơn) bỏ đi.
  let windowsSeen = false;
  const applyWindows = (list: WindowList) => {
    world.setWindows(list.windows, list.closed);
    // Pet ngủ trên taskbar không bị cửa sổ ảnh hưởng: không chạy lại vòng lặp.
    if (world.resting) return;
    // Tạm dừng thì không có vòng lặp, pet đứng trên cửa sổ vẫn phải đi theo.
    if (paused) redraw();
    else wake();
  };
  await api.onWindowsChanged((list) => {
    windowsSeen = true;
    applyWindows(list);
  });
  const windows = await api.listWindows().catch((error: unknown) => {
    console.warn("Không lấy được danh sách cửa sổ:", errorMessage(error));
    return null;
  });
  if (windows && !windowsSeen) applyWindows(windows);

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
    pet.resize(view.width, view.height, view.reach);
    redraw();
  };
  await api.onSettingsChanged(applySettings);
  await api.onQuitRequested(async () => {
    await autosave.flush();
    await api.quit();
  });

  redraw();
  wake();

  // WebView đổi DPI (chuyển màn hình/Windows scaling) thì vẽ lại cả pet đang ngủ.
  window.addEventListener("resize", () => {
    view.resize();
    redraw();
  });

  // Chỉ khi chạy dev: xem và chỉnh pet từ DevTools (tray → Mở DevTools), ví dụ
  // `__tinyworld.pet.sinceInteraction = 1e6` để pet đi ngủ ngay. Bản build không có dòng này.
  if (import.meta.env.DEV) {
    Object.assign(window, { __tinyworld: { world, pet, view, wake, autosave } });
  }
}

start().catch((error) => console.error("Không khởi động được overlay:", errorMessage(error)));
