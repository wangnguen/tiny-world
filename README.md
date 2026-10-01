# TinyWorld

Pet sống trên desktop Windows: app **Tauri v2 + Vite + TypeScript**, backend Rust. Nhân vật được vẽ
trên một cửa sổ overlay trong suốt, luôn nằm trên cùng, chuột đi xuyên qua trừ khi trỏ vào pet.
Lộ trình và các hướng đã chốt: [PLAN.md](PLAN.md).

```
desktop-pet/
├── assets/
│   ├── icon.png          # logo gốc của app
│   ├── sprites/          # sprite pack chạy trong app: <pack>/pet.json + WebP
│   └── sprite-sources/   # atlas, prompt gốc của các nhân vật và trang xem animation
├── packages/
│   ├── core/             # types dùng chung Rust <-> TS, tên event, đọc/kiểm tra pet.json
│   └── sim/              # engine mô phỏng TS thuần: FSM, bước thời gian cố định, RNG có seed
├── desktop/              # Tauri v2
│   ├── src/              # overlay (vẽ pet bằng canvas, TS thuần) và src/settings/ (cửa sổ cài đặt, React)
│   └── src-tauri/        # Rust: overlay, click-through, đọc con trỏ, tray, cài đặt, lưu trạng thái
└── scripts/              # chuẩn hoá sprite từ atlas nguồn, tạo icon
```

## Nhân vật

21 nhân vật tự vẽ cho dự án. Chọn trong **Cài đặt… → Nhân vật**, bấm một ô để thêm hoặc bớt nhân vật
đó: tối đa 3 con cùng sống trên màn hình, luôn còn ít nhất một con (16 nhân vật mỗi trang, lật bằng
mũi tên, chấm trang hoặc lăn chuột). Gồm Momo (mặc định), Bông, Kitsu, Mầm, Bíp, Lumi, Nấm, Mây, Tàn,
Rêu, Cục, Mực, Dứa, Su, Bắp, Boggo, Gloop, Bẹp, Frobu, Byte và Patch. Nhân vật nào cũng đủ 12
animation: 9 của Phase 1 cộng leo, ngồi mép, nhảy. Xem chuyển động bằng [sprite studio](assets/sprite-sources/index.html) (mở
thẳng bằng trình duyệt), thiết kế từng nhân vật ở [assets/sprite-sources/README.md](assets/sprite-sources/README.md).

Định dạng `pet.json` và cách thêm nhân vật: [assets/README.md](assets/README.md).

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
| **Cài đặt…** | Tab **Pet**: chọn 1–3 nhân vật, cỡ nhân vật (50–200%), tốc độ đi lại, chat với pet (mặc định tắt), chạy cùng Windows. Tab **Thế giới**: thành phố (thời tiết thật, giờ ở đó), bật tắt thời tiết quanh pet, câu nói cho vui, lịch sự kiện, con ma lúc 2 giờ sáng; sửa lịch sự kiện. Tab **Sức khoẻ** (mặc định tắt hết): giờ ngồi máy hôm nay và 7 ngày, nhắc nghỉ, nhắc uống nước, nhắc đi ngủ, spam Ctrl+S. Đổi là áp dụng ngay. Dưới tiêu đề là version đang chạy |
| **Thoát** | Lưu trạng thái pet rồi thoát |
| **Mở DevTools** | Chỉ khi chạy dev (overlay để chuột đi xuyên nên không bấm F12 được) |

Khi chạy dev, `window.__tinyworld` trong DevTools cho xem và chỉnh pet (`pet` là con đầu tiên,
`members` là cả nhóm), ví dụ `__tinyworld.world.pets.forEach((p) => (p.sinceInteraction = 1e6))` để
cả nhóm đi ngủ ngay. Bật **Chạy cùng Windows** trong lúc chạy dev
sẽ ghi đường dẫn exe bản debug, nhớ tắt lại.

Tương tác với pet:

| Thao tác | Pet |
|---|---|
| Để yên | Tự đứng, đi, chạy; chạm mép màn hình thì quay đầu. Chỉ có một con thì mép giáp màn hình khác có lúc đi sang bên đó; từ 2 con trở lên thì cả nhóm ở yên một màn hình. Các con đi xuyên qua nhau |
| Có cửa sổ gần | Leo cạnh cửa sổ lên mép trên (cửa sổ lơ lửng thì nhảy lên bám cạnh rồi leo, đáy cao hơn chỗ đứng quá khoảng 270 px thì chịu), đi lại trên đó, ngồi ở mép, nhảy sang cửa sổ khác hoặc nhảy, leo xuống |
| Kéo một cửa sổ khác lại sát pet | Pet chạy trốn về phía ngược lại; đang ở trên cửa sổ mà hết đường thì xuống luôn, dưới đất bị dồn vào mép màn hình thì giật mình nhảy lên. Phóng to hay đổi cỡ cửa sổ không tính |
| Đóng một cửa sổ ở gần (trong khoảng 400 px) | Pet quay về phía đó nhảy cẫng lên hai cái ăn mừng (không phải lần nào cũng vậy, 15 giây mới ăn mừng một lần) |
| Kéo cửa sổ pet đang đứng | Pet đi theo cửa sổ; kéo lên sát mép trên màn hình thì hết chỗ đứng, pet rơi |
| Thu nhỏ hoặc đóng cửa sổ đó | Pet rơi xuống cửa sổ bên dưới hoặc taskbar |
| Cửa sổ khác đè lên | Pet bị che mất phần nằm dưới cửa sổ đó (click vào đó là click cửa sổ), lát sau đi ra chỗ không bị che. Bị che hết (ví dụ bấm vào cửa sổ phóng to nằm dưới) thì rơi ngay ra trước mọi cửa sổ, đang leo thì buông tay |
| Con trỏ ở gần | Đứng yên thì quay về phía con trỏ |
| Click | Nhảy lên một cái rồi đi hoặc chạy tiếp; đang ngủ thì thức dậy; đang leo thì tuột tay. Click dồn dập chỉ nhảy một lần |
| Kéo lên rồi thả | Rơi xuống mép cửa sổ bên dưới hoặc taskbar, nảy nhẹ khi chạm đất; thả từ cao thì choáng: lảo đảo, sao bay quanh đầu |
| Kéo rồi vung chuột và buông | Bị ném bay theo quán tính, đập tường thì bật lại; ném về phía màn hình khác thì bay sang bên đó |
| Kéo pet sang màn hình khác | Pet sang màn hình đó theo con trỏ |
| Có nhiều con, ném hoặc kéo một con sang màn hình khác | Các con còn lại chạy theo, vào từ mép phía màn hình cũ: con ở gần mép vào trước, con ở xa vào sau; con đang ngủ thì nằm luôn ở sát mép đó |
| Bấm vào chỗ hai con chồng lên nhau | Con nằm trên được bấm; con vừa bị bấm, bị kéo nằm lên trên các con khác |
| Bấm đúp | Con đó nói giờ, thứ, ngày, ngày âm lịch, và thời tiết ở thành phố đã chọn (mất mạng thì "Không có mạng :))") |
| Click chuột phải | Mở cửa sổ chat với con đó ngay cạnh nó (bật **Chat với pet** trong Cài đặt trước); con đó đứng yên chờ tới khi đóng khung chat. Hỏi nhanh tỉ giá, tin tức, đổi đơn vị... qua Google Gemini, không cần tài khoản; câu gợi ý đổi theo ngày. Link trong câu trả lời bấm được, có nút Chép. Gửi dồn dập thì app chặn trước (3 giây một câu, 30 câu mỗi giờ); mất mạng thì ghi "Mất mạng" |
| Hai con gặp nhau | Dừng lại quay mặt vào nhau, có lúc chào một câu hoặc đi cùng nhau một đoạn; đi ngủ thì nằm cạnh nhau |
| 3 phút không đụng tới con nào | Cả nhóm xuống taskbar (đang ở trên cửa sổ thì nhảy hoặc leo xuống) rồi lần lượt ngủ: click hay kéo con nào cũng tính chung cho cả nhóm. Click bất kỳ đâu trên màn hình hoặc kéo pet là cả nhóm dậy |
| Giữ **Ctrl** khi click | Click xuyên qua pet xuống app bên dưới |

Đổi độ phân giải, tỉ lệ DPI, chỗ đặt hay cỡ taskbar, cắm hoặc rút màn hình trong lúc app đang chạy thì
overlay tự khớp lại trong khoảng một giây: pet đứng lên mặt đất mới, pet đang đứng trên cửa sổ thì
vẫn đứng đúng mép cửa sổ đó, màn hình pet đang ở bị rút ra thì pet về màn hình chính. Taskbar để tự
ẩn thì lúc taskbar trồi lên pet đứng lên trên nó, taskbar thụt xuống thì pet rơi xuống đáy màn hình.
App chỉ hỗ trợ tỉ lệ 100%.

Click pet không làm mất focus của app đang dùng. Có app fullscreen (video, game, trình chiếu: cửa sổ
phủ kín cả màn hình, không tính cửa sổ phóng to) thì pet tự ẩn, thoát fullscreen thì hiện lại. Lúc ẩn (kể cả ẩn từ tray) app dừng hẳn vòng lặp, pet đứng nguyên
chỗ cũ chờ hiện lại.

Những thứ không cần bấm gì (tắt từng thứ trong tab **Thế giới**):

- Ban đêm (theo giờ mặt trời lặn ở thành phố đã chọn, chưa chọn thì 19:00–6:00) pet đi chậm hơn, buồn
  ngủ sớm hơn
- Thời tiết chỉ quanh pet: mưa, tuyết, sương mù, sấm (chớp sáng, pet giật mình), mùa xuân trời quang thì
  hoa rơi. Trời quang hay nhiều mây thì không có gì. Chưa chọn thành phố thì thời tiết giả lập theo mùa;
  đã chọn mà mất mạng thì một con nói "Không có mạng :))", không có hiệu ứng
- Câu nói cho vui (chào nhau, kêu trời mưa): cả nhóm 15 phút mới được một câu
- Lịch sự kiện: đúng dịp (Tết, Trung thu, Giáng sinh... hoặc dịp tự thêm, dương hoặc âm lịch) thì cả nhóm
  đội mũ, một con nói câu của dịp đó mỗi ngày một lần. Tab Thế giới ghi hôm nay có dịp nào
- 2:00–2:30 sáng, có con còn thức thì một con ma bay ngang qua, mỗi đêm một lần

Tab **Sức khoẻ** (mặc định tắt hết, số liệu chỉ ở máy này):

- Giờ ngồi máy: tính lúc có chạm chuột hay phím, vắng quá 5 phút là đang nghỉ; xem hôm nay, đang ngồi
  liền, lượt ngồi liền lâu nhất, đã đứng dậy nghỉ mấy lần, 7 ngày gần nhất. Xoá được
- Ngồi liền 50 phút (chỉnh được) thì một con nhảy lên nhắc nghỉ mắt; cứ ngồi máy 60 phút (chỉnh được) thì
  nhắc uống nước; sau giờ đi ngủ (mặc định 23:00) mà còn ngồi máy thì nhắc đi ngủ, 30 phút một lần. Bấm vào
  pet đang nhắc là tắt
- Bấm Ctrl+S 5 lần trong 10 giây thì pet kêu "Lưu rồi mà :(((("
- Đang gõ phím (có input mà chuột đứng yên) thì pet không nói câu cho vui

## Build bản phát hành

```bash
pnpm icons:desktop   # tạo lại icon nếu vừa thay assets/icon.png
pnpm build:desktop   # installer NSIS trong desktop/src-tauri/target/release/bundle/nsis
```

Logo nguồn ở [`assets/icon.png`](assets/icon.png), dùng chung cho app, system tray, setup và portable
(cách thay: [assets/README.md](assets/README.md#thay-logo)). Build và Release trên GitHub tự tạo bộ
icon từ logo này trước khi compile.

### Tự build khi push lên `main`

Mỗi lần push lên `main`, workflow [`.github/workflows/build.yml`](.github/workflows/build.yml) chạy
typecheck, test, `cargo test` rồi build file `.exe` cho Windows (mất khoảng 10–15 phút lần đầu, các
lần sau nhanh hơn nhờ cache).

Tải về: GitHub → tab **Actions** → bấm vào lần chạy mới nhất → mục **Artifacts** ở cuối trang. Mỗi
file là một artifact, tải về là file `.exe` luôn, không phải zip (giữ 30 ngày). Trước đây cả hai nằm
chung một zip, mà Explorer hiện file exe nằm trong zip bằng icon exe chung, trông như app mất icon.

| File | Dùng khi |
|---|---|
| `TinyWorld_<version>-<số lần chạy>-<commit>_x64-setup.exe` | Cài vào máy (có shortcut, gỡ được trong Settings) |
| `TinyWorld_<version>-<số lần chạy>-<commit>_x64-portable.exe` | Chạy thẳng không cần cài (cần WebView2, Windows 10/11 bản mới có sẵn) |

`<version>` là version sắp phát hành: commit đã được phát hành thì đúng version đó, chưa thì tăng số cuối
của release mới nhất (đã có `v1.3.2` thì là `1.3.3`). Version này cũng được ghi vào app; cửa sổ Cài đặt
hiện kèm số lần chạy và commit (ví dụ `Phiên bản 1.3.3 · build 57 · 14e831c`) để phân biệt với bản Release
cùng version. Chạy dev thì hiện `Bản dev`.

Test hỏng thì không build ra file. Push liên tục thì lần build cũ bị huỷ, chỉ giữ lần mới nhất.
Push chỉ sửa file `.md` hoặc `release.yml` thì không build (sửa sprite, icon trong `assets/` vẫn
build vì chúng nằm trong app). Muốn build lại mà không push: tab **Actions → Build → Run workflow**.

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

- **Cửa sổ** (`src-tauri/src/window_list.rs`): Windows báo mỗi khi cửa sổ mở, đóng, di chuyển, đổi
  thứ tự chồng (`SetWinEventHook`); luồng nền gom lại, đọc danh sách tối đa 30 lần/giây và chỉ gửi
  event `windows-changed` khi có gì khác, không có gì đổi thì ngủ (2 giây đọc lại một lần phòng sót).
  Chỉ lấy khung nhìn thấy (DWM, không tính viền trong suốt), thứ tự chồng và tên lớp để bỏ desktop,
  taskbar; không đọc tiêu đề hay nội dung. Bỏ cửa sổ thu nhỏ, ẩn, ở desktop ảo khác, cửa sổ công cụ và
  cửa sổ để chuột đi xuyên. Windows báo cả lúc con trỏ di chuyển (cùng event với cửa sổ di chuyển),
  nên pet ngủ thì việc di chuyển chỉ theo dõi cửa sổ của explorer (taskbar) và 10 giây mới đọc lại;
  overlay ẩn (app fullscreen) thì gỡ hẳn hook.
- **Địa hình** (`packages/sim/src/terrain.ts`): mép trên cửa sổ là chỗ đứng, cạnh bên là tường để
  leo, trừ phần bị cửa sổ nằm trên che. Pet đứng trên taskbar hoặc đang bay thì nằm trước mọi cửa sổ;
  đứng hay leo trên cửa sổ nào thì phần bị cửa sổ nằm trên che không được vẽ (xoá trên canvas) và
  không bắt chuột.
- **Overlay** (`src-tauri/src/overlay.rs`): cửa sổ trong suốt, không viền, luôn trên cùng, không có
  nút taskbar, phủ vùng làm việc (trừ taskbar) của một màn hình, lúc mở là màn hình chính. Không phủ
  kín cả màn hình vì Windows sẽ coi đó là app fullscreen: tắt thông báo, và pet tự ẩn rồi hiện liên
  tục. Toạ độ gửi cho frontend đều là CSS pixel của overlay; Rust đổi từ pixel vật lý theo DPI.
- **Nhiều màn hình**: `screen_info` kèm vùng làm việc của các màn hình khác (`neighbors`), để pet biết
  mép nào giáp màn hình khác. Pet bị kéo, bị ném hoặc tự đi ra khỏi mép thì frontend gọi
  `move_overlay` với điểm nằm bên kia mép; điểm đó thuộc màn hình khác thì Rust chuyển overlay sang,
  rồi gửi event `screen-changed` kèm cách đổi toạ độ cũ sang mới (`remap`). Cứ mỗi giây Rust đo lại
  màn hình; độ phân giải, DPI, taskbar đổi hay màn hình bị rút thì đặt lại overlay và gửi cùng event đó.
- **Con trỏ** (`src-tauri/src/cursor.rs`): Rust đọc con trỏ khoảng 60 lần/giây nhưng chỉ gửi
  `cursor-moved` khi con trỏ ở trong vùng quanh một con mà frontend báo (`set_cursor_interest`, mỗi con
  một vùng: con thức thì trong tầm nhìn theo 300 px, con ngủ thì chỉ quanh thân), lúc vừa ra khỏi vùng, lúc bấm/nhả chuột (click ở
  đâu pet đang ngủ cũng dậy) và lúc vùng vừa đổi. Nhờ vậy chuột di chuyển ở xa pet không đánh thức
  WebView. Pet chỉ dùng vị trí con trỏ để quay đầu nhìn theo và để bật/tắt click-through.
- **Taskbar tự ẩn**: lúc trồi lên, taskbar che mép dưới overlay. Luồng theo dõi cửa sổ đọc khung
  taskbar cùng lúc với danh sách cửa sổ và gửi mép trên của nó (`taskbarTop` trong `windows-changed`);
  mép đó cao hơn mép dưới vùng làm việc thì frontend lấy nó làm mặt đất.
- **Pet**: mỗi pet là một `<canvas>` nhỏ đúng bằng một frame, di chuyển bằng CSS transform, chỉ vẽ
  lại khi đổi frame. Cả nhóm (tối đa 3 con) dùng chung một overlay, một vòng lặp; chỉ nạp sprite của
  các nhân vật đang hiện, bớt nhân vật thì bỏ. Vòng lặp tối đa 30 fps, hẹn giờ rồi mới xin `requestAnimationFrame` (xin rAF liên
  tục thì WebView thức dậy theo tần số màn hình, 60–144 lần/giây). Cả nhóm đứng yên hay ngồi mép thì chỉ
  thức dậy lúc có con đổi frame (khoảng 7 lần/giây), cả nhóm ngủ thì dừng hẳn và Rust bảo WebView2 dùng ít RAM
  (`MemoryUsageTargetLevel`), overlay ẩn cũng vậy. Mặt nạ alpha để bắt chuột lưu 1 bit mỗi pixel.
- **Logic** (`packages/sim`): không phụ thuộc DOM hay Tauri, test bằng vitest. Bước thời gian cố
  định nên hành vi không phụ thuộc fps; RNG có seed để test được hành vi ngẫu nhiên.
- **Thời tiết** (`src-tauri/src/weather.rs`): chỉ Rust gọi mạng, webview không gọi. Đã chọn thành phố
  thì hỏi Open-Meteo lúc mở app, lúc đổi thành phố, rồi 30 phút một lần, gửi event `weather-changed`;
  lỗi thì chờ 1 phút, gấp đôi mỗi lần tới tối đa 30 phút, và chỉ gửi `weather-failed` ở lần lỗi đầu.
  Tìm thành phố (`search_city`) cũng qua Rust. Overlay (`src/overlay/ambience.ts`) đổi thời tiết thành
  hiệu ứng, ban đêm, mũ của dịp lễ, con ma, theo giờ ở thành phố đã chọn (`wallClock`, múi giờ IANA);
  âm lịch tính trên máy (`packages/sim/src/lunar.ts`). Mọi câu pet nói nằm ở `packages/sim/src/lines.ts`.
- **Sức khoẻ** (`src-tauri/src/activity.rs`): luồng nền đọc mỗi giây thời điểm input cuối
  (`GetLastInputInfo`) và vị trí con trỏ, không hook bàn phím. `Tracker` (thuần logic, có test) cộng giờ
  ngồi máy theo khoảng giữa hai lần input, quyết định nhắc nghỉ, uống nước, nhắc khuya, gửi event `reminder`; có input
  mà chuột đứng yên thì gửi `activity-changed` (đang gõ). Spam Ctrl+S đọc trong vòng con trỏ của
  `cursor.rs`. Overlay chỉ cho pet nói (`Ambience.remind`).
- **Chat** (`src-tauri/src/chat.rs`, `src-tauri/src/gemini.rs`, `src/chat/`): cửa sổ React riêng
  (`chat.html`), chỉ tạo khi click chuột phải vào pet và huỷ khi đóng (Rust gửi `chat-closed` cho overlay
  để con đang chat đi lại). Khung chat ghép câu gửi đi (tính cách nhân vật, giờ máy, 6 lượt gần nhất,
  `prompt.ts`), Rust kiểm tra giới hạn gửi (`Limiter`) rồi gọi Gemini như trang gemini.google.com lúc chưa
  đăng nhập (`gemini.rs`: chọn Flash bằng header như trang, build label và mã model đọc từ trang `/app`,
  chat tạm). Không chạy server, không mở cổng. Câu trả lời markdown được đọc thành khối (`markdown.ts`)
  rồi vẽ bằng React, không dùng HTML thô.
- **Cửa sổ cài đặt** (`src/settings/`, `src-tauri/src/settings.rs`): trang React riêng
  (`settings.html`), chỉ tạo khi bấm **Cài đặt…** và huỷ khi đóng để đỡ tốn RAM. Overlay không kéo
  React theo.
- **Dữ liệu** trong `%APPDATA%\com.tinyworld.app\`, ghi ra file tạm rồi đổi tên:
  - `world.json`: vị trí, hướng, đang ngủ hay không của từng con (id là tên thư mục pack). Lưu 30 giây một lần (chỉ khi có thay
    đổi) và khi bấm **Thoát**; tắt máy ngang thì mất tối đa 30 giây.
  - `settings.json`: các nhân vật đang hiện (`pets`, tên thư mục pack theo thứ tự chọn), cỡ nhân vật,
    tốc độ, thành phố (tên, toạ độ, múi giờ), các công tắc của tab Thế giới, lịch sự kiện (`occasions`,
    tối đa 30 dịp). Sửa tay sai thì app kẹp về khoảng cho phép, bỏ tên trùng, quá 3 con thì bỏ bớt, bỏ dịp
    có ngày không có thật; pack không còn thì bỏ qua, không còn pack nào thì dùng pack đầu tiên. File của
    bản cũ (`pet`, một nhân vật; chưa có các mục của tab Thế giới) vẫn đọc được.
  - `weather.json`: thời tiết lấy được gần nhất, mở app lúc mất mạng vẫn dùng được nếu chưa quá 3 giờ.
  - `stats.json`: theo ngày, 30 ngày gần nhất: giờ ngồi máy, lượt ngồi liền lâu nhất (ms), số lần nghỉ;
    chỉ có khi bật đếm giờ. Ghi mỗi phút (khi có thay đổi) và lúc thoát; nút xoá trong tab Sức khoẻ ghi đè thành rỗng.
  - **Chạy cùng Windows** không lưu ở đây mà là giá trị `TinyWorld` trong
    `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (trỏ tới exe đang chạy).

## Kiểm thử

```bash
pnpm typecheck
pnpm test                               # sprite pack, kéo thả/bấm đúp overlay, pet.json, FSM, World/Pet, nhóm pet, địa hình cửa sổ, con trỏ, đổi màn hình, world.json, thời tiết, âm lịch, lịch sự kiện
cd desktop/src-tauri && cargo test      # toạ độ theo DPI, màn hình bên cạnh, đổi toạ độ giữa hai màn hình, danh sách cửa sổ, lưu trạng thái, cài đặt, lịch sự kiện, đọc kết quả Open-Meteo, giờ ngồi máy và lời nhắc, spam Ctrl+S, giới hạn gửi chat, vị trí cửa sổ chat, đọc trang và câu trả lời Gemini, câu gợi ý, registry
```

`pnpm test` gồm `pnpm test:sprites` (script chuẩn hoá sprite, `prepare-sprites.mjs --check` trên
cả 21 pack và `prepare-items.mjs --check` cho ảnh mũ), `pnpm test:overlay` (kéo thả, bấm đúp trên overlay, đo đỉnh đầu để đội mũ, markdown và câu gửi của khung chat) và vitest của `packages/*`.
