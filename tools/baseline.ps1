# tools/baseline.ps1 — Phase-0 performance baseline suite.
# See docs/PERF_BASELINE_PLAN.md. Run from repo root:
#   pwsh tools/baseline.ps1 [-Games 500] [-Repeat 3] [-SkipBench] [-SkipFlame] [-Quick]
#
# Produces: test_output/baseline_<date>.md + raw logs under test_output/baseline_<date>/

param(
    [int]$Games = 500,          # games per deck for sim_bench random sweep
    [int]$Repeat = 3,           # repeats for variance
    [int]$V2Games = 150,        # games for policy-inclusive v2 run
    [int]$ArenaGames = 500,     # bot_arena cross-check games
    [int]$AllocGames = 200,     # games for alloc profile
    [string]$Deck = "5CP3Z idou",
    [string]$Seed = "1",
    [switch]$SkipBench,         # skip Criterion
    [switch]$SkipFlame,         # skip flamegraph capture
    [switch]$SkipAlloc,         # skip alloc profile
    [switch]$Quick              # small smoke: 50 games, 1 repeat, skip bench/flame/alloc
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path (Join-Path $Root "engine"))) {
    $Root = (Get-Location).Path
}
$Engine = Join-Path $Root "engine"
$Stamp = Get-Date -Format "yyyy-MM-dd_HHmm"
$OutDir = Join-Path $Root "test_output\baseline_$Stamp"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

if ($Quick) {
    $Games = 50; $Repeat = 1; $V2Games = 20; $ArenaGames = 50; $AllocGames = 20
    $SkipBench = $true; $SkipFlame = $true; $SkipAlloc = $true
}

function Invoke-Logged {
    param([string]$Name, [string]$FilePath, [string[]]$ArgList, [string]$WorkDir)
    $log = Join-Path $OutDir "$Name.log"
    Write-Host ">> $Name" -ForegroundColor Cyan
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    # Start-Process does not quote args containing spaces — embed quotes ourselves.
    $quoted = $ArgList | ForEach-Object {
        if ($_ -match '[\s"]') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ }
    }
    $p = Start-Process -FilePath $FilePath -ArgumentList $quoted -WorkingDirectory $WorkDir `
        -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput "$log.out" -RedirectStandardError "$log.err"
    $sw.Stop()
    [pscustomobject]@{
        Name = $Name
        Exit = $p.ExitCode
        Sec  = [math]::Round($sw.Elapsed.TotalSeconds, 2)
        Log  = $log
    }
}

$results = @()
$summary = @()
$failed = $false

# ── 1. Provenance ────────────────────────────────────────────────────────
$gitHead = (git -C $Root rev-parse HEAD 2>$null)
$gitStatus = (git -C $Root status --short 2>$null) -join "`n"
$features = "default (no profiling, no headless, release)"
$provenance = @"
# Engine performance baseline — $Stamp

- git HEAD: ``$gitHead``
- git status (dirty files listed as-is):
``````
$gitStatus
``````
- features: $features
- timing region: card DB + deck templates outside; per-game deal + full loop inside
- execute path: ``game_setup::execute_action`` + ``settle_single_player_state``
- policy default: random (Lcg); engine seed via ``rng::seed`` per game
- deck sweep: all ``web_ui/decks/*.txt``; v2/arena/alloc use ``$Deck``
- games/deck: $Games × $Repeat repeats; v2: $V2Games; arena cross-check: $ArenaGames

"@

# ── 2. Build sim_bench ───────────────────────────────────────────────────
$build = Invoke-Logged -Name "00_build_sim_bench" -FilePath "cargo" `
    -ArgList @("build", "--release", "--bin", "sim_bench") -WorkDir $Engine
$results += $build
if ($build.Exit -ne 0) { throw "sim_bench build failed" }

# ── 3. Canonical random gps (all decks) ─────────────────────────────────
$rand = Invoke-Logged -Name "01_sim_bench_random" -FilePath "cargo" `
    -ArgList @("run", "--release", "--bin", "sim_bench", "--",
        "--games", "$Games", "--seed", $Seed, "--policy", "random",
        "--repeat", "$Repeat") -WorkDir $Engine
$results += $rand
if ($rand.Exit -ne 0) { $failed = $true }

# ── 4. Policy-inclusive v2 gps (fixed deck) ─────────────────────────────
$v2 = Invoke-Logged -Name "02_sim_bench_v2" -FilePath "cargo" `
    -ArgList @("run", "--release", "--bin", "sim_bench", "--",
        "--games", "$V2Games", "--deck", $Deck, "--seed", $Seed,
        "--policy", "v2", "--repeat", "1") -WorkDir $Engine
$results += $v2
if ($v2.Exit -ne 0) { $failed = $true }

# ── 5. bot_arena cross-check (random random) ────────────────────────────
$arena = Invoke-Logged -Name "03_bot_arena_crosscheck" -FilePath "cargo" `
    -ArgList @("run", "--release", "--bin", "bot_arena", "--",
        "--games", "$ArenaGames", "--seed", $Seed,
        "random", "random", "10", $Deck) -WorkDir $Engine
$results += $arena
if ($arena.Exit -ne 0) { $failed = $true }

# ── 6. Criterion baseline (optional) ────────────────────────────────────
if (-not $SkipBench) {
    # --bench performance: bare `cargo bench` also runs lib unittest harnesses
    # which reject criterion flags like --save-baseline.
    $bench = Invoke-Logged -Name "04_criterion_game_throughput" -FilePath "cargo" `
        -ArgList @("bench", "--features", "profiling", "--bench", "performance", "--",
            "game_throughput", "--save-baseline", "base") -WorkDir $Engine
    $results += $bench
    if ($bench.Exit -ne 0) { $failed = $true }
}

# ── 7. Flamegraph capture (optional) ────────────────────────────────────
if (-not $SkipFlame) {
    $folded = Join-Path $OutDir "profile_target.folded"
    $stderrLog = Join-Path $OutDir "profile_target.stderr"
    Write-Host ">> 05_flamegraph_capture" -ForegroundColor Cyan
    Push-Location $Engine
    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = "cargo"
        $psi.Arguments = "run --release --features profiling --bin profile_target"
        $psi.WorkingDirectory = $Engine
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.UseShellExecute = $false
        $p = [System.Diagnostics.Process]::Start($psi)
        $outTask = $p.StandardOutput.ReadToEndAsync()
        $errTask = $p.StandardError.ReadToEndAsync()
        $p.WaitForExit()
        [System.IO.File]::WriteAllText($folded, $outTask.Result)
        [System.IO.File]::WriteAllText($stderrLog, $errTask.Result)
        $flameExit = $p.ExitCode
    } finally { Pop-Location }
    $results += [pscustomobject]@{ Name = "05_flamegraph_capture"; Exit = $flameExit; Sec = ""; Log = $folded }
    if ($flameExit -ne 0) { $failed = $true }
    if ((Get-Item $folded).Length -gt 0) {
        $selfSvg = Join-Path $OutDir "profile_target.self.svg"
        $callerSvg = Join-Path $OutDir "profile_target.caller.svg"
        Invoke-Logged -Name "06_gen_flamegraph_self" -FilePath "cargo" `
            -ArgList @("run", "--example", "gen_flamegraph", "--",
                $folded, $selfSvg, "--self") -WorkDir $Engine | ForEach-Object { $results += $_ }
        Invoke-Logged -Name "07_gen_flamegraph_caller" -FilePath "cargo" `
            -ArgList @("run", "--example", "gen_flamegraph", "--",
                $folded, $callerSvg) -WorkDir $Engine | ForEach-Object { $results += $_ }
    } else {
        Write-Warning "folded output empty — flamegraph skipped"
        $failed = $true
    }
}

# ── 8. Alloc profile (optional) ─────────────────────────────────────────
if (-not $SkipAlloc) {
    # Separate build: alloc_tracker changes the global allocator; keep it out
    # of the timing builds above.
    $env:RABUKA_ALLOC_TRACK = "1"
    try {
        $alloc = Invoke-Logged -Name "08_sim_bench_alloc" -FilePath "cargo" `
            -ArgList @("run", "--release", "--features", "alloc_tracker",
                "--bin", "sim_bench", "--",
                "--games", "$AllocGames", "--deck", $Deck, "--seed", $Seed,
                "--policy", "random", "--repeat", "1", "--alloc") -WorkDir $Engine
        $results += $alloc
        if ($alloc.Exit -ne 0) { $failed = $true }
    } finally {
        Remove-Item Env:RABUKA_ALLOC_TRACK -ErrorAction SilentlyContinue
    }
}

# ── 9. Extract key numbers ──────────────────────────────────────────────
function Get-SummaryLine([string]$logBase) {
    foreach ($path in @("$logBase.log.out", "$logBase.out", "$logBase")) {
        if (Test-Path $path) {
            $hit = Select-String -Path $path -Pattern "SIM_BENCH_SUMMARY" | Select-Object -First 1
            if ($hit) { return $hit.Line }
        }
    }
    return $null
}
$randSummary = Get-SummaryLine (Join-Path $OutDir "01_sim_bench_random")
$v2Summary = Get-SummaryLine (Join-Path $OutDir "02_sim_bench_v2")
$arenaLine = $null
$arenaOut = Join-Path $OutDir "03_bot_arena_crosscheck.log.out"
$arenaErr = Join-Path $OutDir "03_bot_arena_crosscheck.log.err"
if (-not (Test-Path $arenaOut)) { $arenaOut = Join-Path $OutDir "03_bot_arena_crosscheck.out" }
if (-not (Test-Path $arenaErr)) { $arenaErr = Join-Path $OutDir "03_bot_arena_crosscheck.err" }
foreach ($f in @($arenaOut, $arenaErr)) {
    if (Test-Path $f) {
        $hit = Select-String -Path $f -Pattern "gps" | Select-Object -First 1
        if ($hit) { $arenaLine = $hit.Line; break }
    }
}

$fence3 = [string][char]96 * 3
$fence6 = [string][char]96 * 6
$nl = [string][char]10

$md = $provenance
$md += $nl + "## Steps" + $nl + $nl + "| step | exit | secs | log |" + $nl + "|---|---:|---:|---|" + $nl
foreach ($r in $results) {
    $md += "| $($r.Name) | $($r.Exit) | $($r.Sec) | $(Split-Path -Leaf $r.Log) |" + $nl
}
$md += $nl + "## Canonical gps (sim_bench --policy random)" + $nl + $nl + $fence6 + $nl
if ($randSummary) { $md += ($randSummary -join $nl) + $nl }
$md += $fence6 + $nl
$md += $nl + "## Policy-inclusive gps (sim_bench --policy v2, deck=$Deck)" + $nl + $nl + $fence6 + $nl
if ($v2Summary) { $md += ($v2Summary -join $nl) + $nl }
$md += $fence6 + $nl
$md += $nl + "## bot_arena cross-check (random random)" + $nl + $nl + $fence3 + $nl
if ($arenaLine) { $md += $arenaLine + $nl }
$md += $fence3 + $nl
$md += $nl + "## Caveats" + $nl + $nl
$md += "- Do NOT compare absolute gps of Criterion / profiling captures to sim_bench." + $nl
$md += "- Criterion and flamegraph runs use --features profiling, which changes gameplay metadata work." + $nl
$md += "- bot_arena gps includes policy_call checkpoints + V3Plan detection overhead." + $nl
$md += "- Re-run within ~5% of these numbers before attributing a later delta to an optimization." + $nl
if ($failed) { $md += $nl + "**WARNING:** one or more steps exited nonzero - see step table." + $nl }

$mdPath = Join-Path $Root "test_output\baseline_$Stamp.md"
[System.IO.File]::WriteAllText($mdPath, $md)
Write-Host ""
Write-Host "Baseline written: $mdPath" -ForegroundColor Green
Write-Host "Raw logs: $OutDir" -ForegroundColor Green
if ($failed) { exit 1 } else { exit 0 }
