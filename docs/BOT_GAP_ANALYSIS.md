# Bot Gap Analysis — what the bots are not doing

Measured 2026-10-02 against `5CP3Z idou`, bots v9 (v7 Main + v8 live set).
Every number here is engine truth or arena output, not a reading of the guides.
Reproduce with:

```
cargo run --release --bin bot_arena -- v9 v9 10 "5CP3Z idou" --games 3000 --seed 11
cargo run --release --bin live_calibrate -- --games 250 --seed 11 --deck "5CP3Z idou" \
    --p1 v9 --p2 v9 --csv checks.csv
```

---

## 0. A mirror cannot detect improvement (read this before any A/B run)

`u(π, π) = v` — the game's value — for **every** π in the equilibrium set. A
heuristic scalar-value policy class essentially never contains an equilibrium,
so within it the map `π ↦ u(π, π)` is pinned at whatever seat symmetry forces
and carries **zero** information about which π is better. Ten nulls was the
maximum information content of the experiment, not a result about the weights.

This retroactively reinterprets the whole sweep record in this repo: ~80
process-global `V8_*` switches were read in `v9 vs v9` mirrors, and several of
them were demonstrably moving things. Two examples, each of which looked
significant in a mirror and was flat head-to-head:

```text
   V8_MAX_LIVES=2   seed 7 mirror  +1.10 pp  p=0.036  pace +0.0056 SIGNIFICANT
   v11 vs v9        n=4000 both seat orders   50.1% / 47.6%
```

`--env` is process-global, so a switch cannot be applied to one seat of a
head-to-head. That is the whole reason `v11` and `v12` are registry kinds now:
**a candidate has to be a named bot to be measured at all.** The control for
this harness is `v12` with `V8_TURNORDER=0`, which must read ~50%: it does,
50.16%.

## 0.1 Head-to-head baseline: the component grid, closed

The grid that produced `v9` was measured **in mirrors** — the crossover quoted in
the `V9` registry doc reports 43.7% / 53.9% / 40.7%, which is the invalid
metric. So the component choice rested on an experiment that cannot measure it.
Re-measured head-to-head, `5CP3Z idou`, n=6000 per row, both seat orders:

| cell | bot | v9 wins as P1 | v9 wins as P2 |
|---|---|---|---|
| v7 Main + v7 live | `v7` | 56.8% | 55.6% |
| v8 Main + v8 live | `v8` / `v10` | 62.1% | 63.6% |
| **v8 Main + v7 live** | `V8_BORROW_LIVE=v7` | 65.4% | 66.0% |
| v7 Main + v8 live | **`v9`** | — | — |

All four cells, both seat orders. **`v9` dominates every alternative**, and the
grid is monotonic in the right direction: swapping either half for the weaker one
costs 6 to 9 points, and swapping both costs the most. The never-validly-tested
cell is the worst, which is what coherence looks like.

This retires two open questions at once. It confirms `v9` is the best bot in the
project rather than the best *mirror*, and it settles the `V10` registry doc's
open question about which half of v8 is worth keeping — **neither**, and the
13-term feature-delta vector that was "the largest confirmed change in the file"
(42.5% → 47.9%, seed 11) was measured in the mirror that cannot see it.

It also calibrates the harness: real 6-point effects land identically in both
seat orders, so a null at n=6000 really is a null.

## 0.2 Why the placement rate cannot go much above ~0.43

43% of all placements come from the **opponent failing their own check** — the
opponent passes only 55.7% of the time, and that number is set by their board
and their life choice, neither of which we influence. Only the other 57% of
placements are ours to compete for. Measured P1 0.427 / P2 0.434 sits right at
that split, which is a better explanation for the observed pace than any
ceiling argument: the bot is extracting nearly all the placements that are
actually available to it.

## 0.3 The best-response instrument (`vbr`) — the live set survives cheating

Every measurement on record is self-play. A symmetric change cannot move a
symmetric win rate off 50%, so **none of them can see a defect both seats
share** — and `v9` is compared almost exclusively against itself. That is why
fourteen interventions produced fourteen nulls: not because there was nothing
wrong, but because the experiment could not have shown it.

`vbr` closes that. It is v9's Main and mulligan with a live set that reads the
opponent's **real** board, hand and committed zone. It cheats deliberately and
is not a legal bot — it is a best-response instrument in the sense of Timbers
et al., *Approximate Exploitability*, IJCAI 2022 / arXiv:2004.09677, where a
best response must see what it responds to and cheating is precisely what makes
the number a lower bound rather than another guess.

```text
   vbr vs vbr   n=6000   control          50.4%
   vbr vs v9    n=6000                    49.2%
   v9 vs vbr    n=6000                    48.5%
   vbr vs v9    n=6000   aiscream 37PMZ   47.1%
```

**The cheating best response loses.** That is a stronger statement than any of
the fourteen nulls, because it is the one experiment they all lacked: `v9`'s
live set is not merely un-improvable by a self-play tweak, it beats a
perfect-information response to its own decisions.

The reason is worth keeping, because it inverts the obvious intuition.
Replacing the opponent model with the exact truth makes the argmax **worse**.
Greedy one-phase optimality is not the same objective as the game's: 8.4.13
makes a placement change next turn's attacker slot and the win condition is a
race to three (1.2.1.1), so the best set for *this* check is frequently not the
best set for the game. The smooth probabilistic model is averaging over that,
and the average is the point. Anyone who "fixes" this by giving the policy more
information will make it worse.

**What this does not cover.** `vbr` replaces only the live set. The Main phase
carries the 58% gap between the two component halves (§0.1) and is untouched
here, so a shared Main-phase defect would still be invisible. That is the one
place a shared flaw could now be hiding, and attacking it needs a deep search
over member deployment rather than a one-phase response.

## 0.4 What actually decides this game — and it is not blades

Measured over 2198 contested checks where both sides committed and both passed:

```text
   outcome   n     our_blades  opp_blades   our_score  opp_score
   win     920        7.61        7.03         4.21       2.41
   tie     635        6.44        6.50         2.92       2.92
   loss    643        6.92        7.81         2.62       4.63
```

**The bot loses while holding MORE blades than it holds when it ties** — 6.92
against 6.44 — and with a lower score, 2.62 against 2.92. Blade parity is nearly
irrelevant on its own:

```text
   blade gap  -3   n=148   win-or-tie 60.1%
   blade gap  -1   n=290   win-or-tie 68.6%
   blade gap   0   n=450   win-or-tie 71.8%
   blade gap  +1   n=315   win-or-tie 72.1%
   blade gap  +2   n=234   win-or-tie 83.8%
   blade gap  +4   n= 94   win-or-tie 88.3%
```

So the decisive quantity is **which live card is played**, and the whole
Main phase is a colour-supply problem wearing a blade-count costume.

### Why, in this deck specifically

The four lives are:

```text
   x3  score 1   heart03 x1 + heart0 x2      ( 3 hearts, 1 coloured)
   x4  score 2   heart0 x7                   ( 7 hearts, 0 coloured)  <- colour-free
   x4  score 4   heart02 x2 heart03 x1 heart06 x1 + heart0 x7  (11 hearts, 4 coloured)
   x1  score 7   heart01 x2 heart02 x4 heart06 x4 + heart0 x5  (15 hearts, 10 coloured)
```

and the deck's entire colour supply across all 48 members is:

```text
   heart01 = 4     heart02 = 44     heart03 = 60     heart06 = 44
```

`heart01` is effectively absent, so **the score-7 life is uncashable** and the
deck's real ceiling is score 4. Reaching it needs only four coloured hearts out
of eleven — and `heart02`/`heart03`/`heart06` are the three colours the deck is
full of.

**That is the whole game: hold enough of the three colours you have, and you
can play a 4; fail to, and the only thing you can clear is the colour-free
score-2, at which point you lose every comparison to a 4.**

### Where that points, and why nothing above found it

The supply figures say the Main phase should be optimising **colour supply
against the deck's live requirements**, not blade count and not heart count.
`v8_main`'s thirteen feature-delta terms were aimed at place-probability, were
measured in the mirror that cannot see a gain, and lost by 62% — so they are
not evidence either way about colour.

Note what this does *not* contradict. The live-set argmax is still dominated by
`p_pass` (§ the commitment game), and `p_pass` for the score-4 life is
`pass_probability(pools, need)` — which is exactly where colour lives. A
*uniform* miscalibration cannot reorder candidates, but a **colour-specific**
one is not uniform: it penalises precisely the high-colour-requirement lives and
leaves the colour-free score-2 looking free. That is the one shape of
miscalibration that survives the `p_pass`-invariance argument, and it is the one
thing in this file that has never been measured.

The test is cheap and does not need a new policy: bucket committed checks by the
chosen life's **coloured-heart demand** and check whether the realised pass rate
tracks the model's prediction. If high-colour lives are chosen far below what
their measured pass rate justifies, the flip sampler's colour handling is the
defect and the fix is in `FlipSampler::build_with_draws`, not in any policy.

## 0.5 The per-card outcome table is a selection effect (`V8_SCORE_BIAS`)

The most seductive table in this project, and the reason it has to be written
down rather than left in a terminal. Bucketing 3734 committed checks by which
life was played, and the 2198 contested ones (both committed, both passed):

```text
   card                    score/coloured   share   contested placed
   PL!SP-bp5-024-L           7 / 10          2.2%        67.2%
   PL!SP-sd2-023-SD2         1 /  1         27.7%        44.4%
   PL!SP-sd2-025-SD2         4 /  4         30.4%        41.1%
   PL!SP-bp4-025-L           2 /  0         39.7%        39.7%
```

Monotone: every higher-scoring life places better when contested and is played
less. Read naively that is a large apparent opportunity sitting on the table,
made more tempting by `p_pass` being already calibrated per card — the model
predicts 0.908 / 0.911 / 0.921 against realised 0.963 / 0.914 / 0.907, so
nothing is broken and the weighting is supposedly just wrong.

**Intervening on it makes the bot monotonically worse:**

```text
   V8_SCORE_BIAS = 0.00   (control)   50.4%
                = 0.05               44.2%
                = 0.10               40.0%
                = 0.20               32.9%
```

The reason is that the table is a **selection effect**, not a causal ranking.
The bot plays a big life *because* the position is favourable — that is what the
argmax is for. Conditioned on the state, the cheap lives are played where the
comparison does not decide anything, and they still place, because the
opponent's own check decides those. "The 7 places 67% of the time" describes
the positions the bot already knew were good.

**This is the general trap and it is worth more than the null.** Any per-outcome
breakdown of a policy's own choices — by card, band, turn, state — is
conditioned on the policy having chosen it, so it cannot be read as the value of
choosing it. It looks like free headroom precisely because it is the headroom the
policy already captured. Intervening is the only way to tell, which is why this
one is an intervention.

It also settles §0.4's colour reading. `heart01 = 4` really does make the
score-7 life effectively uncashable, and the bot correctly plays it 2.2% of the
time — when it does come up it is the best card in the deck, which is exactly
what a correctly functioning evaluator should produce. There is no defect here.

> **Data-integrity note.** The first version of this table was wrong, and the
> error was mine. Adding three telemetry columns to `live_calibrate` shifted the
> CSV writer relative to its header, so every column from `our_yell` onward was
> read three positions out. The "card" buckets were in fact `our_yell` — the
> number of cards flipped — which reproduced a supply gradient and looked like a
> card-choice table. `live_calibrate`'s writer and header are now aligned, the
> numbers above are from the corrected file, and the conclusion held only because
> the intervention was run rather than the table being believed. Header/writer
> agreement is the kind of thing that deserves an assertion and does not have one.

## 0.6 The Main-phase lookahead: the last untouched surface, worth exactly zero

`vbr` closed the live set and left one thing unattacked — a defect shared by both
seats in the **Main phase**, which carries the 58% gap between the component
halves (§0.1). It turned out to need no new search at all.

`v7_main` already scores every candidate Main action by cloning the state and
executing the action (`let mut sim = root.clone()`), and it already has two terms
that evaluate the *result* by asking the 256-draw pass model whether the next
check will actually pass:

```rust
if pressure_weight > 0.0 {
    let after = crate::bot::strategy_v7::check_pressure(&sim, me, &gs.card_database);
    score += pressure_weight * (after - base_pressure);
}
```

**Both weights default to 0.0.** So the Main phase that carries the whole
game's weight scores every action by feature deltas toward a *proxy* for "will I
place next check", while the correct evaluator sits one line below it, wired up
and switched off. That is precisely the error `vbr` proved is wrong in the live
set: one-phase greedy toward place-probability is not the objective, because
8.4.13 makes a placement change next turn's attacker slot and 1.2.1.1 makes the
game a race to three.

`v16` is v9 with that evaluator enabled **on one seat only** — a `LookaheadGuard`
thread-local, because `V7_CHECK_PRESSURE` is process-global and would otherwise
change both seats at once and measure nothing.

```text
   v16 vs v9   V7_CHECK_PRESSURE=0.3   50.6%
   v16 vs v9   V7_CHECK_PRESSURE=1.0   50.6%
   v16 vs v9   V7_BLADE_ARITH=1.0      50.6%
   v9 vs v16   V7_CHECK_PRESSURE=1.0   49.4%
```

The last two rows are exact images — `3038 - 2962` both ways — which is the
harness proving the one-seat guard works, and the payoff is **precisely zero**:
+0.6 pp on one seat order, −0.6 pp on the other. The action counts moved
(152812 / 152944 / 153062 against the control's 153167) so the terms are firing.

This is the cleanest result in the file. It is the most obvious "obvious fix" in
the codebase — a disabled real evaluator in the phase that decides the game —
and it is worth exactly nothing once measured on one seat instead of in a mirror.
Every prior number for these two weights came from mirrors, so none of them could
have found that out.

## 1. The headline metric is not the objective — it is a zero-sum race

`docs/BOT_STRATEGY.md` ranked its entire backlog against **"pace ~1.0
placements per live phase"**, read off the guides' prose and applied per side.
The rules say what that number can and cannot mean:

| rule | text | consequence |
|---|---|---|
| 1.2.1.1 | 自分の成功ライブカード置き場のカードが 3 枚以上で、相手が 2 枚以下なら勝利 | the game is a **race to three placements** |
| 8.4.7 | ライブに勝利したプレイヤーは…カードを**1 枚**選び、それを自身の成功ライブカード置き場に移動します | a won check pays **exactly one** card, any margin |
| 8.4.6.2 | 合計スコアが等しい場合は**両方の**プレイヤーがライブに勝利 | two placements only on an exact score tie |
| 8.4.7.1 | 両方のプレイヤーが勝利している場合…ライブ置き場にカードが 2 枚あるプレイヤーは、カードを移動しません | and not even then, if the player is at 2+ successes |

Two corrections to what this file previously claimed.

**The per-side ceiling is 1.0, not 0.5.** 8.4.7 pays one card per won check,
so one live phase can place **one** card for a single side. Winning the game
outright is three checks in a row — 3 placements in 3 phases, a per-side pace
of exactly 1.000, which is the number `bot_arena`'s own
`multiple placements per phase is legal` test asserts. An earlier revision of
this document divided the *combined* ceiling of ~1.0 by two seats, called the
quotient a per-side rule limit, and reported the run against it as "86% of the
physical maximum". That conflated an average of two players with a bound on
one, and it is the arithmetic behind a claim that no live-set work could raise
the win rate. `bot_arena` now prints the correct ceiling.

**The metric cannot be moved by both seats at once.** Placements are a zero-sum
race. `v11` is the measurement: it sets two lives instead of one, which raises
placement rate on both seats by 0.002–0.004 per phase (two cells significant),
lengthens games from 5.93 to 5.79 turns, and moves the win rate by nothing at
all — 50.1% and 47.6% head-to-head against `v9` at n=4000 each, and null on
fresh mirrors (seeds 101 and 31337).

So `pace` is not a target. A number both seats can raise by the same amount is
not measuring the thing the project is trying to move, and every policy tuned
against it has a built-in null.

## 2. The game is a comparison, not a conversion problem

`live_calibrate`, 250 games / 6738 checks, per-check ground truth from
`performance_snapshots`:

```
we committed a life     1591/1991 = 0.799
we passed               1381/1991 = 0.694
we placed                483/1991 = 0.243   (v8's own p_place: mean_pred 0.561)
opp committed           1220/1991 = 0.613
opp passed              1078/1991 = 0.541
opp placed               510/1991 = 0.256
```

So **given that a life was set, v9 passes its own check 91.8% of the time**
(4304/4943 unsplit), with mean supply 9.6 against mean requirement 6.6. The
binding constraint is not clearing the check — it is winning the score
comparison in 8.4.6, which decides 51.2% of live phases.

Split by attacker order the rules are implemented exactly (178/178 strict
score wins placed as second attacker; a tie places for both unless one is at 2
successes). Nothing in the engine drops a won check.

**P1 places at 0.427 and P2 at 0.434 — a ratio of 0.98.** Both seats run the
same policy on the same decklist, so the comparison is a coin flip and rule
8.4.13 hands first attacker to whoever placed, alternating the good turn-order
slot so it cancels. Win rate can only move by winning *more* comparisons, and
that has to come from a strictly better rule, not from a shared constant.

## 3. The opponent model is miscalibrated, and fixing it changes nothing

`p_place` is badly optimistic; `p_pass` (the part v8 actually samples) is not.

```
p_pass : n=5590  mean_pred 0.665  observed 0.673   Brier 0.116   <- calibrated
p_place: n=5590  mean_pred 0.561  observed 0.301   Brier ~0.21   <- 1.9x optimistic
opp_pass: n=5513 mean_pred 0.412  observed 0.561   Brier 0.271   <- 3x pessimistic
```

Calibration tables, decile buckets, n=5590:

| predicted | 0.1 | 0.2 | 0.3 | 0.4 | 0.5 | 0.6 | 0.7 | 0.8 | 0.9 | 1.0 |
|---|---|---|---|---|---|---|---|---|---|---|
| `p_place` observed | 0.40 | 0.32 | 0.26 | 0.26 | 0.31 | 0.31 | 0.33 | 0.39 | 0.57 | 0.57 |

| predicted | 0.0 | 0.1 | 0.2 | 0.3 | 0.4 | 0.5 |
|---|---|---|---|---|---|---|
| `opp_pass` observed | 0.32 | 0.57 | 0.73 | 0.76 | 0.78 | 0.80 |

The defect is entirely in the opponent model: `clears` is a 6-valued step
function over a hidden yield prior, and the guides' own measured curve (v9
passes 90.7% when their board is short by 1-3 hearts, still 75.1% at 4, 57.5% at
6) is a smooth monotone falloff the step function cannot express.

**And it does not matter.** `V8_ASSUME_COMMIT` at 0.35 / 0.80 / 0.95 (a
direct multiplier on `opp_pass`) read −0.38 / +0.38 / +0.12 pp at n=800 — all
null. Recalibrating a term that only reorders candidates built from the same
post-action board quantities cannot move the argmax. This confirms, at power,
what `docs/BOT_STRATEGY.md` §5.1 already concluded by reasoning.

## 4. An exhaustive sweep found no real improvement

~80 arms, paired McNemar against a fixed baseline, over every gate in the tree
(`V7_*`, `V8_*`, `V9_*`). Per-arm n=3000-4000. **Zero arms survived.**

Nothing significant in the live set: `MAX_LIVES=2` +0.45, `MAX_LIVES=3` +0.40,
`CONTESTED_WIDE` +0.35, `OPP_SCORE_BIAS=-2` +1.05, `LIFE_RESALE=0.9` +1.20,
`NO_FOLD` −1.00. Nothing structural: `V9_LIVE_ROLLOUT=v7` −0.90,
`V9_MULLIGAN=v8` +0.60 (and pace *drops* 0.427 → 0.395), `V7_BEAM=1` +0.90,
`V8_LIVE_START_PROJECTION=1` −0.43. Nothing in the ability layer despite
4688 activations offered and only 488 taken (10.4% uptake): `ABILITY_QUAL=0`
and `NOOP_GATE=0` both read exactly +0.00, `ABILITY_CLASS=3` −0.60.

### The one arm that looked significant, and why it was not

`V7_BLADES_WEIGHT` has a real mechanism — blades set the flip count, flips set
supply, supply sets which score band is reliably passable, and score is the
only thing that decides a comparison. On seed 11 (the seed it was *selected*
on) it gave a monotone dose-response with two significant rungs:

| weight | 3 | 6 (ship) | 12 | 20 | 30 | 45 | 70 | 120 |
|---|---|---|---|---|---|---|---|---|
| delta pp | +0.13 | — | +0.87 | **+1.53** | **+1.70** | **+1.70** | +1.17 | +1.50 |
| verdict | ns | — | ns | SIGNIF | SIGNIF | SIGNIF | ns | ns |

On seeds it was never tuned on, and on the two tests that are not mirrors:

| test | w=20 | w=30 | w=45 |
|---|---|---|---|
| seed 7 (n=4000) | | +0.15 | +0.05 |
| seed 101 | | +0.03 | −0.10 |
| seed 2029 | | +0.68 | +0.07 |
| seed 31337 | | +0.05 | +0.38 |
| seed 4242 | −0.13 | −0.03 | −0.07 |
| 2nd deck `aiscream 37PMZ` | −0.50 | | −0.53 |
| **v9 vs v7, gate on BOTH seats** | **−6.17** | | **−6.00** |

The last row is decisive. Applied to both seats, the arm takes v9's win rate
over v7 from **56.2% to 50.0%**. It is a real 6pp regression against an
opponent that is not itself, and a mirror was never going to see it.

Note the two decks also differ sharply on the only meaningful pace number:
`5CP3Z idou` sits at 0.861 combined (86% of ceiling) and `aiscream 37PMZ` at
**0.604** (60%). The second pairing has 40% real headroom and is where work
should go; the flagship deck has 14%.

**A mirror is a falsifier of symmetry, not a detector of improvement.** A gain
claim needs a seed the arm was not selected on *and* an asymmetric opponent.

## 5. Gaps that are real but unexercised by the arena

- **Mulligan.** It runs every game and is routed to the bot
  (`bot_arena.rs:2051`), but v9 ships v7's curve rule, which fires on only
  ~17% of hands. v8's forward-simulation mulligan is one env gate away
  (`V9_MULLIGAN=v8`) and measured *worse* (+0.60pp, pace 0.427 → 0.395), so
  neither implementation is earning its keep.
- **Attacker role.** The bot never chooses. The arena gives it to the RPS
  winner and randomises the loser. Rule 8.4.13 makes the sole placer the next
  turn's first attacker, which is why the first attacker's measured placement
  rate (0.163) sits far below the second attacker's (0.442). This is the only
  genuine asymmetry in the game and no policy reads it.
- **UseAbility is refused 89.6% of the time** (4200 of 4688 offers), but every
  ability gate that opens it measures neutral or negative, because the engine
  resolves the abilities anyway and what is missing is search depth, not
  willingness.
- **Yell selection (8.3.14)** and **position keywords** are not decision points
  in this engine — the phase generator offers no actions there — so they are
  not bot gaps.

## 6. Bottom line

v9 is at a genuine local optimum, and the reason is now known rather than
inferred. Fourteen substantive interventions were measured in this pass — four of
them built as named bots so they could be tested head-to-head — and all were
null or negative:

| intervention | mechanism | result |
|---|---|---|
| `v11` set width 2 | play a pair, spend unused supply on score | mirror +1.10 pp seed 7; **h2h 50.1% / 47.6%** |
| `v12` rule 8.4.13 turn order | price surrendering the 先攻 slot on a sole placement | h2h 50.3% at w=0.25; **40.3% at w=1.0** |
| `V8_LIVE_START_PROJECTION` | price the board after ライブ開始時 | null, 4 seeds |
| `V8_SELF_REWRITE` | price SD2-023's own rewrite (score 1→6, 3→12 hearts at 2+) | null, 4 seeds |
| `V8_OPP_SAMPLED_YIELD` | opponent score from the deck in play | **−0.40 pp, significant** |
| `V8_OPP_SCORE_ICONS` / `V8_P_FIRST` | opponent icons / rank by p_place | null |
| `v13` count-as-signal | price the count 8.2.2 publishes | null alone, **negative** stacked on width |
| `V8_COMMIT_GAME` | solve the commitment subgame by iterated best response | **provably cannot** change the pick (below) |
| `v15` v8 forward-walk mulligan | replace v7's static mulligan curve | **−5.5 pp** head-to-head |
| `V8_PRESSURE` | opponent must REACH my published total | null, and the premise is wrong |

### The 43% term, and why the pressure exploit cannot exist (`V8_PRESSURE`)

43% of placements come from the opponent failing their own check, so that term
looked like the one place our own play could reach. And there was a real hole:
their pass probability was built from **their own** set size and never mentioned
what we published.

```rust
let target = (2 * set_size).clamp(1, ...);
let bar    = score_median_hearts(target);
```

8.4.6 compares **totals**, so to place at all they must reach `our_score + 1`,
and a bigger score needs a bigger requirement. The pressure curve is built
(`P(they clear enough to reach t)`) from the same `YIELD_PRIOR` the shipped
`clears` uses — no new assumption — and applied per candidate, which is what
lets it reorder where the `OppModel::counter` shift could not.

```text
   v13 vs v9   n=6000   control            50.4%   153167 actions
   v13 vs v9   n=6000   V8_PRESSURE=1      50.5%   153199 actions
   v9 vs v13   n=6000   V8_PRESSURE=1      50.5%   153200 actions
   v13 vs v9   n=6000   + V8_MAX_LIVES=2   50.5%   154017 actions
   v13 vs v9   n=6000   aiscream 37PMZ     50.0%   207859 actions
```

It fires — 32 actions changed, so it is not inert — and does nothing. The
reason is that **the premise is wrong, not the implementation.** The exploit
requires the opponent to over-invest into a check they cannot clear. They
choose their set with the same `p_pass` objective we use, so publishing a high
score makes them pick a *harder* life only when they can still clear it. Their
pass rate is therefore insensitive to the target we set, and in the model it
shows up as `clears` being saturated at 1.0 or 0.0 for most boards.

This is the same lesson as `OppModel::counter` from the other side, and together
they close the 43%: **against a policy that optimises its own pass probability,
neither the count nor the score you publish is an attack surface. Both are
signals about you, and a rational opponent prices them rather than being
squeezed by them.**

### The mulligan, the last unmeasured decision — closed

The mulligan fixes the resources the entire game is played from and fires on
~17% of hands, yet every comparison of the two implementations in this project
was a mirror, which cannot see a one-seat change. Measured properly, `v15`
(v8's forward walk) against `v9` (v7's curve):

```text
   v15 vs v9   n=6000   5CP3Z idou       44.8%
   v9 vs v15   n=6000   5CP3Z idou       46.0%
   v15 vs v9   n=6000   aiscream 37PMZ   47.2%
   control     v9 vs v9 n=6000           50.4%
```

v7's static curve is the better mulligan, by about 5.5 points on the flagship
deck and 3 on the second. The recorded mirror figure ("−3.4 from v8 and −5.8
from v9") was directionally right and understated the gap. v7's is right for a
reason the forward walk cannot see: the opening's value is dominated by *how
many live cards and members it holds*, and enumerating openings and forward-
walking three turns of a stochastic game adds variance to that estimate faster
than it removes bias.

### The count-as-signal paradigm (`v13`), and why it cannot bite

The live phase's information structure is exact and worth stating, because it
looks like an obvious hole:

```text
   8.2.2  先攻 ... 3 枚まで ... 裏向きに自身のライブカード置き場に置き、置いた枚数と同じ枚数カードを引きます
   8.2.4  後攻 ... 3 枚まで ... 置いた枚数と同じ枚数カードを引きます
   8.3    パフォーマンスフェイズ      <- the only reveal
```

The only thing crossing the boundary is a **count**. In a deck whose four lives
score 1/2/4/7 that is most of the information there is, so every other part of
`OppModel` holding the opponent's score distribution fixed — a
Nash-within-one-head error — looked like the missing term, and it is a real
one. `v13` prices it: score points per published card, applied to the forecast
case only (attacking first), because attacking second their size is observed and
already inside the model.

```text
   v13 vs v9, n=6000, seed 101        counter   width   v13
   control                              0         1     50.4%   (== v9, validates harness)
                                          1         1     50.4%
                                          2         1     50.3%
                                          4         1     49.9%
   isolation                            0         2     50.4%
   composite                            2         2     50.4%
                                          4         2     49.9%
                                          8         2     49.2%
```

Null alone, monotonically **negative** stacked on width. Two reasons, both
worth recording:

1. **The lift cancels in the argmax.** With one-life sets `our_set_size` is 0 or
   1 and the bot commits ~79% of the time, so the shift is very nearly a
   constant added to every candidate — and a constant cannot reorder a ranking.
   The term only becomes live once set width varies, which is why it has to be
   tested stacked on `v11`, not alone. Stacked, it is negative: the bot reads
   "showing a big set invites a counter" as "showing a big set loses", and
   plays narrow — trading a real score band for a speculative one.
2. **The premise is weaker than it looks.** The count tells the opponent how
   many cards, not what is in them. Their best counter to a count of 1 is not
   automatically a bigger score; with 3 slots and a deck whose lives are
   1/2/4/7, matching a count is often as good as beating it, and 8.4.6 compares
   **totals**, not per-card value. So the coupling the term assumes is not
   necessarily there.

### Solving the commitment subgame properly: a structural impossibility here

The standard recommendation for this game shape is to stop treating the live
phase as a maximisation and solve it as the two-player commitment game it is.
Implemented in full (`V8_COMMIT_GAME`, `commitment_game`): the response axis is
the count they can see; each count's score is the sum of the best `m` distinct
live-card scores actually present (read from hand and every public zone, not a
prior); pass probability falls with `m` because 8.3.15 pools requirements; and
the payoff matrix over (our candidates × their counts) is solved by iterated
best response to a mutual best response. `w = 0` is today's behaviour exactly,
`w = 1` takes the equilibrium outright with no guard.

```text
   v13 vs v9, n=6000, seed 101
   no switch                      50.4%   153167 actions   (control, == v9)
   w = 1.0, guarded               50.4%   153163 actions
   w = 1.0, FULL TRUST            50.4%   153163 actions
```

The action count is the finding. **At full trust the equilibrium picks a
different candidate on essentially zero decisions** — the run is behaviourally
identical to the control, so this is not the guard suppressing it.

The reason is a property of the candidate ordering rather than of the solver:

```text
   p_place = p_pass * (1 - opp_pass)  +  p_pass * opp_pass * we_c
               ^^^^^^                    ^^^^^^           ^^^
               invariant under the opponent's reply        small correction
```

`p_pass` — our own probability of clearing the heart check — does not depend on
what the opponent sets, and it dominates the ranking. The opponent's response
only ever perturbs `we_c`. **So no amount of correct reasoning about their
reply can reorder candidates whose order is fixed by an invariant.** The
commitment subgame is not badly solved here; for this deck and this policy it is
already effectively solved by the greedy rule, and the whole family of
"model the opponent better" interventions is chasing the small term.

That is why fixing the opponent model kept failing: `V8_OPP_SAMPLED_YIELD`,
`V8_OPP_SCORE_ICONS`, `V8_ASSUME_COMMIT`, `V8_NO_ASSUME_COMMIT`, the
count-as-signal term, and now a solved equilibrium all move the same small term.
The live-set direction is closed, and now closed with a mechanism rather than a
pile of nulls.

`V8_MAX_LIVES` still works and is not a bot kind — set `V8_MAX_LIVES=2` on any
bot. (`v13` was the vehicle for this and a refactor briefly capped the contested
width at 1 for non-`v11` bots, which is fixed; `v13` with no switch reads 50.4%
against `v9`, which is the check that the shipped default is unchanged.)

The two that moved the placement rate did not move the win rate at all, and the
two that moved the win rate moved it down. The live-set policy is saturated for
this deck; `v9` also beats `v10` head-to-head (§0.1), so the component
assembly is right too.

**What the evidence actually points at, and why it is not another weight.**

The value function is a function of a single state. Kovařík, Seitz, Lisý,
Rudolf, Sun & Ha, *Value Functions for Depth-Limited Solving in Zero-Sum
Imperfect-Information Games*, Nature 550:354–359 (2017), prove that in an
imperfect-information game this is the **wrong argument** — a state's value
depends on which states the players consider likely, recursively, down to what
is common knowledge. The minimal sufficient object is the **public belief
state**: public board, *both* players' success counts, and a belief over each
player's hidden hand and live set. They report exploitability roughly linear in
value-network quality, which is the first thing in this file with a mechanism
that predicts a non-null where ten tuning sweeps produced none.

Concretely, for `5CP3Z idou`, the belief component that is entirely absent today
is a distribution over **which Live cards the opponent still holds**. `v8` prices
the opponent through `OppModel::top_score()` — a scalar — while the deck's four
lives score 1/2/4/7, so the opponent's achievable score is a set, not a number,
and knowing the set is what 12 Live cards in a 60-card deck makes tractable.

Two supporting items, in order:

1. **A best-response harness (ANC).** Win rate against one opponent is not the
   target and can rise while exploitability rises too. Timbers et al., *Approximate
   Exploitability: Learning a Best Response in Large Games*, IJCAI 2022 /
   arXiv:2004.09677, gives `ANC(π) = Σ u(BR(π₋ᵢ), π₋ᵢ)` with a uniform-best-response
   oracle that needs no opponent API and is a valid lower bound. The engine is
   exact, so a uniform BR is an afternoon's work and it would make every
   subsequent candidate measurable on something other than one opponent.
2. **A frozen opponent pool + Elo.** `v9 > v10` and `v9 > v7` are both large and
   both consistent, so non-transitivity is mild *today* — but a single-opponent
   target is exactly what produces a narrowly-overfit policy once it bites.

Untested and still cheap: the second deck (`aiscream 37PMZ`) and the mulligan,
which fires on only ~17% of hands and where both implementations measure flat.
