# tools/queue_runner.ps1 - persistent build-and-measure loop.
#
# Reads arm specs from test_output/arms.txt, one per line:
#
#     LABEL | KEY=VALUE;KEY=VALUE
#
# and for each: rebuilds release, regenerates the seed-11 baseline, runs the arm
# paired against it, and appends one result line to test_output/arms_results.txt.
# The line is moved to test_output/arms_done.txt so a restart resumes where it
# stopped. Survives session close (started with background_process -Persistent).
#
# This exists so that a measurement is never blocked by a foreground tool call
# being cut short, and so that the workspace stays rebuildable while the parallel
# sweep runs: tools/sweep_daemon.ps1 runs a SNAPSHOT copy of the binary, so
# cargo can overwrite the installed one while the sweep is mid-round.
#
# Usage: pwsh tools/queue_runner.ps1

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent $PSScriptRoot
$arena = "C:\rust_targets\release\bot_arena.exe"
$outDir = Join-Path $root "test_output"
$queue = Join-Path $outDir "arms.txt"
$done = Join-Path $outDir "arms_done.txt"
$results = Join-Path $outDir "arms_results.txt"
$csv = Join-Path $outDir "qr_base_s11.csv"
$outTxt = Join-Path $outDir "qr_base_s11.txt"
$Games = 1200
$seeds = @(11, 23, 31)

Set-Location (Join-Path $root "engine")

function Note([string]$line) {
    Add-Content -Path $results -Value ("{0}  {1}" -f (Get-Date -Format "HH:mm:ss"), $line)
}

Note "queue runner starting (arena=$arena)"
if (-not (Test-Path $queue)) { Set-Content -Path $queue -Value "" }
if (-not (Test-Path $done)) { Set-Content -Path $done -Value "" }

while ($true) {
    $doneKeys = @{}
    foreach ($l in Get-Content $done -ErrorAction SilentlyContinue) {
        if ($l -match "^([^|]+)\|") { $doneKeys[$Matches[1].Trim()] = $true }
    }
    $next = $null
    foreach ($l in Get-Content $queue -ErrorAction SilentlyContinue) {
        $l = $l.Trim()
        if (-not $l -or $l.StartsWith("#")) { continue }
        $label = ($l -split "\|")[0].Trim()
        if (-not $doneKeys.ContainsKey($label)) { $next = $l; break }
    }
    if ($null -eq $next) { Start-Sleep -Seconds 20; continue }

    $parts = $next -split "\|", 2
    $label = $parts[0].Trim()
    $envSpec = if ($parts.Count -gt 1) { $parts[1].Trim() } else { "" }

    # Directives: `@games=2400` and `@seeds=23,31,41,53` prefix the env list.
    #
    # Power, not more arms. Everything still in contention sits at net +/-3 games
    # over 3600, which is exactly the size of the effect and none of its
    # standard error, so no amount of extra arms at n=1200 can separate them.
    # `@games`/`@seeds` buys resolution on the handful that are still in play.
    $Games = 1200
    $armSeeds = $seeds
    if ($envSpec -match "(^|;)\s*@games=(\d+)") {
        $Games = [int]$Matches[2]
    }
    if ($envSpec -match "(^|;)\s*@seeds=([0-9,]+)") {
        $armSeeds = $Matches[2].Split(",") | ForEach-Object { [int]$_ }
    }
    $envSpec = ($envSpec -split ";" | Where-Object { $_ -and $_ -notmatch "^\s*@(games|seeds)=" }) -join ";"

    Note "BUILD $label ($envSpec)"
    $build = & cargo build --release --bin bot_arena 2>&1
    if ($LASTEXITCODE -ne 0) {
        Note "  BUILD-FAILED $label"
        ($next) | Add-Content -Path $done
        continue
    }

    # Baselines and candidates both run, six arenas at a time. Serial, this was
    # six 1200-game runs per arm and the arm queue moved at four arms an hour,
    # which is slower than anyone will wait: a measurement you do not wait for
    # is a measurement that does not happen. Each run is an independent process
    # with its own --seed, so contention changes nothing about the result.
    $envArgs = @()
    if ($envSpec) {
        foreach ($kv in $envSpec -split ";") {
            if ($kv.Trim()) { $envArgs += @("--env", $kv.Trim()) }
        }
    }
    $jobs = @()
    foreach ($s in $armSeeds) {
        $bc = Join-Path $outDir "qr_base_s$s.csv"
        Remove-Item $bc -ErrorAction SilentlyContinue
        $bt = Join-Path $outDir "qr_base_s$s.txt"
        $jobs += Start-Process -FilePath $arena -NoNewWindow -PassThru `
            -RedirectStandardOutput $bt `
            -ArgumentList (@("v9", "v7", "$Games", '"5CP3Z idou"', "--games", "$Games",
                "--seed", "$s", "--outcomes", $bc))
        $jobs[-1] | Add-Member -NotePropertyName Meta -NotePropertyValue @{ Seed = $s; Base = $true; Csv = $bc; Txt = $bt }
    }
    foreach ($j in $jobs) { $j.WaitForExit() }

    # Only now start the candidates. Launching all six together looks faster and
    # is not: the candidate reads the baseline outcomes file as it is being
    # written, and `--vs` then finds no comparable rows and prints no
    # discordance at all - which reads as "0 discordant" rather than as the
    # error it is. The batches stay three-wide; only the barrier is new.
    $cands = @()
    foreach ($s in $armSeeds) {
        $tag = "$label-s$s"
        $cc = Join-Path $outDir "qr_$tag.csv"
        Remove-Item $cc -ErrorAction SilentlyContinue
        $ct = Join-Path $outDir "qr_$tag.txt"
        $cands += Start-Process -FilePath $arena -NoNewWindow -PassThru `
            -RedirectStandardOutput $ct `
            -ArgumentList (@("v9", "v7", "$Games", '"5CP3Z idou"', "--games", "$Games",
                "--seed", "$s", "--outcomes", $cc, "--vs", (Join-Path $outDir "qr_base_s$s.csv")) + $envArgs)
    }
    foreach ($j in $cands) { $j.WaitForExit() }

    # A paired discordance is only meaningful if the two files have the same
    # number of rows. The arena APPENDS to an outcomes file, so a baseline that
    # was not deleted and regenerated silently pairs 3000 games against several
    # thousand and prints a discordance that means nothing - which is exactly
    # what §28.12 caught after a 30,000-game run reported the wrong direction.
    # The row counts are printed with every result from here on.
    $rowcheck = @()
    foreach ($s in $armSeeds) {
        $bcsv = Join-Path $outDir "qr_base_s$s.csv"
        $ccsv = Join-Path $outDir "qr_$label-s$s.csv"
        $bn = if (Test-Path $bcsv) { (Get-Content $bcsv | Measure-Object -Line).Lines - 1 } else { -1 }
        $cn = if (Test-Path $ccsv) { (Get-Content $ccsv | Measure-Object -Line).Lines - 1 } else { -1 }
        $rowcheck += "s$s $bn/$cn"
        if ($bn -ne $cn) {
            Note "  ROW-COUNT MISMATCH $label s$s baseline=$bn candidate=$cn - discordance for this seed is NOT usable"
        }
    }
    Note ("  rows $label " + ($rowcheck -join " "))

    $summary = @()
    $aggDelta = @()
    foreach ($s in $armSeeds) {
        $o = Join-Path $outDir "qr_$label-s$s.txt"
        $txt = if (Test-Path $o) { Get-Content $o -Raw } else { "" }
        $rec = [regex]::Match($txt, "P1\(v9\)\s+(\d+)\s+-\s+P2\(v7\)\s+(\d+)")
        if (-not $rec.Success) { Note "  $label-s$s ARENA-FAILED"; continue }
        $w = [int]$rec.Groups[1].Value
        $l = [int]$rec.Groups[2].Value
        $rate = [math]::Round(100.0 * $w / ($w + $l), 2)
        $dis = [regex]::Match($txt, "baseline-only\s*(\d+)\s*\|\s*candidate-only\s*(\d+)")
        $p = [regex]::Match($txt, "McNemar two-sided p = ([0-9.]+)")
        $b = if ($dis.Success) { [int]$dis.Groups[1].Value } else { -1 }
        $c2 = if ($dis.Success) { [int]$dis.Groups[2].Value } else { -1 }
        $pv = if ($p.Success) { $p.Groups[1].Value } else { "?" }
        $summary += "s$s $w-$l ($rate%) disc=$b/$c2 p=$pv"
        if ($b -ge 0) { $aggDelta += ($c2 - $b) }
    }
    $net = if ($aggDelta.Count) { ($aggDelta | Measure-Object -Sum).Sum } else { 0 }
    Note ("RESULT {0,-22} {1}  net-games={2}  [{3}]" -f $label, $envSpec, $net, ($summary -join " | "))
    ($next) | Add-Content -Path $done
}
