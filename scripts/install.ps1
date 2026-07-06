#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$Repo = if ($env:DOKTUI_REPO) { $env:DOKTUI_REPO } else { "doklabs/doktui" }
$InstallDir = if ($env:DOKTUI_INSTALL_DIR) { $env:DOKTUI_INSTALL_DIR } else { "$env:LOCALAPPDATA\Programs\doktui" }
$Version = if ($env:DOKTUI_VERSION) { $env:DOKTUI_VERSION } else { "latest" }

$arch = switch ((Get-CimInstance Win32_Processor).Architecture) {
    9 { "x86_64" }   # x64
    12 { "aarch64" } # ARM64
    default { throw "unsupported architecture" }
}

$target = "${arch}-pc-windows-msvc"
$assetName = "doktui-${target}.exe"
$api = "https://api.github.com/repos/$Repo/releases"
$release = if ($Version -eq "latest") {
    Invoke-RestMethod "$api/latest"
} else {
    Invoke-RestMethod "$api/tags/v$Version"
}

$asset = $release.assets | Where-Object { $_.name -eq $assetName } | Select-Object -First 1
if (-not $asset) { throw "no release asset $assetName" }

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$dest = Join-Path $InstallDir "doktui.exe"
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $dest

$dataDir = Join-Path $env:APPDATA "doktui"
New-Item -ItemType Directory -Force -Path $dataDir | Out-Null
'"script"' | Set-Content (Join-Path $dataDir "install_method")

Write-Host "DokTUI installed to $dest"
Write-Host "Add $InstallDir to PATH, then run: doktui"
