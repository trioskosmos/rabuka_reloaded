# Ability Gameplay Test Plan

This document outlines specific abilities to test in the actual game interface to verify the ability system works correctly in real gameplay scenarios.

## Test Cards Selected

### 1. Activation Abilities (起動)

**Card: 嵐 千砂都 (PL!SP-bp1-003-R+)**
- **Ability:** Reveal hand member cards, if cost total is 10/20/30/40/50, gain score +1 ability until live end
- **Test Steps:**
  1. Play 嵐 千砂都 to stage
  2. Activate ability (起動)
  3. Select hand cards to reveal
  4. Verify cost calculation
  5. Verify ability gain if condition met
- **Expected:** User choice for which cards to reveal, conditional ability gain

**Card: 夕霧綴理 (PL!HS-bp1-004-R+)**
- **Ability:** Pay 3 energy, add '蓮ノ空' live card from discard to hand
- **Test Steps:**
  1. Play 夕霧綴理 to stage
  2. Ensure '蓮ノ空' live card is in discard
  3. Activate ability (起動)
  4. Pay 3 energy
  5. Verify live card moved to hand
- **Expected:** Energy payment, group filtering, card movement

### 2. Appearance Abilities (登場)

**Card: 唐 可可 (PL!SP-bp1-002-R+)**
- **Ability:** Optional pay 2 energy, if appearing in left side, draw 2 cards
- **Test Steps:**
  1. Play 唐 可可 to LEFT SIDE specifically
  2. Choose to pay 2 energy (optional)
  3. Verify 2 cards drawn
- **Expected:** Optional cost prompt, position condition check, draw effect

**Card: 米女メイ (PL!SP-bp1-007-R+)**
- **Ability:** If energy >= 11, add live card from discard to hand
- **Test Steps:**
  1. Have 11+ energy cards
  2. Play 米女メイ to stage
  3. Verify condition check (energy count)
  4. Verify live card added to hand
- **Expected:** Condition evaluation on energy count, card movement

**Card: 日野下花帆 (PL!HS-bp1-001-R)**
- **Ability:** Activate 2 energy on appearance
- **Test Steps:**
  1. Play 日野下花帆 to stage
  2. Verify 2 energy cards become active
- **Expected:** Energy activation effect

**Card: 渡辺 曜 (PL!S-bp2-005-R+)**
- **Ability:** Look at top 7 cards, select up to 3 with heart02/04/05, add to hand, rest to discard
- **Test Steps:**
  1. Play 渡辺 曜 to stage
  2. Look at top 7 cards
  3. Select cards with specific heart colors
  4. Verify selection and movement
- **Expected:** Look_and_select with heart color filtering, user choice

**Card: 小原鞠莉 (PL!S-bp2-008-R+)**
- **Ability:** Move up to 1 live card from discard to deck bottom
- **Test Steps:**
  1. Play 小原鞠莉 to stage
  2. Verify live card moved to deck bottom (or none if no live cards)
- **Expected:** Max parameter handling, card movement to deck_bottom

### 3. Live Start Abilities (ライブ開始時)

**Card: 夕霧綴理 (PL!HS-bp1-004-R+ ab#1)**
- **Ability:** Optional pay 1 energy, gain blade per live card until live end
- **Test Steps:**
  1. Have live cards in live card zone
  2. Start live (trigger ライブ開始時)
  3. Choose to pay 1 energy (optional)
  4. Verify blade gain per live card
  5. Verify duration tracking (until live end)
- **Expected:** Optional cost, per_unit calculation, duration tracking

**Card: 藤島 慈 (PL!HS-bp1-006-R+ ab#1)**
- **Ability:** Optional discard 1, if other members on stage, gain heart of chosen color
- **Test Steps:**
  1. Have other members on stage
  2. Start live (trigger ライブ開始時)
  3. Choose to discard 1 (optional)
  4. Choose heart color
  5. Verify heart gain
- **Expected:** Optional cost, condition check (other members), user choice for heart color

### 4. Conditional Abilities

**Card: (常時 ability with compound condition)**
- **Ability:** If 3+ live cards with 1+ 虹ヶ咲, gain 2 hearts and 2 blades
- **Test Steps:**
  1. Have 3+ live cards including 虹ヶ咲
  2. Trigger 常時 ability
  3. Verify compound condition evaluation
  4. Verify resource gain
- **Expected:** Compound condition (AND), group filtering, resource gain

**Card: 小原鞠莉 (PL!S-bp2-008-R+ ab#1)**
- **Ability:** Conditional alternative - if 3+ live cards, score +2 instead of +1
- **Test Steps:**
  1. Trigger ability
  2. Test with 1-2 live cards (should be +1)
  3. Test with 3+ live cards (should be +2)
- **Expected:** Conditional alternative, card count condition

### 5. Complex Combinations

**Sequential Cost:**
- **Ability:** Reveal live card, optionally move to deck bottom, then look_and_select
- **Test Steps:**
  1. Execute sequential cost (reveal + optional move)
  2. Execute look_and_select effect
  3. Verify both parts execute
- **Expected:** Sequential cost execution, look_and_select

**Invalidate Ability:**
- **Card: 澁谷かのん (PL!SP-bp2-001-R+)**
- **Ability:** Invalidate Liella live start abilities, add Liella card from discard
- **Test Steps:**
  1. Have Liella member on stage with live start ability
  2. Play 澁谷かのん
  3. Verify invalidation
  4. Verify card addition
- **Expected:** Ability invalidation, group filtering

## Test Priority

1. **High Priority:** Basic activation and appearance abilities (simple effects)
2. **Medium Priority:** Conditional abilities, live start abilities
3. **Low Priority:** Complex combinations, invalidate abilities

## Success Criteria

- Ability triggers fire at correct timing
- Costs are paid correctly (optional and mandatory)
- Conditions are evaluated accurately
- Effects execute as described
- User choices are prompted when needed
- Card filtering (group, cost, heart color) works
- Duration tracking (live_end) works
- Sequential effects execute in order
