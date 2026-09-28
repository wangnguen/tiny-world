# Tạo icon desktop từ logo gốc; các icon Android/iOS chỉ nằm trong target/.
$ErrorActionPreference = 'Stop'
$workspaceRoot = Split-Path -Parent $PSScriptRoot
$generatedIcons = Join-Path $workspaceRoot 'desktop/src-tauri/target/generated-icons'
$desktopIcons = Join-Path $workspaceRoot 'desktop/src-tauri/icons'

Push-Location $workspaceRoot
try {
    & pnpm.cmd --filter desktop tauri icon ../assets/icon.png --output src-tauri/target/generated-icons
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri không tạo được icon (exit code $LASTEXITCODE)."
    }

    New-Item -ItemType Directory -Path $desktopIcons -Force | Out-Null
    # App chỉ chạy Windows: bỏ icon macOS (.icns) và Microsoft Store (Square*Logo, StoreLogo).
    # Giữ icon.ico, favicon 32x32 và các PNG mà small-icons.mjs ghép lại vào icon.ico.
    $keep = '32x32.png', '64x64.png', '128x128.png', '128x128@2x.png', 'icon.ico', 'icon.png'
    Get-ChildItem -LiteralPath $generatedIcons -File |
        Where-Object { $keep -contains $_.Name } |
        Copy-Item -Destination $desktopIcons -Force
    Write-Output 'Đã cập nhật desktop/src-tauri/icons từ assets/icon.png.'

    # Cỡ 16/24/32 px thu nhỏ từ logo lớn bị nhoè: vẽ lại theo lưới pixel và dựng lại icon.ico.
    & node scripts/small-icons.mjs
    if ($LASTEXITCODE -ne 0) {
        throw "Không vẽ được icon cỡ nhỏ (exit code $LASTEXITCODE)."
    }
}
finally {
    Pop-Location
}
