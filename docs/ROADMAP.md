# Roadmap

_Consolidated from: REFACTOR_ROADMAP_2026-08-26.md, REFACTOR_BACKLOG.md, REWRITE_PRIORITIES.md, TEST_HARDENING_PLAN_2026-08-26.md_

## Refactor Roadmap  (`REFACTOR_ROADMAP_2026-08-26.md`)

# Refactor & Test Roadmap — 2026-08-26 (deep audit)

Derived from full reads of rules.txt structure, cards/abilities.json marker
scans, engine duplication mapping, and parser-pipeline architecture audit.
Supersedes the stale entries in RULES_GAP_ANALYSIS.md where noted.

## A. Correctness risks (do first — these are bug factories)

### A1. Three divergent stage-heart pipelines ★ highest priority
| Impl | Order applied |
|---|---|
| `zones.rs::get_available_hearts` | override → copy → base → multiplier → additive mods |
| `player.rs::calculate_stage_hearts` | same shape, but override path **duplicated internally** (already drifted once — the 9.9 bug we fixed lived here) |
| `turn/live.rs` member-contribution loop (~1576–1685) | copy → blades → mods → multiplier → override (**different order**) |

Two of the three run during a single live resolution (`live.rs:392/399` call
B; `live.rs:1743/2602` call A). The heart_override-swallowed-additives bug we
fixed existed *because* of this triplication; the ordering divergence is a
live correctness risk for any card combining copy+multiplier+override.

**Rewrite:** single `HeartPipeline::compute(stage, mods…) -> BaseHeart` (+ a
per-member detail view feeding `MemberContribution`). Pin with a
characterization test FIRST: same board through all three entry points must
produce identical multisets (today it may not — that diff IS the test).

### A2. Blade set-rule re-implemented 4×
Canonical `zones.rs::total_blades`; inline copies at `live.rs:1624`,
`condition/card.rs:3044`, `condition/card.rs:2789`. Extract
`effective_blade(card_id) -> u8`. Same characterization-test-first approach.

### A3. `resolve_target_player` mut/non-mut divergence
`abilities.rs:2308` (_mut) vs `:2346` — non-mut falls back through
activating_card/owner_of_card, mut does not; both silently default unknowns
(incl. `"both"`) to player1. 100+ call sites. Make the two arms identical,
return Result or an explicit Owner enum, log-and-default only at the UI edge.

### A4. need-heart satisfaction computed by hand twice inside live.rs
(~218–255 allocation scoring, ~613–700 pass/fail) instead of calling
`card.rs::check_heart_requirement`. Fold into A1's rewrite.

## B. Engine refactors (mechanical, medium value)

- **B1. Split `execute_live_victory_determination` (882 lines)** — contains
  its own heart math + need-heart scorer + trigger ordering; splitting forces A1/A4.
- **B2. Movement-event recording choke point** — 25+ raw-string
  `push_movement_event` callsites (misc.rs, move_cards.rs, phases.rs,
  actions.rs, choice.rs, cost.rs); make it take `(Zone, Zone)` typed args so
  `"energy"|"energy_zone"`-style aliases die at the boundary. Persisted
  strings are pattern-matched later — alias drift = silently dead watchers.
- **B3. Collapse four parallel zone vocabularies** (`ability/enums::Zone`,
  `core/types::ZoneId`, `bot/encoding::ZoneId`, `core/card::Location`) into
  one conversion layer.
- **B4. Typed errors**: 104 `Result<_, String>` in ability/ — callers match on
  English prose ("Cannot baton touch - no member…"). Error enum.
- **B5. Candidate-pool builders ×5** (`game_setup.rs:579/:1353`,
  `choice.rs:402`, `look.rs:356/607`, `state.rs:20`) share filter logic that
  will drift — extract shared pool builder.
- **B6. Dead code decisions**: `ExclusionZone` fully modeled but ZERO cards
  use 除外/バインド (keep — future sets); `repeat_prompt_choice()` doc claims
  to be the single construction point yet is dead — verify & delete;
  consolidate bot strategy v2→v5 layer cake; add ONE no_std target to CI so
  gated stubs can't rot.

## C. Parser/pipeline hardening (each ships with its own guard)

1. **CI hole: `effect_decoder_gen.rs` has NO freshness check** — coverage.yml
   diffs only the condition decoder. A Rust type missing from the effect
   READER_MAP is *silently skipped* at decode. Add the second diff + wire
   `validate_schema.py` into CI or delete it (currently implies safety,
   delivers none, checked nowhere).
2. **Golden snapshot for abilities.json** — today the only shape guards are
   validation-count baselines; a walker reshuffle ships unnoticed until a
   gameplay test trips. Commit a golden (diff ignoring generated_at) or
   hash-pin; regenerate deliberately.
3. **Version the bytecode blob** — abilities.bin.z has no magic/header; a
   stale blob paired with fresh code fails far from the cause. Prepend
   magic+version, assert in vm.rs; make build.rs staleness warnings errors.
4. **Dissolve `_register_action` (767-line inline table)** into declarative
   data rows like CONDITION_PATTERNS — greppable/diffable, no logic change.
5. **Retire zero-hit handlers** (7 confirmed: those_cards_add_hand_optional,
   opponent_after_conditional, sou_shinakatta, heart_choice,
   kore_niyori_cascade, baton_touch_effect, energy_under_member) via the
   repo's own disable→regen→removal-diff methodology; fold ~20 single-hit
   handlers into `_EFFECT_RULES` rows over time.
6. **`_fill_defaults` ownership split** (PARSER_NOTES structural issue #1):
   move extraction into parse_action, leave defaults-only. Reduces the 3–5×
   field-extraction surface that makes walker-guard changes risky.
7. **Do NOT touch**: `_walk`↔`_propagate_context` merge, FIX 10/12/13a/13b
   dissolution — removal-diffed as live with tiny blast radii.

## D. Test-frontier additions (from rules mass × card data)

- §9 is 382 lines (~23% of the rulebook) — check-timing cascade (9.5),
  play-procedure (9.6), auto-ability processing (9.7), replacement effects
  (9.10), LKI (9.11), source identification (9.12) deserve the same
  phase-machine treatment §7 of TEST_HARDENING_PLAN gave the turn machine.
- Replacement effects (9.10.2): multiple replacements on one event =
  affected-party chooses order — only "each-applies-once" exists; write the
  choose-order scenario when ≥2 replacement cards coexist (data check needed).
- Q39/Q34/Q33/Q31/Q29 rulings still unpinned (flagged in TEST_COVERAGE.md).
- Cross-seat mirror coverage per TEST_HARDENING_PLAN §5 ledger — two planned
  rows remain (SP-bp5-027-L conditional gate, S-bp7-025-L option carry-over).
- Timestamp-ordering layer collisions (9.9.1.7): the two singleton set-cards
  (PL!S-bp3-019-L score=4, LL-bp7-001-R＋ cost=10) are the only future
  collision candidates — pin them individually NOW so later stacks are
  detectable.

## E. Facts worth remembering

- Exclusion zone: engine-complete, card-empty (rule 4.13).
- Mulligan: fully interactive implementation, rulebook-only (6.2.1.6).
- 手札に加える = biggest effect family (119 unique abilities / 309 cards).
- Surplus hearts: 17 cards, heavyweight support — asymmetry OK.
- Trigger labels well-centralized (triggers.rs); residual raw substring
  checks at game_state/modifiers.rs:1030/1666 should use TriggerKind.

---

## Refactor Backlog  (`REFACTOR_BACKLOG.md`)

# Refactor Backlog — only what is actually left

_Created 2026-08-22. Supersedes `docs/simplification_plan.md`, `docs/ENGINE_BIG_REFACTOR.md`,
and `docs/OPTIONAL_GATE_CENTRALIZATION.md` (deleted; full text in git history). This file lists
**only items verified as still undone**, each with a necessity verdict against the current tree._

Baseline when written: ~2,503 engine tests green, 936 unique abilities, parser mid-WIP
(`parser.py`, `move_cards.rs`, `abilities_gen.rs` have uncommitted work).

---

## 1. Verified-dead code (safe to remove, low value — do opportunistically)

### 1a. `ActionType::SetCardIdentityAllRegions` — DEAD
- **Proof**: `"set_card_identity_all_regions"` occurs 0 times in `cards/abilities.json`; no test
  constructs the variant; the only entry points are `enums.rs` from_str/to_str/label and the
  dispatch arm at `effects/mod.rs:419`.
- **Keep**: `execute_set_card_identity_all_regions()` (`effects/state.rs:1431`) — it is called
  live from `ability_effects.rs:45` (via `SetCardIdentity` + all-regions flag) and must survive.
- **Verdict**: removal is a 4-file mechanical edit. Not urgent; bundle with the next enum-touching change.

### 1b. `ActionType::ConditionalOptional` — DEAD as input, ALIVE as internal tag — DO NOT naively remove
- 0 occurrences in `abilities.json`, but `compound.rs:955` synthesizes `target:
  "conditional_optional"` as an **internal routing tag** re-entering through `ActionType::from_str`.
- **Verdict**: removing requires migrating the internal tag to a typed enum first. Defer until
  someone touches `compound.rs` anyway.

### 1c. `ActionType::ChoiceCondition` (the *action*, not the condition) — likely dead, verify first
- The 4 `choice_condition` hits in `abilities.json` are the **EffectFilter field**
  (`effect_decoder_gen.rs:247`), not actions. `Condition::Choice` is heavily used and stays.
- **Verdict**: confirm the action variant has no emitter, then treat like 1a.

### 1d. Parser duplicate dispatch rules — real, but blocked
- `引いてもよい` standalone rule shadows behind the broader `引く/引き/引い` rule
  (`parser.py:2203-2213` vs `2189-2195`); `ハート.*得る` registered twice (`parser.py:2464`,
  `2562`).
- **Blocked**: `parser.py` currently has uncommitted WIP from another session. Removing rules
  changes parse output → requires regenerating `abilities.json` + bytecode + full suite run.
  Do after the WIP lands.

---

## 2. Deliberately KEPT (do not "clean up")

### 2a. `vm.rs::populate_from_json` deep-compare oracle (old Phase 3)
- Called "dead decoder duplication" by the old plan — wrong framing: it is the JSON-vs-bytecode
  equivalence oracle used by `bytecode_deep_compare_test.rs`. It is the safety net that makes
  bytecode regeneration trustworthy.
- **Verdict**: KEEP permanently while bytecode abilities exist.

### 2b. `ModifyRequiredHeartsGlobal`
- Old plan claimed the parser never emits it. False: **3 live abilities** use it
  (verified in `abilities.json`). Variant stays.

### 2c. God-function decomposition (old Big-Refactor Phase 1)
- Still true that `execute_gain_resource` (~1,225 lines, `effects/misc.rs:739`),
  `handle_select_card` (~662, `choice.rs:409`), `recalculate_constants` (~606,
  `game_state/modifiers.rs:221`) are huge — but `SelectionContext` already landed, and
  decomposition is pure readability churn with regression risk across ~2,500 tests.
- **Verdict**: decompose opportunistically, one function per PR, only when a behavior change
  already requires touching that function. Never as a dedicated sweep.

### 2d. Action-unification ideas (draw_card→move_cards+flag, unified until_count, etc.)
- Cross-cutting parser+engine+testschema churn; every item invalidates baked bytecode and the
  coverage matrix for zero behavioral gain. The `EffectFilter::target` magic-string
  (`"position|destination"` compared in 5 sites) is the only piece with real bug potential —
  fix that one string into a typed enum if it ever bites; ignore the rest.

---

## 3. Doc corrections recorded here because their source docs were deleted

### 3a. Optional-gate centralization never existed as described
- The deleted `OPTIONAL_GATE_CENTRALIZATION.md` claimed a central gate +
  `offer_optional_skip` + `is_optional_self_gating_action` allowlist. **None of these symbols
  ever existed in `engine/src`** (verified via git log -S). What is real:
  `handle_optional_cost_payment` (`cost.rs:988`) + `ChoiceRoute::OptionalCost`; optional-cost
  prompting remains distributed (`effects/state.rs:79`, `draw.rs:96`, `misc.rs:29`).
- If optional gating ever feels inconsistent, centralizing it is NEW work, not a done deed.

### 3b. Platform runner unification — DONE, doc deleted
- Executed in commit `cf261ee3`: shared runner lives at `engine/src/game/match_runner.rs`
  (`run_embedded_game` / `run_match`); all ports including later snes/genesis/cdi/wasm call it.
  No duplicated front-end loops remain.

### 3c. Known-issues list refreshed
- `engine/ISSUES_FOUND.md` (2026-06-16) was rewritten 2026-08-22: 7 of 9 entries verified fixed
  (Default impls at `card.rs:355` / `game_modifiers.rs:120`, `.or_default()`, unused
  imports/vars/fns). Only "commands return exit code 1" remains unverified.

---

## Rewrite Priorities  (`REWRITE_PRIORITIES.md`)

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

---

## Test Hardening Plan  (`TEST_HARDENING_PLAN_2026-08-26.md`)

# Test Hardening & Parser/Engine Fix Plan 窶・2026-08-26

## Mission (standing directive)

**Engine and parser rewrites while adding more tests for possible in-game
scenarios that have not yet been considered.** Keep examining the existing test
corpus to find what still needs writing. Tests where **both players use
abilities against each other** are required 窶・one-sided scenarios are not
enough for a two-player simulator.

Method, in order:
1. Read `engine/rules/rules.txt` and `cards/qa_data.json` for the EXACT rules
   process before writing any scenario 窶・no guessed semantics.
2. Mine abilities.json for interaction shapes (whose-effect gates, cross-player
   triggers, opponent choices) and check them against existing tests.
3. Write real-game flows, not synthetic state injection.
4. When a test fails: run it under `$env:RUST_LOG="debug"` and READ THE FULL
   FLOW before touching code or expectations; classify test bug vs engine bug
   vs parser gap, then fix end-to-end (AGENTS.md).

## Status (updated as work lands)

| Item | State |
|---|---|
| P0.1 orphaned mod.rs registration | DONE 窶・6 files recovered, `sumire_bp5_test_debug.rs` deleted as stale duplicate; konata/location/shizuku fixed to current APIs |
| P0.2 DIVE! false-trigger audit | DONE 窶・6 real-game tests in `dive_false_trigger_test.rs`; found + fixed **engine gap**: `movement` key on non-Movement condition variants was dropped by both serde and bytecode decode paths, so static presence re-triggered "when placed" abilities. Fixed via `ConditionCommon.movement` + regenerated decoder |
| P0.3 `has_pending_choice` soft-guard migration | DONE 窶・12 -> 4 sites: b9_more probe-if deleted; parser_issues_p3 nested redundant if removed; umi_bp3 + edelnote advance loops rewritten as `while !pending`; performance_snapshot_audit (3 sites) and performance_phase_rules migrated to dispatch-on-prompt-type with panic on ANY unexpected prompt; helpers::scan_autos_both now answers via variant-dispatched `answer_choice`. Remaining 4 baselined as genuinely legitimate (generic helper, optional DASH arrange, structural on_choice loop) |
| P1.1 per-test inventory | OPEN |
| P1.2 or-assertions sweep | DONE 窶・36 multi-line `||` hits triaged: 10 files tightened to exact outcomes (pl_s_bp7_007 ﾃ・ 竊・exact stage slots; kuroe_dia 竊・selected card on deck top; l0_gap_livesuccess vacuous disjunct removed; batch34 decline now pins waitroom retention; natsumi Q264 adds not-discarded check; opponent_choice wait竕leave both-stay; mia/card_ability/izumi/look_and_select deterministic picks pinned). 22 hits judged legitimate (match guards, loop conditions, error-string chains, engine zone aliasing) |
| P1.3 helper dedup (`fire_trigger` x40) | DONE 窶・21 byte-identical clones deleted onto `helpers::fire_trigger`; batch30 keeps a 3-line drain wrapper; batch8's `fire_trigger_nth` kept as genuine extension |
| P2.* parser/pipeline hardening | OPEN |
| P3.* engine quality debt | MOSTLY DONE -- either-target fixed, mojibake fixed, non-skippable silent no-op fixed (empty answers now Err); remaining: queue-u8 indexing, runaway counters per-game semantics, web-server lock logging |

## §5 Round 5 — seat-relative / cross-player abilities (L1-depth mining)

Mined TEST_INVENTORY for `相手`-shaped abilities still at L1 depth (no
negative/mirror coverage). New file `cross_player_round5_test.rs`. Ledger of
what existed vs what was missing:

| Card | Existing coverage | Missing until now → added test |
|---|---|---|
| PL!S-bp6-022-L (ライブ成功時: 相手のエネルギーが自分より多い→スコア+1) | batch12, both tests fired from P1 only (`fire_trigger` hardcodes pid=p1) | P2-owned copy fired AS p2 (+/- mirror); mirror boards each side pinned independently; waited energy counts per rules 4.7.4 (`cards.len()` not `active_count()`) |
| PL!SP-pb2-029-N (登場/開始時: 相手のコスト2以下をウェイト) | batch11, single-sided debut only | Mirror standoff — both seats debut their own メイ via real play_to_stage with seat flip; cost gate protects opposing メイ (9>2); already-waited eligible target re-selected no-op; cost-4 negative; 9.6.3.1.3 no-prompt pin |
| PL!N-bp7-009-R (登場: 自分と相手それぞれ7枚ミル) | bp7_q267 file pins refresh boundaries + both-refresh THOROUGHLY (do NOT duplicate) | ADDED: P2-fired copy (seat-relative "自分と相手"); exact top-card identity across both mills — no cross-pollination. Note: engine resolves both-target move_cards OPPONENT-first then self (`execute_move_cards_both`); fill_decks seeds 30/deck not 20 |
| PL!HS-PR-035-PR (登場: 相手の控え室メンバー3枚→相手デッキ底+ブレード3以下ウェイト) | bp7_ginko file (L1, placement only) | ADDED accept/decline/P2-seat tests. Found + fixed **engine bug #6**: the follow-up rest resolved against the prior step's `selected_cards` (the 3 discard→deck-bottom members, never on stage) instead of scanning the opponent's STAGE — the printed clause 「相手のステージにいる元々…ブレード3つ以下のメンバー1人をウェイトにする」 could never fire. Fixed END-TO-END: (a) state.rs member_op falls back to the stage scan when NO selected card is on the target stage (diagnostic line kept); (b) **parser fix** — the select-followup emitter no longer stamps `source="selected_cards"` onto clauses that scope their own objects with an existence qualifier (`〜にいる／〜にある`); PR-035-PR now parses `source="stage"`. Backreference verbs (「そのライブカードを手札に加える」, しずく bp5-003) keep the stamp — first regen attempt over-matched `手札に` and broke shizuku_bp5, caught by suite, narrowed to existence qualifiers. Golden diff vs HEAD = exactly one semantic line. Grounding: printed text scopes the rest to stage members; Q118/Q154 gate the rest on the move; Q102 + 9.6.3.1.3 keep the zero-candidates no-op; DB scan confirmed this was the ONLY change_state+source=selected_cards ability |
| PL!SP-bp5-027-L (成功時: 自エネルギー置き→そうした場合相手は1枚引く) | batch32 (accept/skip, own-side) | planned: conditional gate = opponent draw ONLY on accept; empty energy deck edge; combo with 4.7.4 (waited energy it places feeds bp6-022-L comparison) |
| PL!S-bp7-025-L (成功時 choice: 相手コスト4以下2人までウェイト+次ターン非アクティブ制限 / draw 1) | batch9 (L1+choice) | planned: option A cost gate + restriction carry-over; option B draw |

Rules grounding used: 4.7.4 ('エネルギー'=energy-zone cards, orientation-
independent), 8.4.4→8.4.5→8.4.6 (LiveSuccess resolves before score compare,
both players fire same phase), 5.2.1 (ウェイトにする has no state precondition),
9.6.3.1/.1.3 (exact-count must choose when possible; zero eligible ⇒ ignored,
no prompt), Q267 (mill deck-out refresh).

## §6 Round 6 — finder blind spots (integration-level)

Gaps the per-ability inventory structurally cannot see; new file
`integration_blindspot_test.rs`:

1. **Real-rollover expiry** — live_end temporary effects were only ever
   expired by CALLING `check_expired_effects()` directly with a hand-set turn
   phase (modifier_layer_characterization_test). Nothing proved the actual
   victory→Active rollover (`turn/phases.rs`) invokes it. New test grants via a
   real ability INSIDE the live, passes through performances/victory, asserts
   the grant reverts exactly at the rollover.
2. **Dual-trigger second windows** — 13 abilities carry 「登場/ライブ開始時」;
   tests always fire ONE window manually. PL!N-bp5-004-R 果林 had debut-only
   coverage, PL!HS-bp6-004-R 吟子 live-start-only. New tests drive the missing
   windows through REAL flow (play_to_stage / phase advance), including gate
   edges: original-blade==EXACTLY-4 (果林) and cost<=9 (吟子).
3. Flow lesson pinned by these tests: after a mid-turn main-phase action, the
   fixed 5-pass walk lands on the WRONG seat's LiveCardSet — drive phases
   explicitly; SelectAutoAbility prompts must be answered with SELECTIONS
   (empty answer declines the resolution under test).

RESOLVED observation: 果林's dual-trigger change_state raises a
「Pay optional cost: Put this member to wait state?」 gate during resolution.
NOT a bug: her printed text is 「このメンバーをウェイトにしてもよい：相手の…」
— an optional self-cost that the parser correctly emits as
cost={optional:true, self_cost:true} and the engine correctly gates on.
Initial "suspect gate" diagnosis came from reading only `effect`, never
`cost`; the new permanent [PAY_SKIP_GATE] diagnostic (resolver.rs) logs
route+description for every gate emission and would have answered this
immediately.

Ruling gaps surfaced for future rounds: Q29 (arrival-turn baton-touch ban —
mechanism `deployed_this_turn` exists and is cleared at rollover; covered
indirectly via kasumi/sumire_bp4/qa_new_tests198 but no direct Q29 pin),
Q31 (duplicate card numbers in live zone — implicitly covered by
performance_snapshot_audit test 2 using two copies of one card number),
Q39/Q34/Q33 (yell-before-heart-check ordering + live-card cleanup timing).

## §7 Round 7 — non-ability game systems (phase machine)

New `phase_machine_rules_test.rs` — systems with no ability text that the
ability inventory structurally cannot see. Empirical facts now pinned:

| Rule | Pinned behavior |
|---|---|
| 7.4.1 | Waited member + waited energy stand ONLY for the turn player across the Active→Energy boundary; the other player's persist |
| 7.5.2 | Energy phase moves the TOP energy-deck card into the zone (executes on the Energy→Draw transition); EMPTY deck = silent skip, no panic |
| 7.6.2+Q267 | Draw phase draws exactly 1 even from an empty main deck — waitroom refresh feeds it silently (deck ends at waitroom−1) |

CRITICAL identity model for future tests: `is_first_attacker` NEVER flips
(fixed RPS result); the turn player comes from `current_turn_phase`
(FirstAttackerNormal / SecondAttackerNormal). Several earlier test drafts
wrongly assumed the flag tracks the turn.

Also pinned implicitly: `deck_refreshed_this_turn` is set ONLY by explicit
effect-driven refreshes (mill/look overdraw), NOT by the silent phase-draw
refresh — an internal-marker distinction, documented on the test.

## ﾂｧ2 Zone-change / source-gate audit (round 2)

Systematic scan of every trigger-gate key in abilities.json (zone_change
source/destination, movement on non-Movement variants, self_effect_only,
check_self, temporal phase_target) cross-referenced against the test corpus.
Result: 16 gate shapes, all covered except three families 窶・now tested in
`engine/tests/test_modules/zone_change_gate_test.rs`:

| Card | Gate | Tests |
|---|---|---|
| PL!S-bp6-002-R+ 譯懷・譴ｨ蟄・ab#0 | live_card_zone竊壇iscard, Aqours live, target=self | positive deck-top/bottom placement; ﾎｼ's-live negative; own-side-only negative |
| PL!N-pb1-009-R 螟ｩ邇句ｯｺ迺・･・ab#0 | live_card_zone竊壇iscard + negated has_blade_heart, this_turn | positive draw+hearts; blade-heart-departure negative |
| PL!N-bp5-005-R+ 螳ｮ荳区・ ab#0 | stage竊壇iscard via REAL baton touch (rules 9.6.2.3.2/.1) | cost-15 newcomer full payoff (+2 energy AND draw); cost-13 boundary (energy only); blade-heart newcomer negative |

Baton-touch semantics pinned from rules.txt + qa_data.json: the 縲後ヰ繝医Φ繧ｿ繝・メ縺励◆縲・event belongs to the ARRIVING member's play (9.6.2.3.2.1); the departed ability's
conditions describe the NEWCOMER; net payment = newcomer cost 竏・occupant cost.
Energy assertions require wait-state cards in the pool (activation flips wait竊誕ctive).

## ﾂｧ4 Round 4 窶・opponent-live-success across seats

The only card reading `opponent_live_success` (Strawberry Trapper PL!S-pb1-021-L)
was "covered" exclusively by synthetic flag injection; no test verified the
real pipeline ever sets it, and the legacy flag was armed ONLY for P2's success
(`player2_won`) 窶・a P2-owned card could never see P1 succeed.

Fixes:
1. Per-seat tracking: `p1/p2_live_success_this_turn` + `_no_excess` on
   GameState, written at victory determination; reset with the other turn flags.
2. Owner-relative evaluation: `evaluate_opponent_live_success_condition` picks
   the seat opposite the ACTIVATING card's owner ([OPP_LIVE_SUCCESS_EVAL]
   diagnostic).
3. **Engine bug #5 (ordering)**: LiveSuccess triggers fired at the TOP of
   `execute_live_victory_determination`, before verdicts/results existed 窶・   every real-flow opponent-success evaluation saw stale state. Fix:
   `record_pretrigger_live_results()` records per-seat outcomes before the
   trigger block using the SAME score formula as the post-trigger totals
   (extras = 0 by construction). The later authoritative pass remains as
   post-extras truth. Skipped seats don't clobber explicitly-armed values.
   Follow-up refactor item RESOLVED: mirror upgraded to exact score formula
   rather than deleted (deleting would require reordering the trigger/score
   dependency chain 窶・triggers feed extras feed final scores).

New `opponent_live_success_flow_test.rs`: full organic round 窶・P1 exact-fill
success (豬ｷ譛ｪ + START:DASH!!, heart-less deck so yell adds nothing) vs P2-owned
Trapper 竊・+2 pinned through the real pipeline.

Suite state after round 4: **2945 passed / 0 failed / 0 ignored.**

## ﾂｧ3 Round 3 窶・both-player interaction (mission directive)

Mined abilities.json for the 縲悟ｯｾ謌ｦ逶ｸ謇九・繧ｫ繝ｼ繝峨・蜉ｹ譫懊〒繧ら匱蜍輔☆繧九・marker
class (6 cards) and for effects that move OPPONENT members (exactly one:
PL!HS-pb1-014-R 螳蛾､雁ｯｺ蟋ｫ闃ｽ debut, position_change dest=front target=opponent).

New `cross_player_interaction_test.rs`: 蟋ｫ闃ｽ's MiraKura-gated debut force-
repositions P2's 蜿ｯ蜿ｯ 竊・蜿ｯ蜿ｯ's own area-move watcher must fire from the
opponent-caused move (heart06 until live end); plus a gate-blocked negative.

**Engine bug #4 found and fixed:**
`fire_opponent_cause_watchers_for_move` enqueues the watcher with a
trigger_moved_cards SNAPSHOT, but resolution-time evaluation of
`position_change` conditions only consulted live scratch state
(`position_change_events`, `recently_moved_cards`) 窶・which the enqueuing
player's batch loop legitimately clears (abilities.rs:1433-1445) before the
other seat's queue resolves. The watcher armed, then silently did nothing.
Fix: the position_change arm of evaluate_movement_condition now falls back to
the entry snapshot via entry_trigger_moved_cards(), with a [POS_CHANGE_EVAL]
diagnostic line. Also noted: answering a non-skippable SelectTarget with empty
indices silently no-ops ("Unknown source position") 窶・queued as P0.3/P3 work.

Suite state after round 3: **2944 passed / 0 failed / 0 ignored.**

**Engine bugs found and fixed by this round:**

1. **Indices-channel answers to `position|destination` choices dropped cards**
   (actions.rs BCR dispatch): only the `card_id` channel was mapped through
   the options array; an answer arriving via `card_indices` (the normal
   `select_indices`/web-UI channel) fell through and the raw index became the
   destination string `"0"`, matching no zone 窶・the chosen card silently
   vanished. Fixed by mapping `card_indices.first()` through `options` first.
   Found by `riko_responds_only_to_own_side_live_zone`.

2. *(Prior round)* `movement` on non-Movement condition variants was dropped by
   both decode paths 窶・see commit history.

Note: `blade_heart` is its OWN printed field (`cards.json .blade_heart`,
b_heart icons) distinct from the `blade` stat 窶・picked test members accordingly.

Suite state after round 2: **2939 passed / 0 failed / 0 ignored.**

Suite state after this round: **2934 passed / 0 failed / 0 ignored** (was 2927 passing
with 7 failing orphans + 2 ignores before recovery). The two former `#[ignore]`s were
full-width-plus card-ID typos, not engine gaps.

---

Working plan derived from the full-repo audit (engine src, cards/ parser ecosystem,
abilities.json, engine/tests). Guiding directive: **work on tests and fixes together** 窶・every parser gap gets an end-to-end test, every engine fix pins behavior, no documenting
around gaps (AGENTS.md rule).

---

## P0 窶・Silent test death / false confidence

### P0.1 Register the 7 orphaned test files (`test_modules/mod.rs` drift)
Files present on disk but never compiled into `run_all`:
- `untested_abilities_batch22_test.rs` (entire coverage batch)
- `l0_gap_constant4_test.rs` (also holds 2 `#[ignore]`d tests)
- `heart_color_test.rs`
- `konata_bp1_test.rs`
- `location_condition_cost_test.rs`
- `shizuku_bp4_aggregate_test.rs`
- `sumire_bp5_test_debug.rs`

Register all, compile, fix fallout. If a file cannot compile against current helpers,
fix the file (it may encode stale APIs worth resurrecting). Delete only true duplicates
(`sumire_bp5_test_debug.rs` vs `sumire_bp5_test.rs`: compare assertions before deleting).
Then add a CI guard: every `*_test.rs` under `test_modules/` must appear in `mod.rs`
(extend `cards/test_inventory.py --check`).

### P0.2 False-positive trigger audits (the "DIVE! class" of bugs)
Pattern: auto abilities whose triggers are **compound** (temporal gate AND a
zone_change/count gate). Any effect elsewhere in the DB that touches the same zone
transition is a candidate for falsely arming them. See ﾂｧ1 for the worked example.

Workstream:
1. Enumerate all auto abilities whose trigger includes `zone_change` +
   `source/destination` gates (grep `abilities.json`).
2. For each, identify sibling effects in the DB performing the same transition
   (mass retrieval, per-card retrieval, reveal/look-without-move, deck竊檀and,
   opponent-side moves).
3. Write negative tests: sibling card moved 竕 self moved 竊・NO trigger.
4. Write positive controls: self actually moved via another card's effect 竊・trigger.
5. Where a negative fails, classify: test bug vs engine bug vs parser gap; fix end-to-end.

First concrete instance done here: **DIVE! (PL!N-bp4-026-L)** 窶・see ﾂｧ1 below.

### P0.3 Kill the `if game.has_pending_choice()` soft-guard pattern
WRITING_TESTS.md explicitly forbids it; it appears ~250ﾃ・across ~105 files.
A missing prompt means the ability fires free / skips cost and the assert still passes.
Plan: migrate file-by-file to `drain_choices_strict` / dispatch-on-choice loops
(already exist in helpers/mod.rs), add a grep-based CI lint banning the guard outside
explicitly-negative tests.

## P1 窶・Coverage honesty

### P1.1 Make `test_inventory.py` per-test, not per-file
Current: substring presence of a card_no anywhere in a file = "covered"; L1 inferred
from `"assert" in text` anywhere in the covering file. Headline "771/771 covered" is
file-granular, not ability-granular. Change to parse each `#[test] fn` body and map
card 竊・asserting test; exclude unregistered (orphaned) files from stats until P0.1 lands.

### P1.2 Or-assertions and vacuous asserts
Replace `assert!(a || b)` outcome-tolerance tests (e.g. `pl_s_bp7_007_test.rs`) with
exact-outcome or decision-point assertions. Sweep `assert!(result.is_ok())`-style
tests in early `qa_new_tests*.rs`.

### P1.3 Deduplicate test helpers
~40 private copies of `fire_trigger` (with behavioral drift between copies),
~50 `fill_decks` variants with three signatures. Consolidate onto `helpers::`,
delete local clones mechanically, one batch family per commit.

## P2 窶・Parser/pipeline hardening (each item ships with a regression test)

### P2.1 Single source of truth for opcode/tag tables
`COND_TO_VARIANT_TAG` / `ACTION_TO_VARIANT_TAG` / `COST_TYPE_TO_ACTION`
(`cards/compile_abilities.py`) are hand-synced copies of facts derivable from
`engine/src/core/card.rs`. Derive them from card.rs the way
`generate_*_decoder.py` already does. Regression test: generated table diff ==
handwritten table before switchover; then CI freshness gate like the condition decoder has.

### P2.2 Decoder generator parity
`generate_condition_decoder.py` drops all `serde(alias)` attrs (effect generator keeps
them); unknown field types emit silent `skip_value()` arms. Add generation-time warnings
+ a parity test asserting both generators agree on shared READER_MAP entries.

### P2.3 Compiler input mutation
`compile_abilities.py` renames keys on the live parsed JSON mid-walk (non-idempotent).
Encode from deep-copied data; add a round-trip test: compile twice 竊・identical bytes.

### P2.4 Scraper loudness
Fetch failure currently breaks pagination and writes truncated output with exit 0;
type-extraction miss silently defaults to member. Make failures non-zero exit +
explicit "INCOMPLETE" marker; add a smoke fixture test with a frozen HTML sample.

### P2.5 abilities.json hygiene
`input_hash` always null, `parser_version` never bumps, dead `use_limit`/`cost` null
keys, `type` mixing three namespaces, `zone` duplicating `source`. Fill hash/version at
generation time; drop provably-dead keys in one coordinated regeneration (bytecode
regenerate + golden diff + full suite).

## P3 窶・Engine quality debt (fix alongside touching nearby code)

- u8 indexing in `ability_queue.rs` slot bookkeeping 竊・widen or saturate.
- Process-lifetime runaway counters (`PCA_CALLS`, `CHOICE_CALLS`, queue `clear()`)
  竊・per-game reset semantics.
- Web-server lock handling: replace silent `if let Ok(lock())` skips with logged
  recovery via existing `lock_recover`.
- Mojibake in runtime strings/comments (`game_state/abilities.rs:1515,756`;
  AI-artifact comment `player.rs:491`); pin `PYTHONUTF8=1` in Python scripts.
- Consolidate 11ﾃ・duplicated `MemberArea竊段ndex` matches onto `to_index()`.
- Pair-call invariant `evaluate_success_zone_constant_abilities()` +
  `restore_performance_need_heart_modifiers()` 竊・single method.

## ﾂｧ1 Worked example: DIVE! false-positive trigger surface

Card: `PL!N-bp4-026-L` ("DIVE!"), two auto abilities:

- **ab#0** 窶・trigger = AND(temporal: *own* main phase, `zone_change discard竊檀and`,
  self-target). Effect: optional move 1 "DIVE!" hand竊値ive zone; then
  `reduce_live_card_set_limit` +1.
- **ab#1** 窶・condition = location(self in live zone, `movement: moved`). Effect:
  1 Nijigasaki member gains blade+2 until live end.

Existing tests (`dive_auto_trigger_test.rs`, `dive_edge_test.rs`, `dive_live_card_test.rs`)
always move DIVE! itself and always set `recently_moved_cards(vec![dive])` by hand.
They never probe cross-card interference. Candidate false-positive classes being tested
in `dive_false_trigger_test.rs`:

| # | Scenario | Expected |
|---|----------|----------|
| F1 | Another live card moved discard竊檀and | ab#0 must NOT fire |
| F2 | DIVE! moved main_deck竊檀and | ab#0 must NOT fire (source gate) |
| F3 | DIVE! revealed/looked at in discard, not moved | ab#0 must NOT fire |
| F4 | Opponent's DIVE! moved during our phase scan | P1's copy unaffected; P2's fires only in P2's own main phase |
| F5 | Mass retrieval sweeping DIVE! discard竊檀and | ab#0 SHOULD fire (positive control) |
| F6 | ab#1 with DIVE! statically in live zone, nothing moved this turn | NO blade grant |
| F7 | ab#1 re-scan next turn after movement flag cleared | NO re-grant |
| F8 | Two DIVE!s, only one moved | exactly one placement choice / limit bump |

Any failure here is classified test-bug/engine-bug/parser-gap and fixed end-to-end.

---
