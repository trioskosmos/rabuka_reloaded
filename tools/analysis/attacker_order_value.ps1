# Independent re-derivation of the attacker-order claim.
#
# The turn-order audit reported P(place | first attacker) = 0.5447 vs
# P(place | second attacker) = 0.5408 over ~11400 checks, "z ~ 0.6, not
# significant", and recommended closing the axis. That is the basis for
# declaring a whole direction dead, so it gets re-derived here from the raw
# trace rather than taken on trust.
#
# Method: rule 8.4.13 RE-ASSIGNS attacker order each turn from whoever SOLELY
# placed on the previous check. So order for check i is reconstructable from
# check i-1's placement delta, and check 1 (RPS) is not reconstructable.
#
# Order is ENDOGENOUS - assigned by the previous outcome - which is exactly the
# setting where a genuine first-attacker edge would surface, so this is the
# right test rather than a randomised one.
#
# NOTE ON THE TRACE FORMAT: the file's first line is a positional header
# (game,turn,phase,player,action_type,card_no,live_p1,...), but every DATA line
# uses trailing `key=value` pairs instead. Positional indexing parses
# "sel=[4+2+0]" as a number and dies; this reads the named fields.

$trace = "..\test_output\bot_arena_trace.csv"

# game -> ordered list of per-turn (p1succ, p2succ) snapshots at the live set
$games = @{}
$reader = [System.IO.StreamReader]::new((Resolve-Path $trace))
$null = $reader.ReadLine()
while ($null -ne ($line = $reader.ReadLine())) {
    $c = $line.IndexOf(',')
    if ($c -lt 0) { continue }
    $game = $line.Substring(0, $c)
    $rest = $line.Substring($c + 1)
    if ($rest -notmatch 'LiveCardSet') { continue }

    $p1s = $null; $p2s = $null
    $parts = $rest -split ','
    # The turn is the first field AFTER the game column. Testing each part for
    # `^\d+,` can never match because the split already removed every comma --
    # which is why an earlier version of this script found 0 games and looked
    # like a data problem rather than a parse bug.
    $turn = [int]$parts[0]
    foreach ($part in $parts) {
        if ($part -like 'succ_p1=*') { $p1s = [int]$part.Substring(8) }
        elseif ($part -like 'succ_p2=*') { $p2s = [int]$part.Substring(8) }
    }
    if ($null -eq $p1s -or $null -eq $p2s) { continue }

    if (-not $games.ContainsKey($game)) { $games[$game] = New-Object System.Collections.ArrayList }
    [void]$games[$game].Add([pscustomobject]@{ turn = $turn; p1 = $p1s; p2 = $p2s })
}
$reader.Close()

Write-Output "games in trace: $($games.Count)"

$faPlace = 0; $faChecks = 0
$saPlace = 0; $saChecks = 0
$bothChecks = 0; $neitherChecks = 0
$faSole = 0; $saSole = 0
$carried = 0   # checks whose order was carried over (tie / double fold)

foreach ($g in $games.Keys) {
    $checks = $games[$g] | Sort-Object turn
    # one snapshot per turn (the file has several rows per turn)
    $byTurn = @{}
    foreach ($c in $checks) { $byTurn[$c.turn] = $c }
    $ordered = @()
    foreach ($k in ($byTurn.Keys | Sort-Object)) { $ordered += $byTurn[$k] }
    if ($ordered.Count -lt 2) { continue }

    # placement delta for each check
    $deltas = @()
    for ($i = 0; $i -lt $ordered.Count; $i++) {
        $d1 = $ordered[$i].p1 - $(if ($i -eq 0) { 0 } else { $ordered[$i - 1].p1 })
        $d2 = $ordered[$i].p2 - $(if ($i -eq 0) { 0 } else { $ordered[$i - 1].p2 })
        $deltas += [pscustomobject]@{ d1 = $d1; d2 = $d2 }
    }

    # carry order forward across checks; only reassign on a sole placer
    $faIsP1 = $null
    for ($i = 1; $i -lt $deltas.Count; $i++) {
        $prev = $deltas[$i - 1]
        $cur = $deltas[$i]
        if ($prev.d1 -gt 0 -and $prev.d2 -eq 0) { $faIsP1 = $true; $faSole++ }
        elseif ($prev.d2 -gt 0 -and $prev.d1 -eq 0) { $faIsP1 = $false; $saSole++ }
        if ($null -eq $faIsP1) { $carried++; continue }   # order never established

        $p1Place = ($cur.d1 -gt 0)
        $p2Place = ($cur.d2 -gt 0)
        $faPlaced = $(if ($faIsP1) { $p1Place } else { $p2Place })
        $saPlaced = $(if ($faIsP1) { $p2Place } else { $p1Place })
        $faChecks++; $saChecks++
        if ($faPlaced) { $faPlace++ }
        if ($saPlaced) { $saPlace++ }
        if ($p1Place -and $p2Place) { $bothChecks++ }
        if (-not $p1Place -and -not $p2Place) { $neitherChecks++ }
    }
}

Write-Output "checks with a reconstructable order: $($faChecks + $saChecks)  (carried-over: $carried)"
$pf = $faPlace / [math]::Max(1, $faChecks)
$ps = $saPlace / [math]::Max(1, $saChecks)
Write-Output "  P(place | first attacker)  = {0:N2}%  ({1}/{2})" -f (100 * $pf), $faPlace, $faChecks
Write-Output "  P(place | second attacker) = {0:N2}%  ({1}/{2})" -f (100 * $ps), $saPlace, $saChecks
$d = $pf - $ps
Write-Output "  difference = {0:N2} pp" -f (100 * $d)
$se = [math]::Sqrt(($pf * (1 - $pf) / $faChecks) + ($ps * (1 - $ps) / $saChecks))
Write-Output "  pooled SE = {0:N4}  ->  z = {1:N2}" -f $se, ($d / $se)
Write-Output ""
Write-Output "  sole placer was FA: $faSole | SA: $saSole"
Write-Output "  both placed: $bothChecks | neither: $neitherChecks"