# Tạo nhân vật mới cho TinyWorld

Mỗi nhân vật không phải GIF: đó là sprite atlas được tách thành 12 animation WebP để app tự phát theo trạng thái.

## 1. Tạo atlas bằng AI

Tạo **một ảnh PNG RGBA nền trong suốt** gồm đúng 4 cột x 12 hàng (48 pose). Mỗi hàng có 4 frame theo đúng thứ tự:

`idle`, `walk`, `run`, `sleep`, `react`, `fall`, `dragged`, `land`, `dizzy`, `climb`, `perch`, `jump`.

Nên dùng canvas 768 x 2304 px (tỉ lệ 1:3), nhân vật quay sang phải ở mọi frame, cùng kích thước và có chân chạm cùng một đường baseline. Không dùng nền, chữ, ô lưới, đổ bóng hoặc hiệu ứng hạt. Nền trống phải có alpha bằng 0.

Prompt mẫu (thay phần `Subject identity`):

```text
Create an original pixel-art desktop pet sprite atlas for TinyWorld.

Output one RGBA PNG with genuinely transparent alpha background: no white background, checkerboard, text, labels, border, grid, scenery, floor, shadow, particles, watermark, or extra characters.

Canvas layout: EXACTLY 4 equal columns x 12 equal rows, 48 isolated poses, tall 1:3 aspect ratio, preferably 768x2304 px. Every cell is square; character fully inside its own cell with transparent margins. Face RIGHT in every frame. Keep the same character size, colors, anatomy, markings, and accessories across all 48 poses.

Style: cute compact chibi side-view non-human mascot, crisp hand-pixelled retro game sprite. Use 12-18 flat solid colors, a one-pixel dark warm outline, no gradients, blur, anti-aliasing, 3D rendering, or realism. Feet/contact baseline must align across frames.

Rows from top to bottom; each row has exactly four left-to-right frames:
1. idle: breathe, blink, recover
2. walk: clear alternating four-step walk cycle
3. run: clear energetic alternating four-step run cycle
4. sleep: curled/resting, subtle breathing
5. react: squat, happy surprised reaction, recover
6. fall: startled falling with tucked/extended feet
7. dragged: both arms raised from shoulders, legs dangling
8. land: impact squash, gradual recovery
9. dizzy: unsteady with spiral/narrowed eyes, no stars
10. climb: climb an invisible wall on the RIGHT
11. perch: sit on an invisible window ledge
12. jump: crouch, spring, airborne, recover

Subject identity: [NAME], an ORIGINAL [SPECIES/CREATURE].
Design: [COLORS, SHAPE, FACE, ACCESSORIES].
Personality: [for example: curious and cheerful].
Keep all identity details consistent in every pose. No franchise inspiration.
```

Ví dụ phần nhận diện nhân vật:

```text
Subject identity: Nori, an ORIGINAL tiny chubby green turtle developer mascot.
Design: moss-green shell with three pale mint spots, cream belly, tiny round glasses, a small orange scarf, four short turtle legs, and a gentle smile.
Personality: calm, curious, slightly sleepy.
```

## 2. Đưa atlas vào dự án

1. Tạo thư mục `assets/sprite-sources/<ten>/` (nên dùng chữ thường, ví dụ `nori`).
2. Lưu ảnh nguồn thành `assets/sprite-sources/<ten>/atlas.png`.
3. Thêm một dòng vào mảng `PETS` trong `scripts/prepare-sprites.mjs`. Với atlas chuẩn chỉ cần ba trường sau:

   ```js
   { source: "long", folder: "c-long", name: "Long — Eastern Dragon" },
   ```

   - `source`: tên thư mục dưới `assets/sprite-sources/`;
   - `folder`: tên thư mục output dưới `assets/sprites/` (nên đặt tiền tố `c-`);
   - `name`: tên hiển thị trong phần cài đặt.

   Các trường như `poses`, `overrides`, `reuse` chỉ dùng khi cần thay hoặc sửa riêng một vài hàng của atlas. Nhân vật mới chưa cần các trường đó.

4. Tạo pack chạy trong app và kiểm tra:

   ```powershell
   node scripts/prepare-sprites.mjs --pet=<ten>
   pnpm test:sprites
   ```

Script sẽ tạo `assets/sprites/c-long/pet.json` và 12 file WebP (mỗi WebP là một động tác có 4 frame đặt ngang). Không chỉnh trực tiếp các file trong `assets/sprites/`, vì lần chạy script sau sẽ ghi đè chúng.

## 3. Kiểm tra trước khi push

Mở `assets/sprite-sources/index.html` bằng trình duyệt để xem từng animation. Cần kiểm tra đặc biệt:

- `walk` và `run` có bốn dáng chân luân phiên rõ ràng;
- `dragged` có tay/chân nối đúng vào thân và chân đang buông xuống;
- `climb` hướng về bức tường vô hình ở bên phải;
- các frame không bị cắt, không có nền đục, và nhân vật không đổi màu/phụ kiện giữa các pose.

Cuối cùng chạy `pnpm test` trước khi đẩy nhánh `test`; CI sẽ kiểm tra lại và build file `.exe`.
