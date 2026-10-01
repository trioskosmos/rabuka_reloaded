param(
    [string]$LiveLog = "..\test_output\live_results.txt"
)

# CORRECTED live-phase analysis.
#
# The previous pass (and §13.3 of the log) read `yell N/Mflips` as the heart
# check. It is not: that field is the total number of BLADE-HEARTS revealed
# across the N yell cards, i.e. how many flips landed a blade heart. It has no
# relation to whether the live's heart requirement was met.
#
# The authoritative verdict is the engine's own annotation inside `set[...]`:
#   `set[Special Color:2pt]`      -> that live PASSED (name:score)
#   `set[Aspire:FAIL(c0 3/7,c3 1/2)]` -> that live FAILED, and the parenthesised
#                                      terms are per-colour filled/required
#
# So the real shortfall is recoverable, per colour, and that is what this reads.

$phases = New-Object System.Collections.ArrayList

foreach ($l in Get-Content $LiveLog) {
    if ($l -notmatch 'P1 set\[(?<s1>[^\]]*)\].*?P2 set\[(?<s2>[^\]]*)\]') { continue }

    $p1raw = $Matches.s1
    $p2raw = $Matches.s2

    # succ deltas are the placement truth
    $d1 = 0; $d2 = 0
    if ($l -match 'succ=\d+\(\+(?<a>\d+)\)') { $d1 = [int]$Matches.a }
    if ($l -match '\|\s*P2 set\[.*succ=\d+\(\+(?<b>\d+)\)') { $d2 = [int]$Matches.b }

    # A side "set" if its zone annotation is non-empty. PASS if no `FAIL(` in it.
    $p1set = ($p1raw -ne '')
    $p2set = ($p2raw -ne '')
    $p1pass = ($p1set -and ($p1raw -notmatch 'FAIL\('))
    $p2pass = ($p2set -and ($p2raw -notmatch 'FAIL\('))

    # Total heart requirement and total filled, from the FAIL annotation.
    # Passed lives have no annotation, so their need is not recoverable here;
    # those are counted separately and excluded from the shortfall histogram.
    function Get-Shortfall {
        param([string]$raw)
        if ($raw -eq '' -or $raw -notmatch 'FAIL\((?<terms>[^\)]*)\)') { return $null }
        $short = 0; $need = 0
        foreach ($t in ($Matches['terms'] -split ',')) {
            if ($t -match 'c\d+\s+(?<f>\d+)/(?<r>\d+)') {
                $f = [int]$Matches.f; $r = [int]$Matches.r
                $short += ($r - $f); $need += $r
            }
        }
        return @{ short = $short; need = $need }
    }
    $s1 = Get-Shortfall $p1raw
    $s2 = Get-Shortfall $p2raw

    [void]$phases.Add([pscustomobject]@{
        p1act = $(if ($p1set) { 'SET' } else { 'FOLD' })
        p2act = $(if ($p2set) { 'SET' } else { 'FOLD' })
        p1raw = $p1raw
        p2raw = $p2raw
        p1pass = $p1pass
        p2pass = $p2pass
        p1place = $d1
        p2place = $d2
        p1short = $(if ($s1) { $s1.short } else { $null })
        p1need = $(if ($s1) { $s1.need } else { $null })
        p2short = $(if ($s2) { $s2.short } else { $null })
        p2need = $(if ($s2) { $s2.need } else { $null })
    })
}

Write-Output "live phases parsed: $($phases.Count)"

Write-Output ""
Write-Output "=== SANITY: does the engine's FAIL annotation agree with who placed? ==="
$contested = $phases | Where-Object { $_.p1act -eq 'SET' -and $_.p2act -eq 'SET' }
"  SET/SET phases: $($contested.Count)"
"  P1 passed & P2 failed -> P1 should place: $(($contested | Where-Object { $_.p1pass -and -not $_.p2pass }).Count), actually placed $(($contested | Where-Object { $_.p1pass -and -not $_.p2pass -and $_.p1place -gt 0 }).Count)"
"  P2 passed & P1 failed -> P2 should place: $(($contested | Where-Object { $_.p2pass -and -not $_.p1pass }).Count), actually placed $(($contested | Where-Object { $_.p2pass -and -not $_.p1pass -and $_.p2place -gt 0 }).Count)"
"  both passed (score cmp): $(($contested | Where-Object { $_.p1pass -and $_.p2pass }).Count)"
"  neither passed (NO-CONTEST): $(($contested | Where-Object { -not $_.p1pass -and -not $_.p2pass }).Count)"

Write-Output ""
Write-Output "=== PHASE CLASSIFICATION (P1 = v9) ==="
$phases | Group-Object { "$($_.p1act)/$($_.p2act)" } | Sort-Object Name | ForEach-Object {
    $g = $_.Group
    $p1p = ($g | Measure-Object p1place -Sum).Sum
    $p2p = ($g | Measure-Object p2place -Sum).Sum
    "{0,-12} n={1,-5} ({2,5:P1})  P1 places={3,-5} P2 places={4,-5}  per phase={5:N2}" -f `
        $_.Name, $g.Count, ($g.Count / $phases.Count), $p1p, $p2p, (($p1p + $p2p) / $g.Count)
}

Write-Output ""
Write-Output "=== TRUE HEART SHORTFALL on v9's failed committed checks ==="
$failed = $phases | Where-Object { $_.p1act -eq 'SET' -and -not $_.p1pass -and $null -ne $_.p1short }
"  v9 committed-and-failed with a readable FAIL annotation: $($failed.Count)"
if ($failed.Count -gt 0) {
    $failed | Group-Object { $s = $_.p1short; if ($s -le 1) { 'short 1' } elseif ($s -le 2) { 'short 2' } elseif ($s -le 3) { 'short 3' } elseif ($s -le 5) { 'short 4-5' } else { 'short 6+' } } |
        Sort-Object Name | ForEach-Object { "    {0,-10} {1,4} ({2,5:P1})" -f $_.Name, $_.Count, ($_.Count / $failed.Count) }
    $near = ($failed | Where-Object { $_.p1short -le 2 }).Count
    "  within 2 hearts: $near / $($failed.Count) = $([math]::Round(100 * $near / $failed.Count, 1))%"
    "  avg requirement of the failed checks: $([math]::Round((($failed | Measure-Object p1need -Average).Average), 2)) hearts"
}

Write-Output ""
Write-Output "=== PER-CARD: committed, and did the ENGINE say it passed? ==="
$phases | Where-Object { $_.p1act -eq 'SET' } | ForEach-Object {
    $name = ($_.p1raw -split ':')[0]
    [pscustomobject]@{ name = $name; pass = $_.p1pass; place = ($_.p1place -gt 0) }
} | Group-Object name | Sort-Object Count -Descending | ForEach-Object {
    $g = $_.Group
    $p = ($g | Where-Object { $_.pass }).Count
    $pl = ($g | Where-Object { $_.place }).Count
    "{0,-26} committed={1,-5} engine_pass={2,-5} ({3,5:P0})  placed={4,-5} ({5,5:P0})" -f `
        $_.Name, $g.Count, $p, ($p / $g.Count), $pl, ($pl / $g.Count)
}