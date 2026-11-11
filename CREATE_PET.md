# Tạo nhân vật mới cho TinyWorld

Mỗi nhân vật không phải GIF: đó là bốn sheet tư thế được tách thành 12 animation WebP để app tự phát theo trạng thái.

## 1. Vẽ bốn sheet bằng AI

Mỗi sheet là **một ảnh PNG RGBA nền trong suốt**, 4 cột, mỗi hàng một animation 4 frame, ô vuông bằng nhau. Nhân vật quay sang phải ở mọi frame, nằm trọn trong ô của nó, có khoảng trong suốt giữa các pose. Không dùng nền, chữ, ô lưới, đổ bóng hoặc hiệu ứng hạt. Nền trống phải có alpha bằng 0.

| File | Lưới | Hàng (trên xuống) |
|---|---|---|
| `idle-walk-run.png` | 4 × 3 (tỉ lệ 4:3) | `idle`, `walk`, `run` |
| `sleep-react-fall.png` | 4 × 4 (vuông) | `ref`, `sleep`, `react`, `fall` |
| `dragged-land-dizzy.png` | 4 × 4 (vuông) | `ref`, `dragged`, `land`, `dizzy` |
| `climb-perch-jump.png` | 4 × 4 (vuông) | `ref`, `climb`, `perch`, `jump` |

Vẽ `idle-walk-run.png` trước. Ba sheet còn lại vẽ **kèm `idle-walk-run.png` làm ảnh mẫu nhận diện**, để mọi trạng thái cùng nét vẽ, màu, phụ kiện và cỡ đầu. Hàng `ref` chép lại tư thế đứng yên của ảnh mẫu, chỉ dùng để đo cỡ.

Prompt mẫu cho `idle-walk-run.png` (thay phần `Character`):

```text
Original pixel-art sprite sheet for TinyWorld, a desktop pet game.

Output one RGBA PNG with a genuinely transparent alpha background: no white background, checkerboard, text, labels, border, grid, scenery, floor, shadow, particles, watermark, or extra characters.

Layout: EXACTLY 4 columns x 3 rows = 12 equal square cells (4:3 image). One full-body character per cell, centred, the whole body inside its cell with a clear transparent margin. Face RIGHT in every frame. Same size, colors, anatomy and accessories in every frame.

Style: cute compact chibi side-view mascot, crisp flat pixel-art clusters, 12-18 flat colors, clean dark outline, no gradients, blur, anti-aliasing or noise.

Row 1 IDLE: standing calmly, gentle breathing; frame 3 blinks.
Row 2 WALK: 4-phase gait moving right: near leg forward, far leg passing, far leg forward, near leg passing.
Row 3 RUN: same four phases with a longer stride, body leaning forward; legs keep their normal length.

Character: [NAME], an ORIGINAL [SPECIES/CREATURE].
Design: [COLORS, SHAPE, FACE, ACCESSORIES].
Anatomy in EVERY cell: [for example: TWO arms attached at the shoulders and TWO legs]. Never drop, merge, duplicate or float a limb.
```

Prompt cho ba sheet còn lại: xem `prompt.txt` của một nhân vật có sẵn (ví dụ `assets/sprite-sources/gloop/prompt.txt`), đổi phần mô tả nhân vật. Tư thế từng hàng:

- `sleep`: nằm cuộn tròn ngủ, mắt nhắm, thở chậm;
- `react`: giật mình, nhảy nhẹ vui vẻ, cười, trở lại đứng;
- `fall`: đang rơi giữa không trung, tay chân dang ra;
- `dragged`: bị nhấc từ gáy, thân thõng dọc, chân buông xuống;
- `land`: chạm đất nén xuống, đứng lên dần;
- `dizzy`: ngồi choáng, **frame đầu có mắt xoáy** (app chỉ hiện frame này), không vẽ sao;
- `climb`: bám bức tường vô hình bên phải, thân dựng đứng, tay vươn lên, chân đạp tường, đổi tay luân phiên;
- `perch`: ngồi trên mép cửa sổ, chân thõng xuống;
- `jump`: ngồi thụp, bật lên, trên không, chuẩn bị đáp.

Ví dụ phần nhận diện nhân vật:

```text
Character: Nori, an ORIGINAL tiny chubby green turtle developer mascot.
Design: moss-green shell with three pale mint spots, cream belly, tiny round glasses, a small orange scarf, and a gentle smile.
Anatomy in EVERY cell: FOUR short turtle legs and ONE short tail.
```

## 2. Đưa sheet vào dự án

1. Tạo thư mục `assets/sprite-sources/<ten>/` (nên dùng chữ thường, ví dụ `nori`).
2. Lưu bốn sheet đúng tên ở bảng trên, và ghi các prompt đã dùng vào `prompt.txt`. Không lưu thêm bản `-v2`: sinh lại thì ghi đè đúng file đó.
3. Thêm một dòng vào mảng `PETS` trong `scripts/prepare-sprites.mjs`:

   ```js
   { source: "nori", folder: "c-nori", mouthY: 115, name: "Nori — Turtle", height: 135 },
   ```

   - `source`: tên thư mục dưới `assets/sprite-sources/`;
   - `folder`: tên thư mục output dưới `assets/sprites/` (nên đặt tiền tố `c-`);
   - `name`: tên hiển thị trong phần cài đặt;
   - `height`: chiều cao tư thế đứng yên trong khung 192px (các pet hiện có từ khoảng 100 đến 155);
   - `mouthY`: hàng pixel của miệng trong khung 192px, chỗ bong bóng chat chỉ vào.

   Các trường như `reuse`, `recolor`, `fill` chỉ dùng khi cần sửa nhỏ vài frame (xem `assets/sprite-sources/README.md`). Nhân vật mới chưa cần các trường đó.

4. Tạo pack chạy trong app và kiểm tra:

   ```powershell
   node scripts/prepare-sprites.mjs --pet=<ten>
   pnpm test:sprites
   ```

Script sẽ tạo `assets/sprites/c-nori/pet.json` và 12 file WebP (mỗi WebP là một động tác có 4 frame đặt ngang). Không chỉnh trực tiếp các file trong `assets/sprites/`, vì lần chạy script sau sẽ ghi đè chúng.

## 3. Kiểm tra trước khi push

Mở `assets/sprite-sources/index.html` bằng trình duyệt để xem từng animation. Cần kiểm tra đặc biệt:

- mọi frame đủ tay chân, chân không dài ra khi chạy, nhìn nghiêng thì chỉ thấy một mắt;
- `walk` và `run` có bốn dáng chân luân phiên rõ ràng;
- `dragged` có tay/chân nối đúng vào thân và chân đang buông xuống;
- `climb` hướng về bức tường vô hình ở bên phải;
- các frame không bị cắt, không có nền đục, và nhân vật không đổi màu/phụ kiện/cỡ giữa các trạng thái.

Cuối cùng chạy `pnpm test` trước khi đẩy nhánh `test`; CI sẽ kiểm tra lại và build file `.exe`.
