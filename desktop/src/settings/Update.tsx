import { useEffect, useState } from "react";
import { errorMessage, type UpdateInfo, type UpdateProgress } from "@tinyworld/core";
import { api } from "../api";

/** Câu kết quả của nút Kiểm tra bản mới hiện chừng này ms rồi ẩn. */
const CHECK_MESSAGE_MS = 5_000;

type Phase = "idle" | "checking" | "downloading" | "restarting";

export interface Update {
  /** Bản mới (theo lần hỏi gần nhất), `null` nếu chưa thấy. */
  info: UpdateInfo | null;
  phase: Phase;
  progress: UpdateProgress | null;
  /** Câu báo nhỏ: đang dùng bản mới nhất, lỗi khi hỏi hay tải. */
  message: string | null;
  check: () => void;
  install: () => void;
}

/** Trạng thái cập nhật cho cửa sổ Cài đặt: nghe Rust báo có bản mới, tiến độ tải (update.rs). */
export function useUpdate(): Update {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [phase, setPhase] = useState<Phase>("idle");
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [message, setMessage] = useState<string | null>(null);

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
    if (!message || phase !== "idle" || info) return;
    const timer = window.setTimeout(() => setMessage(null), CHECK_MESSAGE_MS);
    return () => window.clearTimeout(timer);
  }, [message, phase, info]);

  const check = () => {
    setPhase("checking");
    setMessage(null);
    api
      .checkUpdate()
      .then((next) => {
        setInfo(next);
        if (!next) setMessage("Đang dùng bản mới nhất.");
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

  return { info, phase, progress, message, check, install };
}

const megabytes = (bytes: number) =>
  `${(bytes / 1024 / 1024).toLocaleString("vi-VN", { maximumFractionDigits: 1 })} MB`;

/** Thẻ "Có bản mới" ngay dưới đầu trang: cập nhật ngay (tải, cài đè, tự mở lại) hoặc xem có gì mới. */
export function UpdateCard({ update }: { update: Update }) {
  const { info, phase, progress, message, install } = update;
  if (!info) return null;
  const percent = progress && progress.total > 0 ? Math.floor((progress.received / progress.total) * 100) : 0;
  const openPage = () =>
    api.openLink(info.page).catch((e: unknown) => console.warn("Không mở được trang bản mới:", errorMessage(e)));

  let detail = `Đang dùng ${info.current} · bộ cài ${megabytes(info.size)}. Cài xong pet tự mở lại.`;
  if (phase === "downloading") detail = `Đang tải bản mới... ${percent}%`;
  else if (phase === "restarting") detail = "Đang cài bản mới, pet sẽ tự mở lại sau ít giây...";

  return (
    <section className="update" aria-live="polite">
      <div className="update__text">
        <strong>Có bản mới {info.version}</strong>
        <small>{detail}</small>
      </div>
      <div className="update__actions">
        <button type="button" className="button button--quiet" onClick={openPage}>
          Xem có gì mới
        </button>
        <button type="button" className="button button--primary" disabled={phase !== "idle"} onClick={install}>
          Cập nhật ngay
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
  const { info, phase, message, check } = update;
  if (info) return null;
  if (message && phase === "idle") return <p className="hero__check hero__check--message">{message}</p>;
  return (
    <button type="button" className="hero__check" disabled={phase !== "idle"} onClick={check}>
      {phase === "checking" ? "Đang kiểm tra..." : "Kiểm tra bản mới"}
    </button>
  );
}
