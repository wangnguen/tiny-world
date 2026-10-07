// Demo thời tiết: một con đứng giữa cảnh nhỏ, bấm nút là đổi thời tiết, nóng lạnh. Hiệu ứng là đúng code của
// app (gen/weather.js từ desktop/src/overlay/weather.ts), câu pet nói cũng của app (gen/lines.js).
import { WeatherEffect } from "./gen/weather.js";
import { skyLine, warmthLine } from "./gen/lines.js";
import { skyIconSvg } from "./gen/skyIcons.js";
import { Bubble, Sprite, preload } from "./sprite.js";

/** Pet trong demo to gần gấp đôi cỡ mặc định, để hiệu ứng quanh nó (theo cỡ thân) dễ nhìn. */
const SCALE = 0.95;
/** Cao của dải cỏ dưới cảnh (CSS px), khớp `.demo__ground`. */
const GROUND = 44;
const WALK = 26;
/** Nhiệt độ mỗi kiểu thời tiết khi không bấm Nóng/Lạnh (°C); từ 33°C là nóng, từ 15°C trở xuống là lạnh như app. */
const TEMPERATURES = { sunny: 31, clear: 24, cloudy: 27, rain: 24, storm: 23, snow: -1, fog: 17, petals: 22 };
const HOT_FROM = 33;
const COLD_UNTIL = 15;
const THUNDER_GAP = [3500, 7000];

const between = (min, max) => min + Math.random() * (max - min);
const warmthOf = (t) => (t >= HOT_FROM ? "hot" : t <= COLD_UNTIL ? "cold" : null);

export class WeatherDemo {
  constructor(root, def) {
    this.stage = root.querySelector("[data-demo-stage]");
    this.reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    this.effect = new WeatherEffect();
    this.tag = document.createElement("div");
    this.tag.className = "temperature";
    this.bubble = new Bubble(this.stage);
    this.sky = "rain";
    this.override = null;
    this.state = "idle";
    this.until = 0;
    this.nextThunder = 0;
    this.visible = false;
    this.running = false;
    this.setPet(def, false);
    this.stage.append(this.effect.element, this.tag);

    this.skyChips = [...document.querySelectorAll("[data-sky-chips] [data-sky]")];
    this.warmthChips = [...document.querySelectorAll("[data-warmth-chips] [data-warmth]")];
    for (const chip of this.skyChips) chip.addEventListener("click", () => this.setSky(chip.dataset.sky));
    for (const chip of this.warmthChips) {
      chip.addEventListener("click", () => {
        const temperature = Number(chip.dataset.temperature);
        this.setOverride(this.override === temperature ? null : temperature);
      });
    }
    this.apply(false);
    // Vẽ sẵn một khung để lúc cuộn tới không thấy pet nằm ở góc.
    this.draw(performance.now());
    new IntersectionObserver(([entry]) => {
      this.visible = entry.isIntersecting;
      if (this.visible) this.run();
    }).observe(this.stage);
  }

  /** Đổi con đứng trong demo (bấm một nhân vật trong bảng). */
  setPet(def, speak = true) {
    preload(def);
    this.sprite?.el.remove();
    this.sprite = new Sprite(def, SCALE, "demo-pet");
    this.stage.insertBefore(this.sprite.el, this.effect.element.parentNode ? this.effect.element : null);
    this.x = (this.stage.clientWidth || 420) / 2;
    this.dir = 1;
    this.target = this.x;
    this.state = "idle";
    this.sprite.play("idle", performance.now());
    if (speak) this.say();
    this.run();
  }

  get temperature() {
    return this.override ?? TEMPERATURES[this.sky];
  }

  setSky(sky) {
    this.sky = sky;
    this.apply(true);
  }

  setOverride(temperature) {
    this.override = temperature;
    this.apply(true);
  }

  apply(speak) {
    const warmth = warmthOf(this.temperature);
    this.stage.dataset.sky = this.sky;
    this.effect.setSky(this.sky);
    this.effect.setWarmth(warmth);
    for (const chip of this.skyChips) chip.setAttribute("aria-pressed", String(chip.dataset.sky === this.sky));
    for (const chip of this.warmthChips) {
      chip.setAttribute("aria-pressed", String(Number(chip.dataset.temperature) === this.override));
    }
    this.tag.innerHTML = `${skyIconSvg(this.sky)}<span>${Math.round(this.temperature)}°C</span>`;
    if (warmth) this.tag.dataset.warmth = warmth;
    else delete this.tag.dataset.warmth;
    this.nextThunder = 0;
    if (speak) this.say();
    this.run();
  }

  /** Câu của app khi trời đổi, ví dụ "Mưa rồi, 24°C :(". */
  say() {
    const warmth = warmthOf(this.temperature);
    const line =
      this.override !== null && warmth ? warmthLine(warmth, this.temperature) : skyLine(this.sky, this.temperature, warmth);
    this.bubble.say(line, performance.now(), 4);
  }

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
    this.step(dt, now);
    this.draw(now);
    requestAnimationFrame((t) => this.frame(t));
  }

  /** Đứng một lúc, đi vài bước qua lại; có sấm thì giật mình. */
  step(dt, now) {
    const width = this.stage.clientWidth;
    if (this.sky === "storm" && !this.reduced) {
      if (this.nextThunder === 0) this.nextThunder = now + 1200;
      else if (now >= this.nextThunder) {
        this.nextThunder = now + between(...THUNDER_GAP);
        this.effect.flash(now);
        this.stage.classList.remove("demo__stage--flash");
        void this.stage.offsetWidth;
        this.stage.classList.add("demo__stage--flash");
        this.state = "react";
        this.sprite.play("react", now, true);
      }
    }
    if (this.state === "react") {
      if (this.sprite.done(now)) {
        this.state = "idle";
        this.until = now + between(800, 1600);
        this.sprite.play("idle", now);
      }
      return;
    }
    if (this.state === "walk") {
      const dx = this.target - this.x;
      this.x += Math.sign(dx) * Math.min(Math.abs(dx), WALK * dt);
      if (Math.abs(this.target - this.x) < 0.5) {
        this.state = "idle";
        this.until = now + between(2500, 5000);
        this.sprite.play("idle", now);
      }
      return;
    }
    if (now < this.until || this.reduced) return;
    this.target = between(width * 0.3, width * 0.7);
    if (Math.abs(this.target - this.x) < 20) {
      this.until = now + 1500;
      return;
    }
    this.dir = this.target > this.x ? 1 : -1;
    this.state = "walk";
    this.sprite.play("walk", now);
  }

  draw(now) {
    const width = this.stage.clientWidth;
    const y = this.stage.clientHeight - GROUND + 2;
    this.x = Math.min(Math.max(this.x, 60), width - 60);
    this.sprite.update(now);
    this.sprite.place(this.x, y, this.dir);
    const body = this.sprite.body(this.x, y, this.dir);
    const effort = this.state === "walk" ? 0.5 : 0;
    this.effect.update(body, now, true, this.sprite.mouth(this.x, y, this.dir), effort);
    // Nhãn nhiệt độ đứng sau lưng pet, như trong app.
    const tagWidth = this.tag.offsetWidth;
    const tagX = this.dir > 0 ? body.x - tagWidth - 6 : body.x + body.width + 6;
    this.tag.style.transform = `translate(${Math.round(tagX)}px, ${Math.round(body.y + 6)}px)`;
    this.bubble.place(now, this.x, body.y, width);
  }
}
