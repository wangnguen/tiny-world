import { useEffect, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { errorMessage, isAppError, type ChatTarget } from "@tinyworld/core";
import { api } from "../api";
import { loadThumbnail } from "../overlay/sprites";
import { ChatScene, HILL_FOOT } from "./ChatScene";
import { parseReply, type Inline } from "./markdown";
import {
  MESSAGE_MAX,
  SUGGESTION_POOL,
  buildPrompt,
  dayKey,
  dayText,
  personaOf,
  pickSuggestions,
  type Turn,
} from "./prompt";

/** Chiều cao frame của pet ở đầu khung chat (CSS pixel); frame có chừa khoảng trống quanh nhân vật. */
const AVATAR_HEIGHT = 76;
/** Chữ "Đã chép" hiện chừng này ms. */
const COPIED_MS = 1500;

/** Một dòng trong khung chat: câu của người dùng, câu của pet, hoặc ghi chú (mất mạng, gửi nhanh quá...). */
interface Entry {
  id: number;
  kind: Turn["from"] | "note";
  text: string;
}

/**
 * Cửa sổ chat với một pet (click chuột phải vào pet). Lịch sử chỉ giữ trong lúc cửa sổ còn mở; đổi sang
 * con khác thì bắt đầu đoạn chat mới.
 */
export function ChatApp() {
  const [target, setTarget] = useState<ChatTarget | null>(null);
  const [entries, setEntries] = useState<Entry[]>([]);
  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  /** Gợi ý lúc chưa hỏi gì (bấm là gửi luôn): câu Gemini viết cho hôm nay nếu có, không thì câu có sẵn. */
  const [suggestions, setSuggestions] = useState(() => pickSuggestions(SUGGESTION_POOL));
  const written = useRef<string[]>([]);
  const nextId = useRef(1);
  const end = useRef<HTMLDivElement>(null);
  const box = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    api
      .chatTarget()
      .then((t) => t && setTarget(t))
      .catch((e: unknown) => console.warn("Không biết đang chat với ai:", errorMessage(e)));
    // Mỗi ngày Rust chỉ hỏi Gemini một lần, mở lại khung chat thì lấy lại danh sách đã có.
    const now = new Date();
    api
      .chatSuggestions(dayKey(now), dayText(now))
      .then((list) => {
        if (list.length < 2) return;
        written.current = list;
        setSuggestions(pickSuggestions(list));
      })
      .catch((e: unknown) => console.warn("Không lấy được câu gợi ý:", errorMessage(e)));
    const unlisten = api.onChatTarget((next) => {
      setTarget((current) => {
        if (current?.pet !== next.pet) {
          setEntries([]);
          setSuggestions(pickSuggestions(written.current.length > 0 ? written.current : SUGGESTION_POOL));
        }
        return next;
      });
      box.current?.focus();
    });
    return () => {
      unlisten.then((stop) => stop()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    end.current?.scrollIntoView({ block: "end" });
  }, [entries, sending]);

  const add = (kind: Entry["kind"], text: string) =>
    setEntries((list) => [...list, { id: nextId.current++, kind, text }]);

  const send = async (text: string) => {
    const message = text.trim().slice(0, MESSAGE_MAX);
    if (!target || sending || !message) return;
    const history = entries.flatMap((e): Turn[] => (e.kind === "note" ? [] : [{ from: e.kind, text: e.text }]));
    const prompt = buildPrompt(personaOf(target.pet, target.name), history, message, new Date());
    const id = nextId.current++;
    setEntries((list) => [...list, { id, kind: "user", text: message }]);
    setInput("");
    setSending(true);
    try {
      add("pet", await api.sendChat(prompt));
    } catch (e) {
      if (isAppError(e) && e.code === "BUSY") {
        // Chưa gửi đi: trả lại câu vào ô nhập để gửi lại sau.
        setEntries((list) => list.filter((entry) => entry.id !== id));
        setInput(message);
      }
      add("note", isAppError(e) && e.code === "OFFLINE" ? "Mất mạng" : errorMessage(e));
    } finally {
      setSending(false);
      box.current?.focus();
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
      event.preventDefault();
      void send(input);
    }
  };

  const name = target?.name ?? "pet";
  return (
    <main className="chat">
      <header className="chat__head">
        <ChatScene />
        <strong className="chat__title">{name}</strong>
        {target && <Avatar pet={target.pet} />}
      </header>
      <section className="chat__list" aria-live="polite">
        {entries.length === 0 && (
          <div className="chat__empty">
            <div className="chat__hello">
              <strong>Hỏi {name} gì cũng được</strong>
              <span>Tỉ giá, tin tức, đổi đơn vị, viết giúp một câu...</span>
            </div>
            <div className="chat__suggestions">
              {suggestions.map((s) => (
                <button key={s} type="button" className="chip" disabled={!target || sending} onClick={() => void send(s)}>
                  {s}
                </button>
              ))}
            </div>
          </div>
        )}
        {entries.map((entry) =>
          entry.kind === "note" ? (
            <p key={entry.id} className="note">
              {entry.text}
            </p>
          ) : (
            <Message key={entry.id} entry={entry} name={name} />
          ),
        )}
        {sending && (
          <p className="typing">
            <span className="typing__name">{name}</span> đang nghĩ<span className="typing__dots" aria-hidden="true" />
          </p>
        )}
        <div ref={end} />
      </section>
      <form
        className="composer"
        onSubmit={(event) => {
          event.preventDefault();
          void send(input);
        }}
      >
        <textarea
          ref={box}
          className="composer__input"
          rows={1}
          autoFocus
          maxLength={MESSAGE_MAX}
          placeholder={`Nhắn cho ${name}...`}
          value={input}
          onChange={(event) => setInput(event.target.value)}
          onKeyDown={onKeyDown}
        />
        <button
          type="submit"
          className="composer__send"
          aria-label="Gửi"
          title="Gửi (Enter)"
          disabled={!target || sending || !input.trim()}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 12l16-8-6 16-2.5-6.5z" />
            <path d="M11.5 13.5L20 4" />
          </svg>
        </button>
      </form>
    </main>
  );
}

/** Một câu trong đoạn chat; câu của pet có tên pet phía trên và nút Chép. */
function Message({ entry, name }: { entry: Entry; name: string }) {
  const [copied, setCopied] = useState(false);
  if (entry.kind === "user") return <div className="msg msg--user">{entry.text}</div>;
  const copy = () => {
    navigator.clipboard
      .writeText(entry.text)
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), COPIED_MS);
      })
      .catch((e: unknown) => console.warn("Không chép được:", e));
  };
  return (
    <div className="msg msg--pet">
      <span className="msg__name">{name}</span>
      <Reply text={entry.text} />
      <button type="button" className="msg__copy" onClick={copy}>
        {copied ? "Đã chép" : "Chép"}
      </button>
    </div>
  );
}

function Reply({ text }: { text: string }) {
  return (
    <>
      {parseReply(text).map((block, i) => {
        switch (block.kind) {
          case "heading":
            return (
              <p key={i} className="md-heading">
                <Inlines parts={block.content} />
              </p>
            );
          case "code":
            return (
              <pre key={i} className="md-code">
                {block.text}
              </pre>
            );
          case "list": {
            const items = block.items.map((item, j) => (
              <li key={j}>
                <Inlines parts={item} />
              </li>
            ));
            return block.ordered ? <ol key={i}>{items}</ol> : <ul key={i}>{items}</ul>;
          }
          default:
            return (
              <p key={i}>
                {block.lines.map((line, j) => (
                  <span key={j}>
                    {j > 0 && <br />}
                    <Inlines parts={line} />
                  </span>
                ))}
              </p>
            );
        }
      })}
    </>
  );
}

function Inlines({ parts }: { parts: Inline[] }): ReactNode {
  return parts.map((part, i) => {
    switch (part.kind) {
      case "bold":
        return <strong key={i}>{part.text}</strong>;
      case "italic":
        return <em key={i}>{part.text}</em>;
      case "code":
        return <code key={i}>{part.text}</code>;
      case "link":
        return (
          <a
            key={i}
            href={part.url}
            title={part.url}
            onClick={(event) => {
              event.preventDefault();
              api.openLink(part.url).catch((e: unknown) => console.warn("Không mở được link:", errorMessage(e)));
            }}
          >
            {part.text}
          </a>
        );
      default:
        return <span key={i}>{part.text}</span>;
    }
  });
}

/**
 * Pet đứng trên đồi ở đầu khung chat, chạy animation `idle`, quay mặt sang trái (về phía tên và đoạn chat).
 * Chỉ nạp một ảnh `idle` của pack, không nạp cả bộ như overlay.
 */
function Avatar({ pet }: { pet: string }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    let cancelled = false;
    let timer = 0;
    loadThumbnail(pet)
      .then(({ image, frames, fps, anchor, pixelArt, facing }) => {
        const canvas = ref.current;
        const ctx = canvas?.getContext("2d");
        if (cancelled || !canvas || !ctx) return;
        const { width, height } = frames[0];
        // Ảnh lớn thu về đúng cỡ; ảnh nhỏ phóng theo số nguyên để giữ nét pixel.
        const fit = AVATAR_HEIGHT / height;
        const scale = fit >= 1 ? Math.round(fit) : fit;
        const flip = facing === "right";
        const footX = (flip ? width - anchor.x : anchor.x) * scale;
        const dpr = window.devicePixelRatio || 1;
        canvas.width = Math.round(width * scale * dpr);
        canvas.height = Math.round(height * scale * dpr);
        Object.assign(canvas.style, {
          width: `${width * scale}px`,
          height: `${height * scale}px`,
          right: `${HILL_FOOT.right - (width * scale - footX)}px`,
          top: `${HILL_FOOT.y - anchor.y * scale}px`,
        });
        let index = 0;
        const draw = () => {
          const frame = frames[index++ % frames.length];
          ctx.setTransform(1, 0, 0, 1, 0, 0);
          ctx.clearRect(0, 0, canvas.width, canvas.height);
          ctx.imageSmoothingEnabled = !pixelArt;
          if (flip) ctx.setTransform(-1, 0, 0, 1, canvas.width, 0);
          ctx.drawImage(image, frame.x, frame.y, frame.width, frame.height, 0, 0, canvas.width, canvas.height);
        };
        draw();
        if (frames.length > 1) timer = window.setInterval(draw, 1000 / fps);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [pet]);
  return <canvas ref={ref} className="chat__avatar" aria-hidden="true" />;
}
