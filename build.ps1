param(
    [switch]$Release,
    [string]$Deck1,
    [string]$Deck2
)

Write-Host "=== Rabuka CLI Build Script ===" -ForegroundColor Cyan
Write-Host ""

# Step 1 - Check Rust
Write-Host "[1/4] Checking Rust toolchain..."
$rust = Get-Command rustc -ErrorAction SilentlyContinue
if (-not $rust) {
    Write-Host "[FAIL] Rust not found. Install from https://rustup.rs" -ForegroundColor Red
    pause
    exit 1
}
$rustVer = rustc --version
Write-Host "  Found: $rustVer"

# Step 2 - Build
Write-Host "[2/4] Building CLI binary..."
$buildFlag = if ($Release) { "--release" } else { "" }
Set-Location (Join-Path $PSScriptRoot "engine")
$env:RUSTFLAGS = ""
$buildResult = cargo build $buildFlag --bin play_cli 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "[FAIL] Build failed:" -ForegroundColor Red
    $buildResult | ForEach-Object { Write-Host "  $_" -ForegroundColor Red }
    pause
    exit 1
}
Write-Host "[2/4] Build succeeded"

# Step 3 - Copy output
Write-Host "[3/4] Copying binary..."
$outDir = Join-Path $PSScriptRoot "output_cli"
if (-not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Path $outDir | Out-Null
}

$profileDir = if ($Release) { "release" } else { "debug" }
$srcBin = Join-Path $PSScriptRoot "engine\target\$profileDir\play_cli.exe"
$dstBin = Join-Path $outDir "rabuka_cli.exe"
Copy-Item $srcBin $dstBin -Force

Write-Host "[3/4] Binary copied to: $dstBin"

# Step 4 - Cleanup/Info
Write-Host "[4/4] Done!"
Write-Host ""
Write-Host "=== Build Complete ===" -ForegroundColor Green
Write-Host "Run: $dstBin"
if ($Deck1) { Write-Host "  P1 deck: $Deck1" }
if ($Deck2) { Write-Host "  P2 deck: $Deck2" }
Write-Host ""
Write-Host "Usage:" -ForegroundColor Yellow
Write-Host "  .\output_cli\rabuka_cli.exe"
Write-Host "  .\output_cli\rabuka_cli.exe --deck1 muse_cup --deck2 aqours_cup"
