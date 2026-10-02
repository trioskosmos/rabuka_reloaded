# tools/round_robin.ps1 - v9 against every bot in the registry, both seats.
#
# `bot_arena v9 v7` is a near-mirror: both seats run v7's Main, so a 55.5% there
# is a statement about two copies of the same policy, not about the bot. The
# question this answers is whether v9's edge is real or a property of that
# pairing, by playing it against the other five registered bots.
#
# Both seats are run, because a first/second asymmetry in ANY of these pairings
# would otherwise be read as skill.

param([int]$Games = 1200, [int]$Seed = 11)

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$arena = "C:\rust_targets\release\bot_arena.exe"
$outDir = Join-Path $root "test_output\ladder"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$results = Join-Path $outDir "round_robin.txt"
Set-Location (Join-Path $root "engine")

$opponents = @("v6", "v7", "v7rollout", "v8", "v10")

function Run {
    param([string]$P1, [string]$P2)
    $a = & $arena $P1 $P2 $Games "5CP3Z idou" --games $Games --seed $Seed 2>&1 | Out-String
    $rec = [regex]::Match($a, "P1\((\w+)\)\s+(\d+)\s+-\s+P2\((\w+)\)\s+(\d+)")
    if (-not $rec.Success) { return "FAILED" }
    $name = $rec.Groups[1].Value
    $w = [int]$rec.Groups[2].Value
    $l = [int]$rec.Groups[3].Value
    $n = $w + $l
    "{0,-10} {1,5}-{2,-5} {3,6}%  (n={4})" -f "$name vs $($rec.Groups[3].Value)", $w, $l,
        [math]::Round(100.0 * $w / $n, 2), $n
}

"$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  round robin: games=$Games seed=$Seed" | Add-Content $results
foreach ($opp in $opponents) {
    $line = Run "v9" $opp
    "$line" | Add-Content $results
    "$line"
    $rev = Run $opp "v9"
    "$rev  (reversed)" | Add-Content $results
    "$rev  (reversed)"
}
"$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  round robin done" | Add-Content $results
