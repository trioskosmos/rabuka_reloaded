# Mari Card (PL!S-bp2-008-R+) — Gain Ability Fix Plan

## Current State: Broken

The parser generates `action: "conditional_alternative"` for a `trigger: "常時"` ability.

### Root Cause

In `engine/src/core/game_state/modifiers.rs:135-420`, the constant evaluation dispatch (`recalculate_constants()`) handles:
- `ModifyScore` → applies score modifier
- `GainAbility` → applies gained effect directly
- `ModifyCost` → applies cost modifier
- `Restriction` → adds prohibition
- etc.

**`ConditionalAlternative` is NOT in the dispatch table** — the ability is silently skipped.

### What the Card Text Says

```
常時:
  自分のステージのエリアすべてに『Aqours』のメンバーが登場しており、かつ名前が異なる場合、
  「ライブ成功時: エールにより公開された自分のカードの中にライブカードが1枚以上ある場合、
   ライブの合計スコアを+1する。3枚以上ある場合、代わりに合計スコアを+2する。」
  を得る。
```

### What the Structure Should Be

```json
{
  "trigger": "常時",
  "effect": {
    "action": "gain_ability",
    "condition": {
      "type": "compound",
      "operator": "and",
      "conditions": [
        { "type": "appearance_condition", "all_areas": true, "group_names": ["Aqours"] },
        { "type": "location_condition", "distinct": "card_name", "group_names": ["Aqours"] }
      ]
    },
    "ability_gain": "...",
    "gained_effect": {
      "trigger": "live_success",
      "action": "conditional_alternative",
      "alternative_condition": {
        "type": "card_count_condition",
        "count": 3, "operator": ">=",
        "source": "revealed_cards",
        "card_type": "live_card"
      },
      "alternative_effect": {
        "action": "modify_score",
        "value": 2, "operation": "add"
      },
      "primary_effect": {
        "action": "modify_score",
        "value": 1, "operation": "add"
      }
    }
  }
}
```

### Required Changes

1. **Parser** (`cards/ability_extraction/parser.py`):
   - When the quoted inner ability has a different trigger (e.g. "ライブ成功時"), generate `gain_ability` instead of `conditional_alternative`
   - The `gained_effect` should contain the inner ability's effect structure

2. **Engine** (`engine/src/core/game_state/modifiers.rs`):
   - The constant `gain_ability` handler currently only handles `ModifyScore` in `gained_effect`
   - Need to handle `conditional_alternative` gained effects: evaluate conditions and apply the appropriate score
   - The gained ability's timing ("live_success") needs to be respected — score should be applied at live success, not at constant evaluation time
   - This may require registering the gained ability as a triggered ability rather than applying it immediately

3. **Tests**: E2E test with Aqours stage meeting the condition, performing a live, and verifying total score bonus.
