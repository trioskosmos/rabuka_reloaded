# Pain Points – Parser & Engine Refactoring Needed

## 1. Parser `parser.py` – Text Context Propagation (`_walk` family)

**Problem:** `_walk_propagate_text_context_fields` propagates `group_names` from parent text to every child node based on substring matching. For heterogeneous sequential abilities like Maki `「A。その後、B場合 C」` the parent contains `『μ's』` from `A`, but `C` (draw) does not contain it. The generic propagation leaks `group_names: ["μ's"]` into `C`, and also into top-level sequential container. This required ad-hoc `_strip_leaked_draw_g` and special-casing `draw_card`/`sequential`.

**Refactor:** Replace string-contains propagation with AST-aware scoping. Each sequential clause should be parsed in isolation; `group_names` should be extracted per-clause, not inherited via `ctx_text`. The `_walk` pipeline runs 7+ passes (`_walk_propagate_*`, `_collapse`, `_enrich`, etc.) that interact via shared mutable dicts – hard to reason about order. Consolidate into a single context stack passed explicitly.

**Files:** `cards/ability_extraction/parser.py:9717` `_walk_propagate_text_context_fields`, `10255` `_normalize_effect_tree`

## 2. Parser – Condition Extraction for Sequential (`parse_ability`)

**Problem:** `parse_ability` does a fallback “fill missing condition” by scanning `remaining_text` for first `場合、` occurrence. For `「A。その後、B場合 C」` it captures `「A。その後、B場合」` as condition text, producing a bogus `state_condition` with `target: both` that includes the prior clause. Then it promotes sub-action conditions to top-level, gating the whole sequential (so unconditional `A` would be blocked by `B`'s condition).

**Fix applied:** Split on `SEQUENTIAL_MARKER` (`その後、`) and only consider last segment; avoid promoting sub-conditions to parent when parent is `sequential`.

**Refactor:** The trigger/condition split should be done per sequential segment during `_try_sequential`, not as a post-hoc fallback in `parse_ability`. Remove the generic fallback entirely; rely on handler-level condition attachment.

**Files:** `parser.py:968` `parse_ability`, `8200` `_try_sequential`

## 3. Parser – State Condition Enrichment (`_try_state`)

**Problem:** `_try_state` only emitted `{state, text, group_names}`. Contextual fields `target` (相手/自分), `location` (ステージ), `card_type` (メンバー), `count/operator` (いる → >=1) were missing, so `state_condition` for Maki was `location: None`. Engine then fell back to checking activating card, failing. Enrichment was attempted via `_walk` string matching but was fragile (relied on `"メンバー" in text` heuristic).

**Fix applied:** Added explicit `extract_target`/`extract_location`/`extract_card_type` + existence count inside `_try_state`.

**Refactor:** All condition handlers should use a shared `_extract_generic_fields` path, not per-handler ad-hoc heuristics. The current split between handler-path (`_enrich_condition_common` only) vs fallthrough (`_extract_generic_fields`) is inconsistent.

**Files:** `parser.py:4275` `_try_state`, `parser.py:1592` `parse_condition` handler dispatch

## 4. Parser – Post-processing Order & Regeneration

**Problem:** `extract_card_abilities.py` calls `parse_effect` → `_normalize_effect_tree` → `process_abilities` → `_walk` again, with multiple fix passes (`_fix_sequential_chain`, `_propagate_context`, etc.) that can re-introduce leaks after earlier cleaning. Regeneration requires `python ability_extraction/extract_card_abilities.py` from `cards/` and then `cargo test` – not CI-enforced, so `cards/abilities.json` and `engine/baked/*.json` (`engine/src/ability/abilities_gen.rs`) easily go stale. The manifest `cards/build/generation_manifest.json` is manual.

**Refactor:** Make `cargo build` invoke the Python step (build.rs) or at least `cargo test` fail if `abilities.json` is out-of-date (hash check). Collapse `process_abilities` fixes into declarative rules.

**Files:** `cards/ability_extraction/extract_card_abilities.py:538`, `cards/compile_abilities.py`, `engine/build.rs`

## 5. Engine – Sequential Resume After Choice (`choice.rs:149` `finalize_choice`)

**Problem:** `execute_sequential_effect` saves remaining actions via `save_remaining` when a sub-action creates `pending_choice` (e.g. `move_cards` with `max: true` → `SelectCard`). `finalize_choice` only called `resume_pending_actions` when `!sub_choice`. For `SelectCard`, `sub_choice` is false but `pending_choice` was set by the handler, and the original code cleared `pending_choice` before resuming, losing the continuation. This caused second step (`draw_card`) to never execute after the first step's selection – observed as `DEBUG_RA i=0` only.

**Fix applied:** In `finalize_choice`, detect `has_pending && was_select_card` and clear `pending_choice` then resume.

**Refactor:** The distinction between `pending_choice`, `sub_choice_created`, `has_pending_actions`, `pending_repeat_actions` is scattered across `compound.rs`, `choice.rs`, `move_cards.rs`. A single `EffectContinuation` struct with explicit state machine (enums) would be clearer than boolean flags.

**Files:** `engine/src/ability/choice.rs:149` `finalize_choice`, `engine/src/ability/compound.rs:44` `execute_sequential_effect`, `engine/src/ability/move_cards.rs`

## 6. Engine – Debug Logging Pollution

**Problem:** `compound.rs` used unconditional `eprintln!("[DEBUG_SEQ]")` and `eprintln!("[DEBUG_RA]")`, polluting `cargo test` output even without `RUST_LOG=debug`. This was added during debugging and left in.

**Refactor:** Gate all `eprintln!` behind `ABILITY_DEBUG` or `log::debug!`; remove ad-hoc prints. Centralize tracing via `AbilityTraceNode` instead of multiple log channels.

**Files:** `engine/src/ability/compound.rs:64`, `150`

## 7. Engine – Condition Evaluation Duplication

**Problem:** `execute_sequential_effect` evaluates each sub-action's condition via `ConditionContext` before cloning and stripping it, then `execute_effect` → `can_activate_effect` re-evaluates the same condition (if not stripped) via `condition_cache` mechanism. The cache key is `condition.text`, which for Maki's original buggy condition included the prior clause, causing mismatched caching.

**Refactor:** Sequential should be the sole evaluator; `execute_effect` should not re-check conditions for sub-actions. Remove `can_activate` check for `sequential` children.

**Files:** `engine/src/ability/compound.rs:260`, `engine/src/ability/condition.rs:509`, `engine/src/ability/effects/mod.rs:269`

## 8. Testing – Fixture Ambiguity

**Problem:** Original `maki_pb1_006_debut_test` used `PL!-sd1-019-SD` (START:DASH!!) as waitroom filler, whose `μ's` membership is inferred via `card_series_matches_group` (`series: ラブライブ！` → μ's). This is non-obvious and caused deck-size assertions to be off by one (move+draw net 0 vs expected -1). Tests also did not assert waitroom→deck→hand flow.

**Fix applied:** Introduced `MUS_LIVE = PL!-bp3-019-L` (or explicit μ's live) and corrected assertions: `deck_after == deck_before` (move+draw), `hand` checks, `under` checks, plus 6 edge cases (all positions, multiple waits, active vs wait, empty deck, no mus, self wait).

**Refactor:** Add a helper `assert_mus_live` and centralize card ID constants with comments linking to `card_series_matches_group` logic.

**Files:** `engine/tests/test_modules/maki_pb1_006_debut_test.rs`

## 9. General – Japanese Text Handling

**Problem:** Full-width `！` vs `!`, `μ` vs `µ`, `『』` vs `「」` normalization is duplicated in `parser_utils.py` and `engine/src/ability/util.rs:491` `norm`. Parser normalizes digits via `normalize_fullwidth_digits` but not exclamation, leading to mismatched `group_names` (`μ's` vs `µ's`).

**Refactor:** Share a single `normalize_group` crate between Python and Rust (e.g. generate a JSON table).

## Summary Priority

1. **High:** Extract condition scoping (2) and `finalize_choice` state machine (5) – directly caused silent ability failures.
2. **Medium:** `_walk` propagation (1) and `_try_state` enrichment (3) – fix with shared extraction helpers.
3. **Low:** Build regeneration (4) and logging (6) – ergonomics.
