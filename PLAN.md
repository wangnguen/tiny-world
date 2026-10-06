# TinyWorld — Plan

Vài pet nhỏ sống trên desktop, tự đi lại cạnh cửa sổ, taskbar và con trỏ chuột trong lúc người dùng làm
việc cho đỡ nhàm; nhắc nghỉ, nhắc khuya, chat để tra cứu nhanh. Không phải game phải mở lên chơi. Ưu tiên
số 1: **nhẹ**.

## Hướng đã chốt

| Chủ đề | Quyết định |
|---|---|
| Tên | TinyWorld, identifier `com.tinyworld.app` (dữ liệu ở `%APPDATA%\com.tinyworld.app\`) |
| Nền tảng | Chỉ Windows 10/11. Desktop awareness dùng Win32, CI chỉ build NSIS |
| Stack | Tauri v2 + Vite + TypeScript, backend Rust. Không Electron, không game engine, không server |
| Vẽ pet | Mỗi pet một `<canvas>` nhỏ, di chuyển bằng CSS transform, vòng lặp `requestAnimationFrame` tối đa 30 fps. React chỉ dùng cho cửa sổ Settings (Phase 1) |
| Logic pet | `packages/sim`: TS thuần (FSM, vật lý, RNG có seed), test bằng vitest |
| Click-through | Overlay mặc định để chuột đi xuyên. Rust đọc con trỏ khoảng 60 lần/giây gửi sang, overlay kiểm tra theo alpha của sprite, con trỏ nằm trên pet thì tắt click-through |
| Asset | Nhân vật tự vẽ bằng imagegen: atlas gốc ở `assets/sprite-sources/`, `scripts/prepare-sprites.mjs` chuẩn hoá thành pack `assets/sprites/<pack>/` có `pet.json`. Không dùng asset của bên thứ ba. Chưa có pack thì dùng pet tạm vẽ bằng code |
| Riêng tư | Mọi dữ liệu xử lý trên máy, không đọc tiêu đề cửa sổ, tính năng nhạy cảm (bàn phím, thói quen) mặc định tắt. Chỉ gọi mạng cho thời tiết thật (toạ độ thành phố người dùng tự nhập) và chat với pet (Phase 6, tuỳ chọn) |
| Thời tiết | Thời tiết thật từ Open-Meteo (Phase 4): miễn phí, không cần key hay tài khoản. Người dùng tự chọn thành phố, không đoán vị trí qua IP/GPS; giờ, ngày và lịch sự kiện theo múi giờ của thành phố đó. Chưa chọn thành phố thì thời tiết giả lập theo mùa. Đã chọn mà mất mạng hay API lỗi thì pet nói "Không có mạng :))" / "Không có thời tiết :))", không giả lập |
| AI | Chỉ để chat với pet và tra cứu nhanh (Phase 6, tuỳ chọn), không điều khiển di chuyển. Rust gọi Gemini như trang gemini.google.com lúc chưa đăng nhập (`gemini.rs`): không cần key hay tài khoản, tự theo bản Flash mới nhất Google cho người chưa đăng nhập |
| Ngôn ngữ | UI, comment, README tiếng Việt |
| Build / phát hành | Mỗi lần push lên `main`, CI tự test rồi build `.exe` (bản cài + bản chạy thẳng), tải ở mục Artifacts của Actions; push nhiều lần thì chỉ build bản mới nhất, push chỉ sửa tài liệu (`.md`) thì không build. Phát hành chính thức dùng workflow Release chạy tay (nhập version) |
| Version | Bản Release mang version nhập lúc chạy workflow. Bản build khi push mang version sắp phát hành: commit đã phát hành thì đúng version đó, chưa thì tăng số cuối của release mới nhất (`v1.3.2` → `1.3.3`). Cả hai ghi vào `tauri.conf.json` trên máy build, không sửa file trong repo. Cửa sổ Cài đặt hiện version dưới tiêu đề, bản build kèm số lần chạy và commit |

### Quy ước code

- pnpm monorepo: `packages/core`, `packages/sim`, `desktop`; package nội bộ export thẳng source TS, `tsconfig.base.json` strict
- Rust: `main.rs → app.rs → commands.rs → module nghiệp vụ`; lỗi `AppError { code, message }` trả thẳng về frontend
- Frontend gọi Rust qua `desktop/src/api.ts`; types dùng chung ở `packages/core` khớp struct Rust (serde camelCase)
- Dữ liệu JSON trong `%APPDATA%\com.tinyworld.app\`, ghi ra file tạm rồi đổi tên; `assets/` giữ file gốc kèm README

## Lộ trình

| Phase | Tên | Kết quả chính | Trạng thái |
|---|---|---|---|
| 0 | Base | Monorepo, overlay trong suốt click-through, pet tạm, tray, CI | Xong: test pass, đã chạy thử trên máy thật (overlay trong suốt, click-through theo alpha, click → phản ứng) |
| 1 | MVP | 1 pet: tự đi, quay đầu ở mép, click phản ứng, kéo thả + rơi, bỏ mặc thì ngủ | Xong, đã thử trên máy thật: hành vi, vật lý, kéo thả/ném, Ctrl xuyên pet, không cướp focus, tự ẩn khi fullscreen, lưu trạng thái, tray Tạm dừng, Settings (nhân vật, cỡ, tốc độ, chạy cùng Windows). Đo 10 phút: heap JS ~2 MB không tăng; còn chạy thử 8 tiếng |
| 2 | Desktop awareness | Đứng/leo/nhảy trên cửa sổ thật, ngủ trên taskbar, nhìn theo con trỏ, đa màn hình | Xong: 2a–2e đã thử trên máy thật (2 màn hình 1920×1080). Chỉ hỗ trợ scale 100% |
| 3 | Nhiều pet | Tối đa 3 nhân vật cùng sống trên màn hình, tốn ít RAM và CPU | Xong phần code, test pass; đã chạy thử bản dev 1 màn hình (3 con, đọc file cài đặt cũ). Chưa thử ném sang màn hình kia |
| 4 | Vui nhẹ | Pet chơi với nhau, ngày/đêm, thời tiết thật quanh pet (theo thành phố tự chọn), lịch sự kiện, bấm đúp xem giờ/ngày/thời tiết; không làm phiền lúc làm việc | Xong phần code, test pass; đang chạy thử bản dev |
| 5 | Sức khoẻ & công việc | Giờ ngồi máy, nhắc nghỉ, nhắc uống nước, nhắc khuya, spam Ctrl+S (chỉ trên máy, tự bật) | Xong phần code, test pass (`cargo test`, sim); đã thử lời nhắc trên bản dev bằng event giả. Chưa đối chiếu giờ ngồi máy cả ngày |
| 6 | Chat với pet | Click pet để chat, hỏi đáp và tra cứu nhanh qua Gemini, đỡ phải mở trình duyệt (tuỳ chọn) | Xong phần code, test pass; đã gửi thật: chưa đăng nhập Google trả lời bằng 3.6 Flash trong khoảng 3 giây |

TinyWorld không phải game: không làm chỉ số tính cách, quan hệ giữa các pet, nhu cầu, xây nhà, tài
nguyên, tiến hoá, trứng nở hay trao đổi pet.

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
- Bị che hết thì rơi ngay ra trước mọi cửa sổ: mép đang đứng không còn chỗ nào không bị che (bấm vào
  cửa sổ phóng to nằm dưới cửa sổ pet đứng, kéo cửa sổ khác đè lên), hoặc chỗ đang bám trên cạnh bị che
  (buông tay). Xét mỗi bước, không chờ hết lượt đứng yên. Trước đó pet đi ra đầu mép rồi leo xuống cạnh
  cũng bị che, khuất sau cửa sổ cả chục giây như đã biến mất (thử trên máy thật: pet đứng trên cửa sổ
  lơ lửng, bấm vào trình duyệt phóng to). Bị che một phần thì vẫn đi ra chỗ không bị che; xuống khỏi
  mép thì chỉ leo cạnh không bị che, không thì nhảy khỏi mép
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

### Phase 3 — Nhiều pet

- Cài đặt chọn 1 đến 3 nhân vật khác nhau, thêm/bớt là áp dụng ngay. `settings.json` đổi `pet` thành
  danh sách `pets`; file cũ chỉ có `pet` thì đọc thành danh sách một con
- Mỗi con một id, `world.json` lưu theo id. Con mới xuất hiện rải dọc taskbar, không chồng lên nhau
- Từng con kéo thả, ném, click, leo cửa sổ như Phase 1–2. Các con đi xuyên qua nhau, không va chạm
- Tốn ít RAM và CPU:
  - Một overlay, một WebView, một vòng lặp vẽ cho cả nhóm; không mỗi con một cửa sổ
  - Mỗi con một canvas nhỏ, chỉ vẽ lại con vừa đổi frame. Chỉ nạp pack đang dùng, bớt con thì bỏ pack đó
  - Vòng lặp hẹn giờ theo con cần thức dậy sớm nhất: cả nhóm đứng yên thì vẫn chỉ thức lúc đổi frame
  - Ngủ chung: đồng hồ "không đụng tới" tính chung cả nhóm, nên cả nhóm cùng đi ngủ. Cả nhóm ngủ thì
    dừng hẳn vòng lặp và WebView2 dùng ít RAM như hiện nay; click bất kỳ đâu thì cả nhóm dậy
  - `set_cursor_interest` nhận danh sách vùng quanh từng con. Gộp thành một hình chữ nhật thì hai con ở
    hai đầu màn hình phủ gần hết màn hình, chuột di chuyển ở đâu WebView cũng thức dậy
- Hai màn hình, vẫn một overlay cho cả nhóm: từ 2 con trở lên thì pet không tự đi sang màn hình khác
  (mép giáp màn hình khác như mép thường). Kéo hoặc ném một con sang thì overlay sang, các con còn lại
  đi vào từ mép đó. Chỉ có một con thì như Phase 2
- Cả nhóm ít khi cùng chạy nhảy một lúc: mỗi con khác đang đi lại làm con này bớt 35% khả năng đi lại
  (`groupCalm`), đỡ rối mắt và vòng lặp được nghỉ nhiều hơn

| Bước | Nội dung | Trạng thái |
|---|---|---|
| 3a | Cài đặt chọn nhiều nhân vật; overlay hiện đủ các con, click/kéo thả từng con; `world.json` theo id; vùng con trỏ theo danh sách | Xong, đã chạy thử bản dev: thêm con từ Cài đặt thì hiện ngay dọc taskbar, mở lại app thì về đúng chỗ, `settings.json` cũ (`pet`) đọc thành `pets`, vẫn ghi `pet` (con đầu tiên) để bản cũ đọc được |
| 3b | Ngủ chung, cả nhóm ngủ thì dừng vòng lặp; đo CPU, RAM với 1 con và 3 con | Xong (test sim). Đo bản debug, máy 4 nhân, cửa sổ Cài đặt đóng: 1 con thức 0,49% tổng CPU, working set 316 MB; 3 con thức 1,07%, 361 MB (đo trước khi có `groupCalm`). Mở cửa sổ Cài đặt thì thêm một renderer, đo lẫn vào sẽ sai |
| 3c | Hai màn hình: cả nhóm sang theo con bị kéo hoặc ném | Xong phần code (test sim): con gần mép vào trước, con xa vào sau, xuất hiện cách mép tối đa 320 px; con đứng trên cửa sổ ở màn hình cũ thì xuống đất chạy theo; con đang ngủ nằm ở sát mép. Chưa thử trên máy thật |

**Xong khi:** 3 con chạy cùng lúc trên máy thật; cả nhóm ngủ thì CPU, RAM như lúc 1 con ngủ; 3 con thức
thì đo và ghi lại CPU so với 1 con, cao quá thì giới hạn số con đi lại cùng lúc.

### Phase 4 — Vui nhẹ

Không làm phiền lúc làm việc:

- Không âm thanh, không popup, không tự chạy về phía con trỏ
- Speech bubble ngắn. Câu nói cho vui (chào nhau, kêu trời mưa) cả nhóm 15 phút mới được một câu
  (`chatGap`), tối đa 4 câu mỗi giờ; đang gõ phím (Phase 5) thì không nói
- Mỗi thứ có công tắc tắt trong tab **Thế giới** của Cài đặt
- Chỉ dùng animation có sẵn hoặc vẽ bằng code (bubble, hạt mưa, con ma). Không sinh animation mới

Nội dung:

- Pet chơi với nhau: gặp nhau thì dừng lại quay mặt vào nhau, có lúc chào một câu, có lúc đi cùng một đoạn,
  đi ngủ thì nằm cạnh nhau. Không có chỉ số hay quan hệ
- Ngày/đêm: đêm đi chậm hơn (`nightPace`), buồn ngủ sớm hơn (`sleepAfterNight`). Có thời tiết thật thì
  theo `is_day` (giờ mặt trời mọc/lặn thật), không thì 19:00–6:00 theo giờ ở thành phố đã chọn
- Thời tiết chỉ quanh pet (mưa, tuyết, sương mù, sấm, cánh hoa): canvas nhỏ đi theo từng con, vẽ 12 fps,
  hạt mờ dần ra xa. Trời quang, nhiều mây hay pet đang ngủ thì không vẽ gì. Có sấm thì chớp sáng và cả nhóm
  giật mình. Trời đổi kiểu thì một con nói một câu (theo giới hạn câu nói cho vui)
- Bấm đúp vào một con: con đó nói giờ, thứ, ngày dương lịch, ngày âm lịch, và thời tiết ở thành phố đã
  chọn (`nowText`), ví dụ `15:04 · Thứ Năm 01/10`, `Âm lịch 21/8`, `Hà Nội: mưa, 27°C`. Âm lịch tính
  trên máy (thuật toán Hồ Ngọc Đức, múi giờ +7), không gọi mạng
- Thành phố và giờ theo thành phố:
  - Tab **Thế giới** có ô **Thành phố**: gõ tên rồi chọn trong danh sách, tìm bằng API geocoding
    (`geocoding-api.open-meteo.com/v1/search`). Chỉ lưu tên, toạ độ và múi giờ IANA trong
    `settings.json`. Chưa chọn là thời tiết giả lập, giờ theo máy
  - Giờ khi bấm đúp, ngày/đêm, ngày của lịch sự kiện và giờ con ma đều theo múi giờ của thành phố đó
    (`wallClock`)
- Thời tiết thật từ [Open-Meteo](https://open-meteo.com):
  - Rust hỏi thời tiết hiện tại (`api.open-meteo.com/v1/forecast`, `current=weather_code,is_day,temperature_2m,wind_speed_10m`)
    lúc mở app, lúc đổi thành phố, rồi 30 phút một lần, gửi qua event `weather-changed`. Webview không gọi
    mạng. Kết quả gần nhất lưu ở `weather.json`, mở app lúc chưa có mạng vẫn dùng được nếu chưa quá 3 giờ
  - Mã WMO thành hiệu ứng: 0–3 quang/nhiều mây, 45/48 sương mù, 51–67 và 80–82 mưa, 71–77 và 85–86
    tuyết, 95–99 sấm. Cánh hoa không có trong dữ liệu: rơi vào tháng 2–4 những ngày quang
  - Lỗi thì chờ lâu dần mới hỏi lại (1 phút, gấp đôi mỗi lần, tối đa 30 phút). Lần lỗi đầu của mỗi đợt
    Rust gửi `weather-failed`: một con nói "Không có mạng :))" (mất mạng, hết giờ chờ) hoặc "Không có
    thời tiết :))" (API trả lỗi); bấm đúp lúc đó cũng nói câu này thay cho thời tiết. Không có kết quả còn
    dùng được thì không có hiệu ứng thời tiết, không giả lập
  - Hạn mức miễn phí (dùng phi thương mại): dưới 10.000 lần/ngày, 5.000 lần/giờ, 600 lần/phút. App chỉ gọi
    khoảng 50 lần/ngày
  - Dữ liệu theo giấy phép CC BY 4.0 (ghi nguồn kèm link cạnh chỗ hiện dữ liệu): một dòng nhỏ "Thời tiết từ
    Open-Meteo.com" dưới ô Thành phố, chỉ khi đã chọn thành phố hoặc đang tìm; trên pet không ghi
- Chưa chọn thành phố: thời tiết giả lập theo mùa, đổi mỗi 3 tiếng; cùng ngày, cùng khung giờ thì luôn ra
  cùng một kiểu nên mở lại app không bị đổi
- Lịch sự kiện (chỉnh trong tab **Thế giới**): mỗi dịp có tên, ngày/tháng dương hoặc âm lịch, kéo dài
  1–10 ngày và một câu nói. Mặc định có Tết Nguyên Đán, Tết Dương lịch, Giỗ Tổ Hùng Vương, 30/4, 1/5, 2/9,
  Trung thu, Giáng sinh; bật tắt, sửa, xoá từng dịp, thêm dịp riêng (sinh nhật, ngày kỷ niệm), tối đa 30
  dịp. Đúng dịp thì một con nói câu của dịp đó mỗi ngày một lần. Không làm: đội mũ theo dịp (đặt mũ lên
  đầu bằng đo tự động lệch ở nhiều nhân vật, nhìn xấu)
- Sự kiện hiếm: 2:00–2:30 sáng, có con còn thức thì một con ma bay ngang qua, mỗi đêm một lần; con nào
  nó bay qua thì giật mình, con đầu tiên kêu "Ma... ma kìa :((("

| Bước | Nội dung | Trạng thái |
|---|---|---|
| 4a | Pet chơi với nhau, ngày/đêm, giới hạn câu nói cho vui | Xong (test sim) |
| 4b | Thời tiết thật/giả lập quanh pet, chọn thành phố, giờ theo thành phố, báo mất mạng, bấm đúp xem giờ/ngày/âm lịch/thời tiết | Xong phần code (test sim, `cargo test`); đang chạy thử bản dev |
| 4c | Lịch sự kiện chỉnh được trong Cài đặt (câu chúc), con ma lúc 2 giờ sáng | Xong phần code (test sim, `cargo test`); đang chạy thử bản dev |

**Xong khi:** đổi thành phố thì thời tiết quanh pet đổi theo trong vài giây; rút mạng thì pet báo một lần
"Không có mạng :))", không có lỗi nào khác hiện ra; chưa chọn thành phố thì vẫn có thời tiết giả lập; để
pet chạy cả ngày làm việc thì bubble và sự kiện không vượt giới hạn tần suất.

### Phase 5 — Sức khoẻ & công việc (chỉ trên máy, mặc định tắt)

Tất cả nằm ở tab **Sức khoẻ** trong Cài đặt, mặc định tắt. Rust (`activity.rs`) đọc mỗi giây một lần,
không hook bàn phím hay chuột:

- Giờ ngồi máy: theo thời điểm input cuối (`GetLastInputInfo`, chỉ biết lúc nào có phím hay chuột, không
  biết phím gì). Ngồi máy là khoảng giữa hai lần input cách nhau dưới 5 phút; xa hơn là vắng, không tính.
  Lưu theo ngày (giờ máy) trong `stats.json`, giữ 30 ngày: giờ ngồi máy, lượt ngồi liền lâu nhất, số lần
  đứng dậy nghỉ (vắng từ 5 phút sau một lượt ngồi). Tab Sức khoẻ hiện bốn ô (hôm nay, đang ngồi liền, lâu
  nhất, đã nghỉ mấy lần), cột 7 ngày gần nhất (rê chuột vào cột thì ô số nổi trên cột), nút xoá hết.
  Lượt ngồi liền hiện ra chỉ tính từ lúc bật đếm giờ; nhắc nghỉ thì tính cả lúc chưa bật
- Không đếm theo từng app (đã bỏ: ít hữu ích, lại phải đọc tên process)
- Nhận biết đang gõ phím mà không cần hook: có input nhưng chuột đứng yên (luôn bật, không lưu gì). Đang
  gõ thì pet không nói câu cho vui
- Nhắc nghỉ: ngồi liền 50 phút (chọn 30/45/50/60/90) thì một con nhảy lên nhắc "Ngồi liền 50 phút rồi,
  đứng dậy nghỉ mắt chút đi :)))", cả nhóm đang ngủ thì một con dậy nhắc; vắng 5 phút là tính lại. Bấm vào
  pet là tắt lời nhắc đó (bấm vào con nào đang nói cũng tắt bubble)
- Nhắc uống nước: cứ ngồi máy đủ 60 phút (chọn 30/45/60/90/120, cộng dồn qua các lần nghỉ) thì pet nhắc
  "Uống ngụm nước đi nè :)))"; trùng lúc nhắc nghỉ thì nhắc nghỉ trước
- Nhắc khuya: sau giờ đặt (mặc định 23:00) tới 5:00 sáng mà vẫn ngồi máy (có input trong 1 phút) thì pet
  nhắc "23:40 rồi, đi ngủ thôi :(((", tối đa 30 phút một lần
- Spam Ctrl+S (5 lần trong 10 giây) → "Lưu rồi mà :((((". `cursor.rs` đã đọc phím Ctrl khoảng 60
  lần/giây bằng `GetAsyncKeyState`; bật tuỳ chọn này thì đọc thêm phím S, chỉ lúc đang giữ Ctrl. Không ghi
  lại phím nào
- Lời nhắc không qua giới hạn câu nói cho vui; overlay đang ẩn hay tạm dừng thì bỏ qua lời nhắc

**Xong khi:** giờ trong tab Thống kê khớp với thời gian ngồi máy thật (lệch dưới vài phút mỗi ngày); tắt
tính năng thì không đọc gì nữa; bấm xoá thì mất hết số liệu đã lưu.

### Phase 6 — Chat với pet (tuỳ chọn, mặc định tắt)

Mục tiêu: hỏi nhanh, tra cứu vài thông tin hữu ích ngay trên desktop, đỡ phải mở trình duyệt hay Google.

- Bật trong Cài đặt → Pet → **Chat với pet** (mặc định tắt; chưa bật mà click chuột phải vào pet thì pet
  nhắc bật). Click chuột phải vào pet: cửa sổ chat nhỏ cạnh pet (bên trái nếu đủ chỗ), chỉ tạo khi mở và
  huỷ khi đóng như cửa sổ Cài đặt (overlay không kéo theo khung chat); đang mở mà click con khác thì đổi
  sang con đó, bắt đầu đoạn chat mới. Pet trả lời theo giọng nhân vật nhưng ưu tiên đúng và gọn; markdown
  (đậm, gạch đầu dòng, code, link) vẽ thành chữ, không chèn HTML; link bấm được (Rust mở trình duyệt, chỉ
  http/https), nút Chép câu trả lời. Lịch sử chỉ giữ trong lúc cửa sổ còn mở. Tắt chat thì cửa sổ chat
  đóng luôn
- Rust gọi Gemini như trang gemini.google.com lúc chưa đăng nhập (`gemini.rs`, tự viết, chỉ dùng `reqwest`
  có sẵn): không cần key, cookie hay tài khoản, không chạy server, không mở cổng localhost. Webview không gọi
  mạng. Gửi dạng chat tạm (không vào lịch sử Gemini)
- Model: chọn bằng header `x-goog-ext-525001261-jspb` (mã model + loại Flash) như trang Gemini. Build label
  và mã Flash đọc từ chính trang `/app` (6 tiếng một lần, chỉ khi có người chat; đọc lỗi thì dùng giá trị
  dự phòng), nên Google lên bản Flash mới thì app tự theo, không cần cập nhật app
  - Đã thử (10/2026): chưa đăng nhập thì Google trả lời bằng **3.6 Flash** (phản hồi có ghi tên model). Gửi
    kiểu cũ của gemini-web2api (không có header) thì mọi model đều ra 3.5 Flash-Lite. Pro và 3.8 Flash (gói
    AI Pro/Ultra) thì Google tự đổi về bản nhẹ hơn
  - Không hiện tên model ở đâu cả: người dùng chỉ cần biết đang chat với pet nào
- Cửa sổ chat đặt sát cạnh pet (bên trái nếu đủ chỗ), ngang tầm giữa pet, tính cả viền và thanh tiêu đề
  để luôn nằm trọn trên taskbar. Con đang chat đứng yên quay mặt về phía khung chat (không ngủ, không bị con
  khác rủ đi) tới khi đóng khung chat
- Đầu khung chat là cảnh đêm cùng kiểu Cài đặt, pet đứng trên đồi (chỉ nạp một ảnh `idle`)
- Câu gợi ý lúc chưa hỏi gì: Gemini viết 8 câu hợp với ngày hôm nay, mỗi ngày hỏi một lần (giữ trong bộ
  nhớ, lỗi thì 1 tiếng sau mới hỏi lại, Google đang bận thì không hỏi), khung chat bốc ngẫu nhiên 4 câu;
  trong lúc chờ hay mất mạng thì bốc từ 12 câu có sẵn
- Tránh bị chặn IP (`chat.rs`, `Limiter` có test):
  - Chỉ gọi khi người dùng bấm gửi (hoặc bấm một gợi ý), không tự gọi nền; mỗi lúc chỉ một câu
  - Hai lần gửi cách nhau ít nhất 3 giây, tối đa 30 câu mỗi giờ; app tự chặn trước, câu chưa gửi trả lại ô
    nhập
  - Chỉ gửi kèm 6 lượt chat gần nhất, cả câu tối đa 6000 ký tự
  - Google trả lỗi, chặn (429) hay trả rỗng thì chờ 30 giây, gấp đôi mỗi lần, tối đa 30 phút; khung chat ghi
    "Google đang bận, gửi lại được sau 30 giây", không hiện lỗi kỹ thuật. Mất mạng, hết giờ chờ (60 giây)
    không tính là bị chặn
- Gửi kèm: tên và tính cách ngắn của nhân vật (`desktop/src/chat/prompt.ts`), ngày giờ máy. Không gửi gì
  khác (không thành phố, không cửa sổ đang mở)
- Mất mạng thì khung chat ghi "Mất mạng", không dùng câu mẫu

**Xong khi:** chat được từ pet, câu trả lời về trong vài giây; hỏi thông tin mới (tỉ giá, tin tức) thì trả
lời được; gửi dồn dập thì app chặn trước, không để Google chặn; rút mạng thì khung chat ghi "Mất mạng".

## Rủi ro kỹ thuật

| Rủi ro | Cách xử lý |
|---|---|
| Click-through đổi chậm → click đầu tiên lọt xuống app bên dưới | Đọc con trỏ khoảng 60 lần/giây; đang kéo thì giữ quyền nhận chuột |
| Toạ độ Win32 (pixel vật lý) lệch với CSS pixel khi DPI khác nhau | Một quy ước toạ độ (CSS pixel của overlay), Rust đổi toạ độ, có test |
| Overlay trong suốt phủ màn hình tốn GPU khi vẽ liên tục | Giới hạn fps, chỉ vẽ lại canvas của pet khi đổi frame, dừng vòng lặp khi không có gì chuyển động |
| Overlay đè lên video/game fullscreen | Tự ẩn khi app đang dùng chiếm trọn màn hình (Phase 1); cửa sổ phóng to còn thanh tiêu đề không tính (Phase 2) |
| Overlay luôn trên mà che kín màn hình bị Windows coi là app fullscreen: tắt thông báo (Focus Assist), pet tự ẩn/hiện mỗi giây | Overlay chỉ phủ vùng làm việc; taskbar tự ẩn (vùng làm việc trùng màn hình) thì thấp đi 1 pixel. Phase 2 giữ quy tắc này: pet đứng trên mép taskbar, không vẽ lên taskbar; taskbar tự ẩn trồi lên thì pet đứng lên trên nó |
| Theo dõi bàn phím dễ bị antivirus nghi ngờ | Không hook bàn phím: chỉ đọc trạng thái Ctrl và S bằng `GetAsyncKeyState` trong luồng đọc con trỏ có sẵn; tuỳ chọn riêng, mặc định tắt |
| 3 pet cùng thức tốn CPU gấp mấy lần 1 con | Một vòng lặp chung, ngủ chung, đo ở bước 3b; cao quá thì giới hạn số con đi lại cùng lúc |
| Mỗi lượt sinh lại sprite bằng AI có thể làm mất hoặc đổi màu tay chân mà `--check` không bắt được | Soát từng frame so với atlas gốc trước khi nhận sheet mới; lỗi nhỏ sửa bằng tuỳ chọn trong `PETS` của script thay vì sinh lại |
| Open-Meteo lỗi, đổi API hoặc chặn vì quá hạn mức | Không có thời tiết thật thì dùng thời tiết giả lập; 30 phút mới hỏi một lần, lỗi thì chờ lâu dần |
| Chat đi qua trang web Gemini (không phải API chính thức): Google đổi trang thì hỏng, gọi nhiều thì bị chặn IP | Build label và mã model đọc từ chính trang nên đổi bản thường không hỏng; giới hạn tần suất ngay trong app, lỗi thì chờ lâu dần, khung chat ghi ngắn gọn lý do. Đổi cách gửi thì sửa `gemini.rs`. Chat là tuỳ chọn: hỏng thì phần còn lại của app vẫn chạy |
| App chưa ký số → SmartScreen cảnh báo | Hướng dẫn "More info → Run anyway" trong README và trang Release |
