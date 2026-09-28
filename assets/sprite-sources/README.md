# TinyWorld original pets

15 nhân vật được tạo riêng cho dự án bằng công cụ **imagegen tích hợp** ngày 2026-09-28:
ba pet động vật và 12 nhân vật hoạt hình tự thiết kế.
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

Mỗi thư mục nguồn có `atlas.png` (ảnh gốc 724×2172), `prompt.txt` (prompt đầy đủ),
`palette.json` và `preview.png` (sheet đã chuẩn hóa). Ảnh gốc giữ nguyên để có thể xử lý lại.
Nguồn đặt ngoài `assets/sprites/` để Vite không nhúng atlas lớn và ảnh xem thử vào app.
Riêng Cục có thêm `atlas-v2.png` và `edit-prompt.txt`: sửa chi tiết màu tay/tinh thể ở pose
bị nhấc để giữ thiết kế đồng nhất. Script chọn bản v2; bản gốc vẫn được giữ lại.
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
Trang xem thử luôn lấy danh sách đầy đủ. Có 180 dải ảnh, tổng cộng 720 frame, trong 15 pack.

Script không gọi AI và không cần cài thư viện ảnh. Nó tìm khoảng trong suốt giữa 12 hàng
và giữa 4 pose của từng hàng để cắt atlas, vì ảnh nguồn có hàng/cột không cách đều.
Pose chạm ranh giới cắt sẽ bị báo lỗi trước khi thu nhỏ; bố cục không rõ đủ hàng/cột cũng
bị từ chối. Script bỏ alpha nhỏ hơn 128 và điểm nhiễu rời nhỏ, giữ các bộ phận rời đủ lớn,
dùng nearest-neighbor về lưới pixel, dùng chung một hệ số
thu nhỏ cho cả pack, giới hạn bảng màu chung 24 màu và căn pixel thấp nhất về y=44.
Không thêm pixel, không nội suy mờ, không kéo giãn riêng từng pose. Ảnh đầu ra có alpha
0 hoặc 255; mỗi dải 192×48 gồm 4 frame 48×48. `anchor` là `(24,45)`.

Thứ tự hàng: `idle`, `walk`, `run`, `sleep`, `react`, `fall`, `dragged`, `land`, `dizzy`,
`climb`, `perch`, `jump`. Tất cả quay sang phải. Mỗi nhân vật đã có viền tối trong ảnh nên
`outline: false`, tránh chồng thêm viền. `react` không lặp, đủ 4 frame trong state 0.5 giây;
`land` không lặp ở 16 fps để hiện đủ 4 frame trước khi state kết thúc sau 0.25 giây.

Chỉ 9 animation Phase 1 được ghi vào `pet.json`. Ba dải Phase 2 ở `phase2/`, chưa khai báo
vào manifest vì engine hiện chưa nhận tên đó.

Lệnh `--check` dùng chính `parseSpriteManifest` và `frameRects` của app để kiểm tra manifest,
kích thước dải, đủ animation, frame không trống/không bị cắt, alpha nhị phân, bảng màu chung
và hàng chân. Nó cũng so từng silhouette đầu ra với pose đầy đủ được cắt lại từ nguồn,
để bắt các pack cũ mất đầu/phụ kiện dù vẫn có kích thước và alpha hợp lệ.
Test hồi quy bao gồm hàng/cột lệch, nhiễu, bộ phận rời, pose chạm ranh giới và mất bắt chuột
khi kéo. Các kiểm tra này không chứng minh thử nghiệm RAM 8 tiếng đã hoàn tất.
