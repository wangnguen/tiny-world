# TinyWorld: hướng dẫn cho agent

## Tổng quan

TinyWorld là desktop pet cho Windows. Ứng dụng dùng Tauri v2: Rust trong `desktop/src-tauri/`, Vite + TypeScript trong `desktop/src/`, React chỉ cho Settings và Chat. Overlay trong suốt được vẽ bằng canvas; mô phỏng hành vi thuần TypeScript ở `packages/sim/`.

## Lệnh chính

```powershell
pnpm install --frozen-lockfile
pnpm typecheck
pnpm test
pnpm test:sprites
pnpm dev:desktop
pnpm build:desktop
```

`pnpm dev:desktop` và `pnpm build:desktop` cần Rust MSVC, Visual Studio Build Tools và WebView2. Các kiểm tra TypeScript/sprite chỉ cần Node.js 22+ và pnpm 11+ sau khi cài dependencies.

## Cấu trúc và quy ước

- `packages/core/`: types, protocol và đọc/kiểm tra manifest sprite.
- `packages/sim/`: FSM, vật lý/movement, terrain, weather và test Vitest; giữ phần này không phụ thuộc DOM/Tauri.
- `desktop/src/overlay/`: render/click interaction; `desktop/src/settings/` và `desktop/src/chat/` là React.
- `desktop/src-tauri/`: tích hợp Windows, overlay, tray, cursor, storage và commands. Không thay đổi hành vi Windows mà không kiểm tra trên Windows.
- `assets/sprite-sources/<source>/`: file nguồn của nhân vật, gồm `atlas.png` 4 cột x 12 hàng và các prompt/sheet sửa nếu có.
- `assets/sprites/<pack>/`: output chạy app (`pet.json` và WebP); không chỉnh trực tiếp vì script sẽ ghi đè.

## Thêm hoặc sửa nhân vật

Đọc `CREATE_PET.md` và `assets/README.md` trước. Thêm atlas nguồn, khai báo nhân vật trong `PETS` của `scripts/prepare-sprites.mjs`, sau đó chạy:

```powershell
node scripts/prepare-sprites.mjs --pet=<source>
pnpm test:sprites
```

Atlas thông thường được cắt bằng các khoảng alpha. Nếu atlas là lưới đều nhưng pose chạm nhau ở mép ô, dùng `grid: true` trong khai báo `PETS`; chỉ dùng khi kích thước mỗi ô là chính xác. Mỗi pack cần 12 animation theo thứ tự: `idle`, `walk`, `run`, `sleep`, `react`, `fall`, `dragged`, `land`, `dizzy`, `climb`, `perch`, `jump`.

## CI và Git

- Workflow Build chạy khi push vào `main` hoặc `test`, xuất installer và portable `.exe` dưới dạng Actions artifacts.
- Workflow Release chỉ chạy tay và tạo GitHub Release.
- Dùng `test` để kiểm thử artifact trước khi merge vào `main`.
- Trước khi commit thay đổi sprite hoặc script, chạy ít nhất `pnpm test:sprites`; trước thay đổi rộng hơn chạy `pnpm test` và `pnpm typecheck`.
