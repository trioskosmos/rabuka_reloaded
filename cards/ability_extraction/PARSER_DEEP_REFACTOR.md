# Parser Deep Refactoring — Findings & Plan

## Current State
- 2306 engine tests pass (4 bp6-003 broken by recent change — needs fix)
- 0 custom actions (was 1)
- Validation: 0 issues

---

## Finding 1: `parse_action` is a 460-line monolith (parser.py:2587-3053)

Does 21 things in order: strip parens, per-unit extraction, duration, count/target/card_type, constraints, source, destination, cost_limit, card_name, distinct, state_change, count fallback, card_type, target, position, exclude_self, group_names, ability_gain (early return), quoted_text, optional, max, heart+blade split, then DISPATCH TABLE.

Steps 1-18 are unconditional field extraction before dispatch. Most are only relevant for specific action types but run for EVERY action.

**Refactor**: Extract into `_extract_common_fields(text, action)` helper. parse_action goes from 460 to ~150 lines.

---

## Finding 2: `place_energy_under_member` promotion misnames member cards

**Location**: parser.py:5764-5770

When `source == "under_member"`, promotes to `place_energy_under_member` and sets `energy_count: 1`. When `destination == "energy_deck"`, forces `card_type: "energy_card"`. The Rust engine's handler hardcodes `card_type=energy_card` for the selection filter — ignoring the parser's `card_type: "member_card"`.

**Impact**: PL!-bp6-003 (kotori) LiveSuccess fails. Engine asks for energy_card but test puts member_card under center.

**Refactor**: (1) Promotion code should check text for card_type BEFORE promoting. (2) Rust engine should respect card_type from parsed effect. (3) Consider renaming to `place_under_member` and letting card_type determine the filter.

---

## Finding 3: `_fill_defaults_move_cards` demotes move_cards to custom for zone-only destinations

**Location**: parser.py:5948-5955

When destination is "stage"/"live_card_zone"/"success_live_zone" and source is missing, demotes to `custom`. This is a heuristic that breaks legitimate cases (appear-to-empty-slot).

**Current workaround**: Text-based check "メンバーのいないエリア" — fragile.

**Refactor**: Instead of demoting to custom, set `unresolved_source: true`. Or move logic to `_resolve_move_sources`.

---

## Finding 4: `_try_*` cascade runs 84 functions sequentially

**Largest _try_* functions** (>100 lines):

| Function | Lines | What |
|---|---|---|
| `_try_per_unit` | 376 | 6 jobs: detect, extract, classify, parse groups/hearts, split sequential, propagate |
| `_try_live_mid` | 191 | Multiple structural patterns for live-card-zone effects |
| `_try_conditional` | 129 | Generic conditional with 15+ special-case if blocks |
| `_try_heart_select_reveal` | 123 | 3-step pipeline: select color, reveal, conditionally act |
| `_try_place_under_heart_copy` | 119 | Place under + copy heart type |
| `_try_look_and_select` | 118 | Look at N, select some, discard rest |
| `_try_conditional_sequential` | 113 | select+pay_energy+followup splitting |

**Refactor priorities**:
1. `_try_per_unit` (376 lines): Split into detection, extraction, type classification, group/heart parsing, sequential splitting, propagation
2. `_try_conditional` (129 lines): Extract 15+ special-case if blocks into post-parse enrichers
3. Consider table-driven approach for complex pattern matching

---

## Finding 5: `_walk` sub-walkers accumulated special cases

11 sub-walkers called by `_walk`. `_walk_propagate_text_context_fields` (90 lines) handles exclude_self, distinct, same_name, original_value, group_names each with special-case logic.

**Refactor**: Group related propagations: `_walk_propagate_identity` (same_name/distinct/group_names), `_walk_propagate_targeting` (exclude_self/position).

---

## Finding 6: Two validation systems (now merged — done)

Was: `_validate_semantic` (581 lines) + `_validate_output` (241 lines) with 7 duplicated checks.
Now: Single `_validate_semantic` with 33 rules + 4 structural checks. DONE.

---

## Finding 7: `extract_cost_operator` fully duplicated (now fixed — done)

Was: parser.py had its own `extract_cost_operator` with dead `より大きい` pattern.
Now: Uses `extract_operator` from parser_utils. DONE.

---

## Finding 8: `POSITION_KEYWORDS` duplicated (now fixed — done)

Was: Identical dict in both parser.py and parser_utils.py.
Now: Imported from parser_utils. DONE.

---

## Fix Priority (what to do NOW)

### P0: Fix bp6-003 regression
The `place_energy_under_member` handler in Rust hardcodes `card_type=energy_card`. Need to either:
- Make Rust engine respect `card_type` from parsed effect
- OR revert the S-bp7-007 fix and handle it differently

### P1: `_try_per_unit` breakdown (376 lines)
Split into 5-6 focused helpers. Highest ROI refactoring remaining.

### P2: `parse_action` field extraction extraction
Extract 460-line monolith into `_extract_common_fields` + dispatch.

### P3: `_try_conditional` cleanup
Extract 15+ special-case if blocks into post-parse enrichers.

### P4: `_walk` sub-walker grouping
Group 11 sub-walkers into 4-5 logical groups.
