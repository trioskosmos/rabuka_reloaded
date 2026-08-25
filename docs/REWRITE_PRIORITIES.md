# Rewrite Priorities — Ranked by Size / Effort

Derived from full-suite analysis (parser ecosystem, engine ability core, test suite).
Ground truth = `engine/tests` (~2,946 tests). Constraint honored throughout: **no file
splits** — all improvements stay within existing file layout.

> **Blocker:** HEAD does not compile (half-landed `Box<AbilityEffect>` migration:
> card.rs:2237, choice.rs:3016/3018, cost.rs:1054, live.rs:777; 5 stale WIP stashes
> likely hold related work). Nothing below can be validated until this lands.

---

## P0 — `describe.rs`: merge EN/JA twin tables into one data-driven table

**Effort: Large · Risk: Low · Payoff: removes the largest mechanical duplication in the core**

- `describe_effect_en` (:96–549, ~453 lines) and `describe_effect_ja` (:771–1200,
  ~429 lines) are parallel giant matches over identical action strings.
- Parallel helper families duplicated too: `zone_label`(:26)/`zone_label_ja`(:669),
  `card_type_label`(:49)/`card_type_label_ja`(:691), `state_verb`(:60)/`state_verb_ja`(:702),
  `resource_label`(:69)/`resource_label_ja`(:711), `duration_label`(:78)/`duration_label_ja`(:720).
- Rewrite as one table `action → (en_fmt, ja_fmt)`; ~880 lines → ~450.
- Drift class already has a guarding test (`choice_prompt_templates_all_have_japanese`,
  :1202) — the table makes that structural instead of enforced-by-test.
- Validation: golden-diff current output via `bin/describe_dump.rs` before/after.

## P1 — Python parser registry hardening (`cards/ability_extraction/parser.py`, ~12.6k lines)

**Effort: Large · Risk: Medium (corpus-wide) · Payoff: kills the #1 silent-corruption class**

- 84 ordered `_register_action(...)` substring rules where **registration order =
  semantics** (move_cards must beat change_state only by ordering accident,
  e.g. parser.py:2095–2123). Introduce per-rule dependency declaration or explicit
  priority so reordering can't silently change corpus output.
- Deduplicate cost-field extraction: `_extract_basic_cost_fields` (:911–971) vs
  `parse_cost` fallback tail (:1487–1535) extract characters/groups/count/card_type/
  target 3–5× (lru_cache blunts cost, not clarity debt).
- Unify continuation-line detection in `extract_card_abilities.py:243–260`
  (three ad-hoc code paths for similar clause shapes).
- Loud failures for subprocess steps in `extract_card_abilities.py:583–605`
  (nonzero exit from `compile_abilities.py` currently ignored; decoder-regen failure
  only WARNs → stale bytecode can pass locally).
- Litter removal: `_d.py` (reads `%TEMP%`), print-only `test_parsing()` run on every
  extraction (:457–500), dead duplicate `lines = []` (generate_condition_decoder.py:197).

## P2 — Derive variant-tag mirror tables from card.rs (`cards/compile_abilities.py`)

**Effort: Medium-Large · Risk: Low · Payoff: eliminates a whole drift class**

- `COND_TO_VARIANT_TAG` / `ACTION_TO_VARIANT_TAG` (:122–231) are hand-maintained
  mirrors of the serde enums in `engine/src/core/card.rs`. Unknown keys silently fall
  back to generic TAG_OBJECT encoding (safe but larger/slower, unreported).
- Reuse the brace-counting Rust-decl parsing already proven in
  `generate_condition_decoder.py` / `generate_effect_decoder.py` to emit these maps.
- Bonus while there: shared `rust_decl_parser.py` used by all three generators
  (they currently carry two subtly different parsers; alias handling exists in only one),
  plus generator-side assertion that every parsed field type resolves in READER_MAP
  (unknown types currently emit silent skip arms — generate_effect_decoder.py:264+).

## P3 — `choice.rs` + `look.rs`: deduplicate selection/prompt machinery

**Effort: Medium · Risk: Low-Medium · Payoff: shrinks the worst god-function's surface**

- `handle_select_card` is ~678 lines (choice.rs:402–1080).
- "Select {} more card(s)…" EN+JA reprompt hand-rolled **12×** (:627, :679, :722,
  :1147, :1273, :1358, :1459, :1542, :1847, :2137, :2254, :2305×2 inside
  `handle_discard_selection`). One private builder next to existing `build_reprompt` (:1080).
- `look.rs`: verbatim 65-line `or_card_types` block copy-pasted between
  `execute_select` (:362–424) and `execute_select_cards` (:660–722); route both
  zone listings through `util::zone_cards` (util.rs:1812) instead of local reimplementations.
- Trivial adjacent win: `resolver.rs::can_activate_effect` duplicates its position-merge
  block verbatim at :297–305 and :340–346.

## P4 — `modifiers.rs`: unify GainResource candidate-selection

**Effort: Medium · Risk: High (needs care) · Payoff: highest bug-risk duplication removed**

- Two independent implementations of "which stage members get how much":
  - `recalculate_constants` blade path (:232–870 overall; GainResource arm :391+)
  - `apply_success_zone_effect` GainResource path (:1528–1639)
- Subtly different filter sets (position/group/all_any/under-card on one side;
  stage iteration + group filters on the other) — divergence here produces wrong BP
  that tests may not cover yet.
- Extract shared candidate-selection + amount-resolution helper; keep both entry points.
- Adjacent: `misc.rs` `apply_heart_resource` (:1287–1448) vs `apply_blade_resource`
  (:1603–1739) share the empty-targets → position-based → fallback-to-self skeleton;
  parameterize by resource kind (~150 lines saved).

## P5 — Deck/draw loop unification (`move_cards.rs`, `effects/draw.rs`)

**Effort: Medium · Risk: Medium · Payoff: one loop shape instead of four drifting ones**

- `resolve_from_deck` (:1160–1212, incl. Q104 refresh loop + type/group re-push),
  `resolve_from_deck_bottom` (:1214–1239), `resolve_from_energy_deck` (:1241–1272),
  and `draw.rs` distinct-filter draw (:458–481) are four shapes of the same loop.
- Parameterize on draw direction + optional gate text; fold distinct-draw in last.

## P6 — Test infrastructure: registration gate + strict-drain backfill

**Effort: Medium spread over time · Risk: None · Payoff: stops silent test loss forever**

- **mod.rs drift is real**: 8 files on disk never declared in `test_modules/mod.rs`
  (~21 tests never compiled/run): `check_self_condition_test`, `heart_color_test`,
  `konata_bp1_test`, `location_condition_cost_test`, `shizuku_bp4_aggregate_test`,
  `sumire_bp5_test_debug` (debug scaffolding?), `untested_abilities_batch22_test`,
  `l0_gap_constant4_test` (holds both `#[ignore]`d tests). Fix or delete explicitly.
- Extend `cards/test_inventory.py --check` to assert every `test_modules/*.rs` is
  declared — converts drift detection into an automated CI gate.
- Delete empty placeholder stubs: `abundant_test.rs`, `qa_remaining_tests2.rs`,
  `unique_abilities_test.rs`.
- Backfill adoption of the (excellent, underused) helper API:
  - ~862 hand-rolled drain guard-loops in 246 files vs `drain_choices_strict`
    (helpers/mod.rs:1041, used in 20 files) — migrate opportunistically per touched file.
  - `fire_trigger` (27 files) vs ~198 local debut/drain setup re-implementations.
  - `board_snapshot`/`assert_board_matches` (helpers/mod.rs:1084): **used 0 times**
    — built for exactly the move/mill/refresh tests where collateral zone damage slips through.
- Tighten weak assertions when touched: inequality bounds with derivable exact values
  (`b7_constant_ability_test.rs:71–75`, energy-leak-proofing), and
  `action_coverage_test.rs:96–112` counting arbitrary `Err`s as success via error-string
  whitelisting.

## P7 — God-function opportunistic decomposition (per existing backlog policy)

**Effort: Ongoing, one function per PR · Risk: varies · Payoff: maintainability**

Per `docs/REFACTOR_BACKLOG.md` policy (decompose opportunistically, no big-bang):
`handle_select_card` (678 L), `recalculate_constants` (638 L),
`execute_gain_resource` (517 L), `execute_position_change` (514 L),
`evaluate_appearance_stage` (453 L), `trigger_auto_abilities_for_player_with_event`
(401 L), `process_current_ability` (388 L), `evaluate_comparison_condition` (376 L),
`execute_move_cards` (352 L), `get_count_for_condition` (323 L).
P3/P4/P5 above pre-collapse the duplication inside several of these.

## P8 — Small hygiene sweep

**Effort: Hours · Risk: Trivial**

- Dead code: `vm.rs:109 decode_fallback_count` (never called);
  `resolver.rs:292/326/329` set-then-discarded `activation_condition_passed`;
  backlog-verified dead enum variants (`SetCardIdentityAllRegions`) when an
  enum-touching PR happens anyway.
- `parser_utils.py:973–977`: `EffectPattern.setter` swallows exceptions while the
  parallel `ActionRule.apply` logs loudly (:913–917) — make it match.
- Zone membership checks duplicated across modules (`condition.rs:895`,
  `resolver.rs:414`, `condition/card.rs:3288`) → route through one helper.
- 47 hand-built `Choice::SelectTarget{..}` structs across `src/ability` → builder
  pattern (36 `description_ja:` sites alone).

---

## Explicitly NOT recommended

- Splitting any file (user constraint; also conflicts with the no-big-bang backlog policy).
- Big-bang test DSL migration — the 862 drain loops should convert opportunistically,
  each verified against green suite, never in bulk.
- Rewriting the generated decoder files directly — edit the generators only
  (AGENTS.md rule).
