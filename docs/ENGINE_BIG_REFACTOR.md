# Rabuka Engine — Big Refactor Plan

Status: **PLANNED** — not yet started. This document scopes the full decomposition of the engine's god-functions and its triple-encoded rule pipeline. It is written to be executed in stages, each verified by the full test suite (`cargo test`, currently 2256 passing) before moving on.

---

## 1. Why this refactor exists

The user's core complaint: **fixing a card rule requires far too much thinking.** Every card-rule change touches many giant files, and the same game semantics are encoded three separate times that must be kept in sync.

Three systemic problems (confirmed by code mapping):

1. **Triple-encoded rules.** Every ability is parsed by the Python parser (`cards/ability_extraction/parser.py`, 12.8k lines), compiled to bytecode (`cards/compile_abilities.py`), decoded at runtime by regenerated Rust decoders (`engine/src/ability/effect_decoder_gen.rs`, `condition_decoder_gen.rs`, plus a duplicated oracle `ability/vm.rs:1372+`), and executed by Rust behavior files (`move_cards.rs`, `effects/misc.rs`, `choice.rs`, `cost.rs`, `condition/card.rs`). A single rule change often must be made in all three layers and kept in sync.

2. **`EffectFilter` god-struct.** `engine/src/core/card.rs:750-918` is a ~170-field flat struct shared by every `EffectKind`. Its `target` / `destination` / `source` are **strings** re-parsed differently by each consumer. `self_target` means source-filtering in one place and destination semantics in another (the exact trap that caused the `under_self` / `self_target` bug). This is the biggest lever for reducing "thinking per fix."

3. **True god-functions** (by body span), not merely large files:
   - `effects/misc.rs::execute_gain_resource` — ~1290 lines
   - `core/game_state/modifiers.rs::recalculate_constants` — ~722 lines
   - `ability/choice.rs::handle_select_card` — ~681 lines
   - `ability/move_cards.rs::resolve_from_zone` — ~637 lines
   - `ability/condition/card.rs::evaluate_appearance_condition` — ~547 lines
   - `cards/ability_extraction/parser.py::parse_action` — ~520 lines

---

## 2. Pipeline overview (end-to-end)

- **Layer A — Text parsing (Python).** `cards/ability_extraction/extract_card_abilities.py` reads `cards.json`, calls parser entry points (`parse_cost`, `parse_effect`, `parse_condition`). `parser.py::parse_action` matches Japanese phrases against a ~775-line `_ACTION_RULES` registry. Output: `cards/abilities.json`.
- **Layer B — Compile to bytecode (Python).** `cards/compile_abilities.py` encodes each ability into tagged binary-JSON, emits `engine/src/ability/abilities_gen.rs` (bytecode + `OFFSET_DELTAS` + `STRINGS` + `CARD_ABILITY_PAIRS`) and `cards/build/abilities.bin`. `cards/generate_effect_decoder.py` / `generate_condition_decoder.py` generate the Rust decoders.
- **Layer C — Runtime decode (Rust).** `ability/vm.rs::get_ability` → slice bytecode → `decode_ability` → `decode_effect_field` / `decode_condition_field`. `ability_store.rs::AbilityRef::resolve()` lazily decodes. `vm.rs:1372+` keeps a **second** decoder path (`populate_from_json`) used only as a deep-compare oracle.
- **Layer D — Runtime resolution (Rust).** `game_state/abilities.rs` triggers/enqueues auto-abilities → `resolver.rs::resolve_ability` (checks / use-limit / cost) → `effects/mod.rs::execute_effect` (big `match action_type`) → `move_cards.rs`, `effects/misc.rs`, `effects/draw.rs`, `effects/state.rs`, `effects/score.rs`, `compound.rs`, `look.rs`. Pending choices are answered via `choice.rs::provide_choice_result`.

---

## 3. Scope decision

The full refactor is split into three phases. The recommendation is to execute **Phase 1 fully, then Phase 2 for `self_target`/`under_self`/`destination`** (the exact ambiguity that caused the recent bug), and defer Phase 3 until the first two are stable. The user has approved all three; sequencing is at the implementer's discretion but each phase must leave the suite green.

**Hard requirement: behavior-preserving.** Any behavior *discrepancy* surfaced by the refactor (two call sites reading the same field with different intent) is a latent bug — it should be **flagged for review** and only fixed explicitly, not silently "corrected" during a mechanical extraction.

---

## Phase 1 — Decompose the god-functions (mechanical, behavior-preserving)

Extract each into focused methods, adding a dedicated unit test per extracted piece where practical. Verify the full suite after each extraction.

1. **`ability/move_cards.rs::resolve_from_zone`** (882–1519, ~637 lines)
   Extract one fn per destination arm: `resolve_discard_destination`, `resolve_deck_destination`, `resolve_under_member_destination`, `resolve_stage_destination`, `resolve_hand_destination`. Each arm already has its own block boundaries.

2. **`ability/effects/misc.rs::execute_gain_resource`** (599–1890, ~1290 lines)
   Extract the discrete sub-mechanisms: `gain_from_selected_card_hearts` (613–672), `gain_surplus_heart`, `gain_heart_colors_threshold`, `gain_per_unit`. Each begins at a distinct `if effect.<flag>_any()` block.

3. **`ability/choice.rs::handle_select_card`** (376–1057, ~681 lines)
   Collapse the 23-arg signature into a builder; pass the `SelectionContext` to each zone handler (hand/reveal/discard/energy/looked-at/stage) instead of 20 positional args.

4. **`ability/resolver.rs::resolve_ability`** (679–1036, ~357 lines)
   Extract `check_and_record_use_limit`, `check_position_keywords`, `pay_ability_cost`, `persist_pending_choice`. Note the use-limit logic is duplicated at 802–818, 962–966, and 989–1023 — extract once.

5. **`ability/condition/card.rs::get_count_for_condition`** (3898–4268) and **`evaluate_appearance_condition`** (2805–3352)
   Split per condition subtype; each subtype already calls a small helper.

6. **`core/game_state/modifiers.rs::recalculate_constants`** (64–786, ~722 lines)
   Extract `recompute_score_sources`, `recompute_yell_sources`, `recompute_constant_abilities`. (`recalculate_constant_cost_modifiers_with_ids` at 823–953 already exists as a partial model.)

7. **`ability/describe.rs::describe_effect_en/ja`** (96–509, 690+)
   Two near-parallel ~400-line fns. Extract per-action description templates and make language a parameter.

8. **`ability/condition.rs::evaluate_condition`** (413–595)
   Already dispatches to card.rs/state.rs/compound.rs — make it a thin table lookup mapping variant → evaluator, removing the inline `match`.

---

## Phase 2 — Kill the `EffectFilter` ambiguity (higher risk; the real fix)

Replace string-typed, overloaded fields with typed enums.

1. **`destination` / `source` → typed `Zone` enum.**
   Eliminate string re-parsing (`== Some("deck")`, `Zone::from_str(destination)`). A single typed field read everywhere.

2. **`target` → split into `target_player` and `target_kind`.**
   `target` is currently reused as: a player ("self"/"opponent"/"both"), a destination zone ("deck"/"discard"), and a choice-type marker ("conditional_optional"/"position|destination"/"area_select"). The resolver string-compares `target == "position|destination"` (resolver.rs:952,959). Split into distinct typed fields.

3. **Fold `self_target` + `under_self` into a single `PlacementTarget` enum.**
   This is the exact trap from the recent session. One enum: `UnderThisMember`, `UnderChosenMember`, `FilterSelfAsSource`, etc., replacing two booleans that overlap.

4. **Group the ~170 flat fields into per-effect-kind sub-structs.**
   `MoveEffect`, `GainResourceEffect`, `ChangeStateEffect`, `DrawEffect`, … so each behavior file reads only its relevant fields instead of the shared 170-field god-struct. `EffectKind` variants (card.rs:925–970) each wrap `Option<Box<EffectFilter>>`; the 14-arm `filter()`/`filter_mut()` match (973–1011) collapses into per-variant typed accessors.

> During this phase, the typed enums will expose places where two call sites were already disagreeing on a field's meaning. Those are latent bugs — **flag for review**, do not silently "fix."

---

## Phase 3 — Collapse triple-encoding (highest value, highest risk)

1. **Make the Rust bytecode decoder the single source of truth.**
   Delete the duplicated `ability/vm.rs::populate_from_json` oracle (~150 lines, 1372–1531). Keep one decoder; update the deep-compare harness to compare against the single path.

2. **Move the parser's phrase→action rules into a data file.**
   Export the `_ACTION_RULES` table (`parser.py:1813–2588`) to a JSON rule file consumed by both the Python parser and a Rust-side validator, so phrase rules are not only in Python and can be validated/cross-checked from Rust.

---

## 4. Verification

- **After every extraction/change:** run the full suite `cargo test` in `engine/`. Current baseline: **2256 passing, 0 failing** (commit `4823fa4b`).
- **After Phase 2 type changes:** regenerate parser output + bytecode, and confirm the bytecode↔JSON round-trip matches (the deep-compare oracle must stay green until Phase 3 removes it).
- **After Phase 3:** confirm the removed oracle is not referenced anywhere and the single decoder path produces identical output.

## 5. Git hygiene

- One commit per extraction/phase, with a clear message referencing the file+function refactored.
- Behavior-preserving commits must show no test-count change.
- Any latent-bug fix that changes a test expectation must be a separate, explicitly-labeled commit ("fixes a latent bug surfaced by refactor: <field> <description>").

---

## 6. Reference: god-function map

| File | Function | Span | Lines |
|---|---|---|---|
| `ability/effects/misc.rs` | `execute_gain_resource` | 599–1890 | ~1290 |
| `core/game_state/modifiers.rs` | `recalculate_constants` | 64–786 | ~722 |
| `ability/choice.rs` | `handle_select_card` | 376–1057 | ~681 |
| `ability/move_cards.rs` | `resolve_from_zone` | 882–1519 | ~637 |
| `ability/condition/card.rs` | `evaluate_appearance_condition` | 2805–3352 | ~547 |
| `cards/ability_extraction/parser.py` | `parse_action` | 2631–3151 | ~520 |
| `ability/resolver.rs` | `resolve_ability` | 679–1036 | ~357 |
| `ability/cost.rs` | `pay_cost_move_cards` | 168–508 | ~340 |

All paths are under `C:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\` and `C:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\` unless otherwise noted.
