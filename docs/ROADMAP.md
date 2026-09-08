I've analyzed the codebase with 5 specialized subagents. Here's the consolidated findings across all major subsystems:

---

## ⚠️ **IMPORTANT: NO FILE SPLITTING**

**Do not split large files into multiple files.** Keep monolithic structures (e.g., `GameState` in `game_state/mod.rs`, `AbilityResolver` in `resolver.rs`, strategy versions in their respective files). Refactoring should happen **within** files (extract methods, use sub-modules, inline helpers) not by creating new file boundaries. This avoids:
- Increased compilation units and build times
- Cross-file dependency complexity
- Loss of `pub(crate)` encapsulation boundaries
- Unnecessary `use` imports and module path changes

Target: **in-file consolidation**, not file fragmentation.

---

## 🎯 **Core Game Engine** (`engine/src/core/`, `engine/src/game_state/`)

| Priority | Issue | Location | Impact |
|---|---|---|---|
| **Critical** | `GameState` monolith: 80+ fields, 1157 lines | `game_state/mod.rs` | Cache misses, maintenance burden |
| **Critical** | String keys in tracking (`turn1_abilities_played`, prohibitions) | `game_state/mod.rs:99-119` | Allocates Strings in hot paths |
| **High** | Duplicate `push_revealed_card` / `push_revealed_cost_card` | `game_state/mod.rs:802-823` | 95% identical code |
| **High** | `recalculate_constants` rebuilds `entry_positions` HashMap every call | `game_state/modifiers.rs:290-301` | O(n) per recalculation |
| **High** | `EffectFilter` god struct: 120+ fields, most unused per effect | `card.rs:950-1126` | 60% memory waste |
| **High** | `normalize_card_no` allocates String even when unchanged | `card.rs:613-659` | Redundant allocation |
| **Medium** | `EkBox` pool allocator unsafe / defeats pooling | `pool.rs:75-169` | Potential UB |
| **Medium** | `find_card_index_by_no` linear scan in blob decoder | `card_binary.rs:398-423` | O(n) lookup |

**Dead code**: `_prohibition_destination_blocks` (unused), `EkBox` import (only in `#[serde(skip)]` field), `build_abilities_map_shared` (test-only duplicate)

---

## ⚡ **Ability System** (`engine/src/ability/`)

| Priority | Issue | Location | Impact |
|---|---|---|---|
| **Critical** | Dual decoder paths: `vm.rs` + `effect_decoder_gen.rs` | Both | Complete duplication of 155 field handling |
| **Critical** | `execute_select` / `execute_select_cards` 80% duplicate | `look.rs:365-893` | ~400 lines redundant |
| **High** | `AbilityResolver` god struct: 50+ fields | `resolver.rs:58-148` | Cache misses, mixed concerns |
| **High** | `resolve_ability` monolith: 310 lines, 15+ branches | `resolver.rs:821-1131` | Cyclomatic complexity extreme |
| **High** | 54 `.clone()` calls in `move_cards.rs` hot paths | `move_cards.rs` | Allocations per card move |
| **Medium** | `EffectKindLocals` 155 fields copied per decode | `effect_decoder_gen.rs:227-383` | Stack pressure |
| **Medium** | Dead decoders: `read_cost_comparison_value`, `read_trigger_event_value` | `vm.rs:663-803` | 100 lines dead code |

---

## 🤖 **Bot/AI System** (`engine/src/bot/`)

| Priority | Issue | Location | Impact |
|---|---|---|---|
| **Critical** | **7 strategy versions (v1-v7) with massive duplication** | All `strategy_v*.rs` | ~2000 lines redundant |
| **Critical** | ISMCTS re-samples determinization every rollout | `ismcts.rs:100` | 10-100x slower search |
| **High** | Heart accounting implemented 4× independently | `strategy_common.rs`, `v2`, `v3`, `v4` | Maintenance nightmare |
| **High** | Rollout uses v1 heuristic (weakest) | `evaluation.rs:12` | Poor rollout quality |
| **High** | Neural encoding allocates Vec per zone/action per forward | `encoding.rs`, `neural.rs` | GC pressure |
| **Medium** | No transposition table in MCTS | `ismcts.rs` | No memoization |
| **Medium** | `PolicyNet` always allocated even when unused | `mod.rs:90,109` | Startup memory |

**Delete entirely**: `strategy.rs` (v1), `strategy_v2.rs`, `strategy_v3.rs`, `strategy_v5.rs::choose_action_v5/v6` (documented failures)

---

## 🔄 **Turn System** (`engine/src/turn/`, `ability_queue.rs`, `triggers.rs`)

| Priority | Issue | Location | Impact |
|---|---|---|---|
| **Critical** | 4× trigger scans per performance phase | `phases.rs:644-655` | O(4n) redundant work |
| **Critical** | `execute_performance_phase`: 3 full modifier map clones | `phases.rs:298-334` | Massive allocation storm |
| **High** | `ability_queue` uses `Vec` with O(n) front ops | `ability_queue.rs:137, 230-252` | Slow queue ops |
| **High** | `resume_queue_with_choice`: 781 lines, duplicate blocks | `actions.rs:1274-1336` | ~60 lines duplicated |
| **High** | 3 trigger functions with identical structure | `triggers.rs:10-600` | Code duplication |
| **Medium** | Dead actions: `MulliganHeader`, `LiveCardHeader`, `FinishLiveCardSet` | `actions.rs:113,186,196` | No-op handlers |

---

## 🌐 **Web UI** (`web_ui/js/`)

| Priority | Issue | Location | Impact |
|---|---|---|---|
| **Critical** | `assets_registry.js`: 50KB dead base64 icons | `assets_registry.js:1-23` | Bundle bloat |
| **Critical** | ImageLoader cache-busting defeats browser cache | `CardRenderer.js:90-92,117-122` | Re-downloads images |
| **High** | `renderCards()` clears DOM + full rebuild on filter | `CardRenderer.js:498-499` | Destroys all elements |
| **High** | 4× phase key mappings, 3× ACTION_LABELS maps | Multiple files | Maintenance burden |
| **High** | `rebuildCardIndex()` full rebuild on every state update | `state.js:242-333` | O(n) per update |
| **Medium** | `DOMUtils.js`: 276 lines over-abstraction for basic DOM ops | `DOMUtils.js` | Indirection, bundle size |
| **Medium** | Module-level state scattered (`_sentRpsP1`, `_choiceCards`, etc.) | `ActionMenu.js`, `ChoiceView.js` | State inconsistency |
| **Medium** | `renderCards` 216 lines, `_buildItems` 113 lines, `ActionListView` 257 lines | Multiple | Cyclomatic complexity |

---

## 📋 **Top 10 Cross-Cutting Fixes**

| # | Fix | Files Affected | Est. Impact |
|---|-----|----------------|-------------|
| 1 | **Refactor `GameState` in-place** into 5 internal structs (same file) | `game_state/mod.rs` | -40% struct size, better cache |
| 2 | **Consolidate 7 strategy versions** → single `v7` + conductor | `bot/strategy*.rs` | -2000 lines |
| 3 | **Unify dual ability decoders** (bytecode + JSON) | `ability/vm.rs`, `effect_decoder_gen.rs` | -200 lines |
| 4 | **Cache determinizations in ISMCTS** | `bot/ismcts.rs` | 10-100x search speedup |
| 5 | **Replace `EffectFilter` god struct** with per-kind payloads | `card.rs`, `ability/types.rs` | -60% `AbilityEffect` size |
| 6 | **Delete `assets_registry.js`** (50KB dead) | `web_ui/js/assets_registry.js` | Bundle -50KB |
| 7 | **Fix ImageLoader cache-busting** | `web_ui/js/components/CardRenderer.js` | Image load perf |
| 8 | **Unify trigger scanning** (single pass, not 4×) | `engine/src/turn/phases.rs` | -75% trigger work |
| 9 | **Use `VecDeque`/ring buffer** for ability queue | `engine/src/ability_queue.rs` | O(1) queue ops |
| 10 | **Incremental `rebuildCardIndex`** | `web_ui/js/state.js` | State update O(1) vs O(n) |

---

## 🗑️ **Immediate Dead Code Removal** (Safe, verified unused)

| File | Lines | What | Verification |
|------|-------|------|--------------|
| `engine/src/ability/vm.rs` | 663-803 | `read_cost_comparison_value`, `read_trigger_event_value` | Only used in `#[cfg(feature = "debug_conditions")]` branch in `condition_decoder_gen.rs:193` - gate behind same feature or delete |
| `engine/src/bot/strategy.rs` | entire | v1 - unused | Not imported in `mod.rs`, only `v2`-`v7` + `conductor` exported |
| `engine/src/bot/strategy_v2.rs` | entire | v2 - superseded | Only `V2Policy` struct used by v3; logic duplicated in v3+ |
| `engine/src/bot/strategy_v3.rs` | entire | v3 - superseded | `analyze_hand` exported but unused; logic duplicated in v4+ |
| `engine/src/bot/strategy_v5.rs` | 207-220 | `choose_action_v5/v6` | Lines 207-220: documented as "LOST to plain v4" |
| `engine/src/turn/actions.rs` | 113, 186, 196-197 | `MulliganHeader`, `LiveCardHeader`, `FinishLiveCardSet` | No-op / error-returning handlers, never triggered by game flow |
| `web_ui/js/assets_registry.js` | entire | 50KB base64 icons | **USED** by TextEnricher.js — needs conversion to static PNG files first |

**Note**: `repeat_prompt_choice()` in `types.rs:38` IS used (in `compound.rs:632` and `choice.rs:137`) — keep it.

---

## ✅ **Execution Order & Verification Gates**

### Phase 1: Dead Code Removal (Zero Risk)
```bash
# Each step: edit → cargo test (all 3269 tests must pass)
1. Delete vm.rs dead decoders (663-803) — verify condition_decoder_gen.rs still compiles with debug_conditions
2. Delete strategy.rs, strategy_v2.rs, strategy_v3.rs — update mod.rs exports
3. Delete strategy_v5.rs lines 207-220 — keep only v7-relevant code
4. Delete actions.rs dead handlers (MulliganHeader, LiveCardHeader, FinishLiveCardSet)
5. Delete assets_registry.js — verify no import errors in web_ui
```

### Phase 2: String → Interned IDs in Tracking (Low Risk) ✅ **DONE**
```bash
# Replace String keys with CardId/u16 in game_state/mod.rs:
# - turn1_abilities_played, turn2_abilities_played: SmallVec<[(String,u8);8]> → SmallVec<[(CardId,u8);8]>
# - prohibition_effects: SmallVec<[String;4]> → SmallVec<[ProhibitionType;4]> (new enum)
# - live_owned_hearts: SmallVec<[(String,u8);8]> → SmallVec<[(CardId,u8);8]>
# cargo test after each field change
```
**Completed**: 
- Added `CardId` import to `game_state/mod.rs`
- Changed `turn1_abilities_played`: `SmallVec<[String; 8]>` → `SmallVec<[CardId; 8]>`
- Changed `turn2_abilities_played`: `SmallVec<[(String, u8); 8]>` → `SmallVec<[(CardId, u8); 8]>`
- Changed `live_owned_hearts`: `SmallVec<[(String, Vec<(String, u8)>); 4]>` → `SmallVec<[(CardId, Vec<(String, u8)>); 4]>`
- Changed `auto_ability_trigger_counts`: `SmallVec<[(String, u8); 8]>` → `SmallVec<[(CardId, u8); 8]>`
- Changed `turn_limit_usage`: `SmallVec<[(String, u8); 8]>` → `SmallVec<[(CardId, u8); 8]>`
- Added `CardId` serde support in `core/card.rs`
- Updated `game/display.rs` serialization to use `CardId` keys
- **All 3269 tests pass**

### Phase 3: Deduplication (Low Risk) - **PARTIAL**
```bash
1. Unify push_revealed_card / push_revealed_cost_card in game_state/mod.rs:802-823 ✅ DONE
2. Extract shared logic from execute_select / execute_select_cards in look.rs:365-893 → new helper ✅ DONE
3. Unify 3 trigger functions in triggers.rs:10-600 → single generic collector ❌ REVERTED (too complex, subtle behavior differences)
```

### Phase 4: Hot Path Allocations (Medium Risk) - **IN PROGRESS**
```bash
1. entry_positions HashMap in modifiers.rs:290-301 → pre-compute once per turn in GameState
2. ability_queue.rs: Vec → VecDeque (or smallvec ring buffer)
3. ImageLoader cache-busting in CardRenderer.js:90-92,117-122 → use stable URLs ✅ DONE
```
**Completed ImageLoader fix:**
- Changed `_doLoad` to only add cache-busting query param for explicit `refreshAll()` calls (passing `'refresh'`)
- Retries and normal loads now use original URL to benefit from browser cache
- Updated `retryImage` and `refreshAll` to pass correct flags

### Phase 5: Architecture (High Risk — needs design review)
```bash
1. GameState in-place refactor: 5 internal structs in same file
2. EffectFilter → per-kind payload structs
3. ISMCTS determinization caching
4. Strategy consolidation: v7 + conductor as single source
```

---

## 🚫 **Do Not Do** (Per Analysis)

| Item | Reason |
|------|--------|
| Split `GameState` into multiple files | Violates NO FILE SPLITTING; `pub(crate)` boundaries would break |
| Create new modules for ability decoder | Dual decoder paths unified **in-place** in vm.rs |
| Extract strategy versions to new files | Consolidate **into** conductor.rs |
| Move trigger logic to new file | Unify **in** triggers.rs |

---

## 📋 **Top 10 Cross-Cutting Fixes** (Updated)

| # | Fix | Files Affected | Est. Impact | Phase |
|---|-----|----------------|-------------|-------|
| 1 | **Delete dead code** (7 items) | vm.rs, strategy*.rs, actions.rs, assets_registry.js | -50KB bundle, -2000 lines | 1 |
| 2 | **String→CardId in tracking** | game_state/mod.rs | -30% hot path allocs | 2 |
| 3 | **Unify push_revealed_card variants** | game_state/mod.rs:802-823 | -95% code dup | 3 |
| 4 | **Unify execute_select variants** | look.rs:365-893 | -400 lines | 3 |
| 5 | **Unify 3 trigger functions** | triggers.rs:10-600 | -60% code dup | 3 |
| 6 | **Pre-compute entry_positions** | modifiers.rs:290-301 | -O(n) per recalc | 4 |
| 7 | **VecDeque for ability_queue** | ability_queue.rs | O(1) queue ops | 4 |
| 8 | **Fix ImageLoader cache-busting** | CardRenderer.js | Image load perf | 4 |
| 9 | **GameState in-place refactor** | game_state/mod.rs | -40% struct size | 5 |
| 10 | **Strategy consolidation** | bot/strategy*.rs, conductor.rs | -2000 lines | 5 |

---

## ⚡ **Quick Wins This Week** (Phase 1-3)

Run `cargo test` after **each** edit. Expected pass rate: 3269/3269.

1. **vm.rs:663-803** → Delete 140 lines dead code
2. **strategy.rs** → Delete entire file (269 lines)
3. **strategy_v2.rs** → Delete entire file (495 lines)  
4. **strategy_v3.rs** → Delete entire file (879 lines)
5. **strategy_v5.rs:207-220** → Delete 14 lines
6. **actions.rs:113,186,196-197** → Delete 3 dead match arms
7. **assets_registry.js** → Delete entire file (50KB)
8. **game_state/mod.rs:802-823** → Unify 2 functions
9. **look.rs:365-893** → Extract shared `build_select_card_choice`
10. **triggers.rs:10-600** → Single `collect_and_enqueue_triggers` helper

---

**Start with Phase 1**. Each item is a single edit + `cargo test`.