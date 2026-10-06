import {
  PREVIEW_SECONDS,
  type City,
  type Point,
  type EffectPreview,
  type Reminder,
  type Settings,
  type WeatherFailure,
  type WeatherReport,
} from "@tinyworld/core";
import {
  GHOST_LINE,
  SAVE_SPAM_LINE,
  WATER_LINE,
  bedtimeLine,
  breakLine,
  NO_NETWORK,
  NO_WEATHER,
  REPORT_MAX_AGE,
  SKY_LINES,
  activeOccasions,
  nowText,
  simulatedSky,
  skyOf,
  toLunar,
  wallClock,
  weatherLine,
  withPetals,
  type Pet,
  type PlaceWeather,
  type Sky,
  type World,
} from "@tinyworld/sim";
import { GHOST_SIZE, Ghost } from "./ghost";
import type { PetView } from "./petView";
import { EFFECT_FPS, type WeatherEffect } from "./weather";
import type { AuraEffect } from "./aura";

/** Chưa có thời tiết thật thì ban đêm theo giờ ở thành phố đã chọn: từ 19 giờ tới trước 6 giờ sáng. */
const NIGHT_FROM = 19;
const NIGHT_UNTIL = 6;
/** Hai tiếng sấm cách nhau ngẫu nhiên trong khoảng này (ms). */
const THUNDER_GAP: [number, number] = [15_000, 45_000];
/** Xem thử giông: tiếng sấm đầu tiên sau chừng này ms, các tiếng sau cách nhau chừng này (ms). */
const PREVIEW_FIRST_THUNDER_MS = 1_500;
const PREVIEW_THUNDER_GAP: [number, number] = [5_000, 9_000];
/** Con ma chỉ bay trong khoảng 2:00–2:30 (giờ ở thành phố đã chọn), mỗi đêm một lần. */
const GHOST_HOUR = 2;
const GHOST_MINUTES = 30;
/** Con ma sà xuống ngang tầm chừng này lần chiều cao pet tính từ chân (bụng con ma ở đó). */
const GHOST_HEIGHT = 0.6;
/** Bấm đúp: câu giờ, ngày, thời tiết hiện chừng này giây. */
const INFO_SECONDS = 6;
/** Câu báo (lỗi thời tiết, câu của dịp lễ) hiện chừng này giây. */
const NOTICE_SECONDS = 5;
/** Nhắc nghỉ, nhắc khuya hiện chừng này giây, bấm vào pet thì tắt sớm. */
const REMINDER_SECONDS = 30;
/** Tính lại thời tiết, ban đêm, dịp lễ, con ma mỗi chừng này ms. */
const REFRESH_MS = 60_000;
/** Những câu đã nói hôm nay (câu của dịp lễ, con ma), để mở lại app không nói lại. */
const STORAGE_KEY = "tinyworld.said";

/** Một con trên màn hình, kèm hiệu ứng thời tiết đi theo nó. */
export interface Resident {
  pet: Pet;
  view: PetView;
  effect: WeatherEffect;
  /** Aura sau lưng, chỉ Long có. */
  aura: AuraEffect | null;
}

export interface AmbienceHost {
  world: World;
  residents(): readonly Resident[];
  /** Overlay đang hiện và không tạm dừng. */
  live(): boolean;
  /** Chạy lại vòng lặp vẽ. */
  wake(): void;
  /** Người dùng đang ngồi máy (vừa có phím hay chuột ở app bất kỳ). */
  present(): Promise<boolean>;
}

/** Câu bắt buộc phải nói (không qua giới hạn câu nói cho vui); `key`: đánh dấu đã nói hôm nay. */
interface Notice {
  text: string;
  day?: string;
  key?: string;
  /** Hiện chừng này giây (mặc định `NOTICE_SECONDS`). */
  seconds?: number;
  /** Lời nhắc: con nói nhảy lên một cái cho dễ thấy, cả nhóm đang ngủ thì đánh thức một con dậy nói. */
  urgent?: boolean;
}

/**
 * Những thứ quanh nhóm pet theo giờ ở thành phố đã chọn: thời tiết (thật hoặc giả lập), ban đêm, câu của
 * dịp lễ, con ma lúc 2 giờ sáng, câu giờ/ngày/thời tiết khi bấm đúp.
 */
export class Ambience {
  private settings: Settings;
  private report: WeatherReport | null = null;
  private failure: WeatherFailure | null = null;
  /** `undefined`: chưa tính lần nào, lần đầu không nói câu thời tiết. */
  private sky: Sky | null | undefined = undefined;
  /** Câu thời tiết đang chờ tới lượt nói cho vui (`World.chat`). */
  private skyLine: string | null = null;
  private notices: Notice[] = [];
  /** Khoá (`Notice.key`) đã xếp hàng, để mỗi phút tính lại không xếp thêm lần nữa. */
  private queued = new Set<string>();
  private readonly said = new DailyMemory();
  private nextThunder = 0;
  private readonly ghost: Ghost;
  /** Các con đã bị con ma đang bay làm giật mình. */
  private scared = new Set<Pet>();
  /** Đang hỏi Rust người dùng có ngồi máy không, để gọi con ma. */
  private ghostPending = false;
  /** Thời tiết đang xem thử (mục Xem thử trong Cài đặt), thay thời tiết thật; `null` là không xem thử. */
  private previewSky: Sky | null = null;
  private previewTimer = 0;
  /** Vừa chọn thành phố mới: lấy được thời tiết ở đó thì một con báo luôn, để biết là đã chạy. */
  private announce = false;

  constructor(
    private readonly host: AmbienceHost,
    container: HTMLElement,
    settings: Settings,
  ) {
    this.settings = settings;
    this.ghost = new Ghost(container);
    host.world.chatter = settings.chatter;
    this.refresh();
    window.setInterval(() => this.refresh(), REFRESH_MS);
  }

  /** Con ma đang bay: vòng lặp phải chạy tiếp kể cả khi cả nhóm đã ngủ. */
  get flying(): boolean {
    return this.ghost.flying;
  }

  /** Vòng lặp phải vẽ lại trong vòng chừng này ms (hiệu ứng thời tiết, con ma); `Infinity` nếu không cần. */
  get frameMs(): number {
    if (this.ghost.flying) return 0;
    const animating = this.host.residents().some((r) => r.effect.animating || r.aura?.animating);
    return animating ? 1000 / EFFECT_FPS : Number.POSITIVE_INFINITY;
  }

  setSettings(next: Settings): void {
    // Đổi thành phố: lỗi của thành phố cũ không còn đúng, thời tiết mới tới sau qua `setReport`.
    if (!sameCity(next.city, this.settings.city)) {
      this.failure = null;
      this.announce = next.city !== null;
    }
    this.settings = next;
    this.host.world.chatter = next.chatter;
    this.refresh();
  }

  /** Thời tiết mới của thành phố đang chọn (`null`: chưa có). */
  setReport(report: WeatherReport | null): void {
    this.report = report;
    if (report) this.failure = null;
    this.refresh();
    const now = new Date();
    if (this.announce && this.freshReport(now)) {
      this.announce = false;
      const weather = this.placeWeather(wallClock(now, this.settings.city?.timezone));
      if (weather) this.notices.push({ text: weatherLine(weather) });
      this.host.wake();
    }
  }

  /** Lấy thời tiết bị lỗi (Rust chỉ báo ở lần lỗi đầu của mỗi đợt): một con nói là không có mạng hay không có thời tiết. */
  fail(failure: WeatherFailure): void {
    this.failure = failure;
    this.announce = false;
    if (!this.settings.city) return;
    this.notices.push({ text: failure.offline ? NO_NETWORK : NO_WEATHER });
    this.refresh();
    this.host.wake();
  }

  /** Con mới hiện ra (đổi nhân vật): theo thời tiết như cả nhóm. */
  adopt(resident: Resident): void {
    resident.effect.setSky(this.sky ?? null);
  }

  /** Bấm đúp vào `pet`: nói giờ, thứ, ngày dương lịch, âm lịch và thời tiết ở thành phố đã chọn. */
  tellTime(pet: Pet): void {
    const wall = wallClock(new Date(), this.settings.city?.timezone);
    pet.say(nowText(wall, toLunar(wall), this.placeWeather(wall)), INFO_SECONDS);
  }

  /** Mỗi lần vẽ: đặt hiệu ứng thời tiết theo pet, con ma bay tiếp. */
  place(now: number): void {
    for (const { pet, view, effect, aura } of this.host.residents()) {
      const visible = pet.state !== "sleep" && !view.spriteHidden;
      aura?.update(view.foot ?? pet, view.width, view.height, now, visible);
      // Aura của Long thay thế thời tiết quanh chính Long; pet khác vẫn theo thời tiết chung.
      effect.update(view.foot ?? pet, view.width, view.height, now, visible && !aura?.animating);
    }
  }

  /** Mỗi lượt của vòng lặp: sấm, con ma làm giật mình, câu báo đang chờ. */
  act(now: number): void {
    const residents = this.host.residents();
    if (this.sky === "storm" && residents.some((r) => r.effect.animating)) {
      const previewing = this.previewSky === "storm";
      if (this.nextThunder === 0) this.nextThunder = now + (previewing ? PREVIEW_FIRST_THUNDER_MS : gap(THUNDER_GAP));
      else if (now >= this.nextThunder) {
        this.nextThunder = now + gap(previewing ? PREVIEW_THUNDER_GAP : THUNDER_GAP);
        for (const { pet, effect } of residents) {
          effect.flash(now);
          pet.startle();
        }
      }
    } else {
      this.nextThunder = 0;
    }

    const ghost = this.ghost.update(now, ghostTargets(residents));
    if (ghost === null) this.scared.clear();
    else {
      for (const { pet, view } of residents) {
        if (this.scared.has(pet) || pet.vanished > 0) continue;
        // Con ma bay ngang qua ngay trên pet (từ ngang thân tới hơn đầu một chút) thì mới giật mình.
        if (Math.abs(pet.x - ghost.x) > view.width / 2) continue;
        if (ghost.y > pet.y || ghost.y < pet.y - view.height * 1.5 - GHOST_SIZE.height) continue;
        // Đang ngủ thì giật mình tỉnh dậy, đang thức thì nhảy dựng lên.
        if (pet.state === "sleep") pet.wake();
        else pet.startle();
        if (this.scared.size === 0) pet.say(GHOST_LINE);
        this.scared.add(pet);
      }
    }

    if (this.skyLine) {
      const speaker = pickAwake(residents);
      if (speaker && this.host.world.chat(speaker.pet, this.skyLine)) this.skyLine = null;
    }

    const notice = this.notices[0];
    if (notice && !residents.some((r) => r.pet.speech)) {
      const speaker = pickAwake(residents) ?? (notice.urgent ? (residents[0] ?? null) : null);
      if (speaker) {
        this.notices.shift();
        const { pet } = speaker;
        if (notice.urgent) {
          // Đang ngủ thì giật mình dậy, đang rảnh thì nhảy lên.
          if (pet.state === "sleep") pet.wake();
          else pet.startle();
        }
        pet.say(notice.text, notice.seconds ?? NOTICE_SECONDS);
        if (notice.day && notice.key) this.said.add(notice.day, notice.key);
      }
    }
  }

  /**
   * Mục Xem thử trong Cài đặt: thời tiết `effect` quanh pet `PREVIEW_SECONDS` giây (bỏ qua công tắc và thời
   * tiết thật; một con nói câu của thời tiết đó; giông thì sấm ngay), con ma bay qua ngay (không tính vào
   * lượt mỗi đêm), `stop` thì về thời tiết như cũ.
   */
  preview(effect: EffectPreview, now: number): void {
    if (effect === "ghost") {
      if (!this.ghost.flying && this.host.residents().length > 0) this.ghost.fly(this.host.world.bounds, now);
      this.host.wake();
      return;
    }
    window.clearTimeout(this.previewTimer);
    this.previewSky = effect === "stop" ? null : effect;
    if (this.previewSky) {
      this.previewTimer = window.setTimeout(() => {
        this.previewSky = null;
        this.refresh();
      }, PREVIEW_SECONDS * 1000);
    }
    this.refresh();
    if (this.previewSky) {
      // Câu của thời tiết đó nói luôn, không chờ tới lượt câu nói cho vui.
      this.skyLine = null;
      const line = SKY_LINES[this.previewSky];
      if (line) this.notices.push({ text: line });
      this.nextThunder = 0;
    }
    this.host.wake();
  }

  /** Rust nhắc (activity.rs): nhắc nghỉ, uống nước, nhắc khuya, spam Ctrl+S. Không qua giới hạn câu nói cho vui. */
  remind(reminder: Reminder): void {
    if (reminder.kind === "saveSpam") {
      this.notices.push({ text: SAVE_SPAM_LINE, urgent: true });
    } else {
      const now = new Date();
      const time = `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
      const text =
        reminder.kind === "break" ? breakLine(reminder.minutes) : reminder.kind === "water" ? WATER_LINE : bedtimeLine(time);
      this.notices.push({ text, seconds: REMINDER_SECONDS, urgent: true });
    }
    this.host.wake();
  }

  /**
   * Tính lại thời tiết, ban đêm, câu của dịp lễ và con ma theo giờ hiện tại; có gì đổi thì chạy lại
   * vòng lặp vẽ (cả nhóm đang ngủ mà không có gì đổi thì để yên).
   */
  private refresh(): void {
    let changed = false;
    const { world } = this.host;
    const now = new Date();
    const wall = wallClock(now, this.settings.city?.timezone);
    const report = this.freshReport(now);
    const hour = wall.getHours();
    world.night = report ? !report.isDay : hour >= NIGHT_FROM || hour < NIGHT_UNTIL;

    const sky = this.previewSky ?? this.skyAt(wall, report);
    if (sky !== this.sky) {
      const first = this.sky === undefined;
      this.sky = sky;
      for (const { effect } of this.host.residents()) effect.setSky(sky);
      this.skyLine = first || !sky ? null : (SKY_LINES[sky] ?? null);
      changed = true;
    }

    const day = dayKey(wall);
    const active = this.settings.events ? activeOccasions(this.settings.occasions, wall) : [];
    for (const occasion of active) {
      const key = `occasion:${occasion.name}`;
      if (!occasion.message || this.said.has(day, key) || this.queued.has(`${day}/${key}`)) continue;
      this.queued.add(`${day}/${key}`);
      this.notices.push({ text: occasion.message, day, key });
      changed = true;
    }

    const ghostTime = hour === GHOST_HOUR && wall.getMinutes() < GHOST_MINUTES;
    if (this.settings.ghost && ghostTime && !this.ghost.flying && !this.said.has(day, "ghost")) {
      this.summonGhost(day);
    }
    if (changed) this.host.wake();
  }

  /**
   * Con ma bay qua nếu người dùng đang ngồi máy để còn thấy, kể cả khi cả nhóm đã ngủ: 3 phút không đụng tới
   * là pet ngủ, nên chờ có con thức thì gần như không bao giờ thấy ma. Mỗi đêm một lần.
   */
  private summonGhost(day: string): void {
    if (this.ghostPending || !this.host.live()) return;
    this.ghostPending = true;
    this.host
      .present()
      .then((present) => {
        if (!present || this.ghost.flying || this.said.has(day, "ghost") || !this.host.live()) return;
        const residents = this.host.residents();
        if (residents.length === 0) return;
        this.said.add(day, "ghost");
        this.ghost.fly(this.host.world.bounds, performance.now());
        this.host.wake();
      })
      .finally(() => {
        this.ghostPending = false;
      });
  }

  /** Thời tiết quanh pet: thành phố đã chọn thì theo thời tiết thật (chưa có thì không có gì), chưa chọn thì giả lập. */
  private skyAt(wall: Date, report: WeatherReport | null): Sky | null {
    if (!this.settings.weather) return null;
    if (this.settings.city) return report ? withPetals(skyOf(report.code), wall.getMonth(), report.isDay) : null;
    return withPetals(simulatedSky(wall), wall.getMonth(), !this.host.world.night);
  }

  /** Thời tiết thật của đúng thành phố đang chọn, chưa quá cũ; `null` nếu không có. */
  private freshReport(now: Date): WeatherReport | null {
    const { report } = this;
    const city = this.settings.city;
    if (!report || !city) return null;
    if (Math.abs(report.latitude - city.latitude) > 1e-6 || Math.abs(report.longitude - city.longitude) > 1e-6) return null;
    return now.getTime() / 1000 - report.fetchedAt < REPORT_MAX_AGE ? report : null;
  }

  private placeWeather(wall: Date): PlaceWeather | null {
    const city = this.settings.city;
    if (!city) return null;
    const place = city.name.split(",")[0].trim() || city.name;
    const report = this.freshReport(new Date());
    if (!report) {
      const problem = this.failure ? (this.failure.offline ? NO_NETWORK : NO_WEATHER) : undefined;
      return { place, sky: null, problem };
    }
    return { place, sky: withPetals(skyOf(report.code), wall.getMonth(), report.isDay), temperature: report.temperature };
  }
}

function sameCity(a: City | null, b: City | null): boolean {
  return a?.latitude === b?.latitude && a?.longitude === b?.longitude;
}

/** Chỗ con ma sà qua (giữa thân con ma), mỗi pet một chỗ, ngang tầm đầu con đó. */
function ghostTargets(residents: readonly Resident[]): Point[] {
  return residents
    .filter((r) => r.pet.vanished === 0)
    .map(({ pet, view }) => ({ x: pet.x, y: pet.y - view.height * GHOST_HEIGHT - GHOST_SIZE.height / 2 }));
}

/** Một con đang thức (và đang hiện) bất kỳ, `null` nếu cả nhóm ngủ. */
function pickAwake(residents: readonly Resident[]): Resident | null {
  const awake = residents.filter((r) => r.pet.state !== "sleep" && r.pet.vanished === 0);
  return awake.length ? awake[Math.floor(Math.random() * awake.length)] : null;
}

function gap([min, max]: [number, number]): number {
  return min + Math.random() * (max - min);
}

/** Ngày `wall` dạng "2026-10-01". */
function dayKey(wall: Date): string {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${wall.getFullYear()}-${pad(wall.getMonth() + 1)}-${pad(wall.getDate())}`;
}

/** Những câu đã nói trong một ngày, lưu trong localStorage của overlay; mất thì cùng lắm nói lại một lần. */
class DailyMemory {
  private day = "";
  private keys = new Set<string>();

  constructor() {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
      if (saved && typeof saved === "object" && "day" in saved && "keys" in saved) {
        const { day, keys } = saved as { day: unknown; keys: unknown };
        if (typeof day === "string" && Array.isArray(keys)) {
          this.day = day;
          this.keys = new Set(keys.filter((k): k is string => typeof k === "string"));
        }
      }
    } catch {
      // Không đọc được thì coi như hôm nay chưa nói gì.
    }
  }

  has(day: string, key: string): boolean {
    return this.day === day && this.keys.has(key);
  }

  add(day: string, key: string): void {
    if (day !== this.day) {
      this.day = day;
      this.keys = new Set();
    }
    this.keys.add(key);
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ day, keys: [...this.keys] }));
    } catch {
      // Không lưu được thì mở lại app có thể nói lại một lần.
    }
  }
}
