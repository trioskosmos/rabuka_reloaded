param(
    [string]$Dir = "..\test_output\cal3",
    [int]$Games = 150
)

# WHY does v9 fold 39.5% as second attacker but only 21.3% as first?
#
# Both hypotheses below were wrong on inspection, so this measures instead:
#   (a) "the second-attacker bar (5 hearts for a 1-card set) is too high"
#   (b) "v9 can read the opponent's card and shouldn't"  -- REFUTED: 8.2.2 keeps
#       the set face-down until the performance, so count-only is correct.
#
# What is left is a real asymmetry in the model's own arithmetic: as first
# attacker `pass_prob = clears * commit_rate` with commit_rate < 1, so an
# unobserved opponent is modelled as only *sometimes* committing. As second
# attacker commit_rate = 1.0 and the SAME `clears` applies, so the opponent is
# modelled as always committing and always clearing -- i.e. strictly harder to
# beat. The fold rate difference is then explained by `clears` being evaluated
# on a bar that only ever goes up.
#
# So: what does v9 fold away from, in each seat?

$rows = New-Object System.Collections.ArrayList

for ($i = 1; $i -le $Games; $i++) {
    $n = "{0:D3}" -f $i
    $errPath = Join-Path $Dir "c_$n.err"
    $logPath = Join-Path $Dir "l_$n.txt"
    if (-not (Test-Path $errPath) -or -not (Test-Path $logPath)) { continue }

    $live = @{}
    foreach ($l in [System.IO.File]::ReadAllLines($logPath)) {
        if ($l -match '^t(\d+)\|SYSTEM\|live_result\|.*?\| P1 set\[(?<s>[^\]]*)\] yell \d+/\d+flips score=\d+ . succ=\d+\(\+(?<d>\d+)\) \| P2 set\[(?<p2>[^\]]*)\]') {
            $t = [int]$Matches[1]
            if (-not $live.ContainsKey($t)) {
                $live[$t] = @{ p1set = ($Matches.s -ne ''); p2set = ($Matches.p2 -ne ''); p1place = [int]$Matches.d }
            }
        }
    }

    $last = $null; $lastT = $null
    foreach ($cl in [System.IO.File]::ReadAllLines($errPath)) {
        if ($cl -notmatch 'V8CAL t=(\d+) me=0 second=(?<sec>\w+) set=(?<set>\d) ppass=(?<pp>[\d.]+) pplace=(?<pl>[\d.]+) opp_pass=(?<op>[\-0-9.]+) opp_set=(?<os>\d+) mysucc=(?<ms>\d+) opsucc=(?<os2>\d+)') { continue }
        $t = [int]$Matches[1]
        $cm = [regex]::Match($cl, ' cands=(?<c>.*)$')
        $bestV = -999.0; $bestPP = 0.0; $bestPL = 0.0
        foreach ($e in [regex]::Matches($cm.Groups['c'].Value, '\[(?<l>[^\|]*)\|pp=(?<pp>[\d.]+)\|pl=(?<pl>[\d.]+)\|v=(?<v>[\-0-9.]+)\]')) {
            if ($e.Groups['l'].Value -ne 'FOLD') {
                $v = [double]$e.Groups['v'].Value
                if ($v -gt $bestV) { $bestV = $v; $bestPP = [double]$e.Groups['pp'].Value; $bestPL = [double]$e.Groups['pl'].Value }
            }
        }
        $last = @{
            second = ($Matches['sec'] -eq 'true')
            didSet = ([int]$Matches['set'] -eq 1)
            oppPass = [double]$Matches['op']
            oppSet = [int]$Matches['os']
            mysucc = [int]$Matches['ms']
            opsucc = [int]$Matches['os2']
            foldValue = $(if ($cm.Groups['c'].Value -match 'FOLD\|pp=[\d.]+\|pl=[\d.]+\|v=(?<fv>[\-0-9.]+)') { [double]$Matches['fv'] } else { $null })
            bestV = $bestV; bestPP = $bestPP; bestPL = $bestPL
        }
        $lastT = $t
    }
    if ($null -eq $last -or -not $live.ContainsKey($lastT)) { continue }
    $r = $live[$lastT]

    [void]$rows.Add([pscustomobject]@{
        game = $i
        seat = $(if ($last.second) { 'OBS' } else { 'UNOBS' })
        didSet = $last.didSet
        oppSet = $r.p2set
        oppPass = $last.oppPass
        oppSetSize = $last.oppSet
        mysucc = $last.mysucc
        bestPP = $last.bestPP
        bestPL = $last.bestPL
        bestV = $last.bestV
        foldV = $last.foldValue
        placed = ($r.p1place -gt 0)
    })
}

Write-Output "decisions: $($rows.Count)"
Write-Output ""
Write-Output "=== FOLD RATE BY SEAT ==="
foreach ($s in @('OBS', 'UNOBS')) {
    $g = $rows | Where-Object { $_.seat -eq $s }
    $f = ($g | Where-Object { -not $_.didSet }).Count
    "  {0,-6} n={1,-5} folded={2,-5} ({3,5:P1})" -f $s, $g.Count, $f, ($f / $g.Count)
}

Write-Output ""
Write-Output "=== MODELLED opp_pass BY SEAT (the suspect quantity) ==="
foreach ($s in @('UNOBS', 'OBS')) {
    $g = $rows | Where-Object { $_.seat -eq $s }
    "  {0,-6} mean opp_pass={1:N4}  opp_set_size mean={2:N2}" -f `
        $s, (($g | Measure-Object oppPass -Average).Average), (($g | Measure-Object oppSetSize -Average).Average)
}

Write-Output ""
Write-Output "=== THE FOLD MARGIN: best commit value vs the FOLD candidate's value ==="
foreach ($s in @('UNOBS', 'OBS')) {
    $g = $rows | Where-Object { $_.seat -eq $s -and -not $_.didSet }
    if ($g.Count -eq 0) { continue }
    $avgBest = ($g | Measure-Object bestPL -Average).Average
    $avgPP = ($g | Measure-Object bestPP -Average).Average
    "  {0,-6} folds n={1,-5} | best commit had p_pass={2:N3} p_place={3:N3}" -f $s, $g.Count, $avgPP, $avgBest
}
foreach ($s in @('UNOBS', 'OBS')) {
    $g = $rows | Where-Object { $_.seat -eq $s -and $_.didSet }
    if ($g.Count -eq 0) { continue }
    "  {0,-6} commits n={1,-4} | p_pass={2:N3} p_place={3:N3}" -f `
        $s, $g.Count, (($g | Measure-Object bestPP -Average).Average), (($g | Measure-Object bestPL -Average).Average)
}

Write-Output ""
Write-Output "=== DID THE FOLD ACTUALLY HELP? (folded, opp set, did WE place anyway?) ==="
$bothFolded = $rows | Where-Object { -not $_.didSet -and -not $_.oppSet }
"  we folded AND they folded: $($bothFolded.Count) phases -> both got nothing"
$conceded = $rows | Where-Object { -not $_.didSet -and $_.oppSet }
"  we folded, they set:      $($conceded.Count) phases -> conceded a placement"