$ErrorActionPreference = "Stop"

$mingw = "$env:USERPROFILE\mingw64"
if (-not (Test-Path "$mingw\bin\x86_64-w64-mingw32-gcc.exe")) {
    throw "MinGW-w64 not found at $mingw."
}

$env:PATH = "$mingw\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:CARGO_HTTP_PROXY = ""
$env:HTTPS_PROXY = ""
$env:HTTP_PROXY = ""
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "$mingw\bin\x86_64-w64-mingw32-gcc.exe"

$root = Resolve-Path "$PSScriptRoot\.."

Write-Host "Building frontend (vite)..." -ForegroundColor Cyan
Push-Location $root
npm run web:build
Pop-Location

Write-Host "Building GameFloat (release)..." -ForegroundColor Cyan
cargo build --release --manifest-path "$root\src-tauri\Cargo.toml"

$exe = Join-Path $root "src-tauri\target\release\gamefloat.exe"
$size = [math]::Round((Get-Item $exe).Length / 1MB, 2)
Write-Host "Done: $exe ($size MB)" -ForegroundColor Green
