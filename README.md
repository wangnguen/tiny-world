# TinyWorld

Pet sống trên desktop Windows: app **Tauri v2 + Vite + TypeScript**, backend Rust. Nhân vật được vẽ
trên một cửa sổ overlay trong suốt, luôn nằm trên cùng, chuột đi xuyên qua trừ khi trỏ vào pet.
Lộ trình và các hướng đã chốt: [PLAN.md](PLAN.md).

```
desktop-pet/
├── assets/            # logo (icon.png) và sprite pack của nhân vật (sprites/<pack>/pet.json)
├── packages/
│   ├── core/          # types dùng chung Rust <-> TS, tên event, đọc/kiểm tra pet.json
│   └── sim/           # engine mô phỏng TS thuần: FSM, bước thời gian cố định, RNG có seed
└── desktop/           # Tauri v2
    ├── src/           # overlay (vẽ pet bằng canvas, TS thuần) và src/settings/ (cửa sổ cài đặt, React)
    └── src-tauri/     # Rust: overlay, click-through, đọc con trỏ, tray, cài đặt, lưu trạng thái
```

**15 nhân vật có sẵn**, chọn trong **Cài đặt… → Nhân vật**: Momo (axolotl hồng, mặc định), Bông,
Kitsu, Mầm, Bíp, Lumi, Nấm, Mây, Tàn, Rêu, Cục, Mực, Dứa, Su và Bắp. Mỗi nhân vật đủ 9 animation
Phase 1. Xem chuyển động tại
[sprite studio](assets/sprite-sources/index.html) hoặc [danh mục nhân vật](assets/sprite-sources/README.md).

**Thêm nhân vật:** bỏ sprite pack vào `assets/sprites/<tên-pack>/` kèm
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

App không có cửa sổ trên taskbar: lần đầu pet xuất hiện ở góc phải dưới, đứng trên mép taskbar; các
lần sau pet ở đúng chỗ lúc tắt app (đang ngủ thì vẫn ngủ). Điều khiển bằng icon **TinyWorld** ở system
tray:

| Mục | Tác dụng |
|---|---|
| **Tạm dừng** | Pet đứng yên, chuột đi xuyên qua pet; bấm lần nữa để pet sống tiếp |
| **Ẩn / hiện pet** | Ẩn hẳn pet (app dừng vòng lặp, không tốn CPU) |
| **Cài đặt…** | Chọn nhân vật, cỡ nhân vật (50–200%), tốc độ đi lại, chạy cùng Windows. Đổi là áp dụng ngay |
| **Thoát** | Lưu trạng thái pet rồi thoát |
| **Mở DevTools** | Chỉ khi chạy dev (overlay để chuột đi xuyên nên không bấm F12 được) |

Khi chạy dev, `window.__tinyworld` trong DevTools cho xem và chỉnh pet, ví dụ
`__tinyworld.pet.sinceInteraction = 1e6` để pet đi ngủ ngay. Bật **Chạy cùng Windows** trong lúc chạy dev
sẽ ghi đường dẫn exe bản debug, nhớ tắt lại.

Tương tác với pet:

| Thao tác | Pet |
|---|---|
| Để yên | Tự đứng, đi, chạy; chạm mép màn hình thì quay đầu |
| Click | Nhảy lên một cái rồi đi hoặc chạy tiếp; đang ngủ thì thức dậy |
| Kéo lên rồi thả | Rơi xuống, nảy nhẹ khi chạm đất; thả từ cao thì choáng: lảo đảo, sao bay quanh đầu |
| Kéo rồi vung chuột và buông | Bị ném bay theo quán tính, đập tường thì bật lại |
| 3 phút không đụng tới | Ngủ 💤, ngủ tới khi được click hoặc kéo |
| Giữ **Ctrl** khi click | Click xuyên qua pet xuống app bên dưới |

Click pet không làm mất focus của app đang dùng. Có app fullscreen (video, game, trình chiếu) thì pet
tự ẩn, thoát fullscreen thì hiện lại. Lúc ẩn (kể cả ẩn từ tray) app dừng hẳn vòng lặp, pet đứng nguyên
chỗ cũ chờ hiện lại.

## Build bản phát hành

```bash
pnpm icons:desktop   # tạo lại icon nếu vừa thay assets/icon.png
pnpm build:desktop   # installer NSIS trong desktop/src-tauri/target/release/bundle/nsis
```

Logo nguồn ở [`assets/icon.png`](assets/icon.png), dùng chung cho app, system tray, setup và portable.
Build và Release trên GitHub tự tạo bộ icon từ logo này trước khi compile.

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
Push chỉ sửa file `.md`, thư mục `docs/` hoặc `release.yml` thì không build (sửa sprite, icon trong
`assets/` vẫn build vì chúng nằm trong app).
Muốn build lại mà không push: tab **Actions → Build → Run workflow**.

### Release bằng GitHub Actions (chạy tay)

Khi muốn phát hành chính thức lên trang **Releases**, workflow
[`.github/workflows/release.yml`](.github/workflows/release.yml) chạy test, build
`TinyWorld_<version>_x64-setup.exe` và `TinyWorld_<version>_x64-portable.exe`, rồi gắn cả hai vào
GitHub Release `v<version>`.

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
  nút taskbar, phủ vùng làm việc (trừ taskbar) của màn hình chính. Không phủ kín cả màn hình vì
  Windows sẽ coi đó là app fullscreen: tắt thông báo, và pet tự ẩn rồi hiện liên tục. Toạ độ gửi cho
  frontend đều là CSS pixel của overlay; Rust đổi từ pixel vật lý theo DPI.
- **Pet**: mỗi pet là một `<canvas>` nhỏ đúng bằng một frame, di chuyển bằng CSS transform, chỉ vẽ
  lại khi đổi frame. Vòng lặp tối đa 30 fps.
- **Logic** (`packages/sim`): không phụ thuộc DOM hay Tauri, test bằng vitest. Bước thời gian cố
  định nên hành vi không phụ thuộc fps; RNG có seed để test được hành vi ngẫu nhiên.
- **Cửa sổ cài đặt** (`src/settings/`, `src-tauri/src/settings.rs`): trang React riêng
  (`settings.html`), chỉ tạo khi bấm **Cài đặt…** và huỷ khi đóng để đỡ tốn RAM. Overlay không kéo
  React theo.
- **Dữ liệu** trong `%APPDATA%\com.tinyworld.app\`, ghi ra file tạm rồi đổi tên:
  - `world.json`: vị trí, hướng, đang ngủ hay không của pet. Lưu 30 giây một lần (chỉ khi có thay
    đổi) và khi bấm **Thoát**; tắt máy ngang thì mất tối đa 30 giây.
  - `settings.json`: nhân vật đang chọn (tên thư mục pack), cỡ nhân vật, tốc độ. Sửa tay sai thì app
    kẹp về khoảng cho phép; pack không còn thì dùng pack đầu tiên.
  - **Chạy cùng Windows** không lưu ở đây mà là giá trị `TinyWorld` trong
    `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (trỏ tới exe đang chạy).

## Kiểm thử

```bash
pnpm typecheck
pnpm test                               # pet.json, frame, FSM, World/Pet, lưu/đọc world.json
cd desktop/src-tauri && cargo test      # toạ độ theo DPI, lưu trạng thái, cài đặt, registry
```
