# Nhân vật TinyWorld

21 nhân vật đầu được tạo riêng cho dự án bằng công cụ **imagegen tích hợp** ngày 2026-09-28; Mướp, Bơ, Nhím, Vịt,
Quác, Trôi, Lu, Tèo, Cú, Len, Nai được thêm ngày 2026-11-11 (cách tạo nhân vật mới: [CREATE_PET.md](../../CREATE_PET.md)). Ngày 2026-11-11 các
tư thế được vẽ lại theo một bố cục chung: mỗi nhân vật có đúng bốn sheet, vẽ theo hàng `idle`/`walk`/`run`
làm mẫu nhận diện, để mọi trạng thái cùng nét vẽ, cùng cỡ và đủ tay chân. Ảnh nguồn đặt ở đây, ngoài
`assets/sprites/`, để Vite không nhúng ảnh lớn vào app.

| Nhân vật | Thiết kế | Pack chạy trong app |
|---|---|---|
| Boggo | Ếch coder xanh rêu, mắt mệt, hoodie than, ly cà phê nâu | `../sprites/c-boggo/` (mặc định) |
| Momo | Axolotl hồng, mang san hô, bụng kem | `../sprites/a-momo/` |
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
| Mực | Bạch tuộc tím tròn, năm xúc tu hồng | `../sprites/c-muc/` |
| Dứa | Bé dứa vàng, ba lá xanh trên đầu | `../sprites/c-dua/` |
| Su | Phi hành gia nhỏ, bộ đồ trắng tím | `../sprites/c-su/` |
| Bắp | Ong vàng tròn, cánh xanh nhạt | `../sprites/c-bap/` |
| Gloop | Ếch nghịch xanh lá, khăn quàng cam gạch | `../sprites/c-gloop/` |
| Bẹp | Cóc lùn xanh nâu, mặt chán đời, túi đeo chéo | `../sprites/c-bep/` |
| Frobu | Ếch thức khuya, hoodie xanh navy, tai nghe tím nhạt | `../sprites/c-frobu/` |
| Byte | Chim cánh cụt coder xanh đen, mỏ/chân vàng, kính tròn, hoodie teal | `../sprites/c-byte/` |
| Patch | Gấu trúc đỏ coder, tai nghe/hoodie tím, đuôi sọc cuộn ngắn | `../sprites/c-patch/` |
| Mướp | Mèo mướp xám vằn đậm, mắt xanh lá, vòng cổ đỏ có chuông vàng | `../sprites/c-muop/` |
| Bơ | Chuột lang nước nâu caramel, mắt lim dim, quả quýt trên đầu | `../sprites/c-bo/` |
| Nhím | Nhím gai nâu hạt dẻ, mặt/bụng kem, quả táo đỏ trên lưng | `../sprites/c-nhim/` |
| Vịt | Vịt con vàng chanh, mỏ/chân cam, mũ tai bèo xanh da trời | `../sprites/c-vit/` |
| Quác | Ngỗng trắng hề, mắt trợn, cổ áo xếp nếp đỏ/vàng/xanh, mũ chóp chấm bi | `../sprites/c-quac/` |
| Trôi | Viên bánh trôi trắng, mắt chấm, má hồng, hạt vừng trên đầu, hay há miệng hét vui | `../sprites/c-troi/` |
| Lu | Chó shiba cam kem, mặt liếc xéo tự mãn, kính râm đẩy lên trán | `../sprites/c-lu/` |
| Tèo | Gấu mèo xám xanh, mặt nạ đen, cười nhe răng, vương miện giấy vàng | `../sprites/c-teo/` |
| Cú | Cú nâu ấm, đĩa mặt kem, mắt hổ phách, chỏm tai, nơ xanh rêu | `../sprites/c-cu/` |
| Len | Cừu non lông xoăn kem, mặt/chân xám than, nơ xanh mòng két trên tai | `../sprites/c-len/` |
| Nai | Nai con nâu vàng, đốm trắng trên lưng, sừng nhú, bông cúc trắng sau tai | `../sprites/c-nai/` |

Nhân vật mặc định (chưa chọn gì, hoặc pack đã chọn không còn) là Boggo: hằng `DEFAULT_PACK` trong
`native/src/sprites.rs` và `desktop/src/sprites/sprites.ts`, `--check` so cả hai.

## File nguồn

Mọi thư mục nhân vật có cùng một bộ file, không giữ bản cũ hay bản đánh số:

| File | Nội dung |
|---|---|
| `idle-walk-run.png` | 4 cột × 3 hàng: `idle`, `walk`, `run`. Đây cũng là mẫu nhận diện khi vẽ các sheet còn lại |
| `sleep-react-fall.png` | 4 cột × 4 hàng: `ref`, `sleep`, `react`, `fall` |
| `dragged-land-dizzy.png` | 4 cột × 4 hàng: `ref`, `dragged`, `land`, `dizzy` |
| `climb-perch-jump.png` | 4 cột × 4 hàng: `ref`, `climb`, `perch`, `jump` |
| `prompt.txt` | Prompt đã dùng cho từng sheet |
| `palette.json` | Bảng màu 24 màu của pack, script ghi lại mỗi lần tạo pack |

Hàng `ref` chép lại tư thế đứng yên, chỉ dùng để đo cỡ, không xuất vào pack. Ô của mỗi sheet là hình
vuông, nền trong suốt; các pose cách nhau bằng khoảng trong suốt (script cắt theo khoảng trống, không
cần lưới đều).

Quy ước khi vẽ: quay sang phải; đi/chạy có bốn pha luân phiên (chân gần đặt trước → chân xa vung
qua chân trụ → chân xa đặt trước → chân gần vung qua), lúc chạy sải chân rộng hơn nhưng chân giữ độ dài
bình thường. Tay nối vào vai, không mọc từ đầu. Chân bị che phía xa tô tối hơn một chút nhưng vẫn thấy.
Momo, Kitsu và Rêu bốn chân; Mực có năm xúc tu, không tay chân. `climb` bám bức tường vô hình bên
phải (thân dựng đứng, tay vươn lên, chân đạp vào tường), `perch` ngồi trên mép cửa sổ thõng chân, `dizzy`
có mắt xoáy ở frame đầu (app chỉ hiện frame này, tự thêm lảo đảo và sao).

## Xem trước

Mở `index.html` bằng trình duyệt, chạy thẳng qua `file://`, không cần Tauri: phát/dừng, chọn
animation, chọn nền (có ô caro để soát alpha), đổi tốc độ, xem riêng từng frame.
Trang đọc thẳng ảnh trong `../sprites/` theo `gallery-data.js`, file này do script tạo ra.

## Tạo lại pack

Từ thư mục gốc dự án:

```powershell
node scripts/prepare-sprites.mjs              # tạo lại cả 32 pack và gallery-data.js
node scripts/prepare-sprites.mjs --pet=gloop  # một nhân vật, theo tên thư mục nguồn
node scripts/prepare-sprites.mjs --only-new   # chỉ các pack chưa có pet.json
node scripts/prepare-sprites.mjs --check      # chỉ kiểm tra, không ghi file
pnpm test:sprites                             # test của script + --check (CI chạy qua pnpm test)
```

Script không gọi AI. Sheet nguồn (PNG) được đọc bằng bộ giải mã PNG viết sẵn trong script; ảnh pack ghi
ra dạng WebP lossless qua [`sharp`](https://sharp.pixelplumbing.com/) (có sẵn sau `pnpm install`). Với
mỗi nhân vật trong `PETS`:

1. Cắt từng sheet theo khoảng trong suốt giữa các hàng và giữa 4 pose của mỗi hàng. Bố cục không rõ đủ
   hàng/cột, hoặc pose chạm ranh giới cắt, thì báo lỗi. Bỏ alpha < 128 và đốm nhiễu rời, giữ các bộ phận
   rời đủ lớn (tai, tay, ăng-ten).
2. Thu nhỏ mỗi pose một lần, theo tỉ lệ `height` (chiều cao tư thế đứng yên trong khung 192px) chia cho
   chiều cao hàng `idle`/`ref` của chính sheet đó, nên nhân vật không to lên/nhỏ đi khi đổi trạng thái.
   Pose nào vượt khung thì cả pack cùng thu nhỏ thêm. Lấy mẫu theo vùng, màu tối (viền, đồng tử, miệng)
   được ưu tiên để nét mảnh không mất; tỉ lệ đúng 1 là chép nguyên pixel. Pose đặt giữa frame, pixel thấp
   nhất ở y=179.
3. Giới hạn bảng màu chung 24 màu (hoặc dùng bảng `palette` cố định), xoá đốm lẻ một pixel (không đụng màu
   tối), làm mịn hàng `dizzy` (lọc Kuwahara: app giữ nguyên frame đó suốt lúc choáng và cho lảo đảo, nên
   bóng lấm tấm trông như nhiễu), khép viền ngoài 2px bằng màu viền của pack, rồi áp `reuse`, `recolor`,
   `fill` nếu có.
4. Ghi mỗi animation thành dải WebP 768×192 (4 frame 192×192), hoặc một sheet 768×2304 với
   `singleSheet` (Byte, Patch), rồi ghi `pet.json` đủ 12 animation và xoá ảnh cũ mà `pet.json` không
   dùng. Mọi ảnh được encode xong trong bộ nhớ trước khi ghi, nên lỗi giữa chừng không làm hỏng pack cũ.

Thứ tự hàng: `idle`, `walk`, `run`, `sleep`, `react`, `fall`, `dragged`, `land`, `dizzy`, `climb`,
`perch`, `jump`. `react` không lặp, đủ 4 frame trong state 0,5 giây; `land` không lặp, 16 fps để hiện
đủ 4 frame trong 0,25 giây. `dizzy` chỉ dùng frame đầu (`frames: 1`, `fps: 1`), cả 4 pose vẫn được lưu.

`--check` dùng chính `parseSpriteManifest` và `frameRects` của app để kiểm tra manifest, kích thước
ảnh, đủ animation, frame không trống/không bị cắt, alpha nhị phân, bảng màu và hàng chân, rồi so
silhouette từng frame với kết quả cắt lại từ nguồn. Ảnh phải là WebP lossless, và trong pack chỉ có
đúng các ảnh `pet.json` dùng (Vite gom mọi ảnh trong thư mục vào bản build). Pack lệch nguồn làm
`pnpm test` và CI lỗi. Test còn kiểm tra mỗi thư mục nguồn chỉ có đúng bộ file ở trên.

## Sửa pose

Mỗi lượt sinh lại bằng AI có thể làm mất hoặc đổi màu tay chân, và `--check` không bắt được lỗi giải
phẫu. Trước khi nhận sheet mới, soát từng frame (phóng to) xem đủ tay chân, đúng màu, đúng một mắt khi
nhìn nghiêng. Đừng sửa ảnh trong `assets/sprites/`: script sẽ ghi đè.

Sửa một nhóm tư thế:

1. Sinh lại cả sheet đó (prompt mẫu trong `prompt.txt` của nhân vật), kèm `idle-walk-run.png` làm ảnh
   mẫu nhận diện. Sheet phải giữ đúng 4 cột, hàng `ref` đầu tiên và thứ tự hàng như bảng trên.
2. Ghi đè đúng tên file cũ (ví dụ `climb-perch-jump.png`), không lưu thêm bản `-v2`; cập nhật prompt trong
   `prompt.txt`.
3. Chạy `node scripts/prepare-sprites.mjs --pet=<nhân vật>`, mở `index.html` để so từng frame.

Lỗi nhỏ thì sửa bằng tùy chọn trong `PETS` thay vì sinh lại. Các trường của mỗi nhân vật:

| Trường | Tác dụng |
|---|---|
| `source`, `folder`, `name`, `mouthY` | Thư mục nguồn, thư mục pack, tên hiển thị, hàng miệng (bong bóng chat) |
| `height` | Chiều cao tư thế đứng yên trong khung 192px; mọi sheet được thu về cỡ này |
| `palette` | Bảng màu cố định thay vì tính lại, để các hàng không đổi giữ nguyên từng byte (Boggo) |
| `singleSheet` | Gộp 12 hàng vào một `atlas.webp` trong pack (Byte, Patch) |
| `dizzy` | Cột có pose choáng rõ nhất, được đưa vào slot đầu |
| `reuse` | Thứ tự cột của một hàng để thay frame hỏng bằng frame tốt cùng hàng, ví dụ `{ fall: [0, 3, 0, 3] }` |
| `recolor` | `{ rows, frames?, box?, colors }`: đổi màu trong các hàng, có thể chỉ vài frame và một ô `[trái, trên, phải, dưới]` (toạ độ trong frame 192px). Nhiều luật thì dùng mảng |
| `fill` | `[{ row, frame, at: [x, y], box?, color }]`: tô vùng được viền bao quanh điểm `at` |

Màu trong `recolor` / `fill` được so với màu gần nhất của bảng màu pack (lệch tối đa 24), nên luật
vẫn chạy khi bảng màu xê dịch nhẹ sau khi thêm sheet mới; lệch xa hơn thì script báo lỗi để cập nhật luật.
