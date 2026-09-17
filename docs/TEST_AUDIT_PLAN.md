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
      `effects/score/chika_test.rs` (activation score + cost reduction),
      `effects/choice/ll_bp7_001_triple_member_test.rs`,
      `effects/recover/to_hand/ren_test.rs` (blades half),
      `effects/recover/to_hand/kotori_bp5_003_test.rs`,
      `effects/ability_mod/s_pb1_019_live_test.rs`,
      `effects/state/cards_6_thru_13_test.rs` (multi-card sweep),
      `jidou/movement/bp7_auto_gap_test.rs` (multi-sensor sweep),
      `effects/gain/blades/b7_constant_ability_test.rs` (multi-constant),
      `effects/cost_mod/himeno_test.rs` (look + cost + restriction),
      `effects/score/link_to_future_test.rs`,
      `effects/gain/hearts/constant_edge_case_test.rs`.

## Placement confidence

~130 files were placed by plurality vote at <50% confidence (filler-card
and multi-ability noise). They are in the right neighborhood but should
be re-verified when their family is audited — the per-file vote breakdown
is in the sort working notes, and `TEST_INVENTORY.md` shows per-ability
covering files as ground truth.

## Large naming and placement sweep (2026-09-17)

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
