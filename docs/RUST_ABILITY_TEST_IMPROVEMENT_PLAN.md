# Rust Ability Test Improvement Plan

## Goal

Make the Rust suite prove the printed behavior of each card ability, rather
than proving only that a card number appears in a file. Organize tests by
behavior, keep parser and engine failures distinguishable, and preserve the
real player-action pipeline in every gameplay test.

## Current findings

- `engine/tests/TEST_COVERAGE.md` reports 936/936 abilities, but
  `cards/test_inventory.py:838-866` matches a card number anywhere in a file
  and credits every test function in that file. The report is useful for
  navigation, but is not behavioral coverage.
- `PL!SP-bp2-001-R＋` Kanon has a real invalidation gap:
  `engine/src/ability/effects/ability_effects.rs:229-279` selects the first
  target automatically, while `negated_abilities` suppresses every ability on
  that card and has no live-end cleanup. The result condition is decoded as
  `AlwaysTrue` in `engine/src/core/card.rs:3270-3281`.
- Multi-ability cards have tests for only one ability in several families,
  including Ruby, Kosuzu, Dazzling Game, AQUARIUM, Rina, and TOKIMEKI Runners.
- Several tests assert only counts or a logically true condition, including
  NEO SKY, Q242, Shiki, and Emma.

## Workstreams

### P0: Correctness

1. Add Kanon regression tests for target selection, accept/decline, multiple
   targets, no-target behavior, follow-up recovery only after an actual
   invalidation, LiveStart-only scope, unaffected LiveSuccess/constant
   abilities, and live-end expiry.
2. Fix invalidation state, optional choice routing, trigger scope, and
   duration cleanup end to end.
3. Assert the exact staged card identity whenever a test stages a print.

### P0: Coverage accounting

- Replace file-level attribution with per-test, per-ability metadata.
- Distinguish mentioned, triggered, positive, negative, and exact outcome.
- Exclude comments, helpers, production source, and uncompiled test files.
- Treat the current 100% figure as a reference metric until the ledger changes.

### P1: Missing behavior coverage

- Ruby `PL!S-bp6-009-R＋`: center position, revealed score-icon Aqours live,
  and total-score +1.
- Kosuzu `PL!HS-pb1-013-R`: look at two, keep zero/one/two, arbitrary order,
  remainder to discard, and unchanged deck suffix.
- TOKIMEKI `PL!N-bp5-026-L`: score-3 recovery and non-recovery branches.
- Dazzling Game: every colored blade and ALL blade becomes purple, non-yell
  sources remain unchanged, and the effect expires.
- AQUARIUM: required heart02/heart04/heart05 cards, wrong group, missing
  color, and exact +1.
- Rina: execute a copied LiveSuccess ability rather than only storing it.
- Karin: exact 5/6/7-member thresholds across both stages.

### P1: Strengthen weak tests

- Replace count-only assertions with exact card IDs, zone membership, order,
  card state, and exact modifier values.
- Repair Emma's prior-opponent-live setup and test both answer branches.
- Repair Q242's both-player movement, deck order, exact threshold, and expiry.
- Add boundary and decline branches for Hanamaru and other optional costs.

### P2: Organization and quality

- Keep the existing ability-first folders: `rules`, `effects`, `jidou`,
  `characterization`, `integration`, and `support`.
- Use shared setup/drain helpers with thin per-card branch wrappers.
- Move behavior tests out of mixed `characterization` files as families are
  audited; do not perform a bulk rename.
- Detect tautologies, conditional assertions, no-op-passing tests, wrong-card
  staging, and unbounded choice drains.
- Fix the quality report's test-count lookup and the self-comparing golden
  hash.

## Verification

- Inspect `git status` and recent commits before each wave.
- Run focused failing tests with debug logging and full captured output.
- Run the full suite from `engine/` with `cargo test --test run_all`.
- Run parser tests and `python cards/test_inventory.py --check` after
  parser or ledger changes.
- Never weaken assertions, add ignored tests, or use generated-card substring
  references as behavioral coverage.

## Execution order

1. Kanon engine semantics and regression tests.
2. Per-ability coverage ledger and quality detectors.
3. Missing multi-ability behavior tests.
4. Weak assertion repairs and duplicate cleanup.
5. Documentation and CI synchronization.
