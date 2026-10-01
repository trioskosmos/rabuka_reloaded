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
(note.com/miumi_loveca — "1回のエールでブレードハートがめくれる確率"), and
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
| 始まりは君の空 | 4 | **0** |
| Aspire | 17 | 5 |
| 葉月 恋 | 1 | 12 |
| 嵐 千砂都 | 0 | 9 |

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

That is a genuine modelling error. `MAX_LIVE_CARDS` is 3, the win condition is
three successes, and each card is checked and moved independently
(`move_live_to_success_and_handle_wins` sends winners to success and failures to
the waitroom, per card). Three good lives can bank three successes in one live
phase and win outright at 2-2. The model cannot price any of that.

`V8_JOINT` prices it: `zone_success_distribution` resolves a set one card at a
time against a shared supply and returns `P(k successes)`, and
`joint_success_value` pays for the 2nd and 3rd successes with the same race
payoff. It is a real fix to a real defect.

**It changes nothing: 0 discordant games out of 1200, with `V8_JOINT`, again
with `V8_JOINT + V8_MAX_LIVES=3 + V8_ZONE_ORDER`, and again with width alone.**

That is not a null result about the fix, it is a fact about the game. Hearts
come from a **shared** pool: the first card draws until its requirement is met
and the second gets what is left. Two lives therefore rarely both pass, so
`P(k >= 2)` is close to zero across every candidate the live set ever considers,
the joint term is worth approximately nothing, and **setting one good life
genuinely beats setting two mediocre ones**.

So the live set is not leaving anything on the table by committing a single card.
The "the zone is underused" reading was wrong, and it was wrong in a way worth
having measured: three separate mechanisms all landed on the same bit-identical
result, which is a much stronger statement than any one of them alone.

The defect is real and the fix is correct. It is a no-op **for this deck and
this format**, and it should stay switched off — a switch whose effect is
identically zero is a switch that will be misread as load-bearing later.

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
  ライブ開始時 / ライブ成功時 / メイン. ターン1回 is a condition, enforced by
  `turn_limited_abilities_used`, which is cleared at the **end of the live
  phase** (`phases.rs:278`) — so the window spans the whole turn including live.
  Per-*effect*, not per-card: two members granting the same thing pay out once.

### 12.2 The deck's three lives are all live-start, and all keyed to movement

- `Special Color` `PL!SP-bp4-025-L` x4, score 2, need **7 colorless**.
  ライブ開始時: centre Liella members' **printed blade count becomes 3**.
  ライブ成功時: +1 if a centre Liella member moved this turn.
- `MIRACLE NEW STORY` `PL!SP-bp5-024-L` x1, score 7 +1, need **15**
  (heart01x2 + heart02x4 + heart06x4 + colorlessx5).
  ライブ開始時: choose one of heart01/02/06; **every member that moved this turn
  gains that heart**.
- `始まりは君の空` `PL!SP-sd2-023-sd2` x3, score 1 +1, need 3
  (heart03 + colorlessx2). ライブ開始時: **if own success zone has 2+ cards,
  score +5 and the need becomes 12 specific hearts**. The FAQ confirms the
  trigger **cannot be declined** — the FAQ answer is いいえ, it cannot be used
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
| `葉月 恋` cost 2 登場: top 5, reveal 1 Liella card | hand **count** +1, identical for every option | **coin flip** |
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
history that resolves 「このターン中にエリアを移動した」 (`state.rs:1135`), and
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
   「このターン中にエリアを移動した」 is false for every member and MIRACLE and
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
moves 2→4, 4→6, 5→7: `Aspire`'s 「このターン中にエリアを移動した」 +1 blade per
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
| `始まりは君の空` | 3 | 69 | 29 (42%) |
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

§13.3 reported `始まりは君の空` passing 42%, `Special Color` 13%, `Aspire` 8%,
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
| `始まりは君の空` | 78 | 71 (91%) | 54 (69%) |
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
engine. The same correction applies to `始まりは君の空`'s conditional: the audit
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
score 7; `始まり` scores 1-3 at two successes, not 6), and two of my own
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
