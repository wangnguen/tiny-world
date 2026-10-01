import { useEffect, useRef, useState, type WheelEvent } from "react";
import { MAX_PETS } from "@tinyworld/core";
import { loadThumbnail, type PackInfo, type Thumbnail } from "../overlay/sprites";
import { ChevronIcon, PawIcon } from "./icons";

/** Cỡ ảnh trong mỗi ô (CSS pixel), khớp `.pet-choice canvas` trong settings.css. */
const THUMB = 44;
/** Hàng đặt chân nhân vật trong ảnh, để mọi nhân vật đứng trên cùng một mặt đất. */
const THUMB_FLOOR = 42;
/** Số ô mỗi trang: 8 cột × 2 hàng, khớp `.pets` trong settings.css. */
const PAGE_SIZE = 16;
/** Touchpad bắn liền nhiều sự kiện lăn: mỗi khoảng này chỉ lật một trang. */
const WHEEL_GAP_MS = 350;

interface Props {
  packs: PackInfo[];
  /** Các nhân vật đang hiện, theo thứ tự chọn (luôn có ít nhất một). */
  value: string[];
  onChange: (ids: string[]) => void;
}

/**
 * Mục "Nhân vật": tên các nhân vật đang chọn, lưới nhân vật chia trang (mỗi ô là frame đầu của `idle`,
 * tên đầy đủ hiện khi rê chuột). Bấm một ô để thêm hoặc bớt nhân vật đó: tối đa `MAX_PETS` con, luôn
 * còn ít nhất một con; đủ rồi thì các ô chưa chọn mờ đi. Nhiều hơn một trang thì có nút lật trang cạnh
 * tiêu đề, lăn chuột trên lưới cũng lật được. Mở ra ở đúng trang có nhân vật chọn đầu tiên.
 */
export function PetPicker({ packs, value, onChange }: Props) {
  const pages = Math.ceil(packs.length / PAGE_SIZE);
  const [page, setPage] = useState(() =>
    Math.max(0, Math.floor(packs.findIndex((pack) => pack.id === value[0]) / PAGE_SIZE)),
  );
  const lastWheel = useRef(0);
  const go = (next: number) => setPage(Math.min(pages - 1, Math.max(0, next)));
  const onWheel = (event: WheelEvent) => {
    if (pages < 2 || event.deltaY === 0) return;
    const now = performance.now();
    if (now - lastWheel.current < WHEEL_GAP_MS) return;
    lastWheel.current = now;
    go(page + Math.sign(event.deltaY));
  };
  const chosen = value.flatMap((id) => packs.find((pack) => pack.id === id) ?? []);
  const full = value.length >= MAX_PETS;
  const toggle = (id: string) => {
    if (value.includes(id)) {
      if (value.length > 1) onChange(value.filter((v) => v !== id));
    } else if (!full) {
      onChange([...value, id]);
    }
  };

  return (
    <section className="field">
      <div className="field__head">
        <h2 className="field__label">
          <PawIcon />
          Nhân vật
          <span className="field__count">
            {value.length}/{MAX_PETS}
          </span>
        </h2>
        {pages > 1 && <Pager page={page} pages={pages} onChange={go} />}
      </div>
      {/* Tên đầy đủ của các nhân vật đang hiện, theo thứ tự chọn; dài quá thì xuống dòng, không cắt. */}
      <ul className="chosen" aria-label="Đang hiện">
        {chosen.map((pack) => (
          <li key={pack.id} className="chosen__name">
            {pack.name}
          </li>
        ))}
      </ul>
      <div className="pets" role="group" aria-label={`Nhân vật, chọn tối đa ${MAX_PETS}`} onWheel={onWheel}>
        {packs.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE).map((pack) => {
          const active = value.includes(pack.id);
          const locked = !active && full;
          const only = active && value.length === 1;
          const hint = locked
            ? `${pack.name}: đã đủ ${MAX_PETS} nhân vật, bỏ chọn bớt một con trước`
            : only
              ? `${pack.name}: cần ít nhất một nhân vật`
              : pack.name;
          return (
            <button
              key={pack.id}
              type="button"
              role="checkbox"
              aria-checked={active}
              aria-disabled={locked || only}
              aria-label={pack.name}
              title={hint}
              className={["pet-choice", active && "pet-choice--active", locked && "pet-choice--locked"]
                .filter(Boolean)
                .join(" ")}
              onClick={() => toggle(pack.id)}
            >
              <PetThumbnail id={pack.id} />
            </button>
          );
        })}
      </div>
    </section>
  );
}

/** Mũi tên trước/sau và một chấm cho mỗi trang (chấm trang hiện tại dài hơn, bấm chấm để nhảy tới). */
function Pager({ page, pages, onChange }: { page: number; pages: number; onChange: (page: number) => void }) {
  return (
    <nav className="pager" aria-label="Trang nhân vật">
      <button
        type="button"
        className="pager__arrow"
        aria-label="Trang trước"
        disabled={page === 0}
        onClick={() => onChange(page - 1)}
      >
        <ChevronIcon direction="left" />
      </button>
      {Array.from({ length: pages }, (_, i) => (
        <button
          key={i}
          type="button"
          className={i === page ? "pager__dot pager__dot--active" : "pager__dot"}
          aria-label={`Trang ${i + 1}`}
          aria-current={i === page ? "page" : undefined}
          onClick={() => onChange(i)}
        />
      ))}
      <button
        type="button"
        className="pager__arrow"
        aria-label="Trang sau"
        disabled={page === pages - 1}
        onClick={() => onChange(page + 1)}
      >
        <ChevronIcon direction="right" />
      </button>
    </nav>
  );
}

function PetThumbnail({ id }: { id: string }) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    let cancelled = false;
    loadThumbnail(id)
      .then((thumbnail) => {
        if (!cancelled && ref.current) draw(ref.current, thumbnail);
      })
      .catch((error) => console.warn(`Không tạo được ảnh nhỏ cho ${id}:`, error));
    return () => {
      cancelled = true;
    };
  }, [id]);

  return <canvas ref={ref} aria-hidden="true" />;
}

/**
 * Cắt sát phần có hình, thu vừa ô hoặc phóng theo số nguyên, đặt chân lên
 * `THUMB_FLOOR` và quay sang trái như pet trên đồi ở đầu trang.
 */
function draw(canvas: HTMLCanvasElement, { image, frame, pixelArt, facing }: Thumbnail): void {
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(THUMB * dpr);
  canvas.height = Math.round(THUMB * dpr);
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const box = boundsOf(image, frame.x, frame.y, frame.width, frame.height);
  if (!box) return;
  const fit = (THUMB - 4) / Math.max(box.width, box.height);
  const scale = fit >= 1 ? Math.floor(fit) : fit;
  const width = box.width * scale;
  const height = box.height * scale;
  const left = Math.round((THUMB - width) / 2);
  const top = THUMB_FLOOR - height;

  ctx.imageSmoothingEnabled = !pixelArt;
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  if (facing === "right") ctx.setTransform(-dpr, 0, 0, dpr, canvas.width, 0);
  ctx.drawImage(image, frame.x + box.x, frame.y + box.y, box.width, box.height, left, top, width, height);
}

/** Khung nhỏ nhất bao phần có hình (alpha > 0) của một frame, toạ độ trong frame. */
function boundsOf(
  image: HTMLImageElement,
  x: number,
  y: number,
  width: number,
  height: number,
): { x: number; y: number; width: number; height: number } | null {
  const canvas = new OffscreenCanvas(width, height);
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  if (!ctx) return null;
  ctx.drawImage(image, x, y, width, height, 0, 0, width, height);
  const { data } = ctx.getImageData(0, 0, width, height);
  let minX = width;
  let minY = height;
  let maxX = -1;
  let maxY = -1;
  for (let py = 0; py < height; py++) {
    for (let px = 0; px < width; px++) {
      if (data[(py * width + px) * 4 + 3] === 0) continue;
      minX = Math.min(minX, px);
      maxX = Math.max(maxX, px);
      minY = Math.min(minY, py);
      maxY = Math.max(maxY, py);
    }
  }
  if (maxX < 0) return null;
  return { x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1 };
}
