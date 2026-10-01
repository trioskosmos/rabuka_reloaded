param(
    [string]$Dir = "..\test_output\cal3",
    [int]$Games = 150
)

# HYPOTHESIS E, calibration arm: is `clears` (the step function built on
# YIELD_PRIOR) calibrated?
#
# E says the seat asymmetry comes from the model treating a bigger observed
# opponent set as a strictly harder one, via
#     target = 2 * set_size  ->  bar = SCORE_MEDIAN[target]  ->  clears
# That chain is driven by YIELD_PRIOR, the one prior in the model fitted from
# nothing rather than measured.
#
# The test: bucket every observed opponent set by the `opp_pass` the model
# predicted for it, and compare against what the engine actually did. A
# monotone but mis-scaled curve is a calibration fix. A curve that INVERTS, or
# one where the observed (set_size > 1) bucket is worse than predicted while the
# forecast bucket is better, is a structural defect in the bar.

$rows = New-Object System.Collections.ArrayList

for ($i = 1; $i -le $Games; $i++) {
    $n = "{0:D3}" -f $i
    $errPath = Join-Path $Dir "c_$n.err"
    $logPath = Join-Path $Dir "l_$n.txt"
    if (-not (Test-Path $errPath) -or -not (Test-Path $logPath)) { continue }

    $live = @{}
    foreach ($l in [System.IO.File]::ReadAllLines($logPath)) {
        if ($l -match '^t(\d+)\|SYSTEM\|live_result\|.*?\| P1 set\[(?<s>[^\]]*)\].*?\| P2 set\[(?<p2>[^\]]*)\]') {
            $t = [int]$Matches[1]
            if (-not $live.ContainsKey($t)) {
                $p2 = $Matches.p2
                $live[$t] = @{ p2set = ($p2 -ne ''); p2pass = ($p2 -ne '' -and $p2 -notmatch 'FAIL\(') }
            }
        }
    }

    # last cal line per turn, and the opp_set/opp_pass the model used
    $last = @{}
    $order = @()
    foreach ($cl in [System.IO.File]::ReadAllLines($errPath)) {
        if ($cl -notmatch 'V8CAL t=(\d+) me=0 second=(?<sec>\w+) set=(?<set>\d) ppass=(?<pp>[\d.]+) pplace=(?<pl>[\d.]+) opp_pass=(?<op>[\-0-9.]+) opp_set=(?<os>\d+)') { continue }
        $t = [int]$Matches[1]
        if (-not $last.ContainsKey($t)) { $order += $t }
        $last[$t] = @{
            second = ($Matches['sec'] -eq 'true')
            oppPass = [double]$Matches['op']
            oppSet = [int]$Matches['os']
        }
    }

    foreach ($t in $order) {
        if (-not $live.ContainsKey($t)) { continue }
        $e = $last[$t]; $r = $live[$t]
        if (-not $r.p2set) { continue }   # only observed opponent sets have a verdict to score

        [void]$rows.Add([pscustomobject]@{
            game = $i; turn = $t
            seat = $(if ($e.second) { 'OBS' } else { 'UNOBS' })
            oppSet = $e.oppSet
            oppPass = $e.oppPass
            oppPassed = [int]$r.p2pass
        })
    }
}

Write-Output "opponent sets observed: $($rows.Count)"
Write-Output ""
Write-Output "=== MODEL opp_pass vs REALIZED opponent pass, bucketed ==="
$rows | Group-Object {
    if ($_.oppPass -le 0.001) { '0.00 (modelled fold)' } elseif ($_.oppPass -lt 0.4) { '0.01-0.39' } elseif ($_.oppPass -lt 0.7) { '0.40-0.69' } elseif ($_.oppPass -lt 0.95) { '0.70-0.94' } else { '0.95-1.00' }
} | Sort-Object Name | ForEach-Object {
    $g = $_.Group
    $real = ($g | Measure-Object oppPassed -Sum).Sum
    "{0,-20} n={1,-5} model={2:N3}  realized={3,6:P1}  bias={4:N2}" -f `
        $_.Name, $g.Count, (($g | Measure-Object oppPass -Average).Average), ($real / $g.Count), (($real / $g.Count) / [math]::Max(0.001, (($g | Measure-Object oppPass -Average).Average)))
}

Write-Output ""
Write-Output "=== BY MODELLED SET SIZE (the E mechanism) ==="
$rows | Group-Object oppSet | Sort-Object { [int]$_.Name } | ForEach-Object {
    $g = $_.Group
    $real = ($g | Measure-Object oppPassed -Sum).Sum
    "  opp_set={0,-3} n={1,-5} model opp_pass={2:N3}  realized={3,6:P1}" -f `
        $_.Name, $g.Count, (($g | Measure-Object oppPass -Average).Average), ($real / $g.Count)
}

Write-Output ""
Write-Output "=== CALIBRATION VERDICT ==="
$hi = $rows | Where-Object { $_.oppPass -ge 0.70 }
$lo = $rows | Where-Object { $_.oppPass -lt 0.70 -and $_.oppPass -gt 0.001 }
if ($hi.Count -gt 0 -and $lo.Count -gt 0) {
    $hiReal = ($hi | Measure-Object oppPassed -Sum).Sum / $hi.Count
    $loReal = ($lo | Measure-Object oppPassed -Sum).Sum / $lo.Count
    "  high-confidence bucket (model>=0.70): model={0:N3} realized={1:P1}" -f (($hi | Measure-Object oppPass -Average).Average), $hiReal
    "  low-confidence  bucket (model<0.70):  model={0:N3} realized={1:P1}" -f (($lo | Measure-Object oppPass -Average).Average), $loReal
    if ($hiReal -lt $loReal) {
        "  ==> INVERTED: the model is MORE confident exactly where it is LESS right."
    } elseif ($hiReal -gt $loReal -and $hiReal -lt 0.9) {
        "  ==> monotone but OVER-confident at the top (model says 0.9+, reality lower)"
    } else {
        "  ==> monotone and roughly calibrated"
    }
}