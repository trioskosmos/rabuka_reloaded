# V7 vs. Loveca Strategy Guide: Complete Gap Analysis

## Scope

This report compares the active V7 policy with the strategy doctrine recorded in `docs/BOT_STRATEGY.md` and the cited Loveca strategy guides.

Active dispatch:

- Main actions: `engine/src/bot/v7_main.rs`
- Live sets: `strategy_v7::choose_live_set_experiment` in `engine/src/bot/strategy_v7.rs`
- Mulligan: inherited V4 policy unless `V7_MULLIGAN_CURVE=1` or `V7_MULLIGAN_REACHABLE=1`
- `engine/src/bot/v7_live.rs` is not the registered V7 live-set implementation.

The central guide principle is not merely “pass a heart check.” It is:

> Develop a reachable curve, model the public board, preserve useful live ammunition, and maximize the probability of winning the actual live comparison.

## Evidence and Safety Rules

- Replay infrastructure is diagnostic, not the reason V7 is weak.
- Randomness must be controlled only enough for fair A/B comparisons and reproducible diagnostics.
- Do not enable a broad heuristic because it sounds human. Prior rejected experiments include:
  - Flat baton bonus `+45`: approximately `57.7%` to `39.5%`.
  - Baton bonus `+12` with ammo guard: approximately `50.5%`.
  - Pass tax / energy tax: `1569/3000` draws in one experiment.
  - Narrow curve-mulligan variant: `2805` wins versus `2853` for V4 over `6000` games.
  - Blanket late-fold experiment: V7 changed from `118–75–7` to `116–76–8` over 200 games.
  - Free-baton override: `434–489` in the tested arena.
  - Beam experiment: `510–383` with stalls.
  - Rollout continuation policies: neutral or worse than default.

## Priority Summary

1. Model actual live comparison outcomes, not only heart feasibility and printed score.
2. Replace cost-only junk selection with future-utility junk selection.
3. Add reachable-development and ability-to-deploy trajectory evaluation.
4. Improve live candidate enumeration, shared sampling, and effective-score projection.
5. Replace rigid mulligan behavior with a reachable-opening evaluator.
6. Add position and initiative awareness after the larger model is stable.
7. Consolidate the active live-set implementation and clean up stale comments/gates.

---

## 1. Duplicate Live-Set Implementations

### Guide

There should be one authoritative implementation whose behavior is tested and benchmarked.

### Current V7

`engine/src/bot/registry.rs:130-138` dispatches V7 to `strategy_v7::choose_live_set`. That function defaults to `choose_live_set_experiment` at `strategy_v7.rs:457-465`. `v7_live.rs` contains a different sampled implementation with no registered public-ceiling, minimum-win, or strict-closeout path.

### Why This Is Weakness

Fixes and audits can target the wrong module. The source tree gives the appearance of two current V7 policies while only one controls the arena.

### Fixes

1. Make `choose_live_set_experiment` canonical and move the useful shared sampling helpers into it.
2. Keep `v7_live.rs` only as a clearly named experimental module and add a registry test proving the production path.

---

## 2. Default Mulligan Does Not Implement Curve Doctrine

### Guide

Develop a connected opening curve, commonly `4 -> 9 -> 13/15`, and retain cards that enable the next development step (`docs/BOT_STRATEGY.md:245-265`).

### Current V7

The default delegates to V4 (`strategy_v7.rs:1291-1295`). V4 generally replaces up to three cards rather than evaluating the reachable opening line.

### Why This Is Weakness

The bot may discard a useful energy, retrieval, or development card while preserving an expensive member that cannot be played on schedule.

### Fixes

1. Replace fixed replacement count with hand-quality evaluation: preserve affordable development, energy, and retrieval pieces; replace only dead cards.
2. Enumerate replacement counts and opening lines over the first three Main turns, ranking by reachable stage cost, hearts, blades, and baton access.

---

## 3. Curve Experiment Is Too Narrow

### Guide

Curve completion is a general property, not only one four-card cost pattern.

### Current V7

The experimental curve path at `strategy_v7.rs:1205-1234` requires a strict four-member sequence with narrow cost gaps. It ignores already-playable cards, two- or three-member lines, energy, retrieval, position, and live value.

### Why This Is Weakness

The experiment can reject useful openings that do not match the local `2+2 -> 7 -> 11` example.

### Fixes

1. Generalize the search to any reachable sequence of one through four members, including duplicate costs and already-playable cards.
2. Simulate the first three Main turns for each candidate opening and score guaranteed development rather than only static cost order.

---

## 4. Main Evaluation Is Mostly One-Ply Aggregation

### Guide

Strong play requires multi-turn compositions such as ability setup, discounted deployment, and next-turn baton development.

### Current V7

The default search has a `64`-node budget and depth up to eight, but leaves are primarily aggregate feature deltas (`v7_main.rs:218-289`).

### Why This Is Weakness

An action that enables a better next action can look worse than a marginal immediate action.

### Fixes

1. Add a bounded two-action search for ability setup followed by member deployment.
2. Add short deterministic guide-policy continuations and score success-zone readiness, curve reachability, and next-baton access.

---

## 5. Baton Vision Does Not Equal Baton Understanding

### Guide

Prefer discounted upgrades that create a stronger next development step.

### Current V7

Baton detection is correct (`v7_main.rs:14-58`), but there is no true baton-efficiency feature. The former flat bonus was harmful, so the current implementation relies mostly on stage-cost growth.

### Why This Is Weakness

A sideways baton or a baton that destroys the next affordable member can look equivalent to a valuable ladder upgrade.

### Fixes

1. Add marginal baton efficiency based on discounted energy spent per unit of stage, heart, blade, and future-ceiling gain.
2. Add a next-step feature that values making another hand member reachable next turn.

---

## 6. Energy Doctrine Is Not Represented Directly

### Guide

Spend energy on useful discounted development, but reserve it for known ability payoffs.

### Current V7

Energy receives a value only for non-deploy actions (`v7_main.rs:189-193`). Member deploys receive no explicit energy tax or positive deploy-side utility.

### Why This Is Weakness

The bot can hoard energy despite having a clearly affordable development action. A blanket Pass tax was already proven harmful, so the missing feature must be positive and selective.

### Fixes

1. For each member play, compare net discounted cost with immediate and next-turn stage/heart/blade gain.
2. Search a short `ability activation -> discounted deploy` sequence and reserve energy only when it enables a known payoff.

---

## 7. Position-Specific Values Are Ignored

### Guide

Center, left-side, right-side, and destination restrictions can make the same card valuable or useless.

### Current V7

`v7_main.rs:175-207` has no position-specific feature even though the engine exposes stage destinations and position-gated abilities.

### Why This Is Weakness

V7 can select a legal but strategically poor destination.

### Fixes

1. Add a destination-aware feature that checks the card’s printed and gained position abilities.
2. Add fixtures where the same member is offered in multiple positions and assert changed preference.

---

## 8. Initiative Is Missing

### Guide

Winning alone changes first-attacker order; a tie does not.

### Current V7

No Main value term or live-set term explicitly values initiative.

### Why This Is Weakness

The bot treats immediate score and pass probability as more important than future attacker order.

### Fixes

1. Add a bounded initiative term to live and Main evaluation.
2. Include turn order in the search leaf and score expected next-turn initiative.

---

## 9. Hidden Opponent State Is Not Fairly Modeled

### Guide

Use public information and determinize unknown cards fairly.

### Current V7

`v7_main.rs:297-310` fills opponent hidden zones with `-1` instead of using the public-observation/determinization path.

### Why This Is Weakness

The state can be structurally invalid, and the bot does not represent uncertainty over anonymous opponent cards.

### Fixes

1. Route Main search through `PublicObservation` and `DeterminizationSampler`.
2. Keep placeholder masking as a fallback and add invariance tests proving hidden card identity cannot change decisions.

---

## 10. Public Opponent Ceiling Is One Crude Integer

### Guide

Estimate public score capability and uncertainty, not only a single expected number.

### Current V7

`strategy_v7.rs:91-109` converts public hearts and blades into a capped integer score ceiling.

### Why This Is Weakness

The ceiling ignores color, unknown flip distribution, committed-set size, score modifiers, and the difference between capability and the opponent’s actual selected score.

### Fixes

1. Return a distribution with a median and conservative upper quantile.
2. Sample anonymous public-compatible opponent deck compositions conditioned on the number of committed lives.

---

## 11. Static Live Pass Model Ignores Live-Start Effects

### Guide

Live-start and refresh effects can change the actual check before comparison.

### Current V7

The active model samples current stage resources and current deck flips (`strategy_v7.rs:696-785`) without projecting live-start abilities, refreshes, or delayed effects.

### Why This Is Weakness

The bot can underrate a live whose abilities make the check reliable or overrate a live whose board changes before performance.

### Fixes

1. Clone the candidate state and project deterministic live-start effects through the engine.
2. Use a conservative uncertainty interval when complete projection is unavailable.

---

## 12. Printed Score Is Not Effective Comparison Score

### Guide

Yell score icons, live-start effects, and score modifiers affect final comparison.

### Current V7

`strategy_v7.rs:799-801` uses printed card score only.

### Why This Is Weakness

A lower printed-score live with strong score effects can be undervalued.

### Fixes

1. Project deterministic score additions and yell score icons for each candidate.
2. Use an effective score interval with printed minimum and projected low/high values.

---

## 13. V7 Does Not Directly Model Comparison Outcomes

### Guide

Optimize the probability of passing and winning the comparison, including tie rules.

### Current V7

The objective is still mostly `P(pass) * printed score`, with only partial ceiling logic. It does not fully model opponent commitment, opponent failure, score distribution, or live-success effects. A separate `V7_PAYOFF_MODEL` experiment now models pass/compare/place payoff, but it regressed and is not default.

### Why This Is Weakness

A safe low-score set can fail to place, while a higher-score set can place when the opponent fails. The current proxy does not represent that tradeoff.

### Fixes

1. Estimate `P(our pass)`, `P(opponent commits)`, `P(opponent pass)`, and `P(we win comparison | both pass)`.
2. Use an explicit payoff table for win, tie, loss, and failed check conditioned on both success counts.

---

## 14. “Minimum Winning Set” Is Reliability-First

### Guide

Choose the smallest or cheapest portfolio that can win or tie, subject to sufficient reliability.

### Current V7

The implementation now defaults to true minimum-first ordering, while `V7_NO_TRUE_MIN_WIN` restores the old reliability-first path (`strategy_v7.rs:1231-1277`). The engine-rule correction also makes strict comparison depend on live-zone size rather than success count: a one-card set can tie-place, while a multi-card set cannot.

### Why This Is Weakness

It can spend extra lives and score margin on a safer set even when a smaller set would win reliably enough.

### Fixes

1. Find the minimum score satisfying the comparison threshold, then maximize reliability among portfolios at that score.
2. Minimize excess score subject to a state-dependent minimum placement probability.

---

## 15. Junk Selection Treats Cost as Deadness

### Guide

Junk means genuinely low-future-value non-lives, not merely expensive cards.

### Current V7

`strategy_v7.rs:825-855` sorts non-lives primarily by descending cost.

### Why This Is Weakness

The bot can discard next-turn baton pieces, energy, retrieval, position-specific members, and useful low-cost curve cards.

### Fixes

1. Rank junk by future utility and preserve energy, retrieval, curve, position, and next-baton cards.
2. Reserve a slot when removing junk would destroy the only playable member or next affordable development action.

---

## 16. Free Wins Skip Junk Filling

### Guide

Second attacker versus an empty opponent zone should take the free win and still use safe spare slots when available.

### Current V7

The early return at `strategy_v7.rs:989-993` bypasses `experiment_junk_fill`.

### Why This Is Weakness

The bot gives up legal card-filtering draws in a state where the live itself is safe.

### Fixes

1. Put the free-win life into `desired`, then run common junk filling.
2. Use a common convergence helper and retain the early return only if the UI cannot represent combined selection.

---

## 17. Only the First Eight Lives Are Considered

### Guide

Candidate lives should come from the complete hand, not arbitrary hand order.

### Current V7

`strategy_v7.rs:883` truncates the candidate list to eight lives.

### Why This Is Weakness

A safe life in hand position nine or later is invisible.

### Fixes

1. Enumerate all combinations up to the legal set-size limit.
2. If needed, sort all lives deterministically and use a bounded beam without hand-order truncation.

---

## 18. Independent 256-Sample Estimates Add Ranking Noise

### Guide

Reliability estimates should be stable enough for floor decisions.

### Current V7

Each portfolio uses `256` samples independently (`strategy_v7.rs:696-785`).

### Why This Is Weakness

Sampling error near 50% is large relative to floors such as `0.35`, `0.45`, `0.60`, and `0.85`.

### Fixes

1. Share sampled deck pools across all candidates in one live-set decision.
2. Use exact or dynamic-programmed color-allocation probability where possible.

---

## 19. One Blade Override Represents All Modes

### Guide

Blade-color transformations depend on the actual board and applicable modifier modes.

### Current V7

`strategy_v7.rs:703-715` chooses one override color and applies it to the whole deck.

### Why This Is Weakness

Multiple possible transformation modes can be misclassified as one mode.

### Fixes

1. Build a distribution over applicable override modes.
2. Extend the engine conversion helper to use full modifier context rather than one collapsed color.

---

## 20. First-Attacker Reliability Is Hard-Coded

### Guide

Reliability should reflect opponent development, ammunition, turn horizon, and initiative.

### Current V7

The first-attacker path uses a hard-coded `0.85` bundle threshold and some raw rather than buff-aware heart calculations.

### Why This Is Weakness

The same probability threshold is used in states with very different strategic costs.

### Fixes

1. Use the buff-aware per-color model for all live-set paths.
2. Make the floor depend on opponent public ceiling, remaining lives, initiative, and end-game timing.

---

## 21. Experimental Gates Are Entangled

### Guide

Experiments should isolate one doctrine component and never silently change production behavior.

### Current V7

`V7_PRE_D`, `V7_NO_CEILING`, `V7_NO_MIN_WIN`, `V7_NO_STRICT_CLOSE`, `V7_D2B`, `V7_ROLLOUT_ASSIST`, and other environment gates interact. `V7_NO_CEILING` also disables minimum-win logic.

### Why This Is Weakness

A benchmark can accidentally test several policies at once, and the meaning of a gate is easy to misread.

### Fixes

1. Replace global environment variables with a typed immutable experiment config passed only to research entry points.
2. Split combined gates into independent switches and make quantitative tooling reject tracing or ambiguous configurations.

---

## 22. Source Documentation Is Stale

### Guide

The active policy should be obvious from the source and architecture documentation.

### Current V7

The top-level `strategy_v7.rs` comments still describe the live set as V6-verbatim, while the active path includes ceiling, minimum-win, and strict-close logic.

### Why This Is Weakness

Future fixes and audits can target behavior that is not actually running.

### Fixes

1. Update source comments and design documentation with the active call graph.
2. Add a registry-level test that names the production Main, live, and mulligan implementations.

---

# Implementation Sequence

## Phase 1: Low-Risk Live-Set Corrections

- Enumerate all hand lives instead of the first eight. **Implemented by default.**
- Share sampled deck pools across candidate portfolios. **Implemented by default** with `V7_NO_SHARED_SAMPLES` rollback.
- Make junk selection future-utility-aware. **Implemented by default** with `V7_NO_FUTURE_JUNK` rollback.
- Fill junk after a free win behind a gate. **Implemented as `V7_FREE_JUNK`; neutral in the 100-game A/B.**
- Remove the single blade-override assumption behind a distribution experiment. **Not implemented yet.**

## Phase 2: Comparison Model

- Project effective score where possible. **Implemented experimentally as `V7_YELL_SCORE`; mixed/slightly negative over 1000 games per seat, not default.**
- Replace scalar ceiling use with median/upper-quantile handling. **Not implemented yet.**
- Add explicit win/tie/loss payoff modeling. **Not implemented yet.**
- Make minimum-win truly minimum-first. **Implemented by default** with `V7_NO_TRUE_MIN_WIN` rollback; behavior-neutral over 2000 validation games.
- Correct tie placement to follow the engine's live-zone-size rule. **Implemented by default; +26 combined V7 wins over the 1000-game-per-seat A/B.**

## Phase 3: Main Trajectory

- Add positive deploy-side energy/affordability utility without taxing Pass. **Tested and rejected** at 1000 games per seat; no longer present.
- Add two-action ability-to-deploy search. **Not implemented yet.**
- Add position and initiative features. **Not implemented yet.**
- Add fair determinized opponent states. **Implemented experimentally as `V7_FAIR_DETERMINIZATION`; neutral/+3 in the second seat but roughly doubled runtime, so not default.**

## Phase 4: Opening and Architecture

- Replace rigid mulligan with reachable-line evaluation. **Implemented as `V7_MULLIGAN_REACHABLE`; mixed result** at 1000 games per seat, so it remains opt-in.
- Consolidate the live implementation. **Not implemented yet.**
- Replace environment gates with typed experiment configuration. **Not implemented yet.**
- Update stale documentation and tests. **Partially implemented by this report and focused V7 tests.**

# Measured Results

## Promoted

- Future-utility junk, 1000 games per seat:
  - Default: V7 `562–381–55` as P1 and `376–548–76` as P2.
  - Future-utility junk: V7 `562–382–56` as P1 and `377–547–76` as P2 in the first comparison; a later 1000-game pair measured V7 `564–381–55` and `376–548–76` without the gate versus `552–383–65` and `357–574–69` with the reachable-mulligan experiment.
  - The clean future-junk A/B was positive overall and is the only substantive live policy promotion from this pass.
- All-life enumeration was neutral on the 100-game sample but removes an arbitrary hand-order dependency.
- Shared sampling was outcome-identical over 1000 games per seat and reduced runtime from approximately 33 seconds to 30.5 seconds.
- True minimum-win ordering was outcome-identical over 2000 games and now matches the guide’s intended ordering.
- Engine-aligned tie placement: the 1000-game per-seat A/B changed from `562–382–56` / `548–376–76` to `575–367–58` / `560–366–74`, a combined `+26` V7 wins.
- The promoted default then measured `1639–1197–164` as P1 and `1607–1206–187` as P2 over 3000 games per seat, or `3246–2403–351` combined.

## Rejected or Left Experimental

- Deploy-efficiency bonus: V7 combined wins changed from `1109` to `1108` over 2000 games; removed.
- Free-win junk filtering: neutral over 200 games; retained behind `V7_FREE_JUNK`.
- Reachable mulligan: mixed by seat, `552–383–65` as P1 and `574–357–69` as P2; retained behind `V7_MULLIGAN_REACHABLE` but not default.
- Fair determinization: `564–381–55` as P1 and `551–375–74` as P2 versus baseline `564–381–55` and `548–376–76`; slight second-seat gain but approximately 2x runtime, retained behind `V7_FAIR_DETERMINIZATION` but not default.
- Explicit payoff model: broad and endgame-only variants both regressed, with the broad version changing combined V7 wins from `1109` to `1077`; retained behind `V7_PAYOFF_MODEL` only for further research.
- Yell-score projection: `563–382–55` as P1 and `547–377–76` as P2 versus baseline `564–381–55` and `548–376–76`; retained behind `V7_YELL_SCORE` but not default.

# Validation Protocol

For every change:

1. Add a focused regression test.
2. Run `cargo test --test run_all <module_filter>`.
3. Run `cargo check`.
4. Run 100-game both-seat A/B on the same seed.
5. Run at least 1000 games in both seat orientations.
6. Run the held-out seed/deck matrix.
7. Keep tracing disabled during quantitative benchmarks.
8. Revert or gate any change that does not improve both seats, held-out seeds, and the relevant decision-quality metrics.

# Current Conclusion

V7 is not weak because it lacks one missing constant. Its strongest legal-rule safeguards are already present, including all-lives-fail-together, second-attacker free wins, strict closeout, and minimum-score enforcement in some paths. The largest mismatch is structural: it still treats “heart check passes with printed score” as a proxy for “wins the comparison and places a card,” while the guide requires a public, uncertainty-aware, multi-turn model of that outcome.

This pass fixed or exposed the low-risk parts of that gap. The next high-value work is effective-score projection, explicit comparison payoff modeling, fair opponent determinization, and position/initiative-aware Main search. Those should be implemented as separate measured changes rather than bundled into one untestable rewrite.
