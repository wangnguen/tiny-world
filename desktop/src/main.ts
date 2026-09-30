import "./overlay.css";
import {
  errorMessage,
  type CursorInfo,
  type Point,
  type Rect,
  type ScreenInfo,
  type Settings,
  type WindowList,
} from "@tinyworld/core";
import { FixedStep, StepBlend, TUNING, World, clamp, parseWorldSnapshot, type Bounds } from "@tinyworld/sim";
import { api } from "./api";
import { AutoSave } from "./overlay/autosave";
import { ClickThrough } from "./overlay/clickThrough";
import { PetInteraction } from "./overlay/interaction";
import { PetView } from "./overlay/petView";
import { loadSpriteSet, resolvePack } from "./overlay/sprites";

/**
 * Tần số mô phỏng, cũng là tần số vẽ lúc pet đi đứng bình thường: sprite thường chỉ 8–12 fps nên 30 là đủ
 * mượt mà vẫn nhẹ. Lúc bay hay bị kéo (`FAST_STATES`) thì vẽ theo tần số màn hình.
 */
const FPS = 30;
/**
 * Pet đang bay (bị ném, rơi, nhảy) hoặc đang bị kéo: vẽ lại mỗi lần làm tươi màn hình, ở vị trí nội suy giữa
 * hai bước mô phỏng (`StepBlend`). Vẽ 30 fps thì pet bị ném nhanh nhảy cóc từng đoạn dài, nhìn giật.
 * Chỉ kéo dài vài giây nên không tốn thêm CPU đáng kể.
 */
const FAST_STATES: ReadonlySet<string> = new Set(["fall", "jump", "dragged"]);
/**
 * Vẽ bằng hẹn giờ rồi mới xin `requestAnimationFrame`, chứ không xin rAF liên tục: rAF chạy theo tần số
 * màn hình (60–144 Hz), WebView phải thức dậy gấp mấy lần cần. Hẹn sớm chừng này ms để chờ tới lần làm
 * tươi màn hình kế tiếp thì vừa đúng nhịp.
 */
const VSYNC_SLACK_MS = 8;
const FRAME_MS = 1000 / FPS - VSYNC_SLACK_MS;
/**
 * Pet đứng yên (đứng, ngồi mép) thì chỉ cần thức dậy lúc đổi frame (animation đứng yên chỉ 3–6 fps),
 * nhưng tối đa chừng này ms một lần để mô phỏng không dồn quá `MAX_STEPS` bước một lượt.
 */
const CALM_MAX_MS = 250;
/** Số bước mô phỏng tối đa mỗi lượt vẽ: 10 bước là 333 ms, dư cho lần chờ lâu nhất lúc pet đứng yên. */
const MAX_STEPS = 10;
const CALM_STATES: ReadonlySet<string> = new Set(["idle", "perch"]);
/**
 * Rust chỉ gửi vị trí con trỏ khi nó ở gần pet: pet thức thì trong tầm nhìn theo con trỏ, ngủ thì chỉ quanh
 * thân để bắt click. Báo rộng thêm `INTEREST_SLACK` để pet đi một đoạn mới phải báo lại.
 */
const INTEREST_AWAKE = TUNING.lookRange;
const INTEREST_ASLEEP = 16;
const INTEREST_SLACK = 64;
/** Khoảng cách từ mép phải vùng làm việc tới pet lúc xuất hiện lần đầu (CSS pixel). */
const SPAWN_MARGIN = 48;
/** Chu kỳ lưu world.json (chỉ ghi khi có thay đổi). */
const SAVE_INTERVAL_MS = 30_000;
/** Hai lần xin Rust cho overlay sang màn hình khác cách nhau ít nhất chừng này (ms). */
const MOVE_INTERVAL_MS = 200;

/**
 * Pet sống trong vùng làm việc; mặt đất là mép dưới, tức là mép trên taskbar. Taskbar tự ẩn đang trồi
 * lên (`taskbarTop`) thì mặt đất là mép trên của nó.
 */
function boundsOf(screen: ScreenInfo, taskbarTop: number | null = null): Bounds {
  const { x, y, width, height } = screen.workArea;
  const floor = taskbarTop === null ? y + height : Math.min(y + height, taskbarTop);
  return { left: x, right: x + width, top: y, floor };
}

function inflate(r: Rect, by: number): Rect {
  return { x: r.x - by, y: r.y - by, width: r.width + 2 * by, height: r.height + 2 * by };
}

function containsRect(outer: Rect, inner: Rect): boolean {
  return (
    inner.x >= outer.x &&
    inner.y >= outer.y &&
    inner.x + inner.width <= outer.x + outer.width &&
    inner.y + inner.height <= outer.y + outer.height
  );
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
  /** Màn hình overlay đang phủ, và mép trên taskbar tự ẩn đang trồi lên (`WindowList.taskbarTop`). */
  let current = screen;
  let taskbarTop: number | null = null;
  // Khai báo trước khi đăng ký event: event có thể tới ngay khi vừa đăng ký xong.
  const step = new FixedStep(1 / FPS, MAX_STEPS);
  const blend = new StepBlend();
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
    // Pet đứng yên thì quay đầu nhìn theo con trỏ. Pet đang ngủ không để ý con trỏ.
    if (running) world.moveCursor(info.x, info.y);
    refreshClickThrough();
  });
  window.addEventListener("contextmenu", (event) => event.preventDefault());

  /** Vùng đã báo Rust (`api.setCursorInterest`), lúc báo pet có đang ngủ không. */
  let interest: Rect | null = null;
  let interestAsleep = false;
  const refreshInterest = () => {
    const box = view.bounds;
    if (!box) return;
    const asleep = pet.state === "sleep";
    const needed = inflate(box, asleep ? INTEREST_ASLEEP : INTEREST_AWAKE);
    if (interest && asleep === interestAsleep && containsRect(interest, needed)) return;
    interest = inflate(needed, INTEREST_SLACK);
    interestAsleep = asleep;
    api
      .setCursorInterest(interest)
      .catch((error: unknown) => console.warn("Không báo được vùng quanh pet:", errorMessage(error)));
  };

  /** Hẹn giờ đang chờ để vẽ lần sau (0: không có), và lần đó có phải là chờ lâu lúc pet đứng yên không. */
  let timer = 0;
  let calmWait = false;
  /** Đã báo Rust pet đang ngủ (`api.setResting`). */
  let resting = false;
  const setResting = (value: boolean) => {
    if (value === resting) return;
    resting = value;
    api.setResting(value).catch((error: unknown) => console.warn("Không báo được pet ngủ/thức:", errorMessage(error)));
  };
  const schedule = (delay: number) => {
    calmWait = delay > FRAME_MS;
    timer = window.setTimeout(() => {
      timer = 0;
      requestAnimationFrame(frame);
    }, delay);
  };
  const nextDelay = () => {
    if (!CALM_STATES.has(pet.state)) return FRAME_MS;
    return clamp(view.nextFrameIn(pet) * 1000 - VSYNC_SLACK_MS, FRAME_MS, CALM_MAX_MS);
  };
  const frame = (now: number) => {
    if (hidden || paused) {
      running = false;
      return;
    }
    const elapsed = (now - last) / 1000;
    last = now;
    for (let n = step.advance(elapsed); n > 0; n--) blend.step(pet, () => world.step(step.dt));
    // Pet đang đi hoặc bay ra khỏi mép giáp màn hình khác: overlay sang bên đó.
    const leaving = pet.leaving;
    if (leaving) moveOverlay(leaving);
    redraw();
    // Pet ngủ thì dừng hẳn vòng lặp; click hoặc kéo sẽ chạy lại.
    if (world.resting) {
      running = false;
      setResting(true);
      return;
    }
    if (FAST_STATES.has(pet.state)) requestAnimationFrame(frame);
    else schedule(nextDelay());
  };
  /** Vẽ lại pet; pet đổi frame, bị cửa sổ che, hoặc đi dưới con trỏ đang đứng yên thì cũng tính lại click-through. */
  const redraw = () => {
    view.update(pet, world.occluders(pet), blend.at(pet, step.alpha));
    refreshClickThrough();
    refreshInterest();
  };
  /** Chạy lại vòng lặp; đang chờ lâu lúc pet đứng yên mà có chuyện (click, cửa sổ đổi) thì vẽ ngay lần sau. */
  const wake = () => {
    if (hidden || paused) return;
    if (running) {
      if (timer !== 0 && calmWait) {
        window.clearTimeout(timer);
        schedule(0);
      }
      return;
    }
    running = true;
    setResting(false);
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
    // Mép trên taskbar tự ẩn tính theo overlay cũ: bỏ, danh sách cửa sổ tới ngay sau đó sẽ báo lại.
    current = screen;
    taskbarTop = null;
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
    // Taskbar tự ẩn trồi lên hoặc thụt xuống: pet đứng lên mép trên của nó, hoặc rơi xuống lại đáy màn hình.
    const top = list.taskbarTop ?? null;
    const floorMoved = top !== taskbarTop;
    if (floorMoved) {
      taskbarTop = top;
      world.setScreen(boundsOf(current, taskbarTop), current.neighbors);
    }
    // Pet ngủ trên taskbar không bị cửa sổ ảnh hưởng: không chạy lại vòng lặp, mặt đất đổi thì chỉ vẽ lại.
    if (world.resting) {
      if (floorMoved) redraw();
      return;
    }
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
