# TinyWorld: hướng dẫn cho agent

## Tổng quan

TinyWorld là desktop pet cho Windows, viết bằng Rust. Overlay trong suốt vẽ bằng DirectComposition (Direct2D, không WebView2) ở `native/`, mô phỏng hành vi ở `native/src/sim/`. Cửa sổ Cài đặt và Chat là Tauri v2 + React (`desktop/src/`), chạy ở tiến trình riêng `tinyworld.exe --ui` lúc cần và nói chuyện với overlay qua kết nối nội bộ (`native/src/ipc.rs`, `desktop/src-tauri/src/ui.rs`).

## Lệnh chính

```powershell
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test
pnpm test:sprites
cargo test -p tinyworld-backend -p tinyworld-native
cargo test -p tinyworld
cargo run -p tinyworld-native
pnpm dev:desktop
pnpm build:desktop
```

`pnpm dev:desktop`, `pnpm build:desktop` và `cargo` cần Rust MSVC, Visual Studio Build Tools và WebView2. Test shell Tauri (`-p tinyworld`) chạy riêng: chung một lần build thì exe test của overlay không chạy được. `cargo run -p tinyworld-native` chỉ chạy overlay (build nhanh hơn cả app). Bản dev dùng thư mục dữ liệu `%APPDATA%\com.tinyworld.app.dev` và chạy song song được với bản cài; overlay dev ghi cổng và mã kết nối vào `ipc.json` ở đó để script gọi command (`debug_state`, `debug_pet`). Các kiểm tra TypeScript/sprite chỉ cần Node.js 22+ và pnpm 11+ sau khi cài dependencies.

## Cấu trúc và quy ước

- `packages/core/`: types, protocol và đọc/kiểm tra manifest sprite.
- `packages/core/src/i18n/vi.json`, `en.json`: mọi chữ người dùng thấy (Cài đặt, chat, câu pet nói, menu khay, câu báo lỗi của Rust). Thêm hay sửa câu thì sửa cả hai file; không viết chữ hiện cho người dùng thẳng trong code.
- `native/src/sim/`: FSM, vật lý/movement, terrain, thời tiết, âm lịch, câu pet nói và test; giữ phần này không phụ thuộc Windows hay phần vẽ.
- `native/src/`: vẽ (DirectComposition, Direct2D, DirectWrite), chuột, click-through, khay, nhiều màn hình, vòng lặp; command của Cài đặt/Chat ở `commands.rs`.
- `desktop/backend/`: phần Rust không có giao diện (cài đặt, lưu trạng thái, thời tiết, giờ ngồi máy, chat, cập nhật, danh sách cửa sổ). Không thay đổi hành vi Windows mà không kiểm tra trên Windows.
- `desktop/src-tauri/`: `tinyworld.exe`, chạy overlay hoặc (với `--ui`) cửa sổ Cài đặt/Chat; `desktop/src/settings/` và `desktop/src/chat/` là React.
- `packages/sim/`: phần TypeScript dùng chung cho Cài đặt và trang giới thiệu (thời tiết, âm lịch, lịch sự kiện, câu pet nói).
- `assets/sprite-sources/<source>/`: file nguồn của nhân vật, gồm `atlas.png` 4 cột x 12 hàng và các prompt/sheet sửa nếu có.
- `assets/sprites/<pack>/`: output chạy app (`pet.json` và WebP); không chỉnh trực tiếp vì script sẽ ghi đè.

## Thêm hoặc sửa nhân vật

Đọc `CREATE_PET.md` và `assets/README.md` trước. Thêm atlas nguồn, khai báo nhân vật trong `PETS` của `scripts/prepare-sprites.mjs`, sau đó chạy:

```powershell
node scripts/prepare-sprites.mjs --pet=<source>
pnpm test:sprites
```

Atlas thông thường được cắt bằng các khoảng alpha. Nếu atlas là lưới đều nhưng pose chạm nhau ở mép ô, dùng `grid: true` trong khai báo `PETS`; chỉ dùng khi kích thước mỗi ô là chính xác. Mỗi pack cần 12 animation theo thứ tự: `idle`, `walk`, `run`, `sleep`, `react`, `fall`, `dragged`, `land`, `dizzy`, `climb`, `perch`, `jump`.

Effect riêng của Long (aura, biến hình, quật đuôi) có sheet nguồn trong `assets/sprite-sources/long/`; sửa sheet rồi chạy `pnpm prepare:aura` để tạo lại `assets/effects/`. Effect được nhúng vào exe lúc build. Combo Shift + click nằm ở `native/src/long.rs`, chỉ điều khiển pet qua `hold`, `blink_to`, `vanish` của `native/src/sim/pet.rs`; không gán thẳng `x`/`y` của pet từ phần vẽ.

## CI và Git

- Workflow Build chạy khi push vào `main` hoặc `test`, xuất installer và portable `.exe` dưới dạng Actions artifacts.
- Workflow Release chỉ chạy tay và tạo GitHub Release.
- Dùng `test` để kiểm thử artifact trước khi merge vào `main`.
- Trước khi commit thay đổi sprite hoặc script, chạy ít nhất `pnpm test:sprites`; trước thay đổi rộng hơn chạy `pnpm test`, `pnpm typecheck` và hai lệnh `cargo test` ở trên.
