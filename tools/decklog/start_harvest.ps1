# One command, start to finish.
#
#   .\tools\decklog\start_harvest.ps1
#
# A Chrome window opens on the decklog search page. Log in there - that is the only
# step that needs a human, and it is needed once. This script notices the login, then
# runs the entire harvest unattended: all 35,536 deck codes, then every deck's card list
# and HTML, then a verification pass.

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$py = Join-Path $PSScriptRoot ".venv\Scripts\python.exe"

if (-not (Test-Path -LiteralPath $py)) {
    Write-Host "creating the python environment (first run only)..."
    uv venv (Join-Path $PSScriptRoot ".venv") --python 3.12 | Out-Null
    uv pip install --python $py playwright | Out-Null
}

Push-Location $root
try {
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $log = Join-Path $PSScriptRoot "harvest-$stamp.log"

    Write-Host ""
    Write-Host "A Chrome window will open on the decklog search page." -ForegroundColor Cyan
    Write-Host "Log in there. Everything else is automatic." -ForegroundColor Cyan
    Write-Host "Log: $log"
    Write-Host ""

    & $py (Join-Path $PSScriptRoot "login.py") 2>&1 | Tee-Object -FilePath $log
    if ($LASTEXITCODE -ne 0) {
        Write-Host "login step did not complete - see $log" -ForegroundColor Yellow
        exit 1
    }

    Write-Host ""
    Write-Host "Logged in. Starting the full harvest; this takes hours." -ForegroundColor Green
    & $py (Join-Path $PSScriptRoot "harvest_all.py") 2>&1 | Tee-Object -FilePath $log -Append
    Write-Host ""
    Write-Host "Done. Full log: $log" -ForegroundColor Green
}
finally {
    Pop-Location
}