<# :
@echo off
title Auto-Continue
powershell -NoProfile -ExecutionPolicy Bypass -Command "$sc = $null; iex ((Get-Content -LiteralPath '%~f0' -Raw))"
pause
exit /b
#>

param(
    [int]$IntervalMinutes = 5,
    [string]$WindowTitle = "",
    [string]$Message = "in the rust engine and parser and tests: I want god files broken up. I want simplification across the board. I want unification of helpers and methods that can be reused. I want less if-if-if-if-if-if- else routing. I want code legibility up. I want interpretability of the codebase and how things connect to each other up. I want elegance. I want superfluous excess bloat code cleaned up and removed. I want it all done fully. No excuses. No waiting for my decisions. Get it all done,"
)

Add-Type -AssemblyName System.Windows.Forms

function Show-Countdown([int]$TotalSeconds) {
    for ($i = $TotalSeconds; $i -gt 0; $i--) {
        $ts = '{0:mm\:ss}' -f [timespan]::FromSeconds($i)
        Write-Host ("`r  next send in {0}     " -f $ts) -NoNewline -ForegroundColor Cyan
        Start-Sleep -Seconds 1
    }
}

function Send-Message {
    if ($WindowTitle) {
        $null = (New-Object -ComObject WScript.Shell).AppActivate($WindowTitle)
        Start-Sleep -Milliseconds 500
    }
    try {
        [System.Windows.Forms.SendKeys]::SendWait("{ESC}")
        Start-Sleep -Milliseconds 200
        for ($i = 0; $i -lt 10; $i++) {
            [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
            Start-Sleep -Milliseconds 100
        }
        Start-Sleep -Milliseconds 200
        [System.Windows.Forms.SendKeys]::SendWait($Message)
        Start-Sleep -Milliseconds 200
        [System.Windows.Forms.SendKeys]::SendWait("{ENTER}")
        Write-Host ("`r  [{0}] message sent + Enter          " -f (Get-Date -Format "HH:mm:ss")) -ForegroundColor Green
    }
    catch {
        Write-Host ("`r  [{0}] FAILED: {1}" -f (Get-Date -Format "HH:mm:ss"), $_.Exception.Message) -ForegroundColor Red
    }
}

Clear-Host
Write-Host "=== Auto-Continue ===" -ForegroundColor Yellow
if ($WindowTitle) { Write-Host "  target window : $WindowTitle" } else { Write-Host "  target window : focused window (don't click away!)" }
Write-Host "  interval      : every $IntervalMinutes minute(s)"
Write-Host ""

Show-Countdown 10
Send-Message

while ($true) {
    Show-Countdown ($IntervalMinutes * 60)
    Send-Message
}
