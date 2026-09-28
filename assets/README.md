# Assets

Thư mục này chứa **file gốc** của logo và hình nhân vật. Không sửa trực tiếp các icon đã tạo ra;
thay file gốc ở đây rồi chạy lại lệnh tạo icon.

| File | Dùng cho |
|---|---|
| `icon.png` | Logo TinyWorld nền trong suốt: pet màu kem ôm hành tinh xanh, có mầm cây; dùng cho app, tray, setup và portable |
| `branding/logo-prompt.txt` | Prompt gốc để tạo logo bằng imagegen |
| `sprites/<pack>/` | Sprite pack của nhân vật: `pet.json` + ảnh PNG/WebP |
| `sprite-sources/` | Atlas và prompt gốc của 22 nhân vật tự tạo, script chuẩn hóa và trang xem animation |

## Sprite pack

Mỗi pack là một thư mục trong `sprites/`, có file `pet.json` mô tả các animation. Người dùng chọn
nhân vật trong tray → **Cài đặt…** → **Nhân vật** (lưu tên thư mục vào `settings.json`). Chưa chọn,
hoặc pack đã chọn không còn, thì dùng pack **đầu tiên theo tên thư mục**. Pack có `pet.json` lỗi
không hiện trong danh sách; chưa có pack nào (hoặc pack lỗi khi nạp) thì dùng pet tạm (cục blob
16×16 vẽ bằng code) và in lý do lỗi ra console (tray → **Mở DevTools** khi chạy dev).

```
assets/sprites/a-momo/
├── pet.json
├── idle.png
├── walk.png
├── run.png
├── ...              # đủ 9 animation Phase 1
└── phase2/          # climb, perch, jump; chưa ghi vào pet.json
```

### Các pet tự tạo của TinyWorld

Có 22 nhân vật: Momo, Bông, Kitsu, Mầm, Bíp, Lumi, Nấm, Mây, Tàn, Rêu, Cục, Mực, Dứa,
Su, Bắp, Boggo, Wobi, Gloop, Bẹp, Frobu, Byte và Patch. [Danh mục thiết kế và thư mục pack](sprite-sources/README.md). Tất cả đủ 9 animation Phase 1,
mỗi animation có 4 frame **48×48**, alpha trong suốt thật, chân cùng hàng y=44,
quay sang phải. Mặc định là `a-momo`; đổi nhân vật trong **Cài đặt… → Nhân vật**, pet đổi ngay tại
chỗ. Thêm pack mới thì chạy lại `pnpm dev:desktop` để pack hiện trong danh sách.

Wobi, Byte và Patch chỉ có `pet.json` và một `atlas.png` **192×576** trong pack. Manifest dùng
`row` để lấy đúng hàng animation từ sheet chung; 9 hàng đầu là Phase 1, 3 hàng cuối là Phase 2.
Wobi cũng chỉ giữ một ảnh nguồn `sprite-sources/wobi/atlas.png`, không giữ các bản nháp hoặc ảnh
preview trùng. Byte và Patch là chim cánh cụt coder đeo kính/hoodie teal và gấu trúc đỏ coder đeo tai nghe/hoodie tím.

Xem tất cả chuyển động bằng [`sprite-sources/index.html`](sprite-sources/index.html), mở
trực tiếp bằng trình duyệt. Ảnh gốc, prompt và cách tạo lại: [sprite-sources/README.md](sprite-sources/README.md).
Pack Cat demo cũ của OboroPixel đã được gỡ khỏi cây mã nguồn hiện tại.
Chạy `node scripts/prepare-sprites.mjs --check` để kiểm tra các pack. Ba animation Phase 2
để riêng trong `phase2/` hoặc ở hàng 9–11 của sheet chung; không thêm vào `pet.json` trước khi engine hỗ trợ.

### Lấy pack từ itch.io

1. Tìm pack pixel art miễn phí trên [itch.io](https://itch.io/game-assets) (từ khoá: *cute pixel pet
   sprite*, *animal sprite sheet*, *32x32 character*). Đọc kỹ license: cần cho phép dùng trong
   project của mình và cho chỉnh sửa.
2. Giải nén, copy các PNG cần dùng vào `assets/sprites/<tên-pack>/`, kèm file license/credit.
3. Viết `pet.json` (xem bên dưới): ghép mỗi animation của engine với một file PNG.
4. Chạy `pnpm dev:desktop`. Vite gom pack vào bản build, nên thêm hoặc sửa pack chỉ cần chạy lại app.

> Nhiều license trên itch.io cho dùng trong app nhưng **cấm phát tán lại file gốc**. Nếu repo
> để public mà license cấm, đừng commit thư mục pack đó (thêm vào `.gitignore`); bản build vẫn
> nhúng hình vào app như bình thường.

### `pet.json`

Pack itch.io thường để **mỗi animation một file PNG dải ngang**, frame 32×32:

```json
{
  "name": "Cat",
  "frameWidth": 32,
  "frameHeight": 32,
  "scale": 2,
  "anchor": { "x": 16, "y": 30 },
  "animations": {
    "idle": { "image": "Idle.png", "fps": 8 },
    "walk": { "image": "Walk.png", "fps": 10 },
    "run": { "image": "Run.png", "fps": 14 },
    "fall": { "image": "Fall.png", "fps": 10 },
    "react": { "image": "Jump.png", "fps": 12, "loop": false }
  }
}
```

| Trường | Bắt buộc | Mặc định | Ý nghĩa |
|---|---|---|---|
| `frameWidth`, `frameHeight` | có | | Kích thước một frame (pixel của ảnh gốc) |
| `name` | | `"pet"` | Tên hiển thị |
| `scale` | | `2` | Phóng to khi vẽ: 32×32 với scale 2 thành 64×64. Muốn pet to/nhỏ hơn thì chỉnh số này |
| `pixelArt` | | `true` | Phóng to kiểu pixel art, không làm mờ. Hình vẽ mượt thì đặt `false` |
| `facing` | | `"right"` | Hướng nhân vật nhìn trong ảnh gốc; đi ngược hướng thì app tự lật |
| `anchor` | | giữa mép dưới frame | Điểm chân nhân vật trong frame. Frame có khoảng trống dưới chân thì giảm `y` cho pet chạm đất |
| `outline` | | `"#1b1622"` | Viền 1 pixel quanh nhân vật để nhân vật màu sáng không chìm vào nền trắng. Nhận màu `"#rrggbb"` / `"#rrggbbaa"`; pack đã vẽ viền sẵn thì đặt `false` |
| `animations` | có | | Mỗi animation của engine ứng với một ảnh |

Mỗi animation:

| Trường | Bắt buộc | Mặc định | Ý nghĩa |
|---|---|---|---|
| `image` | có | | File ảnh, đường dẫn tương đối so với `pet.json`. Không phân biệt hoa thường |
| `fps` | có | | Tốc độ animation (0.5–60) |
| `frames` | | hết ảnh | Số frame. Bỏ trống thì lấy hết các frame từ `start` tới cuối ảnh |
| `start` | | `0` | Frame bắt đầu, đếm từ trái sang phải rồi xuống hàng |
| `row` | | `0` | Hàng bắt đầu, khi nhiều animation nằm chung một sheet |
| `loop` | | `true` | `false`: chạy một lần rồi dừng ở frame cuối |

Tên animation engine dùng: `idle` (bắt buộc), `walk`, `run`, `sleep`, `dragged` (bị nhấc lên),
`fall`, `land` (vừa tiếp đất), `react` (bị click), `dizzy` (choáng khi rơi từ cao). Pack chỉ có vài
animation vẫn chạy được, animation thiếu được thay như sau:

| Thiếu | Dùng thay |
|---|---|
| `walk` / `run` | cái còn lại trong hai cái, không có thì `idle` |
| `dragged` / `fall` | cái còn lại trong hai cái, không có thì `idle` |
| `sleep`, `dizzy` | đứng yên ở frame đầu của `idle` |
| khác | `idle` |

Lúc choáng app tự cho pet lảo đảo và vẽ sao bay vòng quanh đầu, có cả khi pack không có animation riêng,
nên `dizzy.png` không cần vẽ sao hay nghiêng người, chỉ cần mặt choáng. Lúc ngủ app không vẽ thêm gì:
`sleep.png` tự thể hiện (nhắm mắt, cuộn người).
Pack chung một sheet nhiều hàng thì dùng `row` / `start` / `frames`:

```json
"walk": { "image": "cat-sheet.png", "row": 2, "frames": 8, "fps": 10 }
```

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
