# Overlay DirectComposition (PoC)

Overlay của TinyWorld vẽ bằng Direct3D 11, Direct2D và DirectComposition, không có WebView2. Mục đích là đo RAM,
CPU, GPU so với overlay Tauri trước khi quyết định chuyển hẳn. Bản phát hành vẫn là `desktop/`.

Cách vẽ: một cửa sổ trong suốt phủ vùng làm việc của màn hình chính (`WS_EX_NOREDIRECTIONBITMAP`, DWM ghép thẳng
cây visual). Mỗi pet một visual có surface đúng cỡ pet. Đổi frame thì vẽ lại surface; đi lại chỉ đổi offset của
visual rồi `Commit`, không vẽ gì. Đi, đứng thì bước 30 lần mỗi giây; chạy, bị kéo, rơi thì theo nhịp màn hình.

Chưa có: sim thật (`pet.rs` chỉ có đứng, đi, chạy, kéo, rơi, đáp), leo cửa sổ, bóng thoại, thời tiết, nhiều màn
hình, khay, Cài đặt.

## Chạy

Cần Rust MSVC như `desktop/`. Nhân vật, cỡ và tốc độ lấy từ Cài đặt của TinyWorld nếu có (chỉ đọc).

```powershell
cd native
cargo run --release -- --trace
cargo run --release -- --pets c-boggo,c-byte --mode run --exclude-capture
```

| Tham số | Ý nghĩa |
|---|---|
| `--pets a,b` | Nhân vật (tên thư mục trong `assets/sprites/`) |
| `--size`, `--speed` | Cỡ và tốc độ như trong Cài đặt |
| `--mode auto\|idle\|walk\|run` | `auto` như pet thật; còn lại giữ mọi pet ở một trạng thái để đo từng nhịp |
| `--trace` | Mỗi 5 giây in số lần thức, bước, vẽ, commit mỗi giây |
| `--exclude-capture` | Gọi `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` (ẩn pet khi chia sẻ màn hình) |
| `--no-layered` | Bỏ `WS_EX_LAYERED`: chuột không đi xuyên được, chỉ để thử vẽ |

## Cổng điều khiển

`127.0.0.1:47832`, mỗi kết nối gửi một dòng: `quit`, `mode <auto|idle|walk|run>`, `drop <pet> <x>`, `stats`
(trả JSON số lần thức/bước/vẽ/commit mỗi giây và trạng thái pet). Chạy thêm một bản khi đã có bản chạy thì báo lỗi
rồi thoát.

```powershell
$c = [Net.Sockets.TcpClient]::new("127.0.0.1", 47832); $s = $c.GetStream()
$w = [IO.StreamWriter]::new($s); $w.WriteLine("stats"); $w.Flush(); [IO.StreamReader]::new($s).ReadToEnd()
```

## Đo

`scripts/measure.ps1` đo `tinyworld.exe`; với PoC thì đo tiến trình `tinyworld-native.exe` theo cùng cách (CPU,
working set, private, GPU của cả cây tiến trình).
