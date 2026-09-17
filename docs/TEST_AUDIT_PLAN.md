# Test Audit Plan — behavior-first suite

How the suite is organized (`engine/tests/test_modules/`) and what remains
to be audited. The re-sort (2026-09-17) moved ~510 files; this doc tracks
the audit work that the sort enables.

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

## Next work queue (self-assigned, in priority order)

Each item is a full sweep with a green suite + commit between items:

1. **Ren_test identity audit** — its header claims 葉月恋 (koori) but the
   file is named ren and stages PL!SP-bp5-005-R+. Read cards.json: if the
   card is Ren (得生), rename file/functions to the actual character or,
   better, to the shared shapes (mill-cost blades / optional-pay recovery).
2. **Behavior-first rename pass over legacy names** — remaining files named
   for cards or numbers (`mifune_test`, `l0_gap_*`, `qa_new_tests*`,
   `batch5_test`, `sp_bp2_004_edge2_test`) get behavior-led names on touch;
   card IDs stay as data, not as name heads.
3. **Parametrize twin abilities**: for shapes with 3+ per-card files
   (optional-discard looks, constant blade/heart thresholds), extract shared
   helper flows and leave one thin wrapper per card (cost9_group_search
   pattern). Target: delete ≥1 copy-paste family without losing coverage.
4. **Vacuous-test purge**: TEST_QUALITY.md rows (`no_assert`,
   `pendency_only`) fixed on contact while auditing, never as standalone
   campaigns; each fix asserts the real printed behavior end-to-end.
5. **Confusable-number sweep**: the remaining `similar_cards` rows in
   TEST_QUALITY.md — verify staged identity vs cards.json per file.
6. **mod.rs de-branding**: the generated headers say "Auto-generated by
   build.rs — DO NOT EDIT" — add a short pointer to the generator + the
   exclusion list so the next agent does not re-introduce manual edits.
7. **Inventory slimming**: TEST_INVENTORY.json is 1.7 MB and dominates
   `--check` churn; keep the four human docs in --check, move the JSON to a
   separate opt-in check so normal runs stay fast.

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

## Large naming and placement sweep (2026-09-17)

Naming principle (applies to every file/test/helper in the suite): the
**ability behavior is the identity**, not the card. Many cards share an
ability, so names must lead with the behavior (trigger, cost, effect,
condition) and describe the asserted contract. Card-print identifiers
(`hanayo_bp5_008`, Q-numbers) are secondary disambiguators, only there to
tell two copies of the same ability apart or to pin a printed ruling —
never the headline. A file named for a card that tests a shared ability is
misfiled; prefer moving it to the shared shape (or a shared helper + thin
per-card wrappers) over per-card naming.

Baseline after extracting batches 7, 17, 37, and 42: `cargo test --test run_all`
passes all 3308 tests. The next sweep covers all 33 remaining `untested_*`
Rust files, not isolated opportunistic renames.

1. Inventory each assigned source completely: tests, helpers, card prints,
   actual asserted behaviors, and destination effect shapes.
2. Execute independent source-owned partitions:
   - Look/select: batches 15, 20, 25, 26, 33, 51, 52, 53.
   - Recovery and blade/cost effects: batches 4, 8, 36, 43, 45, 49, 50, 54.
   - Compound A: batches 28, 30, 32, 34, 38, 39.
   - Compound B: batches 40, 41, 44, 55 and secondary abilities.
   - Sensors/rules/integration/choice: batches 9, 23, 46, abilities choice,
     change-target choice, and abilities playthrough.
3. Replace batch/history labels with searchable behavior names. Test and
   card-specific helper names retain unambiguous print identifiers. Split
   unrelated shapes; keep genuinely shared flows together. Jidou follows its
   sensor; integration remains integration when it tests a full playthrough.
4. Preserve test setups, assertions, and test counts during mechanical moves.
   Flag substantive coverage issues separately rather than silently weakening
   tests or claiming a naming sweep audited their behavior.
5. Workers own source files and new destinations; the coordinator alone
   integrates shared module declarations and generated reports. Reserve new
   destination names before writing to avoid concurrent collisions.
6. Verify no remaining `untested_*` Rust files or obsolete module references,
   reconcile test counts, run full tests and Clippy, then regenerate inventory
   and run its freshness/soft-guard checks. Do not automatically commit the
   new sweep: the requested checkpoint commit preceded these changes.

## Verification commands

- `cargo test --test run_all` from `engine/` (3292 passed / 0 failed at sort).
- `cargo clippy --test run_all` from `engine/`.
- `python cards/test_inventory.py` then `--check` from repo root;
  `--families` refreshes `docs/ABILITY_FAMILIES.md`.
- `python cards/test_inventory.py --update-softguard-baseline` only when a
  guarded site is genuinely legitimate (count may only go down otherwise).
