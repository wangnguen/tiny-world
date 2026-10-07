import { useEffect, useState } from "react";
import { errorMessage, fill, type UpdateInfo, type UpdateProgress } from "@tinyworld/core";
import { api } from "../api";
import { useLang, useMessages } from "../i18n";

/** Câu kết quả của nút Kiểm tra bản mới hiện chừng này ms rồi ẩn. */
const CHECK_MESSAGE_MS = 5_000;

type Phase = "idle" | "checking" | "downloading" | "restarting";

export interface Update {
  /** Bản mới (theo lần hỏi gần nhất), `null` nếu chưa thấy. */
  info: UpdateInfo | null;
  phase: Phase;
  progress: UpdateProgress | null;
  /** Câu báo lỗi khi hỏi hay tải. */
  message: string | null;
  /** Vừa bấm Kiểm tra bản mới và đang dùng bản mới nhất. */
  upToDate: boolean;
  check: () => void;
  install: () => void;
}

/** Trạng thái cập nhật cho cửa sổ Cài đặt: nghe Rust báo có bản mới, tiến độ tải (update.rs). */
export function useUpdate(): Update {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [upToDate, setUpToDate] = useState(false);

  useEffect(() => {
    let alive = true;
    const unlisten = [api.onUpdateAvailable(setInfo), api.onUpdateProgress(setProgress)];
    api
      .getUpdate()
      .then((next) => alive && next && setInfo(next))
      .catch((e: unknown) => console.warn("Không đọc được bản mới:", errorMessage(e)));
    return () => {
      alive = false;
      for (const promise of unlisten) promise.then((stop) => stop()).catch(() => {});
    };
  }, []);

  // Câu kết quả cạnh nút Kiểm tra bản mới tự ẩn; câu lỗi trong thẻ bản mới thì giữ tới lần bấm sau.
  useEffect(() => {
    if (!(message || upToDate) || phase !== "idle" || info) return;
    const timer = window.setTimeout(() => {
      setMessage(null);
      setUpToDate(false);
    }, CHECK_MESSAGE_MS);
    return () => window.clearTimeout(timer);
  }, [message, upToDate, phase, info]);

  const check = () => {
    setPhase("checking");
    setMessage(null);
    setUpToDate(false);
    api
      .checkUpdate()
      .then((next) => {
        setInfo(next);
        setUpToDate(!next);
      })
      .catch((e: unknown) => setMessage(errorMessage(e)))
      .finally(() => setPhase("idle"));
  };

  const install = () => {
    setPhase("downloading");
    setProgress(null);
    setMessage(null);
    api
      .installUpdate()
      // Tải xong: app thoát, bộ cài chạy rồi tự mở lại TinyWorld.
      .then(() => setPhase("restarting"))
      .catch((e: unknown) => {
        setPhase("idle");
        setMessage(errorMessage(e));
      });
  };

  return { info, phase, progress, message, upToDate, check, install };
}

const megabytes = (bytes: number, locale: string) =>
  `${(bytes / 1024 / 1024).toLocaleString(locale, { maximumFractionDigits: 1 })} MB`;

/** Thẻ "Có bản mới" ngay dưới đầu trang: cập nhật ngay (tải, cài đè, tự mở lại) hoặc xem có gì mới. */
export function UpdateCard({ update }: { update: Update }) {
  const { info, phase, progress, message, install } = update;
  const m = useMessages();
  const locale = useLang() === "vi" ? "vi-VN" : "en-US";
  if (!info) return null;
  const percent = progress && progress.total > 0 ? Math.floor((progress.received / progress.total) * 100) : 0;
  const openPage = () =>
    api.openLink(info.page).catch((e: unknown) => console.warn("Không mở được trang bản mới:", errorMessage(e)));

  let detail = fill(m.update.detail, { current: info.current, size: megabytes(info.size, locale) });
  if (phase === "downloading") detail = fill(m.update.downloading, { percent });
  else if (phase === "restarting") detail = m.update.restarting;

  return (
    <section className="update" aria-live="polite">
      <div className="update__text">
        <strong>{fill(m.update.available, { version: info.version })}</strong>
        <small>{detail}</small>
      </div>
      <div className="update__actions">
        <button type="button" className="button button--quiet" onClick={openPage}>
          {m.update.whatsNew}
        </button>
        <button type="button" className="button button--primary" disabled={phase !== "idle"} onClick={install}>
          {m.update.install}
        </button>
      </div>
      {message && <p className="update__message">{message}</p>}
      {phase === "downloading" && <span className="update__bar" style={{ width: `${percent}%` }} />}
    </section>
  );
}

/**
 * Nút nhỏ "Kiểm tra bản mới" dưới dòng phiên bản ở đầu trang; bấm xong câu kết quả hiện thế chỗ vài giây.
 * Đã có thẻ bản mới thì ẩn (thẻ lo hết).
 */
export function UpdateCheck({ update }: { update: Update }) {
  const { info, phase, message, upToDate, check } = update;
  const m = useMessages();
  if (info) return null;
  const said = message ?? (upToDate ? m.update.latest : null);
  if (said && phase === "idle") return <p className="hero__check hero__check--message">{said}</p>;
  return (
    <button type="button" className="hero__check" disabled={phase !== "idle"} onClick={check}>
      {phase === "checking" ? m.update.checking : m.update.check}
    </button>
  );
}
