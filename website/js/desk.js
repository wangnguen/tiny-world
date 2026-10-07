// Màn hình thu nhỏ ở đầu trang: hai cửa sổ, taskbar và tối đa ba con pet sống trên đó như trong app. Pet tự
// đi, chạy, leo cạnh cửa sổ, ngồi mép, nhảy sang cửa sổ khác, ngủ; bấm thì nhảy lên, kéo thả thì rơi, vung
// thì bay, thả cao thì choáng, bấm đúp thì nói giờ. Kéo thanh tiêu đề cửa sổ thì pet trên đó đi theo, đóng
// cửa sổ thì pet rơi và các con gần đó ăn mừng. Đây là bản rút gọn để xem trên web, không phải code của app.
import { Bubble, Sprite, preload } from "./sprite.js";
import { greetings, nowText } from "./gen/lines.js";
import { toLunar } from "./gen/lunar.js";

/** Cỡ pet: 96 CSS px như cỡ mặc định trong app. */
const SCALE = 0.5;
/** Chiều cao cảnh và taskbar (px của cảnh); cảnh hẹp hơn chừng này thì thu nhỏ cả cảnh. */
const HEIGHT = 420;
const TASKBAR = 44;
const MIN_WIDTH = 680;
/** Tốc độ (px/s), trọng lực (px/s²), ném nhanh nhất (px/s). */
const SPEED = { walk: 34, run: 92, climb: 46 };
const GRAVITY = 1500;
const MAX_THROW = 1500;
/** Bị thả từ cao hơn chừng này (px) thì choáng. */
const DIZZY_FROM = 150;
const MAX_PETS = 3;
/** Cửa sổ kéo lên sát mép trên hơn chừng này thì hết chỗ đứng, pet rơi. */
const ROOM_ABOVE = 50;

const between = (min, max) => min + Math.random() * (max - min);
const clamp = (value, min, max) => Math.min(max, Math.max(min, value));
const pick = (list) => list[Math.floor(Math.random() * list.length)];
const fill = (template, params) => template.replace(/\{(\w+)\}/g, (m, k) => (k in params ? String(params[k]) : m));

export class Desk {
  /** `root`: phần tử [data-desk]; `defs`: pets.json; `text`: chữ của trang (#tw-text). */
  constructor(root, defs, text) {
    this.root = root;
    this.screen = root.querySelector(".desk__screen");
    this.world = root.querySelector("[data-world]");
    this.layer = root.querySelector("[data-pets]");
    this.defs = new Map(defs.map((def) => [def.id, def]));
    this.text = text;
    this.reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    this.pets = [];
    this.meetings = new Map();
    this.windows = [...root.querySelectorAll("[data-win]")].map((el) => ({
      el,
      id: el.dataset.win,
      x: 0,
      y: 0,
      w: 0,
      h: 0,
      open: true,
      placed: false,
      reopen: 0,
      icon: root.querySelector(`[data-open="${el.dataset.win}"]`),
    }));
    this.onChange = () => {};
    this.running = false;
    this.visible = true;
    this.last = 0;
    this.layout();
    this.bindWindows();
    this.startClock();
    new ResizeObserver(() => this.layout()).observe(this.screen);
    new IntersectionObserver(([entry]) => {
      this.visible = entry.isIntersecting;
      if (this.visible) this.run();
    }).observe(this.screen);
  }

  /** Mặt đất (mép trên taskbar). */
  get ground() {
    return HEIGHT - TASKBAR;
  }

  /** Cảnh rộng theo khung, hẹp quá thì thu nhỏ cả cảnh cho vừa. */
  layout() {
    const outer = this.screen.clientWidth || MIN_WIDTH;
    this.scale = Math.min(1, outer / MIN_WIDTH);
    this.width = Math.round(outer / this.scale);
    this.world.style.width = `${this.width}px`;
    this.world.style.transform = this.scale < 1 ? `scale(${this.scale})` : "";
    this.screen.style.height = `${Math.round(HEIGHT * this.scale)}px`;
    const [notes, music] = this.windows;
    if (notes && !notes.placed) {
      notes.w = Math.round(clamp(this.width * 0.36, 260, 360));
      notes.h = 200;
      notes.x = Math.round(this.width * 0.07);
      notes.y = this.ground - notes.h;
    }
    if (music && !music.placed) {
      music.w = Math.round(clamp(this.width * 0.26, 210, 270));
      music.h = 150;
      music.x = Math.round(this.width * 0.6);
      music.y = 132;
    }
    for (const w of this.windows) {
      const x = clamp(w.x, 0, this.width - w.w);
      this.moveWindow(w, x - w.x, 0);
      Object.assign(w.el.style, { width: `${w.w}px`, height: `${w.h}px` });
    }
    for (const p of this.pets) {
      p.x = clamp(p.x, 24, this.width - 24);
      if (p.on === "ground") p.y = this.ground;
    }
    this.draw(performance.now());
  }

  // ---------------------------------------------------------------- cửa sổ

  bindWindows() {
    for (const w of this.windows) {
      w.icon?.classList.add("tb--open");
      w.el.addEventListener("pointerdown", () => this.raise(w));
      const bar = w.el.querySelector("[data-win-bar]");
      bar.addEventListener("pointerdown", (event) => {
        if (event.button !== 0 || event.target.closest("button")) return;
        event.preventDefault();
        bar.setPointerCapture(event.pointerId);
        const start = this.toWorld(event);
        w.grab = { id: event.pointerId, dx: start.x - w.x, dy: start.y - w.y };
        w.placed = true;
      });
      bar.addEventListener("pointermove", (event) => {
        if (!w.grab || w.grab.id !== event.pointerId) return;
        const point = this.toWorld(event);
        const x = clamp(point.x - w.grab.dx, 0, this.width - w.w);
        const y = clamp(point.y - w.grab.dy, 0, this.ground - w.h);
        this.moveWindow(w, x - w.x, y - w.y);
        this.run();
      });
      const drop = () => (w.grab = null);
      bar.addEventListener("pointerup", drop);
      bar.addEventListener("pointercancel", drop);
      w.el.addEventListener("click", (event) => {
        const act = event.target.closest("[data-win-act]")?.dataset.winAct;
        if (act) this.closeWindow(w, act === "close");
      });
      w.icon?.addEventListener("click", () => (w.open ? this.closeWindow(w, false) : this.openWindow(w)));
    }
  }

  raise(w) {
    for (const other of this.windows) other.el.style.zIndex = other === w ? "1" : "0";
  }

  /** Dời cửa sổ; pet đứng trên đó đi theo, ra khỏi mép hay hết chỗ đứng thì rơi. */
  moveWindow(w, dx, dy) {
    w.x += dx;
    w.y += dy;
    w.el.style.left = `${w.x}px`;
    w.el.style.top = `${w.y}px`;
    const now = performance.now();
    for (const p of this.pets) {
      if (p.state === "climb" && p.climbWin === w) p.y += dy;
      if (p.on !== w) continue;
      p.x += dx;
      p.target += dx;
      p.y = w.y;
      if (p.x < w.x - 1 || p.x > w.x + w.w + 1 || w.y < ROOM_ABOVE) this.fall(p, now);
    }
  }

  closeWindow(w, celebrate) {
    if (!w.open) return;
    w.open = false;
    w.el.classList.add("win--closed");
    w.icon?.classList.remove("tb--open");
    const now = performance.now();
    for (const p of this.pets) {
      if (p.on === w || (p.state === "climb" && p.climbWin === w)) this.fall(p, now);
    }
    if (celebrate) this.cheer(w, now);
    window.clearTimeout(w.reopen);
    w.reopen = window.setTimeout(() => this.openWindow(w), 6000);
    this.run();
  }

  openWindow(w) {
    window.clearTimeout(w.reopen);
    if (w.open) return;
    w.open = true;
    w.el.classList.remove("win--closed");
    w.icon?.classList.add("tb--open");
    this.raise(w);
  }

  /** Đóng một cửa sổ gần (trong 400 px): các con đang rảnh quay về phía đó nhảy cẫng lên hai cái. */
  cheer(w, now) {
    const center = w.x + w.w / 2;
    let said = false;
    for (const p of this.pets) {
      if (p.on === null || !["idle", "walk", "run", "perch", "land"].includes(p.state)) continue;
      if (Math.abs(p.x - center) > 400) continue;
      p.dir = center >= p.x ? 1 : -1;
      p.hops = 1;
      this.hop(p, now, 280);
      if (!said) {
        p.bubble.say(this.text.desk.cheer, now, 2);
        said = true;
      }
    }
  }

  // ---------------------------------------------------------------- pet

  /** Thả con `id` xuống cảnh (bảng nhân vật); đủ ba con thì con cũ nhất đi. Có rồi thì nó nhảy lên chào. */
  drop(id) {
    const now = performance.now();
    const found = this.pets.find((p) => p.def.id === id);
    if (found) {
      this.poke(found, now);
      found.bubble.say(pick(greetings()), now);
      return;
    }
    if (this.pets.length >= MAX_PETS) this.remove(this.pets[0]);
    const p = this.add(id, between(80, this.width - 80), 40, null);
    if (p) this.fall(p, now);
  }

  add(id, x, y, on) {
    const def = this.defs.get(id);
    if (!def) return null;
    preload(def);
    const sprite = new Sprite(def, SCALE, "desk-pet");
    sprite.el.tabIndex = 0;
    sprite.el.setAttribute("role", "button");
    sprite.el.setAttribute("aria-label", fill(this.text.desk.petLabel, { name: def.name }));
    this.layer.append(sprite.el);
    const p = {
      def,
      sprite,
      bubble: new Bubble(this.layer),
      x,
      y,
      dir: Math.random() < 0.5 ? 1 : -1,
      on,
      state: "idle",
      until: 0,
      target: x,
      goal: null,
      vx: 0,
      vy: 0,
      peak: y,
      dropped: false,
      hops: 0,
      ignore: null,
      ignoreUntil: 0,
      climbWin: null,
      climbSide: 0,
      climbDir: 0,
      press: null,
      zzz: null,
      stars: null,
    };
    sprite.play("idle", performance.now());
    this.bindPet(p);
    this.pets.push(p);
    this.onChange(this.pets.map((pet) => pet.def.id));
    this.run();
    return p;
  }

  remove(p) {
    p.sprite.el.remove();
    p.bubble.remove();
    this.decor(p, null);
    this.pets = this.pets.filter((pet) => pet !== p);
    this.onChange(this.pets.map((pet) => pet.def.id));
  }

  /** Ba con đầu tiên: một con dưới taskbar sắp leo cửa sổ ghi chú, một con trên cửa sổ nhạc, một con đang ngủ. */
  populate(ids) {
    const now = performance.now();
    const [notes, music] = this.windows;
    const [first, second, third] = ids;
    const climber = this.add(first, Math.min(notes.x + notes.w + 150, this.width - 60), this.ground, "ground");
    if (climber) {
      climber.dir = -1;
      this.idle(climber, now, 1.2, 1.2);
      if (!this.reduced) {
        climber.goal = { type: "climb", win: notes, side: 1, footX: notes.x + notes.w + climber.def.reach * SCALE };
      }
    }
    const percher = this.add(second, music.x + music.w * 0.62, music.y, music);
    if (percher) {
      percher.dir = -1;
      this.idle(percher, now, 2, 3);
      window.setTimeout(() => percher.bubble.say(this.text.desk.invite, performance.now(), 4), 1400);
    }
    const sleeper = this.add(third, this.width - 80, this.ground, "ground");
    if (sleeper) this.sleep(sleeper, now, 22000);
  }

  bindPet(p) {
    const el = p.sprite.el;
    el.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) return;
      event.preventDefault();
      el.setPointerCapture(event.pointerId);
      const point = this.toWorld(event);
      const now = performance.now();
      p.press = { id: event.pointerId, x: point.x, y: point.y, dx: p.x - point.x, dy: p.y - point.y, dragging: false, samples: [{ t: now, ...point }] };
    });
    el.addEventListener("pointermove", (event) => {
      const press = p.press;
      if (!press || press.id !== event.pointerId) return;
      const point = this.toWorld(event);
      const now = performance.now();
      if (!press.dragging) {
        if (Math.hypot(point.x - press.x, point.y - press.y) < 5) return;
        press.dragging = true;
        this.grab(p, now);
      }
      p.x = clamp(point.x + press.dx, 24, this.width - 24);
      p.y = clamp(point.y + press.dy, p.sprite.height * 0.85, this.ground);
      press.samples.push({ t: now, ...point });
      while (press.samples.length > 2 && now - press.samples[0].t > 90) press.samples.shift();
      this.run();
    });
    const release = (event, cancelled) => {
      const press = p.press;
      if (!press || press.id !== event.pointerId) return;
      p.press = null;
      const now = performance.now();
      if (!press.dragging) {
        if (!cancelled) this.poke(p, now);
        return;
      }
      const first = press.samples[0];
      const last = press.samples[press.samples.length - 1];
      const dt = Math.max(16, last.t - first.t) / 1000;
      let vx = cancelled ? 0 : (last.x - first.x) / dt;
      let vy = cancelled ? 0 : (last.y - first.y) / dt;
      const speed = Math.hypot(vx, vy);
      if (speed > MAX_THROW) {
        vx *= MAX_THROW / speed;
        vy *= MAX_THROW / speed;
      }
      this.fall(p, now);
      p.vx = vx;
      p.vy = vy;
      p.dropped = true;
    };
    el.addEventListener("pointerup", (event) => release(event, false));
    el.addEventListener("pointercancel", (event) => release(event, true));
    el.addEventListener("dblclick", () => this.tellTime(p));
    el.addEventListener("keydown", (event) => {
      if (event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        this.poke(p, performance.now());
      } else if (event.key.toLowerCase() === "t") this.tellTime(p);
    });
  }

  /** Bấm đúp: giờ, thứ, ngày, ngày âm lịch (câu của app). */
  tellTime(p) {
    const date = new Date();
    p.bubble.say(nowText(date, toLunar(date), null), performance.now(), 5);
    this.run();
  }

  /** Bấm: đang ngủ thì dậy, đang leo thì tuột tay, còn lại nhảy lên một cái. Đang bay thì thôi. */
  poke(p, now) {
    if (p.state === "air" || p.state === "dragged") return;
    if (p.state === "climb") return this.fall(p, now);
    // Đang ngồi đúng mép: nhích vào trong một chút để nhảy lên rồi đáp lại chỗ cũ.
    if (p.state === "perch") p.x -= p.dir * 6;
    if (Math.random() < 0.3) p.bubble.say(pick(greetings()), now);
    this.hop(p, now, 300);
  }

  grab(p, now) {
    this.decor(p, null);
    p.state = "dragged";
    p.on = null;
    p.goal = null;
    p.hops = 0;
    p.sprite.play("dragged", now);
  }

  setState(p, state, animation, now, restart = false) {
    p.state = state;
    p.sprite.play(animation, now, restart);
    if (state !== "sleep" && state !== "dizzy") this.decor(p, null);
  }

  idle(p, now, min = 1.5, max = 4.5) {
    this.setState(p, "idle", "idle", now);
    p.until = now + between(min, max) * 1000;
  }

  sleep(p, now, ms) {
    this.setState(p, "sleep", "sleep", now);
    p.until = now + ms;
    this.decor(p, "zzz");
  }

  land(p, now) {
    this.setState(p, "land", "land", now, true);
    const a = p.def.animations.land;
    p.until = now + (a.frames / a.fps) * 1000;
  }

  dizzy(p, now) {
    this.setState(p, "dizzy", "dizzy", now, true);
    p.until = now + 2200;
    this.decor(p, "stars");
    if (Math.random() < 0.6) p.bubble.say(this.text.desk.dizzy, now, 2.2);
  }

  perch(p, side, now) {
    const w = p.on;
    p.dir = side;
    p.x = side < 0 ? w.x : w.x + w.w;
    this.setState(p, "perch", "perch", now);
    p.until = now + between(4000, 9000);
  }

  walkTo(p, x, now, run = false, goal = null) {
    p.target = x;
    p.goal = goal;
    if (Math.abs(x - p.x) < 1) return this.arrive(p, now);
    p.dir = x > p.x ? 1 : -1;
    this.setState(p, run ? "run" : "walk", run ? "run" : "walk", now);
  }

  /** Bay lên (rơi, nhảy, ném): `vx`, `vy` đặt sau. */
  fall(p, now) {
    this.decor(p, null);
    p.state = "air";
    p.on = null;
    p.goal = null;
    p.vx = 0;
    p.vy = 0;
    p.peak = p.y;
    p.dropped = false;
    p.air = "fall";
    p.sprite.play("fall", now);
  }

  hop(p, now, power) {
    this.decor(p, null);
    p.state = "air";
    p.on = null;
    p.goal = null;
    p.vx = 0;
    p.vy = -power;
    p.peak = p.y;
    p.dropped = false;
    p.air = "react";
    p.ignore = null;
    p.sprite.play("react", now, true);
  }

  /** Nhảy theo đường parabol tới (`tx`, `ty`). */
  jumpTo(p, tx, ty, now) {
    const dx = tx - p.x;
    const dy = ty - p.y;
    const t = clamp(0.45 + Math.abs(dx) / 600 + Math.max(0, -dy) / 500, 0.5, 0.95);
    p.ignore = p.on;
    p.ignoreUntil = now + 180;
    this.decor(p, null);
    p.state = "air";
    p.on = null;
    p.goal = null;
    p.vx = dx / t;
    p.vy = dy / t - 0.5 * GRAVITY * t;
    p.peak = p.y;
    p.dropped = false;
    p.dir = dx >= 0 ? 1 : -1;
    p.air = "jump";
    p.sprite.play("jump", now, true);
  }

  surfaceOf(on) {
    return on === "ground" ? { x0: 0, x1: this.width, y: this.ground } : { x0: on.x, x1: on.x + on.w, y: on.y };
  }

  /** Mặt có thể đáp xuống, từ trên xuống dưới. */
  surfaces() {
    const list = this.windows.filter((w) => w.open).map((w) => ({ on: w, x0: w.x + 2, x1: w.x + w.w - 2, y: w.y }));
    list.sort((a, b) => a.y - b.y);
    list.push({ on: "ground", x0: -Infinity, x1: Infinity, y: this.ground });
    return list;
  }

  /** Cạnh cửa sổ đứng trên taskbar mà pet dưới đất leo được, gần nhất trong 260 px. */
  wallNear(p) {
    const reach = p.def.reach * SCALE;
    let best = null;
    for (const w of this.windows) {
      if (!w.open || w.y + w.h < this.ground - 2 || w.y < ROOM_ABOVE) continue;
      for (const side of [-1, 1]) {
        const footX = side < 0 ? w.x - reach : w.x + w.w + reach;
        if (footX < 24 || footX > this.width - 24) continue;
        const distance = Math.abs(footX - p.x);
        if (distance < 260 && (!best || distance < best.distance)) best = { type: "climb", win: w, side, footX, distance };
      }
    }
    return best;
  }

  /** Đứng yên xong: chọn việc tiếp theo. */
  decide(p, now) {
    if (this.reduced) return this.idle(p, now, 3, 6);
    if (p.goal) {
      const goal = p.goal;
      p.goal = null;
      return this.walkTo(p, goal.footX, now, false, goal);
    }
    const r = Math.random();
    if (p.on === "ground") {
      const wall = this.wallNear(p);
      if (wall && r < 0.3) return this.walkTo(p, wall.footX, now, Math.random() < 0.3, wall);
      if (r < 0.34 && !this.pets.some((other) => other.state === "sleep")) return this.sleep(p, now, between(14000, 24000));
      const x = clamp(p.x + (Math.random() < 0.5 ? -1 : 1) * between(60, 260), 30, this.width - 30);
      if (r < 0.78) return this.walkTo(p, x, now);
      if (r < 0.9) return this.walkTo(p, x, now, true);
      return this.idle(p, now);
    }
    const s = this.surfaceOf(p.on);
    if (r < 0.45) {
      const side = Math.random() < 0.5 ? -1 : 1;
      return this.walkTo(p, side < 0 ? s.x0 + 3 : s.x1 - 3, now, false, { type: "edge", side });
    }
    if (r < 0.8) return this.walkTo(p, between(s.x0 + 14, s.x1 - 14), now);
    return this.idle(p, now);
  }

  arrive(p, now) {
    const goal = p.goal;
    p.goal = null;
    if (goal?.type === "climb") return this.startClimb(p, goal, now);
    if (goal?.type === "edge" && p.on !== "ground" && p.on) return this.atEdge(p, goal.side, now);
    this.idle(p, now);
  }

  startClimb(p, goal, now) {
    const w = goal.win;
    const reach = p.def.reach * SCALE;
    if (!w.open || w.y + w.h < this.ground - 2 || w.y < ROOM_ABOVE) return this.idle(p, now);
    const footX = goal.side < 0 ? w.x - reach : w.x + w.w + reach;
    // Cửa sổ vừa bị dời: đi lại tới chân tường mới.
    if (Math.abs(footX - p.x) > 6) return this.walkTo(p, footX, now, false, { ...goal, footX });
    p.x = footX;
    p.on = null;
    p.climbWin = w;
    p.climbSide = goal.side;
    p.climbDir = -1;
    p.dir = goal.side < 0 ? 1 : -1;
    this.setState(p, "climb", "climb", now);
  }

  /** Tới đầu mép cửa sổ: ngồi mép, nhảy đi, leo xuống, hoặc quay lại. */
  atEdge(p, side, now) {
    const w = p.on;
    const r = Math.random();
    const reach = p.def.reach * SCALE;
    const downX = side < 0 ? w.x - reach : w.x + w.w + reach;
    const canClimbDown = w.y + w.h >= this.ground - 2 && downX > 24 && downX < this.width - 24;
    if (r < 0.4) return this.perch(p, side, now);
    if (r < 0.75) return this.jumpFrom(p, side, now);
    if (r < 0.9 && canClimbDown) {
      p.on = null;
      p.x = downX;
      p.climbWin = w;
      p.climbSide = side;
      p.climbDir = 1;
      p.dir = -side;
      return this.setState(p, "climb", "climb", now);
    }
    const s = this.surfaceOf(w);
    this.walkTo(p, between(s.x0 + 14, s.x1 - 14), now);
  }

  /** Nhảy từ mép phía `side`: sang cửa sổ khác gần đó nếu có, không thì xuống taskbar. */
  jumpFrom(p, side, now) {
    const from = this.surfaceOf(p.on);
    let target = null;
    for (const w of this.windows) {
      if (!w.open || w === p.on || w.y < ROOM_ABOVE || w.y < p.y - 130) continue;
      const gap = side > 0 ? w.x - from.x1 : from.x0 - (w.x + w.w);
      if (gap > 10 && gap < 260) target = { x: side > 0 ? w.x + 24 : w.x + w.w - 24, y: w.y };
    }
    if (!target) {
      const edge = side > 0 ? from.x1 : from.x0;
      target = { x: clamp(edge + side * between(50, 140), 30, this.width - 30), y: this.ground };
    }
    this.jumpTo(p, target.x, target.y, now);
  }

  /** Chữ z lúc ngủ, sao quanh đầu lúc choáng (`kind`), hoặc bỏ hết (`null`). */
  decor(p, kind) {
    if (kind !== "zzz" && p.zzz) {
      p.zzz.remove();
      p.zzz = null;
    }
    if (kind !== "stars" && p.stars) {
      p.stars.remove();
      p.stars = null;
    }
    if (kind === "zzz" && !p.zzz) {
      p.zzz = document.createElement("div");
      p.zzz.className = "zzz";
      p.zzz.innerHTML = "<span>z</span><span>z</span><span>z</span>";
      this.layer.append(p.zzz);
    }
    if (kind === "stars" && !p.stars) {
      // Lớp ngoài đặt chỗ, lớp trong quay (animation CSS dùng transform của lớp trong).
      p.stars = document.createElement("div");
      p.stars.className = "stars";
      p.stars.innerHTML =
        '<div class="stars-orbit"><span style="left:12px;top:-8px">✦</span><span style="left:-20px;top:-6px">✧</span><span style="left:-4px;top:8px">✦</span></div>';
      this.layer.append(p.stars);
    }
  }

  // ---------------------------------------------------------------- vòng lặp

  run() {
    if (this.running || !this.visible) return;
    this.running = true;
    this.last = performance.now();
    requestAnimationFrame((t) => this.frame(t));
  }

  frame(now) {
    if (!this.visible) {
      this.running = false;
      return;
    }
    const dt = Math.min(0.05, (now - this.last) / 1000);
    this.last = now;
    for (const p of this.pets) this.step(p, dt, now);
    this.meet(now);
    this.draw(now);
    requestAnimationFrame((t) => this.frame(t));
  }

  step(p, dt, now) {
    switch (p.state) {
      case "dragged":
        return;
      case "air":
        return this.fly(p, dt, now);
      case "walk":
      case "run": {
        const speed = p.state === "run" ? SPEED.run : SPEED.walk;
        const dx = p.target - p.x;
        p.x += Math.sign(dx) * Math.min(Math.abs(dx), speed * dt);
        if (Math.abs(p.target - p.x) < 0.5) this.arrive(p, now);
        return;
      }
      case "climb":
        return this.climb(p, dt, now);
      default:
        if (now < p.until) return;
        if (p.state === "perch") {
          const s = this.surfaceOf(p.on);
          if (!this.reduced && Math.random() < 0.5) return this.jumpFrom(p, p.dir, now);
          return this.walkTo(p, between(s.x0 + 14, s.x1 - 14), now);
        }
        if (p.state === "land" || p.state === "dizzy" || p.state === "sleep") return this.idle(p, now, 0.4, 1.2);
        return this.decide(p, now);
    }
  }

  climb(p, dt, now) {
    const w = p.climbWin;
    if (!w.open) return this.fall(p, now);
    const reach = p.def.reach * SCALE;
    p.x = p.climbSide < 0 ? w.x - reach : w.x + w.w + reach;
    p.y += p.climbDir * SPEED.climb * dt;
    // Cửa sổ bị kéo lên khỏi chỗ đang bám: tuột tay.
    if (p.y > w.y + w.h + 4 && p.climbDir < 0) return this.fall(p, now);
    if (p.climbDir < 0 && p.y <= w.y) {
      p.y = w.y;
      p.on = w;
      p.x = p.climbSide < 0 ? w.x + 14 : w.x + w.w - 14;
      p.dir = p.climbSide < 0 ? 1 : -1;
      return this.land(p, now);
    }
    if (p.climbDir > 0 && p.y >= this.ground) {
      p.y = this.ground;
      p.on = "ground";
      p.dir = p.climbSide;
      return this.land(p, now);
    }
  }

  fly(p, dt, now) {
    const before = p.y;
    p.vy += GRAVITY * dt;
    p.x += p.vx * dt;
    p.y += p.vy * dt;
    if (p.x < 24) {
      p.x = 24;
      p.vx = Math.abs(p.vx) * 0.5;
      p.dir = 1;
    } else if (p.x > this.width - 24) {
      p.x = this.width - 24;
      p.vx = -Math.abs(p.vx) * 0.5;
      p.dir = -1;
    }
    const ceiling = p.sprite.height * 0.85;
    if (p.y < ceiling) {
      p.y = ceiling;
      p.vy = Math.max(0, p.vy);
    }
    p.peak = Math.min(p.peak, p.y);
    if (p.air === "jump" && p.vy > 0 && p.sprite.done(now)) {
      p.air = "fall";
      p.sprite.play("fall", now);
    }
    if (p.vy <= 0) return;
    for (const s of this.surfaces()) {
      if (s.on === p.ignore && now < p.ignoreUntil) continue;
      if (before <= s.y + 0.01 && p.y >= s.y && p.x >= s.x0 && p.x <= s.x1) {
        p.y = s.y;
        p.on = s.on;
        return this.touchDown(p, now);
      }
    }
  }

  touchDown(p, now) {
    const height = p.y - p.peak;
    p.vx = 0;
    p.vy = 0;
    p.ignore = null;
    if (p.hops > 0) {
      p.hops--;
      return this.hop(p, now, 280);
    }
    const dizzy = p.dropped && height > DIZZY_FROM;
    p.dropped = false;
    if (dizzy) return this.dizzy(p, now);
    this.land(p, now);
  }

  /** Hai con gặp nhau trên cùng một mặt: dừng lại quay mặt vào nhau, có lúc chào một câu. */
  meet(now) {
    for (let i = 0; i < this.pets.length; i++) {
      for (let j = i + 1; j < this.pets.length; j++) {
        const a = this.pets[i];
        const b = this.pets[j];
        if (a.on === null || a.on !== b.on || Math.abs(a.x - b.x) > 40) continue;
        if (![a, b].every((p) => p.state === "idle" || p.state === "walk")) continue;
        const key = `${a.def.id}/${b.def.id}`;
        if ((this.meetings.get(key) ?? 0) > now) continue;
        this.meetings.set(key, now + 15000);
        a.dir = b.x >= a.x ? 1 : -1;
        b.dir = -a.dir;
        this.idle(a, now, 2.4, 2.8);
        this.idle(b, now, 2.4, 2.8);
        a.goal = null;
        b.goal = null;
        if (Math.random() < 0.6) pick([a, b]).bubble.say(pick(greetings()), now);
      }
    }
  }

  draw(now) {
    for (const p of this.pets) {
      p.sprite.update(now);
      const tilt = p.state === "dizzy" ? Math.sin(now / 110) * 9 : 0;
      p.sprite.place(p.x, p.y, p.dir, tilt);
      const body = p.sprite.body(p.x, p.y, p.dir);
      p.bubble.place(now, p.x, body.y, this.width);
      if (p.zzz) p.zzz.style.transform = `translate(${p.x + p.dir * body.width * 0.25}px, ${body.y + 4}px)`;
      if (p.stars) p.stars.style.transform = `translate(${p.x}px, ${body.y - 2}px)`;
    }
  }

  // ---------------------------------------------------------------- khác

  toWorld(event) {
    const rect = this.world.getBoundingClientRect();
    return { x: (event.clientX - rect.left) / this.scale, y: (event.clientY - rect.top) / this.scale };
  }

  /** Đồng hồ trên taskbar theo giờ máy người xem. */
  startClock() {
    const time = this.root.querySelector("[data-clock-time]");
    const date = this.root.querySelector("[data-clock-date]");
    const locale = document.documentElement.lang === "vi" ? "vi-VN" : "en-US";
    const tick = () => {
      const now = new Date();
      time.textContent = now.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
      date.textContent = now.toLocaleDateString(locale);
    };
    tick();
    window.setInterval(tick, 20_000);
  }
}
