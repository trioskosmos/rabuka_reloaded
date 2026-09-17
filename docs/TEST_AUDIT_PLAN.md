# Test Audit Plan — behavior-first suite

How the suite is organized (`engine/tests/test_modules/`) and what remains
to be audited. The re-sort (2026-09-17) moved ~510 files; this doc tracks
the audit work that the sort enables.

## Core principle: the ability is the identity

**The ability a card has is what is important and must be described —
everywhere: file names, test names, helper names, comments, and audit
notes. Card names and card-print IDs are secondary and often not
important at all.** Many different cards carry the exact same ability;
what the test verifies is the ability's behavior (its trigger, cost,
condition, effect, and edge branches), not the card it happens to ride
on. Consequences:

- Name files/tests after the behavior ("optional discard two, split-group
  recovery"), never after a card ("ginko_test", "batch5").
- Use card-print IDs only as disambiguators when two copies of the same
  ability need telling apart, or to pin a printed ruling (Q number) to
  its card — never as the headline of a name.
- When a card has several abilities, name each test after the ability it
  exercises; the card ID is data inside the test, not the identity.
- A wrong card number staged inside a test IS still a bug (it must drive
  the ability it claims), but fix it by correcting the number or asserting
  identity — never by renaming the file to the card.

## Layout (what goes where)

- `rules/` — game-wide mechanics, proven once, not per card:
  `trigger_paths`, `phases`, `zones`, `targeting`, `scoring`,
  `conditions`, `baton`.
- `effects/` — non-jidou abilities by **effect shape** (trigger ignored):
  `look_select/{reveal_to_hand,reorder_top,pin_top,split_top_bottom,bottom_inspect,niche_filters}`,
  `recover/to_hand`, `deck_order/restore`, `deploy`, `mill`,
  `draw/{flat,until_count,chains}`, `gain/{blades,hearts}`, `score`,
  `state`, `position`, `cost_mod`, `choice`, `conditional`,
  `ability_mod`, `restriction`, `energy/{place,under_member}`, `compound`.
- `jidou/` — auto abilities by **trigger sensor** (effect usually trivial):
  `yell`, `movement`, `leaves_stage`, `energy_watch`, `state_watch`,
  `debut_watch`, `discard_watch`, `ability_watch`, `title_once`.
- `characterization/` — parser/corpus pins, known-reduced behavior.
  `integration/` (playthroughs), `support/` (helpers) unchanged.
- Official QA rulings live WITH the behavior they rule on (Q number stays
  in the filename). There is no `qa/` folder.

Sorting rules that were applied (keep applying them):

1. Group-name slots (μ's/Aqours/Liella!/…) are trivial filters — one
   parametrized flow per family, not one file per group.
2. Split folders only where a slot changes engine behavior: cost-prefix
   interaction (decline → skip vs. remainder still routed), niche filters
   (`or_card_types`, heart-count thresholds, ability-presence, `negation`,
   `per_group`, `exclude_self`), durations.
3. Jidou sorts by sensor, not effect. Every sensor family needs
   fire **and** no-fire pairs, plus the causation scope
   (self-caused vs opponent-caused, the （対戦相手のカードの効果でも発動する。) rider).
4. Whole files move; only split a file when its sections test different
   shapes AND you are actively auditing that family.

## How to audit a family

Work in value order — shared engine paths first, card variants last:

1. **Shared cost/skip/indexing semantics** (protects hundreds of cards):
   bundled optional-cost skip (rest-self + discard skips the whole effect —
   pinned for the cost-9 family), filtered-relative indices
   (positions WITHIN filtered_indices, cf. Umi/bp4 joint), remainder
   routing on decline vs. skip, `as_long_as` on/off transitions,
   `use_limit` enforcement, sequential step interactions.
2. **Vacuous tests, on contact.** Tests that cannot fail (drain loops with
   weak asserts, offer-only tests) claim false coverage. Fix when met
   while auditing; never as a standalone campaign. `TEST_QUALITY.md`
   rows are pointers, not a todo list.
3. **Per-shape branch matrices**, riskiest shapes first (niche filters,
   jidou sensor no-fire cases). Procedure per family:
   - Open the family folder. List its cards via `TEST_INVENTORY.md`.
   - For each card, check the branch checklist for that shape:
   - look `reveal_to_hand`: eligible/take, eligible/decline, no-eligible
     auto-skip, cost paid/declined, remainder routing asserted by identity.
   - look `reorder_top`: partial/full/skip selection, deck-refresh edges.
   - jidou sensors: positive fire, negative (wrong content / wrong cause /
     wrong phase), use-limit (`［ターン1回］`), scope rider.
   - optional costs (`てもよい`): both branches, and whether decline skips
     the whole effect or only the pick (differs per shape — pin it).
3. Add missing branch tests **in place** (shared helper + thin per-card
   wrappers, cf. `effects/look_select/reveal_to_hand/cost9_group_search.rs`).
4. Regen + verify: `cargo test --test run_all` (engine/),
   `python cards/test_inventory.py` + `--check` (root).

## Known gaps (found during the sort — fix first)

- [x] Cost-9 search family branch matrix (all five siblings now pin
      take / decline-pick / no-eligible / decline-cost). Killed one
      vacuous test (`eli_bp5_skip_both_costs_still_looks` never looked).
- [x] QA rulings dissolved into behavior families (Q number in filename).
- [x] Toubatsu wrong-card staging (bp2-011 vs pb2-011 are DIFFERENT cards:
      debut-only vs auto+live-start). Auto tests now stage `pb2-011-R`;
      all 3 choice options pinned. See AGENTS.md failure points.
- [ ] Confusable-number sweep (TOP PRIORITY): walk every `similar_cards`
      row in `TEST_QUALITY.md` (~31). For each file, verify the staged
      card's identity against `cards/cards.json` and confirm the test
      drives the card its header claims. Fix the number or assert identity.
      NOTE: a pool-flakiness theory for bp2/pb2 was investigated and
      DISPROVEN — the confusion was static wrong numbers in test source,
      not nondeterministic lookup. Do not chase pool nondeterminism
      without a 3-run probe first (AGENTS.md).

- [ ] Cost-9 search family: DONE (see above) — remaining: 蓮ノ空
      sibling lives in `izumi_bp5_test.rs`; unify into the family helper
      when touched.
- [ ] `TEST_QUALITY.md` review prompts: `no_assert` (kaho pin_top tests
      assert inside a helper — fine, but make it explicit),
      `pendency_only` (22), `similar_cards` (31 confusable bp2/pb2 stagings).
- [x] `look_select/recruit_stage/` stood up: Proof bp6-029 remainder
      routing extracted from the integration parser file; Kanata bp7-018
      negation search moved out of `compound/` (and renamed — "recruit"
      was a misnomer).
- [ ] Thin shapes with no dedicated folder yet (votes exist, no top):
      `live_watch` sensors (pb2-006 dual-sensor covered from `movement/`,
      folder on second card), opponent-deck target-player flows.
- [ ] Whole-file impurities (kept deliberately; split when auditing):
      [x] `effects/score/chika_test.rs` → split into
      `score/chika_bp3_001_wait_self_activation_live_total_score_test.rs` (8
      tests), `cost_mod/chika_bp5_001_no_ability_member_cost_reduction_test.rs`
      (4), `rules/baton/chika_bp5_001_no_ability_baton_touch_draw_test.rs` (4).
      Names lead with behavior; print IDs only disambiguate twin abilities.
      [ ] `effects/choice/ll_bp7_001_triple_member_test.rs`,
      `effects/recover/to_hand/ren_test.rs` (blades half — note its header
      says 葉月恋 but stages a Ren card; identity-check before trusting),
      `effects/recover/to_hand/kotori_bp5_003_test.rs`,
      `effects/ability_mod/s_pb1_019_live_test.rs`,
      `effects/state/cards_6_thru_13_test.rs` (multi-card sweep),
      `jidou/movement/bp7_auto_gap_test.rs` (multi-sensor sweep),
      `effects/gain/blades/b7_constant_ability_test.rs` (multi-constant),
      `effects/cost_mod/himeno_test.rs` (look + cost + restriction),
      `effects/score/link_to_future_test.rs`,
      `effects/gain/hearts/constant_edge_case_test.rs`.

## Work tracking

The active queue at the end of this document is the source of current
assignments. Record each completed ability contract, evidence, test result,
and remaining branch gap there. Do not equate static inventory references
with execution coverage or mark a whole sweep complete after a subset.
Verify each integrated wave with `cargo test --test run_all` and inventory
checks. Commit only when explicitly requested by the user.

Inventory slimming is complete: default `--check` excludes
`TEST_INVENTORY.json`; `--check --check-json` also verifies JSON.

## Sweep-found issues ledger (inspect + fix; 2026-09-17)

Found while re-homing 66 batch files. Each entry: the flagged claim, and
how to verify before touching anything. **Verification gate — read first:**
before editing any flagged test, (a) read the card's actual entry in
`cards/cards.json` (name, cost, ALL abilities), (b) re-read the test body.
A flagged "mismatch" may actually be: a multi-ability card whose file tests
a different ability than the header's (fine if the file is named for the
tested one), a deliberate characterization pin, or a stale comment with
correct code. Confirm the test is ACTUALLY wrong before fixing; fix
end-to-end (assertion + name + placement), classify test bug vs engine bug
vs parser gap, and state which in the commit message. Grep the card print
across the whole test tree, not just the flagged file — the same wrong
assumption often repeats.

1. **Trigger mislabels** (name/header says X, card fires Y):
   - HS-bp6-013: old name said debut; test fires LiveStart. Verify against
     cards.json which trigger is printed, rename to match reality
     (current: `low_blade_opponent_wait_pl_hs_bp6_013_r_test.rs`).
   - SP-PR-018: fires LiveSuccess and asserts energy count only (old name
     said wait-state placement). Current:
     `seven_liella_reveals_pl_sp_pr_018_pr_test.rs` — verify the count-only
     assertion against the printed rider (placed energy, not waited state).
2. **Effect mislabels:**
   - HS-bp6-030: draw+discard, not mill (verified cards.json:81987; renamed
     `draw_discard_pl_hs_bp6_030_l_test.rs`). DONE.
   - PL!-pb1-007-R activation cost is discard-three, not mill (renamed
     `lilywhite_gated_live_pl_pb1_007_r_test.rs`; verify the cost clause in
     cards.json and that both tests pay the full three-card cost).
   - Batch43's "score six" staged a score-9 live; renamed to score9 —
     verify the assertion exercises the >=-threshold boundary, and add the
     boundary-equal case if missing.
3. **Character mislabels** (name ≠ staged card):
   - PL!HS-bp2-008-R is 徒町小鈴 (Kosuzu), NOT 北条そふぃ — old batch9
     test names said Sofiya. Renamed to behavior names; sweep the suite for
     other wrong-character names by checking every staged print's name in
     cards.json against its test name.
   - PL!S-bp6-001-R is 高海千歌, not "Shion" — same treatment.
   - Batch46's header referenced PL!HS-bp6-009-R without any test staging
     it — check whether bp6-009 has coverage elsewhere
     (TEST_INVENTORY.md ground truth); if not, it is a coverage gap.
   - RESOLVED — ren_test.rs identity: cards.json confirms
     PL!SP-bp5-005-R＋ IS 葉月 恋 (given name Ren). The file name "ren" was
     a legitimate short form, NOT a mismatch — the earlier ledger claim
     was wrong (multi-ability card, both abilities tested correctly).
     Remaining work there is only behavior-led renaming per the naming
     principle; the tests themselves assert correctly (Q221 scope, Q233
     decline branch both present and correct).
4. **Tests that never fire the claimed ability (vacuous):**
   - PL!-bp6-016-N: RESOLVED (commit b8b77cc5). The printed LiveSuccess
      look-3/reorder was parsed with a count-1 move (only one card returned,
      no prompt) — engine bug, fixed end-to-end: parser marks the unqualified
      それらを…デッキの上に置く return `all:true`, move_cards treats
      looked_at as order-eligible when all, and the order choice now
      accumulates a full permutation. New file
      `nozomi_bp6_016_live_success_look_three_reorder_test.rs` asserts all
      six top orders + unchanged-order with exact deck suffix preservation;
      old debut-only test renamed to
      `live_success_reorder_does_not_trigger_on_debut` (negative).
    - WWD delayed lock: RESOLVED — verified `PL!SP-bp7-027-L`
       (What a Wonderful Dream!!) in cards.json:93798–93825. Added
       `live_success_placed_energy_skips_next_active_phase_wwd_pl_sp_bp7_027_l`
       in `live_success_waited_pl_sp_bp7_027_l_test.rs`, preserving the
       existing immediate-placement test. The new test resolves the real
       LiveSuccess ability, pins the placed energy's identity, advances via
       Pass through turn rollover and Active→Energy, and asserts it remains
       waited while the one-phase lock expires. Commit classification:
       test coverage gap (not engine bug/parser gap); no engine changes.
       Verification: WWD 5/5, full run_all 3311/3311, Clippy and cargo check
       successful (existing warnings), scoped rustfmt check clean.
   - Hanamaru identity assertion contained `|| true` — RESOLVED
      (commit fc740bc0): `double_heart04_member_pl_s_bp5_007_r_test.rs`
      now asserts the fetched hand card IS the Dia heart04×2 print.
   - Karin second trigger: LEDGER PREMISE DISPROVEN — cards.json probe
      shows NO 朝香果林 card carries two ライブスタート時 triggers (the
      two-jidou card in `jidou_combo_edge_test.rs` is 葉月恋
      PL!SP-bp7-005-R＋, whose `|| true` was fixed in fc740bc0). The
      actual named-look gap was fixed instead (commit ef39f042):
      `pl_n_pb1_016_r` now asserts revealed-to-hand identity AND waitroom
      remainder, not merely "left the deck".
5. **Weak/missing branches (add tests; never weaken existing):**
   - Bounded drains that never assert termination: add a final
     `assert!(!game.has_pending_choice())` on touch.
   - Negatives testing unpayable-cost auto-skip instead of voluntary
     decline (bp2-005-R, pb2-007-R): add the true decline branch.
   - Threshold tests asserting only above-threshold: add exact-boundary
     and below cases per printed inclusivity.
   - Deficit score return checks active count, not energy-deck origin —
     pin the deck the returned energy comes from.
   - Transform tests inspect the modifier map only; assert calculated
     hearts (and expiry where printed).
6. **Batch55 header listed six prints with no tests** — look each up in
   TEST_INVENTORY.md; treat uncovered ones as coverage gaps to fill.
7. **Multi-ability caution for ALL fixes above**: several flagged cards
   have 2-3 abilities; a test may legitimately target a different ability
   than the file header claimed. That is a header/name bug, not a test
   bug — rename, do not rewrite the test body. Only rewrite behavior when
   the assertion contradicts the PRINTED text of the exercised ability.

## Placement confidence

~130 files were placed by plurality vote at <50% confidence (filler-card
and multi-ability noise). They are in the right neighborhood but should
be re-verified when their family is audited — the per-file vote breakdown
is in the sort working notes, and `TEST_INVENTORY.md` shows per-ability
covering files as ground truth.

## Large naming and placement sweep (2026-09-17) — ongoing

The `untested_*` and `qa_new_tests*` extraction waves are complete, but
legacy naming cleanup is NOT complete. The targeted 12 `l0_gap_*` files,
`batch_nico_bp4_hanayo_test.rs`, `upper_batch_on_yell_test.rs`, and
`bp5_333_erena_edge_test2.rs` have now been replaced by 29 behavior-led
files, preserving their 48 tests. Names were chosen after reading the test
bodies and exercised printed abilities; unrelated effect shapes were split
and misplaced constant gains moved out of energy-placement/reveal folders.
Test names describe the assertions actually present, not stronger coverage
implied by former card-led names. This is naming/placement cleanup only:
weak assertions, member cards staged in live zones, and missing negative
assertions remain follow-up work, not verified ability contracts.
Inventory was regenerated and `--check` passed. The pre-edit suite had four
failures in `mari_bp2_test`; post-edit verification is currently blocked by
concurrent engine changes causing E0502 in `core/game_state/modifiers.rs`.
No engine or parser code was changed by this naming wave.

The ability is the identity; omit card names and IDs from names unless
they distinguish a meaningful rule variant. A filename change alone is
not evidence that an ability has been verified.

Post-sweep state (2026-09-17, ~15:30): suite green 3355/0. Coverage gaps
closed the same day: idx 328/559/743 (batch 1), 747/777/783 (Ginko +
required-hearts; 783 also fixed a real parser gap — distinct-count
location conditions over 「メンバー」 now carry `card_type=member_card`),
811/825/870/927 (formation, mill/blades, reorder, reveal). Coverage of
idx 915 remains unverified; do not mark it closed from an assignment alone.
Regenerated
`abilities.json`/bytecode fixed the Private Wars blade-limit and Ginko
failures (stale artifacts, no code change needed for those two).

## Verification commands

- `cargo test --test run_all` from `engine/` (3355 passed / 0 failed at
  2026-09-17 ~15:30).
- Use `cargo test` for Rust verification; do not run separate build,
  check, or Clippy loops. Capture complete output to a file and read it.
- `python cards/test_inventory.py` then `--check` from repo root;
  `--families` refreshes `docs/ABILITY_FAMILIES.md`.
- `python cards/test_inventory.py --update-softguard-baseline` only when a
  guarded site is genuinely legitimate (count may only go down otherwise).

## Active work queue (after the naming sweep; 2026-09-17)

1. **Confusable-number sweep (TOP PRIORITY)**: 27 `similar_cards` rows in
   `engine/tests/TEST_QUALITY.md`. For each row, verify the staged card's
   identity against `cards/cards.json` and confirm the test drives the
   ability its name/header claims; fix the number or assert the staged
   card's identity. The ability is the identity — a wrong staged print is
   fixed by correcting data, never by renaming to the card.
   Identity inspection complete across the 27 rows: no wrong confusable
   print found. Focused runs: A 103, B 62, C 53, D 26 passed; A/B overlap
   on the 28-test under-member module, so these are not unique-test totals.
   Correct print identity does not establish branch or trigger coverage.
    Verified follow-ups: under-member zero/one-material gameplay and exact
    bonuses (28-test module passed); real opponent-caused movement no-fire
    (6-test module passed); movement blade expiry through victory (3 passed).
    Gained-total-score recipient/source baton controls passed (2 tests),
    and LiveEnd expiry with/without a set live passed (2 tests). The no-live
    expiry case was also rerun with `--exact` (1 passed). Full suite:
    3359 passed / 0 failed (granted_ability_full_suite.log); inventory
    regenerated and `--check` clean from repo root.
    Source-leaving regression repaired end-to-end: the invalid same-turn
    baton setup was replaced with Yoshiko PL!S-bp3-006-R＋'s printed
    activation (wait self + hand discard → remove another Aqours member),
    which legally removes the declined-gain source; score drops 1→0 with
    exact stage/waitroom/orientation assertions
    (`declined_discard_total_score_is_lost_when_yoshiko_activation_discards_source`).
    Granted-ability bookkeeping now tracks the recipient: `gained_abilities`
    (and synthetic registration) bind to the actual waited cost member, and
    the baton controls assert the recipient owns the entry while the source
    does not.
    Soft-guard ratchet resolved without baseline changes: the two
    conditional guards in `arise_gated_activate_blade_or_wait_private_wars_test.rs`
    and the EdelNote payment guard were replaced with strict
    `drain_choices_strict` completions. The EdelNote negative branch
    (`low_cost_edelnote_or_costly_wrong_group_cannot_enable_choice_743`) now
    pays the printed pay-or-skip gate explicitly (printed colon structure
    offers payment before the cost-9 condition gates the branches) and then
    asserts the branches stay locked and energy is spent. Debug trace
    confirmed the prompt origin (`[PAY_SKIP_GATE]` → cost.rs optional-energy
    gate); engine behavior matched the print, so the fix was test-side.
    Choice family 92/92 green; final full suite 3359/0
    (final_full_suite.log); inventory `--check` clean from repo root.
    Still open: member incorrectly set as live in `cards_6_thru_13_test.rs`,
    and revealed-score-icon negative branch. RESOLVED: Special Color
    current-center condition — a real engine defect. The `has_moved`
    temporal condition ignored the parsed `position` filter
    (abilities.json:16298/16309 `position: "center"`): when the center slot
    emptied (member moved OUT of center), the fallback scanned the whole
    stage and scored anyway. Fixed in
    `condition::state::evaluate_temporal_condition` (HasMoved arm): a
    positioned subject now requires the CURRENT occupant of that slot to
    match the group filter AND have moved; an empty positioned slot returns
    false; unpositioned conditions (Dancing stars Q255, bp5-014/017,
    LL-bp5-001) keep the prior whole-stage/activating-card behavior.
    Scan bounds: only three parsed prints carry positioned `has_moved`
    (Special Color center, 桜小路きな子 bp4-017 left_side, 鬼塚夏美 bp4-020
    right_side). Tests: positive now uses a legal out-and-back-into-center
    sequence; new negative
    `special_color_no_score_when_moved_liella_left_center` pins 0 score
    after leaving center. Family 5/5, Q255 family 1/1, full suite 3360/0
    (special_color_final.log, q255_after_position_fix.log,
    position_fix_full_suite.log).
2. **Opponent optional live discard → otherwise gain total score**:
   `s_pb1_002_riko_edge_test.rs` already asserted choice shape and score;
   its `pendency_only` report was not proof of an assertion-free test.
   Added exact discarded-card destination and hand preservation assertions
   (accepted-discard test passed). Replaced the mislabeled decline setup,
   which offered no eligible live, with an actual voluntary decline.
    This exposed a real engine refresh defect: synthetic Constant ability
    registration did not refresh the live-total accumulator after choices.
    Fixed at `execute_gain_ability`, after temporary-effect registration,
    using the shared constant-trigger matcher. Debug trace confirms +1;
    original three branches passed without test-side recalculation.
    Expanded matrix: interleaved member/live filtered choice, true decline,
    no-live, empty hand all pass. Source-leaving regression completed: the
    invalid same-turn baton setup (legal area lock, verified by debug trace)
    was replaced with Yoshiko's printed removal; see follow-ups above.
3. **Whole-file impurities** (split on touch, keep bodies): 
   `ll_bp7_001_triple_member_test.rs`, `ren_test.rs` blades half,
   `kotori_bp5_003_test.rs`, `s_pb1_019_live_test.rs`,
   `cards_6_thru_13_test.rs`, `bp7_auto_gap_test.rs`,
   `b7_constant_ability_test.rs`, `himeno_test.rs`,
   `link_to_future_test.rs`, `constant_edge_case_test.rs`.
4. **Parametrize twin abilities**: 3+ per-card files for one shape →
   shared helper + thin wrappers (cost9_group_search pattern).
5. **Weak branches from the ledger** (§5 above): drain termination,
   true voluntary-decline branches, exact-boundary thresholds,
   energy-deck origin, calculated hearts.
6. **`live_watch` sensor folder**: needs a second card to stand up.
7. **mod.rs de-branding**: point at generator + exclusion list.
8. **Placement-confidence re-verification** (~130 files) while auditing
   each family — inventory shows per-ability coverage as ground truth.
9. **New sweeps to keep coming**: after the above, (a) verify every
   `characterization/` pin still matches printed text on touch, (b) add
   no-fire jidou sensor pairs where a sensor family lacks negatives,
   (c) rerun `test_inventory.py --families` and audit any family whose
   per-ability file count diverges wildly from siblings, (d) after every
   parser change regenerate artifacts FIRST (stale artifacts caused two
   false failures today) and rerun the failing filter before diagnosing.
