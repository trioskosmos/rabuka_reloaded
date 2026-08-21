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
    [string]$Message = "the aim is to find issues where the engine doesn't makes the card game works that does not match what is written in the rules.txt qa_data.json and ability texts and so on. not to just get tests passing for the sake of it. continue, look for bad tests to improve, new tests to make whether for untested abilities or combining multiple tests of abilities that interact with each other and so on, if you are struggling with tests failing go read yourself the ability and the tests and work out what is going wrong, also heavy refactors of bullshit you find as you work. again ENSURE it works as written, read the rules and so on of the game it should all be in rules.txt and qa_data.json. also any underscores replace them with what should actually be there don't be lazy"
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
