# Đo CPU và RAM của TinyWorld (tinyworld.exe và các tiến trình WebView2 con của nó) theo thời gian, kèm đang cắm
# sạc hay chạy pin. Dùng cho bài chạy thử 8–24 tiếng (rò RAM) và đo lúc dùng pin (PLAN.md, Phase 7).
#
#   powershell -ExecutionPolicy Bypass -File scripts/measure.ps1 -Minutes 480 -Every 60
#
# Mỗi lần đo ghi một dòng vào file CSV ngay (dừng giữa chừng bằng Ctrl+C vẫn còn số liệu). Hết giờ thì in tóm
# tắt: CPU trung bình, RAM lúc đầu, lúc cuối và cao nhất. Chỉ đọc số liệu của Windows, không gửi đi đâu.
param(
  [int]$Minutes = 480,
  [int]$Every = 60,
  [string]$Out = "tinyworld-measure-$(Get-Date -Format 'yyyyMMdd-HHmm').csv"
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Windows.Forms
$cores = [Environment]::ProcessorCount

# PID của tinyworld.exe và mọi tiến trình con cháu (WebView2: browser, renderer, GPU...).
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
  foreach ($p in $all | Where-Object { $_.Name -eq "tinyworld.exe" }) { $queue.Enqueue([int]$p.ProcessId) }
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

if (-not (Get-Process tinyworld -ErrorAction SilentlyContinue)) {
  Write-Error "TinyWorld chưa chạy. Mở app rồi chạy lại script."
}
"time,cpu_percent,working_set_mb,private_mb,processes,power,battery_percent" | Set-Content -Path $Out -Encoding utf8
Write-Host "Đo $Minutes phút, $Every giây một lần, ghi vào $Out"

$previous = Get-Usage (Get-TinyWorldIds)
$previousAt = Get-Date
$end = $previousAt.AddMinutes($Minutes)
$rows = @()
while ((Get-Date) -lt $end) {
  Start-Sleep -Seconds $Every
  $now = Get-Date
  $usage = Get-Usage (Get-TinyWorldIds)
  if ($usage.Count -eq 0) {
    Write-Warning "$($now.ToString('HH:mm:ss')) TinyWorld đã thoát, dừng đo."
    break
  }
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
    Working = [Math]::Round($usage.Working / 1MB, 1)
    Private = [Math]::Round($usage.Private / 1MB, 1)
  }
  $rows += $row
  "$($now.ToString('s')),$cpuPercent,$($row.Working),$($row.Private),$($usage.Count),$line,$battery" | Add-Content -Path $Out -Encoding utf8
  Write-Host ("{0}  CPU {1,6}%  RAM {2,7} MB (private {3} MB)  {4} tiến trình  {5}" -f $now.ToString("HH:mm:ss"), $cpuPercent, $row.Working, $row.Private, $usage.Count, $line)
  $previous = $usage
  $previousAt = $now
}

if ($rows.Count -gt 0) {
  $avg = [Math]::Round(($rows | Measure-Object Cpu -Average).Average, 2)
  $max = ($rows | Measure-Object Private -Maximum).Maximum
  Write-Host ""
  Write-Host "Tóm tắt $($rows.Count) lần đo: CPU trung bình $avg% (tổng các nhân)"
  Write-Host "RAM private: lúc đầu $($rows[0].Private) MB, lúc cuối $($rows[-1].Private) MB, cao nhất $max MB"
  Write-Host "Cuối tăng đều so với đầu (không chỉ nhảy lúc pet thức) thì nghi rò RAM: gửi kèm $Out khi báo lỗi."
}
