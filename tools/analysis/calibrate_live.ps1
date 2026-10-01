param(
    [string]$Dir = "..\test_output\candsplit",
    [int]$Games = 120
)

# Joins the bot's own per-decision prediction (V8CAL, stderr, one line per live
# set) against the engine's own verdict in the arena's per-game log.
#
# Two things this makes measurable that were previously eyeballed:
#   1. p_place calibration - is the model's confidence honest?
#   2. per-card calibration - is the bias UNIFORM across cards? A uniform bias
#      is a monotone transform and cannot reorder an argmax; a per-card bias
#      can, and is a real defect rather than a scaling nuisance.

$rows = New-Object System.Collections.ArrayList

for ($i = 1; $i -le $Games; $i++) {
    $n = "{0:D3}" -f $i
    $errPath = Join-Path $Dir "c_$n.err"
    $logPath = Join-Path $Dir "l_$n.txt"
    if (-not (Test-Path $errPath) -or -not (Test-Path $logPath)) { continue }

    $live = @{}
    foreach ($l in [System.IO.File]::ReadAllLines($logPath)) {
        if ($l -match '^t(\d+)\|SYSTEM\|live_result\|LIVE P1-WINS \| P1 set\[(?<s>[^\]]*)\] yell (?<y>\d+)/(?<f>\d+)flips score=(?<sc>\d+) . succ=(?<c>\d+)\(\+(?<d>\d+)\)') {
            $t = [int]$Matches[1]
            if (-not $live.ContainsKey($t)) {
                $live[$t] = @{
                    set = $Matches.s
                    y   = [int]$Matches.y
                    f   = [int]$Matches.f
                    sc  = [int]$Matches.sc
                    d   = [int]$Matches.d
                }
            }
        }
    }
    if ($live.Count -eq 0) { continue }

    $seen = @{}
    foreach ($cl in [System.IO.File]::ReadAllLines($errPath)) {
        if ($cl -notmatch 'V8CAL t=(\d+) me=0 ') { continue }
        $t = [int]$Matches[1]
        if ($seen.ContainsKey($t)) { continue }
        if (-not $live.ContainsKey($t)) { continue }
        $seen[$t] = 1
        $r = $live[$t]

        $placed = 0
        if ($r.d -gt 0) { $placed = 1 }
        $passed = 0
        if ($r.set -ne '' -and $r.y -ge $r.f) { $passed = 1 }

        $cm = [regex]::Match($cl, 'cands=(?<c>.*)$')
        foreach ($e in [regex]::Matches($cm.Groups['c'].Value, '\[(?<l>[^\|]*)\|pp=(?<pp>[\d.]+)\|pl=(?<pl>[\d.]+)\|v=(?<v>[\d.]+)\]')) {
            $lab = $e.Groups['l'].Value
            if ($lab -eq 'FOLD') { continue }
            $slash = $lab.IndexOf('/')
            if ($slash -lt 0) { continue }
            $cardNo = $lab.Substring(0, $slash)
            $need = [int]$lab.Substring($slash + 1)
            $colon = $cardNo.IndexOf(':')
            $score = 0
            if ($colon -ge 0) { $score = [int]$cardNo.Substring($colon + 1) }
            [void]$rows.Add([pscustomobject]@{
                game      = $i
                turn      = $t
                card      = $cardNo
                need      = $need
                score     = $score
                model_pp  = [double]$e.Groups['pp'].Value
                model_pl  = [double]$e.Groups['pl'].Value
                model_v   = [double]$e.Groups['v'].Value
                rset      = ($r.set -ne '')
                rpassed   = $passed
                rplaced   = $placed
            })
        }
    }
}

Write-Output "candidate rows: $($rows.Count)"
$rows | Export-Csv "..\test_output\percard.csv" -NoTypeInformation

Write-Output ""
Write-Output "=== PER-CARD: model p_place vs realized placement, restricted to the life actually set ==="
$chosen = $rows | Where-Object { $_.rset }
$chosen | Group-Object card | Sort-Object Count -Descending | ForEach-Object {
    $g = $_.Group
    $mp = ($g | Measure-Object model_pl -Average).Average
    $rp = ($g | Measure-Object rplaced -Sum).Sum
    $need = ($g | Select-Object -First 1).need
    $sc = ($g | Select-Object -First 1).score
    "{0,-26} need={1,-3} score={2,-2} n={3,-5} model_pl={4:N3}  realized={5,6:P1}  ratio={6:N2}" -f `
        $_.Name, $need, $sc, $g.Count, $mp, ($rp / $g.Count), (($rp / $g.Count) / $mp)
}

Write-Output ""
Write-Output "=== OVERALL calibration (chosen life only) ==="
$mpAll = ($chosen | Measure-Object model_pl -Average).Average
$rpAll = ($chosen | Measure-Object rplaced -Sum).Sum
"model p_place mean = {0:N3}   realized placement = {1:P1}   global bias = {2:N2}" -f `
    $mpAll, ($rpAll / $chosen.Count), (($rpAll / $chosen.Count) / $mpAll)

Write-Output ""
Write-Output "=== BIAS BY NEED SIZE (is the miscalibration need-dependent?) ==="
$chosen | Group-Object need | Sort-Object { [int]$_.Name } | ForEach-Object {
    $g = $_.Group
    $mp = ($g | Measure-Object model_pl -Average).Average
    $rp = ($g | Measure-Object rplaced -Sum).Sum
    "  need={0,-3} n={1,-5} model_pl={2:N3}  realized={3,6:P1}  bias={4:N2}" -f `
        $_.Name, $g.Count, $mp, ($rp / $g.Count), (($rp / $g.Count) / $mp)
}

Write-Output ""
Write-Output "=== BIAS BY SCORE (does the model ignore printed score?) ==="
$chosen | Group-Object score | Sort-Object { [int]$_.Name } | ForEach-Object {
    $g = $_.Group
    $mp = ($g | Measure-Object model_pl -Average).Average
    $rp = ($g | Measure-Object rplaced -Sum).Sum
    "  score={0,-3} n={1,-5} model_pl={2:N3}  realized={3,6:P1}  bias={4:N2}" -f `
        $_.Name, $g.Count, $mp, ($rp / $g.Count), (($rp / $g.Count) / $mp)
}