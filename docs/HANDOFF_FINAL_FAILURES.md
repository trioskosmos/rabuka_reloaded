# HANDOFF — final failures (2026-09-22, for a fresh context)

## Branch / tree state
- Branch `master`, ~41 commits ahead of origin. Last commits (newest first):
  `651a2d78` engine mandatory-refusal, `fc0f510a` docs consolidation,
  `8b494b77` L0+Q57 tests, `255d1191` refactor unification.
- Full suite: **3410 passed, 2 failed**. The 2 failures are both in ONE file:
  `engine/tests/test_modules/effects/position/auto_system_stress_test.rs`
  → `step2_p1_then_p2` and `step4_turn2_reset` (P2 legs only).
- Uncommitted work (`git status`): parser.py (PR-025 OR-fix, Shioriko handler),
  abilities.json + bytecode + abilities_gen.rs (regens), engine
  (cost/handlers.rs mandatory validator, abilities.rs just_completed event-scope,
  state.rs group filter + energy menus + rebuild fns, zones.rs energy helpers,
  actions/mod.rs activation gate, game_setup.rs offer gates, util.rs energy
  indices, misc.rs menu labels, move_cards.rs counter honesty,
  helpers/mod.rs new_id template-fallback), ~12 test files, new test files
  (Riko, pb2-020 yell, PR-025 budget, Shioriko×7, energy menus, Natsumi,
  Wien-010), docs/THIN_COVERAGE_ANALYSIS.md (stale in places), docs/ROADMAP.md
  + parity_plan.md + PORT_TO_3DS.md DELETED, engine_c/ → engine_c_wip/ RENAME
  staged. Engine_c binaries were restored to HEAD after a bad `git stash pop`
  of ancient stash@{0} — do NOT pop any stash; tree is clean now.

## The failing tests
File: `engine/tests/test_modules/effects/position/auto_system_stress_test.rs`
- Cards: mover `PL!SP-bp5-006-R` (Kinako: 起動 turn1, mill 3 deck-top → position
  change to a different area, swap if occupied); watcher `PL!SP-pb2-028-N`
  (自動 turn1: during own Main, when this member area-moves → activate 2 energy).
- Failing asserts: step2 line ~151 (`energy_p2 - e2 == 2`, T1 P2 leg) and step4
  line ~251 (same shape, T2 P2 leg). Both got `left: 0` (no activation).
- P1 legs in the same tests PASS. step1/step3/step5 PASS.

## Evidence (debug trace `s4.txt` in temp, key lines)
- T1 P2 leg: mover at LEFT activates, destination choice offers
  `options=2`, answer selects, swap executes (`EPCWD stage:` + 
  `AREA_MOVE_RECORDED card=23581`), watcher fires, +2. WORKS.
- T2 P2 leg: P2 stage `[7983,23581,18314]` (mover 23581 at CENTER), e=18.
  After `activate_and_drain`: stage IDENTICAL, e=18. Earlier probe (since
  reverted) showed `generated_actions()` EMPTY right after activation — no
  destination choice is ever offered, no movement, watcher silent.
- Mover text requires moving to "an area other than the current one"; both
  other areas are occupied (valid swap targets), exactly like the T1 case
  which works from LEFT. Only the CENTER-origin case fizzles.
- P2 energy is genuinely waited at that point (set_active_count(len-2) in
  test), so this is NOT the energy-filter change: the swap itself never
  executes, so there is nothing to count.

## Established root causes (this session, keep)
1. Mandatory-unpayable activations must be refused (Rule 9.4.2.3/Q56), not
   fizzled — `validate_mandatory_cost`, wired into offers + activation.
2. `just_completed_ability_key` never cleared → turn2+ re-fires suppressed.
   Now event-scoped (`just_completed_moved` comparison). q94/s2 4-vs-2
   failures went green with it — DO NOT revert to key-only.
3. Test-id allocator: `new_id` past the 11-copy pool minted colliding/
   data-less ids (per-process flake). Now falls back to template id.
   Budget test rewritten to stay in pool.
4. Energy positional convention (`[0..active)` = active) + purity helpers
   (`push_active/waited`, `set_indices_*`, `remove_at`) in `zones.rs`.

## Hypotheses in flight (unverified — start here)
- H1: the position-change destination builder returns no options when the
  mover starts at CENTER with both other areas occupied (vs LEFT-origin
  working). Read the choice-building code for the swap effect and check what
  differs by origin area. Suspect files: `ability/effects/misc.rs` position
  change handler, `ability/choice.rs` area_select.
- H2: the T2 activation resolves through a different path (use-limit? turn
  reset?) that silently completes. The trace shows NO error and NO choice —
  find where a UseAbility can return Ok with neither.
- H3 (unlikely): P2-side action legality in T2 (active-player resolution);
  T1 P2 works identically though, so discount unless H1/H2 die.

## Suggested next steps
1. Reproduce: `cargo test --test run_all step4_turn2_reset` (fails deterministically).
2. Trace the T2 P2 `handle_use_ability` → resolver → position-change effect;
   log the candidate-destination computation for origin=CENTER.
3. Fix at source (choice building or resolution), NOT by weakening the test:
   the +2 must come from a real swap onto genuinely-waited energy.
4. Then: bp7-020 stacking test (last thin card), `test_inventory.py` regen +
   `--check`, full suite green, commit everything above.
   [DONE 2026-09-22: stacking/empty-deck/own-stage/performance tests added;
   inventory regenerated; suite green; committed.]

## Landmines (do not touch blindly)
- `git stash` list is full of ancient entries — never pop.
- `engine_c_wip/` binaries are build artifacts; leave them.
- `new_id(X)` past 11 instances of one card_no aliases the template — keep
  per-card `new_id` counts ≤11 (deck filler must reuse ONE id).
- ~~`docs/THIN_COVERAGE_ANALYSIS.md` is partly stale~~ — **REFRESHED 2026-09-22**
  (open questions closed, gap #3 filter-aware FIXED, gap #7 expiry largely
  closed + Riko test, Shioriko/HAPPY moved to DONE).
