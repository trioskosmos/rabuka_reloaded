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
