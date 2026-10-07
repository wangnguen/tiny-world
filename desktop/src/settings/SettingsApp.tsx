import { useEffect, useState } from "react";
import { errorMessage, fill, messages, type Language, type Messages, type Settings } from "@tinyworld/core";
import { api } from "../api";
import { LangContext, guessLang } from "../i18n";
import { listPacks, resolvePacks } from "../overlay/sprites";
import { ChatIcon, CubeIcon, GlobeIcon, InfoIcon, RunnerIcon, WindowsIcon } from "./icons";
import { PetPicker } from "./PetPicker";
import { PetPreview } from "./PetPreview";
import { CornerDecor, HILL_SPOT, NightScene, PineDecor } from "./scenery";
import { HealthSettings } from "./HealthSettings";
import { Toggle } from "./Toggle";
import { UpdateCard, UpdateCheck, useUpdate } from "./Update";
import { WorldSettings } from "./WorldSettings";

interface Option<T> {
  value: T;
  label: string;
}

const SIZES: Option<number>[] = [0.5, 0.75, 1, 1.5, 2].map((value) => ({
  value,
  label: `${value * 100}%`,
}));

const SPEEDS: { value: number; key: keyof Messages["settings"]["speeds"] }[] = [
  { value: 0.5, key: "slow" },
  { value: 1, key: "normal" },
  { value: 1.5, key: "fast" },
];

const LANGUAGES: Language[] = ["auto", "vi", "en"];

const PACKS = listPacks();

type Tab = "pet" | "world" | "health";
const TABS: Tab[] = ["pet", "world", "health"];

/** Dòng dưới tiêu đề: bản Release chỉ có số version, bản build khi push lên `main` kèm số lần chạy và commit. */
function versionLabel(version: string, m: Messages): string {
  // Chạy dev thì version trong tauri.conf.json không phải version thật (CI ghi vào lúc build).
  if (import.meta.env.DEV) return m.settings.devBuild;
  const build = import.meta.env.VITE_BUILD;
  return build ? fill(m.settings.versionBuild, { version, build }) : fill(m.settings.version, { version });
}

/** Cửa sổ cài đặt (tray → Cài đặt…). Đổi gì áp dụng ngay cho pet, không cần bấm lưu. */
export function SettingsApp() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("pet");
  const updater = useUpdate();
  // Chưa đọc được cài đặt thì đoán theo Windows; đổi ngôn ngữ thì Rust trả về `lang` mới.
  const lang = settings?.lang ?? guessLang();
  const m = messages(lang);

  useEffect(() => {
    Promise.all([api.getSettings(), api.getAutostart()])
      .then(([loaded, enabled]) => {
        setSettings(loaded);
        setAutostart(enabled);
      })
      .catch((e) => setError(errorMessage(e)));
    // Không đọc được version thì chỉ thiếu một dòng chữ, không báo lỗi.
    api
      .version()
      .then(setVersion)
      .catch((e: unknown) => console.warn("Không đọc được version:", errorMessage(e)));
  }, []);

  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

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

  // Pack đã chọn không còn thì overlay bỏ qua (hết cả thì dùng pack đầu tiên), ở đây cũng đánh dấu đúng như vậy.
  const pets = settings ? resolvePacks(settings.pets).flatMap((id) => id ?? []) : [];

  return (
    <LangContext.Provider value={lang}>
      <main className="page">
        <header className="hero">
          <NightScene />
          {settings && <PetPreview pet={pets[0] ?? null} footX={HILL_SPOT.x} footY={HILL_SPOT.y} height={96} />}
          <h1>{m.settings.title}</h1>
          {version && <p className="hero__version">{versionLabel(version, m)}</p>}
          <UpdateCheck update={updater} />
        </header>
        <UpdateCard update={updater} />

        {!settings || autostart === null ? (
          <p className="muted loading">{error ?? m.settings.loading}</p>
        ) : (
          <>
            <nav className="tabs" role="tablist">
              {TABS.map((id) => (
                <button
                  key={id}
                  type="button"
                  role="tab"
                  aria-selected={tab === id}
                  className={tab === id ? "tab tab--active" : "tab"}
                  onClick={() => setTab(id)}
                >
                  {m.settings.tabs[id]}
                </button>
              ))}
            </nav>
            {tab === "world" && <WorldSettings settings={settings} onChange={update} />}
            {tab === "health" && <HealthSettings settings={settings} onChange={update} />}
            {tab === "pet" && (
              <>
                {PACKS.length > 1 && (
                  <PetPicker packs={PACKS} value={pets} onChange={(ids) => update({ pets: ids })} />
                )}
                <section className="field">
                  <h2 className="field__label">
                    <CubeIcon />
                    {m.settings.size}
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
                    {m.settings.speed}
                  </h2>
                  <Choices
                    options={SPEEDS.map(({ value, key }) => ({ value, label: m.settings.speeds[key] }))}
                    value={settings.speed}
                    onChange={(speed) => update({ speed })}
                  />
                </section>
                <section className="field">
                  <h2 className="field__label">
                    <ChatIcon />
                    {m.settings.chat}
                  </h2>
                  <div className="toggles">
                    <Toggle
                      label={m.settings.chatToggle}
                      hint={m.settings.chatHint}
                      checked={settings.chat}
                      onChange={(chat) => update({ chat })}
                    />
                  </div>
                </section>
                <section className="field">
                  <h2 className="field__label">
                    <GlobeIcon />
                    {m.settings.language}
                  </h2>
                  <Choices
                    options={LANGUAGES.map((value) => ({ value, label: m.settings.languages[value] }))}
                    value={settings.language}
                    onChange={(language) => update({ language })}
                    compact
                  />
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
                    <strong>{m.settings.startup}</strong>
                    <small>{m.settings.startupHint}</small>
                  </span>
                  <PineDecor />
                </label>
              </>
            )}
            {error && <p className="error">{error}</p>}
          </>
        )}

        <footer className="note">
          <InfoIcon />
          {m.settings.applied}
          {/* Nằm trong footer để luôn ở góc dưới cùng của trang, kể cả khi trang dài phải cuộn. */}
          <CornerDecor />
        </footer>
      </main>
    </LangContext.Provider>
  );
}

function Choices<T extends number | string>({
  options,
  value,
  onChange,
  sparkle = false,
  compact = false,
}: {
  options: Option<T>[];
  value: T;
  onChange: (value: T) => void;
  /** Vài tia sáng nhỏ trên nút đang chọn. */
  sparkle?: boolean;
  /** Chữ nhỏ hơn cho nút có chữ dài. */
  compact?: boolean;
}) {
  const className = ["choices", sparkle && "choices--sparkle", compact && "choices--compact"].filter(Boolean).join(" ");
  return (
    <div className={className} role="radiogroup">
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
