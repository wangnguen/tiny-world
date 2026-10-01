import type { Hat } from "@tinyworld/core";
import { CROWN_SHARE, crownOf, type Mask, type SpriteSet } from "./spriteSet";

/**
 * Mũ nhân vật đội trong dịp lễ (lịch sự kiện): ảnh trong `assets/items/`, do `scripts/prepare-items.mjs`
 * tạo từ ảnh gốc trong `assets/sprite-sources/items/`. Ảnh vẽ cho nhân vật quay sang phải, cỡ theo pixel
 * của frame nhân vật 192×192; `anchor` là giữa vành mũ ở mép dưới.
 */
interface ItemMeta {
  file: string;
  width: number;
  height: number;
  anchor: { x: number; y: number };
}

interface Item extends ItemMeta {
  image: HTMLImageElement;
}

type HatName = Exclude<Hat, "none">;

const manifests = import.meta.glob<Record<HatName, ItemMeta>>("../../../assets/items/items.json", {
  eager: true,
  import: "default",
});
const urls = import.meta.glob<string>("../../../assets/items/*.webp", {
  eager: true,
  query: "?url",
  import: "default",
});

/** Vành mũ chồng xuống dưới đỉnh đầu chừng này phần chiều cao ảnh: mũ đội lên đầu chứ không đặt trên đầu. */
const SINK: Record<HatName, number> = {
  party: 0.06,
  noel: 0.22,
  tet: 0.3,
};

/**
 * Chỉnh riêng cho nhân vật mà đỉnh đầu đo tự động (`crownOf`) chưa đúng chỗ:
 * - `crown`: hàng phải rộng chừng này phần bề ngang đầu mới là đỉnh đầu (mặc định `CROWN_SHARE`), tăng
 *   lên khi tai hay lá trên đầu to, mũ bị đặt lên tai, lên lá
 * - `dx`, `dy`: dời mũ (pixel của frame, nhân vật quay sang phải)
 * - `size`: nhân với cỡ mũ
 */
interface HatFit {
  crown?: number;
  dx?: number;
  dy?: number;
  size?: number;
}

const FITS: Record<string, HatFit> = {
  // Hai tai dựng sát nhau, rộng bằng nửa đầu.
  "b-bong": { crown: 0.6 },
  // Hai lá trên đầu rộng bằng cả đầu: mũ đội dưới lá.
  "c-mam": { crown: 0.9 },
  "c-dua": { crown: 0.9 },
  // Ngọn lửa trên đầu: mũ đội ở chỗ đầu đã tròn, dưới chóp lửa.
  "c-tan": { crown: 0.8 },
};

const items = new Map<HatName, Item>();

/** Nạp ảnh mũ một lần lúc mở overlay; thiếu ảnh hay ảnh lỗi thì mũ đó không vẽ, app vẫn chạy. */
export async function loadHats(): Promise<void> {
  const manifest = Object.values(manifests)[0];
  if (!manifest) {
    console.warn("Thiếu assets/items/items.json, không có mũ cho lịch sự kiện.");
    return;
  }
  await Promise.all(
    (Object.entries(manifest) as [HatName, ItemMeta][]).map(async ([name, meta]) => {
      const url = Object.entries(urls).find(([path]) => path.endsWith(`/${meta.file}`))?.[1];
      try {
        if (!url) throw new Error(`thiếu ${meta.file}`);
        const image = new Image();
        image.src = url;
        await image.decode();
        items.set(name, { ...meta, image });
      } catch (error) {
        console.warn(`Không nạp được mũ ${name}:`, error);
      }
    }),
  );
}

/**
 * Vẽ mũ `hat` lên đỉnh đầu của nhân vật `sprite` trong frame có mặt nạ `mask`, theo đơn vị đang vẽ trên
 * `ctx` (pixel frame nhân `density`; `ctx` đã lật sẵn nếu nhân vật quay sang trái, và làm mịn hay không
 * theo `sprite.pixelArt` như lúc vẽ frame). Chỗ đặt đo theo từng frame nên mũ đi theo đầu lúc đi, nhảy, ngủ.
 */
export function drawHat(
  ctx: CanvasRenderingContext2D,
  hat: Hat,
  sprite: SpriteSet,
  mask: Mask,
  density: number,
): void {
  if (hat === "none") return;
  const item = items.get(hat);
  if (!item) return;
  const fit = (sprite.id && FITS[sprite.id]) || {};
  const crown = crownOf(mask, fit.crown ?? CROWN_SHARE);
  // Ảnh mũ tính theo frame 192 px: pack có frame cỡ khác thì mũ to nhỏ theo.
  const scale = (sprite.frameWidth / 192) * (fit.size ?? 1) * density;
  const x = Math.round((crown.x + (fit.dx ?? 0)) * density - item.anchor.x * scale);
  const y = Math.round((crown.y + (fit.dy ?? 0)) * density + (SINK[hat] * item.height - item.anchor.y) * scale);
  ctx.drawImage(item.image, x, y, item.width * scale, item.height * scale);
}
