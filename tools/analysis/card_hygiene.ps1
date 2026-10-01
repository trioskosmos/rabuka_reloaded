param(
    [string]$Dir = "..\test_output\cal3",
    [int]$Games = 150
)

# CARD HYGIENE: are unwanted cards actually cycled out, in BOTH phases?
#
# Two separate questions the audit never asked:
#   (A) LIVE PHASE - `junk_keep_value` prices every non-live hand card and the
#       spare set slots are filled with the cheapest. The source records the
#       metric it tuned against ("0.07 junk cards per set against v7's 1.08")
#       and nothing has re-measured it since. If it is still ~0.07 the bot is
#       holding its hand and the draw-1-per-burn benefit is unclaimed.
#   (B) MULLIGAN - dumps by descending printed cost, never reading a life's
#       requirement. Does it actually remove the cards that are dead?
#
# This measures both from the engine's own per-turn hand composition.

$rows = New-Object System.Collections.ArrayList

for ($i = 1; $i -le $Games; $i++) {
    $n = "{0:D3}" -f $i
    $logPath = Join-Path $Dir "l_$n.txt"
    if (-not (Test-Path $logPath)) { continue }

    $prevHand = $null
    foreach ($l in [System.IO.File]::ReadAllLines($logPath)) {
        # The RULE LOG records hand changes; live zones and success zones.
        if ($l -match '\|\|hand\|(?<who>P\d)\|(?<rest>.*)$') {
            $who = $Matches.who
            $rest = $Matches.rest
            # hand size is printed as a count when known
            if ($rest -match 'size=(?<sz>\d+)') {
                $sz = [int]$Matches.sz
                if ($null -eq $prevHand) { $prevHand = @{} }
                $prevHand[$who] = $sz
            }
        }
    }
    if ($null -eq $prevHand) { continue }
    [void]$rows.Add([pscustomobject]@{
        game = $i
        p1hand = $(if ($prevHand.ContainsKey('P1')) { $prevHand.P1 } else { $null })
        p2hand = $(if ($prevHand.ContainsKey('P2')) { $prevHand.P2 } else { $null })
    })
}

Write-Output "games with a parsable hand trace: $($rows.Count)"
if ($rows.Count -eq 0) {
    Write-Output ""
    Write-Output "The per-game log does not carry a hand-size field, so hand"
    Write-Output "composition has to come from V8_CAL (which prints the candidate"
    Write-Output "table, and therefore every non-life in hand) rather than the log."
}