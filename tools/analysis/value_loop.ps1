param(
    [int]$Rounds = 6,
    [int]$GamesPerRound = 900,
    [int]$VF_EPOCHS = 60,
    [string]$Deck = "5CP3Z idou",
    [int]$Seed = 11,
    [string]$Opponent = "mirror"
)

# The iterative value-function loop from Swiechowski et al. (arXiv 1808.04794),
# Sec. IV-C, adapted to this repo. The paper's own summary:
#
#   "MCTS with a heuristic may be used to generate games of progressively better
#    quality. Those games may then be used to create more accurate heuristics,
#    which may then be used to generate games of even better quality."
#
# Their numbers: 20000 games to start, then 3000 games per round, dataset clipped
# to 1M in a FIFO buffer, retrained from scratch each round, 80/20 split. They ran
# 64 rounds and the BEST net was iteration 21, not the last - which is why this
# script keeps every checkpoint and never assumes the final one wins.
#
# Why iteration is needed here rather than one big run: [P5] warns that a value
# function needs >1e5 INDEPENDENT samples to avoid severe overfitting, and
# [P1] warns consecutive states are highly correlated. Each round therefore
# contributes only `VF_PER_GAME` states per game from FRESH games, and the
# accumulator grows the independent-sample count rather than reusing one pool.
#
# Every round is measured in the arena before the next one starts, so a
# regression is recorded rather than compounded.

$ErrorActionPreference = "Continue"
$arena = "C:\rust_targets\release\bot_arena.exe"
$collector = "C:\rust_targets\release\ppo_collect.exe"
$trainer = "C:\rust_targets\release\value_train.exe"
# `tools/analysis/value_loop.ps1` -> repo root is two levels up, not one. Using
# `Split-Path -Parent $PSScriptRoot` resolved to `tools/engine`, which does not
# exist, and the loop then ran from the wrong working directory while reporting
# progress - the worst kind of failure, because it looked alive.
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Push-Location (Join-Path $root "engine")

$outDir = Join-Path $root "training\iterloop"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$log = Join-Path $outDir "loop.log"

function Say($msg) {
    $line = "[{0}] {1}" -f (Get-Date -Format "HH:mm:ss"), $msg
    Write-Output $line
    Add-Content -Path $log -Value $line
}

$env:PPO_DECK = $Deck
$env:PPO_SEATS = "2"
$env:PPO_EPS = "0.15"
# Discount 1.0 so the stored reward's SIGN is the raw terminal outcome with no
# discount to undo - MuZero: "in board games the discount is assumed to be 1".
$env:PPO_DISCOUNT = "1.0"

$corpora = New-Object System.Collections.Generic.List[string]
$bestRate = 0.0
$bestRound = -1

# Round 0: the corpus that already exists, so the loop starts from a real net.
$seed0 = Join-Path $root "training\val_self.bin"
if (Test-Path $seed0) { $corpora.Add($seed0); Say "seed corpus: $seed0" }

for ($r = 1; $r -le $Rounds; $r++) {
    Say "=== ROUND $r / $Rounds ==="

# Opponent choice matters for throughput, not just fidelity.
    #
    # Against v7 the collector runs at ~30 steps/s because v7 simulates every
    # candidate action, so one round is ~35 minutes and the loop cannot iterate.
    # Mirror self-play runs ~137 steps/s and, because both seats are the same
    # policy, produces a naturally balanced 50/50 outcome - which is exactly the
    # label balance a value net wants.
    #
    # The fidelity cost is measured, not assumed: [P3] trained a value net on
    # RANDOM play and dropped only 0.79 -> 0.75 AUC when evaluated on states from
    # a completely different, strong search agent. So a policy mismatch of this
    # size is worth far less than 4x the games, and games are the binding
    # constraint ([P5]: >10^5 independent samples).
    $dataFile = Join-Path $outDir ("round$r.bin")
    if (Test-Path $dataFile) {
        Say "  round $r data already present, reusing"
    } else {
        $env:PPO_OPP_BOT = $Opponent
        Say "  collecting $GamesPerRound games, opponent=$Opponent (seed $Seed)"
        & $collector $GamesPerRound $dataFile 2>&1 |
            Where-Object { $_ -match "Done:" } | ForEach-Object { Say "  $_" }
    }
    if (Test-Path $dataFile) { $corpora.Add($dataFile) }

    $list = ($corpora -join ",")
    $ckpt = Join-Path $outDir ("value_r$r.bin")
    $env:VF_EPOCHS = "$VF_EPOCHS"
    $env:VF_LR = "0.001"
    $env:VF_BATCH = "256"
    $env:VF_PER_GAME = "3"
    Say "  training on $((($corpora | Measure-Object).Count)) corpora"
    & $trainer $list $ckpt 2>&1 | ForEach-Object {
        if ($_ -match "total samples|baseline MSE|pearson r") { Say "  $_" }
    }
    if (-not (Test-Path $ckpt)) { Say "  training produced no checkpoint; stopping"; break }

    # Measure it. The paper is explicit that win rate, not validation loss, is the
    # selector: "there has not been yet proposed any universal benchmarking
    # methods" (Sec. V).
    $env:ARENA_BRIEF = "1"
    Say "  measuring round $r against v7 (400 games)"
    $out = & $arena v9 v7 120 $Deck --games 400 --seed $Seed --env V8_ALL_TERMS `
        --env "V7_LEARNED_VALUE=$ckpt" --env "V7_LEARNED_VALUE_WEIGHT=1000" 2>&1 |
        Where-Object { $_ -match "RESULT" } | Select-Object -Last 1
    Say "  $out"
    if ($out -match "\|\s*(\d+)\s*-\s*(\d+)\s*\|") {
        $w = [int]$Matches[1]; $l = [int]$Matches[2]
        $rate = $w / [math]::Max(1, $w + $l)
        Say ("  round {0} win rate {1:P1} ({2}-{3})" -f $r, $rate, $w, $l)
        if ($rate -gt $bestRate) {
            $bestRate = $rate
            $bestRound = $r
            Copy-Item $ckpt (Join-Path $outDir "value_best.bin") -Force
            Say "  ** new best: round $r at {0:P1} **" -f $rate
        }
    }
    Remove-Item Env:\V7_LEARNED_VALUE -ErrorAction SilentlyContinue
}

Say "loop finished. best round $bestRound at $bestRate"
if ($bestRound -ge 0) { Say "best checkpoint: $outDir\value_best.bin" }
Pop-Location