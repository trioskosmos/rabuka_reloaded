# v8: full context and next methods

Handoff for continuing the v8 work. Written 2026-09-27.

Companion documents: [`V8_IMPROVEMENT_LOG.md`](V8_IMPROVEMENT_LOG.md) is the
record of what was done and what it bought. [`BOT_STRATEGY.md`](BOT_STRATEGY.md)
is the doctrine — the rules this project is trying to follow, and which are
load-bearing. This file is the state of play plus what to do about it.

---

## 1. The situation in one paragraph

Loveca is a two-player card game. You win by being the first to place **three**
live cards into your success zone (1.2.1.1). Each turn both sides run a Main
phase (deploy members) then a Live phase (declare lives, get checked). A check
is won by satisfying a live's heart requirement from a pool of *all* your
members' hearts (3.2) plus flips from *active* members' blades (Q133) — and a
won check places exactly one card regardless of margin (8.4.7). Score only
matters inside a comparison (section 2). The project's bots are v1…v8; v7 is
the strongest measured, v8 is the newest architecture (rule-accurate placement
model + sampled live set) and it **loses to v7**.

| | v8 |
|---|---|
| as originally written | 38.8% (930/2400) |
| **now** | **44.2%** (1061/2400) mirror, **43.4%** asymmetric |
| v7 | 55.8% |

Numbers are 400 games × 3 seeds × both seats, deck `5CP3Z idou`, untraced.

## 2. The two numbers that matter more than the win rate

**Pace: 0.42 placements per live phase, against a target of ~1.0.** Section 1
calls ≤0.33 "a defect, not an archetype". Both bots sit at roughly half. Game
length is healthy (median T6, target T5–T8), so games are not dragging — checks
simply are not converting. This is the largest single number in the project and
it is not a v8-specific problem.

**v7 and v8 choose the same live only 21.4% of the time.** Measured over 70
life-picking decisions. Two cards v7 sets that v8 *never* sets, and two v8 sets
that v7 never does. The cause is located (§4).

## 3. The instrument, and how to use it

Three things were built so that every claim below is falsifiable. Use them.

### `bot_arena` — paired A/B with real statistics

```
bot_arena.exe v8 v7 10 "5CP3Z idou" --games 800 --seed 11 --outcomes base.csv
bot_arena.exe v8 v7 10 "5CP3Z idou" --games 800 --seed 11 --vs base.csv
```

The deal for game N is a pure function of `(--seed, N)`, so two runs on one seed
face identical shuffles and the comparison is **paired** — only discordant games
carry information. It prints an exact McNemar p-value, a Wilson interval, and a
paired bootstrap on pace. **`--vs` requires a fixed `--games`**; a wall-clock
run's `n` differs every time and cannot pair.

`--deck2 NAME` gives seat 2 a different decklist. Use it for every result.

Also reports, every run: placements per live phase per side, game-length
distribution, the average stage-cost curve by turn, and empty-Main / live-fold
rates. These are the guide's own health metrics and they have far more power
than win rate, because they do not depend on who won the race.

### `human_vs_v7` — play a side, with v7's scores

```
human_vs_v7.exe --seed 3000 --deck "5CP3Z idou" --side p1 --diverge 30
human_vs_v7.exe --seed 21 --deck "5CP3Z idou" --side p1 --script my.txt
```

`--diverge N` plays N games with v8 on one side and v7 on the other and reports
every decision where they disagree, grouped by kind, with a per-phase
breakdown and the per-card life-choice split. At 30 games: 1238 decisions, 53.9%
divergence, and **~1/3 of all decisions are in the live-set phase**.

Without `--diverge` you play the side yourself: it prints the board, the options
numbered, **v7's own `score_actions` ranking of every option**, and what v8
would have played, then exits. Append your choice to the script and re-run. The
game replays deterministically, so nothing is lost.

### The mulligan regression tests

`engine/src/turn/mulligan_tests.rs`, 5 unit tests. The mulligan used to replace
the **opponent's** cards (§4). Keep these passing.

## 4. What is known and located

1. **The mulligan corrupted the opponent's hand.** `handle_mulligan_confirmation`
   advanced the phase before mutating, so `active_player_mut()` had flipped
   sides. Fixed, with `GameState::mulligan_owner_index()` captured *before* the
   advance. Measured effect on v8-vs-v7: **nil** — because in a mirror each
   seat corrupted the other, the two cancelled, and the ablation reported the
   broken policy as "neutral". That is the single most important thing to know
   about this project's methodology.

2. **v8 cannot see live-start abilities.** `MIRACLE NE` (v7: 12, v8: 0),
   `始まりは君の空` (4, 0). Every card v7 favours carries a `{{live_start}}`
   ability; v8's live set sees only a flat `0.5 × PLACEMENT_CREDIT` for a
   non-empty ability string, and only inside the junk filter. v7 sees more
   because its live set rolls out and lets the engine execute the live start.
   The `has_unpassable_icon` filter was the obvious suspect and is **not** the
   cause: instrumented, zero lives filtered.

3. **The live set was spending the wrong currency.** Fixed: one life, every
   spare slot on junk. Lives pool their requirement (8.3.15) and a check still
   pays one placement (8.4.7), so extra lives are pure cost; junk consumes no
   requirement and draws a replacement (8.3.4). +15 wins, p = 0.0026.

4. **Cost is the wrong proxy for what a check reads.** v8's Main now values
   `hearts + Binomial(active blades, own density)` in score-band units. +5.4pp.

5. **v8's Main and v7's Main develop an identical stage-cost curve** (entering
   T4 11.7 vs 12.2, T7 24.5 vs 25.9) and v8 still loses ~5pp on that axis. So
   the remaining Main gap is not about development volume.

## 5. How to work on this, learned the hard way

**Restricting the option set beat improving the value function. Nine times.**
Every attempt to make v8's leaf smarter was inert (p > 0.6, several byte-
identical at every weight): two-horizon leaf, `score_ceiling`, `passable_count`,
v7's entire eval ported as a level term, 32→128 nodes, follow-up breadth, live
samples 192→2048, re-shaped burn term, v7's turn-order choice. They are all
monotone functions of the same post-action board quantities the shipped term
already reads, so they rank actions identically and **no weight reorders the
argmax**. The one change that worked added no constant, no term, and no change
to the value function — it stopped the argmax from seeing options it was getting
wrong.

**A mirror hides anything asymmetric.** Use `--deck2`.

**A tool reporting zero is a bug until proven otherwise.** The divergence tool
reported `0/25 wins, 0 divergences` for several iterations because no game ever
started. That looks like a result.

**Reasoning about the game is not a substitute for measuring it.** The
"extra lives are free when the check is contested" argument was sound, guided by
8.4.7, and measured *worse* (354 vs 364, p = 0.052).

## 6. Next methods, ranked

### Tier 1 — the located cause

**1. Project live-start abilities through the engine.** For the top-K candidate
lives by cheap score: clone the state, set the life, run the live-start hook via
the engine, re-evaluate `p_pass` on the resulting board, then score. Replaces
the flat 0.5 with the truth. This is the guide's own open gap and the direct fix
for a measured 21.4%-agreement problem. K≈4 keeps it affordable; the subset
enumeration is the cost centre, so project *after* the cheap argmax, not inside
it. **Expected: the largest single remaining gain.** Validate: `--diverge` life
agreement should rise well above 21.4%, and win rate with it.

**2. Do the same for the Main phase's ability actions.** v8's Main already
clone-executes every offered action including `UseAbility`, so the effect is
already visible in the leaf — but only one ply deep, and abilities whose payoff
is 2–3 turns out are invisible. Widen the follow-up specifically for actions
whose ability text costs energy this turn and pays later.

**3. Replace the junk filter's ability term.** `junk_keep_value` credits
`0.5 × PLACEMENT_CREDIT` for any non-empty ability string. Delete the credit
once (1) lands; if a card cannot be priced, treat it as worth keeping rather
than as worth burning.

### Tier 2 — the untried structural change

**4. Re-enter the placement model at every search node.** v8 scores an absolute
level at the leaf; v7 scores a delta and re-evaluates after each simulated
action. This is the one structural difference left, and the only way to attack
the 81.8% leaf-tie rate. Do it as: value(node) = placement_model(node) +
Σ over the path, so a sequence that improves the board twice scores twice.
Note that scoring a single level by *deltas* is argmax-invariant — the change
has to be that intermediate nodes are re-priced, not that the leaf becomes a
delta.

**5. Refresh the opponent model through the search.** `OppModel` is built once
at the root and never updated. The opponent gets +1 energy (7.5) and +1 card
(7.6) next turn exactly like we do, so the model systematically under-rates
states where they are about to leapfrog — which is the whole T3/T4 race.

**6. Fix the live-set `2 × set_size` opponent target.** The opponent's
committed score is assumed to be twice the number of lives they set. Derive it
from the §3.3 score bands instead.

**7. Derive `YIELD_PRIOR` from the card database.** Five hand-picked
(density, yield) pairs stand in for the opponent's flip yield. The distribution
of legal decks' densities is computable from `cards.json` without reading
anyone's decklist — still fair information under §9.

### Tier 3 — rules v8 ignores entirely

**8. Yell selection (8.3.14).** Which members are active is a *choice*; v8
assumes all of them. Q133 means waiting members contribute hearts but no
flips, so sending the wrong member to wait is a real decision nobody is making.

**9. Ability resolution order (8.3.3).** The guide calls this the #1 untested
rule. All 900+ abilities queue and resolve; order determines which effects land.

**10. Zone-order heart allocation and surplus hearts (8.3.15).** v8 sums all
lives' needs into one pooled vector. The engine allocates per zone in order, and
surplus after one life feeds the next. Pooled ≠ sequential, and this is the same
8.3.15 that the live-set fix turned on.

**11. Initialize Aim (8.4.14) and Swap to Commander (8.4.15).** Unused
entirely. Guide notes members with 4+ blades are almost always relevant.

**12. Position (センター / 左サイド).** The guide flags 65/18 cards. v8 scores
stage areas as interchangeable, and the engine offers them as equal-cost.

**13. JpG-specific card knowledge.** 3-6-9, 12-value scoring, GRY/大会 legality
gates, and which of the 8 decks each game uses. v8 has none of this and neither
does v7, so it is neutral between them — but it is real edge.

**14. Going second as the default.** Section 2 says seeing the opponent's live
before committing is worth it; whoever alone places becomes first attacker
(8.4.13). Worth a measurement rather than an argument.

**15. `has_unpassable_icon` properly.** A Draw requirement is 8.3.12.1 "free
card advantage" — a computable value, not an unpriceable one. Currently such
lives are neither settable nor discardable. Instrumented, this filter is
rarely hit on this deck, but it is a whole card class the bot cannot touch.

### Tier 4 — correctness and hygiene, cheap

**16. Remove the `V8_NO_FOLD` hand-tuned constant.** The code comment admits
"the argmax is the decision, this is the hand-tuned constant v7 needed, and
shipping requires showing the argmax does not need it." Show it.

**17. The `1 << lives.min(16)` silent truncation** in the subset enumeration —
lives 17+ are invisible with no counter. Either enumerate correctly or log it.

**18. Delete the `set_size < 2` clause** in the tie rule, or cite the rule. The
guide and 8.4.7.1 state the tie purely in terms of success counts.

**19. Fix the `V8_NO_FOLD` double-assignment** — `entry.value` is set to
`NEG_INFINITY` and then immediately overwritten, so the floor never applies.

**20. Delete `budget`'s magic `+3`** in `HandContext` (computed, never read)
and collapse the `v7_live.rs` / `v8_live.rs` duplication.

### Tier 5 — measurement, which gates everything above

**21. Evaluate every change asymmetrically.** Mirror hides seat-asymmetric
effects; it hid a hand-corrupting engine bug for the project's whole life.

**22. Hold out seeds.** Three seeds are used for selection. Add a fourth that
is only ever read at the end, or the selection is fitted.

**23. Add a second opponent baseline.** Everything is v8-vs-v7. v6 exists, and
a change that beats v7 while losing to v6 is not progress.

**24. Target compute where it pays.** Live-set samples were inert 192→2048
uniformly, but contested checks are where the tail resolution matters; sampling
contested checks harder may be where the budget belongs.

**25. Track per-card play rates.** The 21.4% agreement finding came from
counting which card each bot picks. Doing that for the Main phase too would
localise the remaining Main divergence the same way.

## 7. Do not retry

Measured inert; recorded so it is not re-derived as if untried.

| attempt | p |
|---|---|
| two-horizon leaf (T+1 and T+3) | 0.45–1.0 |
| `score_ceiling` as the development signal | superseded by `band_progress` |
| `passable_count` (v7's `60 × Δpassable`) | > 0.62 |
| v7's eval ported as a level term | 1.0, identical at weights 0.5–5.0 |
| Main search depth 32 → 64 → 128 nodes | 1.0 |
| follow-up breadth / width | 0.69 |
| live-set samples 192 → 768 → 2048 | 0.63 |
| burn term re-shaped via `best_life_pass_now_and_next` | 1.0, byte-identical |
| allowing 2 lives when the check is contested | 0.052, *worse* |
| taking v7's turn-order choice | 1.0, byte-identical (the arena does not route that phase through the patched entry) |

The informative one is the fourth: v7's Main is the best-measured in the
project, so copying its four terms, its depth *and* its breadth into v8 produced
nothing, while v7's Main still won 5pp. That is what killed the "v8 just needs
a better evaluation function" theory and pointed at §6.4.

## 8. State of the tree

- Tests: **88 lib + 3689 integration, green.**
- Uncommitted work: the mulligan fix, the arena instrument, `human_vs_v7`, the
  `band_progress` term, the live-set width fix, and the docs.
- A peer session has been editing the same tree during this work and has broken
  the lib several times (`score.rs`, `game_modifiers.rs`, `game_modifiers`).
  Build failures in files you did not touch are probably that, not you. Do not
  revert their work; wait and rebuild.
- `docs/V8_IMPROVEMENT_LOG.md` has the full measurement record.
  `docs/BOT_STRATEGY.md` is the doctrine.
