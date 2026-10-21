# Overlay TinyWorld (DirectComposition)

Tiến trình chính của app: `tinyworld.exe` (desktop/src-tauri) gọi `tinyworld_native::run`. Vẽ pet bằng Direct3D
11, Direct2D, DirectWrite và DirectComposition, không có WebView2. Cửa sổ Cài đặt và khung chat là Tauri + React ở
tiến trình riêng (`tinyworld.exe --ui`), overlay chạy tiến trình đó lúc cần mở và nói chuyện qua `ipc.rs`.

Cách vẽ: một cửa sổ trong suốt phủ vùng làm việc của một màn hình (`WS_EX_NOREDIRECTIONBITMAP`, DWM ghép thẳng cây
visual). Mỗi pet một nhóm visual (sao choáng, aura của Long, sprite, thời tiết, cinematic của Long, bóng thoại), mỗi
visual một surface đúng cỡ. Đổi frame thì vẽ lại surface; đi lại chỉ đổi offset của visual rồi `Commit`.

| File | Nội dung |
|---|---|
| `src/sim/` | Mô phỏng pet (chuyển từ packages/sim): bước cố định, địa hình, FSM, nhóm pet, thời tiết, âm lịch, lịch sự kiện, câu pet nói; test ở `src/sim/tests/` |
| `src/app.rs` | Vòng lặp chính: message, đọc con trỏ, bước mô phỏng, vẽ, nhiều màn hình, khay, lưu `world.json` |
| `src/view.rs`, `bubble.rs`, `dizzy.rs` | Vẽ một pet: frame, phần bị cửa sổ che, lảo đảo, sao choáng, bóng thoại, vùng bắt chuột |
| `src/weather.rs`, `tag.rs`, `ghost.rs`, `long.rs` | Thời tiết quanh pet, nhãn nhiệt độ, con ma, đồ diễn và combo của Long |
| `src/ambience.rs` | Thời tiết thật hay giả lập, ban đêm, dịp lễ, lời nhắc, báo bản mới, mục Xem thử |
| `src/interaction.rs` | Click, kéo, ném, bấm đúp, Shift + click |
| `src/sprites.rs`, `assets.rs`, `build.rs` | Đọc pet.json, cắt frame, mặt nạ; sprite và effect của Long nhúng vào exe lúc build |
| `src/ipc.rs`, `commands.rs` | Kết nối với tiến trình giao diện, command của Cài đặt và khung chat |
| `src/gfx.rs`, `window.rs`, `screens.rs`, `tray.rs` | Direct2D/DirectComposition, cửa sổ overlay, danh sách màn hình, icon khay |

## Chạy riêng

```powershell
cargo run -p tinyworld-native
cargo test -p tinyworld-backend -p tinyworld-native
```

Chạy riêng thì chỉ có overlay (build nhanh hơn cả app). Đã build `tinyworld.exe` cạnh exe này (cùng thư mục
`target/<profile>`) thì Cài đặt và khung chat mở bằng nó; bản debug của `tinyworld.exe` lấy trang từ máy chủ dev của
Vite (`pnpm --filter desktop dev`).

Bản dev dùng thư mục dữ liệu `%APPDATA%\com.tinyworld.app.dev` và ghi cổng, mã kết nối vào `ipc.json` ở đó. Script
nối vào (`{"hello": mã, "debug": true}` rồi mỗi dòng `{"id": n, "call": tên, "args": {...}}`) gọi được mọi command
của Cài đặt, thêm `debug_state` (state, vị trí, câu đang nói của từng con), `debug_pet` (`poke`, `grab`, `drag`,
`release`, `say`, `sleepy`, `skill`, `double` cho con thứ `index`) và `debug_open` (mở Cài đặt, hoặc chat với con thứ
`index`).
