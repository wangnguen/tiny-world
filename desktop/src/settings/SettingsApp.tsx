import { useEffect, useState } from "react";
import { errorMessage, type Settings } from "@tinyworld/core";
import { api } from "../api";
import { CubeIcon, InfoIcon, RunnerIcon, WindowsIcon } from "./icons";
import { PetPreview } from "./PetPreview";
import { CornerDecor, HILL_SPOT, NightScene, PineDecor } from "./scenery";

interface Option {
  value: number;
  label: string;
}

const SIZES: Option[] = [0.5, 0.75, 1, 1.5, 2].map((value) => ({
  value,
  label: `${value * 100}%`,
}));

const SPEEDS: Option[] = [
  { value: 0.5, label: "Chậm" },
  { value: 1, label: "Vừa" },
  { value: 1.5, label: "Nhanh" },
];

/** Cửa sổ cài đặt (tray → Cài đặt…). Đổi gì áp dụng ngay cho pet, không cần bấm lưu. */
export function SettingsApp() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.getSettings(), api.getAutostart()])
      .then(([loaded, enabled]) => {
        setSettings(loaded);
        setAutostart(enabled);
      })
      .catch((e) => setError(errorMessage(e)));
  }, []);

  const update = async (patch: Partial<Settings>) => {
    if (!settings) return;
    const previous = settings;
    setSettings({ ...settings, ...patch });
    try {
      setSettings(await api.setSettings({ ...settings, ...patch }));
      setError(null);
    } catch (e) {
      setSettings(previous);
      setError(errorMessage(e));
    }
  };

  const toggleAutostart = async (enabled: boolean) => {
    setAutostart(enabled);
    try {
      await api.setAutostart(enabled);
      setAutostart(await api.getAutostart());
      setError(null);
    } catch (e) {
      setAutostart(!enabled);
      setError(errorMessage(e));
    }
  };

  return (
    <main className="page">
      <header className="hero">
        <NightScene />
        <PetPreview footX={HILL_SPOT.x} footY={HILL_SPOT.y} scale={3} />
        <h1>Cài đặt</h1>
      </header>

      {!settings || autostart === null ? (
        <p className="muted loading">{error ?? "Đang tải..."}</p>
      ) : (
        <>
          <section className="field">
            <h2 className="field__label">
              <CubeIcon />
              Cỡ nhân vật
            </h2>
            <Choices
              options={SIZES}
              value={settings.size}
              onChange={(size) => update({ size })}
              sparkle
            />
          </section>
          <section className="field">
            <h2 className="field__label">
              <RunnerIcon />
              Tốc độ đi lại
            </h2>
            <Choices options={SPEEDS} value={settings.speed} onChange={(speed) => update({ speed })} />
          </section>
          <label className="startup">
            <WindowsIcon />
            <input
              className="checkbox"
              type="checkbox"
              checked={autostart}
              onChange={(event) => toggleAutostart(event.target.checked)}
            />
            <span className="startup__text">
              <strong>Chạy cùng Windows</strong>
              <small>Tự mở TinyWorld khi đăng nhập Windows</small>
            </span>
            <PineDecor />
          </label>
          {error && <p className="error">{error}</p>}
        </>
      )}

      <footer className="note">
        <InfoIcon />
        Thay đổi được áp dụng ngay.
      </footer>
      <CornerDecor />
    </main>
  );
}

function Choices({
  options,
  value,
  onChange,
  sparkle = false,
}: {
  options: Option[];
  value: number;
  onChange: (value: number) => void;
  /** Vài tia sáng nhỏ trên nút đang chọn. */
  sparkle?: boolean;
}) {
  return (
    <div className={sparkle ? "choices choices--sparkle" : "choices"} role="radiogroup">
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={active}
            className={active ? "choice choice--active" : "choice"}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
