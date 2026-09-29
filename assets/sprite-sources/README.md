# TinyWorld original pets

21 nhân vật được tạo riêng cho dự án bằng công cụ **imagegen tích hợp** ngày 2026-09-28.
Bộ ếch/cóc gồm Boggo, Gloop, Bẹp và Frobu dùng atlas Mầm và Rêu làm tham chiếu
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
| Gloop | Ếch nghịch xanh lá, khăn quàng cam gạch | `../sprites/c-gloop/` |
| Bẹp | Cóc lùn xanh nâu, mặt chán đời, túi đeo chéo | `../sprites/c-bep/` |
| Frobu | Ếch thức khuya, hoodie xanh navy, tai nghe tím nhạt | `../sprites/c-frobu/` |
| Byte | Chim cánh cụt coder xanh đen, mỏ/chân vàng, kính tròn, hoodie teal | `../sprites/c-byte/` |
| Patch | Gấu trúc đỏ coder, tai nghe/hoodie tím, đuôi sọc cuộn ngắn | `../sprites/c-patch/` |

Mỗi thư mục nguồn có `atlas.png` (ảnh gốc 724×2172), `prompt.txt` (prompt đầy đủ) và
`palette.json`. Không giữ ảnh xem thử riêng cho từng nhân vật: trang `index.html` đọc thẳng các dải
(hoặc sheet chung của Byte, Patch) trong pack. Ảnh gốc giữ nguyên để có thể xử lý lại.
Nguồn đặt ngoài `assets/sprites/` để Vite không nhúng atlas lớn vào app.
Byte và Patch là hai mascot động vật coder cùng nét vẽ và tỉ lệ pet của repo, cũng dùng một sheet trong mỗi pack.
Riêng Cục dùng `atlas-v2.png` thay cho `atlas.png`, kèm `edit-prompt.txt`: sửa chi tiết màu
tay/tinh thể ở pose bị nhấc để giữ thiết kế đồng nhất. Bản gốc chưa sửa đã bỏ khỏi repo (còn trong lịch sử git).
Bíp có thêm `dragged-v2.png` (4 pose xếp 2×2) và `edit-prompt.txt`: sửa tay giơ lên để
nối vào vai ở thân, thay vì mọc từ đỉnh đầu. Script chỉ thay hàng `dragged` bằng sheet này,
đưa về cùng tỉ lệ pixel nguồn rồi chuẩn hóa chung với các pose còn lại.
Mực cũng có `dragged-v2.png` và `edit-prompt.txt`: các xúc tu nối dưới đầu và rủ xuống
khi bị nhấc, hơi cong và đung đưa, thay tư thế hai tay giơ lên và một chân ở giữa.

## Bước chân và tư thế choáng

Mỗi thư mục của 21 nhân vật có `locomotion-v2.png` và `locomotion-prompt.txt`, chỉnh bằng
**imagegen tích hợp**. Sheet có 4 cột × 3 hàng: `idle`, `walk`, `run`. Ba trạng thái được render lại
chung để giữ tỷ lệ đầu/thân, màu sắc và phụ kiện nhất quán. Nhóm ếch/cóc giữ tay nối vào vai,
bàn tay hiện trên thân và hai chân tách riêng bên dưới bụng. Thứ tự bốn pha đi/chạy là
chân gần đặt phía trước → chân xa vung qua chân trụ → chân xa đặt phía trước → chân gần vung qua.
Các chân dùng cùng màu gốc, chân vung nhấc nhẹ khỏi đất; lúc chạy sải chân rộng hơn.
Momo, Kitsu và Rêu giữ bốn chân, đổi các cặp chân chéo.
Script chỉ thay ba hàng `idle` / `walk` / `run`; các state còn lại giữ nguyên pixel của atlas gốc.
Sheet chỉnh sửa được cắt ở độ phân giải của chính nó rồi thu nhỏ một lần, theo tỉ lệ chiều cao
`idle` của atlas gốc so với `idle` của sheet, nên nhân vật không to lên/nhỏ đi khi chuyển giữa
đi/chạy và ngủ/bị nhấc.
Một số pose sinh lại bị lỗi nên được xử lý riêng trong `PETS` của script:
- Mực: `walk` / `run` bản render lại mất một hai xúc tu, nên giữ hai hàng này của atlas gốc (`keep`).
- Bông: chân xa ở `idle` / `walk` / `run` / `fall` bị tô nâu; script đổi hai màu đó về tông kem của thân (`recolor`).
- Momo: `walk` frame 2 có chân xa màu san hô của mang và mang dưới chỉ còn viền; script tô lại chân
  trong một ô (`recolor` với `frames` + `box`) và tô kín mang (`fill`).
- Frame lệch hẳn so với cả hàng (Rêu/Byte/Mây/Bẹp `fall`, Lumi/Su `dragged`, Rêu `idle`, Dứa `climb`)
  được thay bằng một frame tốt của cùng hàng (`reuse`).

`dizzy` trong manifest chỉ dùng pose choáng đầu tiên (`frames: 1`, `fps: 1`). Với Bông, Su, Bắp và Patch,
pose ở cột 0 của atlas còn mắt bình thường, nên script đưa pose mắt xoáy ở cột 1 vào slot đầu (`dizzy: 1`). Mỗi nhân vật giữ nguyên
một tư thế đứng hoặc ngồi trong toàn bộ thời gian choáng, rồi chuyển về `idle` khi hết choáng.
Hiệu ứng lảo đảo và sao quanh đầu vẫn do app tạo. Bốn pose của dải nguồn vẫn được lưu và kiểm tra,
nhưng các pose đứng/ngồi/tỉnh ở cuối dải không được luân phiên trong state `dizzy` nữa.
Trang xem trước cũng chỉ hiển thị frame được manifest sử dụng.
Lumi giữ đủ năm góc của đầu sao; Nấm có cả hai chân màu kem. Chân trước/sau của từng nhân vật
giữ cùng màu, phân biệt bước đi bằng vị trí và gập gối thay vì dùng hai màu giày khác nhau.

## Xem trước

Momo có thêm `momo/poses-v3.png`, sheet 4 cột × 4 hàng cho `idle`, `fall`, `land`, `dizzy`,
và `momo/poses-prompt.txt` ghi hai lượt chỉnh bằng imagegen tích hợp. Mắt được làm rõ, bốn chân
ngắn có bàn chân hồng tách biệt: đứng đặt chân yên, rơi co nhẹ dưới bụng, tiếp đất gập chân
rồi nâng người, choáng giữ tư thế đứng. Các hàng này ghi đè sau `locomotion-v2.png`.
`momo/pose-palette.json` cố định bảng màu hiện có, giữ nguyên byte của các dải đi/chạy và
các trạng thái không chỉnh. Mọi pose vẫn dùng cùng tỷ lệ của atlas gốc và baseline y=179.

Mở `index.html` bằng trình duyệt. Trang này hoạt động trực tiếp qua `file://`, không cần chạy
Tauri. Có phát/dừng, chọn animation, chọn nền, đổi tốc độ và xem riêng từng frame.
Animation một lần được phát lại sau một khoảng dừng để tiện xem.

## Sinh lại pose

Cách đưa một sheet pose sinh lại vào pack:

1. Sinh một sheet nền trong suốt **4 cột**, mỗi hàng một animation cần sửa, ô vuông bằng nhau.
   Hàng đầu nên là **`ref`**: chép lại pose đứng yên của nhân vật. Script chỉ dùng hàng này để
   đo chiều cao và đưa nhân vật về đúng cỡ của atlas, không ghi nó vào pack.
2. Lưu vào `sprite-sources/<nhân vật>/`, ví dụ `fix-v1.png`.
3. Khai báo trong `PETS` của [`scripts/prepare-sprites.mjs`](../../scripts/prepare-sprites.mjs), theo đúng thứ tự hàng:
   ```js
   { source: "bap", folder: "c-bap", name: "Bắp — Bumblebee", poses: { image: "fix-v1.png", rows: ["ref", "fall", "dragged"] } },
   ```
   Nhiều sheet thì dùng mảng `poses: [{ … }, { … }]`; sheet sau ghi đè sheet trước (Momo đã có `poses-v3.png`).
4. Chạy `node scripts/prepare-sprites.mjs --pet=bap`, rồi mở `index.html` để so từng frame.
   Soát kỹ số tay/chân trước khi nhận sheet: mỗi lượt sinh lại có thể làm mất hoặc đổi màu chi tiết.

Sheet sinh lại có hàng `dizzy` thì bỏ `dizzy: 1` của nhân vật đó (nếu có), để slot 0 lấy đúng pose mới.
Đừng sửa PNG trong `assets/sprites/` vì script sẽ ghi đè. Lỗi nhỏ thì sửa bằng tùy chọn trong `PETS`
thay vì sinh lại, vì mỗi lượt sinh lại có thể làm hỏng chỗ khác:

- `keep: ["walk", "run"]`: giữ hàng đó từ atlas gốc khi bản sinh lại bị hỏng (như Mực).
- `dizzy: <cột>`: pose choáng rõ nhất không nằm ở cột 0.
- `reuse: { fall: [0, 3, 0, 3] }`: thứ tự cột của một hàng, thay frame hỏng bằng frame tốt cùng hàng.
  Bảng màu vẫn tính từ các pose gốc nên không đổi màu các hàng khác.
- `recolor: { rows, frames?, box?, colors }`: đổi màu trong các hàng, có thể chỉ vài frame và một ô
  `[trái, trên, phải, dưới]` (toạ độ trong frame 192px). Có thể là một mảng nhiều luật.
- `fill: [{ row, frame, at: [x, y], box?, color }]`: tô vùng được viền bao quanh điểm `at`.

Màu trong `recolor` / `fill` được so với màu gần nhất của bảng màu pack (lệch tối đa 24), nên luật
vẫn chạy khi bảng màu xê dịch nhẹ sau khi thêm sheet mới; lệch xa hơn thì script báo lỗi để cập nhật.
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
Trang xem thử luôn lấy danh sách đầy đủ. Có 252 animation, tổng cộng 1008 frame được lưu, trong 21 pack;
945 frame được chọn để phát trong gallery vì `dizzy` chỉ dùng một pose trên mỗi nhân vật.
19 pack dùng dải riêng và 2 pack dùng sheet chung, tổng cộng 230 PNG chạy trong app.

Script không gọi AI và không cần cài thư viện ảnh. Nó tìm khoảng trong suốt giữa 12 hàng
và giữa 4 pose của từng hàng để cắt atlas, vì ảnh nguồn có hàng/cột không cách đều.
Pose chạm ranh giới cắt sẽ bị báo lỗi trước khi thu nhỏ; bố cục không rõ đủ hàng/cột cũng
bị từ chối. Script bỏ alpha nhỏ hơn 128 và điểm nhiễu rời nhỏ, giữ các bộ phận rời đủ lớn.
Pose của atlas gốc được chép nguyên 1:1 pixel khi pose lớn nhất vừa khung (đúng với cả 21 pack).
Trước đây script thu/phóng cả pack về 160px bằng nearest-neighbor: hệ số lẻ như 0.91 bỏ mất
khoảng một hàng pixel trong mười, làm đứt viền 1px và mảnh tay/chân. Sheet chỉnh sửa (lớn hơn atlas)
được thu nhỏ bằng lấy mẫu theo vùng: pixel đục khi ít nhất nửa vùng nguồn đục, lấy màu chiếm ưu thế,
màu tối (viền, đồng tử, miệng) được tính gấp đôi để nét mảnh không biến mất.
Sau đó script giới hạn bảng màu chung 24 màu, xóa đốm lẻ một pixel do bóng đổ mềm của ảnh sinh
(không đụng màu tối), và khép viền ngoài dày 2px bằng màu viền của chính pack ở những chỗ viền gốc
bị mờ/mất, để mọi pose có viền liền như sheet đi/chạy. Căn pixel thấp nhất về y=179.
Không nội suy mờ, không kéo giãn riêng từng pose. Ảnh đầu ra có alpha
0 hoặc 255; mỗi dải 768×192 gồm 4 frame 192×192. Byte và Patch gộp 12 hàng vào một sheet
768×2304 và dùng `row` trong manifest. `anchor` là `(96,180)`, `scale: 0.5`.
Khung mặc định vẫn là 96×96 CSS pixel; ở cỡ 200% khung là 192×192 và vẽ ảnh ở tỉ lệ 1:1 theo CSS pixel.
Pose gốc có khoảng 140–175 pixel ở chiều lớn nhất; tăng số pixel đầu ra quá mức này sẽ không
tự thêm chi tiết.

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
