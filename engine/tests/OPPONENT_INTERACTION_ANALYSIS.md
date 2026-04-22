# Opponent-Targeting Ability Control Flow Analysis

## Overview
This document analyzes the integration test for opponent-targeting abilities, explaining the card selection, ability structure, and code implementation.

## Initial Problem

### Test Output
```
UseAbility actions: 0
```

The test showed 0 UseAbility actions available, indicating the ability wasn't being recognized by the engine's action generation system.

## Card Analysis

### Original Card (Incorrect Selection)
**Card:** 国木田花丸 (PL!S-bp3-016-N)

**Ability Text:**
```
{{jyouji.png|常時}}自分の成功ライブカード置き場にあるカード1枚につき、ステージにいるこのメンバーのコストを＋１する。
```

**Ability Type:** 常時 (Constant Ability)

**Why This Failed:**
- Constant abilities (常時) are always active and don't require activation
- They modify game state continuously without player intervention
- They do NOT generate UseAbility actions
- Only activation abilities (起動) generate UseAbility actions

**Test Filter Used:**
```rust
let activation_card = cards.iter()
    .find(|c| c.is_member() && c.abilities.iter().any(|a| {
        a.triggers.as_ref().map_or(false, |t| t == "起動")
    }))
    .expect("No activation card found");
```

**Problem:** The filter was checking for `triggers == "起動"` but found a card with a constant ability instead. The card's ability doesn't have a trigger field matching "起動".

### Corrected Card Selection
**Card:** 鬼塚冬毬 (PL!SP-bp1-011-R)

**Ability Text:**
```
{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。
```

**Ability Type:** 起動 (Activation Ability)

**Why This Works:**
- Activation abilities (起動) require explicit player activation
- They generate UseAbility actions when conditions are met
- The player can choose to activate them during their turn

## Ability Structure Analysis

### From abilities.json
```json
{
  "full_text": "{{kidou.png|起動}}このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。",
  "triggerless_text": "このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える。",
  "triggers": "起動",
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
    "source": "discard",
    "count": 1,
    "card_type": "live_card",
    "type": "move_cards"
  }
}
```

### Ability Components

1. **Trigger (起動):** Indicates this is an activation ability that can be played by the player
2. **Cost:** "このメンバーをステージから控え室に置く" (Move this member from stage to discard)
3. **Effect:** "自分の控え室からライブカードを1枚手札に加える" (Add 1 live card from discard to hand)

## Rules Analysis

### From rules.txt

**Activation Ability Master (Section 3.1.2.2):**
```
起動能力のマスターとは、それをプレイしたプレイヤーを指します。
```

**Main Phase Activation (Section 7.7.2.1):**
```
自分のカードが持つ起動能力を 1 つ選び、それをプレイする。
```

**Key Rules:**
1. Activation abilities can only be played by the active player
2. They are played during the Main phase
3. The player must pay the cost to activate
4. The effect resolves after cost payment

## Code Implementation

### Test Setup Code

```rust
// Find a card with activation ability
let activation_card = cards.iter()
    .find(|c| c.card_no == "PL!SP-bp1-011-R")
    .expect("鬼塚冬毬 not found");
let activation_id = get_card_id(activation_card, &card_database);

// Place activation card on Player 1's stage (already played in previous turn)
player1.stage.stage[1] = activation_id;

// Place opponent members on stage
player2.stage.stage[0] = opponent_member_ids[0];
player2.stage.stage[1] = opponent_member_ids[1];
player2.stage.stage[2] = opponent_member_ids[2];

// Set up game state
let mut game_state = GameState::new(player1, player2, card_database);
game_state.current_phase = Phase::Main;
game_state.turn_number = 2; // Turn 2 so baton touch is allowed
game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
game_state.player1.is_first_attacker = true;

// Clear locked areas to allow ability activation
game_state.player1.areas_locked_this_turn.clear();
```

### Action Generation Code

```rust
// Generate actions for Player 1
let actions = game_setup::generate_possible_actions(&game_state);

// Check if UseAbility action is available
let ability_actions: Vec<_> = actions.iter()
    .filter(|a| a.action_type == ActionType::UseAbility)
    .collect();

println!("  UseAbility actions: {}", ability_actions.len());
```

### Verification Code

```rust
// Verify that Player 1 is the active player
let active_player = game_state.active_player();
assert_eq!(active_player.id, "player1", "Player 1 should be active");

// Verify opponent has members on stage (targetable)
let opponent_stage_count = game_state.player2.stage.stage.iter().filter(|&&id| id != -1).count();
assert!(opponent_stage_count > 0, "Opponent should have members on stage");
```

## Current Test Status

### What the Test Currently Verifies
1. Player 1 is the active player
2. Player 1 has an activation ability card on stage
3. Opponent has targetable members on stage
4. Engine generates actions for the correct player

### What the Test Does NOT Yet Verify
1. Actual execution of the activation ability
2. Cost payment (moving member to discard)
3. Effect resolution (adding live card to hand)
4. Control flow during ability execution
5. Opponent card state changes

## Next Steps for Complete Testing

To fully test opponent-targeting ability control flow, the test needs to:

1. **Execute the UseAbility action:**
```rust
if !ability_actions.is_empty() {
    let ability_action = &ability_actions[0];
    let card_id = ability_action.parameters.as_ref()
        .and_then(|p| p.card_id)
        .expect("Should have card_id");
    
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::UseAbility,
        Some(card_id),
        None,
        None,
        None,
    ).expect("Should activate ability");
}
```

2. **Verify cost payment:**
```rust
assert!(game_state.player1.waitroom.cards.contains(&activation_id),
    "Card should be in discard after cost payment");
```

3. **Verify effect resolution:**
```rust
// Check that a live card was added to hand
assert!(game_state.player1.hand.cards.len() > initial_hand_count,
    "Hand should have more cards after effect");
```

4. **Verify control returns to Player 1:**
```rust
let active_player_after = game_state.active_player();
assert_eq!(active_player_after.id, "player1",
    "Player 1 should still be active after ability execution");
```

## Conclusion

The initial test failure was due to selecting a card with a constant ability (常時) instead of an activation ability (起動). Constant abilities don't generate UseAbility actions. The test was corrected to use 鬼塚冬毬 which has an actual activation ability.

However, the current test only verifies that Player 1 is active and can generate actions. To fully prove that "each player gets control at the appropriate time," the test needs to:
1. Actually execute the ability
2. Verify the cost is paid correctly
3. Verify the effect resolves correctly
4. Verify control flow returns to the activating player

This requires completing the test implementation to execute the full ability lifecycle through normal gameplay actions.
