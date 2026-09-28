import { useEffect, useRef } from "react";
import { loadThumbnail, type PackInfo, type Thumbnail } from "../overlay/sprites";

/** Cỡ ảnh trong mỗi ô (CSS pixel), khớp `.pet-choice canvas` trong settings.css. */
const THUMB = 44;
/** Hàng đặt chân nhân vật trong ảnh, để mọi nhân vật đứng trên cùng một mặt đất. */
const THUMB_FLOOR = 42;

interface Props {
  packs: PackInfo[];
  value: string | null;
  onChange: (id: string) => void;
}

/** Lưới nhân vật để chọn, mỗi ô là frame đầu của `idle`. Tên đầy đủ hiện khi rê chuột. */
export function PetPicker({ packs, value, onChange }: Props) {
  return (
    <div className="pets" role="radiogroup" aria-label="Nhân vật">
      {packs.map((pack) => {
        const active = pack.id === value;
        return (
          <button
            key={pack.id}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={pack.name}
            title={pack.name}
            className={active ? "pet-choice pet-choice--active" : "pet-choice"}
            onClick={() => onChange(pack.id)}
          >
            <PetThumbnail id={pack.id} />
          </button>
        );
      })}
    </div>
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
 * Cắt sát phần có hình, phóng to theo số nguyên (pixel art không nhoè) cho vừa ô, đặt chân lên
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
  const scale = Math.max(1, Math.floor((THUMB - 4) / Math.max(box.width, box.height)));
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
