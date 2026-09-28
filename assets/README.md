# Assets

Thư mục này chứa **file gốc** của logo và hình nhân vật. Không sửa trực tiếp các icon đã tạo ra;
thay file gốc ở đây rồi chạy lại lệnh tạo icon.

| File | Dùng cho |
|---|---|
| `icon.png` | Logo TinyWorld nền trong suốt: pet màu kem ôm hành tinh xanh, có mầm cây; dùng cho app, tray, setup và portable |
| `branding/logo-prompt.txt` | Prompt gốc để tạo logo bằng imagegen |
| `sprites/<pack>/` | Sprite pack của nhân vật: `pet.json` + ảnh PNG/WebP |

## Sprite pack

Mỗi pack là một thư mục trong `sprites/`, có file `pet.json` mô tả các animation. App dùng pack
**đầu tiên theo tên thư mục**; chưa có pack nào (hoặc pack bị lỗi) thì dùng pet tạm (cục blob
16×16 vẽ bằng code) và in lý do lỗi ra console (tray → **Mở DevTools** khi chạy dev).

```
assets/sprites/cat/
├── pet.json
├── Idle.png
├── Walk.png
├── Run.png
└── LICENSE.txt      # license / credit của tác giả, giữ nguyên khi tải về
```

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

Lúc ngủ và lúc choáng luôn có 💤 / 💫 hiện trên đầu pet, kể cả khi pack không có animation riêng.
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
   Script dùng Tauri CLI, cập nhật `.ico`, `.icns` và các PNG trong `desktop/src-tauri/icons/`.
   Icon Android/iOS được tạo trong `target/`, không đưa vào repo.
3. Build lại app bằng `pnpm build:desktop`. Setup và uninstaller dùng `icons/icon.ico`;
   executable và tray dùng bộ icon cùng nguồn. Workflow Build và Release tự tạo lại icon trước khi build.
