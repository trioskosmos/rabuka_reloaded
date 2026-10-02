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

v9 is at a genuine local optimum. ~80 measured arms produce no win rate change
that survives a seed change and an asymmetric opponent, and 86% of the
placement the rules permit is already being taken. Raising win rate requires
breaking the symmetry — a decision rule that wins strictly more score
comparisons than the shared policy does — not another weight, another gate, or
another live-set term. The most promising untouched surface is the second
deck (`aiscream 37PMZ`, 60% of ceiling) and the attacker-role choice.
