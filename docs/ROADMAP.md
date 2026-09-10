## Summary of Major Findings

I've run 5 sub-agents analyzing different aspects of the codebase. Here are the **critical gaps** found:

---

### 1. **Parser/JSON Gaps** (30 action types → 43 actual)
- **Missing parser support for `「選び」` conjugation** — 44 occurrences used interchangeably with `「選ぶ」` but parser may only handle dictionary form
- **9 action types in JSON with no parser coverage**: `conditional_on_optional`, `conditional_on_result`, `invalidate_ability`, `restriction`, `set_heart_type`, `set_card_identity`, `modify_required_hearts`, `modify_yell_count`, `place_energy_under_member`
- **Missing detection patterns**: `optional` (`「〜してもよい」`), `max` (`「〜枚まで」`), `placement_order` (`「好きな順番で」`)

---

### 2. **Engine Executor Gaps** — 66 ActionType variants, 23 unused
- **All 66 have match arms** but **23 never used** by any card
- **7 internal variants** warn if dispatched (correct defensive coding)
- **Zero missing handlers / wrong delegation** — dispatch is exhaustive

---

### 3. **Zone Operation Inconsistencies** — 11 missing ZoneId variants
| Zone | Missing In |
|------|------------|
| `those_cards`, `selected_cards`, `preceding_moved`, `recently_moved` | `util::zone_cards`, condition evaluation, `resolve_zone_card_count` |
| Stage sub-zones (`center`, `left`, `right`, `front`) | ZoneId conversion |
| Special markers (`preceding_moved`, `recently_moved`) | ZoneId equivalent |

- **Hardcoded "self"/"opponent" in 100+ locations** — should use `spawn_context.target` or `action_by`
- `execute_move_cards` uses `effect.target` instead of `spawn_context.target` (correct pattern documented but not used)

---

### 4. **Conditional Logic Gaps**
- **Special zones (`those_cards`, `selected_cards`, `preceding_moved`, `recently_moved`)** not resolved in condition evaluation
- **Inconsistent `ConditionContext`** — sequential top-level check lacks `moved_cards`; `selected_card_ids` only in 1 code path
- **Compound conditions don't propagate `selected_card_ids`** to sub-evaluations

---

### 5. **Cost/Energy System Gaps**
- **`energy_condition` cost type**: validated but **no handler** in `pay_cost_inner` (cost.rs:1097)
- **`execute_pay_energy` ignores `spawn_context.target`/`action_by`** (effects/misc.rs:3740, 3761)
- **`execute_pay_energy_from_revealed` missing** entirely
- **Hardcoded "self" in optional energy payment** (cost.rs:1160)
- **Cost `cost_paid` tracking inconsistent** for choice-creating costs

---

### Priority Fixes (Impact Order)
1. **Parser**: Add `「選び」` support + missing action types
2. **ZoneId**: Add 11 missing variants + fix conversions
3. **Cost**: Implement `energy_condition` handler + fix target resolution in `execute_pay_energy`
4. **Conditions**: Add special zone resolution + propagate `selected_card_ids`
5. **Hardcoded targets**: Audit 100+ locations for `spawn_context.target`/`action_by` usage

Want me to start implementing any specific fix?