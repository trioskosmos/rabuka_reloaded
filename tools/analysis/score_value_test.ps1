param(
    [string]$Dir = "..\test_output\candsplit",
    [int]$Games = 120
)

# HYPOTHESIS: the live-set argmax under-weights PRINTED SCORE.
#
# The audit established that v9's committed checks PASS 91-100% of the time, and
# that 51% of all live phases are decided purely by the score comparison, with
# razor-thin margins (+1 wins 33/33, -1 loses 32/32). If the check passes either
# way, the ONLY thing that separates a placement from none is score -- and score
# has no term in the value function beyond `contested_masses`' tie/above/below
# lookup against a 6-valued opponent prior.
#
# The test: on every decision where v9 committed, was a HIGHER-SCORING life
# present in the enumerated candidate table, and did v9 take it? If a better-
# scoring life was routinely on the table and declined, the ranking is the defect
# and the fix is to price score, not to rescale p_place.
#
# Reads the whole candidate table V8CAL emits, so this is measured on what the
# argmax actually had in front of it -- not on what a re-run would produce.

$decisions = New-Object System.Collections.ArrayList

for ($i = 1; $i -le $Games; $i++) {
    $n = "{0:D3}" -f $i
    $errPath = Join-Path $Dir "c_$n.err"
    if (-not (Test-Path $errPath)) { continue }

    $live = @{}
    $logPath = Join-Path $Dir "l_$n.txt"
    if (Test-Path $logPath) {
        foreach ($l in [System.IO.File]::ReadAllLines($logPath)) {
            if ($l -match '^t(\d+)\|SYSTEM\|live_result\|LIVE P1-WINS \| P1 set\[(?<s>[^\]]*)\] yell \d+/\d+flips score=(?<sc>\d+) . succ=(?<c>\d+)\(\+(?<d>\d+)\)') {
                $t = [int]$Matches[1]
                if (-not $live.ContainsKey($t)) {
                    $live[$t] = @{
                        raw   = $Matches.s
                        score = [int]$Matches.sc
                        place = [int]$Matches.d
                    }
                }
            }
        }
    }

    $seen = @{}
    foreach ($cl in [System.IO.File]::ReadAllLines($errPath)) {
        if ($cl -notmatch 'V8CAL t=(\d+) me=0 .*?set=(?<set>\d) .*?pplace=(?<pl>[\d.]+) .*? cands=(?<cands>.*)$') { continue }
        $t = [int]$Matches[1]
        if ($seen.ContainsKey($t)) { continue }
        $seen[$t] = 1
        $didSet = [int]$Matches['set']

        $best = $null
        $bestScore = -1
        $chosenScore = $null
        foreach ($e in [regex]::Matches($Matches['cands'], '\[(?<l>[^\|]*)\|pp=(?<pp>[\d.]+)\|pl=(?<pl>[\d.]+)\|v=(?<v>[\d.]+)\]')) {
            $lab = $e.Groups['l'].Value
            $val = [double]$e.Groups['v'].Value
            $pl = [double]$e.Groups['pl'].Value
            if ($lab -eq 'FOLD') { continue }
            $slash = $lab.IndexOf('/')
            if ($slash -lt 0) { continue }
            $cardNo = $lab.Substring(0, $slash)
            $colon = $cardNo.IndexOf(':')
            if ($colon -lt 0) { continue }
            $score = [int]$cardNo.Substring($colon + 1)

            if ($null -eq $best -or $val -gt $best.val) {
                $best = @{ lab = $lab; val = $val; pl = $pl; score = $score }
            }
            if ($score -gt $bestScore) { $bestScore = $score }
        }
        if ($null -eq $best) { continue }

        # Which card did the engine actually see? Match by printed score of the
        # set annotation; ties are rare enough that the max-score card is a fair
        # read of "what was taken" when the table's argmax was taken.
        $chosenScore = $best.score

        $r = $null
        if ($live.ContainsKey($t)) { $r = $live[$t] }

        [void]$decisions.Add([pscustomobject]@{
            game        = $i
            turn        = $t
            didSet      = $didSet
            bestCard    = $best.lab
            bestValue   = $best.val
            bestPL      = $best.pl
            chosenScore = $chosenScore
            maxScore    = $bestScore
            realScore   = $(if ($r) { $r.score } else { $null })
            realPlace   = $(if ($r) { $r.place } else { $null })
        })
    }
}

Write-Output "decisions parsed: $($decisions.Count)"
$commits = $decisions | Where-Object { $_.didSet -eq 1 }
Write-Output "of which committed: $($commits.Count)"
Write-Output ""
Write-Output "=== IS A HIGHER-SCORING LIFE AVAILABLE AT COMMIT TIME? ==="
$up = $commits | Where-Object { $_.maxScore -gt $_.chosenScore }
Write-Output "  commits where a higher-scoring life was on the table: $($up.Count) / $($commits.Count)"
Write-Output ""
Write-Output "=== SCORE OF THE COMMITTED CARD vs THE MAXIMUM AVAILABLE ==="
$commits | Group-Object chosenScore | Sort-Object { [int]$_.Name } | ForEach-Object {
    $g = $_.Group
    "{0,-8} chosen {1,-5} times ({2,5:P1})" -f $_.Name, $g.Count, ($g.Count / $commits.Count)
}
Write-Output ""
Write-Output "=== ARGMAX p_place: is it saturated, i.e. blind to score? ==="
$commits | Group-Object {
    if ($_.bestPL -ge 0.95) { 'p_place >= 0.95' } elseif ($_.bestPL -ge 0.8) { '0.80-0.95' } elseif ($_.bestPL -ge 0.5) { '0.50-0.80' } else { '< 0.50' }
} | Sort-Object Name | ForEach-Object {
    $g = $_.Group
    $avgScore = ($g | Measure-Object chosenScore -Average).Average
    $placed = 0
    foreach ($x in $g) { if ($null -ne $x.realPlace -and $x.realPlace -gt 0) { $placed++ } }
    "  {0,-16} n={1,-5} avg score chosen={2:N2}  placed={3,-5} ({4,5:P0})" -f `
        $_.Name, $g.Count, $avgScore, $placed, ($placed / $g.Count)
}
Write-Output ""
Write-Output "=== REALIZED SCORE: model-chosen card vs what the engine scored ==="
$withReal = $commits | Where-Object { $null -ne $_.realScore }
"  commits with a readable engine score: $($withReal.Count)"
$byScore = $withReal | Group-Object chosenScore | Sort-Object { [int]$_.Name } | ForEach-Object {
    $g = $_.Group
    "{0,-8} n={1,-5} avg engine score={2:N2}  placed={3,5:P0}" -f `
        $_.Name, $g.Count, (($g | Measure-Object realScore -Average).Average),
        ((($g | Where-Object { $_.realPlace -gt 0 }).Count) / $g.Count)
}
$byScore