import { useCallback, useEffect, useState } from "react";
import { BREAK_MINUTES, WATER_MINUTES, errorMessage, type ScreenStats, type Settings } from "@tinyworld/core";
import { api } from "../api";
import { BellIcon, ClockIcon } from "./icons";
import { Toggle } from "./Toggle";

/** Đọc lại giờ ngồi máy mỗi chừng này ms lúc tab đang mở. */
const REFRESH_MS = 30_000;
const within = (range: { min: number; max: number }) => (m: number) => m >= range.min && m <= range.max;
const BREAK_CHOICES = [30, 45, 50, 60, 90].filter(within(BREAK_MINUTES));
const WATER_CHOICES = [30, 45, 60, 90, 120].filter(within(WATER_MINUTES));
/** Cột cao nhất của biểu đồ 7 ngày (CSS pixel), khớp `.days__plot` trong settings.css. */
const PLOT_HEIGHT = 96;
const WEEKDAYS = ["CN", "T2", "T3", "T4", "T5", "T6", "T7"];
const HOUR = 3_600_000;
const MINUTE = 60_000;

interface Props {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
}

/** "3 giờ 12 phút", "42 phút", "< 1 phút". */
export function formatDuration(ms: number): string {
  const minutes = Math.floor(ms / MINUTE);
  if (minutes < 1) return "< 1 phút";
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m} phút`;
  return m === 0 ? `${h} giờ` : `${h} giờ ${m} phút`;
}

/** Dạng ngắn trên đỉnh cột: "3h12", "3h", "42p". */
function shortDuration(ms: number): string {
  const minutes = Math.floor(ms / MINUTE);
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  if (h === 0) return `${m}p`;
  return m === 0 ? `${h}h` : `${h}h${pad(m)}`;
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
  return (
    <>
      <section className="field">
        <h2 className="field__label">
          <ClockIcon />
          Giờ ngồi máy
        </h2>
        <div className="toggles">
          <Toggle
            label="Đếm giờ ngồi máy"
            hint="Chỉ tính lúc có chạm chuột hay phím; vắng quá 5 phút là đang nghỉ"
            checked={settings.screenTime}
            onChange={(screenTime) => onChange({ screenTime })}
          />
        </div>
        <StatsPanel enabled={settings.screenTime} />
      </section>
      <section className="field">
        <h2 className="field__label">
          <BellIcon />
          Nhắc nhở
        </h2>
        <div className="toggles">
          <Toggle
            label="Nhắc nghỉ"
            hint={`Ngồi liền ${settings.breakMinutes} phút thì pet nhắc đứng dậy nghỉ mắt`}
            checked={settings.breakReminder}
            onChange={(breakReminder) => onChange({ breakReminder })}
          >
            {settings.breakReminder && (
              <MinuteChoices
                label="Số phút ngồi liền"
                choices={BREAK_CHOICES}
                value={settings.breakMinutes}
                onChange={(breakMinutes) => onChange({ breakMinutes })}
              />
            )}
          </Toggle>
          <Toggle
            label="Nhắc uống nước"
            hint={`Cứ ngồi máy ${settings.waterMinutes} phút thì pet nhắc uống nước`}
            checked={settings.waterReminder}
            onChange={(waterReminder) => onChange({ waterReminder })}
          >
            {settings.waterReminder && (
              <MinuteChoices
                label="Số phút ngồi máy"
                choices={WATER_CHOICES}
                value={settings.waterMinutes}
                onChange={(waterMinutes) => onChange({ waterMinutes })}
              />
            )}
          </Toggle>
          <Toggle
            label="Nhắc đi ngủ"
            hint="Sau giờ này mà còn ngồi máy thì pet nhắc, 30 phút một lần, tới 5 giờ sáng"
            checked={settings.bedtimeReminder}
            onChange={(bedtimeReminder) => onChange({ bedtimeReminder })}
          >
            {settings.bedtimeReminder && (
              <input
                className="input input--time"
                type="time"
                aria-label="Giờ đi ngủ"
                value={timeOf(settings.bedtime)}
                onChange={(event) => {
                  const bedtime = minutesOf(event.target.value);
                  if (bedtime !== null) onChange({ bedtime });
                }}
              />
            )}
          </Toggle>
          <Toggle
            label="Spam Ctrl+S"
            hint="Bấm Ctrl+S 5 lần trong 10 giây thì pet kêu; chỉ đọc phím S lúc đang giữ Ctrl"
            checked={settings.saveSpam}
            onChange={(saveSpam) => onChange({ saveSpam })}
          />
        </div>
      </section>
      <p className="hint">Số liệu chỉ lưu trên máy này, không gửi đi đâu. Bấm vào pet đang nhắc để tắt lời nhắc đó.</p>
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
  return (
    <div className="segments" role="radiogroup" aria-label={label}>
      {choices.map((minutes) => (
        <button
          key={minutes}
          type="button"
          role="radio"
          aria-checked={value === minutes}
          aria-label={`${minutes} phút`}
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
    const label = i === 6 ? "Hôm nay" : WEEKDAYS[date.getUTCDay()];
    return {
      key,
      label,
      title: `${label} ${pad(date.getUTCDate())}/${pad(date.getUTCMonth() + 1)}`,
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
        <Tile label="Hôm nay" value={formatDuration(today?.activeMs ?? 0)} />
        {enabled && <Tile label="Đang ngồi liền" value={formatDuration(stats.sessionMs)} />}
        <Tile label="Ngồi liền lâu nhất" value={formatDuration(today?.longestMs ?? 0)} />
        <Tile label="Đã đứng dậy nghỉ" value={breaks === 0 ? "Chưa lần nào" : `${breaks} lần`} />
      </dl>
      <div className="stats__part">
        <span className="stats__label">7 ngày gần nhất</span>
        {week.some((day) => day.ms >= MINUTE) ? (
          <WeekChart days={week} />
        ) : (
          <p className="stats__empty">Ngồi máy một lúc là có số liệu ở đây.</p>
        )}
      </div>
      <button
        type="button"
        className={confirming ? "stats__clear stats__clear--confirm" : "stats__clear"}
        onClick={clear}
        onBlur={() => setConfirming(false)}
      >
        {confirming ? "Bấm lần nữa để xoá hết" : "Xoá số liệu"}
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
  const [hover, setHover] = useState<number | null>(null);
  const top = Math.max(1, Math.ceil(Math.max(...days.map((d) => d.ms)) / HOUR)) * HOUR;
  return (
    <figure className="days" aria-label="Giờ ngồi máy 7 ngày gần nhất">
      <div className="days__plot">
        <span className="days__grid" aria-hidden="true">
          <span className="days__tick">{top / HOUR} giờ</span>
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
              aria-label={`${day.title}: ${empty ? "không ngồi máy" : formatDuration(day.ms)}`}
              onMouseEnter={() => setHover(i)}
              onMouseLeave={() => setHover(null)}
              onFocus={() => setHover(i)}
              onBlur={() => setHover(null)}
            >
              {hover === i ? (
                <span className={`days__tip days__tip--${align}`} style={{ bottom: height + 6 }} aria-hidden="true">
                  <span className="days__tip-day">{day.title}</span>
                  {empty ? "Không ngồi máy" : formatDuration(day.ms)}
                </span>
              ) : (
                today &&
                !empty && (
                  <span className="days__value" style={{ bottom: height + 4 }} aria-hidden="true">
                    {shortDuration(day.ms)}
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
        <caption>Giờ ngồi máy 7 ngày gần nhất</caption>
        <tbody>
          {days.map((day) => (
            <tr key={day.key}>
              <th scope="row">{day.title}</th>
              <td>{formatDuration(day.ms)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </figure>
  );
}
