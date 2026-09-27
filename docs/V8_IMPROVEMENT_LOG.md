# v8 improvement log

What was actually done to v8, what it bought, and what it did not. Every number
here is a measurement; every claim that did not survive measurement is recorded
as a dead end rather than deleted, because "we already tried that" is the most
expensive lesson there is.

Doctrine lives in [`BOT_STRATEGY.md`](BOT_STRATEGY.md). This file is the
engineering record. What to do next — the located causes and 25 ranked methods
— is in [`V8_NEXT_METHODS.md`](V8_NEXT_METHODS.md).

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
