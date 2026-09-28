# TinyWorld original pets

22 nhân vật được tạo riêng cho dự án bằng công cụ **imagegen tích hợp** ngày 2026-09-28.
Bộ ếch/cóc gồm Boggo, Wobi, Gloop, Bẹp và Frobu dùng atlas Mầm và Rêu làm tham chiếu
nét vẽ, tỉ lệ chibi và bố cục 4 cột × 12 hàng để đồng bộ với các pet có sẵn.
Không dùng sprite mèo OboroPixel làm ảnh tham chiếu hoặc nguồn để chỉnh sửa.

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
| Wobi | Ếch hề xanh, mũ tím-vàng lệch, mũi đỏ, cổ bèo | `../sprites/c-wobi/` |
| Gloop | Ếch nghịch xanh lá, khăn quàng cam gạch | `../sprites/c-gloop/` |
| Bẹp | Cóc lùn xanh nâu, mặt chán đời, túi đeo chéo | `../sprites/c-bep/` |
| Frobu | Ếch thức khuya, hoodie xanh navy, tai nghe tím nhạt | `../sprites/c-frobu/` |
| Byte | Chim cánh cụt coder xanh đen, mỏ/chân vàng, kính tròn, hoodie teal | `../sprites/c-byte/` |
| Patch | Gấu trúc đỏ coder, tai nghe/hoodie tím, đuôi sọc cuộn ngắn | `../sprites/c-patch/` |

Mỗi thư mục nguồn có `atlas.png` (ảnh gốc 724×2172), `prompt.txt` (prompt đầy đủ) và
`palette.json`. Không giữ ảnh xem thử riêng cho từng nhân vật: trang `index.html` đọc thẳng các dải
(hoặc sheet chung của Wobi, Byte, Patch) trong pack. Ảnh gốc giữ nguyên để có thể xử lý lại.
Nguồn đặt ngoài `assets/sprites/` để Vite không nhúng atlas lớn vào app.
Wobi chỉ giữ một ảnh nguồn `atlas.png`: thiết kế ếch hề nguyên bản, mũ tím-vàng, mũi đỏ và cổ bèo
(bản được chọn trước đây là `atlas-v2.png`). Các bản đầu, bản nháp và preview trùng đã được dọn khỏi repo;
`prompt.txt` ghi prompt của thiết kế đang dùng. Pack Wobi cũng chỉ có một `atlas.png` cho toàn bộ animation.
Byte và Patch là hai mascot động vật coder cùng nét vẽ và tỉ lệ pet của repo, cũng dùng một sheet trong mỗi pack.
Riêng Cục dùng `atlas-v2.png` thay cho `atlas.png`, kèm `edit-prompt.txt`: sửa chi tiết màu
tay/tinh thể ở pose bị nhấc để giữ thiết kế đồng nhất. Bản gốc chưa sửa đã bỏ khỏi repo (còn trong lịch sử git).
Bíp có thêm `dragged-v2.png` (4 pose xếp 2×2) và `edit-prompt.txt`: sửa tay giơ lên để
nối vào vai ở thân, thay vì mọc từ đỉnh đầu. Script chỉ thay hàng `dragged` bằng sheet này,
đưa về cùng tỉ lệ pixel nguồn rồi chuẩn hóa chung với các pose còn lại.

## Xem trước

Mở `index.html` bằng trình duyệt. Trang này hoạt động trực tiếp qua `file://`, không cần chạy
Tauri. Có phát/dừng, chọn animation, chọn nền, đổi tốc độ và xem riêng từng frame.
Animation một lần được phát lại sau một khoảng dừng để tiện xem.
Ảnh xem nhanh trên [nền sáng](preview-light.png) và [nền tối](preview-dark.png).
Pose bị nhấc sau khi sửa: [nền sáng](dragged-preview-light.png), [nền tối](dragged-preview-dark.png).

## Tạo lại các dải ảnh

Từ thư mục gốc dự án:

```powershell
node scripts/prepare-sprites.mjs
node scripts/prepare-sprites.mjs --check
node scripts/prepare-sprites.mjs --only-new
node scripts/prepare-sprites.mjs --pet=cuc
pnpm test:sprites
pnpm test:overlay
```

`--only-new` chỉ chuẩn hóa pack chưa có `pet.json`; `--pet=<source>` xử lý một nhân vật.
Trang xem thử luôn lấy danh sách đầy đủ. Có 264 animation, tổng cộng 1056 frame, trong 22 pack;
19 pack dùng dải riêng và 3 pack dùng sheet chung, tổng cộng 231 PNG chạy trong app.

Script không gọi AI và không cần cài thư viện ảnh. Nó tìm khoảng trong suốt giữa 12 hàng
và giữa 4 pose của từng hàng để cắt atlas, vì ảnh nguồn có hàng/cột không cách đều.
Pose chạm ranh giới cắt sẽ bị báo lỗi trước khi thu nhỏ; bố cục không rõ đủ hàng/cột cũng
bị từ chối. Script bỏ alpha nhỏ hơn 128 và điểm nhiễu rời nhỏ, giữ các bộ phận rời đủ lớn,
dùng nearest-neighbor về lưới pixel, dùng chung một hệ số
thu nhỏ cho cả pack, giới hạn bảng màu chung 24 màu và căn pixel thấp nhất về y=44.
Không thêm pixel, không nội suy mờ, không kéo giãn riêng từng pose. Ảnh đầu ra có alpha
0 hoặc 255; mỗi dải 192×48 gồm 4 frame 48×48. Wobi, Byte và Patch gộp 12 hàng vào một sheet
192×576 và dùng `row` trong manifest. `anchor` là `(24,45)`.

Thứ tự hàng: `idle`, `walk`, `run`, `sleep`, `react`, `fall`, `dragged`, `land`, `dizzy`,
`climb`, `perch`, `jump`. Tất cả quay sang phải. Mỗi nhân vật đã có viền tối trong ảnh nên
`outline: false`, tránh chồng thêm viền. `react` không lặp, đủ 4 frame trong state 0.5 giây;
`land` không lặp ở 16 fps để hiện đủ 4 frame trước khi state kết thúc sau 0.25 giây.

Chỉ 9 animation Phase 1 được ghi vào `pet.json`. Ba animation Phase 2 ở `phase2/` hoặc các hàng
9–11 của sheet chung, chưa khai báo vào manifest vì engine hiện chưa nhận tên đó.

Lệnh `--check` dùng chính `parseSpriteManifest` và `frameRects` của app để kiểm tra manifest,
kích thước dải/sheet, đúng hàng trong manifest, đủ animation, frame không trống/không bị cắt, alpha nhị phân, bảng màu chung
và hàng chân. Nó cũng so từng silhouette đầu ra với pose đầy đủ được cắt lại từ nguồn,
để bắt các pack cũ mất đầu/phụ kiện dù vẫn có kích thước và alpha hợp lệ.
`pnpm test` (và CI) chạy `--check` qua `pnpm test:sprites`, nên pack bị cắt mất phần nào sẽ làm build lỗi.
Test hồi quy bao gồm hàng/cột lệch, nhiễu, bộ phận rời, pose chạm ranh giới và mất bắt chuột
khi kéo. Các kiểm tra này không chứng minh thử nghiệm RAM 8 tiếng đã hoàn tất.
