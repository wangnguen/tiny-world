import {
  PREVIEW_SECONDS,
  type City,
  type Point,
  type PreviewState,
  type WeatherPreview,
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
  activeOccasions,
  daySky,
  nowText,
  simulatedSky,
  skyLine,
  skyOf,
  toLunar,
  updateLine,
  wallClock,
  warmthLine,
  warmthOf,
  weatherLine,
  type Pet,
  type PlaceWeather,
  type Sky,
  type Warmth,
  type World,
} from "@tinyworld/sim";
import { GHOST_SIZE, Ghost } from "./ghost";
import type { PetView } from "./petView";
import { TemperatureTag } from "./temperatureTag";
import type { WeatherEffect } from "./weather";
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
/** Đang chạy, leo, nhảy thì thở dồn (1), đang đi thì vừa (0.5), còn lại thở đều (0). */
const EFFORT: Partial<Record<Pet["state"], number>> = { run: 1, climb: 1, jump: 1, walk: 0.5 };
/** Những câu đã nói hôm nay (câu của dịp lễ, con ma), để mở lại app không nói lại. */
const STORAGE_KEY = "tinyworld.said";
/** Bản mới nhất pet đã báo (`announceUpdate`), để mỗi bản chỉ báo một lần. */
const UPDATE_SAID_KEY = "tinyworld.updateSaid";

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
  /** Đang xem thử gì (gửi sang Cài đặt để nút đang bật sáng lên). */
  report(state: PreviewState): void;
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
  /** Gọi lúc đã nói thật (không phải lúc xếp hàng). */
  onSaid?: () => void;
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
  /** Nhiệt độ ở thành phố đã chọn (°C, `null`: chưa biết) và nóng hay lạnh. */
  private temperature: number | null = null;
  private warmth: Warmth | null = null;
  /** Nóng lạnh đang vẽ quanh pet (tắt hiệu ứng thời tiết thì không vẽ). */
  private warmthShown: Warmth | null = null;
  private readonly tag: TemperatureTag;
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
  /**
   * Thời tiết, nhiệt độ đang xem thử (mục Xem thử trong Cài đặt), thay trời thật tới lúc `until` (ms từ 1970);
   * `null` là không xem thử.
   */
  private preview: (WeatherPreview & { until: number }) | null = null;
  private previewTimer = 0;
  /** Lần báo Cài đặt gần nhất con ma có đang bay không. */
  private ghostReported = false;
  /** Vừa chọn thành phố mới: lấy được thời tiết ở đó thì một con báo luôn, để biết là đã chạy. */
  private announce = false;

  constructor(
    private readonly host: AmbienceHost,
    container: HTMLElement,
    settings: Settings,
  ) {
    this.settings = settings;
    this.ghost = new Ghost(container);
    this.tag = new TemperatureTag(container);
    host.world.chatter = settings.chatter;
    this.refresh();
    window.setInterval(() => this.refresh(), REFRESH_MS);
    // Overlay vừa mở (hay tải lại): không xem thử gì, Cài đặt bỏ sáng nút cũ.
    this.reportPreview();
  }

  /** Con ma đang bay: vòng lặp phải chạy tiếp kể cả khi cả nhóm đã ngủ. */
  get flying(): boolean {
    return this.ghost.flying;
  }

  /** Vòng lặp phải vẽ lại trong vòng chừng này ms (hiệu ứng thời tiết, aura, con ma); `Infinity` nếu không cần. */
  get frameMs(): number {
    if (this.ghost.flying) return 0;
    let ms = Number.POSITIVE_INFINITY;
    for (const { effect, aura } of this.host.residents()) ms = Math.min(ms, effect.frameMs, aura?.frameMs ?? ms);
    return ms;
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
    resident.effect.setWarmth(this.warmthShown);
  }

  /** Bấm đúp vào `pet`: nói giờ, thứ, ngày dương lịch, âm lịch và thời tiết ở thành phố đã chọn. */
  tellTime(pet: Pet): void {
    const wall = wallClock(new Date(), this.settings.city?.timezone);
    pet.say(nowText(wall, toLunar(wall), this.placeWeather(wall)), INFO_SECONDS);
  }

  /**
   * Mỗi lần vẽ: đặt hiệu ứng thời tiết theo thân thật của từng pet (khói thở ra đúng miệng con đó), nhãn nhiệt
   * độ cạnh con đầu tiên đang hiện.
   */
  place(now: number): void {
    let tagged = false;
    for (const { pet, view, effect, aura } of this.host.residents()) {
      const visible = pet.state !== "sleep" && !view.spriteHidden;
      aura?.update(view.foot ?? pet, view.width, view.height, now, visible);
      const body = view.body;
      if (!body) continue;
      // Aura của Long thay thế thời tiết quanh chính Long; pet khác vẫn theo thời tiết chung.
      effect.update(body, now, visible && !aura?.animating, view.mouthAt, EFFORT[pet.state] ?? 0);
      if (!tagged && !view.spriteHidden) {
        this.tag.place(body, pet.facing, this.host.world.bounds);
        tagged = true;
      }
    }
    if (!tagged) this.tag.place(null, 1, this.host.world.bounds);
  }

  /** Mỗi lượt của vòng lặp: sấm, con ma làm giật mình, câu báo đang chờ. */
  act(now: number): void {
    const residents = this.host.residents();
    if (this.sky === "storm" && residents.some((r) => r.effect.animating)) {
      const previewing = this.preview?.sky === "storm";
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
    if ((ghost !== null) !== this.ghostReported) {
      this.ghostReported = ghost !== null;
      this.reportPreview();
    }
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
        notice.onSaid?.();
      }
    }
  }

  /**
   * Mục Xem thử trong Cài đặt: thời tiết, nhiệt độ `next` quanh pet `PREVIEW_SECONDS` giây (bỏ qua công tắc và
   * trời thật; một con nói ngay câu của thứ vừa bật; giông thì sấm ngay), cả hai `null` thì về trời thật. Mỗi
   * lần bấm tính lại từ đầu `PREVIEW_SECONDS` giây.
   */
  previewWeather(next: WeatherPreview): void {
    window.clearTimeout(this.previewTimer);
    const before = this.preview;
    const active = next.sky !== null || next.temperature !== null;
    this.preview = active ? { ...next, until: Date.now() + PREVIEW_SECONDS * 1000 } : null;
    if (active) {
      this.previewTimer = window.setTimeout(() => {
        this.preview = null;
        this.refresh(true);
        this.reportPreview();
      }, PREVIEW_SECONDS * 1000);
    }
    this.refresh(true);
    if (next.sky === "storm" && before?.sky !== "storm") this.nextThunder = 0;
    // Câu của thứ vừa bật nói luôn (thay câu đang nói), không chờ tới lượt câu nói cho vui.
    let line: string | null = null;
    if (next.sky && next.sky !== before?.sky) line = skyLine(next.sky, this.temperature, this.warmth);
    else if (next.temperature !== null && next.temperature !== before?.temperature && this.warmth) {
      line = warmthLine(this.warmth, next.temperature);
    }
    const speaker = pickAwake(this.host.residents());
    if (line && speaker) speaker.pet.say(line, NOTICE_SECONDS);
    this.reportPreview();
    this.host.wake();
  }

  /** Mục Xem thử: con ma bay qua ngay (không tính vào lượt mỗi đêm). */
  previewGhost(now: number): void {
    if (!this.ghost.flying && this.host.residents().length > 0) this.ghost.fly(this.host.world.bounds, now);
    this.host.wake();
  }

  /** Báo Cài đặt đang xem thử gì. */
  private reportPreview(): void {
    const preview = this.preview;
    this.host.report({
      sky: preview?.sky ?? null,
      temperature: preview?.temperature ?? null,
      until: preview?.until ?? null,
      ghost: this.ghost.flying,
    });
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
   * GitHub có bản mới (update.rs, hỏi lại mỗi vài giờ): con thức đầu tiên báo một câu, mỗi bản một lần kể cả
   * khi mở lại app. Không đánh thức ai, không nhảy lên: không gấp.
   */
  announceUpdate(version: string): void {
    const key = `update:${version}`;
    if (this.queued.has(key) || readUpdateSaid() === version) return;
    this.queued.add(key);
    this.notices.push({ text: updateLine(version), seconds: REMINDER_SECONDS, onSaid: () => saveUpdateSaid(version) });
    this.host.wake();
  }

  /**
   * Tính lại thời tiết, ban đêm, câu của dịp lễ và con ma theo giờ hiện tại; có gì đổi thì chạy lại
   * vòng lặp vẽ (cả nhóm đang ngủ mà không có gì đổi thì để yên). `quiet`: trời đổi vì bật, tắt xem thử,
   * không nói câu thời tiết cho vui.
   */
  private refresh(quiet = false): void {
    let changed = false;
    const { world } = this.host;
    const now = new Date();
    const wall = wallClock(now, this.settings.city?.timezone);
    const report = this.freshReport(now);
    const hour = wall.getHours();
    world.night = report ? !report.isDay : hour >= NIGHT_FROM || hour < NIGHT_UNTIL;

    // Đang xem thử thì thay trời thật, kể cả khi tắt hiệu ứng thời tiết.
    const preview = this.preview;
    const sky = preview?.sky ?? this.skyAt(wall, report);
    const temperature = preview?.temperature ?? (report ? report.temperature : null);
    const warmth = warmthOf(temperature);
    const shown = this.settings.weather || preview?.temperature != null ? warmth : null;
    // Câu nói: trời đổi từ kiểu này sang kiểu khác thì nói câu của trời mới (kèm nhiệt độ); trời không đổi mà
    // chuyển nóng, lạnh thì kêu nóng, lạnh. Lúc mới mở app, mới có thời tiết, hay đang xem thử thì không nói.
    const known = this.sky !== undefined && this.sky !== null && !quiet && !preview;
    if (known && sky && sky !== this.sky) this.skyLine = skyLine(sky, temperature, warmth);
    else if (known && sky === this.sky && shown && warmth !== this.warmth && temperature !== null) {
      this.skyLine = warmthLine(shown, temperature);
    } else if (!sky) this.skyLine = null;
    if (sky !== this.sky) {
      this.sky = sky;
      for (const { effect } of this.host.residents()) effect.setSky(sky);
      changed = true;
    }
    if (shown !== this.warmthShown) {
      this.warmthShown = shown;
      for (const { effect } of this.host.residents()) effect.setWarmth(shown);
      changed = true;
    }
    this.temperature = temperature;
    this.warmth = warmth;
    // Hình trên nhãn nhiệt độ theo trời thật, kể cả khi tắt hiệu ứng thời tiết.
    const icon = preview?.sky ?? (report ? daySky(skyOf(report.code), wall.getMonth(), report.isDay) : null);
    const tagged = preview?.temperature ?? (this.settings.temperatureTag ? temperature : null);
    if (this.tag.set(icon, tagged, warmth)) changed = true;

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
    if (this.settings.city) return report ? daySky(skyOf(report.code), wall.getMonth(), report.isDay) : null;
    return daySky(simulatedSky(wall), wall.getMonth(), !this.host.world.night);
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
    return { place, sky: daySky(skyOf(report.code), wall.getMonth(), report.isDay), temperature: report.temperature };
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

/** Bản mới pet đã báo gần nhất, `null` nếu chưa báo hay không đọc được. */
function readUpdateSaid(): string | null {
  try {
    return localStorage.getItem(UPDATE_SAID_KEY);
  } catch {
    return null;
  }
}

function saveUpdateSaid(version: string): void {
  try {
    localStorage.setItem(UPDATE_SAID_KEY, version);
  } catch {
    // Không lưu được thì mở lại app có thể báo lại một lần.
  }
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
