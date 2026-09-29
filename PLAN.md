# TinyWorld — Plan

Một thế giới nhỏ sống trên desktop: các pet có tính cách, quan hệ, tự sống cạnh cửa sổ, taskbar và
con trỏ chuột trong lúc người dùng làm việc. Không phải game phải mở lên chơi. Ưu tiên số 1: **nhẹ**.

## Hướng đã chốt

| Chủ đề | Quyết định |
|---|---|
| Quy trình | Làm theo plan này, **không** dùng AI-DLC |
| Base | Theo cấu trúc và quy ước của `C:\Users\quang.nguyen13\Desktop\authenticator-app` |
| Tên | TinyWorld, identifier `com.tinyworld.app` (dữ liệu ở `%APPDATA%\com.tinyworld.app\`) |
| Nền tảng | Chỉ Windows 10/11. Desktop awareness dùng Win32, CI chỉ build NSIS |
| Stack | Tauri v2 + Vite + TypeScript, backend Rust. Không Electron, không game engine, không server |
| Vẽ pet | Mỗi pet một `<canvas>` nhỏ, di chuyển bằng CSS transform, vòng lặp `requestAnimationFrame` tối đa 30 fps. React chỉ dùng cho cửa sổ Settings (Phase 1) |
| Logic pet | `packages/sim`: TS thuần (FSM, vật lý, RNG có seed), test bằng vitest |
| Click-through | Overlay mặc định để chuột đi xuyên. Rust đọc con trỏ khoảng 60 lần/giây gửi sang, overlay kiểm tra theo alpha của sprite, con trỏ nằm trên pet thì tắt click-through |
| Asset | Nhân vật tự vẽ bằng imagegen: atlas gốc ở `assets/sprite-sources/`, `scripts/prepare-sprites.mjs` chuẩn hoá thành pack `assets/sprites/<pack>/` có `pet.json`. Không dùng asset của bên thứ ba. Chưa có pack thì dùng pet tạm vẽ bằng code |
| Thứ tự | Phase 2 (desktop awareness) làm trước Phase 3 (bộ lạc) |
| Riêng tư | Mọi dữ liệu xử lý trên máy, không đọc tiêu đề cửa sổ, tính năng nhạy cảm (bàn phím, thói quen) mặc định tắt |
| AI | Không cho AI điều khiển di chuyển, chỉ dùng cho hội thoại (Phase 7, tuỳ chọn) |
| Ngôn ngữ | UI, comment, README tiếng Việt như authenticator-app |
| Build / phát hành | Mỗi lần push lên `main`, CI tự test rồi build `.exe` (bản cài + bản chạy thẳng), tải ở mục Artifacts của Actions; push nhiều lần thì chỉ build bản mới nhất, push chỉ sửa tài liệu (`.md`) thì không build. Phát hành chính thức vẫn dùng workflow Release chạy tay (nhập version) như authenticator-app |

### Kế thừa từ authenticator-app

Giữ nguyên:

- pnpm monorepo (`packages/*` + `desktop`), `tsconfig.base.json` strict, package nội bộ export thẳng source TS
- Tauri v2 + Vite (port 1420), release profile `lto`, `opt-level = "s"`, `strip`, `codegen-units = 1`
- Rust: `main.rs → app.rs → commands.rs → module nghiệp vụ`; lỗi `AppError { code, message }` trả thẳng về frontend
- Frontend gọi Rust qua `desktop/src/api.ts`; types dùng chung ở `packages/core` khớp struct Rust (serde camelCase)
- single-instance, dữ liệu JSON trong `%APPDATA%`, `assets/` giữ file gốc kèm README
- Workflow release chạy tay (nhập version → test → release nháp → build NSIS)

Khác đi:

- `packages/ui` thay bằng `packages/sim`
- Overlay trong suốt, luôn nằm trên, không có nút taskbar; điều khiển qua icon ở system tray
- Không có `extension/`, không có Google OAuth
- Thêm workflow Build: push lên `main` là tự build `.exe` ra Artifacts, không cần chạy release

## Lộ trình

| Phase | Tên | Kết quả chính | Trạng thái |
|---|---|---|---|
| 0 | Base | Monorepo, overlay trong suốt click-through, pet tạm, tray, CI | Xong: test pass, đã chạy thử trên máy thật (overlay trong suốt, click-through theo alpha, click → phản ứng) |
| 1 | MVP | 1 pet: tự đi, quay đầu ở mép, click phản ứng, kéo thả + rơi, bỏ mặc thì ngủ | Xong, đã thử trên máy thật: hành vi, vật lý, kéo thả/ném, Ctrl xuyên pet, không cướp focus, tự ẩn khi fullscreen, lưu trạng thái, tray Tạm dừng, Settings (nhân vật, cỡ, tốc độ, chạy cùng Windows). Đo 10 phút: heap JS ~2 MB không tăng; còn chạy thử 8 tiếng |
| 2 | Desktop awareness | Đứng/leo/nhảy trên cửa sổ thật, ngủ trên taskbar, con trỏ là thực thể, đa màn hình | Đang làm: 2a–2d xong, đã thử trên máy thật; 2e xong phần code và test, chờ thử trên máy có 2 màn hình và DPI khác 100% |
| 3 | Bộ lạc | Nhiều pet, tính cách, quan hệ, nhật ký sự kiện, speech bubble, skin | |
| 4 | Thế giới sống | Căn cứ + xây nhà, nhu cầu, thời tiết cục bộ, ngày/đêm, sự kiện hiếm | |
| 5 | Thói quen user | Thống kê app theo giờ, nhắc khuya, phản ứng gõ phím (chỉ trên máy, tự bật) | |
| 6 | Tiến hoá & colony | Tiến hoá theo cách đối xử, trứng nở, Keep/Send away, file `.pet` | |
| 7 | AI dialogue | Chuột phải → Talk, LLM chỉ cho thoại (tuỳ chọn) | |

### Phase 0 — Base

- Monorepo, script `dev:desktop`, `build:desktop`, `typecheck`, `test`
- `packages/core`: types Rust ↔ TS, tên event, `AppError`, đọc/kiểm tra `pet.json`
- `packages/sim`: bước thời gian cố định, RNG có seed, khung FSM, `World`/`Pet` (idle, react)
- Overlay phủ vùng làm việc của màn hình chính (không phủ kín màn hình, xem Rủi ro): trong suốt, luôn trên, ẩn khỏi taskbar, click-through; pet tạm đứng ở góc phải trên mép taskbar, click thì phản ứng
- Loader sprite pack từ `assets/sprites/`, animation thiếu thì dùng `idle`
- Rust: `overlay.rs`, `cursor.rs`, `tray.rs` (ẩn/hiện, thoát, DevTools khi debug), `storage.rs`, single-instance
- CI: push lên `main` → typecheck, test, `cargo test` → build `.exe` Windows, tải ở Artifacts;
  workflow Release chạy tay để phát hành `v<version>` lên trang Releases

**Xong khi:** click vùng trống lọt xuống app bên dưới, click pet thì pet phản ứng; tray chạy được;
`pnpm typecheck`, `pnpm test`, `cargo test` đều qua.

### Phase 1 — MVP: một con pet

- State: idle, walk, run, sleep, dragged, fall, land, react, dizzy
- Pet xuất hiện ở góc phải dưới, đứng trên mép taskbar, tự chọn đứng/đi/chạy, chạm mép quay đầu
- Click → phản ứng; kéo thả, thả ra rơi theo trọng lực; ném có quán tính, chạm đất nảy, thả từ cao thì choáng
- Lâu không tương tác → buồn ngủ → ngủ, click đánh thức
- Lưu trạng thái khi tắt/mở; tray thêm Tạm dừng; Settings (React): chọn nhân vật, kích thước, tốc độ, chạy cùng Windows
  - `world.json` lưu 30 giây/lần khi có thay đổi và khi bấm Thoát (tray → overlay lưu → command `quit`,
    quá 1,5 giây thì Rust tự thoát); chỉ giữ vị trí ngang, hướng, đang ngủ, thời gian chưa được đụng tới
  - Settings là trang React riêng, chỉ tạo cửa sổ khi mở; chạy cùng Windows ghi thẳng khoá `Run` trong
    registry bằng `windows-sys`, không dùng plugin
- Nhẹ: dừng vẽ khi ngủ/ẩn/tạm dừng, tự ẩn khi có app fullscreen, click pet không cướp focus của app đang
  dùng (`focusable: false` → `WS_EX_NOACTIVATE`, kéo thả vẫn chạy)
- 21 sprite pack tự tạo (Momo mặc định): đủ 9 animation, frame 192×192 (96×96 CSS pixel ở cỡ 100%),
  alpha nhị phân và viền tối sẵn trong ảnh; pose leo/ngồi mép/nhảy làm sẵn cho Phase 2, chưa đưa vào `pet.json`

**Xong khi:** chạy 8 tiếng không rò RAM; unit test cho FSM và vật lý.

### Phase 2 — Desktop awareness

- Rust Win32: danh sách cửa sổ (khung thật qua DWM, thứ tự chồng, minimize, tên process), taskbar, màn hình + DPI; theo dõi thay đổi rồi gửi cho overlay
- Mép trên cửa sổ là nền để đứng, cạnh bên là tường để leo: ngồi mép, leo, nhảy giữa cửa sổ, rơi khi minimize/đóng, bám theo khi kéo cửa sổ, bị che khi cửa sổ khác đè lên, ngủ trên taskbar, chạy trốn khi cửa sổ bị kéo tới, ăn mừng khi app đóng
- Con trỏ là thực thể: nhìn theo, đuổi/né, lại gần ngửi khi đứng yên, giật mình ngã khi giật chuột
- Đa màn hình, DPI khác nhau

Chia 5 bước, xong bước nào thử trên máy thật bước đó:

| Bước | Nội dung | Trạng thái |
|---|---|---|
| 2a | `window_list.rs`: `SetWinEventHook` + đọc lại tối đa 30 lần/giây, chỉ gửi khi đổi, không đổi thì ngủ (2 giây đọc lại một lần). `packages/sim/src/terrain.ts`: mép trên là chỗ đứng, cạnh bên là tường, trừ phần bị cửa sổ nằm trên che. Pet đứng/đi trên mép, đi theo khi kéo cửa sổ, rơi khi thu nhỏ/đóng/hết chỗ đứng, thả lên mép thì đáp; bị che thì phần bị che không vẽ, không bắt chuột, lát sau đi ra | Xong, đã thử trên máy thật (Notepad làm địa hình): khung cửa sổ khớp tới từng pixel, pet đi theo khi kéo, rơi khi thu nhỏ/đóng/kéo sát mép trên, `closed` báo đúng cửa sổ bị đóng, phần bị che mất đúng chỗ |
| 2b | `climb`, `perch`, `jump` vào `pet.json` (21 pack). Leo lên/xuống cạnh cửa sổ, nhảy sang cửa sổ khác hoặc xuống taskbar, ngồi mép, buồn ngủ thì xuống taskbar rồi mới ngủ, click lúc đang leo thì tuột tay | Xong, đã thử trên máy thật: tay chạm đúng cạnh cửa sổ lúc leo, lên tới đỉnh nhún qua mép, nhảy sang cửa sổ khác đáp đúng chỗ ngắm, ngồi mép, buồn ngủ thì nhảy xuống taskbar rồi ngủ |
| 2c | Chạy trốn khi cửa sổ bị kéo tới (giữ nguyên cỡ mà lại gần dưới 90 px), ăn mừng khi app đóng (cách dưới 400 px, nhảy 2 cái, xác suất 0,8, 15 giây một lần). Cửa sổ lơ lửng: nhảy thẳng lên bám cạnh rồi leo | Xong, đã thử trên máy thật: nhảy lên bám góc dưới cửa sổ lơ lửng rồi leo lên mép; kéo cửa sổ tới thì chạy (trên mép cửa sổ hết đường thì leo xuống); đóng 3 lần thì cả 3 lần nhảy 2 cái |
| 2d | Con trỏ là thực thể: đứng yên thì nhìn theo con trỏ trong 300 px; con trỏ đứng yên ngang tầm 1,2 giây thì lại gần ngửi (15 giây một lần); lướt qua 60–900 px/s thì có lúc đuổi (xác suất 0,5, bốc thăm 5 giây một lần, tối đa 6 giây); lao tới nhanh hơn 1000 px/s thì né; giật chuột nhanh hơn 2500 px/s quét sát thân thì nhảy dựng lên rồi ngã choáng (10 giây một lần); đang giữ chuột thì chỉ nhìn | Xong, đã thử trên máy thật (bơm vị trí con trỏ vào sim): quay đầu theo con trỏ, đi tới cạnh rồi quay mặt vào ngửi, chạy đuổi con trỏ lướt 250 px/s, né 3/3 lần, giật chuột 4000 px/s thì nhảy rồi choáng 2,5 giây |
| 2e | Đa màn hình và DPI: mỗi giây đo lại màn hình, đổi độ phân giải/DPI/taskbar hay rút màn hình thì đặt lại overlay (`screen-changed` kèm `remap`); kéo, ném hoặc tự đi ra khỏi mép giáp màn hình khác thì overlay sang bên đó | Xong phần code, test sim và Rust. Trên máy thật (lúc thử chỉ còn 1 màn hình 100%): đo lại mỗi giây không gửi event thừa, CPU luồng Rust lúc pet ngủ dưới 0,1% một nhân; đi ra mép giáp màn hình giả thì gọi `move_overlay` mỗi 200 ms, không sang được thì 1,5 giây sau quay vào; taskbar cao lên/thấp xuống (event giả) thì pet đứng lên/rơi xuống mặt đất mới. Chưa thử sang màn hình thật và DPI 125–200% |

Đã chốt trong 2a–2e:

- Pet trên taskbar hoặc đang bay nằm trước mọi cửa sổ; đứng/leo trên cửa sổ nào thì bị các cửa sổ nằm trên cửa sổ đó che. Đáp chỉ lên phần mép nhìn thấy được
- Chưa đọc tên process: Phase 2 chưa cần, để Phase 5. Chỉ đọc tên lớp cửa sổ để bỏ desktop và taskbar
- Đóng cửa sổ thì Windows ẩn trước rồi mới huỷ: cửa sổ biến khỏi danh sách được theo dõi thêm 3 giây
  (đọc lại mỗi 250 ms), bị huỷ trong lúc đó mới báo `closed`, kèm khung lúc còn hiện
- Cửa sổ phóng to không có chỗ đứng (mép trên chạm trần), chỉ che; mọi cửa sổ đều phóng to thì pet sống trên taskbar như Phase 1
- Không có cửa sổ nào gần thì pet sống y như Phase 1 (cùng seed ra cùng hành vi)
- Lúc nhảy, vị trí tính thẳng theo đường parabol, chỗ đáp lấy đúng điểm chân cắt ngang mép: cộng dồn từng
  bước thì pet rơi nhanh hơn đường đã ngắm, đáp hụt tới 17 px, trượt khỏi mép khi chỗ đáp sát đầu mép
- CPU đo trên bản debug, máy 4 nhân (tinyworld + các tiến trình WebView2): pet đang đi trên cửa sổ khoảng
  0,8% tổng CPU; kéo cửa sổ liên tục gần như không tăng (luồng theo dõi cửa sổ khoảng 0,2% một nhân)
- `exit` của walk/run xoá việc đang làm, nên đổi việc từ bên ngoài (chạy trốn, đuổi, ngửi) phải đặt việc mới
  sau `exit` (`StateMachine.go` có `prepare`). Trước đó pet đang đi mà bị cửa sổ kéo tới thì chỉ chạy chơi
- Con trỏ chỉ được báo cho sim lúc vòng lặp đang chạy, nên lúc chạy lại không bị tính một cú giật chuột từ
  chỗ cũ tới chỗ mới. Né xét tốc độ tức thời trong mỗi bước chứ không chờ vận tốc làm mượt: cú lao chỉ
  khoảng 0,1 giây
- Overlay chỉ phủ một màn hình tại một lúc và đi theo pet. Tự đi sang chỉ qua mép trái/phải, nơi vùng làm
  việc của màn hình bên kia có chỗ cho chân pet (màn hình bên kia thấp hơn thì không sang). Đổi toạ độ
  giữa hai màn hình qua toạ độ desktop nên pet sang bên kia liền mạch; đang leo, đang nhảy đúng lúc đổi
  DPI thì buông ra rơi
- Frame choáng lấy từ atlas gốc bị lấm tấm (bóng đổ mềm của ảnh sinh ra, bị giảm màu), lúc lảo đảo lại
  bị vẽ lệch nửa pixel nên nhấp nháy như nhiễu. Script làm mịn riêng hàng choáng (lọc Kuwahara giữ
  viền và chi tiết tương phản cao), PetView thu nhỏ frame một lần rồi dời từng hàng đúng số nguyên
  pixel màn hình

**Xong khi:** pet đứng đúng mép cửa sổ ở scale 100–200%, CPU vẫn trong ngân sách.

### Phase 3 — Bộ lạc

- Nhiều pet, chỉ số energy, curiosity, bravery, friendliness, mischief, affection → trọng số chọn hành vi trên FSM
- Quan hệ từng cặp (bạn/ghét/thích) đổi theo tương tác: ngồi cạnh, chọc, đuổi, trốn, lẽo đẽo theo
- Mood, nhật ký sự kiện ngắn cho mỗi pet, speech bubble theo ngữ cảnh có giới hạn tần suất, skin

**Xong khi:** kịch bản kiểu Mochi–Pip (ngủ → bị chọc → đuổi → trốn sau cửa sổ → ngồi chờ) tự xảy ra, không viết cứng.

### Phase 4 — Thế giới sống

- Căn cứ góc màn hình: thùng carton → nhà nhỏ → cây, giường, bếp, máy arcade; tài nguyên (gỗ, xu, vải, đồ ăn) tăng theo thời gian chạy app, pet có nghề tự xây
- Nhu cầu ngủ/ăn/chơi → pet tự đi tới đồ vật
- Thời tiết chỉ quanh pet (mưa, tuyết, cánh hoa, sấm), ngày/đêm theo giờ thật, pet phản ứng
- Sự kiện hiếm không báo trước: UFO, mèo khổng lồ, ma lúc 2 giờ sáng, mũ sinh nhật

### Phase 5 — Thói quen user (chỉ trên máy, mặc định tắt)

- Thống kê app dùng theo giờ (chỉ tên process), xem/xoá được
- AFK qua thời điểm input cuối; nhận biết đang gõ phím không cần hook (có input nhưng chuột đứng yên)
- Nhắc khuya ("Still coding?", "bro go sleep"), VSCode mở 2 tiếng → pet mang gối tới
- Spam Ctrl+S → "bro it's saved 😭": cần hook bàn phím nên là tuỳ chọn riêng, chỉ đếm tổ hợp phím

### Phase 6 — Tiến hoá & colony

- Tiến hoá theo cách đối xử: friendly / independent / chaotic
- Trứng xuất hiện → nở → thêm pet; giới hạn số lượng; Keep / Send away
- Xuất/nhập `.pet` (có version, kiểm tra dữ liệu khi nhập) để trade

### Phase 7 — AI dialogue (tuỳ chọn)

- Chuột phải → Talk; context gồm tên, tính cách, quan hệ, sự kiện gần đây từ nhật ký Phase 3
- Tự bật; API key khoá bằng DPAPI như authenticator-app lưu token Google; không có mạng thì dùng câu mẫu

## Rủi ro kỹ thuật

| Rủi ro | Cách xử lý |
|---|---|
| Click-through đổi chậm → click đầu tiên lọt xuống app bên dưới | Đọc con trỏ khoảng 60 lần/giây; đang kéo thì giữ quyền nhận chuột |
| Toạ độ Win32 (pixel vật lý) lệch với CSS pixel khi DPI khác nhau | Một quy ước toạ độ (CSS pixel của overlay), Rust đổi toạ độ, có test |
| Overlay trong suốt phủ màn hình tốn GPU khi vẽ liên tục | Giới hạn fps, chỉ vẽ lại canvas của pet khi đổi frame, dừng vòng lặp khi không có gì chuyển động |
| Overlay đè lên video/game fullscreen | Tự ẩn khi app đang dùng chiếm trọn màn hình (Phase 1) |
| Overlay luôn trên mà che kín màn hình bị Windows coi là app fullscreen: tắt thông báo (Focus Assist), pet tự ẩn/hiện mỗi giây | Overlay chỉ phủ vùng làm việc; taskbar tự ẩn (vùng làm việc trùng màn hình) thì thấp đi 1 pixel. Phase 2 cần vẽ lên taskbar thì vẫn giữ quy tắc này |
| Hook bàn phím dễ bị antivirus nghi ngờ | Tuỳ chọn riêng, mặc định tắt, có thể bỏ |
| Mỗi lượt sinh lại sprite bằng AI có thể làm mất hoặc đổi màu tay chân mà `--check` không bắt được | Soát từng frame so với atlas gốc trước khi nhận sheet mới; lỗi nhỏ sửa bằng tuỳ chọn trong `PETS` của script thay vì sinh lại |
| App chưa ký số → SmartScreen cảnh báo | Như authenticator-app: hướng dẫn "More info → Run anyway" |
