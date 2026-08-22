<# :
@echo off
title Auto-Continue
powershell -NoProfile -ExecutionPolicy Bypass -Command "$sc = $null; iex ((Get-Content -LiteralPath '%~f0' -Raw))"
pause
exit /b
#>

param(
    [int]$IntervalMinutes = 21,
    [string]$WindowTitle = "",
    [string]$Message = "continue if the md is done find new things if it's becoming kind of pointless get to test writing find tests and make them harder and more specific to the ability"
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
