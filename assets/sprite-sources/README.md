# Nhân vật TinyWorld

21 nhân vật được tạo riêng cho dự án bằng công cụ **imagegen tích hợp** ngày 2026-09-28. Bộ
ếch/cóc (Boggo, Gloop, Bẹp, Frobu) dùng atlas Mầm và Rêu làm tham chiếu nét vẽ, tỉ lệ chibi và bố
cục để đồng bộ với các pet có trước. Ảnh nguồn đặt ở đây, ngoài `assets/sprites/`, để Vite không
nhúng atlas lớn vào app.

| Nhân vật | Thiết kế | Pack chạy trong app |
|---|---|---|
| Momo | Axolotl hồng, mang san hô, bụng kem | `../sprites/a-momo/` (mặc định) |
| Bông | Thỏ kem, tai hồng, khăn xanh teal | `../sprites/b-bong/` |
| Kitsu | Cáo cam, mõm/bụng kem, chóp đuôi trắng | `../sprites/b-kitsu/` |
| Mầm | Sinh vật rừng màu kem, hai lá xanh trên đầu | `../sprites/c-mam/` |
| Bíp | Robot xanh, mắt LED vàng, biểu tượng tim trên ngực | `../sprites/c-bip/` |
| Lumi | Tinh linh đầu sao vàng, khăn tím nhạt | `../sprites/c-lumi/` |
| Nấm | Bé nấm mũ đỏ, ba đốm trắng, thân kem | `../sprites/c-nam/` |
| Mây | Tinh linh mây xanh nhạt, chân xanh đậm | `../sprites/c-may/` |
| Tàn | Bé lửa cam, mặt vàng, hình tim trên ngực | `../sprites/c-tan/` |
| Rêu | Rồng lá xanh ngọc, bụng vàng nhạt | `../sprites/c-reu/` |
| Cục | Golem đá tím, tinh thể xanh trên đầu | `../sprites/c-cuc/` |
| Mực | Bạch tuộc tím tròn, xúc tu hồng | `../sprites/c-muc/` |
| Dứa | Bé dứa vàng, ba lá xanh trên đầu | `../sprites/c-dua/` |
| Su | Phi hành gia nhỏ, bộ đồ trắng tím | `../sprites/c-su/` |
| Bắp | Ong vàng tròn, cánh xanh nhạt | `../sprites/c-bap/` |
| Boggo | Ếch coder xanh rêu, mắt mệt, hoodie than, ly cà phê nâu | `../sprites/c-boggo/` |
| Gloop | Ếch nghịch xanh lá, khăn quàng cam gạch | `../sprites/c-gloop/` |
| Bẹp | Cóc lùn xanh nâu, mặt chán đời, túi đeo chéo | `../sprites/c-bep/` |
| Frobu | Ếch thức khuya, hoodie xanh navy, tai nghe tím nhạt | `../sprites/c-frobu/` |
| Byte | Chim cánh cụt coder xanh đen, mỏ/chân vàng, kính tròn, hoodie teal | `../sprites/c-byte/` |
| Patch | Gấu trúc đỏ coder, tai nghe/hoodie tím, đuôi sọc cuộn ngắn | `../sprites/c-patch/` |

## File nguồn

Mỗi thư mục nhân vật có:

| File | Nội dung |
|---|---|
| `atlas.png` | Atlas gốc 724×2172: 4 cột × 12 hàng, mỗi hàng một animation |
| `prompt.txt` | Prompt tạo atlas |
| `locomotion-v2.png`, `locomotion-prompt.txt` | Sheet 4 cột × 3 hàng vẽ lại `idle`, `walk`, `run` trong một lượt để giữ tỉ lệ, màu và phụ kiện |
| `fix-v*.png` | Sheet sửa một số pose: 4 cột, mỗi hàng một animation, các hàng khai báo trong `PETS` |
| `palette.json` | Bảng màu 24 màu của pack, script ghi lại mỗi lần tạo pack |

Riêng vài nhân vật:

- **Cục** dùng `atlas-v2.png` thay cho `atlas.png`, kèm `edit-prompt.txt`: sửa màu tay/tinh thể ở hàng `dragged`.
- **Mực** có `dragged-v2.png` (4 pose xếp 2×2) và `edit-prompt.txt`: xúc tu nối dưới đầu, rủ xuống khi bị nhấc.
- **Momo** có `poses-v3.png` và `poses-prompt.txt` (sửa `idle`, `fall`, `land`, `dizzy`: mắt rõ, bốn
  chân ngắn có bàn chân hồng), cùng `pose-palette.json` là bảng màu cố định để các dải không sửa
  giữ nguyên byte.

Quy ước khi vẽ: quay sang phải; đi/chạy có bốn pha luân phiên (chân gần đặt trước → chân xa vung
qua chân trụ → chân xa đặt trước → chân gần vung qua), lúc chạy sải chân rộng hơn. Hai chân cùng màu,
phân biệt bằng vị trí và gập gối. Tay nối vào vai, không mọc từ đầu. Momo, Kitsu và Rêu bốn chân,
đổi cặp chân chéo.

## Xem trước

Mở `index.html` bằng trình duyệt, chạy thẳng qua `file://`, không cần Tauri: phát/dừng, chọn
animation, chọn nền (có ô caro để soát alpha), đổi tốc độ, xem riêng từng frame.
Trang đọc thẳng ảnh trong `../sprites/` theo `gallery-data.js`, file này do script tạo ra.

## Tạo lại pack

Từ thư mục gốc dự án:

```powershell
node scripts/prepare-sprites.mjs              # tạo lại cả 21 pack và gallery-data.js
node scripts/prepare-sprites.mjs --pet=cuc    # một nhân vật, theo tên thư mục nguồn
node scripts/prepare-sprites.mjs --only-new   # chỉ các pack chưa có pet.json
node scripts/prepare-sprites.mjs --check      # chỉ kiểm tra, không ghi file
pnpm test:sprites                             # test của script + --check (CI chạy qua pnpm test)
```

Script không gọi AI. Atlas và sheet nguồn (PNG) được đọc bằng bộ giải mã PNG viết sẵn trong script;
ảnh pack ghi ra dạng WebP lossless qua [`sharp`](https://sharp.pixelplumbing.com/) (có sẵn sau
`pnpm install`). Với mỗi nhân vật trong `PETS`:

1. Cắt atlas theo khoảng trong suốt giữa 12 hàng và giữa 4 pose của mỗi hàng (ảnh sinh không cách
   đều). Bố cục không rõ đủ hàng/cột, hoặc pose chạm ranh giới cắt, thì báo lỗi. Bỏ alpha < 128 và
   đốm nhiễu rời, giữ các bộ phận rời đủ lớn (tai, tay, ăng-ten).
2. Ghép sheet sửa theo thứ tự `overrides` → `locomotion-v2.png` → `poses`; sheet sau ghi đè sheet
   trước. Sheet sửa được cắt ở độ phân giải của nó rồi thu nhỏ một lần theo tỉ lệ chiều cao `idle`
   (hoặc hàng `ref`) so với atlas, để nhân vật không to lên/nhỏ đi khi đổi state.
3. Pose của atlas chép nguyên 1:1 pixel. Sheet sửa thu nhỏ bằng lấy mẫu theo vùng, màu tối (viền,
   đồng tử, miệng) được ưu tiên để nét mảnh không mất. Pose đặt giữa frame, pixel thấp nhất ở y=179.
4. Giới hạn bảng màu chung 24 màu, xoá đốm lẻ một pixel (không đụng màu tối), làm mịn hàng `dizzy`,
   khép viền ngoài 2px bằng màu viền của pack, rồi áp `reuse`, `recolor`, `fill`. Pose choáng của atlas
   giữ bóng đổ lấm tấm của ảnh sinh ra, mà app giữ nguyên frame đó suốt lúc choáng và cho lảo đảo nên
   trông như nhiễu. Bước làm mịn (lọc Kuwahara) làm phẳng các pixel xen kẽ hai màu gần nhau, giữ viền,
   đồng tử, mắt xoáy (màu tối) và chi tiết tương phản cao như chữ trên áo.
5. Ghi mỗi animation thành dải WebP 768×192 (4 frame 192×192), hoặc một sheet 768×2304 với
   `singleSheet` (Byte, Patch), rồi ghi `pet.json` đủ 12 animation và xoá ảnh cũ mà `pet.json` không
   dùng. Mọi ảnh được encode xong trong bộ nhớ trước khi ghi, nên lỗi giữa chừng không làm hỏng pack cũ.

Thứ tự hàng: `idle`, `walk`, `run`, `sleep`, `react`, `fall`, `dragged`, `land`, `dizzy`, `climb`,
`perch`, `jump`. `react` không lặp, đủ 4 frame trong state 0,5 giây; `land` không lặp, 16 fps để hiện
đủ 4 frame trong 0,25 giây. `dizzy` chỉ dùng frame đầu (`frames: 1`, `fps: 1`), app tự thêm lảo đảo và
sao; cả 4 pose vẫn được lưu trong dải.

`--check` dùng chính `parseSpriteManifest` và `frameRects` của app để kiểm tra manifest, kích thước
ảnh, đủ animation, frame không trống/không bị cắt, alpha nhị phân, bảng màu và hàng chân, rồi so
silhouette từng frame với kết quả cắt lại từ nguồn. Ảnh phải là WebP lossless, và trong pack chỉ có
đúng các ảnh `pet.json` dùng (Vite gom mọi ảnh trong thư mục vào bản build). Pack lệch nguồn làm
`pnpm test` và CI lỗi.

## Sửa pose

Mỗi lượt sinh lại bằng AI có thể làm mất hoặc đổi màu tay chân, và `--check` không bắt được lỗi giải
phẫu. Trước khi nhận sheet mới, soát từng frame so với atlas gốc. Lỗi nhỏ thì sửa bằng tùy chọn trong
`PETS` thay vì sinh lại. Đừng sửa ảnh trong `assets/sprites/`: script sẽ ghi đè.

Thêm một sheet sửa:

1. Sinh sheet nền trong suốt **4 cột**, mỗi hàng một animation cần sửa, ô vuông bằng nhau. Hàng
   đầu nên là **`ref`**: chép lại pose đứng yên, chỉ dùng để đo cỡ, không ghi vào pack.
2. Lưu vào `sprite-sources/<nhân vật>/`, ví dụ `fix-v3.png`.
3. Thêm vào `poses` của nhân vật trong [`scripts/prepare-sprites.mjs`](../../scripts/prepare-sprites.mjs), đúng thứ tự hàng:
   ```js
   poses: [{ image: "fix-v1.png", rows: ["ref", "fall"] }, { image: "fix-v3.png", rows: ["ref", "dragged"] }],
   ```
4. Chạy `node scripts/prepare-sprites.mjs --pet=<nhân vật>`, mở `index.html` để so từng frame.

Sheet mới có hàng `dizzy` thì bỏ `dizzy: <cột>` của nhân vật đó (nếu có), để slot đầu lấy pose mới.

Các tùy chọn của mỗi nhân vật trong `PETS`:

| Tùy chọn | Tác dụng |
|---|---|
| `atlas` | Atlas khác `atlas.png` (Cục) |
| `poses` | Một hoặc nhiều sheet sửa `{ image, rows }` |
| `overrides` | Sheet 2×2 thay một animation, ví dụ `{ dragged: "dragged-v2.png" }` (Mực) |
| `palette` | Bảng màu cố định thay vì tính lại (Momo) |
| `keep` | Hàng giữ từ atlas gốc dù sheet đã vẽ lại, ví dụ Mực giữ `walk`/`run` vì bản vẽ lại mất xúc tu |
| `dizzy` | Cột có pose choáng rõ nhất, được đưa vào slot đầu |
| `reuse` | Thứ tự cột của một hàng để thay frame hỏng bằng frame tốt cùng hàng, ví dụ `{ fall: [0, 3, 0, 3] }` |
| `recolor` | `{ rows, frames?, box?, colors }`: đổi màu trong các hàng, có thể chỉ vài frame và một ô `[trái, trên, phải, dưới]` (toạ độ trong frame 192px). Nhiều luật thì dùng mảng |
| `fill` | `[{ row, frame, at: [x, y], box?, color }]`: tô vùng được viền bao quanh điểm `at` |
| `singleSheet` | Gộp 12 hàng vào một `atlas.webp` trong pack (Byte, Patch) |

Màu trong `recolor` / `fill` được so với màu gần nhất của bảng màu pack (lệch tối đa 24), nên luật
vẫn chạy khi bảng màu xê dịch nhẹ sau khi thêm sheet mới; lệch xa hơn thì script báo lỗi để cập nhật luật.
