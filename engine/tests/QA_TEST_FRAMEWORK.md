# QA Test Framework and Ideology

## Purpose
The primary goal of QA-based tests is to stress-test the game engine by simulating real gameplay scenarios from official Q&A data. These tests should:

1. **Use real card abilities** - No synthetic/mock abilities. Load actual cards from `cards.json`.
2. **Play the game realistically** - Simulate the webapp gameplay flow: set up game state, choose actions, resolve abilities.
3. **Make the engine fail** - Edge cases, complex interactions, and unusual scenarios that might reveal bugs.
4. **Verify expected behavior** - Compare engine output against official Q&A answers.
5. **Track all state changes** - Every ability execution must verify all affected game state variables.

## Critical Variable Tracking

Every test MUST track and verify the following variables when abilities are executed:

### Card Position Tracking
When a card moves between zones, track:
- **Source zone** - Where the card came from (hand, stage, discard, deck, resolution zone, etc.)
- **Destination zone** - Where the card moved to
- **Specific area** - For stage: center, left side, right side
- **Card state** - Face up/face down, active/wait orientation
- **Energy underneath** - If moving from stage, what happens to energy under the card?
- **Turn played** - Track when card was played for turn-based restrictions

**Example verification:**
```rust
// Before: card in hand at index 2
// After: card should be in stage center, face up, active
assert_eq!(player1.hand.cards.len(), initial_hand_count - 1);
assert!(player1.stage.center.is_some());
assert_eq!(player1.stage.center.as_ref().unwrap().card.card_no, card_no);
assert_eq!(player1.stage.center.as_ref().unwrap().orientation, Some(Orientation::Active));
```

### Energy Tracking
When energy is manipulated, track:
- **Energy zone count** - Total energy in energy placement area
- **Energy under cards** - Energy stacked under member cards on stage
- **Energy states** - Active vs wait state energy
- **Energy payment** - How much energy was paid as cost
- **Energy to deck** - When members leave stage, energy moves to energy deck (rule 4.5.5.4)

**Example verification:**
```rust
// Track energy payment
let initial_energy_count = player1.energy_zone.cards.len();
// ... execute ability ...
assert_eq!(player1.energy_zone.cards.len(), initial_energy_count - cost_paid);
// Check energy states
let active_energy = player1.energy_zone.cards.iter()
    .filter(|e| e.orientation == Some(Orientation::Active)).count();
```

### Heart Tracking
When hearts are gained/spent/modified, track:
- **Heart icons on members** - Each member's base heart configuration
- **Heart icons from live cards** - Hearts gained via blade hearts from live success
- **Heart colors** - Specific colors (heart01-heart06) vs ALL heart
- **Heart variety** - Different heart colors present for variety conditions
- **Heart consumption** - Hearts spent to meet live card requirements
- **Heart modification** - Temporary heart gain/loss effects

**Example verification:**
```rust
// Track heart changes
let initial_hearts = count_total_hearts(&player1.stage);
// ... execute heart-gaining ability ...
let final_hearts = count_total_hearts(&player1.stage);
assert_eq!(final_hearts, initial_hearts + expected_gain);
// Verify specific colors if relevant
assert!(has_heart_color(&player1.stage, HeartColor::Heart01));
```

### Score Tracking
When scores are modified, track:
- **Live card scores** - Base scores on live cards in live card placement area
- **Score modifiers** - Temporary +1/-1 score effects
- **Total score calculation** - Sum of live card scores plus modifiers
- **Score comparison** - Between players for live victory determination
- **Score icons** - Blade heart icons that add to total score (rule 8.4.2.1)

**Example verification:**
```rust
let initial_total_score = calculate_total_score(&player1.live_card_area);
// ... execute score-modifying ability ...
let final_total_score = calculate_total_score(&player1.live_card_area);
assert_eq!(final_total_score, initial_total_score + expected_modifier);
```

### Blade Tracking
When blades are manipulated, track:
- **Blade count per member** - Each member's blade number
- **Total blade count** - Sum across all active members for cheer calculation
- **Blade modification** - Temporary blade gain/loss effects
- **Blade heart icons** - Icons revealed during cheer that grant hearts or score

**Example verification:**
```rust
let initial_blades = count_total_blades(&player1.stage);
// ... execute blade-gaining ability ...
let final_blades = count_total_blades(&player1.stage);
assert_eq!(final_blades, initial_blades + expected_gain);
```

### Turn and Phase Tracking
When turn/phase matters, track:
- **Current turn number** - For turn-limited abilities
- **Turn phase** - Active, Energy, Draw, Main, Live phases
- **First turn flag** - Special rules for first turn
- **Active player** - Who is currently taking actions
- **Abilities played this turn** - For turn-limited ability restrictions

**Example verification:**
```rust
assert_eq!(game_state.turn_number, expected_turn);
assert_eq!(game_state.current_phase, Phase::Main);
assert!(game_state.turn_limited_abilities_used.contains(&ability_id));
```

### Deck and Hand Tracking
When decks/hands are manipulated, track:
- **Main deck count** - Cards in main deck placement area
- **Hand count** - Cards in hand
- **Discard count** - Cards in discard area
- **Energy deck count** - Cards in energy deck
- **Successful live card count** - Cards in successful live card placement area (victory condition)
- **Deck order** - When cards are placed on top/bottom of deck

**Example verification:**
```rust
let initial_deck_count = player1.main_deck.cards.len();
let initial_hand_count = player1.hand.cards.len();
// ... execute draw ability ...
assert_eq!(player1.main_deck.cards.len(), initial_deck_count - cards_drawn);
assert_eq!(player1.hand.cards.len(), initial_hand_count + cards_drawn);
```

### Multi-Member Card Counting
When multi-member cards (e.g., "LL-bp1-001-R+ 上原歩夢&澁谷かのん&日野下花帆") are involved, track:
- **Member count** - Multi-member cards count as 1 member for counting (Q225, Q207, Q210)
- **Character references** - Can be referenced as any of the named characters (Q208)
- **Group references** - Referenced by group name of any character on the card

**Example verification:**
```rust
// Multi-member card counts as 1 member
let member_count = count_members_on_stage(&player1.stage);
assert_eq!(member_count, 1); // Even if card has 3 characters
// Can be referenced as any character
assert!(has_character_on_stage(&player1.stage, "上原歩夢"));
```

### Baton Touch Tracking
When baton touch is used, track:
- **Cost reduction** - How much cost was reduced by baton touch
- **Cards touched** - Which members were sent to discard
- **Restrictions** - Cannot baton touch with cards played this turn (Q194)
- **Area selection** - Can choose any area the touched members occupied (Q193)
- **Ability triggering** - Baton touch triggers "登場" (appearance) abilities

**Example verification:**
```rust
let original_cost = card.cost;
// ... execute baton touch ...
assert_eq!(actual_cost_paid, original_cost - touched_card.cost);
assert!(!touched_card.turn_played == current_turn); // Must be from previous turn
```

## Test Structure

Each test should:
1. Load the relevant card(s) from `cards.json`
2. Set up the game state as described in the Q&A scenario
3. Record initial state of all relevant variables
4. Execute the actions a player would take in the webapp
5. Record final state of all relevant variables
6. Verify the engine's behavior matches the official answer
7. Use helper functions to avoid code duplication

## Test Categories

Based on Q&A patterns:
- **Cost payment scenarios** - Can costs be paid? What happens if not? (Q234)
- **Ability triggering** - When do auto abilities trigger? Do they trigger multiple times? (Q233, Q227)
- **Card counting** - How are multi-member cards counted? What about energy under cards? (Q225, Q207, Q210, Q184)
- **Batontouch mechanics** - Cost calculations, ability interactions, timing (Q194, Q193, Q206)
- **Heart/score mechanics** - Heart gain, score modification, live card effects (Q231, Q230, Q232)
- **Position/state changes** - Position changes, wait states, activation states (Q223, Q220)
- **Deck/hand manipulation** - Drawing, searching, adding to hand, deck placement (Q226, Q229)
- **Live mechanics** - Live success, live cards, cheer, yell (Q213, Q212, Q211)
- **Energy mechanics** - Energy payment, energy under cards, energy states (Q215)
- **Multi-character references** - How multi-member cards are referenced (Q235, Q208)

## Setup Helpers

Common setup patterns:
- `setup_game_with_cards(card_numbers)` - Load cards and create game state
- `place_card_on_stage(card, area, orientation)` - Place a card on stage with specific state
- `add_cards_to_hand(card_numbers)` - Add cards to player's hand
- `add_energy_to_card(card, count, state)` - Add energy under a card with specific state
- `set_card_face_state(card, face_state)` - Set card face up/face down
- `set_card_orientation(card, orientation)` - Set card active/wait
- `execute_action(action_type, parameters)` - Execute a webapp-style action
- `record_initial_state()` - Snapshot all relevant variables before action
- `verify_state_changes(expected_changes)` - Compare post-action state against expected

## Execution Pattern

Tests should follow the webapp execution flow:
1. Phase setup (Active, Energy, Draw, Main, etc.)
2. Record initial state of all tracked variables
3. Action selection (play card, use ability, etc.)
4. Cost payment (if applicable)
5. Ability resolution
6. Record final state of all tracked variables
7. State verification against expected changes

## Example Test Pattern

```rust
#[test]
fn test_q237_card_name_reference() {
    // Q237: Can you add "Dream Believers (104期Ver.)" to hand after revealing "Dream Believers"?
    // Answer: No, you cannot.
    
    // 1. Load cards
    let cards = load_cards();
    let hana_card = find_card(&cards, "PL!HS-bp5-001-R＋"); // 日野下花帆
    let dream_believers = find_card(&cards, "PL!HS-bp1-019-L"); // Dream Believers
    let dream_believers_104 = find_card(&cards, "PL!HS-sd1-018-SD"); // Dream Believers (104期Ver.)
    
    // 2. Setup game state
    let (mut player1, mut player2) = create_test_players();
    
    // Place 日野下花帆 on stage
    place_card_on_stage(&mut player1, hana_card, MemberArea::Center);
    
    // Add Dream Believers to discard
    player1.discard.cards.push(dream_believers.clone());
    
    // Add Dream Believers (104期Ver.) to discard
    player1.discard.cards.push(dream_believers_104.clone());
    
    let mut game_state = create_test_game_state(player1, player2);
    
    // 3. Record initial state
    let initial_hand_count = game_state.player1.hand.cards.len();
    let initial_discard_count = game_state.player1.discard.cards.len();
    
    // 4. Execute ability: reveal Dream Believers, try to add Dream Believers (104期Ver.) to hand
    let ability = get_ability_by_text(&hana_card.abilities, "起動能力で「Dream Believers」を公開しました");
    let result = execute_ability(&mut game_state, &ability, Some("Dream Believers (104期Ver.)"));
    
    // 5. Verify against Q&A answer
    assert!(result.is_err() || !hand_contains(&game_state.player1.hand, "PL!HS-sd1-018-SD"),
        "Should NOT be able to add Dream Believers (104期Ver.) to hand");
    
    // 6. Verify state changes
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count,
        "Hand count should not change");
    assert_eq!(game_state.player1.discard.cards.len(), initial_discard_count,
        "Discard count should not change");
}
```

## Reading Reference Materials

Before writing a test, consult:
1. **rules.txt** - Comprehensive rule document covering all game mechanics
2. **qa_data.json** - Official Q&A with expected answers
3. **Card ability text** - The actual ability text on the card being tested
4. **Related cards** - Other cards mentioned in the Q&A that may be on stage/in hand

## Starting Point

Begin with Q1, Q2, etc. (lowest numbers) and work upward. Prioritize:
- Simple mechanical tests first (cost payment, basic card movement)
- Complex interaction tests later (multi-card combos, timing issues)
- Edge cases last (empty zones, maximum limits, unusual states)
