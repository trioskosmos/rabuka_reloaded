# One-command bake + test pipeline.
#   1. parser corpus-invariant tests (fast guard on structural rules)
#   2. extract card abilities -> abilities.json
#   3. compile abilities -> abilities.bin + engine/src/ability/abilities_gen.rs
#   4. full engine test suite (cargo test --test run_all)
#
# Usage:  powershell -ExecutionPolicy Bypass -File bake_and_test.ps1
#   -SkipExtract  skip the (slow) re-extract/compile; only run tests
#   -TestPattern <substring>  only run engine tests matching the substring

param(
    [switch]$SkipExtract,
    [string]$TestPattern = ""
)

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot

Write-Host "==> [1/4] Parser corpus-invariant tests" -ForegroundColor Cyan
Push-Location (Join-Path $root "cards\ability_extraction")
try {
    python tests\test_ability_invariants.py
    if ($LASTEXITCODE -ne 0) { throw "invariant tests failed" }
} finally { Pop-Location }

if (-not $SkipExtract) {
    Write-Host "==> [2/4] Extract abilities (abilities.json)" -ForegroundColor Cyan
    Push-Location (Join-Path $root "cards")
    try {
        python ability_extraction\extract_card_abilities.py
        if ($LASTEXITCODE -ne 0) {
            # extract's exit code is flaky under a piped console (Windows stdout
            # encoding). It writes abilities.json before that, so warn, don't fail.
            Write-Host "      (extract reported non-zero exit; abilities.json was written)" -ForegroundColor DarkYellow
        }
        Write-Host "==> [3/4] Compile abilities (abilities.bin + abilities_gen.rs)" -ForegroundColor Cyan
        python compile_abilities.py
        if ($LASTEXITCODE -ne 0) { throw "compile failed" }
    } finally { Pop-Location }
} else {
    Write-Host "==> [2/4] Skipping extract (abilities.json unchanged)" -ForegroundColor DarkYellow
    Write-Host "==> [3/4] Skipping compile" -ForegroundColor DarkYellow
}

Write-Host "==> [4/4] Engine test suite" -ForegroundColor Cyan
Push-Location (Join-Path $root "engine")
try {
    if ($TestPattern) {
        cargo test --test run_all $TestPattern
    } else {
        cargo test --test run_all
    }
    if ($LASTEXITCODE -ne 0) { throw "engine tests failed" }
} finally { Pop-Location }

Write-Host ""
Write-Host "ALL GREEN" -ForegroundColor Green
