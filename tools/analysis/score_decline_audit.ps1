param(
    [string]$Dir = "..\test_output\candsplit",
    [int]$Games = 120,
    [int]$Show = 25
)

# WHY did the argmax decline a higher-scoring life 61 times out of 387 commits?
#
# Two possible explanations, and they demand opposite fixes:
#   (a) JUSTIFIED - the higher-scoring life had a lower p_pass, so committing it
#       would have burned the turn. The argmax is right; there is no defect.
#   (b) DEFECT - the higher-scoring life had comparable p_pass, so the argmax
#       ranked a guaranteed placement below a marginal one. Score is not being
#       priced and that is a real, fixable hole.
#
# Prints both candidate rows side by side so the two cases separate visually.

$rows = New-Object System.Collections.ArrayList

for ($i = 1; $i -le $Games; $i++) {
    $n = "{0:D3}" -f $i
    $errPath = Join-Path $Dir "c_$n.err"
    if (-not (Test-Path $errPath)) { continue }

    $live = @{}
    $logPath = Join-Path $Dir "l_$n.txt"
    if (Test-Path $logPath) {
        foreach ($l in [System.IO.File]::ReadAllLines($logPath)) {
            if ($l -match '^t(\d+)\|SYSTEM\|live_result\|LIVE P1-WINS \| P1 set\[(?<s>[^\]]*)\] yell \d+/\d+flips score=(?<sc>\d+) . succ=(?<c>\d+)\(\+(?<d>\d+)\) \| P2 set\[(?<p2>[^\]]*)\] yell \d+/\d+flips score=(?<sc2>\d+)') {
                $t = [int]$Matches[1]
                if (-not $live.ContainsKey($t)) {
                    $live[$t] = @{
                        raw = $Matches.s; score = [int]$Matches.sc; place = [int]$Matches.d
                        opp = $Matches.p2; oppScore = [int]$Matches.sc2
                    }
                }
            }
        }
    }

    $seen = @{}
    foreach ($cl in [System.IO.File]::ReadAllLines($errPath)) {
        if ($cl -notmatch 'V8CAL t=(\d+) me=0 .*?set=(?<set>\d) .*? cands=(?<cands>.*)$') { continue }
        $t = [int]$Matches[1]
        if ($seen.ContainsKey($t)) { continue }
        $seen[$t] = 1
        if ([int]$Matches['set'] -ne 1) { continue }

        $cands = @()
        foreach ($e in [regex]::Matches($Matches['cands'], '\[(?<l>[^\|]*)\|pp=(?<pp>[\d.]+)\|pl=(?<pl>[\d.]+)\|v=(?<v>[\d.]+)\]')) {
            $lab = $e.Groups['l'].Value
            if ($lab -eq 'FOLD') { continue }
            $slash = $lab.IndexOf('/')
            if ($slash -lt 0) { continue }
            $cardNo = $lab.Substring(0, $slash)
            $colon = $cardNo.IndexOf(':')
            if ($colon -lt 0) { continue }
            $cands += @{
                lab   = $lab
                name  = ($cardNo.Substring(0, $colon) -replace '^PL!SP-', '' -replace '-L$', '' -replace '-SD2$', '')
                score = [int]$cardNo.Substring($colon + 1)
                need  = [int]$lab.Substring($slash + 1)
                pp    = [double]$e.Groups['pp'].Value
                pl    = [double]$e.Groups['pl'].Value
                v     = [double]$e.Groups['v'].Value
            }
        }
        if ($cands.Count -lt 2) { continue }

        $chosen = $null
        foreach ($c in $cands) { if ($null -eq $chosen -or $c.v -gt $chosen.v) { $chosen = $c } }
        $higher = $null
        foreach ($c in $cands) { if ($c.score -gt $chosen.score -and ($null -eq $higher -or $c.score -gt $higher.score)) { $higher = $c } }
        if ($null -eq $higher) { continue }

        $r = $null
        if ($live.ContainsKey($t)) { $r = $live[$t] }

        [void]$rows.Add([pscustomobject]@{
            game = $i; turn = $t
            chosenName = $chosen.name; chosenScore = $chosen.score; chosenPP = $chosen.pp; chosenPL = $chosen.pl
            highName = $higher.name; highScore = $higher.score; highPP = $higher.pp; highPL = $higher.pl
            dPP = ($chosen.pp - $higher.pp)
            oppSet = $(if ($r) { ($r.opp -ne '') } else { $null })
            realScore = $(if ($r) { $r.score } else { $null })
            oppScore = $(if ($r) { $r.oppScore } else { $null })
            placed = $(if ($r) { $r.place } else { $null })
        })
    }
}

Write-Output "commits that declined a higher-scoring life: $($rows.Count)"
Write-Output ""
Write-Output "=== SPLIT: was the decline justified by a lower p_pass? ==="
Write-Output ""
Write-Output "  Bucket A - higher-scoring life had p_pass within 0.10 of the chosen (=> DEFECT if it lost)"
$a = $rows | Where-Object { [math]::Abs($_.dPP) -le 0.10 }
Write-Output "  Bucket B - higher-scoring life was clearly worse on p_pass (=> JUSTIFIED)"
$b = $rows | Where-Object { $_.dPP -lt -0.10 }
Write-Output "  Bucket C - chosen was actually the HIGHER scorer on p_pass grounds (ambiguous)"
$c = $rows | Where-Object { $_.dPP -gt 0.10 }
"A: $($a.Count) | B: $($b.Count) | C: $($c.Count)"
Write-Output ""
foreach ($bucket in @(@{ n = 'A (DEFEAT SUSPECTS)'; d = $a }, @{ n = 'B (JUSTIFIED)'; d = $b }, @{ n = 'C (ambiguous)'; d = $c })) {
    if ($bucket.d.Count -eq 0) { continue }
    $g = $bucket.d
    $placed = 0
    foreach ($x in $g) { if ($null -ne $x.placed -and $x.placed -gt 0) { $placed++ } }
    "  {0,-20} n={1,-5} opp also set={2,-5}  P1 placed={3,-5} ({4,5:P0})" -f `
        $bucket.n, $g.Count,
        (($g | Where-Object { $_.oppSet -eq $true }).Count), $placed, ($placed / $g.Count)
}
Write-Output ""
Write-Output "=== BUCKET A DETAIL (the ones that would be real defects) ==="
$a | Sort-Object -Property dPP | Select-Object -First $Show | ForEach-Object {
    "  g$($_.game) t$($_.turn): took $($_.chosenName)(sc$($_.chosenScore)/pp=$($_.chosenPP)/pl=$($_.chosenPL)) over $($_.highName)(sc$($_.highScore)/pp=$($_.highPP)/pl=$($_.highPL))  dPP=$([math]::Round($_.dPP,3))  oppSet=$($_.oppSet)  placed=$($_.placed)"
}