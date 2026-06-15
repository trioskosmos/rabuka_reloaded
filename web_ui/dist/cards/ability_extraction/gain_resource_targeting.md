# Gain Resource Targeting — Parser vs Engine

## Engine Decision Tree (`execute_gain_resource`)

**File:** `engine/src/ability/effects/misc.rs:200-868`

### Priority cascade (who gets the resource):

| Priority | Field | Behavior |
|----------|-------|----------|
| 1 | `self_target: true` | Activating card only. Short-circuits everything. Ignores group/card_type filters. |
| 2 | `is_all: true` | ALL stage members. Auto-detected when: no `source`, `card_type=="member_card"`, `target=="self"`, `self_target` is false. |
| 3 | `group_names` set | Filters stage cards by group (series/unit). Only first group used. Engine iterates ALL matching members. |
| 4 | `card_type` set | Filters by type: `"member_card"`, `"live_card"`, `"energy_card"`. Combined with group_names. |
| 5 | `target_count` set | Player chooses N from filtered candidates via SelectCard prompt. |
| 6 | Default (nothing set) | Activating card only. |

### `is_all` auto-detection (misc.rs:312-316):

```rust
let is_all = effect.all.unwrap_or(false)
    || (effect.source.is_none()
        && effect.card_type.as_deref() == Some("member_card")
        && target == "self"
        && !is_self_target);
```

When `card_type == "member_card"`, no `source`, target is self, and `self_target` is false,
the engine auto-applies the resource to ALL members on stage. This is intentional for
patterns like "gain 1 blade for each member on your stage."

### Heart distribution special cases:

- `heart_targets` non-empty but `target=="self"` with no `source`/`card_type` → activating card only (lines 777-800)
- `heart_colors` with 1 entry → fixed color, no choice prompt
- `heart_colors` with multiple → player chooses
- `heart_type: "all"` → grants `heart00` (wildcard), returns early

### Filter construction (`util.rs:600-619`):

`filter_from_parts(card_type, group_name, ...)` builds a `CardFilter`. Only the **first**
element of `group_names` is used (`card.rs:744-749`). Cards must match ALL filter criteria.

### Default behavior (no targeting fields):

Resource goes to the activating card only. This is the safe default for simple
"this card gains X blade/heart" effects.

---

## Parser Gain Resource Production

### All code paths that produce `gain_resource`:

| # | Location | Resource | Key Fields Set |
|---|----------|----------|----------------|
| 1 | L2792 dispatch | blade | resource, count |
| 2 | L2800 dispatch (icon) | blade | resource, count |
| 3 | L2810 dispatch | heart | resource |
| 4 | L2821 dispatch (icon_all) | heart | resource, heart_type="all", count |
| 5 | L2832 dispatch | heart/surplus | sign="negative", all |
| 6 | L2924 dispatch | heart | heart_selection=True |
| 7 | L3106 dispatch (bracket) | heart | heart_selection=True, heart_colors |
| 8 | L2888 dispatch (fallback) | heart | resource (inferred) |
| 9 | L3132 concurrent | blade+heart | resource, count, heart_colors (sequential) |
| 10 | L3814 _try_character_specific | blade/heart | characters, target="self", card_type="member_card" |
| 11 | L5207 _try_blade_actions | blade | resource, count |
| 12 | L5216 _try_blade_actions | blade | resource, count, duration |
| 13 | L5098 _try_lose_resource | blade/heart | sign="negative", resource, heart_colors, duration |
| 14 | L2244 per-unit | blade/heart | resource, count, duration, per_unit* |
| 15 | L5669 fallback post | blade/heart | resource, count |
| 16 | L5620 post-fallback | blade | resource, count |

### Targeting fields the parser sets:

| Field | Set when | Location |
|-------|----------|----------|
| `self_target: true` | Text contains `"このカード"` | `_fill_defaults` L1921 |
| `all: true` | Text matches `全員/すべて/全て/全体` | `_fill_defaults` L2069-2072 |
| `group_names` | Text contains `『GroupName』` | `parse_action` L2426-2428 |
| `card_type` | Text contains `メンバーカード/ライブカード` | `parse_action` L2395-2396 |
| `target` | From `extract_target(text)` | `_fill_defaults` L2398-2399 |
| `characters` | Character-specific patterns | `_try_character_specific` L3814 |
| `heart_colors` | From `{{heart_XX.png}}` icons | `_fill_defaults` L1906-1910 |

---

## The Leakage Problem

### group_names leakage onto gain_resource:

**Vector 1: `_clean_action_list` (L5729-5772)**

Propagates parent `group_names` to sub-actions. Has explicit exclusions for:
- `change_state` with `card_type == "energy_card"` (L5749-5754)
- `move_cards` when group name not in sub-action text (L5756-5763)

**No exclusion for `gain_resource`** — group_names from a condition context leaks onto
gain_resource sub-actions, causing the engine to distribute resources to ALL matching
group members instead of the activating card.

**Vector 2: `_normalize_effect_tree._walk` (L5825-5858)**

Propagates group_names from context text to any action node. Checks if group name
appears in the node's own text, but condition context text bleeds through.

### card_type leakage onto gain_resource:

**`_clean_action_list` (L5774-5778)**

```python
pt = parent_effect.get("card_type")
if pt:
    for sub in cleaned:
        if "card_type" not in sub:
            sub["card_type"] = pt
```

Propagates parent `card_type` to any sub-action that doesn't have one. Less severe
than group_names because `card_type` on gain_resource is usually set intentionally
from the action text (e.g., "メンバーカードにブレードを与える").

### Real-world example (PL!HS-bp6-005-R＋ 徒町小鈴):

Ability text: `...蓮ノ空のメンバーのコストの合計が高い場合、ハートとブレードを得る`

Parser produces:
```json
{
  "action": "sequential",
  "actions": [
    { "action": "gain_resource", "resource": "blade", "group_names": ["蓮ノ空"], ... },
    { "action": "gain_resource", "resource": "heart", "group_names": ["蓮ノ空"], ... }
  ],
  "condition": { "group_names": ["蓮ノ空"], ... }
}
```

The `group_names: ["蓮ノ空"]` should NOT be on the gain_resource actions. The ability
text says "gain heart and blade" without specifying a group — the resource should go
to the activating card only. But the leaked `group_names` causes the engine to give
blade to a different 蓮ノ空 member on stage.

---

## Fixes Applied

### Fix: `_clean_action_list` — exclude gain_resource from group_names propagation

When propagating `group_names` from parent to sub-actions, skip `gain_resource` sub-actions
that don't have `group_names` in their own text. This prevents condition context from
leaking group filters onto resource gains.

### Fix: `_clean_action_list` — exclude gain_resource from card_type propagation

Same pattern — when the sub-action is `gain_resource` and its own text doesn't mention
a card type, don't propagate the parent's `card_type`.
