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
| Riêng tư | Mọi dữ liệu xử lý trên máy, không đọc tiêu đề cửa sổ, tính năng nhạy cảm (bàn phím, thói quen) mặc định tắt. Chỉ gọi mạng cho thời tiết thật (toạ độ thành phố người dùng tự nhập) và AI dialogue (Phase 7, tuỳ chọn) |
| Thời tiết | Thời tiết thật từ Open-Meteo (Phase 4): miễn phí, không cần key hay tài khoản. Người dùng tự nhập thành phố, không đoán vị trí qua IP/GPS. Chưa nhập thành phố, mất mạng hoặc API lỗi thì dùng thời tiết giả lập |
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
| 2 | Desktop awareness | Đứng/leo/nhảy trên cửa sổ thật, ngủ trên taskbar, nhìn theo con trỏ, đa màn hình | Xong: 2a–2e đã thử trên máy thật (2 màn hình 1920×1080). Chỉ hỗ trợ scale 100% |
| 3 | Bộ lạc | Nhiều pet, tính cách, quan hệ, nhật ký sự kiện, speech bubble, skin | |
| 4 | Thế giới sống | Căn cứ + xây nhà, nhu cầu, thời tiết thật quanh pet (theo thành phố tự nhập), ngày/đêm, sự kiện hiếm | |
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
  alpha nhị phân và viền tối sẵn trong ảnh; pose leo/ngồi mép/nhảy làm sẵn cho Phase 2 (đưa vào `pet.json` ở bước 2b)

**Xong khi:** chạy 8 tiếng không rò RAM; unit test cho FSM và vật lý.

### Phase 2 — Desktop awareness

- Rust Win32: danh sách cửa sổ (khung thật qua DWM, thứ tự chồng, minimize, tên process), taskbar, màn hình + DPI; theo dõi thay đổi rồi gửi cho overlay
- Mép trên cửa sổ là nền để đứng, cạnh bên là tường để leo: ngồi mép, leo, nhảy giữa cửa sổ, rơi khi minimize/đóng, bám theo khi kéo cửa sổ, bị che khi cửa sổ khác đè lên, ngủ trên taskbar, chạy trốn khi cửa sổ bị kéo tới, ăn mừng khi app đóng
- Con trỏ: pet đứng yên thì quay đầu nhìn theo (đuổi, né, lại gần ngửi, giật mình ngã đã làm rồi bỏ: gây phiền lúc đang làm việc)
- Đa màn hình, chỉ hỗ trợ scale 100%

Chia 5 bước, xong bước nào thử trên máy thật bước đó:

| Bước | Nội dung | Trạng thái |
|---|---|---|
| 2a | `window_list.rs`: `SetWinEventHook` + đọc lại tối đa 30 lần/giây, chỉ gửi khi đổi, không đổi thì ngủ (2 giây đọc lại một lần). `packages/sim/src/terrain.ts`: mép trên là chỗ đứng, cạnh bên là tường, trừ phần bị cửa sổ nằm trên che. Pet đứng/đi trên mép, đi theo khi kéo cửa sổ, rơi khi thu nhỏ/đóng/hết chỗ đứng, thả lên mép thì đáp; bị che thì phần bị che không vẽ, không bắt chuột, lát sau đi ra | Xong, đã thử trên máy thật (Notepad làm địa hình): khung cửa sổ khớp tới từng pixel, pet đi theo khi kéo, rơi khi thu nhỏ/đóng/kéo sát mép trên, `closed` báo đúng cửa sổ bị đóng, phần bị che mất đúng chỗ |
| 2b | `climb`, `perch`, `jump` vào `pet.json` (21 pack). Leo lên/xuống cạnh cửa sổ, nhảy sang cửa sổ khác hoặc xuống taskbar, ngồi mép, buồn ngủ thì xuống taskbar rồi mới ngủ, click lúc đang leo thì tuột tay | Xong, đã thử trên máy thật: tay chạm đúng cạnh cửa sổ lúc leo, lên tới đỉnh nhún qua mép, nhảy sang cửa sổ khác đáp đúng chỗ ngắm, ngồi mép, buồn ngủ thì nhảy xuống taskbar rồi ngủ |
| 2c | Chạy trốn khi cửa sổ bị kéo tới (giữ nguyên cỡ mà lại gần dưới 90 px), ăn mừng khi app đóng (cách dưới 400 px, nhảy 2 cái, xác suất 0,8, 15 giây một lần). Cửa sổ lơ lửng: nhảy thẳng lên bám cạnh rồi leo | Xong, đã thử trên máy thật: nhảy lên bám góc dưới cửa sổ lơ lửng rồi leo lên mép; kéo cửa sổ tới thì chạy (trên mép cửa sổ hết đường thì leo xuống); đóng 3 lần thì cả 3 lần nhảy 2 cái |
| 2d | Con trỏ: đứng yên thì quay đầu nhìn theo con trỏ trong 300 px. Đã làm rồi bỏ hẳn (gây phiền lúc đang làm việc): lại gần ngửi khi con trỏ đứng yên, đuổi khi con trỏ lướt qua, né khi lao tới, nhảy dựng lên rồi ngã choáng khi giật chuột quét qua người | Xong, đã thử trên máy thật (bơm vị trí con trỏ vào sim): quay đầu theo con trỏ. Các hành vi đã bỏ không còn trong code, tuning và test |
| 2e | Đa màn hình và DPI: mỗi giây đo lại màn hình, đổi độ phân giải/DPI/taskbar hay rút màn hình thì đặt lại overlay (`screen-changed` kèm `remap`); kéo, ném hoặc tự đi ra khỏi mép giáp màn hình khác thì overlay sang bên đó | Xong, đã thử trên máy thật (2 màn hình 1920×1080 cạnh nhau, 100%, taskbar tự ẩn; điều khiển pet qua remote debugging): tự đi sang và đi về cả hai chiều, `move_overlay` gọi đúng một lần, pet đi tiếp liền mạch; ném cao bay qua mép không giật; ném sát đất thì chạm đất bên ngoài mép rồi sang; thả sát mép hơi hất ra thì đáp luôn; kéo sang màn hình kia vẫn đang cầm, buông thì rơi; bật/tắt tự ẩn taskbar thì pet lên/xuống mặt đất mới trong 1 giây; taskbar tự ẩn trồi lên thì pet đứng lên trên, thụt xuống thì rơi xuống đáy. Trước đó (1 màn hình): đo lại mỗi giây không gửi event thừa, CPU luồng Rust lúc pet ngủ dưới 0,1% một nhân. Chỉ hỗ trợ scale 100%, không thử DPI khác |

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
- `exit` của walk/run xoá việc đang làm, nên đổi việc từ bên ngoài (chạy trốn) phải đặt việc mới sau
  `exit` (`StateMachine.go` có `prepare`). Trước đó pet đang đi mà bị cửa sổ kéo tới thì chỉ chạy chơi
- Con trỏ chỉ để pet quay đầu nhìn theo. Đuổi, né, lại gần ngửi, giật mình ngã đã bỏ: pet tự chạy tới
  chỗ con trỏ, nhảy dựng lên khi lia chuột, gây phiền lúc đang làm việc
- Giờ chờ overlay sang (`crossTimeout`, 1,5 giây) chỉ tính từ lúc giữa thân pet qua mép. Tính từ lúc bắt
  đầu đi ra thì pet 96 px đi 30 px/s không bao giờ kịp qua mép (test sim dùng pet 60 px nên không bắt được)
- Ném qua mép mà chạm đất lúc giữa thân đã qua mép thì đứng chờ overlay sang, không bị kéo về; chưa qua
  mép thì overlay không sang nên đáp luôn trong màn hình này. Màn hình đổi tại chỗ (taskbar, DPI) lúc pet
  đang đi ra mép thì vào lại trong màn hình, không đi mãi ra ngoài
- Taskbar tự ẩn: vùng làm việc trùng cả màn hình, overlay phủ hết (bớt 1 pixel). Taskbar trồi lên che mép
  dưới overlay thì Rust gửi mép trên của nó kèm danh sách cửa sổ (`taskbarTop`, theo cùng hook
  `SetWinEventHook`), frontend lấy đó làm mặt đất: pet đứng lên trên, taskbar thụt xuống thì pet rơi xuống
- Bị ném ra quá mép thì bị giữ ngay ngoài mép, nhớ vận tốc ngang: overlay sang lúc pet còn đang rơi thì
  bay tiếp vào màn hình mới; đã chạm đất ngoài mép thì overlay sang rồi đi bộ vào, không giật vào trong
- Hiệu năng (bản debug, máy 4 nhân, đo cả tinyworld lẫn các tiến trình WebView2):
  - Chuột di chuyển thì Rust chỉ gửi vị trí khi con trỏ ở gần pet (`set_cursor_interest`). Trước đó
    WebView nhận khoảng 50 event/giây dù pet ngủ ở tít đáy màn hình: pet ngủ mà chuột di chuyển tốn
    2,27% một nhân, nay 0% (chuột trên cửa sổ app khác) hoặc 0,16% (chuột trên desktop)
  - Windows báo `EVENT_OBJECT_LOCATIONCHANGE` cả khi con trỏ di chuyển, gán cho process của cửa sổ dưới
    con trỏ; hook mọi process thì luồng theo dõi cửa sổ thức dậy mỗi lần chuột nhích. Pet ngủ thì chỉ hook
    việc di chuyển của explorer (taskbar tự ẩn vẫn bắt ngay), thức dậy thì hook lại toàn bộ và đọc ngay;
    overlay ẩn (app fullscreen, game) thì gỡ hẳn hook, luồng đọc con trỏ cũng đọc thưa lại
  - Vòng lặp vẽ hẹn giờ rồi mới xin rAF: đi bộ 30 lần/giây thay vì 60–144; đứng yên, ngồi mép chỉ thức
    dậy lúc đổi frame, khoảng 7 lần/giây. Pet thức (đi, đứng xen kẽ) từ 2,58% xuống khoảng 2%; chạy
    liên tục 30 lần/giây thì phần lớn là renderer và GPU ghép hình
  - Pet ngủ hoặc overlay ẩn thì WebView2 `MemoryUsageTargetLevel = Low`: working set cả nhóm tiến trình
    từ khoảng 360 MB xuống 60–140 MB (renderer 84 MB còn 2–18 MB), private từ 162 MB xuống 146 MB
  - Mặt nạ alpha 1 bit mỗi pixel, tính sẵn khung chứa phần có hình: 45 frame từ 1,58 MB còn 203 KB,
    JS heap sau GC còn 2,4 MB
- Tự ẩn khi fullscreen: cửa sổ đang dùng phủ kín cả màn hình, kể cả chỗ taskbar. Cửa sổ phóng to còn thanh
  tiêu đề không tính: taskbar tự ẩn hay màn hình không có taskbar thì nó cũng phủ kín màn hình. Phóng to
  mà bỏ viền (cách WPF, WinForms làm fullscreen) vẫn tính
- Overlay chỉ phủ một màn hình tại một lúc và đi theo pet. Tự đi sang chỉ qua mép trái/phải, nơi vùng làm
  việc của màn hình bên kia có chỗ cho chân pet (màn hình bên kia thấp hơn thì không sang). Đổi toạ độ
  giữa hai màn hình qua toạ độ desktop nên pet sang bên kia liền mạch; đang leo, đang nhảy đúng lúc đổi
  DPI thì buông ra rơi
- Frame choáng lấy từ atlas gốc bị lấm tấm (bóng đổ mềm của ảnh sinh ra, bị giảm màu), lúc lảo đảo lại
  bị vẽ lệch nửa pixel nên nhấp nháy như nhiễu. Script làm mịn riêng hàng choáng (lọc Kuwahara giữ
  viền và chi tiết tương phản cao), PetView thu nhỏ frame một lần rồi dời từng hàng đúng số nguyên
  pixel màn hình

**Xong khi:** pet đứng đúng mép cửa sổ ở scale 100% (chỉ hỗ trợ 100%), CPU vẫn trong ngân sách.

### Phase 3 — Bộ lạc

- Nhiều pet, chỉ số energy, curiosity, bravery, friendliness, mischief, affection → trọng số chọn hành vi trên FSM
- Quan hệ từng cặp (bạn/ghét/thích) đổi theo tương tác: ngồi cạnh, chọc, đuổi, trốn, lẽo đẽo theo
- Mood, nhật ký sự kiện ngắn cho mỗi pet, speech bubble theo ngữ cảnh có giới hạn tần suất, skin

**Xong khi:** kịch bản kiểu Mochi–Pip (ngủ → bị chọc → đuổi → trốn sau cửa sổ → ngồi chờ) tự xảy ra, không viết cứng.

### Phase 4 — Thế giới sống

- Căn cứ góc màn hình: thùng carton → nhà nhỏ → cây, giường, bếp, máy arcade; tài nguyên (gỗ, xu, vải, đồ ăn) tăng theo thời gian chạy app, pet có nghề tự xây
- Nhu cầu ngủ/ăn/chơi → pet tự đi tới đồ vật
- Thời tiết chỉ quanh pet (mưa, tuyết, sương mù, sấm, cánh hoa), ngày/đêm theo giờ thật, pet phản ứng
- Thời tiết thật từ [Open-Meteo](https://open-meteo.com):
  - Cài đặt có ô **Thành phố**: gõ tên rồi chọn trong danh sách, tìm bằng API geocoding
    (`geocoding-api.open-meteo.com/v1/search`). Chỉ lưu tên và toạ độ trong `settings.json`. Ô trống là
    thời tiết giả lập
  - Rust hỏi thời tiết hiện tại (`api.open-meteo.com/v1/forecast`, `current=weather_code,is_day,temperature_2m,wind_speed_10m`)
    lúc mở app, lúc đổi thành phố, rồi 30 phút một lần, gửi cho overlay qua event. Webview không gọi mạng.
    Kết quả gần nhất được lưu lại, mở app lúc chưa có mạng vẫn có thời tiết
  - Mã WMO thành hiệu ứng: 0–3 quang/nhiều mây, 45/48 sương mù, 51–67 và 80–82 mưa, 71–77 và 85–86
    tuyết, 95–99 sấm. Cánh hoa không có trong dữ liệu: rơi vào mùa xuân những ngày quang. Ngày/đêm theo
    `is_day` (giờ mặt trời mọc/lặn thật); chưa có thành phố thì theo giờ máy
  - Mất mạng, API lỗi hay quá hạn mức thì dùng kết quả gần nhất nếu chưa quá vài giờ, không thì dùng thời
    tiết giả lập (đổi ngẫu nhiên theo mùa). Lỗi thì chờ lâu dần mới hỏi lại, không hỏi liên tục
  - Hạn mức miễn phí (dùng phi thương mại): dưới 10.000 lần/ngày, 5.000 lần/giờ,
    600 lần/phút. App chỉ gọi khoảng 50 lần/ngày
  - Dữ liệu theo giấy phép CC BY 4.0: ghi "Dữ liệu thời tiết: Open-Meteo.com" cạnh ô Thành phố
- Sự kiện hiếm không báo trước: UFO, mèo khổng lồ, ma lúc 2 giờ sáng, mũ sinh nhật

**Xong khi:** đổi thành phố thì thời tiết quanh pet đổi theo trong vài giây; rút mạng hay chưa nhập thành
phố thì vẫn có thời tiết giả lập, không có lỗi nào hiện ra.

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
| Overlay đè lên video/game fullscreen | Tự ẩn khi app đang dùng chiếm trọn màn hình (Phase 1); cửa sổ phóng to còn thanh tiêu đề không tính (Phase 2) |
| Overlay luôn trên mà che kín màn hình bị Windows coi là app fullscreen: tắt thông báo (Focus Assist), pet tự ẩn/hiện mỗi giây | Overlay chỉ phủ vùng làm việc; taskbar tự ẩn (vùng làm việc trùng màn hình) thì thấp đi 1 pixel. Phase 2 giữ quy tắc này: pet đứng trên mép taskbar, không vẽ lên taskbar; taskbar tự ẩn trồi lên thì pet đứng lên trên nó |
| Hook bàn phím dễ bị antivirus nghi ngờ | Tuỳ chọn riêng, mặc định tắt, có thể bỏ |
| Mỗi lượt sinh lại sprite bằng AI có thể làm mất hoặc đổi màu tay chân mà `--check` không bắt được | Soát từng frame so với atlas gốc trước khi nhận sheet mới; lỗi nhỏ sửa bằng tuỳ chọn trong `PETS` của script thay vì sinh lại |
| Open-Meteo lỗi, đổi API hoặc chặn vì quá hạn mức | Không có thời tiết thật thì dùng thời tiết giả lập; 30 phút mới hỏi một lần, lỗi thì chờ lâu dần |
| App chưa ký số → SmartScreen cảnh báo | Như authenticator-app: hướng dẫn "More info → Run anyway" |
