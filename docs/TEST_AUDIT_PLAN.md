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

- [ ] Cost-9 search family: DONE (see above) — remaining: 蓮ノ空
      sibling lives in `izumi_bp5_test.rs`; unify into the family helper
      when touched.
- [ ] `TEST_QUALITY.md` review prompts: `no_assert` (kaho pin_top tests
      assert inside a helper — fine, but make it explicit),
      `pendency_only` (22), `similar_cards` (31 confusable bp2/pb2 stagings).
- [ ] Thin shapes with no dedicated folder yet (votes exist, no top):
      `live_watch` sensors, opponent-deck target-player flows,
      recruit-to-stage selects. Folder them when the second card arrives.
- [ ] `effects/compound/inspect_top_recruit_no_blade_member.rs` is a
      recruit-shape look misfiled by vote — move to `look_select/`
      (new `recruit_stage/` folder) when touched.
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

## Verification commands

- `cargo test --test run_all` from `engine/` (3292 passed / 0 failed at sort).
- `cargo clippy --test run_all` from `engine/`.
- `python cards/test_inventory.py` then `--check` from repo root;
  `--families` refreshes `docs/ABILITY_FAMILIES.md`.
- `python cards/test_inventory.py --update-softguard-baseline` only when a
  guarded site is genuinely legitimate (count may only go down otherwise).
