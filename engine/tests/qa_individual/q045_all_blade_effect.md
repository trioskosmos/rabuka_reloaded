# Test Plan: All Blade Effect

## 1. Test Objective
Test the "all blade" effect (全てのブレード). This tests:
- Blade granting to all members on stage
- Target selection for "all" effects
- Blade modifier application to multiple cards
- Blade timing and duration

## 2. Card Selection
- **Card:** A member card with ability "grant 1 blade to all members on stage"
- **Card Name:** [Select card with all blade effect]
- **Why this card:** Tests "all" targeting and blade granting to multiple cards

## 3. Initial Game State

**Player 1:**
- `hand`: [all_blade_card, member_card_1]
- `main_deck`: [energy cards...]
- `stage`: [member_card_2, member_card_3, member_card_4, -1, -1]
- `waitroom`: []
- `energy_zone`: [energy cards...]
- `success_live_card_zone`: []
- `live_card_zone`: []

**Player 2:**
- Same structure (not affected by this test)

**Other State:**
- `turn`: 1
- `current_player`: Player 1
- `phase`: Main
- `baton_touch_count`: 0

## 4. Expected Action Sequence

**Step 1: Play all_blade_card to center stage**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::PlayMemberToStage, card_id=all_blade_card, area=Center
- Expected: Card placed in center stage, debut ability triggers

**Step 2: Debut ability executes - grant blades to all members**
- Engine function called: `AbilityResolver::execute_gain_resource`
- Parameters: resource="blade", target="all", count=1
- Expected: Blade modifier +1 added to member_card_2
- Expected: Blade modifier +1 added to member_card_3
- Expected: Blade modifier +1 added to member_card_4
- Expected: Blade modifier +1 added to all_blade_card (self)

**Step 3: Verify blade modifiers**
- Expected: member_card_2 has +1 blade
- Expected: member_card_3 has +1 blade
- Expected: member_card_4 has +1 blade
- Expected: all_blade_card has +1 blade

**Step 4: Move member_card_2 to waitroom**
- Engine function called: `TurnEngine::execute_main_phase_action`
- Parameters: ActionType::MoveMember, card_id=member_card_2, to_zone=waitroom
- Expected: member_card_2 moved to waitroom
- Expected: Blade modifier cleared for member_card_2

**Step 5: Verify remaining blade modifiers**
- Expected: member_card_3 still has +1 blade
- Expected: member_card_4 still has +1 blade
- Expected: all_blade_card still has +1 blade

## 5. User Choices

None - auto ability executes automatically on debut.

## 6. Expected Final State

**Player 1:**
- `stage`: [-1, all_blade_card, member_card_3, member_card_4, -1]
- `waitroom`: [member_card_2]
- Blade modifiers: all_blade_card (+1), member_card_3 (+1), member_card_4 (+1)

**Player 2:**
- Unchanged

## 7. Verification Assertions

- Blade modifiers granted to all members on stage
- Blade modifiers granted to activating card (self)
- Blade modifiers are independent per card
- Blade modifiers cleared when card leaves stage
- "All" targeting correctly identifies all stage members
- Blade effect executes correctly on debut
