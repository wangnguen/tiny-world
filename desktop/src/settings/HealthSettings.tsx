import { useCallback, useEffect, useState } from "react";
import {
  BREAK_MINUTES,
  WATER_MINUTES,
  errorMessage,
  fill,
  type Messages,
  type ScreenStats,
  type Settings,
} from "@tinyworld/core";
import { api } from "../api";
import { useMessages } from "../i18n";
import { BellIcon, ClockIcon } from "./icons";
import { Toggle } from "./Toggle";

/** Đọc lại giờ ngồi máy mỗi chừng này ms lúc tab đang mở. */
const REFRESH_MS = 30_000;
const within = (range: { min: number; max: number }) => (m: number) => m >= range.min && m <= range.max;
const BREAK_CHOICES = [30, 45, 50, 60, 90].filter(within(BREAK_MINUTES));
const WATER_CHOICES = [30, 45, 60, 90, 120].filter(within(WATER_MINUTES));
/** Cột cao nhất của biểu đồ 7 ngày (CSS pixel), khớp `.days__plot` trong settings.css. */
const PLOT_HEIGHT = 96;
const HOUR = 3_600_000;
const MINUTE = 60_000;

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
}

/** "3 giờ 12 phút", "42 phút", "< 1 phút" ("3 h 12 min", "42 min", "< 1 min"). */
export function formatDuration(ms: number, m: Messages): string {
  const minutes = Math.floor(ms / MINUTE);
  if (minutes < 1) return m.health.underMinute;
  const h = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (h === 0) return fill(m.health.durationMinutes, { m: rest });
  return rest === 0 ? fill(m.health.durationHours, { h }) : fill(m.health.durationHoursMinutes, { h, m: rest });
}

/** Dạng ngắn trên đỉnh cột: "3h12", "3h", "42p" ("42m"). */
function shortDuration(ms: number, m: Messages): string {
  const minutes = Math.floor(ms / MINUTE);
  const h = Math.floor(minutes / 60);
  const rest = minutes % 60;
  if (h === 0) return fill(m.health.shortMinutes, { m: rest });
  return rest === 0 ? fill(m.health.shortHours, { h }) : fill(m.health.shortHoursMinutes, { h, mm: pad(rest) });
}

/** Ngày "2026-10-01" lùi `back` ngày, tính theo lịch (không theo múi giờ). */
function shiftDate(date: string, back: number): Date {
  const [y, m, d] = date.split("-").map(Number);
  return new Date(Date.UTC(y, m - 1, d - back));
}

const keyOf = (date: Date) => date.toISOString().slice(0, 10);
const pad = (n: number) => String(n).padStart(2, "0");

function timeOf(minutes: number): string {
  return `${pad(Math.floor(minutes / 60))}:${pad(minutes % 60)}`;
}

function minutesOf(time: string): number | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(time);
  if (!match) return null;
  const value = Number(match[1]) * 60 + Number(match[2]);
  return value < 24 * 60 ? value : null;
}

/**
 * Tab "Sức khoẻ": giờ ngồi máy, nhắc nghỉ, uống nước, nhắc khuya, spam Ctrl+S. Mặc định tắt hết, số liệu chỉ
 * ở máy này.
 */
export function HealthSettings({ settings, onChange }: Props) {
  const m = useMessages();
  return (
    <>
      <section className="field">
        <h2 className="field__label">
          <ClockIcon />
          {m.health.screenTime}
        </h2>
        <div className="toggles">
          <Toggle
            label={m.health.count}
            hint={m.health.countHint}
            checked={settings.screenTime}
            onChange={(screenTime) => onChange({ screenTime })}
          />
        </div>
        <StatsPanel enabled={settings.screenTime} />
      </section>
      <section className="field">
        <h2 className="field__label">
          <BellIcon />
          {m.health.reminders}
        </h2>
        <div className="toggles">
          <Toggle
            label={m.health.break}
            hint={fill(m.health.breakHint, { minutes: settings.breakMinutes })}
            checked={settings.breakReminder}
            onChange={(breakReminder) => onChange({ breakReminder })}
          >
            {settings.breakReminder && (
              <MinuteChoices
                label={m.health.breakChoices}
                choices={BREAK_CHOICES}
                value={settings.breakMinutes}
                onChange={(breakMinutes) => onChange({ breakMinutes })}
              />
            )}
          </Toggle>
          <Toggle
            label={m.health.water}
            hint={fill(m.health.waterHint, { minutes: settings.waterMinutes })}
            checked={settings.waterReminder}
            onChange={(waterReminder) => onChange({ waterReminder })}
          >
            {settings.waterReminder && (
              <MinuteChoices
                label={m.health.waterChoices}
                choices={WATER_CHOICES}
                value={settings.waterMinutes}
                onChange={(waterMinutes) => onChange({ waterMinutes })}
              />
            )}
          </Toggle>
          <Toggle
            label={m.health.bedtime}
            hint={m.health.bedtimeHint}
            checked={settings.bedtimeReminder}
            onChange={(bedtimeReminder) => onChange({ bedtimeReminder })}
          >
            {settings.bedtimeReminder && (
              <input
                className="input input--time"
                type="time"
                aria-label={m.health.bedtimeLabel}
                value={timeOf(settings.bedtime)}
                onChange={(event) => {
                  const bedtime = minutesOf(event.target.value);
                  if (bedtime !== null) onChange({ bedtime });
                }}
              />
            )}
          </Toggle>
          <Toggle
            label={m.health.saveSpam}
            hint={m.health.saveSpamHint}
            checked={settings.saveSpam}
            onChange={(saveSpam) => onChange({ saveSpam })}
          />
        </div>
      </section>
      <p className="hint">{m.health.privacy}</p>
    </>
  );
}

/** Chọn số phút, hiện số không kèm chữ "phút" cho gọn (trình đọc màn hình đọc đủ). */
function MinuteChoices({
  label,
  choices,
  value,
  onChange,
}: {
  label: string;
  choices: number[];
  value: number;
  onChange: (minutes: number) => void;
}) {
  const m = useMessages();
  return (
    <div className="segments" role="radiogroup" aria-label={label}>
      {choices.map((minutes) => (
        <button
          key={minutes}
          type="button"
          role="radio"
          aria-checked={value === minutes}
          aria-label={fill(m.health.minutes, { n: minutes })}
          className={value === minutes ? "segment segment--active" : "segment"}
          onClick={() => onChange(minutes)}
        >
          {minutes}
        </button>
      ))}
    </div>
  );
}

/** Hôm nay (giờ ngồi máy, ngồi liền, lượt lâu nhất, số lần nghỉ) và 7 ngày gần nhất; xoá được. */
function StatsPanel({ enabled }: { enabled: boolean }) {
  const m = useMessages();
  const [stats, setStats] = useState<ScreenStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const load = useCallback(() => {
    api
      .getStats()
      .then((next) => {
        setStats(next);
        setError(null);
      })
      .catch((e: unknown) => setError(errorMessage(e)));
  }, []);
  useEffect(() => {
    load();
    const timer = window.setInterval(load, REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [load, enabled]);

  if (error) return <p className="error">{error}</p>;
  if (!stats) return null;
  const hasData = Object.keys(stats.days).length > 0;
  if (!enabled && !hasData) return null;

  const today = stats.days[stats.today];
  const week = Array.from({ length: 7 }, (_, i) => {
    const date = shiftDate(stats.today, 6 - i);
    const key = keyOf(date);
    const label = i === 6 ? m.health.today : m.calendar.shortWeekdays[date.getUTCDay()];
    return {
      key,
      label,
      title: fill(m.health.dayTitle, {
        label,
        dd: pad(date.getUTCDate()),
        mm: pad(date.getUTCMonth() + 1),
        day: date.getUTCDate(),
        shortMonth: m.calendar.shortMonths[date.getUTCMonth()],
      }),
      ms: stats.days[key]?.activeMs ?? 0,
    };
  });
  const breaks = today?.breaks ?? 0;

  const clear = () => {
    if (!confirming) {
      setConfirming(true);
      return;
    }
    setConfirming(false);
    api
      .clearStats()
      .then(load)
      .catch((e: unknown) => setError(errorMessage(e)));
  };

  return (
    <div className="stats">
      <dl className="stats__tiles">
        <Tile label={m.health.today} value={formatDuration(today?.activeMs ?? 0, m)} />
        {enabled && <Tile label={m.health.session} value={formatDuration(stats.sessionMs, m)} />}
        <Tile label={m.health.longest} value={formatDuration(today?.longestMs ?? 0, m)} />
        <Tile label={m.health.breaks} value={breaks === 0 ? m.health.noBreaks : fill(m.health.breakCount, { n: breaks })} />
      </dl>
      <div className="stats__part">
        <span className="stats__label">{m.health.week}</span>
        {week.some((day) => day.ms >= MINUTE) ? (
          <WeekChart days={week} />
        ) : (
          <p className="stats__empty">{m.health.weekEmpty}</p>
        )}
      </div>
      <button
        type="button"
        className={confirming ? "stats__clear stats__clear--confirm" : "stats__clear"}
        onClick={clear}
        onBlur={() => setConfirming(false)}
      >
        {confirming ? m.health.clearConfirm : m.health.clear}
      </button>
    </div>
  );
}

function Tile({ label, value }: { label: string; value: string }) {
  return (
    <div className="stats__tile">
      <dt className="stats__label">{label}</dt>
      <dd className="stats__value">{value}</dd>
    </div>
  );
}

interface WeekDay {
  key: string;
  label: string;
  title: string;
  ms: number;
}

/**
 * Cột giờ ngồi máy 7 ngày, một màu: cột hôm nay ghi số trên đỉnh; rê chuột (hay Tab tới) một cột thì ô số
 * nổi ngay trên cột đó, không đẩy gì xuống. Một đường kẻ mờ ở mức giờ tròn cao nhất làm mốc; ngày không ngồi
 * máy là vạch mờ sát đáy.
 */
function WeekChart({ days }: { days: WeekDay[] }) {
  const m = useMessages();
  const [hover, setHover] = useState<number | null>(null);
  const top = Math.max(1, Math.ceil(Math.max(...days.map((d) => d.ms)) / HOUR)) * HOUR;
  return (
    <figure className="days" aria-label={m.health.chart}>
      <div className="days__plot">
        <span className="days__grid" aria-hidden="true">
          <span className="days__tick">{fill(m.health.hoursTick, { n: top / HOUR })}</span>
        </span>
        {days.map((day, i) => {
          const empty = day.ms < MINUTE;
          const height = empty ? 2 : Math.max(3, Math.round((day.ms / top) * PLOT_HEIGHT));
          const today = i === days.length - 1;
          // Cột ở hai đầu thì ô số canh theo mép cột cho khỏi tràn ra ngoài khung.
          const align = i < 2 ? "start" : i > days.length - 3 ? "end" : "center";
          return (
            <button
              key={day.key}
              type="button"
              className="days__slot"
              aria-label={`${day.title}: ${empty ? m.health.idle : formatDuration(day.ms, m)}`}
              onMouseEnter={() => setHover(i)}
              onMouseLeave={() => setHover(null)}
              onFocus={() => setHover(i)}
              onBlur={() => setHover(null)}
            >
              {hover === i ? (
                <span className={`days__tip days__tip--${align}`} style={{ bottom: height + 6 }} aria-hidden="true">
                  <span className="days__tip-day">{day.title}</span>
                  {empty ? m.health.idleTip : formatDuration(day.ms, m)}
                </span>
              ) : (
                today &&
                !empty && (
                  <span className="days__value" style={{ bottom: height + 4 }} aria-hidden="true">
                    {shortDuration(day.ms, m)}
                  </span>
                )
              )}
              <span className={empty ? "days__bar days__bar--empty" : "days__bar"} style={{ height }} />
            </button>
          );
        })}
      </div>
      <div className="days__axis" aria-hidden="true">
        {days.map((day) => (
          <span key={day.key}>{day.label}</span>
        ))}
      </div>
      <table className="sr-only">
        <caption>{m.health.chart}</caption>
        <tbody>
          {days.map((day) => (
            <tr key={day.key}>
              <th scope="row">{day.title}</th>
              <td>{formatDuration(day.ms, m)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </figure>
  );
}
