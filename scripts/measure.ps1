# Đo CPU, GPU và RAM của TinyWorld (tinyworld.exe và mọi tiến trình con: cửa sổ Cài đặt/Chat và WebView2 của nó lúc đang
# mở) theo thời gian, kèm đang cắm sạc hay chạy pin. Dùng cho bài chạy thử 8–24 tiếng (rò RAM) và đo lúc dùng pin
# (PLAN.md, Phase 7).
#
#   powershell -ExecutionPolicy Bypass -File scripts/measure.ps1 -Minutes 480 -Every 60
#   powershell -ExecutionPolicy Bypass -File scripts/measure.ps1 -Name tinyworld-native -Minutes 2 -Every 10
#
# Mỗi lần đo ghi một dòng vào file CSV ngay (dừng giữa chừng bằng Ctrl+C vẫn còn số liệu). Hết giờ thì in tóm
# tắt: CPU, GPU trung bình, RAM lúc đầu, lúc cuối và cao nhất. GPU là mức dùng trong 1 giây ngay lúc đo, cộng mọi
# engine của các tiến trình (chưa tính phần DWM ghép hình). Chỉ đọc số liệu của Windows, không gửi đi đâu.
param(
  [int]$Minutes = 480,
  [int]$Every = 60,
  # Tên tiến trình gốc, không có .exe (`tinyworld-native`: chỉ chạy overlay, `cargo run -p tinyworld-native`).
  [string]$Name = "tinyworld",
  [string]$Out = "$Name-measure-$(Get-Date -Format 'yyyyMMdd-HHmm').csv"
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms
$cores = [Environment]::ProcessorCount

# PID của tiến trình gốc và mọi tiến trình con cháu (WebView2: browser, renderer, GPU...).
function Get-TinyWorldIds {
  $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name
  $children = @{}
  foreach ($p in $all) {
    # CIM trả UInt32; khoá phải cùng kiểu Int32 với lúc tra.
    $parent = [int]$p.ParentProcessId
    if (-not $children.ContainsKey($parent)) { $children[$parent] = @() }
    $children[$parent] += [int]$p.ProcessId
  }
  $queue = [System.Collections.Generic.Queue[int]]::new()
  foreach ($p in $all | Where-Object { $_.Name -eq "$Name.exe" }) { $queue.Enqueue([int]$p.ProcessId) }
  $ids = @()
  while ($queue.Count -gt 0) {
    $id = $queue.Dequeue()
    $ids += $id
    if ($children.ContainsKey($id)) { foreach ($child in $children[$id]) { $queue.Enqueue([int]$child) } }
  }
  return $ids
}

# CPU đã dùng (giây) và RAM của các tiến trình `ids`; tiến trình vừa thoát thì bỏ qua.
function Get-Usage($ids) {
  $cpu = @{}
  $working = 0
  $private = 0
  foreach ($id in $ids) {
    $p = Get-Process -Id $id -ErrorAction SilentlyContinue
    if (-not $p) { continue }
    $cpu[$id] = $p.TotalProcessorTime.TotalSeconds
    $working += $p.WorkingSet64
    $private += $p.PrivateMemorySize64
  }
  return @{ Cpu = $cpu; Working = $working; Private = $private; Count = $cpu.Count }
}

# Phần trăm GPU các tiến trình `ids` dùng trong 1 giây (cộng mọi engine: 3D, copy, video...). Máy không có bộ đếm
# GPU Engine (Windows cũ, driver cũ) thì để trống.
function Get-Gpu($ids) {
  try {
    $samples = (Get-Counter "\GPU Engine(*)\Utilization Percentage" -ErrorAction Stop).CounterSamples
  } catch {
    return ""
  }
  $sum = 0.0
  foreach ($s in $samples) {
    if ($s.InstanceName -match "pid_(\d+)_" -and $ids -contains [int]$Matches[1]) { $sum += $s.CookedValue }
  }
  return [Math]::Round($sum, 2)
}

if (-not (Get-Process $Name -ErrorAction SilentlyContinue)) {
  Write-Error "$Name chưa chạy. Mở app rồi chạy lại script."
}
"time,cpu_percent,gpu_percent,working_set_mb,private_mb,processes,power,battery_percent" | Set-Content -Path $Out -Encoding utf8
Write-Host "Đo $Minutes phút, $Every giây một lần, ghi vào $Out"

$previous = Get-Usage (Get-TinyWorldIds)
$previousAt = Get-Date
$end = $previousAt.AddMinutes($Minutes)
$rows = @()
while ((Get-Date) -lt $end) {
  Start-Sleep -Seconds $Every
  $now = Get-Date
  $ids = Get-TinyWorldIds
  $usage = Get-Usage $ids
  if ($usage.Count -eq 0) {
    Write-Warning "$($now.ToString('HH:mm:ss')) $Name đã thoát, dừng đo."
    break
  }
  $gpu = Get-Gpu $ids
  # CPU trong khoảng vừa qua: tiến trình mới mọc ra thì tính hết thời gian CPU của nó.
  $used = 0
  foreach ($id in $usage.Cpu.Keys) {
    $before = if ($previous.Cpu.ContainsKey($id)) { $previous.Cpu[$id] } else { 0 }
    $used += [Math]::Max(0, $usage.Cpu[$id] - $before)
  }
  $seconds = ($now - $previousAt).TotalSeconds
  $cpuPercent = [Math]::Round($used / ($seconds * $cores) * 100, 2)
  $power = [System.Windows.Forms.SystemInformation]::PowerStatus
  $line = if ($power.PowerLineStatus -eq "Offline") { "battery" } elseif ($power.PowerLineStatus -eq "Online") { "ac" } else { "unknown" }
  $battery = if ($power.BatteryChargeStatus -match "NoSystemBattery") { "" } else { [Math]::Round($power.BatteryLifePercent * 100) }
  $row = [pscustomobject]@{
    Time = $now
    Cpu = $cpuPercent
    Gpu = $gpu
    Working = [Math]::Round($usage.Working / 1MB, 1)
    Private = [Math]::Round($usage.Private / 1MB, 1)
  }
  $rows += $row
  "$($now.ToString('s')),$cpuPercent,$gpu,$($row.Working),$($row.Private),$($usage.Count),$line,$battery" | Add-Content -Path $Out -Encoding utf8
  Write-Host ("{0}  CPU {1,6}%  GPU {2,6}%  RAM {3,7} MB (private {4} MB)  {5} tiến trình  {6}" -f $now.ToString("HH:mm:ss"), $cpuPercent, $gpu, $row.Working, $row.Private, $usage.Count, $line)
  $previous = $usage
  $previousAt = $now
}

if ($rows.Count -gt 0) {
  $avg = [Math]::Round(($rows | Measure-Object Cpu -Average).Average, 2)
  $gpuRows = @($rows | Where-Object { $_.Gpu -ne "" })
  $gpuAvg = if ($gpuRows.Count) { [Math]::Round(($gpuRows | Measure-Object Gpu -Average).Average, 2) } else { "?" }
  $max = ($rows | Measure-Object Private -Maximum).Maximum
  Write-Host ""
  Write-Host "Tóm tắt $($rows.Count) lần đo: CPU trung bình $avg% (tổng các nhân), GPU trung bình $gpuAvg%"
  Write-Host "RAM private: lúc đầu $($rows[0].Private) MB, lúc cuối $($rows[-1].Private) MB, cao nhất $max MB"
  Write-Host "Cuối tăng đều so với đầu (không chỉ nhảy lúc pet thức) thì nghi rò RAM: gửi kèm $Out khi báo lỗi."
}
