param(
    [string]$Tag = "run",
    [int]$Games = 500,
    [int[]]$Seeds = @(11, 12, 13),
    [string]$Deck = "5CP3Z idou",
    [string]$Deck2 = "",
    [switch]$NoVs
)

# Paired A/B across seeds and both seats, with a fixed --games so the runs pair.
#
# The deal for game N is a pure function of (--seed, N) and the seed space is
# mixed, so the same seed on two policies faces identical shuffles and the
# comparison is paired. The baseline is the SAME binary with an environment
# switch set, so a change is measured against its own ablation rather than
# against a run produced by a different build - which is what silently broke
# the first attempt here, when the seed mixer changed underneath a saved
# baseline and every join missed.
$ErrorActionPreference = "Continue"
$arena = "C:\rust_targets\release\bot_arena.exe"
$root = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $root "test_output\$Tag"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# `Start-Process -ArgumentList` joins the array with spaces and does NOT quote
# anything, so a deck name with a space in it ("5CP3Z idou") arrives as two
# arguments and the arena dies with an empty report. Quote anything with a
# space, once, here.
function Quote-Arg {
    param([string[]]$ArenaArgs)
    $ArenaArgs | ForEach-Object {
        if ($_ -match '\s') { '"' + $_ + '"' } else { $_ }
    }
}

# Run one arena and keep stdout (the report) and stderr (the deck banner and any
# error) apart.
#
# NOT `2>&1 | Out-File`: PowerShell turns native stderr into error records and
# re-emits them at the console width, which hard-truncates long lines. The
# report is read as a file, so it has to be written by the process itself.
function Invoke-Arena {
    param([string[]]$ArenaArgs, [string]$EnvName, [string]$EnvValue)
    if ($EnvName) { Set-Item -Path "Env:\$EnvName" -Value $EnvValue }
    $stdout = [System.IO.Path]::GetTempFileName()
    $stderr = [System.IO.Path]::GetTempFileName()
    $p = Start-Process $arena -ArgumentList $ArenaArgs -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    if ($EnvName) { Remove-Item "Env:\$EnvName" -ErrorAction SilentlyContinue }
    if ($p.ExitCode -ne 0) {
        "ARENA FAILED (exit $($p.ExitCode)): $($ArenaArgs -join ' ')"
        Get-Content $stderr | ForEach-Object { "  $_" }
    }
    Remove-Item $stdout, $stderr -ErrorAction SilentlyContinue
}

function Save-Arena {
    param([string]$Name, [string[]]$ArenaArgs, [string]$Switch, [string]$Value)
    $f = Join-Path $outDir "$Name.txt"
    $tmp = [System.IO.Path]::GetTempFileName()
    $err = [System.IO.Path]::GetTempFileName()
    if ($Switch) { Set-Item -Path "Env:\$Switch" -Value $Value }
    $p = Start-Process $arena -ArgumentList (Quote-Arg $ArenaArgs) -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $tmp -RedirectStandardError $err
    if ($Switch) { Remove-Item "Env:\$Switch" -ErrorAction SilentlyContinue }
    Copy-Item $tmp $f -Force
    if ($p.ExitCode -ne 0) {
        "ARENA FAILED (exit $($p.ExitCode)): $($ArenaArgs -join ' ')"
        Get-Content $err | ForEach-Object { "  $_" }
    }
    Remove-Item $tmp, $err -ErrorAction SilentlyContinue
    return $f
}

$deckArgs = @()
if ($Deck2) { $deckArgs = @("--deck2", $Deck2) }

$baseFiles = @()
$fixFiles = @()
foreach ($s in $Seeds) {
    foreach ($seat in 1, 2) {
        if ($seat -eq 1) {
            $common = @("v8", "v7", "10", $Deck) + $deckArgs
            $a = "s$s-a"
        } else {
            $common = @("v7", "v8", "10", $Deck) + $deckArgs
            $a = "s$s-b"
        }
        $baseCsv = Join-Path $outDir "base-$a.csv"
        $baseFiles += Save-Arena -Name "base-$a" -ArenaArgs ($common + @("--games", "$Games", "--seed", "$s", "--outcomes", $baseCsv)) -Switch "V8_NO_ASSUME_COMMIT" -Value "1"
        $fixCsv = Join-Path $outDir "fix-$a.csv"
        $vsArgs = @("--games", "$Games", "--seed", "$s", "--outcomes", $fixCsv)
        if (-not $NoVs) { $vsArgs += @("--vs", $baseCsv) }
        $fixFiles += Save-Arena -Name "fix-$a" -ArenaArgs ($common + $vsArgs)
    }
}

$w = 0; $l = 0; $d = 0
foreach ($s in $Seeds) {
    foreach ($seat in 1, 2) {
        $a = if ($seat -eq 1) { "s$s-a" } else { "s$s-b" }
        $rows = Import-Csv (Join-Path $outDir "fix-$a.csv")
        $mine = ($rows | Where-Object { [int]$_.result -eq 1 }).Count
        $theirs = ($rows | Where-Object { [int]$_.result -eq -1 }).Count
        $dr = ($rows | Where-Object { [int]$_.result -eq 0 }).Count
        if ($seat -eq 2) { $t = $mine; $mine = $theirs; $theirs = $t }
        $w += $mine; $l += $theirs; $d += $dr
    }
}
"==== $Tag ===="
"v8 wins $w  losses $l  draws $d  over $($w + $l + $d) games"
if (($w + $l) -gt 0) { "v8 decisive win rate: {0:N2}%" -f (100 * $w / ($w + $l)) }
"--- paired verdicts ---"
foreach ($f in $fixFiles) { Get-Content $f | Select-String -Pattern "VERDICT|McNemar|swing|pace \(P1\)|discordant|DISCORDANT" | ForEach-Object { "  $(Split-Path -Leaf $f): $($_.Line.Trim())" } }
