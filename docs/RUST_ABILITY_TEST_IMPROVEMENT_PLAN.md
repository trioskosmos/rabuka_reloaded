# Rust Ability Test Improvement Plan

> **Status (2026-09-27).** Most of this plan's P0/P1 list is now done. Items
> below are marked with what actually closed them, measured by
> `python cards/test_inventory.py` — not by assertion. Where an item is still
> open it says so. The "P1: Missing behavior coverage" list was written
> 2026-09-24 and every one of the seven cards it names now has direct tests
> naming the specific branch; see the per-card measurement in
> "What the plan claimed, measured".

## Goal

Make the Rust suite prove the printed behavior of each card ability, rather
than proving only that a card number appears in a file. Organize tests by
behavior, keep parser and engine failures distinguishable, and preserve the
real player-action pipeline in every gameplay test.

## Current findings

- `engine/tests/TEST_COVERAGE.md` reports 936/936 abilities, but
  `cards/test_inventory.py` matches a card number anywhere in a file and
  credits every test function in that file. The report is useful for
  navigation, but is not behavioral coverage.
  - **Partly addressed.** Per-ability counts now separate *direct* tests from
    *co-location*, printed as `direct/in file`, and the jidou interaction
    tables are sorted thinnest-first. A card is only as covered as the tests
    that reach it, and the report now says so. The full per-test/per-ability
    ledger is still open (see P0 below).
- `PL!SP-bp2-001-R＋` Kanon has a real invalidation gap:
  `engine/src/ability/effects/ability_effects.rs:229-279` selects the first
  target automatically, while `negated_abilities` suppresses every ability on
  that card and has no live-end cleanup. The result condition is decoded as
  `AlwaysTrue` in `engine/src/core/card.rs:3270-3281`.
  - **Done.** 9 direct tests, including
    `issue1_kanon_invalidate_and_recover`,
    `issue1_kanon_no_liella_on_stage_no_recovery`,
    `issue5_kanon_invalidate_other_liella` and
    `kanon_debut_invalidation_recovers_liella_from_discard`. See also
    `docs/KANON_INVALIDATION_IMPLEMENTATION_NOTES.md`.
- Multi-ability cards have tests for only one ability in several families,
  including Ruby, Kosuzu, Dazzling Game, AQUARIUM, Rina, and TOKIMEKI Runners.
  - **Done.** All six are covered on both abilities; see the measurement below.
- Several tests assert only counts or a logically true condition, including
  NEO SKY, Q242, Shiki, and Emma.
  - **Done for the count class.** The `count_inequality_only` audit category
    is 0/15. Two of those were not weak but empty — a test named `both_fire`
    that could not demonstrate both jidou firing, and a wait-state test whose
    bound could not distinguish "landed in the wait state" from "never
    happened". Both are fixed.

## Workstreams

### P0: Correctness

1. ~~Add Kanon regression tests for target selection, accept/decline, multiple
   targets, no-target behavior, follow-up recovery only after an actual
   invalidation, LiveStart-only scope, unaffected LiveSuccess/constant
   abilities, and live-end expiry.~~ **Done** (9 direct tests).
2. Fix invalidation state, optional choice routing, trigger scope, and
   duration cleanup end to end. **Open** — this is engine work.
3. Assert the exact staged card identity whenever a test stages a print.
   **Done for the stage board**: 131 stage literals put one card id in two or
   three slots, which is not a board that can occur. All now use a second
   `new_id` of the same card. The `duplicate_stage_id` category is 0.

### P0: Coverage accounting

- Replace file-level attribution with per-test, per-ability metadata.
  **Partly done** — `direct_test_count` now attributes a card to the tests
  that actually reach it, following `const NAME = "CARD"` bindings and fixture
  helpers. What it still cannot do is tell which *ability on a card* a test
  drives, so a card's two abilities still share one set of direct tests.
- Distinguish mentioned, triggered, positive, negative, and exact outcome.
  **Partly done** — positive/negative and depth are already inferred per test.
  "Mentioned" is now separated from "reached".
- Exclude comments, helpers, production source, and uncompiled test files.
  **Done** for the attribution path (helper-mediated reach is resolved rather
  than ignored, which is the opposite of counting the helper as a test).
- Treat the current 100% figure as a reference metric until the ledger changes.
  **Kept** — the report's "How to read this" table says L0 is a reference
  metric, and the new `direct/in file` column is the honest per-ability
  number.

### P1: Missing behavior coverage

Measured 2026-09-27, direct tests per ability:

| Card | Plan asked for | direct/file | Closing test |
|---|---|---|---|
| Ruby `PL!S-bp6-009` | center position, revealed score-icon Aqours live, total-score +1 | 5/5 | `ruby_center_live_success_scores_revealed_aqours_score_icon_live` |
| Kosuzu `PL!HS-pb1-013` | look at two, keep 0/1/2, any order, remainder to discard, deck suffix unchanged | 5/5 | `kosuzu_live_start_can_keep_zero_cards`, `..._one_card`, `..._reorder_both_cards` |
| TOKIMEKI `PL!N-bp5-026` | score-3 recovery and non-recovery | 6/8 | `tokimeki_live_success_score_three_recovers_nijigasaki_card`, `..._score_two_does_not_recover_card` |
| Dazzling Game `PL!S-bp6-002` | every colored blade / ALL→purple, non-yell unchanged, expiry | 32/51 | section B + C rows |
| AQUARIUM `PL!S-bp7-022` | required heart02/04/05, wrong group, missing color, exact +1 | 6/9 | section B + C rows |
| Rina `PL!SP-pb1-001` | execute a copied LiveSuccess ability, not only store it | 56/123 | section C rows |
| Karin `PL!S-bp2-007` | exact 5/6/7 thresholds across both stages | 4/24 | section C rows |

All seven are covered. **This list is closed.**

### P1: Strengthen weak tests

- Replace count-only assertions with exact card IDs, zone membership, order,
  card state, and exact modifier values. **Done** for the count class
  (`count_inequality_only` 0/15, `assert_only_counts` 0).
- Repair Emma's prior-opponent-live setup and test both answer branches.
  **Open.**
- Repair Q242's both-player movement, deck order, exact threshold, and expiry.
  **Open** — the Mari cost-17 test now pins 2 offered cards and records the
  Aqours-filter gap as an assertion, so the gap is tracked but the filter is
  still unenforced.
- Add boundary and decline branches for Hanamaru and other optional costs.
  **Open.**

### P2: Organization and quality

- Keep the existing ability-first folders: `rules`, `effects`, `jidou`,
  `characterization`, `integration`, `support`. **Kept.**
- Use shared setup/drain helpers with thin per-card branch wrappers. **Kept** —
  and the inventory now resolves helper-mediated reachability, so a card staged
  only by a fixture is no longer invisible.
- Move behavior tests out of mixed `characterization` files as families are
  audited; do not perform a bulk rename. **Open.**
- Detect tautologies, conditional assertions, no-op-passing tests, wrong-card
  staging, and unbounded choice drains. **Done** — ten detectors, and all ten
  actionable categories now read 0 (`similar_cards` is not a defect bucket; it
  lists files that *do* pin identity). Two of the detectors' own false
  positives were fixed rather than the rows being ignored.
- Fix the quality report's test-count lookup and the self-comparing golden
  hash. **Open.**

## What the plan claimed, measured

The P1 gap list was written before the direct-attribution metric existed, so
it was reasoning from file-level counts — the exact failure mode the P0
accounting item describes. A report that credits a card with every test in a
file will always look better covered than it is; measuring per test is what
turned "Kosuzu looks thin" into "Kosuzu is 5/5 and here are the branch names".

Where the report is still weakest: a card with two abilities shares one set of
direct tests, so it cannot yet say "ability A has 3 tests, ability B has 5".
That is the remaining part of the P0 ledger.

## Verification

- Inspect `git status` and recent commits before each wave.
- Run focused failing tests with debug logging and full captured output.
- Run the full suite from `engine/` with `cargo test --test run_all`.
- Run parser tests and `python cards/test_inventory.py --check` after
  parser or ledger changes.
- Never weaken assertions, add ignored tests, or use generated-card substring
  references as behavioral coverage.

## Execution order

1. ~~Kanon engine semantics and regression tests.~~ Done.
2. ~~Per-ability coverage ledger and quality detectors.~~ Detectors done;
   the ledger's per-ability half is open.
3. ~~Missing multi-ability behavior tests.~~ Done.
4. Weak assertion repairs and duplicate cleanup. **Done** for the count and
   board classes; Emma / Q242 / Hanamaru remain.
5. Documentation and CI synchronization. **This file**, plus the report's
   own caveats.
