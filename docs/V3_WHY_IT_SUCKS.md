# Why v3 still sucks — post-mortem after the log-driven iteration

State at time of writing: v3 ≈ v2 head-to-head (~47–51% over large samples),
draws fixed (40% → <8%), no more infinite activation loops. All the
session's patches are in. And it still doesn't play better than the version
it was built on. This document is the honest why.

## The five structural causes

### 1. One-ply greedy cannot see sequences — and this game IS sequences

Every v3 decision clones the state, executes ONE action, evaluates the
resulting position, picks the best. But the guides' winning lines are
multi-turn compositions:

    T2: baton 4→9 (sets up hearts)
    T3: baton 9→11 + play extra member (sets up 3点)
    T4: baton to 13/15 + second member → 5–6点 multi-life check

Each individual action on such a line looks MEDIOCRE in isolation (spending
energy, thinning your hand for +4 stage cost) while its value lands two turns
later in a check you were previously guaranteed to lose. A one-ply eval
scores the photo, not the trajectory. It will always prefer three cheap
independent gains over one enabling move — which is exactly why trace
analysis showed v3 playing members without purpose and passing winnable
checks.

**Consequence:** no heuristic weight can fix this. The information "this
action enables a winning line" does not exist in any single resulting state.

### 2. The eval measures a snapshot; states with equal snapshots diverge wildly

`evaluate_state_v2` sums stage_cost, hearts, blades, energy, hand size,
lives-in-hand. Two positions can tie on every term while one has:

- a curve-complete hand (cost 2/5/11 reachable each turn), and the other an
  unplayable pile of cost-15s;
- lives matching the colors ON STAGE versus lives needing colors it can
  never produce;
- blades on active members versus blades sitting in wait.

The color-coverage term patches one case; the session's failures (hand
liquidation, ammo starvation, dead-life hoarding) were each a *different*
case of the same disease: aggregate statistics hide composition. We patched
composition cases reactively — six times — and each patch re-allocated the
blindness somewhere else (the energy penalty literally converted "hoards
energy" into "spams abilities").

### 3. Ten hand-tuned scalars with zero objective

stage_cost=8, heart=12, blade=6, active_energy=3, hand_size=4,
hand_live_card=15, first_attacker=20, +60/+25/+150 desperation, −1000 no-op,
−40 reserve… Nobody ever defined what these multiply INTO. There is no
target like "predicted placement probability by turn N"; there is only
"number went up". This session proved the failure mode empirically: every
new term measured neutral-to-negative because it perturbs a balance whose
correct setting is unknown. Tuning ten interacting weights through a
one-number fitness (win rate) with ±5% sampling noise is not optimization —
it's dice.

### 4. The opponent exists only as a pressure scalar

S2 of our own strategy doc: "estimate the opponent's max score from their
public board before committing lives." The implementation: when opp has 2
successes, multiply my development terms by 1.6. That is not estimation —
it's panic scaling. v3 never asks "can they outscore my portfolio THIS
turn?", which is THE decision input for contest/concede (tree node 1.4).
Result: v3 contests lost checks and concedes winnable ones at rates a human
beginner would beat.

### 5. Placements happen in the live phase, so live-phase quality caps strength

All three success cards enter via won checks. Boards are fine now (trace
data: both bots track the guide curve within 10%). Yet placements run at
~0.33/turn in bad games because:

- portfolios minimize risk instead of maximizing placed-score expectation
  (v2's ladder ranks pass-probability first);
- life ammunition (12 per deck) is invisible to main-phase choices — v3
  spent its arsenal by turn 4 in game 6 and could not contest for 8 turns;
- retrieval engines exist in the card pool but acquisition deltas valued
  them below a single stage upgrade until the hand was already empty.

## What the fixes taught us (method summary)

Every successful change this session came from reading transcripts:
flip-buffer, no-op breakers, hand-reserve, free-win fallback, ammo rules.
Every failure came from predicting behavior without them. But even the
successful fixes plateaued at parity, because items 1–4 above are not bugs —
they are the architecture.

## The only two moves left that change strength structurally

1. **Simulation-backed decisions.** Clone → execute candidate → let the
   ENGINE resolve (all 900+ abilities handled correctly by construction) →
   drive N turns forward with both sides playing guide-heuristics → score
   outcomes by success-zone delta. Replaces term-guessing with measurement.
   Live-set first (highest leverage, bounded depth), then main phase.

2. **ISMCTS with the current eval as leaf/guide.** The infrastructure
   (`ismcts.rs`, `DeterminizationSampler`) already exists and already
   implements fair-information determinization. Search directly solves
   cause #1 and subsumes #3 (no weights needed except leaf eval), and
   determinized rollouts give #4 for free (opponent plays out their hidden
   info samples).

Between them, neither requires a single new hand-tuned scalar.
