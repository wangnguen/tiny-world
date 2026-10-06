import { useEffect, useRef, useState } from "react";
import {
  MAX_OCCASIONS,
  OCCASION_DAYS_MAX,
  OCCASION_MESSAGE_MAX,
  OCCASION_NAME_MAX,
  PREVIEW_COLD,
  PREVIEW_HOT,
  PREVIEW_SECONDS,
  errorMessage,
  isAppError,
  type City,
  type CityResult,
  type Occasion,
  type PreviewState,
  type Settings,
  type Sky,
  type WeatherFailure,
  type WeatherPreview,
  type WeatherReport,
} from "@tinyworld/core";
import {
  NO_NETWORK,
  NO_WEATHER,
  REPORT_MAX_AGE,
  SKY_NAMES,
  activeOccasions,
  daySky,
  skyOf,
  toLunar,
  wallClock,
} from "@tinyworld/sim";
import { api } from "../api";
import { skyIconSvg } from "../skyIcons";
import { CalendarIcon, PinIcon, PlayIcon, SparkIcon } from "./icons";
import { Toggle } from "./Toggle";

/** Gõ xong chừng này ms mới tìm, để không gọi Open-Meteo mỗi lần gõ một chữ. */
const SEARCH_DELAY_MS = 450;
const SEARCH_MIN = 2;
/** Giờ ở thành phố đã chọn cập nhật mỗi chừng này ms. */
const CLOCK_MS = 15_000;

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
}

/** Tab "Thế giới": thành phố (thời tiết thật, giờ ở đó), bật tắt hiệu ứng, lịch sự kiện. */
export function WorldSettings({ settings, onChange }: Props) {
  return (
    <>
      <p className="tip">Bấm đúp vào pet để xem giờ, ngày âm lịch và thời tiết.</p>
      <CityField city={settings.city} onChange={(city) => onChange({ city })} />
      <section className="field">
        <h2 className="field__label">
          <SparkIcon />
          Hiệu ứng
        </h2>
        <div className="toggles">
          <Toggle
            label="Thời tiết quanh pet"
            hint="Nắng, mây, sao, mưa, tuyết, sương mù; nóng thì hơi nóng bốc lên, lạnh thì thở ra khói"
            checked={settings.weather}
            onChange={(weather) => onChange({ weather })}
          />
          <Toggle
            label="Nhiệt độ cạnh pet"
            hint="Số °C ở thành phố đã chọn, kèm hình thời tiết"
            checked={settings.temperatureTag}
            onChange={(temperatureTag) => onChange({ temperatureTag })}
          />
          <Toggle
            label="Pet nói chuyện cho vui"
            hint="Chào nhau, kêu trời mưa; vài câu mỗi giờ"
            checked={settings.chatter}
            onChange={(chatter) => onChange({ chatter })}
          />
          <Toggle
            label="Lịch sự kiện"
            hint="Đúng dịp thì pet nói câu chúc"
            checked={settings.events}
            onChange={(events) => onChange({ events })}
          />
          <Toggle
            label="Ma lúc 2 giờ sáng"
            hint="Thỉnh thoảng có con ma bay qua :)))"
            checked={settings.ghost}
            onChange={(ghost) => onChange({ ghost })}
          />
        </div>
      </section>
      <EffectPreviews />
      <OccasionList
        occasions={settings.occasions}
        timezone={settings.city?.timezone ?? null}
        disabled={!settings.events}
        onChange={(occasions) => onChange({ occasions })}
      />
    </>
  );
}

const PREVIEW_SKIES: { sky: Sky; label: string }[] = [
  { sky: "sunny", label: "Nắng" },
  { sky: "clear", label: "Trời quang" },
  { sky: "cloudy", label: "Nhiều mây" },
  { sky: "rain", label: "Mưa" },
  { sky: "storm", label: "Giông" },
  { sky: "snow", label: "Tuyết" },
  { sky: "fog", label: "Sương mù" },
  { sky: "petals", label: "Hoa rơi" },
];
const PREVIEW_TEMPERATURES: { temperature: number; label: string; warmth: "hot" | "cold" }[] = [
  { temperature: PREVIEW_HOT, label: `Nóng ${PREVIEW_HOT}°C`, warmth: "hot" },
  { temperature: PREVIEW_COLD, label: `Lạnh ${PREVIEW_COLD}°C`, warmth: "cold" },
];
const NO_PREVIEW: PreviewState = { sky: null, temperature: null, until: null, ghost: false };
/** Đếm ngược xem thử mỗi chừng này ms. */
const COUNTDOWN_MS = 500;

/** Hình thời tiết nhỏ trên nút (bộ icon chung với nhãn nhiệt độ cạnh pet; chuỗi SVG cố định). */
function SkyIcon({ sky }: { sky: Sky }) {
  return <span className="sky-icon" dangerouslySetInnerHTML={{ __html: skyIconSvg(sky) }} />;
}

/**
 * Mục "Xem thử": bấm là pet gặp ngay thời tiết, nhiệt độ đó trong `PREVIEW_SECONDS` giây (một kiểu thời tiết và
 * một mức nhiệt cùng lúc), bấm lại để tắt; con ma bay qua một lượt. Nút đang bật sáng lên, kèm đếm ngược, theo
 * đúng thứ overlay đang vẽ (overlay báo lại qua Rust, mở lại Cài đặt vẫn đúng).
 */
function EffectPreviews() {
  const [state, setState] = useState<PreviewState>(NO_PREVIEW);
  const [now, setNow] = useState(() => Date.now());
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    const unlisten = api.onPreviewChanged((next) => {
      setState(next);
      setNow(Date.now());
    });
    api
      .getPreview()
      .then((next) => {
        if (!alive) return;
        setState(next);
        setNow(Date.now());
      })
      .catch((e: unknown) => console.warn("Không đọc được trạng thái xem thử:", errorMessage(e)));
    return () => {
      alive = false;
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    if (state.until === null) return;
    const timer = window.setInterval(() => setNow(Date.now()), COUNTDOWN_MS);
    return () => window.clearInterval(timer);
  }, [state.until]);

  const left = state.until === null ? 0 : Math.max(0, Math.ceil((state.until - now) / 1000));
  const sky = left > 0 ? state.sky : null;
  const temperature = left > 0 ? state.temperature : null;
  const active = sky !== null || temperature !== null;

  const send = (next: WeatherPreview) => {
    setProblem(null);
    api.previewWeather(next).catch((e: unknown) => setProblem(errorMessage(e)));
  };
  const flyGhost = () => {
    setProblem(null);
    api.previewGhost().catch((e: unknown) => setProblem(errorMessage(e)));
  };

  const names = [
    PREVIEW_SKIES.find((p) => p.sky === sky)?.label,
    PREVIEW_TEMPERATURES.find((p) => p.temperature === temperature)?.label ??
      (temperature === null ? undefined : `${temperature}°C`),
  ].filter((name) => name !== undefined);
  let status = "Chưa xem thử gì.";
  if (active) status = `Đang xem: ${names.join(" · ")} · còn ${left} giây`;
  else if (state.ghost) status = "Con ma đang bay qua...";

  return (
    <section className="field">
      <h2 className="field__label">
        <PlayIcon />
        Xem thử
      </h2>
      <p className="hint">
        Bấm để pet gặp ngay trong {PREVIEW_SECONDS} giây, kể cả khi đang tắt ở trên; bấm lại để tắt. Bật được
        một kiểu thời tiết và một mức nhiệt cùng lúc.
      </p>
      <div className="preview-row">
        <span className="preview-row__label">Thời tiết</span>
        <div className="segments" role="group" aria-label="Thời tiết">
          {PREVIEW_SKIES.map((p) => (
            <button
              key={p.sky}
              type="button"
              className={`segment segment--icon${sky === p.sky ? " segment--active" : ""}`}
              aria-pressed={sky === p.sky}
              onClick={() => send({ sky: sky === p.sky ? null : p.sky, temperature })}
            >
              <SkyIcon sky={p.sky} />
              {p.label}
            </button>
          ))}
        </div>
      </div>
      <div className="preview-row">
        <span className="preview-row__label">Nhiệt độ</span>
        <div className="segments" role="group" aria-label="Nhiệt độ">
          {PREVIEW_TEMPERATURES.map((p) => (
            <button
              key={p.temperature}
              type="button"
              className={`segment segment--${p.warmth}${temperature === p.temperature ? " segment--active" : ""}`}
              aria-pressed={temperature === p.temperature}
              onClick={() => send({ sky, temperature: temperature === p.temperature ? null : p.temperature })}
            >
              {p.label}
            </button>
          ))}
        </div>
      </div>
      <div className="preview-row">
        <span className="preview-row__label">Sự kiện</span>
        <div className="segments" role="group" aria-label="Sự kiện">
          <button
            type="button"
            className={`segment${state.ghost ? " segment--active" : ""}`}
            aria-pressed={state.ghost}
            disabled={state.ghost}
            onClick={flyGhost}
          >
            Con ma
          </button>
        </div>
      </div>
      <div className={`preview-status${active || state.ghost ? " preview-status--on" : ""}`} role="status">
        <span className="preview-status__text">{status}</span>
        <button
          type="button"
          className="button button--quiet"
          disabled={!active}
          onClick={() => send({ sky: null, temperature: null })}
        >
          Dừng
        </button>
        {active && <span className="preview-status__bar" style={{ width: `${(left / PREVIEW_SECONDS) * 100}%` }} />}
      </div>
      {problem && <p className="hint hint--error">{problem}</p>}
    </section>
  );
}

const pad = (n: number) => String(n).padStart(2, "0");

/** Kết quả thời tiết còn dùng được của đúng thành phố `city`. */
function freshReport(report: WeatherReport | null, city: City): WeatherReport | null {
  if (!report) return null;
  if (Math.abs(report.latitude - city.latitude) > 1e-6 || Math.abs(report.longitude - city.longitude) > 1e-6) return null;
  return Date.now() / 1000 - report.fetchedAt < REPORT_MAX_AGE ? report : null;
}

/**
 * Mục "Thành phố": tìm theo tên (Open-Meteo), chọn một kết quả thì pet theo thời tiết thật và giờ ở đó. Chưa
 * chọn thì thời tiết giả lập theo mùa, giờ theo máy, không gọi mạng.
 */
function CityField({ city, onChange }: { city: City | null; onChange: (city: City | null) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<CityResult[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [report, setReport] = useState<WeatherReport | null>(null);
  const [failure, setFailure] = useState<WeatherFailure | null>(null);
  const [now, setNow] = useState(() => new Date());
  /** Lần tìm gần nhất; kết quả của lần tìm cũ về sau thì bỏ. */
  const search = useRef(0);

  useEffect(() => {
    const text = query.trim();
    const id = ++search.current;
    if (text.length < SEARCH_MIN) {
      setResults([]);
      setStatus(null);
      return;
    }
    const timer = window.setTimeout(() => {
      setStatus("Đang tìm...");
      api
        .searchCity(text)
        .then((found) => {
          if (id !== search.current) return;
          setResults(found);
          setStatus(found.length ? null : "Không tìm thấy thành phố nào :)");
        })
        .catch((e: unknown) => {
          if (id !== search.current) return;
          setResults([]);
          setStatus(isAppError(e) && e.code === "OFFLINE" ? NO_NETWORK : errorMessage(e));
        });
    }, SEARCH_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [query]);

  // Thời tiết hiện tại: lấy cái đã có, rồi nghe Rust báo mỗi lần lấy được thời tiết mới hoặc bị lỗi.
  useEffect(() => {
    let alive = true;
    const unlisten = [
      api.onWeatherChanged((next) => {
        setReport(next);
        if (next) setFailure(null);
      }),
      api.onWeatherFailed(setFailure),
    ];
    api
      .getWeather()
      .then((next) => alive && setReport(next))
      .catch((e: unknown) => console.warn("Không lấy được thời tiết:", errorMessage(e)));
    return () => {
      alive = false;
      for (const promise of unlisten) promise.then((stop) => stop()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), CLOCK_MS);
    return () => window.clearInterval(timer);
  }, []);

  const choose = (result: CityResult) => {
    setFailure(null);
    setQuery("");
    onChange({
      name: result.label,
      latitude: result.latitude,
      longitude: result.longitude,
      timezone: result.timezone,
    });
  };

  let weatherText = "Đang lấy thời tiết...";
  const fresh = city ? freshReport(report, city) : null;
  const wall = wallClock(now, city?.timezone);
  if (fresh) weatherText = `${SKY_NAMES[daySky(skyOf(fresh.code), wall.getMonth(), fresh.isDay)]}, ${Math.round(fresh.temperature)}°C`;
  else if (failure) weatherText = failure.offline ? NO_NETWORK : NO_WEATHER;

  return (
    <section className="field">
      <h2 className="field__label">
        <PinIcon />
        Thành phố
      </h2>
      {city ? (
        <div className="city">
          <div className="city__text">
            <strong>{city.name}</strong>
            <small>
              {pad(wall.getHours())}:{pad(wall.getMinutes())} · {weatherText}
            </small>
          </div>
          <button type="button" className="button button--quiet" onClick={() => onChange(null)}>
            Bỏ
          </button>
        </div>
      ) : (
        <p className="hint">Chọn thành phố để pet theo thời tiết và giờ ở đó.</p>
      )}
      <input
        className="input"
        type="search"
        placeholder={city ? "Đổi thành phố..." : "Tìm thành phố, ví dụ Hà Nội"}
        value={query}
        maxLength={80}
        onChange={(event) => setQuery(event.target.value)}
      />
      {status && <p className="hint">{status}</p>}
      {results.length > 0 && (
        <ul className="results">
          {results.map((result) => (
            <li key={`${result.latitude},${result.longitude}`}>
              <button type="button" className="result" onClick={() => choose(result)}>
                {result.label}
              </button>
            </li>
          ))}
        </ul>
      )}
      {/* Giấy phép CC BY 4.0 của Open-Meteo: chỗ nào hiện dữ liệu của họ thì ghi nguồn kèm link. */}
      {(city || results.length > 0) && (
        <p className="hint hint--small">
          Thời tiết từ{" "}
          <a
            className="link"
            href="https://open-meteo.com/"
            onClick={(event) => {
              event.preventDefault();
              api.openLink("https://open-meteo.com/").catch((e: unknown) => console.warn("Không mở được link:", errorMessage(e)));
            }}
          >
            Open-Meteo.com
          </a>
        </p>
      )}
    </section>
  );
}

/** Dịp mới: hôm nay, dương lịch, một ngày. */
function blankOccasion(): Occasion {
  const today = new Date();
  return {
    name: "",
    day: today.getDate(),
    month: today.getMonth() + 1,
    lunar: false,
    days: 1,
    message: "",
    enabled: true,
  };
}

/** Ngày lớn nhất của tháng `month` (âm lịch tối đa 30 ngày, 29/2 dương lịch vẫn chọn được), khớp settings.rs. */
function maxDay(month: number, lunar: boolean): number {
  if (lunar) return 30;
  if (month === 2) return 29;
  return [4, 6, 9, 11].includes(month) ? 30 : 31;
}

function describe(occasion: Occasion): string {
  const parts = [`${occasion.day}/${occasion.month}${occasion.lunar ? " âm lịch" : ""}`];
  if (occasion.days > 1) parts.push(`${occasion.days} ngày`);
  return parts.join(" · ");
}

/**
 * Mục "Lịch sự kiện": các dịp lặp lại mỗi năm (dương hoặc âm lịch). Bật tắt từng dịp, sửa, xoá, thêm dịp
 * mới (sinh nhật, ngày kỷ niệm...). Đúng dịp thì một con nói câu của dịp đó mỗi ngày một lần.
 */
function OccasionList({
  occasions,
  timezone,
  disabled,
  onChange,
}: {
  occasions: Occasion[];
  /** Múi giờ của thành phố đã chọn: "hôm nay" theo giờ ở đó, như overlay. */
  timezone: string | null;
  disabled: boolean;
  onChange: (occasions: Occasion[]) => void;
}) {
  const today = wallClock(new Date(), timezone);
  const lunar = toLunar(today);
  const active = activeOccasions(occasions, today);
  const todayText = `Hôm nay ${today.getDate()}/${today.getMonth() + 1}, âm lịch ${lunar.day}/${lunar.month}${lunar.leap ? " nhuận" : ""}`;
  /** Dịp đang sửa: chỉ số trong danh sách, `-1` là đang thêm dịp mới, `null` là không sửa gì. */
  const [editing, setEditing] = useState<number | null>(null);
  const save = (index: number, occasion: Occasion) => {
    onChange(index < 0 ? [...occasions, occasion] : occasions.map((o, i) => (i === index ? occasion : o)));
    setEditing(null);
  };
  const remove = (index: number) => {
    onChange(occasions.filter((_, i) => i !== index));
    setEditing(null);
  };
  const toggle = (index: number, enabled: boolean) =>
    onChange(occasions.map((o, i) => (i === index ? { ...o, enabled } : o)));

  return (
    <section className={disabled ? "field field--off" : "field"}>
      <div className="field__head">
        <h2 className="field__label">
          <CalendarIcon />
          Lịch sự kiện
          <span className="field__count">
            {occasions.length}/{MAX_OCCASIONS}
          </span>
        </h2>
        {editing === null && occasions.length < MAX_OCCASIONS && (
          <button type="button" className="button" onClick={() => setEditing(-1)}>
            Thêm
          </button>
        )}
      </div>
      {editing === -1 && (
        <OccasionEditor initial={blankOccasion()} onSave={(o) => save(-1, o)} onCancel={() => setEditing(null)} />
      )}
      <p className="hint">
        {active.length
          ? `${todayText}: ${active.map((o) => o.name).join(", ")}.`
          : `${todayText}: không có dịp nào.`}
      </p>
      {occasions.length === 0 && editing !== -1 && <p className="hint">Chưa có dịp nào.</p>}
      <ul className="occasions">
        {occasions.map((occasion, index) =>
          editing === index ? (
            <li key={index}>
              <OccasionEditor
                initial={occasion}
                onSave={(o) => save(index, o)}
                onCancel={() => setEditing(null)}
                onDelete={() => remove(index)}
              />
            </li>
          ) : (
            <li key={index} className="occasion">
              <input
                className="checkbox"
                type="checkbox"
                aria-label={`Bật ${occasion.name}`}
                checked={occasion.enabled}
                onChange={(event) => toggle(index, event.target.checked)}
              />
              <span className="occasion__text">
                <strong>
                  {occasion.name}
                  {active.includes(occasion) && <span className="badge">Hôm nay</span>}
                </strong>
                <small>{describe(occasion)}</small>
              </span>
              <button
                type="button"
                className="button button--quiet"
                disabled={editing !== null}
                onClick={() => setEditing(index)}
              >
                Sửa
              </button>
            </li>
          ),
        )}
      </ul>
    </section>
  );
}

function OccasionEditor({
  initial,
  onSave,
  onCancel,
  onDelete,
}: {
  initial: Occasion;
  onSave: (occasion: Occasion) => void;
  onCancel: () => void;
  onDelete?: () => void;
}) {
  const [draft, setDraft] = useState(initial);
  const [problem, setProblem] = useState<string | null>(null);
  const set = (patch: Partial<Occasion>) => setDraft((d) => ({ ...d, ...patch }));
  const number = (value: string) => Math.trunc(Number(value)) || 0;

  const submit = () => {
    const name = draft.name.trim();
    if (!name) return setProblem("Nhập tên dịp.");
    if (draft.month < 1 || draft.month > 12) return setProblem("Tháng từ 1 tới 12.");
    const last = maxDay(draft.month, draft.lunar);
    if (draft.day < 1 || draft.day > last) return setProblem(`Tháng ${draft.month} chỉ có tới ngày ${last}.`);
    if (draft.days < 1 || draft.days > OCCASION_DAYS_MAX) return setProblem(`Kéo dài từ 1 tới ${OCCASION_DAYS_MAX} ngày.`);
    onSave({ ...draft, name, message: draft.message.trim() });
  };

  return (
    <form
      className="editor"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <label className="editor__row">
        <span>Tên</span>
        <input
          className="input"
          value={draft.name}
          maxLength={OCCASION_NAME_MAX}
          placeholder="Ví dụ: Sinh nhật mình"
          autoFocus
          onChange={(event) => set({ name: event.target.value })}
        />
      </label>
      <div className="editor__row">
        <span>Ngày</span>
        <div className="editor__date">
          <input
            className="input input--number"
            type="number"
            min={1}
            max={maxDay(draft.month, draft.lunar)}
            aria-label="Ngày"
            value={draft.day}
            onChange={(event) => set({ day: number(event.target.value) })}
          />
          <span className="muted">/</span>
          <input
            className="input input--number"
            type="number"
            min={1}
            max={12}
            aria-label="Tháng"
            value={draft.month}
            onChange={(event) => set({ month: number(event.target.value) })}
          />
          <div className="segments" role="radiogroup" aria-label="Loại lịch">
            {[false, true].map((lunar) => (
              <button
                key={String(lunar)}
                type="button"
                role="radio"
                aria-checked={draft.lunar === lunar}
                className={draft.lunar === lunar ? "segment segment--active" : "segment"}
                onClick={() => set({ lunar })}
              >
                {lunar ? "Âm lịch" : "Dương lịch"}
              </button>
            ))}
          </div>
        </div>
      </div>
      <label className="editor__row">
        <span>Số ngày</span>
        <input
          className="input input--number"
          type="number"
          min={1}
          max={OCCASION_DAYS_MAX}
          value={draft.days}
          onChange={(event) => set({ days: number(event.target.value) })}
        />
      </label>
      <label className="editor__row">
        <span>Câu nói</span>
        <input
          className="input"
          value={draft.message}
          maxLength={OCCASION_MESSAGE_MAX}
          placeholder="Để trống thì không nói gì"
          onChange={(event) => set({ message: event.target.value })}
        />
      </label>
      {problem && <p className="error editor__problem">{problem}</p>}
      <div className="editor__actions">
        {onDelete && (
          <button type="button" className="button button--danger" onClick={onDelete}>
            Xoá
          </button>
        )}
        <button type="button" className="button button--quiet" onClick={onCancel}>
          Huỷ
        </button>
        <button type="submit" className="button button--primary">
          Lưu
        </button>
      </div>
    </form>
  );
}
