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
    Get-ChildItem -LiteralPath $generatedIcons -File | Copy-Item -Destination $desktopIcons -Force
    Write-Output 'Đã cập nhật desktop/src-tauri/icons từ assets/icon.png.'
}
finally {
    Pop-Location
}
