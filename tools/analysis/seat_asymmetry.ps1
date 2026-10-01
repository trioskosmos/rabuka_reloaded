param(
    [string]$Dir = "..\test_output\candsplit",
    [int]$Games = 120
)

# H-B: does v9 concede placements specifically when it CANNOT see the opponent?
#
# The OppModel gives the second attacker an EXACT `pass_prob` (commit_rate = 1.0,
# so pass_prob = clears from the visible zone) and the first attacker a FORECAST
# (pass_prob = clears * commit_rate, commit_rate < 1). So the first-attacker seat
# is where a bad fold decision should concentrate: v9 is guessing whether v7 will
# commit, guessing wrong costs the placement outright under 8.4.3.2.
#
# If folds cluster on the unobserved seat, the lever is the forecast's
# `commit_rate` constant, not the live-set argmax. If they are spread evenly, the
# fold decision is seat-independent and the lever is elsewhere.

$rows = New-Object System.Collections.ArrayList

for ($i = 1; $i -le $Games; $i++) {
    $n = "{0:D3}" -f $i
    $errPath = Join-Path $Dir "c_$n.err"
    $logPath = Join-Path $Dir "l_$n.txt"
    if (-not (Test-Path $errPath) -or -not (Test-Path $logPath)) { continue }

    # per-turn: which seat was P1 on for the live set, and what happened
    $live = @{}
    foreach ($l in [System.IO.File]::ReadAllLines($logPath)) {
        if ($l -match '^t(\d+)\|SYSTEM\|live_result\|LIVE P1-WINS \| P1 set\[(?<s>[^\]]*)\] yell \d+/\d+flips score=\d+ . succ=\d+\(\+(?<d>\d+)\) \| P2 set\[(?<p2>[^\]]*)\]') {
            $t = [int]$Matches[1]
            if (-not $live.ContainsKey($t)) {
                $live[$t] = @{ p1set = ($Matches.s -ne ''); p2set = ($Matches.p2 -ne ''); p1place = [int]$Matches.d }
            }
        }
    }

    $seen = @{}
    foreach ($cl in [System.IO.File]::ReadAllLines($errPath)) {
        if ($cl -notmatch 'V8CAL t=(\d+) me=0 second=(?<sec>\w+) set=(?<set>\d) ') { continue }
        $t = [int]$Matches[1]
        $second = ($Matches['sec'] -eq 'true')
        if ($seen.ContainsKey($t)) { continue }
        if (-not $live.ContainsKey($t)) { continue }
        $seen[$t] = 1
        $r = $live[$t]

        [void]$rows.Add([pscustomobject]@{
            game = $i; turn = $t
            second = $second
            seat = $(if ($second) { 'OBSERVED (2nd)' } else { 'UNOBSERVED (1st)' })
            didSet = ([int]$Matches['set'] -eq 1)
            oppSet = $r.p2set
            placed = ($r.p1place -gt 0)
        })
    }
}

Write-Output "decisions: $($rows.Count)"
Write-Output ""
Write-Output "=== FOLD RATE BY SEAT ==="
foreach ($seat in @('OBSERVED (2nd)', 'UNOBSERVED (1st)')) {
    $g = $rows | Where-Object { $_.seat -eq $seat }
    if ($g.Count -eq 0) { continue }
    $folds = ($g | Where-Object { -not $_.didSet }).Count
    "{0,-18} n={1,-5} folded={2,-5} ({3,5:P1})" -f $seat, $g.Count, $folds, ($folds / $g.Count)
}

Write-Output ""
Write-Output "=== CONCEDED PLACEMENTS: we folded, they set ==="
$concede = $rows | Where-Object { (-not $_.didSet) -and $_.oppSet }
Write-Output "  total conceded: $($concede.Count)"
$concede | Group-Object seat | Sort-Object Count -Descending | ForEach-Object {
    "    {0,-18} {1,4} ({2,5:P1} of all folds-while-they-set)" -f $_.Name, $_.Count, ($_.Count / $concede.Count)
}

Write-Output ""
Write-Output "=== PLACEMENT RATE BY SEAT AND ACTION ==="
$rows | Group-Object { "$($_.seat) / $(if($_.didSet){'SET'}else{'FOLD'})" } | Sort-Object Name | ForEach-Object {
    $g = $_.Group
    $p = ($g | Where-Object { $_.placed }).Count
    "{0,-32} n={1,-5} placed={2,-5} ({3,5:P0})" -f $_.Name, $g.Count, $p, ($p / $g.Count)
}