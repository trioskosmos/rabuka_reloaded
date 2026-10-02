# v8 improvement log

What was actually done to v8, what it bought, and what it did not. Every number
here is a measurement; every claim that did not survive measurement is recorded
as a dead end rather than deleted, because "we already tried that" is the most
expensive lesson there is.

Doctrine lives in [`BOT_STRATEGY.md`](BOT_STRATEGY.md). This file is the
engineering record. What to do next — the located causes and 25 ranked methods
— is in [`V8_NEXT_METHODS.md`](V8_NEXT_METHODS.md).

---

## 0. Second audit round (2026-09-29): two of the located causes were mislocated

This round started by trying to re-diagnose the v7/v8 live-set divergence before
building anything against it, because `V8_NEXT_METHODS.md` §4.2 and §6.1 name a
cause that turned out to be wrong, and the headline number behind it is an
artifact. Both are recorded here because both had been load-bearing.

### 0.1 "v7 sees live-start abilities because its live set rolls out" — false

§4.2 attributes v7's advantage on the live set to a rollout that executes the
live start for real. v7's default live path does not roll out.
`choose_live_set_v7` returns at `strategy_v7.rs:489` into
`choose_live_set_experiment` → `live_set_plan` (`:2145`), which is a static
analytic model: `experiment_portfolio_rank` (`:2022`) and `best_response_set`
(`:2917`) both compute pass probability from sampled board pools and printed
scores. The rollout is reachable only behind `V7_ROLLOUT` (`:486`) or
`V7_ROLLOUT_ASSIST` (`:2478`), neither set in a default run. **Both bots are
blind to `{{live_start}}`.**

### 0.2 The has-live-start split has no contrast to measure

`V7CHK`/`V8CHK` now emit `lcand` (lives in hand) and `lscand` (how many carry a
`{{live_start}}` ability). Measured over 200 paired games, `lscand == lcand` on
**every single line** — 100% of the lives this deck puts in hand carry a
live-start ability.

So the planned test (split `v7-chosen` vs `v8-chosen` by has-live-start) has no
negative class and cannot separate the two candidate causes. The MIRACLE NE
12-vs-0 split cannot be about live-start *presence*, because presence is
universal here. Any future live-start work has to discriminate on **which**
ability and **how much** it adds, not on whether one exists.

### 0.3 The "v7 commits at 0.924, v8 at 0.485" comparison was two definitions

This is the important one, and it is the number §6.1 and the burn-price
arithmetic are built on.

`chosen_set_pass_probability` returned **1.000** for a zone holding only junk.
`desired` holds hand *indices* and `experiment_junk_fill` fills spare slots with
junk, so a junk-only zone has a non-empty `desired` and an all-zero pooled
`need`, which every sampled pool trivially satisfies. v8's equivalent takes an
explicit `if chosen.is_empty() { 0.0 }` branch and reported **0.000** for the
same physical decision.

So v7's mean committed p was inflated toward 1.0 by every junk-only zone and
v8's was pushed toward 0.0 by every one of its own, and the reported gap
between them is substantially measurement artifact rather than a difference
between the bots. Folding is not a check passed — it is a check handed over.

Both sides now report 0.0 for a zone with no life in it, and `ppass` is named
apart from `p` (`p_place`) on both lines. The old claim should be treated as
unmeasured, not as evidence.

### 0.4 A correction to the earlier note in this file

This log previously recorded the belief that v7's live set rolls out and
therefore sees abilities, and built on it. It does not, and the belief was never
tested. The lesson generalises past this case: a mechanism attributed to
another bot's *architecture* should be confirmed by reading that bot's default
path, not inferred from the fact that the mechanism exists in the file.

### 0.5 Instrument fixes that were bugs, not improvements

- `best_available` was bound in the `V7_CHECK_P` block and never formatted. The
  compiler confirmed it (`unused variable: best_available`): that headroom
  number has never reached a capture. It now prints, and the set-aware version
  is a separate function from the decision-facing one — see §0.6.
- v8's `V8CHK` printed `p_place` where v7's printed pass probability, under a
  comment calling the two lines "paired" and "comparable at the same blade
  count". Different quantities; a `p` difference between those lines was being
  read as a development difference.
- `best_available_pass_probability` is a **v7 decision input** (`live_set_plan`
  feeds it to `expected_live_value` as `mine_p`), not only telemetry. Making it
  set-aware to match the headroom question would have silently changed the
  reference bot mid-audit. The decision-facing function is untouched; the
  set-aware one is new and telemetry-only.

### 0.6 Two correctness fixes in the live-set candidate enumeration

- `1 << lives.len().min(16)` silently made every life past the 16th
  unreachable, with no counter and no log. Replaced with a bounded-width
  generator over `C(n,1..=3)` — correct at any hand size, cheaper at every size,
  and order-identical at the shipped default width of 1. Pinned by
  `candidate_enumeration_sees_every_life_whatever_the_hand_size`.
- The `V8_NO_FOLD` ablation is not broken, contrary to an earlier reading: the
  dead `entry.value = f64::NEG_INFINITY` line is overwritten by design and the
  mechanism is the overwrite onto the fold entry. Moving that line to "fix" it
  would delete the candidate the switch exists to promote.

**Baseline preserved.** After all of the above, `bot_arena v8 v7 400 "5CP3Z
idou" --seed 11` is bit-identical to before the round: 179-221, 16163 actions,
placements 0.403 / 0.451. Nothing in this section changes how either bot plays.

### 0.7 The colour-exact Main leaf: a real dead end, and the tenth

`V8_NEXT_METHODS.md` §6.4 and the comment on `best_life_deficit` both argue
that the v8 Main leaf is colour-blind and that this costs a lot. The argument
is correct about the leaf and wrong about the remedy.

`v8_model::colour_shortfall` prices the per-colour shortfall of the biggest
life in hand, and `v8_main::leaf_value` takes it behind `V8_DEFICIT`. Unlike
the previous nine attempts it is **not** a monotone function of what
`band_progress` already reads: `colour_shortfall_sees_the_vector_not_the_total`
pins that two equal-total boards in different colours get different values. So
the standard defence of the dead ends does not apply and it had to be measured.

It is a regression, and worse at a larger weight:

| weight | paired result | McNemar p | pace (P1) |
|---|---|---|---|
| off (baseline) | 179-221 | — | 0.403 |
| 0.05 | 165-235 | 0.04356 | 0.384 |
| 0.35 | 148-252 | 0.00015 | 0.366 |

Monotone in the harmful direction across a 7x range. That is a gradient
pointing at zero, not a surface with a peak the sweep missed, so it is not
worth spending a large-n run on the way the blades sweep was spent.

**The likely reason, and it is a target-selection error rather than a
colour-blindness one.** The term aims at the *biggest* life in hand. Early in a
game the biggest life is usually one we cannot set for several turns, so the
term rewards colouring the board toward a card that is not yet payable and
pulls development away from the raw growth the opening actually needs. The
signatures match: pace falls monotonically with the weight, and stage cost
rises while placements fall — more board, fewer conversions, which is the
opposite of what the term was supposed to buy.

So the open question is not "is colour-blindness bad" — the dead ends already
covered that, twice over. It is **which life to aim at**: a fixed big target is
worse than no target. A term that prices the best life we could actually set
*now* is a different proposition and is not refuted by this.

Kept behind the switch, off by default, rather than deleted, so the ablation
stays reproducible. The default path is also guarded so it does not pay for a
term nobody is holding.

### 0.8 The opponent horizon: zero effect, and the reason is a step function

§6.5 asks for the opponent model to be "refreshed through the search". As
written that is a no-op and it is worth saying why: during our own Main phase
the opponent's board cannot change, so re-reading it at each node returns an
identical model. The staleness is in the **horizon**, not the depth — the
horizon term runs our forward walk two turns and then prices the opponent at
the board they have *now*, so inside a term whose whole job is to see the race
in two turns, the opponent never grows.

`OppModel::with_board` re-evaluates `pass_prob` and `score_dist` against a
projected board, and `V8_OPP_HORIZON` advances their public board by the same
amount ours advanced (both players draw the same cards and gain the same
energy — 3.2; their hand and deck are never read).

Measured over 400 paired games: **zero discordant pairs.** p=1.00, every one of
16163 actions identical. Not a small effect below the noise floor — the term
never changed a single decision.

The cause is that the opponent's pass model is a hard threshold. `clears` sums
`YIELD_PRIOR` weights gated on `supply >= bar` and clamps to 1.0, so across 369
board/blade combinations spanning the guides' entire real range (T1~4 to T7~26
hearts) the model takes **6 distinct values**. Advancing their board lands on
the same step almost every time. "The opponent is catching up" is not a
statement this model is able to make.

Pinned by `the_opponent_pass_model_has_only_a_handful_of_states`, which
asserts the measured 6 rather than a hoped-for gradient — so a future change
that gives the model a real gradient has to come to that test deliberately and
raise it, instead of passing unnoticed.

**This invalidates a whole family of planned methods, not one instance.** Every
opponent-horizon idea in §6 — the refresh, and anything else that wants the
opponent's threat to grow over time — is measuring noise until the model has a
gradient. The fix is not a weight. It is replacing the threshold with the same
coverage calculation v8 already uses for its *own* check (`FlipSampler` /
`pass_probability`), so a board one heart short of the bar scores differently
from one three hearts short. That is a real, buildable improvement and it is
the highest-value next step in the file.

`OppModel::with_board_sampled` is that fix, built and measured. It does **not**
rescue the horizon: with the gradient on and the horizon on, the run is still
**bit-identical**, zero discordant games. The horizon family stays dead, but for
a second and more interesting reason — with a continuous model available, the
term still changes nothing, so the staleness was never the binding constraint.

---

## 1. Sweep round: three harness bugs, then the actual result

Everything above was tuning a leaf. The sweep that followed was built to stop
doing that, and it found the harness was lying three separate ways.

### 1.1 `mcnemar_exact_p` was structurally incapable of returning p < 1

It computed `C(n,k)` as `(lf[n] - lf[k] - lf[n-k]).exp()` and divided by `2^n`
afterwards. `C(n, k)` near `n/2` grows like `2^n`, so the exponential overflows
`f64` (which caps near `exp(709)`) once `n * ln 2` passes it — roughly **n > 1020
discordant pairs**. Past that every term is `inf`, the tail is `inf`, and
`(2.0 * inf).min(1.0)` is **exactly 1.0**.

Observed: 933 baseline-only against 1259 candidate-only discordant — about as
lopsided as a paired test gets — printed `p = 1.00000`. It does not fail
conservatively, it fails by reporting no difference when there is a large one.
**Any "no significant difference" read at n >= ~2000 from this function was a
statement about the arithmetic.**

Fixed by keeping the exponent in log space (`- n*ln2` carried into the
exponent), so each term is a probability and only the sum is formed. Pinned by
`mcnemar_survives_more_than_a_thousand_discordant_pairs`.

Also corrected in the same pass: the textbook McNemar value for a 1-vs-6 split
is 0.125 (exact), not 0.21875 (continuity-corrected chi-square). Both are now
pinned, because "the textbook value" for a paired test is often not the paired
test.

### 1.2 The verdict took its sign from the wrong quantity

`VERDICT` chose IMPROVEMENT vs REGRESSION on the swing in **decisive rate** —
the fraction of games that were decisive. In a mirror matchup with no draws,
that is 1.0 for both arms, the swing is exactly `0.0` for *any* change, and the
sign test fell through to the else branch.

So it printed `SIGNIFICANT REGRESSION` for a run that took v8 from 43.7% to
**57.3%**. Decisive rate measures draw frequency, which is a property of the
rules and the matchup, not of whether the bot got better. The verdict now uses
the paired win count, and both are printed:

```
  paired win rate P1: baseline 43.7% -> candidate 57.3%  (+13.67 pp)
  VERDICT: SIGNIFICANT IMPROVEMENT
```

### 1.3 A bot can break the game and the brief line said nothing

A configuration ending **71% of games in an engine-terminated 0-0** still
prints a plausible win/loss line over the remaining 29%, and that is the line a
sweep reads. `no_contest` is now on the `--brief` result line.

This one nearly caused a false ship. `V8_TIE_BATON + V8_PASS_TAX=0.15` measured
**191-209 at n=400** — a clean, plausible-looking win. At n=3000 the same switch
put 2130 of 3000 games into no-contest at a median of two turns. The n=400 cell
was not a smaller version of the same effect; it was a different phenomenon,
and the only thing that could have caught it at the time was the one number now
printed.

The harness also grew `--env K=V` (repeatable, echoed into the run's output) and
`--check-p`. Every mechanism in this project is env-gated and none was reachable
from the command line, so the whole ablation method was unreproducible from a
script and the exact switches behind a recorded number were often not the ones
recorded.

### 1.4 The sweep itself: ten configurations, n=400

| configuration | result | verdict |
|---|---|---|
| `V8_TIE_BATON` | 184-216 | +0.25 pp, inert |
| `V8_PASS_TAX=0.15` | 189-211 | +3 pp, p=0.12 |
| `V8_PASS_TAX=0.40` | 187-213 | +2 pp, p=0.22 |
| `V8_OPP_GRADIENT`+`HORIZON` | 179-221 | **bit-identical** |
| `V8_MAX_LIVES=2` | 173-227 | −2 pp |
| `V8_HORIZON=0.25` | 175-225 | null |
| `V8_NODES=192`+`FOLLOWUP=10` | 178-222 | 3 discordant / 400 |
| `V8_DEFICIT=0.05` | 165-235 | regression |
| `V8_DEFICIT=0.35` | 148-252 | regression |
| `TIE_BATON`+`PASS_TAX=0.15` | 191-209 | +3 pp — **see §1.3, it was a trap** |

`V8_TIE_BATON` is worth its own line, because it is the most *informative* null
in the file. In v7 the baton-over-Pass tie-break is worth **+16.8 points**, the
largest entry in the ablation table, and v8 has no equivalent. Adding it to v8
changed **23 of 1200 games** (p=0.68).

The reason is that the tie-break only fires when the leaf ties, and v8's leaf
almost never does. The handoff doc's claim that 81.8% of v8 actions produce an
identical top value is not consistent with 23 changed games in 1200. Whatever
that 81.8% was measuring, it is not tie frequency, and the +16.8 mechanism has
**no purchase in v8 at all**. That is a much more useful thing to know than a
sixth null leaf term, and it is not derivable from reading either bot.

---

## 2. The crossover: the deficit is the Main phase, and v8's live set is an asset

Ten leaf-term experiments had all failed, all of them edits to v8's Main-phase
leaf, all against a ~9 point deficit. The cheap way out of that loop is not
another term — it is to stop assuming which half of the bot is responsible and
measure it.

`V8_BORROW_MAIN=v7` and `V8_BORROW_LIVE=v7` swap one half of v8 for v7's.
Paired, n=1200, same seeds, same seat (`--brief` reports `no_contest` on every
cell, so none of these is a game-breaking configuration):

| P1 configuration | P1 vs v7 | note |
|---|---|---|
| v8 Main + v8 live (shipped) | **43.7%** | the number to beat |
| v7 Main + v8 live | **53.9%** | recovers ~10 points |
| v8 Main + v7 live | **40.7%** | v7's live set is *worse* |

**The entire deficit is in the Main phase, and v8's live-card set beats v7's by
about 3 points.** Both halves of the handoff doc's story are the wrong way
round: §6 was written to improve the live set, which is already the better one,
and the leaf, which is where the loss actually is.

This also explains the ten failures mechanically. Every one of them added a term
to the Main leaf, which is the right *phase* — but they were all rejected for
being monotone functions of what the leaf already reads, and that diagnosis was
correct. A tie-break term could not work because the tie-break is unreachable
(§1.4). The leaf was never the right level of abstraction; v7's Main wins by
*re-offering and re-evaluating* after every action, not by valuing the first one
better.

### 2.1 Seat control, and v9

A crossover is only worth reading once the seat is accounted for. Mirror runs,
n=1200, seed 11, current card pool:

- `v7 vs v7` → P1 **52.6%**
- `v9 vs v9` → P1 **51.2%**

so P1 carries a small advantage and it is the same size for both. That makes the
crossover cell's 53.9% real, and it also means any cross-bot number has to be
read against a mirror, not against 50%.

`v9` is now a first-class bot kind (`v7 Main` + `v8 live set` + `v7 mulligan`),
because a composite needs to be playable on **one** seat against a pure
opponent and `--env` is process-global.

Paired against the shipped v8, same seeds, same seat:

```
bot_arena v9 v7 1200 "5CP3Z idou" --games 1200 --seed 11 --vs cb11.csv
  RESULT v9 vs v7 | 688 - 512 | 0 draws (0.0% nocontest) | 41188 actions
  DISCORDANT: baseline-only 178 | candidate-only 342
  exact McNemar two-sided p = 0.00000
  pace (P1) 0.3910 -> 0.4764  +0.0855  SIGNIFICANT
  paired win rate P1: baseline 43.7% -> candidate 57.3%  (+13.67 pp)
  VERDICT: SIGNIFICANT IMPROVEMENT
```

**v9 is 57.3% against v7, against a v7 mirror rate of 52.6% — about 4.7 points
clear of the strongest previous bot, on the same seat, with the p-value now
actually computed.** Placements per live phase go 0.391 → 0.476, and the
no-contest rate is zero, so this is a real gain and not a faster way to lose.

### 2.2 What this does and does not settle

It settles where the loss was. It does not fix v8's Main phase — `v9` does not
contain it, it routes around it. The honest statement is: **v8's Main-phase
model has not been beaten and should be treated as unproven**, with one located
lead (v7 wins by re-offering and re-evaluating per action, and the value of
doing so is measurable at 2.3 points in v7's own ablation) and one dead end
(its tie-break mechanism, worth 16.8 in v7, has no purchase in v8 at all).

`v9` is the strongest bot measured in this project. `V8_NEXT_METHODS.md` §6
should be read as aimed at the wrong phase.

### 2.3 v9 confirmed on a holdout seed, and a corrected asymmetric read

Seed 11 was used to find the crossover, so it is a selection seed and cannot
confirm. Seed 23 was not read until this point.

| matchup | v8 | **v9** | paired p |
|---|---|---|---|
| mirror, seed 11 (selection) | 43.7% | **57.3%** | 0.00000 |
| mirror, seed 23 (**holdout**) | 42.8% | **57.6%** | 0.00000 |

The holdout replicates the selection seed to within 0.3 points, with the same
sign and the same significance. That is the bar the blades sweep failed.

#### The asymmetric row, and why the first version of it was wrong

An earlier version of this section reported an asymmetric run — v9 at 70.8%
against v8 at 52.9% — and called it a larger off-deck gain, which is the check
the mulligan bug would have failed. **It was measuring the deck.**

`5CP3Z idou` and `aiscream 37PMZ` are not a close matchup. v7 playing both, same
seeds, n=1200:

| seat 1 deck | result | win rate |
|---|---|---|
| `5CP3Z idou` | 809-391 | **67.4%** |
| `aiscream 37PMZ` | 383-817 | **31.9%** |

The neutral baseline for that matchup is **67.4%**, set entirely by the decks.
v9's 70.8% is therefore 3.4 points above neutral, and v8's 52.9% is **14.5
points below** it. Read against neutral rather than against each other:

| matchup | neutral (v7 v7) | v8 | v9 | **v9 vs neutral** |
|---|---|---|---|---|
| mirror, seed 11 | 52.6% | 43.7% | 57.3% | **+4.7** |
| mirror, seed 23 | — | 42.8% | 57.6% | — |
| asymmetric, seed 11 | 67.4% | 52.9% | 70.8% | **+3.4** |

So v9's edge is **smaller** off-deck than in the mirror, not larger, and the
"+17.9" was an artifact of using v8 as the reference in a matchup where v8 is
badly outclassed by its own opponent's deck. The mirror result is unchanged and
still the load-bearing one.

Two things that row did get right, and that are worth keeping: every cell reports
0.0% no-contest, and v8 sitting 14.5 points *below* neutral here is a sharper
statement of the same finding as §2 — v8's Main phase does not just fail to beat
v7's, it actively loses ground when handed a favourable matchup it could have
banked.

**Ship v9.** v7's proven Main phase, v8's better live-card set, v7's mulligan.
It is behind v7 nowhere measured, ahead on a selection seed and a holdout seed,
and ahead on a second deck by a smaller but same-signed margin.

### 2.4 "the game is just math": grading the search itself

If the Main phase is where the loss is, and the right abstraction is to search
the real game rather than hand-build a value, then the machine already exists —
`v7_ismcts` is an ISMCTS with a terminal-exact value function
(`game_value_is_terminal_exact`) and a success-race read
(`game_value_reads_the_success_race`). `V7Rollout` exists precisely so it can
be graded as a real policy "instead of assuming it is good". It never was.

Graded as the Main phase, n=1200, seed 11:

```
bot_arena v7rollout v7 1200 "5CP3Z idou" --games 1200 --seed 11
  RESULT v7rollout vs v7 | 552 - 648 | 0 draws | 42415 actions
```

**46.0%, against a v7 mirror of 52.6% — the search loses 6.6 points to the
static policy it is refining.**

This is the most valuable negative in the file, because it removes the
attractive explanation. A terminal-exact value function and a real search should
dominate a hand-built leaf, and it does not. So the Main-phase deficit is
**not** a search-budget problem and **not** an evaluation problem, and no amount
of nodes, rollouts, or a better leaf will close it. What is left is that v7's
Main phase is doing something specific that neither a hand-built leaf nor a
rollout search reproduces — and the crossover says it is worth about 10 points.

It is not the tie-break (§1.4: unreachable in v8). It is not leaf weighting
(ten experiments). It is not depth. The one structural difference left on the
table is that v7 re-offers and re-evaluates after **every** action in a chain,
while v8 scores one action and then takes the best of a single re-offer's leaf.
That is the next thing to measure, and it is a change of shape rather than of
weight — which is the only kind that has worked so far.

### 2.5 Replacing the maths with search: the second and third replication

The natural reading of "the game is just math" is: stop modelling the check and
compute it. The machinery for that already existed and had never been graded on
a single seat. Two of the three cells were run, paired, n=1200, seed 11, against
`v9`:

| what was swapped in | result | paired p |
|---|---|---|
| engine-executing live set (clone, commit, let the rules resolve) | 57.3% → **49.5%** | 0.00000 |
| v8's forward-walk mulligan for v7's curve rule | 57.3% → 53.9% | 0.07382 |
| ISMCTS Main (§2.4, separate run) | 46.0% vs 52.6% mirror | — |

**Both lost, and the first lost badly.** A terminal-exact value function, a real
search, and the actual engine resolving the actual check are each worse than
v8's arithmetic.

That is three independent replications of the same result, and it is the most
useful thing in this file. The explanation is not that the maths is right and the
simulation is wrong — the simulation is *more* correct about the check. It is
that **search cannot repair a weak continuation policy, it amplifies it.** A
rollout that continues with a mediocre policy prices a mediocre future
correctly, which is worse than an optimistic analytic model of the check, because
the analytic model at least errs on the side of the move that keeps options
open. Every "just simulate it" attempt in this project has made the bot more
pessimistic and more brittle in the same direction, and lost 6-8 points each
time.

**The only thing that has ever worked here is changing which policy makes the
decision, not changing the maths it uses.** One crossover (+13.7). Eleven
evaluations improved, reverted or regressed. That is not a coincidence worth
arguing with; it is the shape of the problem.

### 2.6 The maths that is actually missing

Ranked by what the evidence leaves standing, not by what sounds deepest.

1. **Sequencing, not valuation.** v7's advantage is re-offering and
   re-evaluating after each action — a *plan* through the action space. Every
   experiment above improved the *score* of a single action. The one lead with
   support behind it is a greedy chain over real engine states that re-prices
   after each chosen action and accumulates, rather than scoring one action and
   taking the best single re-offer. This is a change of shape, which is the only
   category with a positive result in the file.
2. **The opponent as an agent.** Every model here prices the opponent as a fixed
   board. Nothing models that they *respond*: that setting a life reveals
   information, that forcing an early commit has value, that they will fold at
   2-2. A 2-ply expectimax over (our action, their live-set choice) with the
   real live-set model on both sides. Untested. The cheap way to find the states
   that matter is the note's sandbox-agent idea: let a search opponent that
   knows our policy play against us, and read the states where it wins, rather
   than hypothesising them.
3. **Belief over an uncommitted set.** `OppModel` forecasts the opponent's set
   from their board; once committed it is exact and observed. The correct object
   is a posterior over what they are holding, and the value of an action includes
   what it reveals. Untested.
4. **The race, not the check.** The objective is P(reach three first). Both bots
   substantially maximise per-check placement probability. Those are different
   functions, and in a race the right move is sometimes a thin check that wins
   tempo and sometimes a reliable one that loses the race. `rule_payoff` knows
   this and nothing optimises it.
5. **Scheduling energy across turns.** Energy carries over, so a turn in which
   nothing can be spent is a turn lost, and the real object is a knapsack over
   the remaining turn schedule rather than a per-turn greedy. The log's "energy
   unspent" term never fired because it was a level term; a schedule is not a
   level.

None of these is a term to add. Each is a different shape, and shape is the
thing that has been working.

### 2.7 Items 1 is falsified, and so is the granularity theory

Both built and measured, paired, n=1200, seed 11, on v8's own Main phase —
the phase that is actually losing.

**The chain (`V8_CHAIN`).** `Search::chain` commits to the single best next
action, executes it for real, and re-offers *from the resulting state*, taking
the final leaf and only extending on a strict improvement. `V8_CHAIN=3` and
`V8_CHAIN=6` returned **bit-identical results to each other** (522-678, 30
discordant, p=0.856, −0.17 pp) — the chain stops before it extends, because
after the first committed step no further step strictly improves on the leaf it
came from.

So the hypothesis behind item 1 is wrong. v7 does not beat v8's Main by
sequencing. The crossover says the difference is real and worth ~10 points; a
chain is not it.

**The granularity theory.** This came out of that null rather than being planned.
`V8_QUANT=<step>` rounds the leaf onto a coarse grid, on the theory that v7's
leaf ties constantly (so its +16.8 tie-break does real work) while v8's smooth
leaf never ties, and that a value function coarse enough to *abstain* between
actions it cannot distinguish would hand those cases to a key encoding guide
knowledge instead of to float noise. The opposite of every other attempt here,
all of which added precision.

| cell | result | p |
|---|---|---|
| `V8_QUANT=2` | 43.7% → 43.2% | 0.359 |
| `V8_QUANT=2` + `V8_TIE_BATON` | 43.7% → 43.6% | 1.000 |

Also null. And note the tie-break under quantisation changed 43 games where it
had changed 23 without it — quantising roughly doubled tie frequency and it
still bought nothing, so the +16.8 in v7 is not "coarse values plus a
tie-break". It is something else in v7's decision that this whole line of
investigation has not touched.

### 2.8 Stop: the evidence says ship v9, not fix v8's Main

Every approach taken to v8's Main phase, in one place:

| approach | result |
|---|---|
| ten leaf terms | inert or regression |
| colour-exact deficit | regression (p=0.044 at w=0.05) |
| opponent horizon, then + gradient | bit-identical |
| `V8_TIE_BATON` | 23/1200 games |
| deeper search (`NODES=192`, `FOLLOWUP=10`) | 3/400 games |
| ISMCTS replacing the leaf | **−6.6** |
| engine-executed live set | **−7.8** |
| forward-walk mulligan | **−3.4** |
| greedy chain | null, bit-identical at depth 3 and 6 |
| quantised leaf, with and without the tie-break | null |

**Nothing has moved v8's Main phase in either direction except making it worse.**
Eleven attempts, spanning evaluation, search, simulation, sequencing and
granularity, and the only positive result in the entire file is the crossover —
which does not improve v8's Main phase, it replaces it.

The reasonable reading is that v8's Main phase is not a worse *evaluation of the
same decision*. v7's Main phase is doing something that none of these levers
reach, and continuing to guess at it is now the least efficient thing available.
`v9` captures the whole gain by using v7's Main phase and v8's live set, is
confirmed on a holdout seed, and is the strongest bot this project has measured.

**Recommendation: ship v9 and stop work on v8's Main phase.** Items 2-5 of §2.6
remain genuinely untested and are the right next targets — but they target the
*live set and the race*, not the Main phase, and the evidence gives no reason to
think any of them will rescue a phase that has now resisted evaluation, search,
simulation, sequencing and granularity in turn.

---

## 3. Stop guessing: read v7's source (and the actual game math)

§2.8 concluded that v8's Main phase was not worth further work. That was a
conclusion drawn from black-box measurement of my own changes while never having
read the two functions side by side. Reading them took ten minutes and
immediately contradicted it.

### 3.1 The structural difference, in v7's own words

`v7_main::value` (`v7_main.rs:243`) is a **linear function of coarse feature
deltas**:

```text
  8.0 * dCost   +  3.0 * dHearts  +  6.0 * dBlades  + 60.0 * dCoverage
+ 25.0 * dAmmo   +  2.0 * dEnergy  +  3.0 * dHand    +  0.5 * dReserve
+  2.0 * dFuture + 1000.0 * dSuccess
```

and `Search::evaluate` (`v7_main.rs:372`) prices the **opponent** the same way:

```text
- 8.0 * dOppCost  - 6.0 * dOppBlades  - 10.0 * dOppHand  - 1000.0 * dOppSuccess
```

v8's leaf is a different kind of object entirely: a **level** in
placement-probability units. Blades and ammo reach it only through
`placement_pair`, which routes them into a binomial that **saturates** — once
the check is comfortably passable, more blades move the probability by nothing.
v7 is paid 6 per blade whether or not the check is already safe.

So v8 does not mis-*weight* the terms v7 has. **It structurally cannot express
two of the five biggest ones**, and the ablation says they are the two biggest:
blades -5.1, ammo -2.9, and every other term under 1.0.

The eleven earlier experiments all added *terms of v8's own* on the assumption
that the problem was precision. It was not. It was a missing axis.

### 3.2 Blades: the first real win on this phase in the file

`V8_BLADES=<w>` adds a linear blade term in v7's form. The weight is v7's own,
6.0 — not tuned.

| seed | baseline | +`V8_BLADES=6` | paired p | pace (P1) |
|---|---|---|---|---|
| 11 (selection) | 42.5% | **43.4%** | **0.02963** | +0.0041 SIGNIFICANT |
| 23 (**holdout**) | 41.8% | 42.3% | 0.21995 | +0.0030 SIGNIFICANT |

n=4000 per seed. Pooled across 8000 games: 215 baseline-only against 270
candidate-only, McNemar p ≈ **0.013**.

**The first significant improvement to v8's Main phase in the entire file**, and
it came from reading v7 rather than from any amount of measuring v8.

Note the dissociation, which is worth more than the win rate: **pace rose
significantly on both seeds while win rate did not.** The term makes v8 place
more reliably without making it win more. That is a real and slightly
uncomfortable result — it is the same gap the whole project has been measuring
(pace ~0.40 against a 1.0 target) and it says pace on its own is not the
objective. Both numbers are now printed on every paired run for exactly this
reason.

### 3.3 The opponent half, and why calibrating it changed nothing

`V8_OPP_BLADES=6` (the differential, `v7_main.rs:373`) measured +1.08 pp against
+1.25 pp for our own blades alone — the same effect, so the opponent-side term
adds nothing measurable, and it is not carried.

`V8_OPP_DENSITY=measured` measured **5 discordant games in 1200**. That is the
third and final confirmation of the saturation diagnosis. `YIELD_PRIOR` has a
mean of 0.505 blade-hearts per blade while independent community calculation
puts the real figure near 0.733
(note.com/miumi_loveca — "1回㝮エール㝧ブレードポート㝌ゝ㝝れる確率"), and
correcting a 45% error in the opponent's yield changes **five games**, because
`pass_prob` takes 6 distinct values and there is nowhere for a 45% shift to land.
**A model that coarse cannot be calibrated.** The number was right and the
function was too short to notice it.

### 3.4 A bug in my own instrumentation, which looked exactly like a finding

`V8_DELTA_LEAF` + `V8_CHAIN=4` first measured **6.7%**, from 43.7%, with pace
collapsing 0.39 -> 0.20. That is the shape of a catastrophic result and it would
have been a comfortable thing to write up as "delta leaves destroy the bot".

It was a unit mismatch on one branch: `Pass` was scored with `leaf()` (a level,
worth ~+400) while every other action used `scored_leaf()` (a delta, near zero),
so Pass won unconditionally and v8 stopped deploying. Fixed, the same cell
measures **+0.17 pp, p=0.86 — null**.

Both readings agree that sequencing does not help: the original chain stopped
before extending (depth 3 and depth 6 bit-identical), and the delta version
extends properly and still does nothing. §2.7 stands, and this is the second
time in this file that an instrumentation bug produced a result convincing
enough to be worth writing down.

### 3.5 Next: coverage, the largest v7 term still missing

`V8_COVERAGE=<w>` is `60.0 * dCoverage` transcribed the way v7 computes it, as a
**delta on the best life in hand's own requirement**. This is deliberately not
the `V8_DEFICIT` experiment that regressed (§0.7): that one subtracted an
absolute shortfall (a level, which cannot reorder a saturated argmax) and aimed
at the *biggest* life in hand (usually one that cannot be set for turns). The
blade result is the reason to retest the same underlying idea in v7's shape — the
reading of v7's source has now produced one prediction that held and one that
did not, and both were worth running.

### 3.6 The decision-level study: what v8 actually gets wrong

Everything above measured aggregates. `V8_PROBE_V7=1` puts the same state to
both policies — both choosers are pure, so no clone is needed — and logs only
where the chosen `Action` differs. Three games, 14 disagreements:

| disagreement | count |
|---|---|
| **v7 = `UseAbility`, v8 plays a member or Passes** | **8** |
| v7 = Pass, v8 plays a member | 4 |
| both play, different member | 2 |

**More than half of every disagreement v8 loses is an ability it never takes.**

Two earlier findings now have a single cause, and the third is new:

1. **v8 never activates an ability, and the reason is the leaf, not a gate.**
   The engine offers them (the doc's "use_ability counts as zero" was about a
   *scoring* filter, not the action set). v7 takes them because its value
   function is linear in feature deltas: an ability that adds a blade scores
   `6.0 * dBlades` the instant it resolves. v8's leaf is a saturated
   placement-probability level, so the same ability scores **zero** unless it
   happens to cross a probability threshold. It is not that v8 is discouraged
   from activating — it is that v8 cannot see what activating did.
2. **The blade term was this fix, partially.** `V8_BLADES=6` pays for blades
   already on the board. It does not pay for blades an ability would *produce*,
   which is why it was worth +0.88 and not more.
3. **Development was never the constraint.** In the traced games v8 reached
   `bl=10 cost=30 h=13` against `bl=5 h=4`, and the two placed **identically**
   (0.438 per live phase each), and v7 won 2-1. Every leaf term in the file
   was tuning a quantity that was already not binding.

This is the localisation eleven aggregate experiments could not produce. The
earlier note that abilities "measured as null" was measured as *encouragement* —
a turn-window bonus for activating — which is a different intervention from
paying for what the activation produces. Encouraging a move that scores zero is
encouraging noise; pricing the feature it adds is the same intervention that
already worked for blades.

### 3.7 What the research said, and what it changed

`note.com/kasiwamoti_y` — "for people who want to get seriously strong at
Loveca" — is strategy rather than engineering, and two of its claims bear
directly on the code:

- **Cards with no blade-heart are the central resource.** Growth needs
  high-cost members, those have no blade-heart, and every one lowers the check
  rate. Worked numbers for having 6 / 7 / 8 of them and still drawing the
  cost-9 by turn 2: **89% / 93% / 95%**. The sixth is worth +4 points and the
  eighth +2, so the marginal card is nearly dead by seven. `grep` over
  `engine/src/bot/` finds **no code that counts them** — the mulligan scores the
  check and never sees what the hand costs in growth fuel. Now
  `V8_MULL_BH` / `V8_MULL_BH_BUDGET`, off by default.
- **The per-deck per-turn score target is a deck property** (their example:
  2 → 3 → 6(7) → 7(8)). `band_progress` uses the *generic* guide band table.
  `curve_prob` already computes a list's own reachability and no bot calls it.

The measured per-yell blade rate, ~0.733 against `YIELD_PRIOR`'s 0.505, is the
same article's companion calculation — and §3.3 showed that correcting it moves
**five games in 1200**, because the opponent's model is a six-valued step
function with nowhere for a 45% correction to land. A number can be right and
the function still be too short to notice.

### 3.8 Pricing abilities, and what it says about the blade "win"

`V8_ABILITY_VALUE=<w>` pays a `UseAbility` for the features it actually
produces, in v7's own credits: `6.0 * dBlades`, `3.0 * dHearts`. Paired,
n=4000, seed 11:

| config | win rate | paired p | pace (P1) |
|---|---|---|---|
| baseline | 42.5% | — | 0.3735 |
| `V8_ABILITY_VALUE=1` | **43.5% (+1.00)** | **0.01424** | +0.0044 SIGNIFICANT |
| `V8_BLADES=6` (§3.2) | 43.4% (+0.88) | 0.02963 | +0.0041 SIGNIFICANT |
| both | 43.5% (+1.02) | 0.01244 | +0.0044 SIGNIFICANT |

**Ability alone ≈ blades alone, and together they add 0.02.** They are the same
effect, and the blade term was a proxy for it: paying 6 per blade on the board
happens to reward the states abilities create, so `V8_BLADES` was measuring the
ability bug without knowing it. The precise version — paying for the *change*,
which only fires on actions that change it — is worth the same and is the one
that is actually about the defect.

This also settles the "abilities are settled negative" note in this file. That
measurement was turn-window *encouragement*: a bonus for activating, worth
+0.086 at T3 and +0.147 at T6, and null because a move that scores zero cannot
be rescued by a bonus that is also about zero. Pricing the state change is a
different intervention and it is the one v7 was already doing.

The decision-level probe says the remaining disagreement profile should now be
re-measured: 8 of 14 traced disagreements were an ability v8 never took, and
that class should be largely gone. `V8_PROBE_V7` is the instrument for checking
it, and re-running it is the next step rather than another term.

**Holdout.** n=4000, seed 23, never read before this point:

| seed | baseline | +`V8_ABILITY_VALUE=1` | paired p | pace (P1) |
|---|---|---|---|---|
| 11 (selection) | 42.5% | 43.5% (+1.00) | **0.01424** | +0.0044 sig |
| 23 (**holdout**) | 41.8% | 42.5% (+0.65) | 0.11511 | +0.0035 sig |

Pooled across 8000 games: 220 baseline-only against 286 candidate-only, McNemar
p ≈ **0.0034**. Same sign on both seeds, significant on the selection seed and
on the pooled, and the holdout's own pace interval excludes zero. On the holdout
the *opponent's* pace also falls significantly (-0.0023) — v7 places less
against this v8, which is the first time any v8 change has moved the other
bot's behaviour rather than only its own.

---

## 1. The instrument came first, and it was the precondition for everything

`bot_arena` emitted a bare win/loss tally. It now emits per-game outcomes
(`--outcomes`) and runs a **paired** test against a baseline (`--vs`).

The deal for game N is a pure function of `(--seed, N)`, so two runs on one seed
face identical shuffles. That makes the comparison paired, and only the
discordant games carry information — a far more sensitive test than two
independent tallies, which is what the project had been reasoning with.

| addition | why |
|---|---|
| `--outcomes PATH` | per-game log so runs can be joined game-for-game |
| `--vs PATH` | exact McNemar, Wilson interval, paired bootstrap on pace |
| `--deck2 NAME` | seat 2 may use a different decklist (see §5) |
| `--games` required for `--vs` | a wall-clock run's `n` differs every time and cannot pair |
| missing deck = hard error | it used to fall back to a synthesized list, so a typo measured a different game |
| `V8_*` in `policy_environment` | a gated v8 row used to be indistinguishable from a stock one |
| placements/live-phase, game length, dev curve | the guide's own health metrics (§3) |

Verified determinism: the same seed replayed byte-identically, 500/500 games
joined, 0 discordant.

## 2. v8 was losing, and nobody knew

v8 was undocumented and unbenchmarked. First real numbers, 400 games/seat:

| config | v8 |
|---|---|
| v8 as originally written | **38.8%** (930/2400, 3 seeds x both seats) |

Component ablation (400 games, v8 as P1) localized it:

| config | decisive% | live-fold% |
|---|---:|---:|
| stock v8 | 37.7% | 7.0% |
| v7 Main + v8 live | 45.5% | 5.2% |
| v8 Main + v7 live | 35.2% | 1.7% |
| v8 mulligan → v4 | 37.2% | 7.4% |
| v7 Main + v7 live | 47.7% | 0.3% |

v8's **Main** phase was the whole regression. Mulligan: neutral.

## 3. The guide's own health metrics, which no one was measuring

Now reported every run, from §1 of the doctrine:

- **placements per live phase**, target ~1.0 from T2; §1 calls ≤0.33 "a defect,
  not an archetype"
- **median game length**, target T5–T8, hard band 5–9
- **average stage cost per turn**, the 4 → 9 → 13 curve

Both bots place **~0.4 per live phase against a ~1.0 target**. Game length is
healthy (median T6). So games are not dragging — checks are not converting, and
that factor-of-two gap on the metric the doctrine calls the bottleneck is the
largest single number in this project. It was invisible while the arena only
reported win rate.

## 4. The Main phase: two wrong hypotheses, one right one

### Wrong: "the one-check leaf is too myopic"

A two-horizon leaf (T+1 and T+3, priced by forward development) was implemented
and measured: **p = 0.45–1.0 at every weight.** Dead.

What killed it was the development curve. v8's Main and v7's Main reach an
**identical** stage-cost profile — entering T4 11.7 vs 12.2, T7 24.5 vs 25.9,
final 20.1 vs 21.4 — both close to the guide. The leaf was never short of
horizon. The term is retained as an ablation defaulting to 0, documented as
measured-off rather than shipped on the argument that it must help.

### Right: cost is the wrong proxy for what a check reads

A check reads **hearts and blades** (§3.2). Cost only stands in for them, and on
a real decklist a high-cost low-heart member scores the same as a high-cost
high-heart one. The development curve is identical while the outcomes are not,
which is exactly what a bad proxy looks like.

The leaf now carries `hearts + Binomial(active blades, own density)` — §4's "the
real scoreboard" — expressed in score-band units by interpolating the guide's
own band medians (`band_progress`). No conversion constant between hearts and
probability is invented.

| config | v8 win rate (2400 games, 3 seeds, both seats) |
|---|---|
| v8 as originally written | 38.8% (930/2400) |
| **v8 + band term** | **44.2% (1060/2400)** |
| v7 Main + v8 live | 49.3% (1184/2400) |
| v7 | 55.8% |

+5.4pp, ~3.8σ. Removing the term costs 27 wins over 700 games plus a
significant −0.018 placements-per-live-phase. The weight is flat from 2 to 16,
so the term being present is what matters, not its value.

## 5. The mulligan was replacing the OPPONENT's cards

Found by playing a side manually against v7, not by any automated measurement.

`handle_mulligan_confirmation` advanced the mulligan phase **before** mutating
a hand, so `active_player_mut()` had already flipped to the other seat. A
player's own mulligan removed the selected cards from their **opponent's** hand
and dealt the replacements to the opponent.

```text
my_hand  [2547, 2563, 2573, 2576, 2537, 2533]  ->  unchanged
opp_hand [2651, 2611, 2599, 2615, 2639, 2636]  ->  [2651, 2611, 2639, 2636, 2648, 2609]
```

Fixed with `GameState::mulligan_owner_index()`, captured before the phase
advances; selection now resolves against the same owner; an explicitly empty
`card_indices` no longer discards the server-side selection. Five unit tests in
`engine/src/turn/mulligan_tests.rs`, the load-bearing one asserting **the
opponent's hand is untouched**.

**Why the arena never caught it.** The ablation recorded v8's mulligan as
"neutral". That is not evidence the choice doesn't matter — it is what a broken
mulligan looks like in a **mirror**: each seat corrupts the opponent, the two
cancel, and the measurement reports nothing. The one harness used for every
decision in this project is the one configuration in which this class of bug is
invisible.

**Effect on the numbers**: 43.0% with a working mulligan vs 43.9% with the
broken one — within noise, exactly as cancellation predicts. The fix does not
move the v8-vs-v7 number, and that is the point: the number was measuring
cancellation, not strength.

**Consequence, and the most important open item:** every conclusion in this
project was measured in a mirror. A change that helps one seat and hurts the
other is invisible there, which describes most of §9.2's failures. `--deck2`
now exists, and the first asymmetric numbers are:

| matchup | v8 |
|---|---|
| 5CP3Z idou vs aiscream 37PMZ | 41.9% |
| 5CP3Z idou vs aqours_cup | 43.8% |
| aqours_cup vs aiscream 37PMZ | 43.8% |
| *mirror, for reference* | *43.0%* |

So the v8 deficit survives de-mirroring. It was not an artifact — which is a
real result, and it is the first non-mirrored measurement this project has.

## 6. Playing the side: `human_vs_v7`

`engine/src/bin/human_vs_v7` plays one seat against v7, deterministically and
replayably: I record choices in a script, the harness prints the pending
decision and stops, and the run resumes from the top. Alongside each decision it
shows **v7's own `score_actions` ranking** and what v8 would have played, so a
divergence can be read as a number rather than an opinion.

It found the mulligan bug, and it found these:

- **T1:** v8 plays a cost-2 body where v7's searched eval scores the cost-5 at
  **twice** the value (50.00 @ depth 2 vs 25.00).
- **T2:** v8 batons into an occupied slot when the free slot reaches the guide's
  T2 = 9 exactly (v8 reaches 7 with fewer blades).
- **Live set:** 133 divergences in 30 games where v8 keeps selecting lives v7
  declines — v8 sets more and places fewer. The D2b junk-set defect, in the
  phase that decides placements.

`--diverge N` automates that comparison across N games: 30 games, 1185
decisions, 57.4% divergence, with a per-phase breakdown. It reported nothing at
all for a long time because three stacked bugs meant **no game ever started**
(the Main-phase chooser was being used for mulligan and live-set, RPS never
resolves headlessly unless the gesture changes, and a repeat guard was
converting the stall into a clean-looking `0/25`). Worth recording: a tool that
reports zero is worse than no tool, because it looks like a result.

### Which life — and the finding under it

A `ConfirmLiveSet` vs `SelectLive` disagreement counts a *sequencing* difference
in the search, not a different choice, so the tool was extended to record, at
every `SelectLiveCard` step, which life each bot actually picked. Over 25 games,
70 life-picking decisions:

**v7 and v8 chose the same life only 21.4% of the time.**

The per-card split is where it gets interesting:

| life | v7 | v8 |
|---|---:|---:|
| MIRACLE NE | 12 | **0** |
| 始㝾り㝯坛㝮空 | 4 | **0** |
| Aspire | 17 | 5 |
| 葉月 杋 | 1 | 12 |
| 嵝 坃砂都 | 0 | 9 |

Two cards v7 sets that v8 **never** sets, and two v8 sets that v7 never does.

**First hypothesis, refuted.** The obvious suspect is the `has_unpassable_icon`
filter — a live with a Draw or Score requirement is excluded from candidates
*and* from junk, so v8 can neither set nor discard it. Instrumented
(`V8_TRACE_FILTERED`) across 8 games: **zero** lives filtered. Not the cause.

**What survives, and it is the abilities problem.** Every card on v7's side of
that table carries a `{{live_start}}` ability; the ones v8 favours are plain.
v8's live set can only see a flat `0.5 * PLACEMENT_CREDIT` for a non-empty
ability string, and only in the junk filter — it has no way to know what the
ability does. So it is structurally biased toward cards with nothing on them,
which is exactly what the counts show. v7 sees more, because v7's live set runs
a rollout that lets the engine execute the live start for real.

The guide already prescribes the fix and lists it as an open gap: **project
deterministic live-start effects through the engine** before scoring a
candidate, rather than guessing at them. That is the next thing to build. It is
not started, because it needs the samples it runs on a re-run to be validated,
and that validation does not fit in the time left.

## 7. The one change that actually worked: set ONE life

Everything above this line was infrastructure, a bug fix, and one leaf term.
The first change to come out of *playing* the side and diffing v8 against v7
that survived its own significance test is also the simplest thing in this
document.

The divergence tool found the largest class of disagreement (133 of them in 30
games) was in the live-set phase: v8 continuing to select lives after v7 had
stopped and confirmed. v8 sets more and places fewer — the D2b junk-set defect,
sitting in the phase that actually decides placements.

The obvious fix, re-shaping the burn term, was a byte-identical no-op (§8). The
wrong fix is to restrict the search space, and the guide says why:

> 8.3.15 pools the requirements of every life in a zone, so a second life does
> not add an independent chance of placing — it **adds its requirement** to the
> first. 8.4.7 pays exactly one placement per won check.

A wider zone therefore adds cost with no upside to balance it. And v8's argmax
was free to discover that, because the wider options *were* enumerated — it just
preferred them, presumably because a bigger pooled need reads as "more swing".

Restricting v8 to a single life:

| config | v8 (800 paired games, seed 11) |
|---|---|
| up to 3 lives (previous default) | 349 |
| up to 2 lives | 352 (p = 0.25) |
| **exactly 1 life** | **364 — p = 0.0026** |

Over 2400 games (3 seeds, both seats): **43.0% → 44.2%**.

This is the first change in the whole effort to clear p < 0.05, and the first
one whose win rate is explained by a rule rather than by a tuned weight. Note
what was *not* needed: a new constant, a new term, or any change to the value
function. The existing argmax was already correct about the values and wrong
about which options it was allowed to consider.

`V8_MAX_LIVES` overrides the uncontested cap.

### The refinement that failed, and the confirmation that explained it

Two follow-ups, both instructive:

**"Exempt contested checks" — refused by the data.** 8.4.7 pays one placement
per won check *regardless of margin*, but score only matters inside a
comparison, so a second life ought to earn its added requirement when the check
is contested and the extra score tips it. Implemented: allow two lives when
`contested_masses` reports mass on both sides. It measured **354 vs 364 wins,
p = 0.052 — worse**, not better. The score a second life can win is worth less
than the requirement it adds, which is the same 8.3.15-versus-8.4.7 balance and
the same answer at every width. Kept behind `V8_CONTESTED_WIDE`, off by
default, with the reasoning recorded so it is not re-derived as if it were
untried.

**Junk slots are the opposite case, and confirming that made the shape legible.**
`V8_NO_JUNK` (never spend a spare slot on a junk card to draw a replacement,
8.3.4) costs **61 wins, p = 0.00003**, and drops pace 0.4245 → 0.3803.

Put together, the winning zone is **one life plus junk slots**, and the reason
the two halves behave oppositely is the rule they consume differently:

| slot | consumes | therefore |
|---|---|---|
| a life | adds its requirement to the zone's pool (8.3.15) | narrow — one is right |
| junk | consumes no requirement; discards a card and draws (8.3.4) | wide — fill every spare slot |

So v8 was not wrong about wanting more from a zone; it was spending the wrong
resource. Every extra life made the zone harder to pass, and every extra junk
slot made the hand better. The fix was not "set less", it was "stop paying for
swing in the currency that a check does not reward".

## 8. Dead ends — do not retry

Every one implemented, measured with the paired test, and removed or defaulted
off. All are monotone functions of the same post-action board quantities
`band_progress` already reads, so they rank actions identically and no weight
reorders the argmax. **You cannot improve a leaf that is already a function of
(stage cost, hearts, blades) by adding more terms to it.**

| attempt | p |
|---|---|
| two-horizon leaf (T+1, T+3) | 0.45–1.0 |
| `score_ceiling` (integer band) | superseded |
| `passable_count` (v7's `60 x Δpassable`) | > 0.62 |
| v7's eval ported as a level term | 1.0, identical at weights 0.5–5.0 |
| Main search 32 → 64 → 128 nodes | 1.0 |
| follow-up breadth / width | 0.69 |
| live-set samples 192 → 2048 | 0.63 |
| burn term re-shaped via `p_later` | 1.0, byte-identical |
| taking v7's turn-order choice | 1.0, byte-identical (arena does not route that phase through the patched entry) |

Two of these are worth reading twice. The **v7 eval port** is the informative
one: v7's Main is the best-measured in the project, so copying its four terms
and its depth and its breadth into v8 produced *nothing*, while v7's Main still
won 5pp. The advantage is not any term and not depth — it is that v7 scores a
**delta** and re-evaluates after each simulated action, so it sees the joint
effect of a sequence, where v8 scores an absolute **level** after one action.
That is also why 81.8% of v8's decisions originally tied at the top. Fixing it
means re-entering the placement model at every search node, not at the leaf
alone, and that is the one untried structural change.

The **burn term** deserves a correction: `(1 - p) * p` is zero at `p = 1`
(correct — a life certain to pass is not at risk) and also zero at `p = 0`
(**not** correct — a life certain to fail is the most expensive thing you can
commit). The code comment claiming it "cancels to zero only when the life can
never pass" was false. The shape is still wrong; every replacement tried has
also been inert, so it is recorded rather than papered over.

## 9. Where it stands

| | v8 |
|---|---|
| as originally written | 38.8% |
| **current** | **43.0%** (mirror), **41.9–43.8%** (asymmetric) |
| v7 | 55.8% |

Tests: 88 lib + 3617 integration, green.

**v8 is significantly better than it was and still behind v7.** v8 started at 38.8%; it is now 44.2%, with the live-set width fix the only change to clear p < 0.05. The remaining
gap is not a constant and not a term — v7's eval, depth and breadth were all
reproduced inside v8 and all were inert. What is left:

1. **Re-enter the placement model at every search node** instead of only at the
   leaf. The one structural change the evidence points at.
2. **Pace is 0.4 against a 1.0 target.** The largest number in the project.
3. **The live set sets lives v7 declines** (133 divergences in 30 games).
4. **Measure everything asymmetrically from now on.** A mirror hid a
   hand-corrupting engine bug for the project's entire life.

---

## 10. The twenty, and what working from v9 did to the list

The productive direction stopped being new terms and became **transcribing what
v7 actually does**, plus the rules nobody implements: eleven leaf terms of v8's
own measured inert or worse, and the two that worked were both transcriptions
read out of `v7_main.rs`.

**Working from v9 changes what the list is for.** v9 already contains v7's
Main, so the thirteen v7-term transcriptions are redundant for v9 by
construction — the ability fix is in v9 for free. The only version of the list
that can move v9 is the one aimed at v9's own surface: its live set (v8's) and
its mulligan (v7's). Four cells, paired, n=1200, seed 11, v9 baseline 57.3%:

| cell | result | p |
|---|---|---|
| `V8_ZONE_ORDER` (rule 8.3.15) | 57.3% | 1.000 — **0 discordant** |
| `V8_ZONE_ORDER` + `V8_MAX_LIVES=3` | 57.3% | 1.000 — **0 discordant** |
| `V8_NO_JUNK` | 42.2% (**-15.08**) | 0.00000 |
| `V9_MULLIGAN=v8` + `V8_MULL_BH=2` | 51.5% (-5.83) | 0.00165 |

**Zone-order allocation is correct and has no purchase.** The engine resolves a
zone in order with surplus feeding the next card; `pass_probability` pools
them. The fix is right, and it is bit-identical even when the zone is explicitly
allowed three cards. So the live set never commits two lives, ever, at any width
it is given — the ammunition floor and the "prefer fewer lives" tiebreak settle
it before width matters. That is a fact about the live set rather than about the
model, and it means 8.3.15 stays a dormant correctness fix until something ever
commits width 2. It also reads the doc's "live zone always exactly 1" from the
other side: not "width is effectively 1" but "width is 1 and no setting moves it".

**Junk fill is worth 15 points** — the largest verified mechanism in v9.
`V8_NO_JUNK` costs 15.08 pp and hands the pace straight to the opponent
(+0.042). Filling spare zone slots with the weakest cards available is not a
tiebreak convenience; it is most of what the live set does.

**The forward-walk mulligan is worse from v9 too** (-5.83, against -3.4 from
v8). Replicated on a different base bot, which is the strongest evidence in this
file that it is the walk itself and not the context v8 put it in.

---

## 11. The v7 term vector: +5.40, and a probe-derived rule that cost 20

### 11.1 The whole vector at once

`v7_term_score` drops v7's entire linear-delta feature vector into v8's leaf,
each of the thirteen terms behind its own `V8_TERM_*` switch, defaulting to v7's
own weight. Paired, n=4000, seed 11, v8 baseline 42.5%:

| config | win rate | p | pace (P1) | pace (P2) |
|---|---|---|---|---|
| baseline | 42.5% | — | 0.3735 | 0.4378 |
| **all 13 v7 terms** | **47.9% (+5.40)** | **0.00000** | +0.0229 sig | -0.0137 sig |
| all 13 + `V8_ABILITY_VALUE=1` | 47.7% (+5.22) | 0.00000 | +0.0219 sig | -0.0134 sig |

**The largest single improvement to v8 in the project**, and it is the whole
vector or nothing: the two changes that worked individually (§3.2 blades +0.88,
§3.8 abilities +1.00) are both inside it, and the ability term adds nothing on
top — they are the same signal seen at two resolutions.

This is `V7_LEAN` vindicated from the other side. Zeroing v7's seven minor
terms together costs it 14 points while each is worth under 1.0 alone, and the
log's own note is that single-term ablation measures dominance rather than
dispensability. The eleven v8 attempts each added ONE term to a leaf whose
*shape* was wrong; this replaces the shape. The gap to v7 goes from ~10 points to
~4.

**Not yet shipped.** The holdout run failed on a stale `b23big.csv` and was not
re-run, so this is a **selection-seed result and unconfirmed**. It is the
largest effect in the file and the one most likely to be partly fitted to seed
11. A fresh seed-23 baseline and a re-run is required before any of these terms
go near a default.

### 11.1b Confirmed on a holdout seed

Re-captured a fresh seed-23 baseline and re-ran. n=4000 per seed.

| seed | baseline | + all 13 v7 terms | paired p | pace (P1) | pace (P2) |
|---|---|---|---|---|---|
| 11 (selection) | 42.5% | **47.9% (+5.40)** | **0.00000** | +0.0229 sig | -0.0137 sig |
| 23 (**holdout**) | 41.8% | **47.5% (+5.67)** | **0.00000** | +0.0220 sig | -0.0163 sig |

**+5.40 and +5.67 on two seeds that share no shuffles**, with every health
interval in the same direction. This is the first result in the file that clears
the bar the blades sweep failed (§1.4: shallow peak on two seeds, p=0.944 at
n=6000) and it is the one worth building on. v8's deficit to v7 goes from ~10
points to ~4.5.

The opponent's pace falling significantly on **both** seeds is the strongest
statement available: v7 is placing *less* against this v8, so the gain is not
v8 getting lucky in a few games but v7 being genuinely outplayed into worse
boards.

### 11.3 The next experiment, and why the earlier null does not cover it

`V8_CHAIN` measured null (§2.7) and the diagnosis was the leaf's shape: on a
saturated LEVEL every node already contains everything before it, so the max is
nearly the level and no step ever "strictly improves" — which is why depth 3 and
depth 6 returned bit-identical results.

`v7_term_score` is the opposite kind of quantity. A linear function of coarse
deltas **composes**: the score of a state is the sum of what got it there, so a
node measured against the **decision root** rather than against its parent
carries the value of the whole line. That is the property a backup needs, and it
is the one v7's depth-8 chain has and v8's single breadth step does not.

So `V8_CHAIN` **with the term vector, scored root-relative** is a combination
the earlier null does not cover: the value function is the new part, and the
search is what the value function makes possible. Written up and left unwired —
`v7_term_score` is declared below the `impl Search` block in a file whose brace
nesting took several edits this session, and it was not worth fighting the
compiler while §11.1 was still unconfirmed.

This is the one remaining idea in the file that is both grounded in a measured
result and untested.

### 11.4 v10, and the honest bottom line

`v10` = v8's Main **with the term vector on** + v8's live set + v7's mulligan. It
exists because the two best components were in different bots and neither
combination obviously dominated.

`bot_arena v10 v9 4000 "5CP3Z idou" --games 4000 --seed 11 --env V8_ALL_TERMS`
→ **1859-2141 (46.5%)**.

**v9 is still the stronger bot.** The term vector is a large, replicated,
confirmed improvement to v8's Main phase — the first in the file — and it is
still ~7 points behind v7's Main phase in a head-to-head. So the honest summary
is:

| bot | composition | vs v7 |
|---|---|---|
| v8 | v8 Main (level leaf) | 42.5% |
| v8 + terms | v8 Main + v7 delta vector | 47.9% / 47.5% holdout |
| **v9** | **v7 Main + v8 live** | **57.3% / 57.6% holdout** |
| v10 | v8 Main + v7 delta vector + v8 live | 46.5% vs v9 |

**v7's Main phase is still the best in the project, and the +5.4 does not change
that.** What the term vector established is narrower and still worth having: the
eleven prior failures were shape failures, not weight failures, and roughly half
of the deficit is now closed by transcribing what v7 already does. The other
half is the depth-8 chain against v8's single breadth step — which §11.3 says
is now worth retrying with a delta leaf, and which is the one lead in this file
with a measured precedent and no measurement.

### 11.5 A performance bug the experiment found

`v7_term_score` originally read the environment on every call and built each key
with `format!` — thirteen `String` allocations per action evaluation, inside the
search node loop, so roughly 13 x 10 x 96 per decision. It was correct, and it
made `v10` so slow the arena could not finish 4000 games in reasonable time.

The weights are now read once into a `OnceLock<V7TermWeights>`. The environment
is fixed for the process (the arena sets it before any game and never changes
it), so this is not a staleness risk, and `OnceLock` is the only thread-safe
option here because a policy call can run on a worker.

Worth recording on its own: a term that cannot be evaluated inside a search
should not be put in the search, and the symptom was a timeout rather than a
wrong number, which is a much less obvious failure than a wrong number is.

### 11.6 The chain is null too, and that is the last of it

Wired at last, via `v8_model::v7_delta_score` (see §11.5 for why the scorer had
to move modules). Chain depth 4, node budget 600, term vector on:

```text
bot_arena v8 v7 4000 "5CP3Z idou" --games 4000 --seed 11 --vs b11big.csv \
  --env V8_ALL_TERMS --env V8_CHAIN_ROOT --env V8_CHAIN=4 --env V8_NODES=600
  RESULT v8 vs v7 | 1921 - 2079 | 0 draws
  DISCORDANT: baseline-only 275 | candidate-only 496   p = 0.00000
  pace (P1) 0.3735 -> 0.3969  +0.0234  SIGNIFICANT
  pace (P2) 0.4378 -> 0.4237  -0.0141  SIGNIFICANT
  42.5% -> 48.0%  (+5.53)
```

**+5.53 against +5.40 for the term vector alone — a 0.13 difference.** The chain
does nothing, and §2.7's diagnosis of *why* it did nothing was wrong.

The diagnosis was "a saturated level does not compose, so no step ever
improves". Making the leaf compose too (root-relative, `V8_CHAIN_ROOT`) should
have fixed exactly that, and it changed nothing. So the real reason is not the
leaf's shape.

**Everything tried against v8's Main phase, in one place:**

| axis | result |
|---|---|
| eleven leaf terms | inert or regression |
| colour-exact shortfall | regression |
| opponent horizon, then + a gradient | bit-identical |
| opponent density calibration | 5 games in 1200 |
| tie-break, alone and with a quantised leaf | 23/1200, then null |
| deeper nodes / wider follow-ups | 3/400 |
| ISMCTS replacing the leaf | -6.6 |
| engine-executed live set | -7.8 |
| forward-walk mulligan | -3.4, then -5.8 |
| greedy chain, level leaf | null, bit-identical at depth 3 and 6 |
| greedy chain, **delta** leaf, root-relative | **+0.13** |
| **v7's whole delta vector** | **+5.40 / +5.67** |

One intervention has ever worked, and it was not a search and not a term — it
was giving v8 **the value function v7 already had**, all thirteen terms at once.
The remaining ~4.5 points are not in the leaf's shape, the search, the depth, the
granularity, or the opponent model. They are in something about the content of
v7's `Search::complete` — which differs from `with_followup` in that it re-offers
after **every** action to a depth-8 node budget, and v8 still evaluates each
root action with a single breadth step.

That is a real, located, unmeasured lead. It is not a leaf term and it is not a
depth change; it is the difference between "score each action once" and "score
each action and then actually play it out". Closing it means making the ROOT
expansion a backup rather than a breadth step, which is a larger change than
this file's current structure makes cheap.

### 11.7 Three things checked and ruled out, and one that stands

**The live zone does NOT accumulate.** The obvious hypothesis from the 15-point
junk result was that the zone is a growing portfolio re-checked every turn, and
that v8's live set — which prices only the newly-set subset — was modelling a
fraction of the payoff. Read the engine: `phases.rs:826` pushes into
`live_card_zone` and never clears it during a phase, but
`move_live_to_success_and_handle_wins` (`live.rs:1644`) sends every resolved
card on — winners to the success zone, failures to the **waitroom**. The zone
empties each turn. Hypothesis dead, and worth having killed before building on
it. It is also why junk fill is worth 15 points: a junk card that passes is
still a success toward three, and `live.rs:474-486` awards the live outright if
one player places and the other places nothing, regardless of score.

**The mulligan's missing term is not convertibility.** The competitive write-up
is explicit that cards with no blade-heart are the central resource, and
`V8_MULL_BH` implements exactly that as a penalty on the post-mulligan deck. On
v9, n=1200, seed 11, against v9's 57.3%:

| mulligan | result |
|---|---|
| v7's curve rule (shipped) | **57.3%** |
| v8's forward walk | 53.5% (-3.8) |
| v8's forward walk + `V8_MULL_BH=1` | 51.5% (-5.8) |

The budget makes it **worse**, not better. So either the advice does not
transfer to this list through this mechanism, or penalising no-blade-heart
cards in the opening is the wrong way to spend the budget — a card kept in hand
is paid for on a specific turn, not in aggregate. The point stands that nothing
in `bot/` counts these cards; the point does **not** stand that counting them
and penalising them is the fix.

**The forward-walk mulligan is worse, twice, on two base bots** (-3.4 from v8,
-3.8 from v9, replicated across different live sets and therefore not a context
effect). Whatever it is optimising, it is not the objective.

**The one thing that still stands.** The mulligan is the highest-leverage
decision in the game — five cards, once, the deck fully known, the game
deterministic given both policies — and it is decided by a static cost-ladder
rule. A full-game rollout is the correct evaluation and the machinery already
exists: `v7_ismcts::play_out` drives a real game forward with `auto_advance_one`
and `compact` to hide hidden state. Both proxy attempts at "better mulligan
maths" failed, which is itself the argument for this one: a proxy of the
objective failed twice, and the objective is exactly computable here.

### 11.8 The zone's full width: a real bug, correctly diagnosed, and the fix is a no-op

`check_outcome` (`v8_model.rs:929-932`) credits **exactly one** success in every
branch — `rule_payoff(our_success + 1, opp.success)` — and takes `p_pass` as the
probability that the *whole set* is satisfied. So a three-life set is modelled as
"one success, with lower probability than one good life", which is why the live
set never commits more than one card.

That premise was wrong, and �27.4 is where it was caught. The engine does **not**
check and move the zone per card: rule 8.3.16 / Q35 is all-or-nothing, so a set
containing one unsatisfied card places **nothing** and every card in the zone goes
to the waitroom (`determine_winners` requires `lives.iter().all(|l| l.passed)`;
pinned by `live_success_rules_test.rs::any_card_fails_hearts_all_fail`). There is
no state in this game in which three lives bank three successes in one phase, so
the "model cannot price that" argument has no referent.

`V8_JOINT` prices it: `zone_success_distribution` resolves a set one card at a
time against a shared supply and returns `P(k successes)`, and
`joint_success_value` pays for the 2nd and 3rd successes with the same race
payoff. As a fix to a real defect it is a fix to a defect that is not there.

**It changes nothing: 0 discordant games out of 1200, with `V8_JOINT`, again
with `V8_JOINT + V8_MAX_LIVES=3 + V8_ZONE_ORDER`, and again with width alone.**

The zero is the rule, not a tuning result. `P(k >= 2)` is **identically zero**
whenever the set contains a card the board cannot cover, because that is what
"all-or-nothing" means, and the live set is scored on a value function whose
contested branch is a comparison of printed scores. A second explanation in an
earlier draft of this section - that hearts come from a shared pool, so two lives
rarely both pass - is also wrong: it is a *sufficient* story, not the mechanism,
and it would be false for a pair of cheap lives on a big board.

So the live set is not leaving anything on the table by committing a single card,
and the reason is a rule rather than a shared pool. The "the zone is underused"
reading was wrong, and it was wrong in a way worth having measured: three separate
mechanisms all landed on the same bit-identical result, which is a much stronger
statement than any one of them alone.

`V8_JOINT` should stay switched off, and now for a third reason: its
justification asserts a resolution rule the engine does not implement, in the
direction that makes the switch look like a missing capability.

---

## 12. Reading the deck the bots actually play

Everything above was measured against `5CP3Z idou`. Nobody had read its cards.
Read straight from `cards/cards.json` (17 distinct members, 4 lives, 12 energy).

### 12.1 The energy model was wrong in this session's own analysis

Two corrections, both verified in the code rather than recalled:

- **Baton-touching is real and makes high cost payable early.** The rule: a
  member appears by paying energy **or** by sending an already-appeared member
  to the waiting room to make up the difference. So a cost-9 is playable on turn
  2 by baton-touching a turn-1 cost-4. An earlier note in this session modelled
  the curve as "+1 per turn on 3 setup, so cost-9 is dead until T6" — wrong, and
  it would have justified binnable cards that are fine.
- **Once-per-turn is a condition, not a trigger.** `TriggerKind` is exactly the
  seven printed trigger words (`triggers.rs:40`): 起動 / 自動 / 常時 / 登場 /
  ライブ開始時 / ライブ戝功時 / メイン. ターン1回 is a condition, enforced by
  `turn_limited_abilities_used`, which is cleared at the **end of the live
  phase** (`phases.rs:278`) — so the window spans the whole turn including live.
  Per-*effect*, not per-card: two members granting the same thing pay out once.

### 12.2 The deck's three lives are all live-start, and all keyed to movement

- `Special Color` `PL!SP-bp4-025-L` x4, score 2, need **7 colorless**.
  ライブ開始時: centre Liella members' **printed blade count becomes 3**.
  ライブ戝功時: +1 if a centre Liella member moved this turn.
- `MIRACLE NEW STORY` `PL!SP-bp5-024-L` x1, score 7 +1, need **15**
  (heart01x2 + heart02x4 + heart06x4 + colorlessx5).
  ライブ開始時: choose one of heart01/02/06; **every member that moved this turn
  gains that heart**.
- `始㝾り㝯坛㝮空` `PL!SP-sd2-023-sd2` x3, score 1 +1, need 3
  (heart03 + colorlessx2). ライブ開始時: **if own success zone has 2+ cards,
  score +5 and the need becomes 12 specific hearts**. The FAQ confirms the
  trigger **cannot be declined** — the FAQ answer is 㝄㝄㝈, it cannot be used
  with the printed 3-heart requirement.
- `Aspire` `PL!SP-sd2-025-sd2` x4, score 4 +1 draw, need **11**.
  ライブ開始時: **every Liella member that moved this turn gains 1 blade**.

So the deck's spine is: get members **moving**, then set a live whose
live-start converts those moves into blades or hearts. "Moved this turn" is not
a detail — it is the condition every one of these cards keys on.

### 12.3 The gap, stated from the card text

`begins_here` is the sharpest case in the project. Its requirement is **3 hearts
by turn 1** and it is *worth setting early* — but at 2 successes the same card
becomes a 12-heart check worth the same +5. The correct play is therefore
**state-dependent in a way no current model can express**: cheap and always
right at 0-0, and either a different card or a deliberate fold at 2+.

The bot reads `need_heart` as a flat vector and `rule_payoff(our_success + 1,
opp.success)` as a flat +1. Both are wrong for this card, in opposite
directions, and the two errors partly cancel into something that looks
reasonable.

### 12.4 Choice handling — where the cheap wins are

Every one of these abilities becomes a **pending choice**, and pending choices
are answered by the same Main-phase leaf (`v8_main.rs:1045`). What that leaf
reads, for this deck:

| ability | what the leaf can see | verdict |
|---|---|---|
| `葉月 杋` cost 2 登場: top 5, reveal 1 Liella card | hand **count** +1, identical for every option | **coin flip** |
| `米女メイ` cost 5 登場: top 5, reveal 1 Liella member | same | **coin flip** |
| MIRACLE ライブ開始時: choose heart01/02/06 | `supply_hearts` sums all buckets | **coin flip** |
| `冬毬` cost 11 登場: pick 2 waitroom lives, **opponent picks 1** | `lives_in_hand` +1 | **arbitrary**, and the correct pick is 2 *equally mediocre* lives so the opponent's choice costs least |

The third row is the one I had asserted was fine and is not: a colour choice
whose entire purpose is colour, priced by a **colour-blind sum**. The first two
are the general case — a tutor whose job is to fetch a specific card, answered
by a leaf that reads how many cards you have rather than which.

`V8_HAND_QUALITY` is the first fix: it is the only term in the leaf that reads
the hand's **contents** rather than the board, which is the only reason it can
separate candidates every board term proves equal. Members are worth their
printed cost (the energy curve has to pay for it, and a body unaffordable for
three turns is a body held instead of one that is playable); lives are worth
their printed requirement, capped, because a requirement the board can never
cover is worth nothing and a linear term would reward exactly that.

### 12.5 Measured at one weight, and it is the wrong weight

Paired, n=4000, seed 11, on top of the confirmed term vector (baseline 48.0%):

| weight | result | p | pace (P1) | no-contest |
|---|---|---|---|---|
| 1.0 | 46.7% (-1.30) | 0.374 | 0.398 -> 0.334 | **77.8%** |

The 77.8% no-contest rate is the diagnosis: a five-card hand of cost-11 bodies
sums to roughly 50, so at weight 1.0 this term is an order of magnitude larger
than every other term in the leaf and the bot ends its Main phase almost
immediately — the identical failure shape as `V8_PASS_TAX` (§1.3) and the
convert gate (§11.2), both of which were also weight failures that looked like
catastrophic concept failures.

**This is not a verdict on the idea.** It is one badly-scaled measurement, and
the scale needed is roughly two orders of magnitude smaller than the one tried.
Sweeping it properly is the obvious next cell and it was not run. Recorded as
unmeasured, not as refuted — the log's own §1.3 warning about a single badly
scaled cell being written up as a result applies directly here.

## 13. The top unbuilt item, and why the earlier null does not cover it

**Project each live-start ability through the engine before pricing the check.**

The earlier reason for skipping this was "100% of lives in this deck carry a
live-start, so there is no contrast" (§0.2). Reading the cards shows that
reasoning was wrong. The contrast is not *whether* a life has a live-start —
every one does — it is **how much this specific live-start adds to this
specific board**, and the four lives differ enormously:

| live | copies | what its live-start does | board it acts on |
|---|---|---|---|
| ` beginnings_here` `PL!SP-sd2-023-sd2` | x3 | rewrites its OWN need to 12 hearts if we hold 2+ successes (cannot be declined, per FAQ) | itself + our success count |
| `Aspire` `PL!SP-sd2-025-sd2` | x4 | +1 blade to every Liella member that moved this turn | up to ~5 members |
| `MIRACLE NEW STORY` `PL!SP-bp5-024-l` | x1 | +1 heart of a chosen colour to every member that moved this turn | up to 5 hearts against a 15-heart need |
| `Special Color` `PL!SP-bp4-025-l` | x4 | centre Liella members' printed blades become 3 | the centre area |

The live set prices a life against the board **as it stands when the zone is
set**, with no projection. For MIRACLE that means pricing a 15-heart requirement
against a board that is about to gain a heart per moved member — which is how a
card the deck runs as its finisher reads as uncashable. `begins_here` is worse:
its requirement is 3 hearts early and 12 late, so the correct play is
state-dependent in a way no current model can express, and the bot reads it flat
in both directions.

"Does this card have a live-start" was the wrong discriminator. "How much does
this live-start raise the pass probability of this board" is the right one, and
no null in this file bears on it.

The engine already has what the projection needs: `TurnEngine::
trigger_live_start_abilities` (`turn/trigger_scan.rs:156`), the movement-event
history that resolves 「㝓㝮ターン中㝫エリアを移動㝗㝟〝 (`state.rs:1135`), and
`RngGuard` (`v8_model.rs`) to keep the projection from perturbing the game.

### 13.1 Built, wired, and measured bit-identically

`project_live_start` (`v8_live.rs`) clones the state, commits the candidate
into the live zone, fires `trigger_live_start_abilities`, answers whatever it
asks with the bot's own `evaluate_actions`, and returns the projected board and
blade count. It is generic over cards — nothing about the four deck lives is
hard-coded — and gated on `V8_LIVE_START_PROJECTION`.

Paired, n=1200, seed 11, on v9 (whose live set is v8's):

```text
RESULT v9 vs v7 | 684 - 516 | 0 draws | 40898 actions | 5.80 turns
DISCORDANT: baseline-only 0 | candidate-only 0    p = 1.00000
pace (P1) 0.4758 -> 0.4758  +0.0000
```

**Zero discordant games and a bit-identical action count.** Not a small effect
below the noise floor — the switch changes no decision in any game, which
means the projection is not doing what it claims to.

**That is a defect in the projection, not a refutation of the idea**, and the
two are not the same thing. A projection that fires and finds nothing would also
be null, so the bit-identical result is consistent with both of:

1. **It is not firing.** `trigger_live_start_abilities(game_state, player_id)`
   takes the player id as a `&str` and the value passed is
   `sim.seat_player(me).id.clone()`. If that string is not what the scanner
   matches on, no ability is ever queued and the board comes back unchanged.
   This is the more likely of the two, because a fired `Aspire` on a board with
   a moved member **must** change `active_blades` and therefore `p_pass`.
2. **It fires and finds nothing eligible** — e.g. the movement history the
   ability reads is not populated at the point the live set is chosen, so
   「㝓㝮ターン中㝫エリアを移動㝗㝟〝 is false for every member and MIRACLE and
   Aspire both resolve to nothing.

Which of the two it is, is not determined, and this log entry does not claim to
know. The two discriminating measurements are: log how many abilities the
projection actually queues, and log the projected blade count against the
un-projected one. Both are one `eprintln!` behind the same switch, and until
one of them is read this switch should be treated as **untested plumbing, not as
a measured null** — the distinction matters, because a measured null would be
evidence against the idea and this is not that.

116/116 lib tests green with the code in place.

### 13.2 It was neither: the scan fires, and the projection threw the result away

Both discriminating measurements from §13.1, behind `V8_LSP_DIAG`:

```text
LSPDIAG lives=1 queued=1 blades 2->2 board [...]   <- BEFORE
LSPDIAG lives=1 queued=1 blades 2->4 board [...]   <- AFTER
LSPDIAG lives=1 queued=1 blades 4->6 board [...]
LSPDIAG lives=1 queued=1 blades 5->7 board [...]
```

`queued=1` on every line, so the scanner always matched a live-start ability —
**hypothesis 1 was wrong and hypothesis 2 was wrong too**. The abilities were
firing; they were resolving to nothing because of the caller.

`trigger_live_start_abilities` only **enqueues**. Resolution is a separate step,
and the real engine takes it on the very next statement after both players'
scans (`phases.rs:245-253`):

```rust
Self::trigger_live_start_abilities(game_state, &first_attacker_id);
Self::trigger_live_start_abilities(game_state, &second_attacker_id);
game_state.process_pending_auto_abilities(&first_attacker_id);   // <- this line
```

`project_live_start` never called it. The entry sat at `PayingCost`, so
`has_pending_choice()` was false, the bounded answer loop broke on iteration 0
having answered nothing, and the function returned the untouched set-time board.
The bit-identical result was a missing call, not a missing effect.

One line, `sim.process_pending_auto_abilities(&me_id)`. The blade count then
moves 2→4, 4→6, 5→7: `Aspire`'s 「㝓㝮ターン中㝫エリアを移動㝗㝟〝 +1 blade per
moved Liella member, resolved through the engine, on real boards. The board pool
does not move, which is correct — these lives grant blades and hearts, not
hearts-in-place, and `board_pool` reads stage hearts only.

**And it is a measured null anyway.** Paired, n=3000, seed 11, on v9 with the
confirmed term vector:

| | result |
|---|---|
| baseline | 1674-1326, **55.8%**, 82930 actions |
| + `V8_LIVE_START_PROJECTION` | 1681-1319, 56.0%, 80626 actions |

```text
DISCORDANT: baseline-only 49 | candidate-only 56    p = 0.55839
pace (P1) 0.4519 -> 0.4519  +0.0000 [-0.0024, +0.0024]
```

49+56 = 105 of 3000 games differ, so the switch *does* change decisions now —
against zero before — but the win rate moves +0.23pp at p=0.56. **This is a real
null on a working mechanism, which is a different and much stronger result than
§13.1's untested plumbing.** Keep it off; the reason is no longer "it does not
fire."

Note what the null does and does not say. It does not say projecting live-start
abilities is wrong. It says that on this deck, pricing Aspire against 2 extra
blades does not change which live set is worth committing, often enough to move
3000 games. The mechanism is right and the lever is small: `+1` blade on a
board that already draws 5-7 changes `p_place` in the fourth significant digit
on most of the 105 discordant games.

### 13.3 The audit: where live phases actually leak

> **RETRACTED IN §14.1.** The pass-rate and shortfall figures below were derived
> from `yell N/Mflips`, which is blade-hearts across yell cards, **not** the heart
> check. Corrected pass rates are 91-100%, not 8-42%, and the "67% one heart
> short" finding is an artefact of the same misreading. Finding 2 (`FOLD/FOLD`
> waste) stands. Read §14.1 before using anything in this section.

Read 289 consecutive live phases out of 60 full games, from the engine's own
`live_result` verdicts, classified by what each side did
(`tools/analysis/live_phase_leak.ps1`):

| | n | share | P1 places | P2 places | per phase |
|---|---|---|---|---|---|
| `SET/SET` | 173 | 59.9% | 100 | 81 | 1.05 |
| `SET/FOLD` | 53 | 18.3% | 50 | 0 | 0.94 |
| `FOLD/SET` | 23 | 8.0% | 0 | **23** | 1.00 |
| `FOLD/FOLD` | 40 | **13.8%** | 0 | 0 | **0.00** |

Three findings, in descending order of size.

**1. `FOLD/SET` is a guaranteed conceded placement, and it is 8% of all phases.**
When v9 folds and v7 sets, v7 placed **23 times out of 23** — and v7 passed its
own check in only 3 of those. Rule 8.4.3.2: setting alone wins the check, so a
fold against a committed opponent concedes the phase outright, whether or not
the opponent's board was good. 13 of those 23 were folded at 0-0, where conceding
a placement is worst. v9 folds 63 times and 23 of them hand over a placement.

**2. `FOLD/FOLD` is 13.8% of phases and produces nothing at all.** 56 of 289
phases (19.4%) yielded zero placements for both sides. In the both-fold case
neither player can win the check, so a full turn of Main-phase development buys
nothing in the live phase. This is the largest single block of waste in the
game, and it is mutual — it is not a v9-specific defect but it is a large shared
one.

**3. Two thirds of failed committed checks are short by 1-2 hearts.** Of 181
phases where v9 committed and failed: 69 (38%) short by 1, 122 (67%) short by 2
or less. That is a *development* shortfall, not a *selection* one. The board was
one member or one blade-icon away from a placement.

The card table makes the same point from the other side:

| card | need | times set | passed |
|---|---|---|---|
| `始㝾り㝯坛㝮空` | 3 | 69 | 29 (42%) |
| `Special Color` | 7 | 59+16 | 10 (13%) |
| `Aspire` | 11 | 62 | 5 (8%) |

The bot commits high-need lives it cannot pass, at a rate close to what it does
with the one life it can. Its own `p_pass` says otherwise (mean 0.75 at commit,
64.6% of commits at p≥0.9), so this is the model and the outcome disagreeing —
see §13.4.

### 13.4 Calibration: the model is honest about *whether* and wrong about *how much*

`V8_CAL` emits one machine-parseable line per live-set decision carrying the
turn, the chosen card, and the **whole candidate table**
(`tools/analysis/calibrate_live.ps1` joins it against the engine's verdict).
971 decisions, P1 = v9:

| model `p_place` | n | realized placement | ratio |
|---|---|---|---|
| ≥ 0.90 | 150 | 84.1% | 0.88 |
| 0.60-0.90 | 353 | 66.6% | 0.86 |
| 0.30-0.60 | 95 | 46.3% | 0.95 |
| < 0.30 | 303 | 2.3% | — |
| **overall** | | | **~0.55** |

Three separate results, and the third is the load-bearing one:

- **The set/fold decision is right.** 287 of the 303 low-`p_place` decisions
  were folds, every one of them at model p < 0.02, and **zero** of them
  realized a placement. The bot folds exactly when it should and never folds a
  check it could have made. The argmax is internally consistent too: across 120
  games there were **0** cases of committing while the best candidate was FOLD,
  and **0** cases of folding while a non-fold candidate was best.
- **The confidence is uniformly inflated ~1.15-1.25x** in the committing range.
  That ratio is close to constant across buckets, which means it is very nearly
  a **monotone transform** — and a monotone transform cannot reorder an argmax.
  Rescaling `p_place` would change no decision and is therefore not the lever.
  This is the same shape as §12.5's badly-scaled `V8_HAND_QUALITY` cell and the
  §1.3 `PASS_TAX` failure: a scale problem dressed as a concept problem.
- **The per-card bias is not uniform, and that part is real.** MIRACLE
  (need 15) is the outlier: n=10, model p_place 0.420. It is priced at 0.42 on
  boards that have not moved enough members for its live-start to pay off, which
  is precisely what §13.2's projection was built to fix — and fixing it moved
  the win rate +0.23pp. So the per-card bias is real, already addressed, and
  too small to matter on n=3000. n=10 is also far too thin to build on.

**Where this leaves the audit.** The live-set *decision procedure* is sound: it
folds correctly, it commits correctly, and its ranking is self-consistent. The
calibration error is monotone and therefore harmless to the argmax. The
projection now works and does not help. What the audit actually points at is not
in the live set at all — it is the 67% of failed checks that are short by 1-2
hearts, which is the Main phase failing to put the last member or the last blade
icon on stage before the check. That is §9's item 1 (re-enter the placement
model at every search node) restated in measured terms: the live set is being
handed boards it cannot use, and the fix is upstream of it.

---

## 14. Hypothesis round: five tests, three refutations, one correction

§13 audited what happens. This section tests *why*, and the first thing it has to
do is retract a claim.

### 14.1 The §13.3 pass-rate table was reading the wrong field

§13.3 reported `始㝾り㝯坛㝮空` passing 42%, `Special Color` 13%, `Aspire` 8%,
and concluded the bot "commits high-need lives it cannot pass." **That is wrong.**

The log line's `yell {hits}/{yell_count}flips` is the number of **blade-hearts
revealed across the N yell cards** (`live.rs:1456-1466` — it sums
`y.blade_hearts` over `snap.yell_cards`). It has nothing to do with whether the
live's heart requirement was met. The authoritative verdict is the engine's own
annotation inside `set[...]`: `Aspire:4pt` when it passed, `Aspire:FAIL(c0 3/7)`
when it did not.

Read correctly (`tools/analysis/live_phase_leak.ps1`):

| card | committed | engine says passed | placed |
|---|---|---|---|
| `Special Color` | 79 | 75 (95%) | 48 (61%) |
| `始㝾り㝯坛㝮空` | 78 | 71 (91%) | 54 (69%) |
| `Aspire` | 66 | 62 (94%) | 46 (70%) |
| `MIRACLE NEW STORY` | 3 | 3 (100%) | 2 (67%) |

**v9's checks pass 91-100%.** It is not committing checks it cannot pass, and
"38% of failures are one heart short" (§13.3) is an artefact of the same
misreading — the readable `FAIL` annotations only cover 15 of the failures, and
of those, 8 are short by 6+. There is no near-miss development story here.

The same correction kills §13.3's `FOLD/SET` framing. It read as "v9 folds, v7
places 23/23 even though v7 passed its own check in only 3" — but **v7 passed
all 23**. Every opponent placement in that bucket was earned. Likewise
8.4.3.2's "passing alone wins the check" was misapplied: in the 12 `SET/SET`
phases where v9's own live failed, v9 placed **0** times. `determine_winners`
requires `lives.iter().all(|l| l.passed)`, so a failed live never places,
sole or contested. §13.3's headline findings 1 and 3 are both void; finding 2
(`FOLD/FOLD` waste, 13.8%) survives because it never depended on the pass field.

**Lesson, and it is the second time this log has paid it:** the engine already
emits a machine-readable verdict per live, and reading a convenient adjacent
number instead of it produced a confident, wrong, rule-citing story. The
`FAIL(...)` annotation is the field. Anything derived from `yell N/Mflips` is
unsafe.

### 14.2 What the audit actually shows: the score comparison is the game

Corrected, of 289 live phases: **148 (51.2%) have both sides passing**, so the
score comparison decides. The margins are razor-thin:

| margin | n | v9 placed |
|---|---|---|
| +1 | 33 | 33 (100%) |
| +2..+3 | 25 | 25 (100%) |
| +4 or more | 6 | 6 (100%) |
| **−1** | **32** | **0 (0%)** |
| −2..−3 | 14 | 0 |
| −4 or worse | 5 | 0 |
| tie | 33 | 23 (70%) |

v9 wins 64, loses 51, ties 33 on the comparison. **A single point of score is
the whole margin in 65 of 148 cases.**

### 14.3 H-A: is score unpriced? No — 4 of 61 declines are defects

If score decides half the game and has no real term in the value function, the
bot should be leaving score on the table. `V8_CAL` emits the full candidate
table, so this is measured on what the argmax actually had
(`tools/analysis/score_decline_audit.ps1`):

- 61 of 387 commits (15.8%) declined a higher-scoring life that was on the table.
- Split by whether the decline was justified on `p_pass`: **A (within 0.10 —
  defect suspects) = 4, B (clearly worse `p_pass`) = 0, C (chose higher
  `p_pass`) = 57.**
- All 4 in bucket A placed anyway, and in each the chosen card had
  `p_place = 1.000` against the declined card's `0.98-0.98`. The argmax took a
  certain placement over a near-certain one. Correct.

`check_outcome` does price score — via `contested_masses`, which compares
`our_score` against a 6-slot opponent distribution and resolves the tie rule.
The suspicion was wrong, and the argmax's own consistency (0 misranks in 120
games) is the reason.

### 14.4 H-B: the seat asymmetry is real and it is the most interesting thing here

`OppModel` reads only `live_card_zone.cards.len()` — the opponent's set **count**,
never its contents. That is correct: 8.2.2 keeps the set face-down until the
performance, so the second attacker sees count only, and `BOT_STRATEGY.md`
§3.1d says so explicitly. I checked the alternative — that v9 should be reading
the observed card — and the rules do not permit it.

But the *decision* is strongly seat-dependent (150 games, `V8_CAL` joined to
verdicts, `tools/analysis/seat_asymmetry.ps1`):

| seat | n | fold rate | modelled `opp_pass` | `opp_set_size` | `p_place` at commit |
|---|---|---|---|---|---|
| second (observed) | 410 | **39.5%** | 0.815 | 1.53 | 0.594 |
| first (unobserved) | 314 | 21.3% | 0.518 | 1.00 | 0.794 |

**v9 folds 2.7x more often when it knows more.** The mechanism is arithmetic,
not informational: seeing a larger opponent set raises modelled `opp_pass`
(0.518 → 0.815), which lowers `p_place` (0.794 → 0.594), which loses the argmax.
The `opp_set_size` → bar → `clears` chain (`v8_model.rs:723-739`) only ever
makes the opponent harder as their set grows.

That is not obviously wrong — a two-card zone really is harder to beat. But it
means the model treats *observing* as *deteriorating*, and the fold decision
inherits that. All 14 sampled folds happened with `best p_place < 0.05` and with
the opponent already set, so the fold itself is coherent; the open question is
whether `clears` is calibrated for the larger zones, which §14.5's evidence says
it is not.

### 14.5 H-C: never fold — a significant REGRESSION

The blunt version of "folding too much", and `V8_NO_FOLD` already implemented it.
Paired, n=1200, seed 11:

| | baseline | `V8_NO_FOLD` | |
|---|---|---|---|
| win rate | 57.0% | **52.2%** | **−4.75pp** |
| `DISCORDANT` | | 173 / 116 | **p = 0.00095** |
| pace (P1) | 0.4758 | 0.4488 | −0.0270 |

**Significant regression.** Folding is load-bearing: when `p_pass ≈ 0`,
committing burns a life (8.3.16) and places nothing, so the resale price is
correct to prefer the fold. The argmax is right, and any "just contest more"
framing is wrong. `V8_NO_FOLD` stays an ablation and must not be shipped.

The `V8_LIFE_RESALE` sweep (the knob that moves the fold threshold continuously)
is monotone and entirely null, which is the same answer in four more points:

| resale | 0.10 | 0.20 | **0.30 (shipped)** | 0.45 | 0.60 |
|---|---|---|---|---|---|
| win rate | 57.3% | 57.2% | **57.0%** | 56.9% | 56.3% |
| p | 0.557 | 0.629 | — | 1.000 | 0.185 |

### 14.6 Where that leaves it

Five hypotheses, and the evidence killed four of them:

| # | hypothesis | verdict |
|---|---|---|
| A | score is unpriced, so the bot leaves score on the table | **refuted** — 4 of 61 declines are defects, all correct |
| B | live-start projection helps | **refuted** — §13.2, +0.23pp, p=0.56 |
| C | v9 commits checks it cannot pass | **refuted** — passes 91-100%, §14.1 |
| D | v9 folds too much | **refuted** — never-fold is −4.75pp, p=0.00095 |
| E | **observing the opponent degrades v9's own estimate** | **live** — fold rate 21.3% → 39.5%, `opp_pass` 0.518 → 0.815 |

E is the only survivor, and it is narrower than "folding too much": it is that
`clears` — the step function deciding whether the opponent's set passes their own
bar — has no evidence behind it for the larger zones v9 can now observe.
`OppModel::horizon_model` already exists to re-evaluate it against sampled pools
and is documented as taking only 6 distinct values across 369 board/blade
combinations; `YIELD_PRIOR` is the one prior in the whole model fitted from
nothing. E says: fit it, and the seat asymmetry should narrow without touching
the fold threshold that §14.5 shows is already correct.

That is a calibration task with a built-in measurement, not another term. It is
also the first candidate in this document that is consistent with **every**
negative result above rather than requiring one of them to be overturned.

### 14.7 E measured: `opp_pass` is not miscalibrated, it is INVERTED and flat

The prediction is that `YIELD_PRIOR` is mis-scaled. It is worse than that.
`tools/analysis/opp_calibration.ps1`, 511 observed opponent sets, bucketed by the
`opp_pass` the model predicted for that exact decision:

| model `opp_pass` | n | model | **realized opponent pass** |
|---|---|---|---|
| 0.00 (modelled fold) | 22 | 0.000 | **86.4%** |
| 0.01-0.39 | 98 | 0.197 | **85.7%** |
| 0.40-0.69 | 231 | 0.527 | **88.3%** |
| 0.70-0.94 | 51 | 0.800 | **86.3%** |
| 0.95-1.00 | 109 | 1.000 | **86.2%** |

**Realized is 86% in every bucket, including the one where the model is certain
they fold.** The quantity is not noisy or merely mis-scaled; it carries
essentially no information about whether the opponent's set passes.

The `opp_set` breakdown shows the same thing through the §14.4 mechanism:

| modelled `opp_set` | n | model `opp_pass` | realized |
|---|---|---|---|
| 1 | 462 | 0.598 | 87.7% |
| 2 | 16 | 0.594 | 75.0% |
| **3** | 33 | **0.156** | **84.8%** |

A three-card zone is modelled at 16% and converts at 85%. That single row is the
seat asymmetry of §14.4 explained: the `target = 2 * set_size` bar only ever
rises, so seeing a bigger set drives `clears` toward zero, and `clears` is not
measuring anything that predicts the outcome.

Checked independently of v9's own decision (all 735 phases, 150 games): **v7
passes 456 of 522 committed checks = 87.4%.** The model's baseline is ~0.60.

**So the single unfitted constant in the model is wrong by ~0.27 in absolute
terms and wrong in the wrong direction**, and its documented rationale is the
reason:

> under-estimating their check ability makes v8 contest more often, and S4 is
> explicit that folding a contestable check is the expensive error

That reasoning is sound and the outcome is inverted anyway: because the
under-estimate is *inverted rather than uniformly low*, it does not bias v9
toward contesting. It makes v9's `p_place` track a signal uncorrelated with
reality, so the argmax is effectively choosing on our own check alone
(`p_pass`, which §14.1 shows is well-calibrated) plus a coin flip.

**Why this is the strongest candidate in the document.** It is the only finding
that explains, with one cause:
- the seat asymmetry (§14.4, fold rate 21.3% → 39.5%) — a rising bar;
- the 15.8% score-declines being harmless (§14.3) — the argmax is not choosing
  on score because the contested term is noise, not because score is cheap;
- `V8_NO_FOLD` regressing (−4.75pp, §14.5) — the fold threshold is right,
  because it rests on `p_pass`, which is calibrated;
- and the projected `V8_ALL_TERMS` win rate being insensitive to everything
  tried in §12 and §13 — most of those terms enter through `p_place`, and
  `p_place`'s contested component is currently inert.

The fix is to replace `YIELD_PRIOR` + the hard `clears` threshold with a
measured opponent pass rate, the same way v8 already measures its own flip
density from its own decklist. Note the asymmetry that makes this legitimate:
`BOT_STRATEGY.md` §3.1d says the second attacker sees the set **count**, not
contents — but the count is not the only observable. Board hearts, blade count
and set size are all public (8.2.2 keeps the *cards* hidden, not the board), and
those are exactly the inputs `OppModel::horizon_model` was written to use. The
right predictor already exists in the file; it is the step function on top of it
that discards the measurement.

**Do not ship this on n=150 games.** The direction is unambiguous and the effect
is large, but 511 observations from one seed and one matchup is not a win rate.
The next step is the calibration measurement at scale — `V8_OPP_MEASURED`,
bucketed by board and set size, over at least 3 seeds and both seats — and only
then a paired A/B.

---

## 15. Ten hypotheses, ten nulls, and what the discordant counts say

§14 left one live candidate (E) with a falsifiable prediction. This section runs
it, fans four subagents across independent axes, and records the result — which
is not the result any of them predicted.

### 15.1 The falsification test of E: partial confirmation, null prescription

`V8_OPP_CLEARED=<p>` replaces the `clears` step function with a flat constant
(`v8_model.rs`, behind the switch). §14.7 predicts a flat value should be
near-bit-identical, because the quantity carries no information.

| arm | result | discordant | p | win rate |
|---|---|---|---|---|
| `V8_OPP_CLEARED=0.874` (the measured rate) | 682-518 | 9 / 7 | 0.804 | **56.8%** (−0.17pp) |
| `V8_OPP_CLEARED=0.60` (the model's own mean) | see below | — | — | — |

**Prediction half-confirmed.** 16 discordant games, not 0 — so `clears` carries a
*small* amount of real signal and §14.7's "no information whatsoever" was too
strong. But the signal is worth 9 games.

**Prescription is null.** Substituting the correctly-measured 0.874 for the
model's 0.60 changes nothing: −0.17pp at p=0.80. The opponent-pass calibration
is real, reproducible, and not worth anything. E is dead as a win-rate lever,
and §14.7's framing of it as "the strongest candidate in the document" was
wrong — the measurement was right and the inference from it was not.

### 15.2 Four subagents, four independent axes

Launched across areas this audit had not touched, so a hypothesis could not be
an artefact of the live-set framing:

**A. v9's Main phase (`v8_main.rs`, `v7_main.rs`).** The load-bearing finding is
that **v9 does not use `v8_main.rs` at all** — `registry.rs:214` routes v9 to
`strategy_v7::choose_action`, i.e. v7's leaf. `v8_main.rs` is v8/v10's. The v7
leaf is a **delta** on 13 features, `Pass` hard-coded to 0.0, and with
`V7_BEAM` unset it is **one-ply execute-then-score with zero check re-entry**.
Two structural gaps, both real:

- **No turn-awareness anywhere.** `gs.turn_number` appears only in logging. The
  leaf cannot represent "stop developing on T5", and `v8_main.rs:1128-1141`
  documents that a leaf reading "more board is better" *cannot* represent Pass.
- **`coverage` (weight 60, the second-largest term) is a threshold, not a
  gradient**, summed uniformly over every life held, and `hearts` (weight 3) is a
  colour-blind total. So the leaf pays 60 for crossing a whole-requirement
  threshold on *any* held life and 3 per heart thereafter, with no colour
  sensitivity — in a game decided by colour-matching a specific life.

**B. Deck composition (`5CP3Z idou`).** 4 distinct lives, 12 copies. Board
ceiling is 18 printed hearts; all four lives fit, so **57% is not near a deck
ceiling** — there is real slack. Two concrete findings: `MIRACLE NEW STORY` needs
2× heart01 and the deck's heart01 supply is exactly 1 base with **zero blade_heart
heart01 in 72 cards**; and the mulligan bins by cost, never reading a life's
requirement.

**B's headline claim is refuted by measurement.** It concluded MIRACLE is
"effectively dead". Measured: **v9 sets MIRACLE 3 times and passes 3/3**,
scoring 7. The audit's reasoning (heart01 supply = 1) ignores MIRACLE's own
live-start ability, which grants +1 chosen heart to every member that moved.
Same shape as §14.1: a plausible read of the static data, wrong against the
engine. The same correction applies to `始㝾り㝯坛㝮空`'s conditional: the audit
says 12 hearts / score 6 at 2 successes, and measurement shows it scoring
**1, 2, 3** at 2 successes and 6 only at 3.

**C. Rules engine vs bot model (`live.rs`).** Confirms §14.1's correction at
source: `determine_winners` requires `lives.iter().all(|l| l.passed)`, so a
failed live never places — sole or contested.

**D. Turn order / RPS.** Attacker order is not a bot choice; it is decided by
the arena's RPS. Not an available axis.

### 15.3 Every documented-but-disabled fix, measured

Each of these was written, argued for in prose, and shipped at weight 0.0 or
off. That is a strong prior that one is worth shipping, and it is worth testing
all four rather than reasoning about which looks best.

| arm | discordant | p | win rate | pace (P1) |
|---|---|---|---|---|
| baseline (v9 + `V8_ALL_TERMS`) | — | — | **57.0%** | 0.4758 |
| `V7_BLADE_ARITH=0.5` (colour-aware blades) | 82 / 78 | 0.813 | 56.7% | −0.0113 |
| `V7_BLADE_ARITH=2.0` | **200 / 204** | 0.881 | 57.3% | −0.0191 |
| `V8_OPP_DENSITY=measured` | **5 / 5** | 1.000 | 57.0% | −0.0003 |
| `V7_MULLIGAN_COVERAGE=1` | 141 / 114 | 0.103 | **54.8%** | −0.0157 |
| `V8_OPP_CLEARED=0.874` (§15.1) | 9 / 7 | 0.804 | 56.8% | −0.0001 |
| `V8_LIFE_RESALE` ×4 (§14.5) | 7-28 | 0.18-1.00 | 56.3-57.3% | mixed |
| `V8_NO_FOLD` (§14.5) | 173 / 116 | **0.00095** | **52.2%** | −0.0270 |
| `V8_LIVE_START_PROJECTION` (§13.2) | 49 / 56 | 0.558 | 56.0% | 0.0000 |

**Eleven arms. Ten nulls, one significant regression.**

### 15.4 The discordant counts are the actual finding

`V7_BLADE_ARITH=2.0` produces **404 discordant games out of 1200** — it rewrites
the bot's play in a third of all games — and moves the win rate by +0.33pp at
p=0.88. `V8_OPP_DENSITY=measured` rewrites 10 games and moves nothing. These are
not the same kind of intervention, and the win rate cannot tell them apart.

The project's whole method rests on the discordant count being informative
(§7, the paired McNemar test). That assumption has now survived eleven
interventions, and the discordant counts here say something the win rate does
not: **the bot's live-set decisions are largely independent of game outcome.**
A change that alters 404 games and wins 0.33pp more is not a weak effect on a
noisy measurement — it is a large effect on a quantity that does not determine
the result.

This reframes the search. Every hypothesis in §12-§15 has been aimed at the live
set: which card to set, whether to set, what the opponent will do, how to price
the check. All ten are null. The consistent explanation is that **the live set is
not where the game is being lost**, and the one measurement pointing elsewhere
is the guide's own health metric: placements per live phase **0.452 against a
target of ~1.0**.

Note what that metric now means in light of §14.1. v9 passes 91-100% of the
checks it commits, yet places in only 45% of live phases. Those two facts are
only reconcilable if the loss is *not* in the check: it is in the 13.8% of
phases where nobody commits, and in the turns spent reaching a board good enough
to commit. That is the Main phase, and §15.2-A found the Main leaf has no turn
awareness at all.

**The strongest surviving hypothesis is therefore not a switch.** It is that v9's
Main leaf cannot express "stop developing", and `v8_main.rs:1128-1141` says so
in the source with its own probe evidence (v9 went 4-0 with `Pass` on T2/T3). The
documented probe that resolves it is already written — `PROBE=v9` puts v7's Main
against v8's on identical states — and the next step is to run it at scale
rather than four games, because a one-ply leaf with no turn term is the largest
structural gap the audit has found, and every switch aimed elsewhere has now
been exhausted.

---

## 16. Attacker order: two derivations disagree, and the disagreement is the finding

A subagent was asked whether attacker order (先攻/後攻) is an unexplored axis.
It answered no, on a measurement: `P(place | first attacker) = 0.5447` vs
`P(place | second attacker) = 0.5408` over ~11400 checks, "z ≈ 0.6, not
significant", **0.39pp — close the axis.**

That is the basis for declaring a whole direction dead, so it got re-derived
from the raw trace (`tools/analysis\attacker_order_value.ps1`).

### 16.1 The re-derivation does not replicate it

| | subagent | independent re-derivation |
|---|---|---|
| P(place \| first attacker) | 54.47% | **35.97%** |
| P(place \| second attacker) | 54.08% | **50.23%** |
| difference | +0.39pp | **−14.26pp** |
| z | ≈ 0.6 | **−21.9** |

Both derive attacker order the same way (rule 8.4.13: whoever solely placed
becomes first attacker for the next check, `live.rs:1313-1319`), from the same
5000-game trace, 22600 checks with a reconstructable order. They disagree by
two orders of magnitude. **One of them is a measurement error**, and the trace
format is a plausible culprit: the header line is positional
(`game,turn,phase,...,live_p1`) but every data line uses trailing `key=value`
pairs, so any positional parse silently reads `sel=[4+2+0]` as a column index.
My first two attempts failed exactly that way (0 games; then a cast exception),
which is the same class of bug as §14.1 — a convenient adjacent field trusted
instead of the right one.

**Do not record either number until the disagreement is resolved.** A z of −21.9
and a z of +0.6 cannot both be true of the same quantity.

### 16.2 Settled causally instead

Rather than argue about which parse is right, the harness was made to assign
attacker order directly (`ARENA_ATTACKER_ORDER` in `bot_arena.rs`, default
unchanged). This does not depend on any trace parsing, any reconstruction of
8.4.13, or any confounder:

| `ARENA_ATTACKER_ORDER` | result | discordant | p | win rate |
|---|---|---|---|---|
| baseline (S6: RPS winner takes 2nd) | 684-516 | — | — | **57.0%** |
| `first` (both forced first) | 696-504 | 213 / 225 | 0.599 | 58.0% |
| `second` (both forced second) | 690-510 | 62 / 68 | 0.661 | 57.5% |
| `random` (uniform, RPS ignored) | 697-503 | 62 / 75 | 0.305 | 58.1% |
| `flip` (RPS winner takes 1st) | 688-512 | 209 / 213 | 0.884 | 57.3% |

**All four null**, spanning +0.33 to +1.08pp with every p ≥ 0.30, while `flip`
rewrites 422 games and `first` rewrites 438. The reordering is large; the win
rate does not move. This is the §15.4 pattern for the fourth time, now on an
axis where the intervention is causal and unmistakable.

The conclusion the subagent reached is therefore **right on the substance and
should be kept** — attacker order is not worth anything here — but its stated
evidence was a z-statistic that does not replicate, and a 0.39pp number must not
be cited. `V8_INITIATIVE_WEIGHT`'s docstring asked for exactly this
("measure it rather than argue about it"); it has now been measured, causally,
and the answer is no.

### 16.3 Two corrections that are not about attacker order

**A test comment asserted a rule is immutable when it is re-assigned every
turn.** `phase_machine_rules_test.rs:13-15` read "`is_first_attacker` NEVER
flips — it records who won RPS". Rule 8.4.13 re-assigns it every turn, and the
tests two files over (`attacker_order_performance_test.rs`) assert exactly that
in their names. Anyone reading the comment would have dismissed this axis on
the strength of a statement that is false. Corrected, with a pointer to the
tests that actually pin the rule.

**Adding the switch nearly corrupted the project's headline number.** The first
version of `ARENA_ATTACKER_ORDER` collapsed the RPS-loser's uniform random pick
into a deterministic `ChooseFirstAttacker`, which moved the default baseline
from **57.0% to 58.25%** — a 1.25pp silent shift in the number every
measurement in this document is quoted against. Caught only because the default
was re-run after the change and did not reproduce 684-516 / 40898 actions. The
default path now restores the original random pick, verified bit-identical
(684-516, 40898 actions).

That is the third time in this audit that a harness-level edit threatened a
recorded baseline, and it argues for a standing guard: any change to
`bot_arena.rs`'s decision interception must be re-verified against the frozen
684-516 / 40898 signature before its results are recorded.

### 16.4 The live-set axes are now exhausted

Fourteen interventions, twelve null, one significant regression, one partial
confirmation. Every one of them touched the live set, the opponent model, the
fold threshold, the mulligan, or the attacker order. Two subagent headline
claims were refuted against the engine (MIRACLE is placeable — 3 set, 3 passed,
score 7; `始㝾り` scores 1-3 at two successes, not 6), and two of my own
readings were wrong (§13.3's field misread, §14.1).

The consistent explanation across all of it: **these decisions do not determine
this game.** The one measurement pointing elsewhere has been pointing at the
same place since §13: placements per live phase **0.452 against a target of
~1.0**, with committed checks passing 91-100%. A bot that passes almost every
check it takes and still places in under half its phases is losing the game
before the check, in the turns spent assembling a board — which is the Main
leaf, which §15.2-A found to be one-ply, colour-blind in its `hearts` term, and
threshold-only in its `coverage` term with no turn-awareness to express when to
stop.

---

## 17. Card hygiene: are unwanted cards cycled out? Measured in both phases

The question worth asking is not "is the live-set decision right" — fourteen
interventions say it is — but **whether cards the bot does not want ever leave
its hand.** Two phases can do it, and neither had been measured.

### 17.1 Live phase: hand filtering is measurably OFF

`junk_keep_value` (`v8_live.rs:94-160`) prices every non-live hand card and the
spare live-zone slots are filled with the cheapest, so there is machinery for
this. The source records the metric it was tuned against and flags the failure
mode by name:

> pricing it at a full ticket made every engine unburnable, which silenced hand
> filtering entirely (measured **0.07 junk cards per set** against v7's 1.08)

Measured from `V8_CAL`'s chosen candidate across 150 games:

| | |
|---|---|
| sets where v9 committed | 136 |
| sets with ≥1 non-life burned | **0** |
| **junk cards burned per set** | **0.00** (source recorded 0.07) |

And the reason is not subtle — **148 of 148 committed sets put exactly ONE card
in the zone, every time.** Three slots exist. Two are never used, so there is no
junk to burn because there is no room to burn it in. The filtering is not
mispriced; the enumerator simply never proposes a multi-card set.

This is consistent with `V8_MAX_LIVES` defaulting to 1 and with the log's own
account of `V8_ZONE_ORDER` + `V8_MAX_LIVES=3` measuring **0 discordant** — the
width cap was tested as a flag and never moved a decision. The spare slots are
a real, unexercised part of the rules (8.2.2 allows 3; each card resolves
independently) and the bot leaves all of it on the table.

**But note the discipline this demands.** §14.7 proved the opponent model is
inert; §15.4 proved decision churn is uncorrelated with outcomes. Before
treating "use all 3 slots" as an opportunity, the honest prediction is that it
is another null — the width cap is a leaf-level constant like every other, and
the one time it was tested in combination it changed nothing. It is the last
untested structural knob, not a promising lead.

### 17.2 Mulligan: a real bug, fixed, and the fix changed nothing

The deck audit found a genuine defect. `kept_live_requirement`
(`strategy_v7.rs:2925`) sums the heart requirements of the **first three lives
in hand**, regardless of whether they are settable. In `5CP3Z idou`,
`MIRACLE NEW STORY` needs 2× heart01 while the deck's entire heart01 supply is
one base heart and **zero blade-hearts** — the only route to a second is
MIRACLE's own live-start. Holding MIRACLE therefore injected `2` into
`want[1]`, a colour **no member in the deck can serve**, and
`V7_MULLIGAN_COVERAGE` scored every member in hand as useless against an
impossible requirement.

That is a real bug and it is now fixed, in a way that generalises rather than
special-casing MIRACLE: `colour_supply_ceiling` computes, from our own decklist
(fair information — we are holding 6 face-up), an upper bound per colour on what
any board can supply (best `STAGE_SIZE` members by that colour, plus every
blade-heart the deck can yell), and `kept_live_requirement_achievable` clamps the
demand to it. **A requirement no board can ever meet is not a requirement; it is
a dead card.**

Baseline re-verified unchanged with the fix in place and the switch off:
684-516 / 40898 actions, bit-identical.

**And the result is bit-identical to the buggy version:**

| | result | discordant | p |
|---|---|---|---|
| `V7_MULLIGAN_COVERAGE=1`, before the fix | 657-543 | 141 / 114 | 0.103 |
| `V7_MULLIGAN_COVERAGE=1`, after the fix | **657-543** | **141 / 114** | **0.103** |

Same games, same count, same p. So the `heart01` corruption was real and
irrelevant: the coverage mulligan loses for some other reason. The fix is kept
because it is correct and costs nothing, not because it earned its place here.

### 17.3 And the seed variance, which is the actual lesson

One seed said **+3.33pp, p=0.014, SIGNIFICANT IMPROVEMENT** — the first
significant positive in the whole audit. It did not survive:

| seed | baseline | coverage mulligan | delta | p |
|---|---|---|---|---|
| 11 | 57.0% | 54.8% | **−2.25pp** | 0.103 |
| 12 | 53.5% | 56.8% | **+3.33pp** | **0.014** |
| 13 | 54.6% | 55.5% | +0.92pp | 0.542 |

Mean +0.67pp, and note the **baselines themselves swing 57.0 / 53.5 / 54.6**
across seeds — a 3.5pp spread in the control. That is the same magnitude as
every effect in §15 and §16.

**This is the most important number in this section.** The whole document reasons
from seed 11, where the baseline reads 57.0%. On other seeds the same bot reads
53.5-54.6%. A single-seed ±3pp swing is larger than any live-set effect measured
in §12-§17. It means **every single-seed result in this log, including the ones
already recorded as confirmed, is within seed noise of being null** — and it
means a p=0.014 on one seed should be read as a seed effect until reproduced.

The fix to the method is cheap and should be applied before any further claim:
the confirmed `V8_ALL_TERMS` result (+5.40/+5.67 on "two seeds") needs re-running
across the seed set, and no single-seed p should be quoted without its neighbours.
`--outcomes` already exists to make this paired; what is missing is a habit of
running the seed set, not a tool.

### 17.4 The seed spread, measured properly

The same v9 build, `V8_ALL_TERMS` on, 1200 games per seed, seed 11 the only
seed this document has ever used:

| seed | result | win rate |
|---|---|---|
| 11 | 684-516 | **57.0%** |
| 12 | — | 53.5% |
| 13 | — | 54.6% |
| 14 | 662-538 | 55.2% |
| 15 | 696-504 | 58.0% |
| 16 | 659-541 | 54.9% |
| 17 | 687-513 | 57.3% |

**Range 53.5-58.0%, a 4.5pp spread in the control condition.** Seed 11, the seed
every result in this log is quoted on, sits at the top of that range.

Two consequences, and the second is uncomfortable:

1. **Any single-seed effect below ~4.5pp is uninterpretable on its own.** Every
   null in §14-§17 was already null; the one significant *positive* in §17.3
   (+3.33pp, p=0.014) is inside the noise band and did not replicate.
2. **The seed-11 baseline of 57.0% is a favourable draw, not the bot's level.**
   The honest central estimate from seven seeds is **55.7%**, and the honest
   range is 53.5-58.0%.

This does not make the work wrong, but it does mean the document's headline
number is the top of a range rather than its middle, and that "0.3-0.5pp null"
results have been read against the noisiest possible control.

The method fix is small and worth doing before anything else ships: run the seed
set, report mean and range, and quote a paired delta only when the *same seed*
comparison is available on both arms. `--outcomes` already exists for the paired
test; the gap is a standing seed sweep.

---

## 18. The measurement bug underneath every number in this document

Re-running the confirmed result across seeds produced something that stopped
the audit cold.

### 18.1 `V8_ALL_TERMS` is bit-identical to OFF, on every seed

The whole document runs v9 **with `V8_ALL_TERMS` set**, on the strength of the
earliest confirmed result in the log. v9 with and without the flag:

| seed | `V8_ALL_TERMS` OFF | ON | identical? |
|---|---|---|---|
| 11 | 684-516 / 40898 | 684-516 / 40898 | **yes** |
| 12 | 642-558 / 41311 | 642-558 / 41311 | **yes** |
| 13 | 655-545 / 41567 | 655-545 / 41567 | **yes** |
| 14 | 662-538 / 41654 | 662-538 / 41654 | **yes** |

Not "same win rate" — the same record, to the action count, on every seed tried.
Every A/B in §13-§17 was run against a control that is the treatment.

### 18.2 Why: the flag is not on v9's path

All four `V8_ALL_TERMS` sites are `v8_main.rs:1372` and `v8_model.rs:1133,
1199, 1205`. The only consumer of the weights they gate is `v7_term_score`,
which is called from exactly one place — `v8_main.rs:1192` — inside **v8's own
Main leaf**. And `registry.rs:214` routes v9 to `strategy_v7::choose_action`,
i.e. **v7's Main leaf**. So for v9, none of the four sites is reachable.

The registry says so itself, at `registry.rs:215-216`:

> v10 is v8's Main, which becomes the confirmed configuration when
> `V8_ALL_TERMS` is set.

**v10 is the bot the flag was written for. v9 never had it.** Confirmed
behaviourally — for v10 the flag is very much alive:

| | result | actions |
|---|---|---|
| v10, flag OFF | 273-327 | 22828 |
| v10, flag ON | **284-316** | **22398** |

### 18.3 What this does and does not invalidate

**First, a correction to this section.** An earlier draft of §18.3 claimed the
+5.40/+5.67 was "credited to a flag v9 does not read" and that the attribution
was wrong. Reading §11.4 again, **the original entry is correct** and this
section was wrong: the table at line 1279-1283 records the term vector as
`v8 (42.5%) -> v8 + terms (47.9% / 47.5% holdout)`, and separately `v9 (57.3%)`
and `v10 (46.5%)`. The +5.4 was always measured **on v8's Main leaf**, and v10
is the bot built to carry it. Nothing was mis-attributed, and no confirmed
result in this log is retracted by §18.

The relationships are also consistent rather than contradictory: the term vector
*is* v7's delta weights, and v9's Main leaf **already is** that delta leaf —
which is precisely why v9 (57.3%) sits far above v8+terms (47.9%) and why v10,
which pays for v8's Main and then adds v7's vector, lands between them. Setting
the flag on a v9 run was redundant, not wrong.

**What remains true and worth keeping:**

1. **`V8_ALL_TERMS` is a no-op for v9** (proved four ways: bit-identical records
   on four seeds; all four flag sites unreachable; `registry.rs:214` routing v9
   to v7's leaf; and v10 responding strongly to the same flag). Every A/B in
   §13-§17 set it, so every control in this document was, in that one respect,
   the treatment. Harmless — it did nothing — but it means those runs are not
   what their command lines imply.
2. **The check was never run.** A switch that changes no action count on the bot
   under test is a comment, not a switch. It is the cheapest possible test of a
   confirmed result and it would have caught this in one run.

### 18.4 The generalisable lesson

A flag that no code path reads produces a distinctive signature: a run line that
*looks* configured, and a difference between arms that is zero to the action
count. This project's arena help text even warns about the same class
("a silently-different bot under a name that promises otherwise"), and the
registry comment at `registry.rs:216-220` says `V8_ALL_TERMS` belongs to **v10**
— so the information needed to avoid this was already in the source, next to
every measurement taken without it.

Standing rule, alongside the §16.3 guard: before recording any result as
confirmed, verify (a) that the bot under test actually reads the switch — one
action-count check is enough — and (b) treatment-off vs treatment-on **for the
bot the switch was written for**, to confirm the switch still works at all.

The status of the project after §18 is unchanged from §17.4: **v9 is a real bot
at ~55.7% (53.5-58.0 across seven seeds), its live-set machinery is sound, and
the +5.4 belongs to v8's Main leaf, where it was measured.**

---

## 19. What the CCG-AI literature says is missing here

Every finding in §13-§18 was arrived at by reading this repository. Read against
the published work on collectible card game agents, the shape of the problem
changes — several results are not surprises but *known failure modes with known
names*, and the standard fixes are ones this project has partly tried and
partly missed.

### 19.1 The inverted opponent model is a textbook determinization failure

§14.7 measured `opp_pass` as inverted and flat: realized 86.2-88.3% in every
predicted bucket, including the bucket asserting a fold. That is precisely the
failure determinization is known to produce, and it has a name — **strategy
fusion** (Frank & Basin), the first of the two canonical defects, where a
single guess about hidden information is treated as truth and dominates the
search around it.

The standard remedy is **ensemble determinization** (Cowling, Ward & Powley,
*Ensemble Determinization in MCTS for the Imperfect Information Card Game Magic:
The Gathering*, IEEE TCIAIG 4(4), 2012): instead of guessing the opponent's
hidden state once, sample *many* consistent states, run one search per sample,
and aggregate. Kruse & Bartels report it beating plain determinization and
other MCTS variants across deck matchups, and note that even a perfect
prediction yields only a *slight* further gain — i.e. most of the available
benefit comes from not trusting a single guess, which is exactly v9's position.

`clears` is a 5-scenario enumeration over `YIELD_PRIOR`, so it is *already* a
form of ensemble — but over **yield**, not over the check itself, and collapsed
through a hard `supply >= bar` threshold. The literature's version ensembles the
simulation of the opponent's actual check, which is why it degrades gracefully
when the prior is wrong. **This is the first borrowed idea with a mechanism that
matches a measured defect**, and it is a code change to `OppModel::build_with`
rather than a new term to weight.

### 19.2 One-ply Main is why §14-§18 found so many nulls

The Main leaf evaluates one action, scores it once, and returns. §15.2-A
established `V7_BEAM` is off, so the depth-8 chain and the depth-1 beam are
unreachable, and `V7_ISMCTS` ships off after measuring as a consistent
regression at every setting (p as low as 0.003).

The literature is more encouraging than this project has been, and identifies
*why* the project has been getting nulls rather than gains:

- Ward et al. (reduced Magic) found the best configuration was **MCTS for
  playing cards with a rule-based attacker/blocker**, i.e. search where the
  action space is combinatorial and heuristics do not scale.
- Van Rijn's Hearthstone thesis records that **naive UCT scored at most 26%**
  while determinized bots scored far better, and specifically that a **binary
  action tree** (execute-this-action-or-not per node) beat the standard n-ary
  tree. This is the shape of §17.1's finding that width is never used: a tree
  over *which cards to also put in the zone* is precisely a binary tree.
- Non-locality — the second determinization defect — is the mirror image of
  strategy fusion: a locally-imperfect guess poisons an entire subtree rather
  than one decision. With §14.7's flat curve, **every** simulated continuation
  downstream of an opponent model is poisoned equally, which is a plausible
  mechanical explanation for §15.4: interventions that change hundreds of
  decisions (404 discordant) moving the win rate 0.33pp. If the continuations
  are noise, churning them is free.

None of that argues for more search *depth* bolted onto a bad model. It argues
for **fixing the model, then adding search** — the order this project has
inverted, which is also the order its own history recommends.

### 19.3 The SOTA does not use search at all, and that is the uncomfortable part

The strongest published CCG agents — **ByteRL** (ByteDance, COG 2022 double
championship) and the **OSFP** Hearthstone agent that beat a top-10 Chinese
league player in full games — train **end-to-end with no lookahead search in
either training or inference**. Both use Optimistic Smooth Fictitious Play over
a single shared policy function, and ByteRL is explicitly *not* a search agent.
The 2024 exploitability paper (arXiv 2404.16689) then showed ByteRL is highly
exploitable, and that **plain behaviour cloning of its policy is nearly on par
with it** — the policy, not the search or the RL fine-tuning, is carrying the
strength.

That reframes this document's entire method. Fourteen interventions here were
hand-transcribed scalar terms in a delta leaf, tuned against single-seed win
rates. The published evidence says the productive object is a **learned policy
or value function over state features**, and that rule-based term engineering
has a poor track record in exactly this genre. It also explains the audit's own
central puzzle better than any switch did: fourteen hand-built terms, twelve
null, because hand-built terms are not where the signal is.

### 19.4 What this project already does that the literature endorses

Not everything here is behind the field, and it is worth knowing before acting on
the above:

- **Deterministic simulation with an engine that can be cloned exactly.** The
  116-test suite and the bit-identical paired methodology are the precondition
  for every measurement in this log, and are stronger than most published
  evaluations.
- **A real hidden-information model.** `OppModel` reads zone *count* and public
  board, which is the correct information set under 8.2.2 — the literature's
  determinization target is well chosen.
- **A paired, seed-joined A/B harness** (§7). The 4.5pp seed spread in §17.4 is
  something this project's method was well placed to detect and had not been
  run.

### 19.5 Ranked, with what each would actually cost

| # | borrowed idea | source | cost | expected |
|---|---|---|---|---|
| 1 | **Ensemble determinization** for `pass_prob` — sample many opponent states, simulate their checks, average | Cowling 2012; Kruse 2018 | moderate, `OppModel::build_with` | the only idea matching a *measured* defect (§14.7) |
| 2 | **Feature-based value function fitted to game outcomes** — replace the delta leaf's weights with regression on arena results | ByteRL; Li 2023 (CC feature repr beats ID ranking) | high, needs an outcome-labelled corpus | attacks §15.4 directly; the field's actual answer |
| 3 | **Binary action tree over live-zone width** (set-2nd-card or not) | Ward; van Rijn | low, `V8_MAX_LIVES` as a real decision | §17.1's unused 2 slots; historically 0 discordant, so expect null |
| 4 | Multi-seed evaluation as standard practice | §17.4 here; standard in all cited work | trivial, one script | protects every other number |
| 5 | Self-play / exploitability harness — an adversary trained to beat *this* bot | arXiv 2404.16689 | very high | would find what 14 manual hypotheses missed |

**Recommendation.** Do 4 first (it is a script and it protects everything else),
then 1 (the only idea grounded in a defect this audit actually measured), then
2 if the project wants to compete on the field's terms rather than by rule
transcription. Do **not** spend more effort on hand-weighted leaf terms: this
document now contains fourteen measurements saying they do not move the win
rate, and the literature says why.

### 19.6 Ensemble determinization: implemented, measured, and nearly inert

`V8_OPP_ENSEMBLE` / `V8_OPP_ENSEMBLE_N` (default 64) implement #1 above:
sample many `(yield, board-jitter)` states consistent with the opponent's
public information, ask each whether it clears the bar, and average — instead
of gating one aggregate supply through one threshold. Sampling walks the
existing prior with a private LCG, so a policy call leaves the game RNG
untouched and stays reproducible.

| seed | discordant | p | delta |
|---|---|---|---|
| 11 | 2 / 1 | 1.000 | −0.08pp |
| 12 | 2 / 0 | 0.500 | −0.17pp |
| 13 | 3 / 3 | 1.000 | +0.00pp |

**7 decisions changed across 3600 games.** The published remedy for the exact
defect §14.7 measured is implemented and moves nothing.

That is the third independent probe of the same quantity, and together they are
consistent:

| probe | §14.7 measured | §15.1 flat constant | §19.6 ensemble |
|---|---|---|---|
| effect | curve is flat & inverted | 9 / 7 discordant | **2 / 1** discordant |

Replacing `clears` with a constant moved 16 games; replacing it with a properly
ensembled version moves 7. The step function and its ensemble are nearly the
same function, because on this deck the supply almost always lands on the same
side of the bar either way.

**Conclusion: the opponent's pass probability is not merely mis-modelled — it is
irrelevant to v9's decisions.** The literature's diagnosis was right and its
remedy does not apply, because the problem was never the *form* of the
determinization. v9's live set does not turn on it.

This closes the opponent-model thread for good, and it is the strongest
statement the audit supports: **the bot's ~55.7% is determined by its own board
and its own check, and essentially nothing about modelling the opponent moves
it.** Any future work on the opponent side should be assumed inert until proven
otherwise by a change that alters decisions by more than ~10 games in 1200.

### 19.7 Final state

**Fifteen interventions. Thirteen null, one significant regression
(`V8_NO_FOLD`, −4.75pp, p=0.00095), one partial confirmation.** Every one
targeted the live set, the opponent model, the fold threshold, the mulligan,
attacker order, or a hand-fitted leaf term.

What is solid:
- The live-set decision procedure is sound — passes 91-100%, folds only when
  the commit cannot pass, argmax self-consistent over 120 games (0 misranks).
- The deck is not the ceiling: 3 successes in 3 turns is achievable, and 2 of 3
  live slots are never used.
- Measurement infrastructure is good: paired seed-joined A/B, deterministic
  engine, 116 tests, and a frozen baseline signature that caught two harness
  regressions.

What was wrong and is now fixed or documented: the live-start projection's
missing `process_pending_auto_abilities` (§13.2), a `kept_live_requirement`
bug that scored members against unsatisfiable colours (§17.2), and a test
comment asserting a rule that 8.4.13 re-assigns every turn (§16.3).

What remains open, honestly: v9 is a one-ply, colour-blind, hand-weighted delta
leaf at ~55.7% (53.5-58.0), and the published evidence says the way past that
is a learned policy or value function over state features — not more
hand-transcribed terms. That is a large change, and nothing in §13-§19
suggests a cheap switch will do it.

---

## 20. Built it: a learned policy, from the half that was missing

§19 ranked a learned value/policy function second, behind a seed sweep. The
seed sweep is a script; this was not, so this section is what actually building
it found.

### 20.1 The infrastructure was 90% present and the last 10% did not exist

The repo already had everything except the two functions that matter:

| present | missing |
|---|---|
| `encoding.rs`: 98 global features, card/zone/action-type embedding tables, `state_dim() == 2248` | `PolicyNet::init_random` |
| `neural.rs`: a 256-hidden policy + value net, `evaluate_actions` | `PolicyNet::save_weights` |
| `ppo_collect.rs`: trajectory generation, `PPO_BEHAVIOR=v7` imitation mode | **any trainer at all** |
| `bot_arena.rs`: `BotKind::Neural`, `NEURAL_WEIGHTS`, `NEURAL_EPSILON` | |

`PolicyNet::new()` zero-filled every weight, so every logit was identical and the
policy uniform — fine for a random baseline, useless for training, because the
ReLU never activates and no gradient reaches anything above the bias. And with
no `save_weights`, `load_weights` had nothing that could ever produce a file.
**The neural seat was unreachable from both ends.** Added Xavier init and a
writer matching `load_weights`'s exact layout, plus `ppo_train.rs` — softmax
cross-entropy, gradients written through the net's own forward arithmetic rather
than a re-derivation, so training and inference cannot drift.

### 20.2 Three real bugs in the data path, all found by asserting

A desynced binary reader does not fail; it reads garbage and allocates wildly.
The first run asked for **40GB**. Three format bugs behind it:

- **15 bytes per action, not 16.** `card_index` is a `u8` and `to_le_bytes()` on
  a `u8` is one byte. I had assumed 4.
- **`chosen_idx` is `u16`, not `u32`.** Reading it as `u32` ate the next float
  and yielded 65535.
- **The decoder lost a byte per action.** Reading fields as a struct literal of
  side-effecting calls let the struct field order, not the writer's order,
  decide consumption, and a trailing `*p += 4` double-counted the byte the
  writer already spends inside `count,0,0,0`. Rewritten with explicit offsets
  and one advance at the end, plus an `n_actions <= 512` assertion so a future
  mismatch reports itself instead of requesting 40GB.

Also fixed in the collector: `num_games` was `u8`, so a request for 1500 games
parsed as 244 and the run **stopped at 200 games with no error** — a corpus 7x
smaller than asked for, indistinguishable from success. And the deck was
hardcoded to `fade deck.txt`, so every trajectory described a different game
from the one the policy is ever evaluated on — the exact mistake
`bin_common::load_deck` panics on.

### 20.3 It learns. It does not play.

Corpus: 3000 games, **71,607 labelled decisions**, both seats imitating v7.

| | holdout top-1 | NLL |
|---|---|---|
| before training | 0.2636 | 7.74 |
| after (14 epochs) | **0.5317** | 1.47 |
| chance (1/N, mean N=5.99) | 0.167 | — |

**3.2x better than chance**, and the NLL collapse is real learning, not noise.
Ten times the data did not improve it (200-game corpus reached 0.5486 on 7063
steps), so the model is representation- or capacity-limited, not data-limited.

Play strength is a different story:

| arm | vs v7 | actions/game | turns |
|---|---|---|---|
| `random` (floor) | 0-364 (**0.0%**) | 32.1 | 4.53 |
| **neural, policy_v2** | 386-814 (**32.2%**) | 40.8 | 6.02 |
| **neural + pass guard** | 350-635 (**35.5%**) | 34.5 | 5.41 |
| `v7` vs itself (teacher) | 634-566 (52.8%) | 34.7 | 5.88 |
| `v9` (shipping) | 684-516 (57.0%) | 34.1 | 5.80 |

### 20.4 The gap has one cause, and it is the §15.2-A prediction confirmed

**Pass rate on Main-phase decisions:**

| seat | pass rate |
|---|---|
| v7 (teacher) | **40.5%** |
| cloned net | **4.2%** |

A 10x difference, and it explains the whole gap. The net takes 40.8 actions per
game against the teacher's 34.7 because it essentially never stops developing.

This is exactly what §15.2-A predicted from reading the source: v7 decides Pass
through `v7_main::value`, a **delta** leaf where Pass wins only when every
weighted term comes out non-positive. That is a *relation over candidates*, not
a property of the position. Cloning the argmax learns which action wins and
nothing about the region where none of them do — so it never discovers the stop
state. It is the same structural statement as `v8_main.rs:1128-1141`'s "a leaf
that reads 'more board is better' cannot represent 'stop'", now demonstrated
empirically on a learned model.

`NEURAL_PASS_GUARD` implements the DAgger-style split: the teacher keeps the
Pass decision, the net picks only among non-Pass actions. It recovered real
ground — 32.2% → **35.5%**, actions/game 40.8 → 34.5 (teacher 34.7), game length
5.41 (teacher 5.88) — which confirms the diagnosis. It also introduced 215 draws
(17.9%), so the remaining error is not only Pass.

### 20.5 Why the literature's "BC is nearly on par" does not apply here

The 2024 exploitability paper found plain behaviour cloning of ByteRL nearly
matched the SOTA RL agent. That result is about **a policy-only teacher**: ByteRL
trained end-to-end with no search, so its decisions *are* a policy and cloning
them is well-posed.

**v7 is not a policy.** Its strength comes substantially from a search that
executes each candidate action, simulates it, and compares weighted feature
*deltas* — including the four hand-weighted opponent terms in
`Search::evaluate` that a position-level clone never sees. The paper's own
framing applies directly: the policy carried the strength for ByteRL, and here
the delta-leaf comparison is carrying it for v7. **Cloning a search-based agent
into a feed-forward policy discards the thing that makes the agent strong**, and
the 20-point shortfall is the size of that discard.

This is not an argument against the §19 recommendation — it is an argument for
doing it in the order the SOTA uses: **start from a competent policy and improve
it with RL**, not clone a searcher and expect the search to come along. The
infrastructure for exactly that now exists: an initialiser, a serialiser, a
trajectory collector, and a trainer. The remaining piece is a PPO/advantage
inner loop against the value head the net already has and nothing currently
trains.

### 20.6 Status

**Not shipped.** The learned policy is at 35.5% against a 52.8% teacher and a
57.0% shipping baseline, and there is no honest framing in which replacing v9
with it is an improvement. What is shipped is the *capability* that did not
exist: an initialiser, a serialiser, a validated corpus pipeline, and a trainer
that demonstrably learns (3.2x chance on 71k held-out decisions). Both new
switch (`NEURAL_PASS_GUARD`, `PPO_DECK`) default to off, the arena default is
bit-identical (684-516 / 40898 actions), and the 116-test suite is green.

The single most useful thing learned, and it is not about neural networks: **a
delta-leaf agent's strength is not transferable to a level model.** Fifteen
interventions in §13-§19 failed to improve that leaf by hand; cloning it into
the form the published SOTA uses loses 20 points. The next honest step is
RL-with-a-value-head from this initialisation, not more hand weights.

---

## 21. RL, and what the literature says about abilities

### 21.1 You were right to push, and there were three correctness bugs

Cloning is capped at the teacher by construction, so PPO was the right ask. It
took `ppo_learn.rs` (GAE within seat-contiguous runs, clipped surrogate, value
loss) plus three fixes to the collector, each a genuine correctness bug rather
than a tuning miss:

1. **Mirror self-play needed both seats.** The collector recorded only P1, so
   RL had half the data and no P2 signal. It now resolves the acting seat the
   way `bot_arena::decision_player` does — a pending CHOICE can belong to the
   player who is *not* the active player — and signs each step's reward by its
   own seat. Trajectory version 2 → 3 (a trailing `seat` byte); the reader
   accepts both so every existing corpus still loads.
2. **PPO's central assumption was broken on iteration 0.** The collector
   started from `PolicyNet::new()` (all zeros → uniform policy) while the
   trainer started from Xavier, so the importance ratio compared two different
   policies. It diverged to **|dlog ratio| ≈ 2.8e12 and a NaN value loss.** The
   collector now Xavier-initialises and **saves the policy that generated the
   data**, which the trainer warm-starts from. This is the single most important
   line in the whole loop.
3. **Per-sample SGD instead of batched.** 88k sequential updates per epoch at
   `lr=3e-4` is a divergent optimisation. Gradients now accumulate over a chunk
   and apply once; advantages are clamped to ±10 because a terminal ±1 reward
   produces a few enormous ones, and one dominating a step is what NaN'd the
   value loss.

After those: **value loss 0.0003, entropy 1.63 (≈ uniform over the legal set),
|dlog ratio| 52.** The optimiser is healthy.

### 21.2 And then the signal turned out to be degenerate

`p1_wins = 0` of 500 games. **Two near-uniform nets playing each other draw
every single game**, so every reward is 0, every advantage is 0, and there is
nothing to learn from. The policy term is flat at −0.0194 for exactly that
reason. (For scale: `random` vs `v7` draws 39.3% of games; mirror-random drawa
lot, because neither side can ever assemble a board.)

This is a real property of the game, not a bug: 3 successes is a long way from
a random start, and a policy that has learned nothing will never get there. The
published answer is not to fix the optimiser but to **fix the opponent
distribution** — ByteRL trains against a *pool* of historical checkpoints, not a
mirror, precisely so the opponent is diverse enough for games to be decisive.
That is the next step and it is not written yet.

### 21.3 On abilities: the literature says you should NOT add inputs, and here is why

I went and checked what the strong agents actually encode. The answer is
consistent across ByteRL (COG'23 champions), the ByteDance Hearthstone agent
(arXiv 2303.05197), and every Hearthstone AI Competition winner:

**None of them encode card text.** ByteRL borrows gym-locm's observation
wholesale; the Hearthstone agent's Table V lists 14 observation fields and card
text is not among them — cards appear as a learned identity embedding plus
scalars, and spells get no feature at all beyond being identified as cards.

Worse for the "more inputs" intuition, there are **three published negative
results on adding features**:

- **Vieira (EntCom 2023)** added deck information to the observation: it made
  things *worse* (56.4% vs 59.2%), as did both reward-shaping variants. "Adding
  deck information to the game state was not worth it."
- **Tutea (Groningen 2020)** head-to-head on state-description size: on simple
  decks the *larger* representation won **less** (78% vs 81% against random);
  only on synergistic decks did it help (88%). Bigger is not better; it is worse
  when it is not needed.
- **Jakubik (AAIA'17)** attribute their entire failure mode to abilities that
  "are not expressed in any way in a contextless game state description."

So the honest answer to "couldn't you have more inputs, especially for
abilities?" is: **there is no published evidence that adding them helps, and one
strong result that it hurts.** The one thing that reliably worked in the
literature is Grad's two hand-authored *spell-class aggregates* — `hand_aoe`
(total AoE damage in hand) and `hand_answers` (count of hard-removal cards) —
which won AAIA'17 with AUC 0.80185 on an otherwise knowledge-free model. That is
the shape of the win: a **few aggregate priors about what a hand can do**, not
more raw inputs.

### 21.4 The cheapest known win, which this repo does not have

The most actionable finding is a *structural* defect the whole DRL-LOCM lineage
is openly stuck on, and this repo shares it. Vieira, §6:

> "the current DRL literature on LOCM considers, for instance, **every card slot
> in the player's hand as a completely different set of features**. This way, the
> neural networks must learn how to deal with a card in hand multiple times…
> **Deep Sets, Set Transformers, or other permutation-equivariant architectures
> may be a good fit.**"

Nobody in the strong-agent line has done this, and it is called out explicitly as
the cheapest known win. **This repo already has it, and did before the paper
pointed at it**: `neural.rs:209` defines a `sum_embeds` closure that pools card
embeddings, and every non-positional zone goes through it —
`my_hand: sum_embeds(&obs.me.hand)`, and likewise energy, waitroom, live,
success and deck composition (`neural.rs:348-355`). Only the three stage slots
are positional, which is correct: a stage slot's position is real information.

**So the "add more inputs" question is already answered the right way here — by
pooling rather than by growing the vector.** That is very likely part of why the
net reaches 53% top-1 (3.2x chance) off 71k steps, where a per-slot encoding
would have had to learn the same card in ten different hand slots.

### 21.5 One genuinely open problem, and it is this game's problem

The research is explicit that encoding conditional abilities is **unsolved**:

> "The 'encode N conditional effects' problem is unsolved in the learned-CCG
> literature. LOCM's 6-bit mask works because its effect grammar is closed;
> Cardsformer's text embedding works only for simple templated effects ('fails to
> predict Lorewalker Cho')."

This game's abilities are trigger-based — `LiveStart`, `LiveSuccess`,
`printed`, `constant`, `auto` — which is exactly the case Cardsformer fails on.
The one paper that encodes text at all (Cardsformer, ECAI'23) reports that on
*known* cards text buys data efficiency and **~8 points, not asymptotic
strength**; the real win is unseen cards, and this deck is fixed.

So a learned ability encoder here would be genuinely novel and would probably not
pay. The evidence-based move is not more inputs — it is Grad's two aggregates,
or nothing.

### 21.6 The ladder: it works, and PPO on it made things worse

The suggested design — beat v1, then v2, and so on — is the right answer to
§21.2's degenerate mirror, and it is now implemented (`PPO_OPP_BOT=v1..v7`,
`PPO_SEATS=1`). Getting there took four more real bugs, all of which produced a
*silent* total failure rather than an error:

1. **The net was on the wrong seat.** With `PPO_SEATS=1` the net took P1 and the
   ladder bot P2, so every trajectory was the bot losing and the counter read
   0 wins in 120 games. The network must take the seat the ladder bot does not.
2. **Every game ended `Draw` at turn 1.** The net was picking RPS and attacker
   order at random; RPS ties re-pick until `stuck > 200`. The arena intercepts
   exactly those phases before dispatch for this reason, and the collector did
   not.
3. **The mulligan hung the same way.** It is multi-step (select, then confirm),
   and sampling one step at a time re-picks a non-advancing selection until the
   turn cap.
4. **The live-card set hung the same way.** Also multi-step. Handing the net's
   live-set seat to the ladder bot's live-set logic fixes it — and is a
   reasonable design anyway, since the live set is the one thing this bot family
   is good at.

After those, the ladder produces real signal: **decisive games, net 15W/185L
(7.5%) against v1.** The four PPO correctness bugs from §21.1 were the earlier
half of the work; the collector now also saves the policy that generated the
data, which is what makes the ratio correct.

One more real fix: the critic target was in game-reward units while the advantage
carried accumulated GAE magnitude, so with a mean reward of −0.69 on a losing
rung the value loss sat at **160+** every iteration and swamped the policy term
(−0.004) by four orders of magnitude. Normalising `ret` on the same scale as
`adv` brought it to **1.004**, stable. That is the kind of bug that makes RL
"not work" for reasons no amount of hyperparameter tuning will fix.

### 21.7 And the honest result: the ladder measures, but PPO regressed the net

With the ladder measuring properly, the paired comparison at matched settings
(argena, `NEURAL_EPSILON=0`, no pass guard, 400 games vs v1):

| net | result | win rate |
|---|---|---|
| cold init (Xavier, never trained) | 189-211 | **47.3%** |
| after 5 PPO iterations on the v1 rung | 150-250 | **37.5%** |

**PPO made it worse by 9.8pp.** The collector's own reported figure (10.0%) is
*not* the same measurement — the collector samples with `PPO_EPSILON=0.15`
exploration noise while the arena plays the argmax, and §20.4 already showed
that pass rate is the dominant variable — so the two numbers are not comparable
and the collector's is not evidence of anything either way. That is itself a
finding: **the collector reports a metric that does not predict arena strength**,
and a training loop tuned against it would be optimising the wrong thing.

The policy-term trace says why: `policy −0.0042 → −0.0055` with `|dlogratio|`
climbing **4.5 → 41.3** over five iterations. The ratio drifting that far inside
a clipped PPO update means the per-chunk advantage normalisation is re-centring
each batch, so every chunk pushes in whatever direction its own mean sits, and
the policy walks away from the behaviour policy it was supposed to stay near.

**Not shipped.** The ladder is a working measurement harness and it is worth
keeping; the PPO loop on top of it regresses and is not ready. What would need to
hold before another attempt:

- stop normalising advantages per chunk (it is the cause of the ratio drift), and
- use the argmax policy for evaluation, not the exploring one, so the metric the
  trainer optimises is the metric the arena scores.

Two honest lessons, both worth more than the result: **RL's failure modes here
are bookkeeping, not algorithms** — five distinct silent bugs, every one of which
looked like "learning isn't helping"; and **an agent that is at 47% against a
weak opponent after zero training is a much more informative starting point than
the 32% the cloned policy managed against its own teacher.**

---

## 22. The learned evaluation function — the intervention the evidence supports

### 22.1 Why not a policy, restated by the literature

Both RL attempts replaced the *policy* and both failed for the same structural
reason (§20.4): v7 decides Pass through a delta leaf, which a level model cannot
represent. Zhang & Buro published exactly that measurement — the **same** network
is worth **18.8%** standalone (Table V) and **57.2%** placed inside a search that
kept its hand-written evaluation (Table VI). Janusz et al. ran both in the same
lab: their LSTM policy reached 23-26% against a search agent even when retrained
on that search's own data, while their value net dropped only 0.79 → 0.75 AUC
across a near-total policy-distribution shift.

Expert Iteration (Anthony et al. 2017) makes the general statement: "value
networks act to reduce the required search depth" where "a policy network allows
us to narrow the search" — and **value-and-policy significantly outperforms
policy-only**.

So: keep the search, replace the leaf's opinion. `Search::evaluate` in
`v7_main.rs` is exactly the drop-in point, and it is inert by default — verified
684-516 / 40898 actions, unchanged.

### 22.2 Four corrections the research forced, and they mattered

The first value trainer reached Pearson r ≈ 0.235. Four changes from
Świechowski et al. (arXiv 1808.04794) and AlphaZero/MuZero took it to **0.336**:

| | first attempt | corrected |
|---|---|---|
| target | gamma-discounted reward | **raw terminal outcome {-1,+1}** — MuZero: "in board games the discount is assumed to be 1" |
| normalisation | per-chunk z-score | **none** — the value is in [-1,1] because the target is |
| optimiser | plain SGD 3e-4 | **Adam, lr 1e-3, batch 256** |
| data | every state of every game | **3 per game**, reservoir-sampled |

The discounted target was the substantive error: it asks the net to predict its
own distance from the end of the game rather than the outcome. Since `gamma^k > 0`,
the *sign* of the stored reward recovers the undiscounted outcome exactly, so the
fix costs nothing at collection time.

### 22.3 And it does not yet win

| arm | result (150 games, seed 11) | actions |
|---|---|---|
| baseline v9 | **88-62** (58.7%) | 5558 |
| + learned value (r=0.336, w=250) | 85-65 (56.7%) | 6693 |

Slightly worse, and **~3x slower** — the leaf runs a 2248x512 forward pass on
every candidate evaluation.

The diagnosis is data, not method. [P5]: "to train a value function without
severe overfitting requires more than 10^5 **independent** samples." This net
trained on **4,488** — roughly 20x short — and [P1] supplies the other half of
that warning: "consecutive states are highly correlated. Random sampling is
required." Capping at 3 states per game was the right move and made the corpus
much smaller than it looks; the 139MB of trajectories is ~1,500 games, not the
~90,000 independent samples the byte count suggests.

Also worth recording: [P1]'s combination ablation found the learned value *alone*
beat value+hand-written (82.0 vs 78.4 as P1), i.e. **stacking two heuristics was
not reliably better than the better one alone**. If the value net ever works here,
it may be worth testing `V7_LEAN`-style replacement as carefully as the sum.

### 22.4 The iterative loop, now running

`tools/analysis/value_loop.ps1` implements Świechowski §IV-C as a persistent
background job: fresh games → sample → retrain from scratch → **measure in the
arena** → keep the best checkpoint, never assuming the last round wins. They
trained 64 rounds and the best was **iteration 21**, which is the reason the
script ranks by win rate and copies `value_best.bin` rather than overwriting.

It is running: 8 rounds, 800 games each, 50 epochs, against v7. Progress is in
`training/iterloop/loop.log`. **No win is claimed from it until a round beats
58.7% on a multi-seed paired run.**

---

## 23. Closing the neural detour, and the 16th live-set null

### 23.1 The learning attempts, all measured, none shipped

The user is right that this was the wrong priority, and the record says so:

| approach | result | vs v7 |
|---|---|---|
| behaviour cloning (71k steps) | 53.2% holdout top-1, **35.5%** play | far below |
| PPO on a measured ladder rung | **regressed** 47.3% → 37.5% | far below |
| learned value in the leaf, weight 250 | 56.7% (150 games) | noise |
| learned value in the leaf, weight 1000 | **39.5%** (400 games) | regression |

And the controlling fact is not the net's quality but the reference point: **an
untrained, randomly-initialised net beats a random player 90-10** (270-30), so the
encoder genuinely learns the game — and v7 still beats it 84-16. The literature
explains the gap rather than excusing it: Zhang & Buro measured the *same*
network at **18.8% standalone and 57.2% inside a search** that kept its
hand-written evaluation. v9 *is* that search. Learning was being pointed at the
one part of the bot that was not the problem.

Everything shipped from that detour is capability, not strength: `init_random`,
`save_weights`, `ppo_train`/`ppo_learn`/`value_train`, the ladder, and the
`V8_LEARNED_VALUE` leaf hook — all defaulted **off**, with the arena verified
bit-identical (684-516 / 40898) at every step.

### 23.2 External agents: surveyed, and none are usable here

GitHub is reachable. Real card-game simulators and agents exist — SabberStone
and Fireplace (Hearthstone), poke-env (Pokémon Showdown), gym-locm, Slay the
Spire agents — but every one targets a **different game**, so none can be seated
as an opponent in this engine. The ladder therefore still runs against v1..v9,
which are all this project's own heuristics; that is a real limitation of the
training setup and worth stating rather than hiding.

The one transferable artefact is gym-locm's `encode_card`, fetched in full: a
**17-dim attribute vector with 6 explicit ability-keyword bits and card ID
deliberately excluded** — the encoding behind ByteRL's double championship. This
repo's is the inverse, a 128-dim learned ID embedding with no ability bits on
cards. That is a genuine design difference and the most concrete transfer
candidate identified. It is **not** implemented, because the research on this exact
question is discouraging (§21.3: three published negative results on adding
features) and this game's abilities are trigger-based, which is the documented
unsolved case for learned ability encoding.

### 23.3 The 16th null: a measured defect, correctly fixed, worth nothing

`score_dist` — the opponent's expected score, which decides the score comparison
in 51.2% of live phases — is derived from the same `yield_prior`/`board_total`
that §14.7 found inverted. Measured over **187 committed opponent sets**:

- model's expected opponent score **1.67**, realised **2.60** → **−0.93 bias**
- 47.1% of cases more than a point low vs 11.2% over-high
- **Pearson r = 0.601** — so unlike `pass_prob` it carries real signal

`V8_OPP_SCORE_BIAS` shifts the distribution and corrects the measured bias:

| shift | result | discordant | p | delta |
|---|---|---|---|---|
| +1 | 688-512 | **8 / 12** | 0.503 | +0.33pp |
| +2 | 672-528 | 37 / 25 | 0.162 | −1.00pp |
| −1 | 677-523 | 15 / 8 | 0.210 | −0.67pp |

All null. The +1 arm changed **20 games in 1200** — the smallest intervention
count of anything in this document. So the score comparison, despite deciding
half of all phases and having a measured one-point bias, almost never *binds*:
when v9 passes 91-100% of the checks it takes, most phases are decided before
score is consulted.

That is the cleanest possible statement of where this project stands. **A real,
measured, correctly-repaired defect in the live-set model, and the repair is
worth 20 games.** Sixteen interventions now, all pointing the same way.

### 23.4 What is actually left

Not the live set. Every quantity in it has now been measured against reality:
`p_pass` (inverted, and inert once corrected — §19.6), `p_place` (well
calibrated), score (`score_dist`, biased and inert once corrected), the set
width (tested, flat cap of 2 was p=0.25), and the fold threshold (load-bearing,
`V8_NO_FOLD` −4.75pp).

The one structural gap identified in this audit that has **never been
implemented or tested** is the Main leaf. §15.2-A found v9 runs v7's leaf: a
**one-ply** function with `Pass` hard-coded to 0.0, a **colour-blind** hearts
term, a `coverage` term (weight 60, the second-largest) that is a **threshold
rather than a gradient**, and **no term anywhere that varies with the turn**.
`v8_main.rs:1128-1141` documents in-source that a leaf reading "more board is
better" cannot represent "stop", with probe evidence that v9 passing was right
three times out of four.

That is where the next attempt should go, and it is not a switch — it is
rebuilding the leaf. It is also, notably, the *opposite* of the direction this
session spent most of its budget on.




### 11.2 The probe is a localiser, not a rule generator

`PROBE=v9` put v9's Main (v7's) against v8's on the same states. Four games,
four disagreements, three of them v9 passing where v8 kept developing:

```text
t=2 bl=4 cost=9  h=4 lives=3  v9=Pass v8=PlayMemberToStage
t=2 bl=6 cost=9  h=4 lives=1  v9=Pass v8=PlayMemberToStage
t=3 bl=8 cost=15 h=6 lives=0  v9=Pass v8=PlayMemberToStage
t=4 bl=8 cost=21 h=8 lives=2  v9=UseAbility v8=Pass
```

and the health metrics said v9 was right to: **4-0, pace 0.800, stage-cost curve
4.0 / 9.0 / 14.0** — on the guide's band of 4 / 9 / 13. v8 on the same seeds
reaches cost 30 and loses.

So I wrote the rule down — `V8_CONVERT_GATE`: refuse a deploy that closes no
part of the shortfall it is aimed at. Stated directly, it reads like the thing a
player does.

**It costs 20.48 points** (47.9% → 22.0%, p=0.00000) and drops pace to 0.235,
worse than any configuration in this file.

The probe was *right* — v9 really does pass more, and stopping at the curve
really is what wins. The **rule was wrong**. A per-action gate cannot see that a
deploy is unconvertible *now* but still correct, because v7's depth-8 chain sees
several plies out while the gate sees one. It fires on members kept for their
hearts next turn, on batons that book a blade, and on development toward a life
not yet in hand. The observation was 3 disagreements in 4 UNTRACED games.

The distinction worth keeping: a decision-level probe tells you **where** two
policies differ, which is what located the ability defect (§3.6) and was worth
+1.00. It does not tell you the **rule** that would produce the better decision
in every state, and turning a three-observation pattern into a hard gate is how
a correct observation becomes a 20-point regression.

---

## 24. Porting from the downloaded repos: one win, one negative, one impossible

`research/` now holds `gym-locm` (ByteRL's encoding, COG2022 double champions,
MIT) and `SabberStone` (search-based, no neural net, the kind that placed in the
Hearthstone AI Competitions, **AGPL-3.0** - vendored and gitignored so no
copyleft source enters this repo's history).

### 24.1 gym-locm card encoding - copied, tested, capability gained

`LOCMEnv.encode_card` -> `encoding.rs::locm_card_features`. A 17-dim attribute
vector per card, **card ID excluded**, with **6 ability-keyword bits inside the
card vector`.

The point for this repo is not ID-vs-attributes (arXiv 2407.05879 measured that
as a wash in-distribution). It is that `AbilityFeatures` is attached **only to
on-stage positions**, so before this port **a card in hand carried no ability
representation at all** - and hand is exactly where the live set is chosen. The
keyword vocabulary is mapped, not copied; the mapping is documented in the
function. 3 new tests, **119/119 green**. No win-rate claim yet.

### 24.2 SabberStone `Score` shapes - both copied, both measured

Two structural properties of `core-extensions/SabberStoneBasicAI/src/Score/`,
ported and gated off by default.

**Board presence as a discrete term - real, and negative.**

| arm | result | discordant | p | delta |
|---|---|---|---|---|
| `V7_PRESENCE=1` | 667-533 | 113 / 96 | 0.268 | **-1.42pp** |
| `V7_PRESENCE=1` occ=20 | 662-538 | 161 / 139 | 0.225 | **-1.83pp** |

Null-to-negative, and **worse at a higher weight** - the shape of a real effect
pointing the wrong way rather than noise. The motivation was sound (��17.3
measured `coverage` as a threshold where a gradient was wanted; ��15.2-A found no
presence term at all), but this leaf already prices `stage_cost` and
`active_blades`, so occupancy is counted, and counting it twice costs more than it
adds. Hearthstone is won on damage; this game on placements. **The shape did not
transfer. Port stays off.**

**Terminal short-circuit - structurally impossible here.**

| arm | result | discordant | p |
|---|---|---|---|
| `V7_TERMINAL=1` | 684-516 | **0 / 0** | 1.000 |

**Zero decisions changed in 1200 games.** Not a null result - a structural
finding. `evaluate` is only reached on non-terminal states, because `complete()`
tests `game_result != Ongoing` first and returns before calling it. The
short-circuit is dead code *by construction*: the ported idea presupposes an
evaluator that sees terminal states, and this one does not.

That is worth more than the null implies. The leaf's inability to express "this
placement ends the game" - the gap I cited when motivating the port - **is not
located in `evaluate` at all.** It is a property of `complete`'s early return, so
the fix belongs there, not in leaf scoring. Sixteen nulls had not surfaced this;
reading a competitor's code did.

### 24.3 Status

Neither port improved v9. The encoding is a real capability gain with no measured
win yet; SabberStone's two shapes are both refuted for this game, one empirically
and one structurally. The repos stay in `research/` because the encoding is the
right basis for a real attempt and `gym_locm/trained_models/` holds the only
permissively-licensed trained card-game policy weights available as a
head-shape reference.

---

## 25. The opponent terms are provably dead code - measured, not argued

��15.2-A flagged that the Main leaf "prices four things about the opponent".
Two of those four turned out to be structurally incapable of affecting a single
decision.

### 25.1 The claim

`evaluate` is called once per candidate action **within one Main-phase turn,
while we hold the seat**. The opponent takes no actions in between. So their
stage cost, blade count and hand size are byte-identical for every candidate,
and

```text
-8.0 * (opp_cost   - self.opponent.cost)     }  identical offsets across
-6.0 * (opp_blades - self.opponent.blades)   }  the whole candidate set,
-10.0 * (opp_hand   - original_opp.hand)     }  cancelling in the argmax
```

are a **constant added to every candidate**. A constant cannot change which
candidate is largest. This is the same shape `v8_main.rs:1058-1063` documents for
`initiative`, and the same defect class `V8_NO_ASSUME_COMMIT` exists to neutralise
in the live set - which `tools/arena_ab.ps1` already sets as the standard run's
baseline, so the project had already met this bug once and not here.

### 25.2 Measured

`V7_OPP_TERMS` removes all three.

| arm | result | discordant | p | delta |
|---|---|---|---|---|
| `V7_OPP_TERMS=1` (terms removed) | 682-518 | 35 / 33 | **0.904** | ?0.17pp |

**35 + 33 = 68 of 1200 games differ, p = 0.904, win rate ?0.17pp.** That is
within noise of nothing. Three terms carrying a combined weight of 24 units
in a leaf where `coverage` carries 60 - and deleting them costs nothing
measurable.

The 68 discordant games (vs 20 for the `score_dist` fix in ��23.3) are not zero,
so the terms are *nearly* dead rather than perfectly dead. The residual is the
one path where `evaluate` sees the opponent move: the choice-handling branches
(`opponent-choice`, `choice-budget`) recurse across turns, so a small amount of
the opponent's state does leak into those leaves. That is a real but tiny
effect, and it is 68 games rather than the hundreds the weight suggested.

### 25.3 And the counterpart that does not exist

The leaf gives OUR side `coverage` at weight 60 - the second-largest term - and
the opponent has no counterpart at all. `V7_OPP_COVERAGE` adds one:

| arm | result | discordant | p | delta |
|---|---|---|---|---|
| `V7_OPP_COVERAGE=60` | 681-519 | 7 / 4 | 0.549 | ?0.25pp |
| same, weights 10 / 30 / ?20 | 681-519 | 7 / 4 | 0.549 | identical |

**Identical records at four different weights** - the signature of a term that
cancels in the argmax, exactly as ��25.1 predicts. The asymmetry is real (our
coverage is weighted, theirs is absent) but *fixing* it is impossible in a
single-turn search, for the same structural reason.

### 25.4 What this actually says about the project

`value()`'s own note warns that "the terms are MUTUALLY REDUNDANT, not inert"
and that zeroing all seven together costs 14 points. That remains true of the
**own-side** terms. It does not extend to the opponent terms, which are
redundant with nothing - they are arithmetic that never reaches the decision.

This is the fourth structural finding in this document that a null result
exposed and reading the code did not (��13.2's missing
`process_pending_auto_abilities`, ��23.3's uncorrectable `score_dist`, ��24.2's
unreachable terminal short-circuit, and now this). The pattern is consistent
enough to state as a rule for this codebase: **a term in this leaf that does not
depend on a quantity the search can move is decorative, and the weights make it
look load-bearing.** There are 24 env-tunable weights in this leaf and 119 tests;
none of either checks that a term can change a decision.

Both switches default off. Baseline verified 684-516 / 40898, 119/119 green.

---

## 26. Reading the games instead of the aggregates - the first real defect

Asked directly whether the choices are handled properly, I went back and read
individual games rather than bucket summaries. Immediately a single losing game
(game 14, v9 lost **0-3**) shows the whole problem in four lines:

```text
t1  NO-CONTEST   P1 set[]                  P2 set[]
t2  P2-WINS      P1 set[]                  P2 set[Special Color:2pt] 3/7 -> succ 1
t3  P2-WINS      P1 set[�n�܂�͌N�̋�] 4/5   P2 set[Aspire:4pt]     6/8 -> succ 2
t4  P2-WINS      P1 set[Special Color]  6/7   P2 set[Aspire:4pt]     8/11 -> succ 3
```

v9 committed THREE checks and placed ZERO times. On every one it was one or two
icons short.

### 26.1 Across 193 live phases, v9 is short on 82% of what it commits

| | v9 (P1) | v7 (P2) |
|---|---|---|
| mean YELL icons revealed | **5.1** | 4.2 |
| mean REQUIREMENT | **6.9** | 5.8 |
| phases short by 1-2 | 83 of 148 (56%) | 81 of 130 (62%) |
| phases short by 3+ | 39 (26%) | 25 (19%) |
| **short on at least one icon** | **122 / 148 = 82%** | 106 / 130 = 82% |

Per card, on the checks v9 actually committed:

| card | n | requirement | **actual yield** | **short by** | placed |
|---|---|---|---|---|---|
| `�n�܂�͌N�̋�` | 49 | 3.5 | 2.6 | **0.9** | 80% |
| `Special Color` | 51 | 7.8 | 5.9 | **1.9** | 65% |
| `Aspire` | 45 | 9.2 | 6.8 | **2.4** | 76% |

The deficit scales with the requirement: 0.9 short on a 3-icon card, 2.4 short on
a 9-icon card. That is a systematic yield shortfall, not bad luck - a bot that
passed its checks 91-100% of the time (��14.1) is not seeing this, because the
model does not model the thing that is actually short.

### 26.2 The cause: yield is computed from a deck-wide average, against a 3-member ceiling

`v7_main::features` computes expected yell supply as

```rust
mean[i] += narrow_hearts(score_to_i32(
    (units[i] * blades as f64 / p.main_deck.cards.len().max(1) as f64).floor(),
));
```

where `units[i]` sums `blade_heart` counts for colour `i` **over all 72 cards in
the deck**, and `blades` is the count of *active members on stage*. Yell can
only ever come from those members' icons, so the true expectation is

```text
expected_yell(colour) = (blade_hearts on the MEMBERS I PLAYED) / (number of those members)
```

The code divides a **72-card** average by the **stage occupancy** instead. Two
consequences, both visible in the table above:

1. **It is diluted by cards that will never be played.** `blade_heart` is counted
   across energy cards, lives, and every member copy not on stage, so the
   per-icon yield is understated for the actual board.
2. **It is not capped by the ceiling.** Yell tops out at what ~3 members can
   produce. A requirement of 9-11 is unreachable from a full board, and the model
   has no term that says so - it just reports a probability.

`pass_estimate_from_supply` in `v8_model.rs` then runs a **binomial over
`blades`** with a deck-average `density`, which is the same averaging assumption
one level up. So both the live set's `p_pass` and v7's `coverage` are computed
from a quantity that is not what the rules use.

### 26.3 Why this survived nineteen interventions

Every ��12-��25 experiment changed a **weight, a cap, or a prior**. This is a
different class of defect: the leaf's `blades` term (weight 6.0) rewards having
more active members, and it correctly gets v9 to fill its board. But *filling the
board* is not the goal - **covering the requirement with the icons on it** is,
and no term in the leaf measures that. `coverage` (weight 60) does measure
whether the board covers a held life's requirement - and ��17.3 established it is
a **binary threshold over the whole requirement**, so a board that covers 6 of 7
icons scores exactly the same as one covering 0 of 7.

So the bot has a weight-60 term that says "is this check coverable" and it is
**on/off**, with no gradient, while the thing that actually determines the
outcome - partial coverage - is invisible. A bot that maximises a binary
threshold will happily sit at 6.8 yield against a 7.8 requirement, because
6.8 and 5.9 both read as "not covered" and neither is better.

**That is the first genuinely new causal story this audit has produced**, and it
is consistent with everything that came before: ��13-��17 found the live set
impossible to improve because the quantity it optimises (`p_pass`) is computed
from a yield model that does not match the rules; ��22-��23 found every calibration
fix inert because the same averaging sits underneath.

### 26.4 What follows from it, concretely

The fix is to compute expected supply from the **members actually in play**:

```text
expected_yell(colour) = sum of blade_heart[colour] over stage members / count
```

and to give `coverage` a **gradient** - hearts covered - so 6 of 7 beats 0 of 7.
Neither is a weight change, which is why nineteen weight/cap/prior
interventions could not have found it. `V7_DIAG_TERMS` exists to confirm the
term-liveness claim empirically; the yield computation has not yet been changed,
because doing so silently would break the frozen 684-516 baseline that every
measurement in this document is quoted against, and it needs its own paired run
rather than a swap.

---

## 27. Move by move: what 193 live phases actually say

§26 read four lines of a game log and concluded v9 is "short on every one" of
its checks. The conclusion is a misparse, and the log line it came from already
contains the answer. This section is the correction, the decomposition it
replaces, the one real modelling error it turned up, and the two nulls that
followed.

Every number here is measured on this build at the frozen baseline
(`bot_arena v9 v7 1200 "5CP3Z idou" --games 1200 --seed 11` -> **684-516**,
40898 actions, reproduced before and after every change in this section).

### 27.1 §26 is wrong, and the four lines say so

The trace field §26 quoted is this one (`live.rs:1440-1460`)::

    t3|...|LIVE P2-WINS | P1 set[始まりは君の空:1pt] yell 4/5flips score=1 → succ=0(+0)
                                 ^^^^^^^^^^^^^^  ^^^^^^ ^^

- `始まりは君の空:1pt` is the engine recording that the life **PASSED**,
  printed with its score. A failure is printed as `:FAIL(c0 0/2, c3 0/1)` and
  scores zero. The engine has been printing the verdict in that field all along.
- `yell 4/5flips` is **blade-heart icons revealed / flips taken** - not
  "4 of 5 needed". Neither number is a requirement, and neither is a shortfall.
- The requirement is in the FAIL detail that only appears on failure, and
  `filled[]` (which includes members' printed hearts) is what the engine
  compares against it.

§26 compared a requirement against icons-revealed, which ignores every heart the
board already had. Measured directly over 200 games / 865 live phases, with both
sides' own verdict fields:

| | v9 | v7 |
|---|---|---|
| committed sets | **592** | 494 |
| passed the heart check | 542 (91.6%) | 464 (93.9%) |
| failed the heart check | **50 (8.4%)** | 30 |

So v9 is short on **8.4%** of what it commits, not 82%. This agrees with §14.1
("passes 91-100%") and disagrees with §26, and the log line §26 quoted is the
evidence against it: **v9 passed both of the checks in game 14 that it
committed** and lost both on score comparison.

The 50 failures are real but narrow: 34 of them are the 7-icon any-colour
bucket on Aspire or Special Color, and 9 happen on turn 1 (21% of v9's turn-1
commits, the only turn where the failure rate is materially out of line).

### 27.2 Where the placements actually go

865 live phases, 200 games, both seats:

| | v9 | |
|---|---|---|
| folded | 273 | opponent then placed **144** of them, nobody 129 |
| committed, opponent had folded | 179 | **163 placed** (91%), 16 lost |
| committed, both set (contested) | 413 | **227 placed** (55%), 186 lost |

**Contested checks are 70% of all commits and carry 94% of all losses.** Of the
186 contested losses:

| cause | n | share |
|---|---|---|
| set a lower printed-score card than theirs, passed, lost the comparison | **88** | 47% |
| equal printed score, lost the +1 yell score icon | 55 | 30% |
| failed the heart requirement | 34 | 18% |
| lost despite a higher printed score | 9 | 5% |

48 of the 55 equal-card losses are exactly **-1**. The score comparison is
decided by a single point (§14.2: +1 wins 33/33, -1 loses 32/32) and half of
those are a coin flip on one score icon. That is the floor, not a defect.

For the 88: joining the arena's own audit rows against the live-result log, **104
of the 152 passed-but-lost checks (68%) had a strictly higher-printed-score life
in hand at set time** (deficits: 29x 2->7, 28x 1->4, 26x 2->4, 12x 4->7, 9x
1->7). This is the largest addressable bucket in the game. It is not
automatically a defect, because the four lives in this deck trade score against
requirement steeply (1pt/3 icons, 2pt/7, 4pt/11, 7pt/15 - score per icon
0.33, 0.29, 0.36, 0.47), and p_pass is computed exactly by the flip sampler.
The model's arithmetic on a 2pt-vs-4pt choice with a contesting opponent does
favour the 2pt, and re-deriving it with the opponent's *measured* score
distribution still favours the 2pt (§27.3). So this bucket is where the next
real improvement has to come from, and it is a *passability* problem, not a
weighting one: the way to make the 4pt and 7pt cards playable earlier is a
bigger board earlier, which is the Main phase, not the live set.

### 27.3 The one real modelling error: the opponent's yield prior is 31% low

v9 prices its own check by sampling the actual deck through `FlipSampler` and
prices the opponent's from `YIELD_PRIOR` - five hand-picked yield values whose
mean is **0.505** blade-hearts per blade, against the ~0.733 this file's own
docstring cites. In a mirror, which is the configuration every measurement here
runs in, the two answers describe the same deck.

Measured by joining `V8_CAL2` against the engine's own verdict log (1324 live-set
decisions, 100 games, seed 11):

| turn | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| modelled E[opponent score] | 0.50 | 0.89 | 1.25 | 1.57 | 2.02 | 2.56 |
| **observed opponent printed score** | **1.00** | **1.56** | **1.94** | **2.49** | **3.40** | **4.21** |
| bias | -0.50 | -0.67 | -0.70 | -0.92 | **-1.38** | **-1.64** |

The error grows with the turn, which is the signature of a per-blade bias:
`blades x (0.505 - 0.733)` is 0.5 hearts at 2 blades and 1.8 at 8. At t4 the
model puts 2.8% of its mass on the opponent scoring 4; they score 4 in 32% of
checks. At t5-t6 the model is 1.4-1.6 points low in a comparison decided by one
point, and it is low **in the direction it keeps losing** - it believes it is
ahead in precisely the comparisons it loses.

Two things make this actionable where `pass_prob` was not (§14.7): the
distribution still carries signal (pearson r = 0.601 between predicted and
realised), and the bias has a *known cause* - a prior that is documented as too
low - rather than being an unexplained inversion.

**The fix**, `V8_OPP_SAMPLED_YIELD`: build the opponent's supply from the deck
actually in play, sampled through the same `FlipSampler` that prices our own
check, at their blade count, instead of from the prior. Same bands, same
half-credit, same weights - only the yield quantiles are measured rather than
asserted. It also fixes a quiet self-inconsistency: the shipped code read the
opponent's yield as 0.505 in the score comparison and (under
`V8_OPP_DENSITY=measured`) as 0.25 in the pass comparison, because
`measured_opp_density` returns *the fraction of deck cards with a blade heart*
and `yield_prior` rescales it as if it were *blade-hearts per blade* - a unit
mismatch that made the "calibration" push the opponent down instead of up. The
switch is off by default, so the frozen baseline is untouched; the default path
is bit-identical and re-verified (684-516 / 40898) with the code compiled in.

**It is a null.** Paired, n=1200, seed 11:

| arm | record | discordant | p | delta |
|---|---|---|---|---|
| baseline | 684-516 | - | - | - |
| `V8_OPP_SAMPLED_YIELD` | 683-517 | 6 / 5 | 1.000 | -0.08pp |
| `V8_OPP_SCORE_BIAS=1` | 688-512 | 8 / 12 | 0.503 | +0.33pp |
| `V8_OPP_SCORE_BIAS=2` | 672-528 | 37 / 25 | 0.162 | -1.00pp |

The constant-bias arms are the control, and they behave exactly as the
turn-resolved shape predicts: +1 helps late and hurts early, +2 hurts more,
neither is significant. Correcting a measurably wrong model changes 11-62 games
out of 1200 and wins nothing.

The reason is visible in the arithmetic rather than hidden: raising the
opponent's score lowers `we_c` for *every* candidate roughly equally, so it
moves the fold boundary, not the choice between cards. The score axis of the
opponent model is measurably wrong and outcome-irrelevant. That is worth more
than a null: it means the next improvement cannot come from modelling the
opponent better, and the 88-check bucket in §27.2 has to be attacked from the
supply side (board size, earlier) or not at all.

### 27.4 A rule the codebase stated wrongly, verified against the engine

`zone_success_distribution`'s justification (`v8_live.rs`, and §11.8) argued
that each live in the zone resolves independently, so three lives can bank three
successes in one phase and `V8_JOINT` prices what the model otherwise cannot.

**The engine does not do that.** `determine_winners` requires
`lives.iter().all(|l| l.passed)`; rule 8.3.16 / Q35 is all-or-nothing, and a set
containing one unsatisfied card places **nothing** with every card going to the
waitroom. Pinned by
`live_success_rules_test.rs::any_card_fails_hearts_all_fail` ("one failure
fails ALL - nothing places, both cards to waitroom"), and visible in a 200-game
trace: **7 sets held a passed life next to a failed one and every one of them
scored zero placements** (116 sets containing a failure, +1 placements: 0).

So the never-sets-two-cards behaviour is *correct*, the "0 discordant games" of
`V8_MAX_LIVES=3` is the rule talking rather than a model that cannot speak, and
`V8_JOINT` is a switch whose stated reason is a false statement about the rules
in the direction that makes it look load-bearing. The comment and §11.8 are
corrected. The switch stays off.

### 27.5 §26's proposed fix is premised on a misread of where yell supply comes from

§26 proposed computing expected yell from the blade-heart icons of the members
**in play**: "yell can only come from the icons on members actually in play, and
the board holds at most 3".

The engine does the opposite. `start_yell` reveals `total_blade` cards **from
the deck** and grants the blade hearts printed on **those revealed cards**
(`live.rs:1905-1920`, `process_yell_revealed_card_icons` on the drawn card).
A member on stage contributes its printed `base_heart` to the pool and its
`blade` to the flip count - its own `blade_heart` icons contribute nothing
(pinned by `distinct_blade_heart_types_three_and_six_gain_heart01_test.rs::
blade_hearts_count_base_hearts_ignored`).

`units[i] * blades / deck_len` is therefore "blade-heart icons per deck card,
times the number drawn" - the right first-order expectation, diluted by exactly
the cards that *should* dilute it. The proposed substitution would replace a
correct deck-level mean with a stage-level mean over at most three members, which
is both a different quantity and a much smaller one. It was not implemented, and
the reason is now a rule reference rather than a preference.

### 27.6 What is left

Ranked by measured size, not by how easy it is to write:

1. **Board size earlier** (Main phase). 88 of 413 contested checks are lost to a
   lower printed score with a better card in hand; the 4pt and 7pt lives are
   unplayable until supply reaches 11 and 15 icons, and supply is a function of
   board plus blades. §2.2 already flags v9's Main-phase model as unproven and
   locates the v7 lead (re-offer and re-evaluate per action, worth 2.3 points in
   v7's own ablation). This is the only lever left that can move the 88.
2. **Turn-1 waste**: 21% of v9's turn-1 commits fail (9 of 61 live cards burned
   before a board exists). Small in absolute terms, but it is a life spent for
   nothing in the phase where the board cannot support a 7-icon requirement.
3. **Dead leaf terms** (§25.1): three of eight contribute nothing measurable.
4. Not the opponent model, not the opponent's score, not set width, not fold
   rate (§14.5), not a 19th weight.

### 27.7 The fold bucket is empty (counterfactual on all 132 conceded checks)

v9 folds 273 times in 200 games. 129 of those are mutual folds, 8 are the
opponent setting and failing, and 4 are the opponent winning the comparison
after v9 folded. The other **132 hand the opponent a success** - 48% of all folds
and 0.66 successes per game.

A concession that size is worth a counterfactual, so: for every one of those 132,
read the audit's own view of the decision (hand lives with their printed scores
and requirement totals, stage members' printed hearts) and ask the only question
that matters - **did v9 hold a card that fits in its realistic supply (stage hearts
+ its own flip count x the measured 0.73 icons per flip) and scores above what
the opponent committed?**

| | n |
|---|---|
| **held a card that would have won the check** | **4 (3.0%)** |
| held a passable card, none scoring higher | 0 |
| held nothing within supply | 128 (97.0%) |

97% of the conceded checks were folds where v9 had nothing playable. The fold
decision is not losing placements; it is declining to burn a card it cannot pass.
This is §14.5's result a third time, and this time read out of played games rather
than inferred from an ablation: **the fold bucket is empty.**

It also means the deck's lives are the binding constraint on the contested checks,
not the decision. With 1pt/3, 2pt/7, 4pt/11 and 7pt/15 icons on the table, a side
whose supply is below 7 icons has exactly one playable card, and a check decided
by printed score against an opponent holding a 4pt is then lost by construction.
Every remaining lever has to raise supply earlier, and that is the Main phase,
where v9 already runs v7's - the one §2.5 shows is the best of the three
available.

### 27.8 External check on the supply model (web)

The community live-success calculator the project already cites for its 0.733
figure (loveca.vercel.app/live) takes exactly four inputs: total hearts required
for the live, stage heart total, **number of yells**, and **blade hearts in the
deck over current deck size**, with the explicit caveat that it totals hearts
rather than counting per colour. That is

    supply = stage hearts + yells x (blade hearts in deck / deck size)

which is the engine's model and strategy_v4::flip_stats's, and it is the model
§26 proposed replacing. An independent tool built by the deck's players prices the
same quantity from the deck, not from the members on stage, which is a third
independent confirmation of §27.5.

The realised yield measured in the 200-game trace set is 0.72-0.75 icons per flip
for both seats, consistent with the 0.733 the prior is calibrated against - so
V8_OPP_SAMPLED_YIELD's null in §27.3 is a null on the *opponent's* pricing, not
a refutation of the 0.733 the comment cites.

---

## 28. The one calibration that was wrong, and a parallel sweep harness

§27 closed the live-set buckets and left the fold decision, the Main phase and
the mulligan as the only things left. This section closes two of those three by
measurement and finds the change that moves the win rate.

### 28.1 The fold bucket and the Main phase are both empty

Two counterfactuals, both read out of the 200-game trace rather than inferred
from an ablation.

**Folds.** v9 folds 273 times. 129 are mutual, 8 are the opponent setting and
failing, 4 are the opponent losing the comparison after v9 folded - and **132
hand the opponent a success**. For every one of those 132, the audit's own view
of the decision was read back: did v9 hold a life whose requirement fits its
realistic supply (stage hearts + its own flips x the measured 0.73 icons per
flip) and whose printed score beats what the opponent committed?

| | n |
|---|---|
| **held a card that would have won the check** | **4 (3.0%)** |
| held a passable card, none scoring higher | 0 |
| held nothing within supply | 128 (97.0%) |

**Main phase.** Across 939 Main-phase passes by v9 in 200 games, the number that
held a member in hand *and* an open stage slot is **0**. The board is limited by
the deal, not by the policy - and the 33-arm sweep confirms it from the other
side: every development weight is flat to negative (V7_DEV_WEIGHT=16 -0.8pp,
V7_BLADES_WEIGHT=10 -2.1pp, V7_AMMO_WEIGHT=15 -1.0pp, V7_NODES=128 -0.2pp).

**Mulligan.** The mulligan keeps up to three lives and dumps members by cost. It
almost never has to choose *which* lives: **4 of 1600 mulligan decisions (0.25%)
face more than three**, so "keep the best three" is a dead lever. It also loses
when swapped for v8's forward walk (V9_MULLIGAN=v8, -3.4pp, p = 0.073), which
is the second independent measurement of that crossover.

### 28.2 V8_ASSUME_COMMIT is calibrated against the wrong games

A 43-arm paired sweep (all against the same-build seed-11 baseline, one arm per
core, 11 minutes a round) produced exactly one arm that moved, and moved
**positive**:

`
V8_ASSUME_COMMIT=0.8    691-509    57.58%   discordant 1/8   p = 0.039    +0.6pp
`

The switch sets the rate at which the model assumes an **unobservable** opponent
(the first attacker has set face-down, so their zone reads empty) will commit at
all. The default is 0.48, and its own comment says where that came from: "over
1762 first-attacker phases the second attacker committed on **48%** of them".

Measured in the matchup that is actually being graded - v9 vs v7, 200 games,
865 live phases, joined on the audit's own is_first_attacker:

| role | commit rate |
|---|---|
| first attacker | 592/865 = **68.4%** |
| second attacker | 557/865 = **64.4%** |

and the second attacker's rate, which is the one the forecast models, is steeply
turn-dependent:

| turn | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| second-attacker commit rate | **29.6%** | 52.8% | 71.8% | 80.6% | 89.3% | **98.0%** |

So the constant is wrong by 15 points on average, and it is wrong by **50 points
at turn 6**, where the model is certain the opponent sets and believes they set
roughly half the time. A single constant cannot fix a curve whose ends differ by
68 points, which is why a constant step toward the truth (=0.8) is worth
+0.6pp while the error remains.

V8_ASSUME_COMMIT_CURVE replaces the constant with the measured per-turn rate
(0.296 / 0.528 / 0.718 / 0.806 / 0.893 / 0.980), applied only when the
commitment is a forecast, exactly where the old constant lived. The curve is the
measurement, not a fit, and the two ends are the interesting ones: turn 1 the
opponent usually folds, which is why v9 should *not* assume a commit early - and
v9 currently fails 21% of its turn-1 commits, burning a live card in the one
phase where the board cannot support a 7-icon requirement. Turn 6 they set
essentially always.

This is the same shape of argument that came back null for the opponent's score
model (§27.3) and the difference is why: there the error was symmetric in
magnitude and the correction moved the fold boundary against its own benefit,
where here it is one-directional across the whole turn range and the constant
step in the correct direction was already the only positive arm in the sweep.

### 28.3 What else the sweep settled

Everything else in the 43 arms is null or negative, and the negatives are
load-bearing - they are the shipped defaults being confirmed as local optima
rather than being untested:

| arm | delta | |
|---|---|---|
| V8_P_FLOOR=0.9 | **-42.1pp** | a fold floor near 1 is a near-total fold |
| V8_P_FLOOR=0.75 | -22.2pp | |
| V8_P_FLOOR=0.6 | -9.1pp | |
| V9_LIVE_ROLLOUT=v7 | -7.0pp | engine-executing live set, 4th independent loss |
| V9_MULLIGAN=v8 | -3.4pp | |
| V8_MAX_LIVES=3 | -1.9pp | width, at every cap |
| V8_MAX_LIVES=2 | -1.7pp | |
| V8_CONTESTED_WIDE | -1.3pp | |
| V7_PRESENCE | -1.4pp | SabberStone's board-occupancy shape |
| V8_ASSUME_SET=2 | -0.5pp | |
| V8_OPP_CLEARED=0.5 | -0.8pp | the clears step function beats a flat rate |
| V7_COVERAGE_GRADIENT=60 | -0.7pp | §17.3's graded coverage, measured: no |
| V7_COVERAGE_GRADIENT=150 | -0.8pp | |
| V8_JOINT, V8_DEFICIT=0.15, V8_OPP_GRADIENT | 0.0pp | 0 discordant |
| V7_LEAN, V7_CHECK_PRESSURE, V7_FORECAST, V7_FUTURE | +0.1 to +0.2pp | inert |

**V7_COVERAGE_GRADIENT is a direct answer to the §17.3 complaint that
coverage is a binary threshold and therefore has no gradient.** The gradient
was built (graded_coverage, 7_main.rs: the fraction of a life's requirement
the board can supply, following the engine's own allocation order) and measured
at three weights: **-0.7pp, -0.8pp, -1.1pp**. The flat threshold is not costing
what the shape argument predicted, because the terms it would have to be traded
against - coverage itself at weight 60, and lades at weight 6 - already
carry the same information. The switch stays in the tree, off, with the
measurement attached, so the argument is settled rather than open.

### 28.4 Harness

	ools/sweep_daemon.ps1 and 	ools/queue_runner.ps1 are the new tooling, both
persistent and both built around one rule: **a measurement that is slower than
the patience of whoever asked for it will not get re-run.** The sweep daemon runs
one arena per core against a SNAPSHOT copy of the binary (so cargo can rebuild
the installed one mid-round), regenerates its own baseline from the same
snapshot, and promotes anything that moves on the selection seed to two holdout
seeds it has never been read against. The queue runner reads arm specs from
	est_output/arms.txt, builds, measures on three seeds, and appends to
	est_output/arms_results.txt, so new arms are queued by appending a line and
are measured without anyone waiting on a terminal.

### 28.5 The commit curve is the default, and it is replicated

V8_ASSUME_COMMIT_CURVE was measured on the selection seed and on two seeds
that had never been read while looking for an effect:

| seed | record | rate | discordant | p |
|---|---|---|---|---|
| 11 (selection) | **691-509** | 57.58% | 1 / 11 | **0.00635** |
| 23 (holdout) | **693-507** | 57.75% | 5 / 4 | 1.00000 |
| 31 (holdout) | **650-550** | 54.17% | 4 / 10 | 0.17957 |

All three seeds move the same way, and the discordant games are lopsided in the
same direction on every one: 1 vs 11, 5 vs 4, 4 vs 10. Pooled, 10 baseline-only
against 25 candidate-only, exact McNemar p = 0.017. The change fires rarely -
twelve of 3600 games - and when it fires it is right about three times out of
four, which is the profile you want from a calibration fix and the opposite of
the profile of the nulls in §28.3, where the discordant games are balanced or
lean the wrong way.

The curve is now the **default**, with V8_ASSUME_COMMIT_CURVE=0 restoring the
old constant and V8_ASSUME_COMMIT=<r> still overriding both. The frozen
684-516 record is therefore retired as the shipping baseline and the new one is
whatever the queue runner writes next; every measurement quoted in §28.3 was
taken against the old baseline and stays valid as an ablation of the OLD
constant, which is exactly what those arms are.

What the change does, in the game's own terms: it stops the model from believing
that an opponent who is visibly building toward a live will sit out. At turn 6
it believed they would commit 48% of the time when they commit 98% of the time,
and at turn 1 it over-estimated a commit (29.6% real). The published doctrine
says the same thing from the other end - do not waste a live card on a score you
know you cannot beat, and hold the card for the check where you can
(note.com/miya_duel/n/n1cefd065b054) - and the fix is in the same place the
doctrine points, the moment where the bot decides whether showing up is worth a
card.

**Twenty interventions in this project changed a weight, a cap or a prior. This
one changed a number that was measured on the wrong games.**

### 28.6 The same experiment run in reverse

With the curve shipped, the arm V8_ASSUME_COMMIT=0.48 re-runs the OLD
behaviour against the NEW baseline. It is the mirror image, on every seed, to
the game:

| seed | old constant, against new baseline | curve, against old baseline |
|---|---|---|
| 11 | 681-519, disc **11/1**, p = 0.00635 | 691-509, disc **1/11**, p = 0.00635 |
| 23 | 694-506, disc 4/5 | 693-507, disc 5/4 |
| 31 | 644-556, disc **10/4**, p = 0.17957 | 650-550, disc **4/10**, p = 0.17957 |

Net -15 against net +15, with the discordant counts transposed on all three
seeds. The same games flip in both directions; this is the same effect measured
twice, once from each side, not two effects.

**The shipping baseline is now 691-509 (57.58%) on seed 11, up from 684-516
(57.00%).** The old record is retired rather than broken: it is still exactly
reproducible with V8_ASSUME_COMMIT_CURVE=0, which is what the §28.3 arms
measure against and what §28.6's reverse arm just measured.

### 28.7 The band-crossing pair, and the dead branch that hid it

V8_BAND_WIDE allows a two-card set in a contested check when - and only when -
the pair's printed score reaches a band that no single card in hand reaches. The
card pool is what makes it expressible: 5CP3Z idou has four lives printing
**1, 2, 4 and 7**, and both seats run the same list, so single-card scores and
the opponent's scores can only ever be 1, 2, 4 or 7. Pairs reach 3, 5, 6, 8, 9,
11, 14 - and only **5, 6** (over a 4-point opponent) and **8, 9** (over a
7-point one) clear a band. In exactly those cases a single card is not losing by
a point, it is losing by construction, so the comparison is a band test rather
than a margin. The published doctrine says the same thing from the play side:
hold the life for a late multi-card live that outscores them rather than
spending it on a card that cannot (note.com/miya_duel/n/n1cefd065b054; the
turn 5-6 lines of the miumi_loveca meta report, where 11-13 point sets are the
plan once supply reaches 21-31 hearts).

**The first version of it measured 0 discordant games on all three seeds.** Not
because it does nothing useful - because the pair candidates were dropped at
ENUMERATION (cap = max_lives.max(contested_lives) = 1) before the per-candidate
width gate the rule widens ever saw them. A rule priced correctly and wired
behind a cap is not implemented. This is the third time this tree has hit that
trap (V7_LIVE_LEGACY, V7_MULLIGAN_COVERAGE, now this), and the pattern is
worth writing down as a rule: **any new candidate needs the enumeration cap
raised in the same change, and the first measurement after any cap change is
suspect until a non-zero discordance appears.**

With the cap raised:

| | seed 11 | seed 23 | seed 31 |
|---|---|---|---|
| band-wide OFF | 691-509 (57.58%) | 693-507 (57.75%) | 650-550 (54.17%) |
| band-wide ON | **696-504 (58.00%)** | **696-504 (58.00%)** | **654-546 (54.50%)** |

Paired discordance 7/12 (p = 0.359) and 5/8 (p = 0.581). **The direction is
consistent on all three seeds and the paired test is not significant**, so this
is shipped on mechanism and direction rather than on significance, and it is
re-measured every sweep round rather than filed. That is a deliberate departure
from this file's usual bar, and the reason it is defensible here is that the
change can only ever widen in the narrow band-crossing case, the alternative is
provably dominated (a single card that cannot reach the opponent's band loses
the comparison for certain), and it fires on 32 discordant games rather than on
one or two.

The band threshold itself is a measured knob, not a guess: V8_BAND_MIN_MASS
0.05 gives 683-517, 0.15 (default) gives 696-504, 0.25 gives 692-508. The middle
is the peak, which is what a genuine optimum looks like and not what a noise
pattern looks like - a noise peak would not be bracketed on both sides.

### 28.8 Where the win rate stands

| build | seed 11 | seed 23 | seed 31 |
|---|---|---|---|
| original (0.48 flat commit rate) | 684-516 (57.00%) | - | - |
| + per-turn commit curve | 691-509 (57.58%) | 693-507 | 650-550 |
| + band-crossing pair | **696-504 (58.00%)** | **696-504 (58.00%)** | 654-546 |

**+1.0 point on the shipping configuration**, from two changes, neither of which
is a weight: one is a constant that was measured on the wrong games, the other
is a fact about which score bands a pair of cards in this deck can reach.

### 28.9 Two more one-sided omissions, one neutral and one significantly negative

**V8_OPP_SCORE_ICONS** - the [score +1] icons in the yell are added to OUR
total before the comparison and never to the opponent's. Measured over 1212
committed sets: the realised score exceeds the sum of the committed lives'
printed scores by **+0.41**, median 0, with 35% of sets banking at least one,
and 48 of the 55 equal-printed-score losses were exactly -1. Crediting the
opponent with the same term: 692-508 / 697-503 / 655-545 against the default's
696-504 / 696-504 / 654-546, net +1 discordant over 3600 games. Better on two
seeds, worse on one, indistinguishable. Left in the tree, off, with the number.

**V8_BAND_DECK** - derive the score-band table from the lives in the pool
instead of the guides' constants, which are two hearts optimistic in the middle
of the 2-point band (5 against 7) and the 2-point life is the one this deck
commits most (200 in 200 games). Measured: **687-513 / 686-514 / 649-551, net
-20**, discordant 14/6 (p = 0.115) and 18/8 (p = 0.076) - worse on all three
seeds and consistent about it.

The negative is more informative than the direction suggests. Making the table
*stricter* lowers the predicted opponent score, which is the same direction as
the §27.3 sampled-yield fix - and that fix was a **null** while this is a clear
loss. So the response is asymmetric, and the asymmetry is the answer: the
hand-entered table is already at the bottom of the plateau, raising it does not
help and lowering it hurts. Any further work on the opponent's score axis is
working on a dimension that is already saturated, which is worth more than the
null itself - it is a closed door rather than an open one.

**The band-mass threshold is a genuine local optimum**, which is worth recording
because a noise peak would not be bracketed:

| V8_BAND_MIN_MASS | 0.05 | 0.15 (default) | 0.25 |
|---|---|---|---|
| seed 11 | 683-517 (56.92%), disc 16/3, **p = 0.0044** | 696-504 (58.00%) | 692-508 (57.67%) |

Significantly worse on the low side, neutral on the high side, best in the
middle. That is the shape of a parameter that is doing something, and the
opposite of the shape of the arms in §28.3 that moved nothing at all.

### 28.10 The opponent's PASS rate was the miscalibration, and why fixing it changes little

V8_ASSUME_COMMIT was the first of three constants in this model that were
calibrated on games other than the ones being played. The second is the
opponent's pass probability, and it turns out to be wrong by more than the first
was.

8.4.3.2 / 8.4.6.2 make a sole setter win on passing alone, so a phase where they
set and we folded is a clean measurement of their pass rate. Over **323 such
phases** in the 200-game trace:

| turn | 1 | 2 | 3 | 4 | 5 | 6 | all |
|---|---|---|---|---|---|---|---|
| pass rate | 85.4% | 93.3% | 97.0% | 93.8% | 90.0% | 75.0% | **91.3%** |

against the model's own V8CAL2 telemetry, which reads **0.58 at turn 3 and
0.61 at turn 4**. A thirty-point miss, in the direction that makes the bot fold:
it believes an opponent who has visibly committed will fail about two times in
five.

This also explains the two earlier failures on this axis. §27.3 corrected the
*yield* feeding clears (31% low) and measured a null; V8_OPP_CLEARED=0.5
measured -0.8pp. Both are consistent with the real diagnosis - clears is a
supply test whose input was slightly wrong, while the number itself is a third
wrong, and a prior that is 31% off and a constant that is 30 points off deserve
different amounts of attention.

V8_OPP_PASS_CURVE floors pass_prob at the measured per-turn rate, for an
OBSERVED commitment only. Measured: 696-504 / 695-505 / 655-545 against the
default's 696-504 / 696-504 / 654-546, discordant 7/8, 4/3, 4/8, **net +4** over
3600 games. Leaning positive, not significant, and left off.

**Why this is a small effect when the commit curve was a point is the useful
part.** The commit constant feeds the *card selection* and the *whether-to-show-
up* decision; the pass constant only moves the fold boundary. And §27.7 measured
the fold boundary directly: of the 132 checks v9 conceded by folding, **128
(97%) were folds where it held nothing within supply**. A more accurate belief
about whether the opponent will pass cannot buy a check that cannot be passed
anyway. The lever was not the belief; the lever was the supply, and the
measurement said so before this arm existed.

That is the third time a correct-looking fix has come back small after the
bucket was measured first, and it is the argument for measuring the bucket
before writing the fix: §27.7 predicted this null before it was run.

### 28.11 The measurement basis was wrong, and it is the same mistake as §14.1

Everything above is quoted at **1200 games on seed 11**, which is the
configuration every other number in this file uses. Re-running the same binary
and the same seeds at **3000 games** says the absolute record is not that
reliable:

| | seed 11, n=1200 | 6000 games, seeds 41+53 |
|---|---|---|
| current default | 696-504 (**58.00%**) | 13545-12251 (**52.51%**) |

A 5.5-point gap between a 1200-game draw and a 6000-game draw of the same
build, on the same distribution. And it is not the new build that is unusual -
the ORIGINAL 684-516 (57.00%) at n=1200 is the same kind of lucky draw. The
seed-11 1200-game record runs about five points hot.

**The paired tests are unaffected, and that is the only reason anything above
stands.** A paired run holds the 1200 games fixed and flips only the change, so
game-difficulty sampling cancels exactly. The commit-curve result is a 1/11
discordant split (p = 0.006) that reproduced as an exact 11/1 mirror when the
old constant was put back, and the band-wide result is a consistent direction on
three seeds. Those are real. **The headline "+1.0 point" is not** - it is a
comparison of two lucky draws, and the honest statement is:

> two changes that each flip a dozen or fewer games out of 1200, in the right
> direction, replicated in reverse, worth something on the order of a third of a
> point rather than a point.

	ools/queue_runner.ps1 now takes @games= and @seeds= directives for
exactly this reason. The first high-power run immediately earned its cost:
V8_OPP_PASS_CURVE, which measured **net +4 over 3600 games** (two seeds
better, one worse, every p above 0.38), comes out **net -13 over 15,000** at
3000 games x 5 seeds, discordant 69/56. The +4 was noise, and at n=3600 it was
not resolvable - a +0.1-point effect needs roughly 20,000 paired games, not
3600. Without the high-power arm it would have been shipped on a favourable
draw, which is the whole failure mode of §14.1 wearing a different hat.

This is the third time in this file that the number, rather than the code, has
been the defect: §14.1 trusted a convenient adjacent field, §16.8 read a null
from the wrong run, and this one trusted a 1200-game record. **The paired test is
the measurement; the record is an anecdote.**

### 28.12 The whole session, reverted: n=1200 cannot resolve a third of a point

§28.5 through §28.7 shipped two changes on the strength of 1200-game paired
runs. Both are now **reverted**, and the reason is the most useful measurement
in this section.

**What was shipped, and what it looked like:**

| | seed 11, n=1200 | discordance |
|---|---|---|
| commit-rate curve | 691-509 (57.58%) | 1 / 11, p = 0.006 |
| + band-crossing pair | 696-504 (58.00%) | 7/12, 5/8, 4/10 |

**What 30,000 paired games says** (@games=3000 x 10 seeds, both changes off
against the shipped default):

| seed | old (both off) | discordance |
|---|---|---|
| 11 | 1674-1326 (55.80%) | 36 / 15, p = 0.0046 |
| 23 | 1658-1342 (55.27%) | 22 / 16 |
| 31 | 1611-1389 (53.70%) | 34 / 25 |
| 41 | 1706-1294 (56.87%) | 29 / 24 |
| 53 | 1658-1342 (55.27%) | 33 / 22 |
| 67 | 1672-1328 (55.73%) | 31 / 20 |
| 79 | 1686-1314 (56.20%) | 31 / 33 |
| 97 | 1656-1344 (55.20%) | 25 / 20 |
| 103 | 1742-1258 (58.07%) | 23 / 21 |
| 113 | 1645-1355 (54.83%) | 27 / 17 |

**Pooled: 291 baseline-only against 213 candidate-only, net -78, exact McNemar
p = 0.00059 - the wrong direction, at a power the n=1200 run could not have
detected.** Re-verified on a clean run with row counts checked 3000/3000 on
every seed: identical, net -52 over the five seeds, seed 11 still 36/15.

The 1200-game run saw **12** discordant games and split them 1/11. The
30,000-game run found **504** and split them the other way. The p = 0.006 was
real arithmetic on twelve games, and twelve games cannot distinguish a
quarter-point effect from a coin with tails weighted 0.9.

**What is shipped now: nothing.** engine/src/bot/v8_model.rs and
8_live.rs are back at HEAD, and the arena reproduces the original record
**684-516 / 40898 exactly**. Every candidate - commit curve, band-wide pairs,
opponent pass curve, opponent score icons, deck-derived bands, score-weighted
resale - measured null or negative once it was run at a power that could see it,
and the two that looked best at low power were the two that reversed.

The measurements are kept, in the code as reachable switches where they were
reachable, and in this file. The commit-rate curve is a correct fact about the
matchup (64.4% overall, 29.6% at turn 1 rising to 98.0% at turn 6) and the
opponent's pass rate really is 91.3% where the model reads 60%. **Knowing a
constant is wrong is not the same as knowing which direction the error pays in,
and at 0.25 points the answer was the opposite of the arithmetic for both.**

### 28.13 What this session established, and what it did not

**Established, by measurement:**

1. §26's "82% short" is a misparse of the trace line. The set[...] field
   records the verdict (
ame:Npt = passed, :FAIL(c0 0/2, ...) = failed) and
   yell 4/5flips is icons revealed over flips taken. Over 200 games and 865
   live phases v9 **passes 91.6%** of what it commits, against v7's 93.9%.
2. **Contested checks are the whole game**: 413 of 592 commits, carrying 94% of
   all losses. Of the 186 contested losses, 88 are a lower printed score, 55 are
   an equal-card loss decided by a single +1 score icon (48 of them exactly
   -1), 34 are requirement failures, 9 are worse than bad luck.
3. **The fold decision has nothing in it.** Of the 132 checks v9 conceded by
   folding, **128 (97%)** were folds where it held nothing within its realistic
   supply. Confirmed by the counterfactual, not inferred from an ablation.
4. **The Main phase never idles**: 0 of 939 Main-phase passes held a member in
   hand with an open slot. The board is limited by the deal.
5. **The mulligan is not a lever**: 4 of 1600 decisions face more than three
   lives, and v8's forward-walk mulligan is 3.4pp worse.
6. **§17.3's graded coverage is right in form and wrong in effect**: built and
   measured at three weights, -0.7 / -0.8 / -1.1pp. The flat threshold is not
   costing what the shape argument predicted.
7. **Set width is correctly at one**, and the justification for V8_JOINT was a
   false statement about the rules - 8.3.16 / Q35 is all-or-nothing, pinned by
   ny_card_fails_hearts_all_fail and by 7 played games where a set held a
   passed life beside a failed one and placed nothing, 116 such sets, +1
   placements: 0. Corrected in the code and in §11.8.
8. **The opponent's yield prior is 31% low** (mean 0.505 per blade against the
   0.733 the file's own docstring cites), and the bias grows with the turn
   (-0.50 at turn 1 to -1.64 at turn 6). Correcting it changes 11 games in 1200
   and wins nothing. The whole opponent-score axis is closed: making the table
   stricter is significantly worse (-20 net) and making it looser is a null, so
   it is already at the bottom of its plateau.
9. **n = 1200 on one seed carries about ±1.2 points of sampling error**, and an
   arm that moves 0.1 points is invisible at that size. V8_OPP_PASS_CURVE
   measured net +4 over 3600 and net -13 over 15000.

**Not established:** any improvement. Nothing in this section is shippable, and
the honest total is one refuted hypothesis, one false rule claim corrected, four
candidate buckets closed by counterfactual, and a measurement harness that can
now resolve a quarter-point effect where the previous one could not.

**What the next session should not do:** run another 43-arm sweep at n=1200 and
believe the ranking. Every negative in §28.3 is measured with ±1.2 points of
error, which means V7_BLADES_WEIGHT=10 at -2.1pp is really somewhere in
[-4, 0] and might be a gain. The surface has to be re-swept at 3000 games
minimum, and the cheap arms re-run first, because a file with nineteen
consecutive nulls at n=1200 and one significant result that reversed at n=30000
is telling you the resolution was the problem, not the search.

### 28.14 A field read as the wrong quantity, for the third time in one session

Mid-session I reported a "13-point first-attacker advantage" and an unexplored
seat lever, on the strength of 113/200 first-attacker wins against 87/200.
That number is an artefact and the claim is withdrawn.

The audit view carries own.is_first_attacker, and it is **phase-local, not the
game's seat**. Across the 200-game trace:

| phase | is_first_attacker = true | false |
|---|---|---|
| Main | 3298 | 3244 |
| LiveCardSetFirstAttacker | 2710 | — |
| LiveCardSetSecondAttacker | — | 2415 |
| MulliganFirstAttacker | 800 | — |
| MulliganSecondAttacker | — | 800 |
| FirstAttackerPerformance | 507 | 351 |

It flips inside a single game, and it is true in 6,542 Main-phase decisions and
false in 3,244 of them. Reading the first decision of a game and treating the
flag as the seat gives 113 vs 87 for no reason, and the 113 games where it read
true happened to be the 113 games P1 won.

The game-level seat is in the **mulligan phase name**, which is decided once per
game (800 / 800 across the trace). Read properly:

| | P1 wins |
|---|---|
| P1 is the first attacker | 30/51 = 58.8% |
| P1 is the second attacker | 83/149 = 55.7% |

**3.1 points, in opposite directions from what the bad read said, and inside the
noise of n = 51 and n = 149.** There is no seat advantage, the bot is not
missing a seat lever, and the axis is closed. The 51/149 split is itself the
arena's rule showing through: the RPS winner takes second and the loser picks at
random (ot_arena.rs, ARENA_ATTACKER_ORDER).

This is the third time in this file's history that a convenient adjacent field
was read as the right one - §14.1 with a debug field, §16.8 with a null read
from the wrong run, and here with a phase-local flag. **The tell is the same
each time: a number that is suspiciously clean, in a table that should be
roughly balanced.** 113/87 is a suspiciously neat split of 200 and nobody
checked whether the field meant what the reader assumed. The correction costs
two minutes of reading the field's own distribution across phases; the false
finding cost a reported claim.

### 28.15 The published doctrine does not transfer to this decklist, and why

The one external strategy source worth reading (note.com/miya_duel/n/n1cefd065b054,
plus the deckbuilding guide at kasiwamoti_y) states its rules in heart counts:
"a 1-point live needs 3-4 hearts, a 2-point live 5-6", a 2-point live is
mandatory on turn 2 because it seals the opponent's 1-point, and a 1-point live
is mostly waste. Those numbers are the guides' averages across real decks.

**5CP3Z idou is not that deck.** Its four lives need, in icons:

| printed score | 1 | 2 | 4 | 7 |
|---|---|---|---|---|
| actual requirement | 3 | **7** | 11 | 15 |
| the guide's rule of thumb | 3-4 | **5-6** | 7-8 | - |

Every band is shifted, and the 2-point one - the card the doctrine makes
mandatory - is two icons harder than the rule assumes. Measured on the
200-game trace, at turn 2 with a 2-point life in hand (141 games):

| | |
|---|---|
| committed it | 107 (75.9%) |
| folded it | 34 (24.1%) |

and the bot is right to fold those 34. Turn-2 supply is a 1-2 member board plus
2-3 flips, roughly 4-5 icons; a 7-icon requirement is not clearable, and the
doctrine's 5-6 makes it clearable. **The commitment rate the doctrine asks for
is not a policy the bot is failing to follow - it is a policy the deck makes
impossible.**

That is the general caution, and it applies to every numeric threshold this
project has taken from an external source: a rule of thumb quoted for the
average deck is a rule of thumb about *the average deck*. The two things worth
carrying across are structural rather than numeric - score bands are reached by
sums of printed score, so a pair can reach a band a single card cannot (§28.7),
and a check is decided by printed score against the opponent's (§27.2), which
holds for every deck and is where the only unexplored room is.

### 28.16 The seat axis, measured instead of inferred

§28.14 withdrew the seat-advantage claim as an artefact of reading a phase-local
field. ARENA_ATTACKER_ORDER=first settles it as a measurement: it forces P1 to
be the first attacker in every game, so if the seat were worth 13 points the
forced configuration would move the win rate by roughly 13 points, or about 2000
games in 15,000.

Measured over 15,000 paired games at 3000 x 5 seeds, row counts verified:

| seed | P1 always first | discordance |
|---|---|---|
| 11 | 1697-1303 (56.57%) | 535 / 558 |
| 23 | 1671-1329 (55.70%) | 547 / 565 |
| 31 | 1665-1335 (55.50%) | 520 / 569 |
| 41 | 1646-1354 (54.87%) | 580 / 521 |
| 53 | 1651-1349 (55.03%) | 573 / 568 |

**Net +26 games in 15,000, discordant 2755 / 2781, p = 0.62.** Not 2000 games;
twenty-six. The seat is worth about a sixth of a point, not thirteen, and the
bot is already indifferent to it.

A withdrawn claim is worth as much as a measurement that confirms the
withdrawal, and this is the one. The difference took one environment variable.

### 28.17 The race-state audit, and what is left

One dimension the decomposition had not touched: **does the bot escalate as it
approaches three?** It should, because at 2-2 a placement ends the game
(1.2.1.1) while at 0-0 it is worth a third, and check_outcome prices that
through ule_payoff reaching 3.

Reconstructed from the trace's own succ=N(+M) running counts, v9's commit rate
by its own success count:

| my successes | opponent | commits | folds | commit rate |
|---|---|---|---|---|
| 0 | 0 | 259 | 199 | 56.6% |
| 1 | 0 | 220 | 56 | 79.7% |
| 2 | 0 | 113 | 18 | **86.3%** |

It already escalates, by 30 points from 0-0 to 2-0, and it does not fold at
match point - 18 folds in 131 two-success phases, which §27.7's counterfactual
says are the 97% with nothing passable in hand. **Closed, and closed in the
right direction: the terminal term is working.**

### 28.18 Where this leaves the search

Every axis available to a parameter or a switch has now been measured at a power
that can see a quarter-point effect, and every one is flat or closed:

| axis | how it was closed |
|---|---|
| live-set choice | 88/413 contested losses are a lower printed score and the arithmetic defends it; 104/152 passed-but-lost had a better card, and a better card costs p_pass |
| score comparison | 55 of the losses are a single +1 icon, 48 of them exactly -1 |
| requirement supply | 8.4% of commits fail against a model calibrated to 0.92 - that is the variance tail, not a bias |
| fold rate | 128/132 conceded checks had no winnable card in hand |
| Main phase | 0/939 passes idled on a playable member; every development weight flat at 18,000 games |
| mulligan | 4/1600 decisions face a real choice; v8's alternative is 3.4pp worse |
| set width | correctly one, with the rule pinned and the false justification corrected |
| opponent pass rate | measurably wrong (60% vs 91.3%) and worth nothing: the fold bucket it feeds is empty |
| opponent score / bands | asymmetric response - stricter is -20, looser is a null, so already at the plateau |
| seat order | worth 26 games in 15,000 |
| race state | already escalating correctly |
| published doctrine | does not transfer; this deck's lives are 3/7/11/15 icons, not 3-4/5-6/7-8 |

**The remaining lead is not a parameter.** It is the one this file named in §2.6
and never took: *sequencing, not valuation*. v7 beats v8's Main because it
re-offers and re-evaluates after every action in a chain, while v8's
8_main scores one action and takes the best of a single re-offer's leaf. v9
already runs v7's Main, so the deficit is not in what is being shipped - it is
that **the only Main-phase policy in the tree that re-evaluates is being
measured against a v8 baseline rather than against v9's own live set.** A
v7-Main plus v8-live ablation with V7_LIVE_ROLLOUT semantics is the arm that
was never run, and it is a structural change rather than a weight.

### 28.19 The calibration harness could not see the shipping bot, and what it says

live_calibrate joins what the live-set model predicted against what the engine
did, per check, and prints a reliability table for p_pass, p_place,
opp_pass and opp_place. It is the right instrument and this session should
have reached for it in the first hour.

It also reported **0 rows for --p1 v9**, and said so: "no rows: a tool
reporting zero is a bug until proven otherwise." The harness was right and the
question was wrong. live_calibrate.rs:386 matched only BotKind::V8, while v9
ships **v8's live set** (egistry.rs), so the predictor was correct and the
match was not. One line, and the calibration of the bot this project actually
measures became available for the first time.

**400 games, 5332 checks. OUR p_place: mean predicted 0.569, observed
0.310.**

| predicted | n | observed | bias |
|---|---|---|---|
| 0.001 | 1396 | 0.000 | -0.001 |
| 0.149 | 56 | 0.411 | +0.262 |
| 0.257 | 126 | 0.270 | +0.013 |
| 0.352 | 113 | 0.159 | **-0.193** |
| 0.446 | 109 | 0.202 | **-0.245** |
| 0.558 | 283 | 0.290 | **-0.268** |
| 0.652 | 465 | 0.327 | **-0.325** |
| 0.753 | 623 | 0.396 | **-0.357** |
| 0.851 | 916 | 0.397 | **-0.454** |
| 0.958 | 1245 | 0.573 | **-0.385** |

**The bot's belief that a check it has committed becomes a placement is
optimistic by up to 45 points, and the two most confident buckets - 2161 of 5332
checks - are the worst.** Every live-set decision is priced on this number. The
self-check explains the mechanism in one line: **2007 of 5332 checks passed and
did not place** - they were lost comparisons - and the model's
p_place = p_pass * (1 - opp_pass) + contested * we_c prices the lost
comparison away.

p_pass itself is fine: mean predicted 0.670, observed 0.687, Brier 0.1139. So
the pass model is honest and the *place* model is not, and the gap between them
is the lost comparison.

**And here is the mistake in §28.10, caught by the instrument.** opp_pass:
mean predicted 0.477, **observed 0.563** - the model UNDER-predicts the
opponent's pass by 8.6 points, with +0.519 bias in the 0.25 bucket. Under-
predicting opp_pass inflates the (1 - opp_pass) term, which is the branch
where the opponent folds and we place regardless of score. That is the whole
p_place optimism, and it is one wrong number in one place.

§28.10 answered this with a floor at 0.913, measured from 323 sole-setter
phases, and it measured **-13 games in 15,000**. The floor was wrong because the
population was: a sole setter passes far more often than an opponent who has
chosen not to be one, and reading pass rate off sole setters is the §14.1 error
in a new costume. **The opponent passes 56.3% of the time, not 91.3%.**

### 28.20 A 26-point calibration error that changes nothing

V8_RECALIBRATE=0.55 scales the model's own placement estimate by the ratio
its measured reliability implies, before the value is built. §28.19 measured
p_place at 0.569 predicted against 0.310 observed, optimistic by 27 to 45
points across the range the decisions are made in, and 2007 of 5332 checks
passed without placing.

Measured, 3000 games x 5 seeds, 15,000 paired games:

| seed | record | discordance |
|---|---|---|
| 11 | 1668-1332 (55.60%) | 36 / 30 |
| 23 | 1652-1348 (55.07%) | 37 / 36 |
| 31 | 1610-1390 (53.67%) | 39 / 33 |
| 41 | 1697-1303 (56.57%) | 44 / 36 |
| 53 | 1657-1343 (55.23%) | 39 / 40 |

**Net -20 in 15,000, discordant 195 / 175.** Neutral.

The arm was reverted. What is left is the finding, and it is a strange one worth
stating plainly: **the bot's live-set decisions are nearly insensitive to the
scale of the probability they are priced on.** Two reasons, and both are visible
in the table.

The reliability curve is monotone above p = 0.5 - 0.56 to 0.29, 0.65 to 0.33,
0.75 to 0.40, 0.85 to 0.40, 0.96 to 0.57 - so a rescale cannot reorder the
candidates, and WHICH life is set was never a function of the absolute level.
And the fold threshold is not set by p_place either: the change moved 370 of
15,000 games' outcomes, 2.5%, which is about how much a probability rescale can
reach when the other term in the comparison (the life burn price and the filter
value) is doing the work.

That is the third independent confirmation of the same thing. §27.2's
decomposition said the card choice is defended by the p_pass arithmetic.
§14.5 said the fold rate is load-bearing and a floor costs 42 points.
§28.20 says the decision variable is optimistic by 26 points and it does not
matter. **The live set is not mis-tuned; it is insensitive**, and the reason a
hundred arms have moved nothing is that there is very little between the
candidate and the fold boundary for a number to move.

### 28.21 What is kept from this session

One line of tooling, in a binary, with no effect on any bot:

`
live_calibrate.rs:386   if live_phase && kind == BotKind::V8 {
                    ->  if live_phase && matches!(kind, BotKind::V8 | BotKind::V9 | BotKind::V10) {
`

--p1 v9 reported **0 rows** and said "a tool reporting zero is a bug until
proven otherwise" - the harness was right, the question was wrong, and v9 ships
v8's live set so the predictor was already correct. The fix makes the
calibration of the shipping bot available for the first time, and it is what
found the 26-point error in §28.19 and the 8.6-point error in opp_pass. Two
hours of log archaeology would not have produced that table; one line and one
command did. **The instrument existed. It had never been pointed at the bot that
ships.**

### 28.22 The recalibration ladder, and what "0 discordant" means here

| V8_RECALIBRATE | paired games | net | discordant |
|---|---|---|---|
| 0.40 | 15,000 | 0 | **0 / 0 on every seed** |
| 0.55 | 15,000 | -20 | 195 / 175 |
| 0.70 | 15,000 | -18 | 135 / 128 |

k = 0.40 returns **byte-identical records to the baseline on all five seeds** -
1674-1326, 1653-1347, 1616-1384, 1705-1295, 1656-1344 - and zero discordant
games in fifteen thousand. A smaller factor changing strictly less than a larger
one is not a dose-response curve, and the reason is in the value function:
p_place * PLACEMENT_CREDIT is one positive term in a sum whose other terms do
not move, so the commit survives until the term stops mattering at all, and below
that point the fold is decided by the life burn price and the filter value
instead. The arm's effect is a step, not a dial, and the step is at the wrong
end.

Either way the conclusion of §28.20 holds and is now sharper: **the live-set
decision is not a function of the placement probability's level.** The 26-point
error in §28.19 is real, measured over 5332 checks, and three different
corrections of it - a 0.913 floor on the opponent's pass rate (§28.10, -13),
a 0.55 rescale of our own placement estimate (this section, -20), and a 0.40
rescale (this section, inert) - all land inside noise.

### 28.23 The Main phase, audited move by move

The live set got a decomposition and a counterfactual. The Main phase got
weights. Here is what it gets when it gets the same treatment - 200 games, 1156
Main phases, joined to the outcome of the live phase that follows.

**Does deploying a member change whether the next check converts?**

| turn | no member played | member played |
|---|---|---|
| 1 | 46/61 (75%) | 46/61 (75%) |
| 2 | 71/102 (70%) | 69/99 (70%) |
| 3 | 102/145 (70%) | 100/142 (70%) |
| 4 | 99/140 (71%) | 90/124 (73%) |
| 5 | 54/85 (64%) | 53/83 (64%) |
| 6 | 18/47 (38%) | 17/44 (39%) |

**Nothing, at any turn.** The 70% that carries turns 2 to 4 is the check, not
the Main phase. By count there is a mild positive trend - 0 members 55.6%, 1
member 65.5%, 2 members 70.0% - and it is confounded with the turn, because a
second member costs more and comes later.

**Does using an ability?** A usable ability was on the table at **982** Main
decisions and v9 used it at **131** of them, 13.3%. Declining it 851 times is
either correct valuation or a blind spot, so:

| | next check converts |
|---|---|
| used an ability (87 phases) | 58 placed / 29 lost = **66.7%** |
| declined it (334 phases) | 229 placed / 105 lost = **68.6%** |

**2 points, the wrong way, on a third of the sample.** Using an ability does not
make the following check more likely to convert, so the 87% decline rate is not
costing placements either.

That is the whole Main phase. It plays 1290 members and 131 abilities across 200
games, and neither the members nor the abilities move the number that decides the
game.

### 28.24 The ceiling, stated as a structural claim

Every dimension of this game has now been audited rather than tuned, and the
result is consistent enough to name:

| what | what the audit found |
|---|---|
| live-set card choice | defended by the p_pass arithmetic; 55 of 186 losses are one +1 icon, 48 of them exactly -1 |
| live-set fold | 128 of 132 conceded checks had nothing within supply |
| live-set probability | 26 points optimistic over 5332 checks; three corrections, all inside noise |
| Main-phase members | no effect on the next check at any turn |
| Main-phase abilities | used 13% of the time it could, worth -2 points |
| Main-phase idling | 0 of 939 passes had a playable member in hand |
| mulligan | 4 of 1600 decisions face a real choice |
| seat order | 26 games in 15,000 |
| race state | already escalates 56.6% -> 86.3% with its own success count |
| opponent pass rate | wrong by 8.6 points; correcting it changes nothing |
| opponent score / bands | asymmetric response, already at the plateau |
| set width | correctly one; the rule is pinned and a false justification corrected |
| external doctrine | does not transfer; this deck's lives are 3/7/11/15 icons |

**The claim these support is not "v9 is well tuned". It is that the live check is
decided by the cards in hand, the supply on the table and the opponent's set,
and that at 55.5% there is very little slack left between what the bot decides
and what the engine then does.** A policy that is insensitive to a 26-point error
in its own decision variable (§28.20), to whether it deployed a member this turn
(§28.23) and to whether it used an ability (§28.23) is not going to be moved by
another hundredth of a point of tuning.

What that leaves, honestly, is not another arm. It is a different class of
policy: a Main-phase planner whose objective is the supply the board will have at
the NEXT check rather than the cost curve of the current turn. That is a
rewrite, not a switch, and it is the only thing in this list that has a
mechanism large enough to be worth writing.

### 28.25 Testing the rewrite's premise before writing the rewrite

§28.24 named one lead with a mechanism big enough to be worth building: a
Main-phase planner whose objective is the supply the board will have at the NEXT
check rather than the current turn's cost curve. A rewrite is worth writing only
if supply has marginal value, and that is measurable before a line of it is
written: bucket every committed check by **supply minus the requirement of the
life it actually set**, where supply is stage hearts plus the check's own flip
count times the measured 0.73 icons per flip.

200 games, 403 committed checks:

| slack (icons) | placed | lost | place rate |
|---|---|---|---|
| **-2 to -1** | 11 | 23 | **32.4%** |
| **0 to 1** | 145 | 54 | **72.9%** |
| 2 to 3 | 94 | 48 | 66.2% |
| 4 to 5 | 64 | 28 | 69.6% |
| 6 to 7 | 30 | 6 | 83.3% |
| 8 to 9 | 23 | 7 | 76.7% |
| **10 to 11** | 9 | 13 | **40.9%** |
| 12 to 13 | 7 | 5 | 58.3% |
| 14 to 15 | 4 | 4 | 50.0% |

**Conversion is flat above zero slack and gets worse past ten.** The entire 17.6
point spread is between "cannot quite pass" (32%) and "can just pass" (73%).
Supply past the requirement buys nothing at all, and at large slack it costs -
because a bigger board is a bigger board for the opponent too, and the check is
a score comparison.

**So the planner's objective has no marginal value above the requirement, and the
rewrite is dead.** This is the same fact §28.23 found from the other end - the
Main phase's member deployments do not change the next check's conversion - and
the two together are why: supply above what the check needs is not the binding
constraint. The binding constraint is the score comparison, and the only way to
win more of those is to hold a card printing more, which is a property of the
four lives in this decklist rather than of anything a policy does.

Ten minutes of measurement retired the last lead. That is the right order.

### 28.26 The late-game score ladder, and the one gap that is v9's

The competitive reports are consistent about shape: an aggro deck takes 1-2
points on turn 1, a mid-range deck 5 points on turn 3 and 9-10 on turn 4, a
range deck 13-15 on turn 5, and the current A-RISE build 19 points on turns 5-6.
**Every winning deck is a score ladder indexed by turn**, and the note that
"a 1-point lead lets you spend a turn on your hand and hold your next live card"
is the tempo argument for the early point.

5CP3Z idou is a scaled-down version of the same shape - four lives printing
1, 2, 4, 7, and a pair reaching 2, 3, 5, 6, 8, 9 - so the ladder is what this
deck can do. Measured from the 200-game trace, the printed score of what each
bot actually committed:

| turn | v9 mean | v7 mean | n (v9 / v7) |
|---|---|---|---|
| 1 | 1.00 | 1.00 | 48 / 46 |
| 2 | 1.54 | 1.56 | 94 / 87 |
| 3 | **2.03** | 1.94 | 136 / 117 |
| 4 | **3.14** | 2.49 | 132 / 111 |
| 5 | 3.23 | **3.40** | 88 / 88 |
| 6 | 4.00 | **4.29** | 44 / 45 |

**v9 out-scores v7 on turns 3 and 4 and is out-scored on turns 5 and 6** - by
0.17 and 0.29 points, on identical commit counts. The per-turn distributions say
where: at turn 6 v9 reaches 4 in 24 checks and 7 in 6, while v7 reaches 4 in 25
and 7 in 7, but v7 also takes 6-point sets four times to v9's once and 3-point
sets three times to v9's six.

This is the one measured gap in the whole file that points the bot's own way,
and it points at the constraint §27.2 identified as binding: **the check is a
score comparison, and the comparison is lost on late turns.** 88 of 413 contested
losses are a lower printed score, and the late game is where the scores get high
enough for that to bite.

The mechanism available is the one §28.7 reached and could not make pay: a pair
reaches a band a single card cannot, and at turn 5-6 the supply (measured mean
15-16 at T5, 18-20 at T6) is large enough to clear **1pt + 4pt = 14 icons for a
score of 5**, which is the band that beats a 4-point opponent and which no
single card in this deck can reach. The unconditional version of that rule
measured null, because it fired everywhere including turns 1-4 where the gap does
not exist. **The gap is turn-gated, so the rule should be too.**

V8_LATE_BAND=<turn> is the arm: the band-crossing pair, allowed only from that
turn onward. It is the only idea left with a measurement behind it, an external
strategy source, and a rule reference.

### 28.27 The first real win: the band-crossing pair, gated on the turn

V8_LATE_BAND=5 is §28.7's band-crossing pair, allowed only from turn 5
onward, aimed at the late-game gap §28.26 measured. 3000 games x 5 seeds,
15,000 paired games, row counts verified:

| seed | record | discordance | p |
|---|---|---|---|
| 11 | 1688-1312 (56.27%) | 6 / 20 | **0.0094** |
| 23 | 1662-1338 (55.40%) | 2 / 11 | **0.0225** |
| 31 | 1620-1380 (54.00%) | 13 / 17 | 0.58 |
| 41 | 1716-1284 (57.20%) | 6 / 17 | 0.035 |
| 53 | 1655-1345 (55.17%) | 19 / 18 | 1.00 |

**Pooled: 46 baseline-only against 83 candidate-only, net +37, exact McNemar
p = 0.0014.** Three of five seeds favourable, two individually significant.

This is the first result in the session that clears the bar §28.12 set: 15,000
paired games, a real discordance, and a p three orders of magnitude better than
noise. It is also the first one where the mechanism was found by measurement
rather than by argument, which is the sequence that made the difference:

- §27.2 measured that the check is a **score comparison** and that 88 of 413
  contested losses are a lower printed score.
- §28.26 measured that v9's score ladder leads v7's on turns 3-4 and **trails
  on turns 5-6** by 0.17 and 0.29 points.
- §28.7 established that a pair reaches a band a single card cannot - 5 is the
  one this deck can reach, and 14 icons is clearable at turn 5 - and measured
  the unconditional version null, with the explanation that it fires where the
  gap does not exist.
- The gate closes that loop: the rule is allowed exactly where the gap is.

**Nothing is shipped yet.** A holdout run on eight seeds that were never read
while looking for this effect (61, 71, 83, 97, 103, 113, 127, 139) is queued
first, because §28.12 is what happened last time a p = 0.006 was believed. If
the holdout agrees, this ships and the new baseline is re-frozen. If it does not,
it is reverted and written up as the fifth null.

### 28.28 SHIPPED: the late-game band-crossing pair

V8_LATE_BAND defaults to **5** now. The confirmation run, on eight seeds that
were never read while looking for this effect:

| seed | record | discordance |
|---|---|---|
| 61 | 1694-1306 (56.47%) | 16 / 18 |
| 71 | 1681-1319 (56.03%) | 13 / 18 |
| 83 | 1741-1259 (58.03%) | 7 / 15 |
| 97 | 1667-1333 (55.57%) | 8 / 14 |
| 103 | 1741-1259 (58.03%) | 10 / 11 |
| 113 | 1651-1349 (55.03%) | 11 / 15 |
| 127 | 1691-1309 (56.37%) | 10 / 17 |
| 139 | 1683-1317 (56.10%) | 8 / 15 |

**Every one of the eight holdout seeds is favourable.** Combined with the five
discovery seeds, over **39,000 paired games**:

- discordant **129 baseline-only against 206 candidate-only**
- **net +77, exact McNemar two-sided p = 0.0000305**
- **12 of 13 seeds** in the candidate's favour, sign test p = 0.00012
- per-seed nets: +14 +9 +4 +11 -1 +2 +5 +8 +6 +1 +4 +7 +7

The shipped default reproduces the measured arm exactly - 1688-1312, 1662-1338,
1620-1380 on seeds 11, 23 and 31 - and the n=1200 seed-11 record moves
**684-516 (57.00%) -> 687-513 (56.25%)**. 4087 tests pass.

**The mechanism, for the record, because it is the part that is transferable:**

1. The check is a **score comparison** (8.4.3.3), and 88 of 413 contested
   losses are a lower printed score (§27.2).
2. v9's score ladder **leads** v7 on turns 3-4 and **trails** on turns 5-6 by
   0.17 and 0.29 points (§28.26) - the only measured gap where the bot is
   behind itself.
3. This deck's four lives print 1, 2, 4 and 7, so no single card reaches 5.
   A pair of 1pt + 4pt does, for 14 pooled icons (8.3.15), clearable at turn 5
   where measured supply is 15-20.
4. 8.4.7 pays **one** placement for the set, so the pair buys exactly one thing:
   a band. Nothing else.
5. The same rule **ungated** measured null (§28.7) because it fired on turns 1-4
   where there is no gap. **The gate is the entire change.**

That is the shape this file's forty-odd interventions did not have: a loss
decomposed into its causes, the one cause that is the bot's own doing
identified, a rules-derived action that reaches it, and a parameter narrowed to
the range where the measurement says the gap is.

### 28.29 The ladder, for scale

v9 against every other bot in the registry, 1200 games a side, both seats
(	ools/round_robin.ps1):

| pairing | v9 wins |
|---|---|
| v9 vs v6 | 821 |
| v9 vs v7 | 684 (and 684 from the other seat) |
| v9 vs v7rollout | 778 |
| v9 vs v8 | 750 |

**v9 beats every non-mirror bot by 62-100% and its own mirror by 56%.** The
55.5%-against-57% figure this project has quoted all session is the hardest
matchup in the tree, which is the right way round and worth saying before anyone
reads it as a general strength number.

### 28.30 The gap the fix was aimed at is gone

§28.26 named one band where v9 trailed v7: turns 5-6. Re-measuring the score
ladder on a fresh 300-game trace of the shipped build, against the pre-fix
numbers from the same table:

| turn | v9 before | v9 after | v7 | v9 vs v7 before | v9 vs v7 after |
|---|---|---|---|---|---|
| 1 | 1.00 | 1.00 | 1.00 | +0.00 | +0.00 |
| 2 | 1.54 | 1.57 | 1.51 | -0.02 | **+0.06** |
| 3 | 2.03 | 2.12 | 1.91 | +0.09 | **+0.21** |
| 4 | 3.14 | 3.13 | 2.66 | +0.65 | **+0.47** |
| 5 | 3.23 | 3.33 | 3.39 | **-0.17** | **-0.06** |
| 6 | 4.00 | 3.86 | 4.07 | **-0.29** | **-0.21** |

**The turn-5 deficit, the one the rule was gated on, is two-thirds closed**
(-0.17 to -0.06), and turn 6 narrows from -0.29 to -0.21. The bot no longer
trails itself on any turn by more than the noise of a 74-sample.

The mechanism check, in the trace's own terms: v9 now commits a 3-point set 15
times at turn 5 and 10 at turn 6, where before the gate existed the 3-point band
appeared almost nowhere - those are 1pt + 2pt and 1pt + 4pt pairs, the pairs
that reach a band no single card in this deck can reach, at exactly the turns
where the board can pay for them.

**That is the verification a fix like this should get: the change moved the
number the diagnosis was about, in the direction the diagnosis said, by roughly
the amount it said.** §28.27's net +77 and this ladder are the same fact seen
from two sides, which is the difference between a measurement and a
coincidence.

### 28.31 Half the contested checks are decided by one icon, and v9 loses 78% of those

Post-fix decomposition, 300-game trace, 637 contested phases for v9:

| outcome | n | share |
|---|---|---|
| won | 364 | 57.1% |
| lost on a lower printed score | 127 | 19.9% |
| **lost on an equal printed score** | **78** | **12.2%** |
| failed the heart requirement | 55 | 8.6% |
| lost despite a higher printed score | 13 | 2.0% |

Isolating the equal-card population - equal printed score, both sides passed, so
the comparison falls entirely to the yell's score icons:

| | n | v9 wins |
|---|---|---|
| all turns | 349 | **21.5%** |
| turn 1 | 120 | 3.3% |
| turn 2 | 76 | 11.8% |
| turn 3 | 56 | 28.6% |
| turn 4 | 38 | 39.5% |
| turn 5 | 38 | 55.3% |
| turn 6 | 21 | 47.6% |

**A 21.5% win rate where a coin flip says 50%, and 3.3% on turn 1**, on 349
events. That is not variance; a sign test on 349 near-coin-flips does not land
at p = 0.0000 by accident. And it is a fifth of all contested checks, so it is
the second-largest bucket in the game.

The cause is NOT the icons. Scoring every check where both sides passed, the
realised score surplus (total score minus the sum of the committed lives'
printed scores) is symmetric between the seats - +0.012, -0.004, -0.013, -0.039,
+0.091, +0.125 by turn - and v9 banks 0.291 per check against v7's 0.279. **The
icons are not biased, and v9 still loses four of every five.**

What is biased is the shape. At turn 1 v9 is ahead in **4.5%** of these checks
and the mean surplus is **+0.012**: it almost never gets the edge, and when it
does it gets it large. The model prices a tie with a large tie mass, so it
commits checks it expects to place, and the realised tie rate is not what the
tie mass says. The calibration harness agrees from the other side: its
p_place reliability table over-predicts by 10 points in the middle of the
range (predicted 0.440, observed 0.337), which is exactly the region this
population lives in.

**Honest status of this one.** It is a real and large characterisation - 55% of
contested checks are one-icon decisions and v9 loses 78% of them - and the two
mechanisms that would fix it are both already measured null: scaling p_place
(§28.20) and correcting the tie-adjacent probability mass. So this is very
likely another description of the insensitivity §28.20 found, not a new lever.
It is recorded because it is the sharpest single statement of where the bot
loses, and because if anyone revisits this the population to attack is defined:
349 checks per 300 games, worth 0.78 placements each, concentrated on turns 1-4.
