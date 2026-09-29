# Assets

Thư mục này chứa **file gốc** của logo và hình nhân vật. Không sửa trực tiếp icon hay sprite đã tạo
ra; thay file gốc ở đây rồi chạy lại lệnh tạo.

| File | Dùng cho |
|---|---|
| `icon.png` | Logo TinyWorld nền trong suốt: pet màu kem ôm hành tinh xanh, có mầm cây; dùng cho app, tray, setup và portable |
| `branding/logo-prompt.txt` | Prompt gốc để tạo logo bằng imagegen |
| `sprites/<pack>/` | Sprite pack chạy trong app: `pet.json` + ảnh PNG, do `scripts/prepare-sprites.mjs` tạo ra |
| `sprite-sources/` | Atlas, prompt gốc của 21 nhân vật và trang xem animation ([README](sprite-sources/README.md)) |

## Sprite pack

Mỗi pack là một thư mục trong `sprites/`, có file `pet.json` mô tả các animation. Người dùng chọn
nhân vật trong tray → **Cài đặt…** → **Nhân vật** (lưu tên thư mục vào `settings.json`). Chưa chọn,
hoặc pack đã chọn không còn, thì dùng pack **đầu tiên theo tên thư mục** (`a-momo`). Pack có
`pet.json` lỗi không hiện trong danh sách; chưa có pack nào (hoặc pack lỗi khi nạp) thì dùng pet tạm
(cục blob 16×16 vẽ bằng code) và in lý do lỗi ra console (tray → **Mở DevTools** khi chạy dev).

```
assets/sprites/a-momo/
├── pet.json
├── idle.png
├── walk.png
└── ...              # đủ 12 animation (cả climb, perch, jump), mỗi file một dải 4 frame
```

21 pack có sẵn đều do script tạo từ atlas trong `sprite-sources/`: mỗi animation 4 frame
**192×192**, alpha 0 hoặc 255, viền tối vẽ sẵn trong ảnh, chân cùng hàng y=179, quay sang phải.
`scale: 0.5` cho khung 96×96 CSS pixel ở cỡ 100%; ở cỡ 200% một pixel ảnh là một CSS pixel. Byte
và Patch gộp cả 12 hàng vào một `atlas.png` 768×2304 và dùng `row` trong manifest.

## Thêm nhân vật

- **Cùng kiểu với các nhân vật có sẵn** (nên dùng): đặt atlas 4 cột × 12 hàng vào
  `sprite-sources/<tên>/`, khai báo trong `PETS` của `scripts/prepare-sprites.mjs` rồi chạy script.
  Chi tiết: [sprite-sources/README.md](sprite-sources/README.md).
- **Pack tự làm**: tạo `sprites/<tên-pack>/` gồm các PNG và `pet.json` (xem bên dưới). Pack này
  không qua script nên `--check` không kiểm tra. App vẽ nguyên ảnh, không thêm viền, nên nhân vật
  màu sáng cần có viền tối vẽ sẵn để không chìm vào nền sáng.

Vite gom pack vào bản build, nên thêm hoặc sửa pack chỉ cần chạy lại `pnpm dev:desktop`.

## `pet.json`

Ví dụ rút gọn từ `a-momo`, mỗi animation một file PNG dải ngang:

```json
{
  "name": "Momo — Axolotl",
  "frameWidth": 192,
  "frameHeight": 192,
  "scale": 0.5,
  "anchor": { "x": 96, "y": 180 },
  "animations": {
    "idle": { "image": "idle.png", "frames": 4, "fps": 5 },
    "walk": { "image": "walk.png", "frames": 4, "fps": 8 },
    "react": { "image": "react.png", "frames": 4, "fps": 8, "loop": false },
    "dizzy": { "image": "dizzy.png", "frames": 1, "fps": 1 }
  }
}
```

Nhiều animation chung một sheet thì dùng `row` (như Byte và Patch):

```json
"walk": { "image": "atlas.png", "row": 1, "frames": 4, "fps": 8 }
```

| Trường | Bắt buộc | Mặc định | Ý nghĩa |
|---|---|---|---|
| `frameWidth`, `frameHeight` | có | | Kích thước một frame (pixel của ảnh gốc) |
| `name` | | `"pet"` | Tên hiển thị |
| `scale` | | `2` | Tỉ lệ vẽ ở cỡ 100%: frame 192×192 với scale 0.5 thành 96×96 CSS pixel |
| `pixelArt` | | `true` | Phóng to kiểu pixel art, không làm mờ. Hình vẽ mượt thì đặt `false` |
| `facing` | | `"right"` | Hướng nhân vật nhìn trong ảnh gốc; đi ngược hướng thì app tự lật |
| `anchor` | | giữa mép dưới frame | Điểm chân nhân vật trong frame. Frame có khoảng trống dưới chân thì giảm `y` cho pet chạm đất |
| `animations` | có | | Mỗi animation của engine ứng với một ảnh |

Mỗi animation:

| Trường | Bắt buộc | Mặc định | Ý nghĩa |
|---|---|---|---|
| `image` | có | | File ảnh (PNG/WebP), đường dẫn tương đối so với `pet.json`. Không phân biệt hoa thường |
| `fps` | có | | Tốc độ animation (0.5–60) |
| `frames` | | hết ảnh | Số frame. Bỏ trống thì lấy hết các frame từ `start` tới cuối ảnh |
| `start` | | `0` | Frame bắt đầu, đếm từ trái sang phải rồi xuống hàng |
| `row` | | `0` | Hàng bắt đầu, khi nhiều animation nằm chung một sheet |
| `loop` | | `true` | `false`: chạy một lần rồi dừng ở frame cuối |

Tên animation engine dùng: `idle` (bắt buộc), `walk`, `run`, `sleep`, `dragged` (bị nhấc lên),
`fall`, `land` (vừa tiếp đất), `react` (bị click), `dizzy` (choáng khi rơi từ cao), `climb` (leo cạnh
cửa sổ, mặt quay vào tường, tay chạm mép frame phía trước), `perch` (ngồi ở mép cửa sổ), `jump` (nhảy
giữa cửa sổ: đúng 4 frame lấy đà, bay, chạm chân, đứng dậy; app tự chọn frame theo cú nhảy nên `fps`
không dùng tới). Pack thiếu animation vẫn chạy được, animation thiếu được thay như sau:

| Thiếu | Dùng thay |
|---|---|
| `walk` / `run` | cái còn lại trong hai cái, không có thì `idle` |
| `dragged` / `fall` | cái còn lại trong hai cái, không có thì `idle` |
| `climb` | `dragged`, không có thì `idle` |
| `jump` | `react` |
| `sleep`, `dizzy` | đứng yên ở frame đầu của `idle` |
| khác | `idle` |

Lúc leo, app đặt pet cách tường đúng khoảng từ điểm chân tới pixel xa nhất về phía trước trong
`climb`, nên tay nhân vật chạm vào cạnh cửa sổ.

Lúc choáng app tự cho pet lảo đảo và vẽ sao bay vòng quanh đầu, nên `dizzy` chỉ cần mặt choáng
(pack có sẵn dùng đúng một frame). Frame này được giữ suốt 2,5 giây nên cần mảng màu phẳng: vân lấm
tấm (hai màu gần nhau xen kẽ từng pixel) trông như nhiễu khi pet lảo đảo. Lúc ngủ app không vẽ thêm gì: `sleep` tự thể hiện (nhắm mắt,
cuộn người).

## Thay logo

1. Thay `assets/icon.png` bằng PNG vuông, nền trong suốt, tối thiểu 1024×1024.
2. Tạo lại icon từ thư mục gốc repo (PowerShell):
   ```powershell
   pnpm icons:desktop
   ```
   Script dùng Tauri CLI, cập nhật `icon.ico` và các PNG trong `desktop/src-tauri/icons/`.
   Icon macOS, Microsoft Store, Android/iOS chỉ nằm trong `target/`, không đưa vào repo (app chỉ chạy Windows).
   Sau đó [`scripts/small-icons.mjs`](../scripts/small-icons.mjs) vẽ lại cỡ **16, 24, 32 px** theo lưới
   pixel và dựng lại `icon.ico`: thu nhỏ logo lớn xuống các cỡ này thì mắt, miệng nhoè hết, mà tray và
   thanh tiêu đề lại dùng đúng các cỡ đó. Đổi hẳn hình logo thì sửa luôn hình vẽ trong script này
   (`node scripts/small-icons.mjs --preview xem.png` để xem thử).
3. Build lại app bằng `pnpm build:desktop`. Setup và uninstaller dùng `icons/icon.ico`;
   executable và tray dùng bộ icon cùng nguồn. Workflow Build và Release tự tạo lại icon trước khi build.
   Khi đang chạy dev, sửa `icon.ico` không tự nhúng lại vào exe: chạm vào `desktop/src-tauri/build.rs`
   (hoặc chạy lại `pnpm dev:desktop` sau `cargo clean`) để build script chạy lại.
