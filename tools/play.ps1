param([int]$Seed = 41, [string]$Script = "..\test_output\myscript.txt")
# Replay the scripted game and show only the live-set decisions with v8's
# model, so each round shows one board and one decision instead of a wall.
$env:V8_MODEL = "1"
$out = [System.IO.Path]::GetTempFileName()
$err = [System.IO.Path]::GetTempFileName()
$p = Start-Process "C:\rust_targets\release\human_vs_v7.exe" `
    -ArgumentList @("--deck", "`"5CP3Z idou`"", "--seed", "$Seed", "--script", "`"$Script`"") `
    -NoNewWindow -Wait -PassThru -RedirectStandardOutput $out -RedirectStandardError $err
Get-Content $out
Remove-Item $out, $err -ErrorAction SilentlyContinue
