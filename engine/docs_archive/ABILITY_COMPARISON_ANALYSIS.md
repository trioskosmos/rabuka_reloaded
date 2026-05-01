# Ability Text vs Engine Execution Comparison

This document compares ability text from abilities.json to the actual execution logic in the Rust engine to verify correctness, automation vs choice handling, variable consideration, and identify any missing functionality.

## Test Cases Selected

### 1. Simple Activation with Cost and Effect
**Ability Text:** `{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。`
**Card:** PL!-sd1-005-SD | 星空 凛 (ab#0)
**JSON Structure:**
```json
{
  "cost": {
    "text": "このメンバーをステージから控え室に置く",
    "source": "stage",
    "destination": "discard",
    "card_type": "member_card",
    "type": "move_cards"
  },
  "effect": {
    "text": "自分の控え室からライブカードを1枚手札に加える",
    "destination": "hand",
    "count": 1,
    "card_type": "live_card",
    "target": "self",
    "action": "move_cards"
  }
}
```

**Engine Execution Analysis:**
- **Cost Execution:** `execute_move_cards` with source="stage", destination="discard", card_type="member_card"
  - ✅ Correctly moves member from stage to discard
  - ⚠️ **ISSUE:** Engine moves from center position by default, but ability text says "this member" (could be any position)
  - ✅ Variables considered: source, destination, card_type, target

- **Effect Execution:** `execute_move_cards` with source="discard", destination="hand", card_type="live_card", count=1
  - ✅ Correctly moves 1 live card from discard to hand
  - ✅ Variables considered: source, destination, card_type, count, target

**Automation vs Choice:**
- Cost: ✅ Automated (moves the specific member being activated)
- Effect: ✅ Automated (moves first matching card, no choice needed)

**Missing/Issues:**
- Cost should move the specific member being activated, not just center position
- Need to track which member is being activated to move the correct one

---

### 2. Optional Cost with Duration Effect
**Ability Text:** `{{live_start.png|ライブ開始時}}{{icon_energy.png|E}}支払ってもよい：ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。`
**Card:** PL!HS-PR-018-PR | 大沢瑠璃乃 (ab#0)
**JSON Structure:**
```json
{
  "cost": {
    "text": "{{icon_energy.png|E}}支払ってもよい",
    "optional": true,
    "type": "pay_energy",
    "energy": 1
  },
  "effect": {
    "text": "ライブ終了時まで、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る",
    "duration": "live_end",
    "action": "gain_resource",
    "resource": "blade",
    "count": 2
  }
}
```

**Engine Execution Analysis:**
- **Cost Execution:** `execute_pay_energy` with optional=true, energy=1
  - ✅ Correctly handles optional cost via `optional_cost_behavior` flag
  - ✅ Variables considered: optional, energy count
  
- **Effect Execution:** `execute_gain_resource` with resource="blade", count=2, duration="live_end"
  - ✅ Correctly adds blade modifiers
  - ✅ Variables considered: resource, count, duration
  - ⚠️ **ISSUE:** Duration tracking is not fully implemented - effect is applied immediately but may not expire at live end

**Automation vs Choice:**
- Cost: ✅ Automated based on `optional_cost_behavior` flag (always_pay/never_pay/auto)
- Effect: ✅ Automated (adds blades immediately)

**Missing/Issues:**
- Duration expiration at live_end is not implemented
- Effect should be temporary and expire when live ends

---

### 3. Look and Select with Sequential Actions
**Ability Text:** `{{toujyou.png|登場}}自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。`
**Card:** PL!S-PR-028-PR | 黒澤ダイヤ (ab#0)
**JSON Structure:**
```json
{
  "effect": {
    "text": "自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く",
    "action": "look_and_select",
    "look_action": {
      "text": "自分のデッキの上からカードを3枚見る。",
      "count": 3,
      "target": "self",
      "action": "look_at",
      "source": "deck_top"
    },
    "select_action": {
      "action": "sequential",
      "actions": [
        {
          "text": "好きな枚数を好きな順番でデッキの上に置き",
          "placement_order": "any_order",
          "action": "move_cards",
          "destination": "deck_top",
          "any_number": true
        },
        {
          "text": "残りを控え室に置く",
          "action": "move_cards",
          "destination": "discard",
          "source": "selection_remaining"
        }
      ]
    }
  }
}
```

**Engine Execution Analysis:**
- **Look Action:** `execute_look_at` with count=3, source="deck_top"
  - ✅ Correctly looks at top 3 cards
  - ⚠️ **ISSUE:** Currently just logs, doesn't store cards for selection
  - ✅ Variables considered: count, source, target

- **Select Action:** `execute_sequential_effect` with nested move_cards
  - ✅ Correctly handles sequential execution
  - ⚠️ **ISSUE:** `any_number=true` not implemented - requires user choice
  - ⚠️ **ISSUE:** `placement_order="any_order"` not implemented
  - ⚠️ **ISSUE:** `source="selection_remaining"` not recognized - special source for remaining cards

**Automation vs Choice:**
- Look: ✅ Automated
- Select: ❌ **REQUIRES CHOICE** - User must choose which cards to move and in what order
  - Engine currently doesn't prompt for this choice
  - `any_number=true` indicates user can select 0 to all cards
  - `placement_order="any_order"` indicates user chooses order

**Missing/Issues:**
- Look action doesn't store cards for subsequent selection
- No user choice mechanism for selecting cards and order
- `selection_remaining` source not implemented
- `any_number` parameter not handled
- `placement_order` parameter not handled

---

### 4. Sequential Effects
**Ability Text:** `{{live_success.png|ライブ成功時}}カードを2枚引き、手札を1枚控え室に置く。`
**Card:** PL!S-bp2-024-L | 君のこころは輝いてるかい？ (ab#1)
**JSON Structure:**
```json
{
  "effect": {
    "text": "カードを2枚引き、手札を1枚控え室に置く",
    "action": "sequential",
    "actions": [
      {
        "text": "カードを2枚引き",
        "count": 2,
        "action": "draw_card",
        "source": "deck",
        "destination": "hand"
      },
      {
        "text": "手札を1枚控え室に置く",
        "source": "hand",
        "destination": "discard",
        "count": 1,
        "action": "move_cards"
      }
    ]
  }
}
```

**Engine Execution Analysis:**
- **Sequential Execution:** `execute_sequential_effect`
  - ✅ Correctly executes actions in sequence
  - ✅ First action: draw_card with count=2
  - ✅ Second action: move_cards with source="hand", destination="discard", count=1
  - ✅ Variables considered: count, source, destination for each action

- **Draw Action:** `execute_draw` with count=2
  - ✅ Correctly draws 2 cards from deck to hand
  - ✅ Variables considered: count, source, destination

- **Discard Action:** `execute_move_cards` with source="hand", destination="discard", count=1
  - ⚠️ **ISSUE:** Requires user choice - which card to discard?
  - ✅ Engine sets up pending_choice for user selection
  - ✅ Variables considered: source, destination, count

**Automation vs Choice:**
- Draw: ✅ Automated
- Discard: ✅ **CHOICE HANDLED** - Engine correctly prompts user to select which card to discard

**Missing/Issues:**
- None significant - choice handling is correct

---

### 5. Change State with Cost Limit
**Ability Text:** `{{live_start.png|ライブ開始時}}{{toujyou.png|登場}}このメンバーをウェイトにしてもよい：相手のステージにいるコスト4以下のメンバー1人をウェイトにする。`
**Card:** PL!HS-PR-022-PR | セラス 柳田 リリエンフェルト (ab#0)
**JSON Structure:**
```json
{
  "cost": {
    "text": "このメンバーをウェイトにしてもよい",
    "state_change": "wait",
    "type": "change_state",
    "card_type": "member_card",
    "optional": true
  },
  "effect": {
    "text": "相手のステージにいるコスト4以下のメンバー1人をウェイトにする。",
    "cost_limit": 4,
    "state_change": "wait",
    "count": 1,
    "card_type": "member_card",
    "target": "opponent",
    "action": "change_state"
  }
}
```

**Engine Execution Analysis:**
- **Cost Execution:** `execute_change_state` with state_change="wait", optional=true
  - ✅ Correctly handles optional cost via `optional_cost_behavior` flag
  - ⚠️ **ISSUE:** Should change the specific member being activated to wait state
  - ✅ Variables considered: state_change, optional, card_type

- **Effect Execution:** `execute_change_state` with state_change="wait", cost_limit=4, target="opponent", count=1
  - ✅ Correctly handles cost_limit filtering
  - ✅ Correctly targets opponent's stage
  - ⚠️ **ISSUE:** Requires user choice - which opponent member to target?
  - ✅ Variables considered: state_change, cost_limit, target, count, card_type

**Automation vs Choice:**
- Cost: ✅ Automated based on `optional_cost_behavior` flag
- Effect: ❌ **REQUIRES CHOICE** - User must select which opponent member to change to wait
  - Engine currently doesn't prompt for this choice
  - `count=1` with multiple matching cards requires selection

**Missing/Issues:**
- Cost should change the specific member being activated
- Effect requires user choice but engine doesn't prompt
- `max=true` parameter (for up to count) not checked in this case

---

### 6. Group Filtering
**Ability Text:** `{{toujyou.png|登場}}手札を1枚控え室に置いてもよい：自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える。`
**Card:** PL!N-bp1-003-R＋ | 桜坂しずく (ab#0)
**JSON Structure:**
```json
{
  "cost": {
    "text": "手札を1枚控え室に置いてもよい",
    "source": "hand",
    "destination": "discard",
    "count": 1,
    "optional": true,
    "type": "move_cards"
  },
  "effect": {
    "text": "自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える",
    "destination": "hand",
    "count": 1,
    "card_type": "live_card",
    "target": "self",
    "group": {
      "name": "虹ヶ咲"
    },
    "group_names": ["虹ヶ咲"],
    "action": "move_cards"
  }
}
```

**Engine Execution Analysis:**
- **Cost Execution:** `execute_move_cards` with source="hand", destination="discard", count=1, optional=true
  - ✅ Correctly handles optional cost via `optional_cost_behavior` flag
  - ⚠️ **ISSUE:** Requires user choice - which card to discard?
  - ✅ Engine sets up pending_choice for user selection
  - ✅ Variables considered: source, destination, count, optional

- **Effect Execution:** `execute_move_cards` with group filter for "虹ヶ咲"
  - ✅ Correctly handles group filtering via `matches_group` helper
  - ✅ Correctly moves first matching card
  - ✅ Variables considered: group, card_type, destination, count, target

**Automation vs Choice:**
- Cost: ✅ **CHOICE HANDLED** - Engine correctly prompts user to select which card to discard
- Effect: ✅ Automated (moves first matching card)

**Missing/Issues:**
- None significant - choice handling is correct

---

## Summary of Findings

### Variables Considered ✅
The engine correctly considers most variables:
- source, destination
- count, max
- card_type
- target (self/opponent)
- optional
- cost_limit
- group filtering
- duration (stored but expiration not implemented)
- state_change

### Choice Handling Analysis

**Choices Correctly Handled:**
- ✅ Discarding from hand (cost with optional=true)
- ✅ Selecting cards from hand for effects

**Choices NOT Handled (Missing):**
- ❌ Look and select with `any_number=true` - user must choose which cards and how many
- ❌ Look and select with `placement_order="any_order"` - user must choose order
- ❌ Change state with multiple valid targets (e.g., opponent's stage members) - user must select which target
- ❌ Cost that moves "this member" - engine moves center by default

### Missing Functionality

1. **Duration Expiration:**
   - Effects with `duration="live_end"` are applied but never expire
   - Need mechanism to expire temporary effects at live end

2. **Look and Select Implementation:**
   - Look action doesn't store cards for subsequent selection
   - `any_number` parameter not handled
   - `placement_order` parameter not handled
   - `selection_remaining` source not implemented
   - No user choice mechanism for card selection

3. **Target Selection for Change State:**
   - When multiple valid targets exist (e.g., opponent's stage members), user must select
   - Engine doesn't prompt for this choice

4. **Specific Member Targeting:**
   - Cost that moves "this member" should move the specific member being activated
   - Engine currently moves center position by default

5. **Parenthetical Notes:**
   - Parenthetical notes like "(ウェイト状態のメンバーが持つブレードは、エールで公開する枚数を増やさない。)" are not implemented
   - These are informational but may affect game mechanics

## Recommendations

1. **Implement Duration Expiration:**
   - Add mechanism to track effect creation time and expire at live_end
   - Clean up expired temporary effects

2. **Implement Look and Select User Choice:**
   - Store looked-at cards in a temporary buffer
   - Prompt user to select cards and specify order
   - Handle `any_number` and `placement_order` parameters
   - Implement `selection_remaining` source

3. **Implement Target Selection for Change State:**
   - When multiple valid targets exist, prompt user to select
   - Add choice mechanism similar to hand selection

4. **Fix "This Member" Targeting:**
   - Track which member is being activated
   - Move that specific member instead of center by default

5. **Consider Parenthetical Notes:**
   - Evaluate if parenthetical notes affect game mechanics
   - Implement if necessary (e.g., blade counting for wait-state members)
