# Transpiler & Engine Parity Plan

## Goal
Achieve Rust↔C parity so the generated test suite passes (target: >90% pass rate).

## Current State
- **1377 passing / 1650 failing** (45% pass rate)
- Build is clean, tests run
- Major gaps: choice emission, test infrastructure, transpiler coverage, helper expansion

---

## Phase 1: Core Engine Gaps (Highest Impact)

### 1.1 Choice Emission Pipeline
| Missing | Location | Rust Equivalent |
|---------|----------|-----------------|
| `rb_queue_pause_for_choice` called from effects | `src/ability/effects/*.c` | `AbilityQueue::pause_for_choice` |
| `rb_emit_choice` for SELECT_CARD/SELECT_TARGET/NUMBER | `src/engine.c` handle_action | `engine::emit_choice` |
| Choice resume continuation (`resume_parent`, `resume_child`) | `src/engine.c` `handle_action` | `pending_actions` parking |
| Auto-ability choice drain | `src/engine.c` `main_phase` | `drain_auto_choices` |

**Validation**: Shizuku test emits `SelectCard` → `SelectTarget` → `SelectNumber` choices correctly.

### 1.2 Queue Drain After Every Effect
**Current**: Only called in `rb_activate_card`, `rb_play_member`, `main_phase` loop
**Needed**: After every `rb_execute_effect_ex` call that may emit choices
- `rb_activate_card` ✓ (added)
- `rb_play_member` ✓ (added)  
- `rb_fire_auto` ✓ (added)
- `rb_activate_ability_effect` (missing)
- `rb_fire_debut` (missing)
- Staged member effects in `main_phase` loop (missing)

### 1.3 Live Card Zone Logic
| Missing | Rust | C Status |
|---------|------|----------|
| `live_success` zone handling | `turn/live.rs` | `src/turn/live.c` partial |
| Live success draw/discard | `live.rs` | Missing |
| Live card zone → waitroom on failure | `live.rs` | Missing |
| `rb_perform_live` full implementation | `live.rs::perform_live` | Stub only |

---

## Phase 2: Test Infrastructure (`test_game.c`)

### 2.1 Missing Test Helpers
| Rust Helper | C Status | Priority |
|-------------|----------|----------|
| `game.select_indices(&[idx])` | ✅ `test_select_indices` added | High |
| `game.select_option(n)` | ✅ `test_resume_choice` added | High |
| `game.find_live_by_score(score)` | ❌ Missing | High |
| `game.id("PL!...")` | ⚠️ Partial | Medium |
| `game.give_energy(n)` | ✅ `test_give_energy` | High |
| `game.give_opp_energy(n)` | ✅ `test_give_opp_energy` added | High |
| `game.pass()` | ✅ `test_pass` | High |
| `game.set_live_card(cid)` | ✅ `test_set_live_card` | High |

### 2.2 Database Integration
| Missing | Needed For |
|---------|------------|
| `test_find_live_by_score(score)` | Shizuku test, live score tests |
| `test_zone_has_card_no(pl, zone, "PL!...")` | Zone assertions |
| Card database access from C | All card lookups |

---

## Phase 3: Transpiler Coverage (`gen_tests.py`)

### 3.1 Helper Expansion (Critical)
Currently only per-test-file helpers are collected. **Must load global helpers from `engine/tests/helpers/mod.rs`** and merge into each test's helper dict.

**Helpers needing expansion** (from `engine/tests/helpers/mod.rs`):
| Helper | Calls in Tests | Rust Idioms |
|--------|----------------|-------------|
| `fill_decks` | 52 | `for _ in 0..N` + `.push()` |
| `give_energy` | 324 | `for _ in 0..count` + `.push()` |
| `drain_auto_ability_choices` | 70 | `while let Some(...) = ...` |
| `select_indices` | 150+ | `if let Some(...)` + `resume_with_choice` |
| `select_option` | 80+ | Direct call |
| `pass` | 200+ | `TurnEngine::execute_main_phase_action(Pass)` |

### 3.2 Rust Idiom → C Mapping
| Rust Pattern | C Mapping | Status |
|--------------|-----------|--------|
| `for _ in 0..N { body }` | `for (int i=0; i<N; i++) { body }` | ✅ `expand_for_loops` |
| `for _ in 0..var { body }` | Skip (variable bound) | ❌ |
| `game.state.playerN.zone.cards.push(x)` | `test_add_to_zone(&tg, pl, x)` | ✅ |
| `game.state.mods.get_heart_modifier(id, HeartN)` | `test_get_heart_modifier` | ⚠️ |
| `game.select_indices(&[0])` | `test_select_indices` | ✅ |
| `game.select_option(n)` | `test_resume_choice` | ✅ |
| `assert_eq!(a, b)` | `CHECK_EQ(a, b)` | ✅ |
| `assert!(cond)` | `CHECK(cond)` | ✅ |
| `if let Some(x) = expr { }` | `if (expr >= 0) { int x = expr; }` | ⚠️ |

### 3.3 `execute_main_phase_action` Mapping
| Rust ActionType | C Function | Status |
|-----------------|------------|--------|
| `UseAbility` | `test_activate_ability` | ✅ |
| `Pass` | `test_pass` | ✅ |
| `PlayMemberToStage` | `test_play_to_stage` | ✅ |
| `RockChoice/ScissorsChoice/PaperChoice` | ❌ | ❌ |
| `ChooseFirstAttacker` | ❌ | ❌ |
| `SkipMulligan/MulliganHeader` | ❌ | ❌ |
| `RockChoice` etc. (RPS) | ❌ | ❌ |

---

## Phase 4: Engine Logic Parity

### 4.1 Energy Payment Logic (Shizuku Test)
- Cost: discard hand card → choose live from waitroom → optional pay energy
- Current: `test_activate_ability` doesn't drain queue after activation
- Fix: `test_activate_ability` must call `rb_drain_ability_queue`

### 4.2 Modifier System
| Modifier | C Function | Used In Tests |
|----------|------------|---------------|
| Blade | `rb_mods_get_blade` | Blade stacking tests |
| Heart | `rb_mods_get_heart` | Heart modifier tests |
| Score | `rb_mods_get_score` | Score tests |
| Cost | `rb_mods_get_cost` | Cost reduction tests |

### 4.3 Auto-Ability Triggering
- `rb_fire_auto` must call `rb_drain_ability_queue` after
- `rb_fire_debut` must drain queue
- `rb_fire_recorded_auto` must drain queue

---

## Validation Plan

### Test Targets (Priority Order)
1. **Shizuku BP5** - Full choice chain + energy payment
2. **Live success** - `live_success_both_sides_draw_discard_advances_turn`
3. **Position change** - `q255_dancing_stars_live_success_after_position_change`
4. **Deck refresh** - `q267_deck_exhausts_mid_mill_refreshes_and_completes`
5. **Energy payment** - `ruby_bp5009_accept_optional_payment_fetches_and_grants_blades`

### Success Criteria
- **Phase 1**: >60% pass rate (choice emission working)
- **Phase 2**: >75% pass rate (test infrastructure complete)
- **Phase 3**: >85% pass rate (transpiler covers 90%+ of helper calls)
- **Phase 4**: >95% pass rate (engine logic parity)

---

## Open Questions

1. **RPS phase**: Do we need full RPS implementation or can tests skip?
2. **Card database**: Should we generate `test_find_live_by_score` from the baked card DB?
3. **Mulligan/RPS phases**: Can tests mock these or must we implement?

---

## File Targets Summary

| File | Changes Needed |
|------|----------------|
| `src/engine.c` | Queue drain after effects, RPS actions, `rb_execute_main_phase_action` |
| `src/ability/choice.c` | Choice emission from effects, `rb_get_pending_choice` |
| `src/ability/ability_queue.c` | `rb_queue_pause_for_choice` from effects |
| `src/ability/effects/*.c` | Call `rb_queue_pause_for_choice` on choice actions |
| `src/test_game.c` | `test_find_live_by_score`, `test_select_indices`, DB lookups |
| `tools/gen_tests.py` | Global helper loading, `UseAbility`/`Pass`/`RPS` actions, `select_indices` |
| `src/turn/live.c` | Full live success logic |