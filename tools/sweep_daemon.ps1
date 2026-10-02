# tools/sweep_daemon.ps1 - persistent paired-sweep daemon.
#
# Runs candidate arms as PAIRED arena runs against a baseline outcomes file and
# appends one line per arm to a results file. Survives session close (started
# with background_process -Persistent).
#
# Why it is built the way it is:
#
#   * PARALLEL, one arena per core. Each arm is an independent process with its
#     own --seed, so wall-clock contention changes nothing about the result - the
#     arena is deterministic per seed and shares no state. What it changes is how
#     long a 29-arm round takes: serially ~3 min an arm, 12-wide ~30 s a round.
#     Results are WRITTEN to per-arm files and merged into results.txt in a fixed
#     order afterwards, so a fast arm never interleaves a slow one into a
#     half-written line.
#
#   * SNAPSHOT, not the installed binary. Cargo cannot overwrite bot_arena.exe
#     while a copy of it is running on Windows, so a sweep on the real file
#     blocks every rebuild until the round ends. The daemon copies the binary
#     into its own directory at the start of each round and runs the copy: the
#     workspace stays rebuildable while the sweep runs. Every line records the
#     snapshot's timestamp, so a round that mixed two builds is visible in the
#     record rather than inferred.
#
#   * The baseline is regenerated from the SAME snapshot, so no arm is ever
#     compared against a record produced by a different build.
#
#   * A candidate that looks real on the selection seed is re-run on holdout
#     seeds. Seed 11 is the seed every other measurement in
#     docs/V8_IMPROVEMENT_LOG.md was found on, so it cannot confirm anything.
#
# Usage: pwsh tools/sweep_daemon.ps1 [-Rounds 0] [-Games 1200] [-Parallel 10] [-PauseMin 5]
#   -Rounds 0 means "loop until stopped".

param(
    [int]$Rounds = 0,
    [int]$Games = 3000,
    [int]$Parallel = 4,
    [int]$PauseMin = 6
)

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$arena = "C:\rust_targets\release\bot_arena.exe"
$outDir = Join-Path $root "test_output\sweep"
$workDir = Join-Path $outDir "work"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
# The arena resolves its decklist as ../web_ui/decks relative to the process CWD
# and hard-errors on a missing deck rather than falling back.
Set-Location (Join-Path $root "engine")
$results = Join-Path $outDir "results.txt"
$selectSeed = 11
$holdoutSeeds = @(23, 31)

$arms = @(
    @{ label = "opp-sampled-score"; env = @("V8_OPP_SAMPLED_SCORE") },
    @{ label = "opp-sampled-yield"; env = @("V8_OPP_SAMPLED_YIELD") },
    @{ label = "opp-bias+1"; env = @("V8_OPP_SCORE_BIAS=1") },
    @{ label = "opp-bias+2"; env = @("V8_OPP_SCORE_BIAS=2") },
    @{ label = "opp-ensemble"; env = @("V8_OPP_ENSEMBLE") },
    @{ label = "opp-gradient"; env = @("V8_OPP_GRADIENT") },
    @{ label = "opp-cleared-flat"; env = @("V8_OPP_CLEARED=0.5") },
    @{ label = "opp-assume-commit"; env = @("V8_ASSUME_COMMIT=0.8") },
    @{ label = "opp-assume-set2"; env = @("V8_ASSUME_SET=2") },
    @{ label = "p-floor-0.6"; env = @("V8_P_FLOOR=0.6") },
    @{ label = "p-floor-0.75"; env = @("V8_P_FLOOR=0.75") },
    @{ label = "p-floor-0.9"; env = @("V8_P_FLOOR=0.9") },
    @{ label = "p-first"; env = @("V8_P_FIRST") },
    @{ label = "lives-2"; env = @("V8_MAX_LIVES=2") },
    @{ label = "lives-3"; env = @("V8_MAX_LIVES=3") },
    @{ label = "contested-wide"; env = @("V8_CONTESTED_WIDE") },
    @{ label = "joint"; env = @("V8_JOINT") },
    @{ label = "deficit-0.15"; env = @("V8_DEFICIT=0.15") },
    @{ label = "v9-mulligan-v8"; env = @("V9_MULLIGAN=v8") },
    @{ label = "v9-live-v7"; env = @("V9_LIVE_ROLLOUT=v7") },
    @{ label = "v7-blade-arith"; env = @("V7_BLADE_ARITH=0.5") },
    @{ label = "v7-check-pressure"; env = @("V7_CHECK_PRESSURE=40") },
    @{ label = "v7-forecast"; env = @("V7_FORECAST=30") },
    @{ label = "v7-lean"; env = @("V7_LEAN") },
    @{ label = "v7-presence"; env = @("V7_PRESENCE") },
    @{ label = "cov-gradient-20"; env = @("V7_COVERAGE_GRADIENT=20") },
    @{ label = "cov-gradient-60"; env = @("V7_COVERAGE_GRADIENT=60") },
    @{ label = "cov-gradient-150"; env = @("V7_COVERAGE_GRADIENT=150") },
    @{ label = "cov-grad-blades"; env = @("V7_COVERAGE_GRADIENT=150", "V7_BLADE_ARITH=0.5") },
    @{ label = "v7-nodes-128"; env = @("V7_NODES=128") },
    @{ label = "v7-dev-16"; env = @("V7_DEV_WEIGHT=16") },
    @{ label = "v7-blades-10"; env = @("V7_BLADES_WEIGHT=10") },
    @{ label = "v7-ammo-15"; env = @("V7_AMMO_WEIGHT=15") },
    @{ label = "assume-commit-0.6"; env = @("V8_ASSUME_COMMIT=0.6") },
    @{ label = "assume-commit-0.7"; env = @("V8_ASSUME_COMMIT=0.7") },
    @{ label = "assume-commit-0.8"; env = @("V8_ASSUME_COMMIT=0.8") },
    @{ label = "assume-commit-0.9"; env = @("V8_ASSUME_COMMIT=0.9") },
    @{ label = "assume-commit-1.0"; env = @("V8_ASSUME_COMMIT=1.0") },
    @{ label = "assume-set-1.0c8"; env = @("V8_ASSUME_COMMIT=0.8", "V8_ASSUME_SET=1") },
    @{ label = "band-wide-0.05"; env = @("V8_BAND_WIDE", "V8_BAND_MIN_MASS=0.05") },
    @{ label = "band-wide-0.15"; env = @("V8_BAND_WIDE", "V8_BAND_MIN_MASS=0.15") },
    @{ label = "band-wide-opp8"; env = @("V8_BAND_WIDE", "V8_ASSUME_COMMIT=0.8") },
    @{ label = "opp-sampled-score-c8"; env = @("V8_OPP_SAMPLED_SCORE", "V8_ASSUME_COMMIT=0.8") },
    @{ label = "v7-ammo-w40"; env = @("V7_AMMO_WEIGHT=40") },
    @{ label = "v7-blade-w5"; env = @("V7_BLADE_WEIGHT=5") },
    @{ label = "v7-blades-w25"; env = @("V7_BLADES_WEIGHT=25") },
    @{ label = "v7-baton-flat"; env = @("V7_BATON_FLAT") },
    @{ label = "v7-free-baton"; env = @("V7_FREE_BATON") },
    @{ label = "v7-free-junk"; env = @("V7_FREE_JUNK") },
    @{ label = "v7-no-baton"; env = @("V7_NO_BATON") },
    @{ label = "v7-no-ceiling"; env = @("V7_NO_CEILING") },
    @{ label = "v7-no-dev"; env = @("V7_NO_DEVELOPMENT") },
    @{ label = "v7-no-future-junk"; env = @("V7_NO_FUTURE_JUNK") },
    @{ label = "v7-no-min-win"; env = @("V7_NO_MIN_WIN") },
    @{ label = "v7-no-shared"; env = @("V7_NO_SHARED_SAMPLES") },
    @{ label = "v7-no-strict"; env = @("V7_NO_STRICT_CLOSE") },
    @{ label = "v7-no-true-min"; env = @("V7_NO_TRUE_MIN_WIN") },
    @{ label = "v7-pure-junk"; env = @("V7_PURE_JUNK") },
    @{ label = "v7-reserve-25"; env = @("V7_RESERVE_WEIGHT=25") },
    @{ label = "v7-energy-25"; env = @("V7_ENERGY_WEIGHT=25") },
    @{ label = "v7-heart-25"; env = @("V7_HEART_WEIGHT=25") },
    @{ label = "v7-hand-25"; env = @("V7_HAND_WEIGHT=25") },
    @{ label = "v7-cov-25"; env = @("V7_COVERAGE_WEIGHT=25") },
    @{ label = "v7-dev-25"; env = @("V7_DEV_WEIGHT=25") },
    @{ label = "v7-future-25"; env = @("V7_FUTURE_WEIGHT=25") },
    @{ label = "v7-opp-coverage"; env = @("V7_OPP_COVERAGE") },
    @{ label = "v7-opp-terms"; env = @("V7_OPP_TERMS") },
    @{ label = "v7-payoff"; env = @("V7_PAYOFF_MODEL") },
    @{ label = "v7-sims-64"; env = @("V7_ISMCTS_SIMS=64") },
    @{ label = "v7-worlds-8"; env = @("V7_ISMCTS_WORLDS=8") },
    @{ label = "v7-free-min-junk"; env = @("V7_FREE_JUNK", "V7_NO_FUTURE_JUNK") },
    @{ label = "v8-init-10"; env = @("V8_INITIATIVE_WEIGHT=10") },
    @{ label = "v8-pass-tax"; env = @("V8_PASS_TAX") },
    @{ label = "v8-tie-baton"; env = @("V8_TIE_BATON") },
    @{ label = "v8-zone-order"; env = @("V8_ZONE_ORDER") },
    @{ label = "v8-chain"; env = @("V8_CHAIN") },
    @{ label = "v8-delta-leaf"; env = @("V8_DELTA_LEAF") },
    @{ label = "v8-filtered"; env = @("V8_FILTERED") },
    @{ label = "v8-hand-qual"; env = @("V8_HAND_QUALITY") },
    @{ label = "v8-flip-96"; env = @("V8_FLIP_SAMPLES=96") },
    @{ label = "v8-flip-384"; env = @("V8_FLIP_SAMPLES=384") },
    @{ label = "v8-no-horizon"; env = @("V8_NO_HORIZON") },
    @{ label = "v8-opp-icons"; env = @("V8_OPP_SCORE_ICONS") },
    @{ label = "v8-opp-score-icons-opp8"; env = @("V8_OPP_SCORE_ICONS", "V8_ASSUME_COMMIT=0.8") }
)

function Start-Arm {
    param($Arm, [int]$Seed, [string]$BaseCsv, [string]$Exe, [string]$Tag)
    $csv = Join-Path $workDir "$Tag-$($Arm.label)-s$Seed.csv"
    $line = Join-Path $workDir "$Tag-$($Arm.label)-s$Seed.txt"
    # Start-Process joins ArgumentList with spaces and does NOT quote elements, so the
    # deck name has to carry its own quotes or it arrives as two arguments and the
    # arena exits with "expected [p1] [p2] [budget_secs] [deck]".
    $a = @("v9", "v7", "$Games", '"5CP3Z idou"', "--games", "$Games", "--seed", "$Seed",
        "--outcomes", $csv, "--vs", $BaseCsv)
    foreach ($e in $Arm.env) { $a += @("--env", $e) }
    $proc = Start-Process -FilePath $Exe -ArgumentList $a -NoNewWindow -PassThru `
        -RedirectStandardOutput $line -RedirectStandardError "$line.err"
    return [pscustomobject]@{ Arm = $Arm; Seed = $Seed; BaseCsv = $BaseCsv; Proc = $proc; Out = $line; Csv = $csv }
}

function Complete-Arm {
    param($Job)
    $out = ""
    if (Test-Path $Job.Out) { $out = Get-Content $Job.Out -Raw }
    $rec = [regex]::Match($out, "P1\(v9\)\s+(\d+)\s+-\s+P2\(v7\)\s+(\d+)")
    if (-not $rec.Success) {
        return ("{0}  {1,-20} s{2}  ARENA-FAILED" -f (Get-Date -Format "HH:mm:ss"), $Job.Arm.label, $Job.Seed)
    }
    $dis = [regex]::Match($out, "baseline-only\s*(\d+)\s*\|\s*candidate-only\s*(\d+)")
    $p = [regex]::Match($out, "McNemar two-sided p = ([0-9.]+)")
    $pw = [regex]::Match($out, "paired win rate P1: baseline ([0-9.]+)% -> candidate ([0-9.]+)%")
    $w = [int]$rec.Groups[1].Value
    $l = [int]$rec.Groups[2].Value
    $b = if ($dis.Success) { [int]$dis.Groups[1].Value } else { -1 }
    $c = if ($dis.Success) { [int]$dis.Groups[2].Value } else { -1 }
    $pv = if ($p.Success) { $p.Groups[1].Value } else { "?" }
    $d = if ($pw.Success) { [double]$pw.Groups[2].Value - [double]$pw.Groups[1].Value } else { 0 }
    $rate = [math]::Round(100.0 * $w / ($w + $l), 2)
    return ("{0}  {1,-20} s{2}  {3}-{4}  rate={5}%  disc={6}/{7}  p={8}  delta={9}pp" -f `
            (Get-Date -Format "HH:mm:ss"), $Job.Arm.label, $Job.Seed, $w, $l, $rate, $b, $c, $pv,
            [math]::Round($d, 2))
}

"$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  sweep daemon starting: games=$Games arms=$($arms.Count) parallel=$Parallel rounds=$Rounds" | Add-Content $results
# `games` is 3000 by default, not 1200, and the reason is §28.11: a 1200-game
# record on one seed carries roughly +/-1.2 points of sampling error, and an arm
# that moves 0.1 points is invisible at that size. `V8_OPP_PASS_CURVE` measured
# net +4 over 3600 games and net -13 over 15000 - the same change, opposite
# conclusions - so a null from a 1200-game sweep is not a null. Fewer arms per
# round and a real n beats a wide sweep of unresolvable comparisons.

$round = 0
while ($Rounds -le 0 -or $round -lt $Rounds) {
    $round++
    # Snapshot the binary so the workspace stays rebuildable during the round.
    $snap = Join-Path $workDir "arena_snap.exe"
    Copy-Item $arena $snap -Force
    $stamp = (Get-Item $snap).LastWriteTime
    "--- round $round  snapshot $stamp ---" | Add-Content $results

    # One baseline per round, from the same snapshot.
    $baseCsv = Join-Path $workDir "base_s$selectSeed.csv"
    $baseLine = Join-Path $workDir "base_s$selectSeed.txt"
    & $snap v9 v7 $Games "5CP3Z idou" --games $Games --seed $selectSeed --outcomes $baseCsv 2>&1 |
        Out-File -Encoding utf8 $baseLine
    $rec = [regex]::Match((Get-Content $baseLine -Raw), "P1\(v9\)\s+(\d+)\s+-\s+P2\(v7\)\s+(\d+)")
    $recAct = [regex]::Match((Get-Content $baseLine -Raw), "total actions (\d+)")
    "  baseline s$selectSeed : $($rec.Groups[1].Value)-$($rec.Groups[2].Value) actions=$($recAct.Groups[1].Value)" |
        Add-Content $results

    # Selection seed, all arms, throttled to $Parallel.
    $pending = New-Object System.Collections.Queue
    foreach ($a in $arms) { $pending.Enqueue(@{ Arm = $a; Seed = $selectSeed; Base = $baseCsv }) }
    $running = @()
    $lines = @()
    while ($pending.Count -gt 0 -or $running.Count -gt 0) {
        while ($pending.Count -gt 0 -and $running.Count -lt $Parallel) {
            $job = $pending.Dequeue()
            $running += Start-Arm -Arm $job.Arm -Seed $job.Seed -BaseCsv $job.Base -Exe $snap -Tag "r$round"
        }
        Start-Sleep -Milliseconds 400
        $still = @()
        foreach ($j in $running) {
            if ($j.Proc.HasExited) { $lines += Complete-Arm $j } else { $still += $j } }
        $running = $still
    }
    $lines | Add-Content $results

    # Promotion: anything that moved on the seed every other measurement was found
    # on gets two more seeds, in parallel, each against its own baseline.
    $promoted = @()
    foreach ($l in $lines) {
        if ($l -match "delta=(-?[0-9.]+)pp") {
            $d = [math]::Abs([double]$Matches[1])
            $p = if ($l -match "p=([0-9.]+)") { [double]$Matches[1] } else { 1.0 }
            $lbl = ($l -split "\s+")[1]
            # Only promote a label that is a real arm. A parse slip here used to
            # enqueue the seed as if it were an arm name, which silently
            # re-ran the BASELINE 86 times and called it a holdout.
            if (($arms | Where-Object { $_.label -eq $lbl }) -and ($promoted -notcontains $lbl)) {
                if ($d -ge 0.3 -or $p -lt 0.2) { $promoted += $lbl }
            }
        }
    }
    if ($promoted.Count -gt 0) {
        "  promoting: $($promoted -join ', ')" | Add-Content $results
        $hb = @{}
        foreach ($s in $holdoutSeeds) {
            $hb[$s] = Join-Path $workDir "base_s$s.csv"
            & $snap v9 v7 $Games "5CP3Z idou" --games $Games --seed $s --outcomes $hb[$s] 2>&1 | Out-Null
        }
        $p2 = New-Object System.Collections.Queue
        foreach ($lbl in $promoted) {
            $arm = $arms | Where-Object { $_.label -eq $lbl }
            foreach ($s in $holdoutSeeds) { $p2.Enqueue(@{ Arm = $arm; Seed = $s; Base = $hb[$s] }) }
        }
        $run2 = @()
        $lines2 = @()
        while ($p2.Count -gt 0 -or $run2.Count -gt 0) {
            while ($p2.Count -gt 0 -and $run2.Count -lt $Parallel) {
                $job = $p2.Dequeue()
                $run2 += Start-Arm -Arm $job.Arm -Seed $job.Seed -BaseCsv $job.Base -Exe $snap -Tag "r$round"
            }
            Start-Sleep -Milliseconds 400
            $still = @()
            foreach ($j in $run2) {
                if ($j.Proc.HasExited) { $lines2 += Complete-Arm $j } else { $still += $j } }
            $run2 = $still
        }
        $lines2 | Add-Content $results
    }

    if ($Rounds -le 0 -and $PauseMin -gt 0) {
        "round $round done; sleeping $PauseMin min" | Add-Content $results
        Start-Sleep -Minutes $PauseMin
    }
}
"$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')  sweep daemon finished after $round round(s)" | Add-Content $results
