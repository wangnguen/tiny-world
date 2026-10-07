// Sprite của một con pet: một <div> có ảnh nền là dải frame của animation đang chạy, đổi frame bằng
// background-position. Dải frame do scripts/prepare-website.mjs cắt sẵn từ sprite pack của app
// (/pets/<pack>/<animation>.webp), mô tả ở /pets/pets.json.

export const petUrl = (id, animation) => `/pets/${id}/${animation}.webp`;

/** Nạp sẵn mọi animation của một con, để đổi animation lần đầu không bị nháy trống. */
export function preload(def) {
  for (const name of Object.keys(def.animations)) {
    const image = new Image();
    image.src = petUrl(def.id, name);
  }
}

export class Sprite {
  /** `scale`: cỡ vẽ so với frame gốc (0,5 là 96 CSS px như cỡ mặc định trong app). */
  constructor(def, scale, className) {
    this.def = def;
    this.scale = scale;
    this.width = def.frame.width * scale;
    this.height = def.frame.height * scale;
    this.el = document.createElement("div");
    this.el.className = `sprite ${className}`;
    this.el.style.width = `${this.width}px`;
    this.el.style.height = `${this.height}px`;
    // Xoay, lật quanh điểm chân để đặt chân đúng chỗ.
    this.el.style.transformOrigin = `${def.anchor.x * scale}px ${def.anchor.y * scale}px`;
    this.animation = null;
    this.started = 0;
    this.frame = -1;
  }

  /** Chạy animation `name` từ lúc `now` (ms); đang chạy đúng animation đó thì để yên, trừ khi `restart`. */
  play(name, now, restart = false) {
    if (!this.def.animations[name]) name = "idle";
    if (name === this.animation && !restart) return;
    const a = this.def.animations[name];
    this.animation = name;
    this.started = now;
    this.frame = -1;
    this.el.style.backgroundImage = `url(${petUrl(this.def.id, name)})`;
    this.el.style.backgroundSize = `${a.frames * this.width}px ${this.height}px`;
  }

  /** Animation không lặp đã chạy hết chưa. */
  done(now) {
    const a = this.def.animations[this.animation];
    return !a.loop && ((now - this.started) / 1000) * a.fps >= a.frames;
  }

  /** Vẽ frame của lúc `now`; `pace` nhanh hơn hay chậm hơn fps gốc (đi, chạy). */
  update(now, pace = 1) {
    const a = this.def.animations[this.animation];
    const count = Math.floor(((now - this.started) / 1000) * a.fps * pace);
    const frame = a.loop ? count % a.frames : Math.min(count, a.frames - 1);
    if (frame === this.frame) return;
    this.frame = frame;
    this.el.style.backgroundPosition = `${-frame * this.width}px 0`;
  }

  /** Đặt điểm chân ở (`x`, `y`), mặt quay sang `dir` (1 phải, -1 trái), nghiêng `tilt` độ. */
  place(x, y, dir, tilt = 0) {
    const { anchor } = this.def;
    const left = x - anchor.x * this.scale;
    const top = y - anchor.y * this.scale;
    const turn = tilt ? ` rotate(${tilt}deg)` : "";
    this.el.style.transform = `translate(${left}px, ${top}px) scaleX(${dir})${turn}`;
  }

  /** Khung thân (phần có hình của frame đầu idle) theo toạ độ cảnh, khi chân ở (`x`, `y`) và mặt quay `dir`. */
  body(x, y, dir) {
    const { anchor, body } = this.def;
    const s = this.scale;
    const left = dir > 0 ? body.x - anchor.x : anchor.x - (body.x + body.width);
    return { x: x + left * s, y: y - (anchor.y - body.y) * s, width: body.width * s, height: body.height * s };
  }

  /** Miệng (chỗ thở ra khói lúc lạnh) theo toạ độ cảnh. */
  mouth(x, y, dir) {
    const { anchor, mouth } = this.def;
    return { x: x + dir * (mouth.x - anchor.x) * this.scale, y: y - (anchor.y - mouth.y) * this.scale, dir };
  }
}

/** Speech bubble trên đầu một con, như trong app: tự ẩn sau `seconds` giây. */
export class Bubble {
  constructor(layer) {
    this.layer = layer;
    this.el = null;
    this.until = 0;
  }

  say(text, now, seconds = 3.2) {
    if (!this.el) {
      this.el = document.createElement("div");
      this.el.className = "bubble";
      this.layer.append(this.el);
    }
    this.el.textContent = text;
    this.until = now + seconds * 1000;
  }

  get shown() {
    return this.el !== null;
  }

  /** Đặt bubble ngay trên đầu (`headX`, `headY`), không tràn khỏi cảnh rộng `width`. */
  place(now, headX, headY, width) {
    if (!this.el) return;
    if (now > this.until) {
      this.el.remove();
      this.el = null;
      return;
    }
    const w = this.el.offsetWidth;
    const h = this.el.offsetHeight;
    const left = Math.min(Math.max(4, headX - w / 2), width - w - 4);
    // Pet ở sát mép trên: bubble không thò ra khỏi cảnh.
    const top = Math.max(4, headY - h - 10);
    this.el.style.transform = `translate(${Math.round(left)}px, ${Math.round(top)}px)`;
    this.el.style.setProperty("--tail", `${Math.min(Math.max(12, headX - left), w - 12)}px`);
  }

  remove() {
    this.el?.remove();
    this.el = null;
  }
}
