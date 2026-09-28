# TinyWorld

Pet sống trên desktop Windows: app **Tauri v2 + Vite + TypeScript**, backend Rust. Nhân vật được vẽ
trên một cửa sổ overlay trong suốt, luôn nằm trên cùng, chuột đi xuyên qua trừ khi trỏ vào pet.
Lộ trình và các hướng đã chốt: [PLAN.md](PLAN.md).

```
desktop-pet/
├── assets/            # logo (icon.svg) và sprite pack của nhân vật (sprites/<pack>/pet.json)
├── packages/
│   ├── core/          # types dùng chung Rust <-> TS, tên event, đọc/kiểm tra pet.json
│   └── sim/           # engine mô phỏng TS thuần: FSM, bước thời gian cố định, RNG có seed
└── desktop/           # Tauri v2
    ├── src/           # overlay: vẽ pet bằng canvas, vòng lặp requestAnimationFrame
    └── src-tauri/     # Rust: overlay, click-through, đọc con trỏ, tray, lưu trạng thái
```

**Thêm nhân vật:** bỏ sprite pack (ví dụ tải từ itch.io) vào `assets/sprites/<tên-pack>/` kèm
`pet.json` (định dạng và cách làm: [assets/README.md](assets/README.md)). Chưa có pack thì app dùng
pet tạm vẽ bằng code.

## Yêu cầu

- Node.js 22+, pnpm 11+ (pnpm 10 tự chuyển sang bản ghi trong `packageManager`)
- Rust (toolchain `x86_64-pc-windows-msvc`) và Visual Studio Build Tools (C++). Toolchain
  `x86_64-pc-windows-gnu` cũng build được nhưng linker báo cảnh báo gộp manifest; CI dùng MSVC.
- WebView2 Runtime (có sẵn trên Windows 10/11 bản mới)

## Chạy dev

```bash
pnpm install
pnpm dev:desktop   # lần đầu compile Rust mất vài phút
```

App không có cửa sổ trên taskbar: pet xuất hiện ở góc phải dưới, đứng trên mép taskbar. Điều khiển
bằng icon **TinyWorld** ở system tray: **Ẩn / hiện pet**, **Thoát**, và **Mở DevTools** khi chạy dev
(overlay để chuột đi xuyên nên không bấm F12 được).

## Build bản phát hành

```bash
pnpm build:desktop   # installer NSIS trong desktop/src-tauri/target/release/bundle/nsis
```

### Tự build khi push lên `main`

Mỗi lần push lên `main`, workflow [`.github/workflows/build.yml`](.github/workflows/build.yml) chạy
typecheck, test, `cargo test` rồi build file `.exe` cho Windows (mất khoảng 10–15 phút lần đầu, các
lần sau nhanh hơn nhờ cache).

Tải về: GitHub → tab **Actions** → bấm vào lần chạy mới nhất → mục **Artifacts** ở cuối trang →
tải `TinyWorld-<version>-<số lần chạy>-<commit>` (file zip, giữ 30 ngày). Trong zip có:

| File | Dùng khi |
|---|---|
| `TinyWorld_<version>_x64-setup.exe` | Cài vào máy (có shortcut, gỡ được trong Settings) |
| `TinyWorld_<version>_x64-portable.exe` | Chạy thẳng không cần cài (cần WebView2, Windows 10/11 bản mới có sẵn) |

Test hỏng thì không build ra file. Push liên tục thì lần build cũ bị huỷ, chỉ giữ lần mới nhất.
Muốn build lại mà không push: tab **Actions → Build → Run workflow**.

### Release bằng GitHub Actions (chạy tay)

Khi muốn phát hành chính thức lên trang **Releases**, workflow
[`.github/workflows/release.yml`](.github/workflows/release.yml) chạy test, build installer
`TinyWorld_<version>_x64-setup.exe` và gắn vào GitHub Release `v<version>`.

1. Push code cần phát hành lên GitHub.
2. GitHub → **Actions → Release → Run workflow** → nhập **version** (dạng `x.y.z`, ví dụ `0.2.0`).
   Version này được ghi vào app lúc build, không cần sửa file trong repo.
3. Mặc định tạo **bản nháp**; kiểm tra file ở trang **Releases** rồi bấm **Publish release**.
   Bỏ chọn *draft* nếu muốn phát hành luôn.

Chạy lại với version đã *publish* sẽ bị từ chối (phải tăng version); chạy lại khi còn là bản nháp
thì file cũ được ghi đè. App chưa ký số nên Windows hiện cảnh báo SmartScreen: chọn
**More info → Run anyway**.

## Kiến trúc

```
Rust (cursor.rs) --GetCursorPos ~60 lần/giây--> event "cursor-moved" (CSS pixel của overlay)
                                                      |
                                                      v
                      overlay: con trỏ nằm trên phần có hình của pet? (theo alpha của sprite)
                                                      |
                                invoke("set_click_through", { enabled })  (chỉ khi trạng thái đổi)
                                                      |
                                                      v
                      Rust: bật/tắt chuột đi xuyên overlay (set_ignore_cursor_events)
```

- **Overlay** (`src-tauri/src/overlay.rs`): cửa sổ trong suốt, không viền, luôn trên cùng, không có
  nút taskbar, phủ màn hình chính. Toạ độ gửi cho frontend đều là CSS pixel của overlay; Rust đổi
  từ pixel vật lý theo DPI.
- **Pet**: mỗi pet là một `<canvas>` nhỏ đúng bằng một frame, di chuyển bằng CSS transform, chỉ vẽ
  lại khi đổi frame. Vòng lặp tối đa 30 fps.
- **Logic** (`packages/sim`): không phụ thuộc DOM hay Tauri, test bằng vitest. Bước thời gian cố
  định nên hành vi không phụ thuộc fps; RNG có seed để test được hành vi ngẫu nhiên.
- **Lưu trạng thái**: `%APPDATA%\com.tinyworld.app\world.json`, ghi ra file tạm rồi đổi tên.

## Kiểm thử

```bash
pnpm typecheck
pnpm test                               # pet.json, frame, FSM, bước thời gian, World/Pet
cd desktop/src-tauri && cargo test      # đổi toạ độ theo DPI, lưu/đọc trạng thái
```
