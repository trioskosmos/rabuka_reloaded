# QA Test Framework and Ideology

## Purpose
The primary goal of QA-based tests is to stress-test the game engine by simulating real gameplay scenarios from official Q&A data. These tests should:

1. **Use real card abilities** - No synthetic/mock abilities. Load actual cards from `cards.json`.
2. **Play the game realistically** - Simulate the webapp gameplay flow: set up game state, choose actions, resolve abilities.
3. **Make the engine fail** - Edge cases, complex interactions, and unusual scenarios that might reveal bugs.
4. **Verify expected behavior** - Compare engine output against official Q&A answers.
5. **Track all state changes** - Every ability execution must verify all affected game state variables.

## Test Failures = Engine Bugs

**If a test fails, the engine is broken.** Do not ignore tests or mark them as `#[ignore]`. A failing test indicates:

- The engine doesn't implement a required rule/mechanic
- The engine has a bug in ability resolution
- The engine's state management is incorrect
- The engine's validation logic is missing

**Never ignore failing tests.** Each failure is a concrete bug that must be fixed in the engine code, not the test code. The Q&A answers are the source of truth; the engine must conform to them.

## Gameplay Testing vs Superficial Flag Testing

### What NOT to Do: Superficial Flag Testing

Superficial tests only check internal flags or intermediate states without validating the actual gameplay outcome. These tests give a false sense of security.

**Bad Example - Flag Checking Only:**
```rust
#[test]
fn test_bad_appearance_ability() {
    // BAD: Only checks if a flag was set, not if the ability actually worked
    let mut game_state = setup_game();
    play_card_to_stage(&mut game_state, card_id);
    
    // This only checks a flag - doesn't verify cards were actually drawn
    assert!(game_state.appearance_triggered);
    assert!(game_state.ability_executed);
}
```

**Problems with this approach:**
- Doesn't verify the actual effect (drawing cards, modifying scores, etc.)
- Flags can be set incorrectly while the effect fails
- No validation of game state changes
- Tests pass even when gameplay is broken

### What TO Do: Gameplay Testing

Gameplay tests simulate the actual player experience and verify the concrete results of abilities.

**Good Example - Gameplay Validation:**
```rust
#[test]
fn test_good_appearance_ability() {
    // GOOD: Verifies actual gameplay outcome
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find the specific card with the appearance ability
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!-bp3-004-R＋")
        .expect("Should have the card");
    let member_id = get_card_id(member_card, &card_database);
    
    // Set up deck with cards to draw
    let deck_cards: Vec<_> = cards.iter()
        .filter(|c| !c.is_member() && !c.is_live())
        .take(5)
        .collect();
    for deck_card in deck_cards.iter() {
        player1.main_deck.cards.push(get_card_id(deck_card, &card_database));
    }
    
    setup_player_with_hand(&mut player1, vec![member_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::Main;
    
    // Record initial state
    let initial_hand_size = game_state.player1.hand.cards.len();
    let initial_deck_size = game_state.player1.main_deck.cards.len();
    
    // Execute the gameplay action
    TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    ).expect("Should play card to stage");
    
    // Verify the ACTUAL gameplay outcome
    let hand_size_after = game_state.player1.hand.cards.len();
    let deck_size_after = game_state.player1.main_deck.cards.len();
    
    // Concrete assertion: 1 card was drawn from deck to hand
    assert_eq!(hand_size_after, initial_hand_size + 1, 
        "Should have drawn 1 card after appearance");
    assert_eq!(deck_size_after, initial_deck_size - 1,
        "Deck should have 1 fewer card after drawing");
}
```

**Benefits of this approach:**
- Validates the actual gameplay effect (cards moved from deck to hand)
- Tests the complete execution chain (trigger → resolve → effect)
- Verifies state changes in concrete, observable ways
- Tests fail when gameplay is actually broken

### Key Differences

| Aspect | Superficial Testing | Gameplay Testing |
|--------|-------------------|------------------|
| **What it checks** | Internal flags, intermediate states | Final game state, observable outcomes |
| **Example assertion** | `assert!(ability_triggered)` | `assert_eq!(hand_size, expected)` |
| **False positive risk** | High - flags can be set incorrectly | Low - concrete state must match |
| **Debugging value** | Low - doesn't show what's broken | High - shows exactly what's wrong |
| **Maintenance** | Easy but misleading | Harder but meaningful |

### When to Use Each Approach

**Use Gameplay Testing for:**
- Ability effects (draw, discard, gain blades/hearts, modify scores)
- Card movement between zones
- Cost payment and resource consumption
- Turn and phase transitions
- Win/loss conditions

**Use Flag/State Testing ONLY for:**
- Timing validation (when abilities should trigger)
- Restriction checking (what actions are allowed)
- Turn tracking (what happened this turn)
- These should still be validated with concrete gameplay outcomes when possible

### Concrete Gameplay Assertions

Always prefer assertions that validate observable game state:

**✅ Good - Concrete assertions:**
```rust
// Card counts
assert_eq!(player.hand.cards.len(), 5);
assert_eq!(player.stage.stage.iter().filter(|&&id| id != -1).count(), 3);

// Resource amounts
assert_eq!(player.energy_zone.cards.len(), 10);
assert_eq!(calculate_total_blades(&player.stage, &card_db), 5);

// Score/heart values
assert_eq!(calculate_total_score(&player.live_card_area), 12);
assert_eq!(player.stage.all_heart_icons(&card_db).len(), 8);

// Card locations
assert!(player.stage.stage[1] != -1); // Card in center
assert!(player.hand.cards.contains(&card_id));
```

**❌ Bad - Abstract assertions:**
```rust
// Flag checks without concrete validation
assert!(game_state.ability_triggered);
assert!(game_state.cost_paid);
assert!(game_state.phase_changed);

// Intermediate state without final outcome
assert_eq!(game_state.pending_abilities.len(), 1);
assert!(game_state.effect_queue.is_empty());
```

### Testing Complex Interactions

For complex multi-step abilities, verify each step's concrete outcome:

```rust
#[test]
fn test_sequential_ability() {
    // Ability: "Draw 1 card, then discard 1 card"
    
    let initial_hand = game_state.player1.hand.cards.len();
    let initial_deck = game_state.player1.main_deck.cards.len();
    let initial_discard = game_state.player1.waitroom.cards.len();
    
    execute_ability(&mut game_state, ability);
    
    // Step 1: Verify draw
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand + 1,
        "Should have drawn 1 card");
    assert_eq!(game_state.player1.main_deck.cards.len(), initial_deck - 1,
        "Deck should have 1 fewer card");
    
    // Step 2: Verify discard
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand,
        "Should have discarded 1 card (back to original hand size)");
    assert_eq!(game_state.player1.waitroom.cards.len(), initial_discard + 1,
        "Discard should have 1 more card");
}
```

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
assert!(player1.stage.stage[1] != -1); // center is index 1
let card_id = player1.stage.stage[1];
assert_eq!(card_db.get_card(card_id).unwrap().card_no, card_no);
// Orientation is now tracked in GameState modifiers
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
// Energy cards are now i16, orientation tracked in GameState modifiers
// For now, assume all energy cards are active
let active_energy = player1.energy_zone.cards.len();
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
let card_db = &game_state.card_database;
let initial_hearts = player1.stage.all_heart_icons(card_db).len();
// ... execute heart-gaining ability ...
let final_hearts = player1.stage.all_heart_icons(card_db).len();
assert_eq!(final_hearts, initial_hearts + expected_gain);
// Verify specific colors if relevant
let hearts = player1.stage.all_heart_icons(card_db);
assert!(hearts.contains(&HeartColor::Heart01));
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
let card_db = &game_state.card_database;
let initial_blades = player1.stage.total_blades(card_db);
// ... execute blade-gaining ability ...
let final_blades = player1.stage.total_blades(card_db);
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
// Note: hand.cards is now SmallVec<[i16; 10]>, use .to_vec() if Vec needed
```

### Multi-Member Card Counting
When multi-member cards (e.g., "LL-bp1-001-R+ 上原歩夢&澁谷かのん&日野下花帆") are involved, track:
- **Member count** - Multi-member cards count as 1 member for counting (Q225, Q207, Q210)
- **Character references** - Can be referenced as any of the named characters (Q208)
- **Group references** - Referenced by group name of any character on the card

**Example verification:**
```rust
// Multi-member card counts as 1 member
let member_count = player1.stage.stage.iter().filter(|&&id| id != -1).count();
assert_eq!(member_count, 1); // Even if card has 3 characters
// Can be referenced as any character - use CardDatabase to check card data
let card_db = &game_state.card_database;
let has_character = player1.stage.stage.iter()
    .filter(|&&id| id != -1)
    .any(|&id| {
        if let Some(card) = card_db.get_card(id) {
            card.name.contains("上原歩夢")
        } else {
            false
        }
    });
assert!(has_character);
```

### Baton Touch Tracking
When baton touch is used, track:
- **Cost reduction** - How much cost was reduced by baton touch
- **Cards touched** - Which members were sent to discard
- **Restrictions** - Cannot baton touch with cards played this turn (Q194)
- **Area selection** - Can choose any area the touched members occupied (Q193)
- **Ability triggering** - Baton touch triggers "登場" (appearance) abilities

**Critical Testing Requirements for Baton Touch:**

Baton touch tests MUST follow these conditions to properly simulate gameplay:

1. **Two-Turn Requirement**: Baton touch must be done across two turns. You cannot play multiple cards in the same zone in the same turn normally. The area lock prevents this.

2. **Area Lock Mechanics**: When a card is played to a stage area, that area is locked for the rest of the turn (`areas_locked_this_turn`). This prevents playing another card in the same area in the same turn.

3. **Proper Test Setup**:
   - Turn 1: Play first member card to stage (e.g., Center)
   - Advance to turn 2: `game_state.turn_number = 2`
   - Clear area locks: `game_state.player1.areas_locked_this_turn.clear()`
   - Turn 2: Baton touch with second card to the SAME area (e.g., Center)

4. **Same Area Requirement**: Baton touch only works when replacing a card in the SAME stage area. Playing to a different area (e.g., Center then RightSide) will not trigger baton touch replacement.

**Example verification:**
```rust
// Turn 1: Play first card to stage
TurnEngine::execute_main_phase_action(
    &mut game_state,
    &ActionType::PlayMemberToStage,
    Some(first_card_id),
    None,
    Some(MemberArea::Center),
    Some(false), // not baton touch
).expect("Should play card to stage");

// Advance to turn 2 for baton touch
game_state.turn_number = 2;
// Clear locked areas to simulate end of turn logic
game_state.player1.areas_locked_this_turn.clear();

// Turn 2: Baton touch with second card to SAME area
TurnEngine::execute_main_phase_action(
    &mut game_state,
    &ActionType::PlayMemberToStage,
    Some(second_card_id),
    None,
    Some(MemberArea::Center), // SAME area to trigger replacement
    Some(true), // use baton touch
).expect("Should baton touch");

let original_cost = card.cost;
// ... verify cost reduction ...
assert_eq!(actual_cost_paid, original_cost - touched_card.cost);
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
- `setup_game_with_cards(card_numbers, card_database)` - Load cards and create game state with CardDatabase
- `place_card_on_stage(player, card_id, area_index)` - Place a card on stage using array index (0=left, 1=center, 2=right)
- `add_cards_to_hand(player, card_ids)` - Add card IDs to player's hand (SmallVec)
- `add_energy_to_zone(player, card_ids)` - Add energy cards to energy zone (energy cards are now i16)
- `execute_action(game_state, action_type, parameters)` - Execute a webapp-style action
- `record_initial_state(game_state)` - Snapshot all relevant variables before action
- `verify_state_changes(game_state, expected_changes)` - Compare post-action state against expected
- **Note**: Orientation and energy_underneath tracking moved to GameState modifiers

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
    let card_database = Arc::new(CardDatabase::load_or_create(cards.clone()));
    let hana_card_id = find_card_id(&cards, "PL!HS-bp5-001-R＋"); // 日野下花帆
    let dream_believers_id = find_card_id(&cards, "PL!HS-bp1-019-L"); // Dream Believers
    let dream_believers_104_id = find_card_id(&cards, "PL!HS-sd1-018-SD"); // Dream Believers (104期Ver.)

    // 2. Setup game state
    let (mut player1, mut player2) = create_test_players();
    let mut game_state = GameState::new(player1, player2, card_database.clone());

    // Place 日野下花帆 on stage (center is index 1)
    game_state.player1.stage.stage[1] = hana_card_id;

    // Add Dream Believers to discard (discard cards are now i16)
    game_state.player1.waitroom.cards.push(dream_believers_id);

    // Add Dream Believers (104期Ver.) to discard
    game_state.player1.waitroom.cards.push(dream_believers_104_id);

    // 3. Record initial state
    let initial_hand_count = game_state.player1.hand.cards.len();
    let initial_discard_count = game_state.player1.waitroom.cards.len();

    // 4. Execute ability: reveal Dream Believers, try to add Dream Believers (104期Ver.) to hand
    let hana_card = card_database.get_card(hana_card_id).unwrap();
    let ability = get_ability_by_text(&hana_card.abilities, "起動能力で「Dream Believers」を公開しました");
    let result = execute_ability(&mut game_state, &ability, Some("Dream Believers (104期Ver.)"));

    // 5. Verify against Q&A answer
    assert!(result.is_err() || !hand_contains(&game_state.player1.hand, dream_believers_104_id, &card_database),
        "Should NOT be able to add Dream Believers (104期Ver.) to hand");

    // 6. Verify state changes
    assert_eq!(game_state.player1.hand.cards.len(), initial_hand_count,
        "Hand count should not change");
    assert_eq!(game_state.player1.waitroom.cards.len(), initial_discard_count,
        "Discard count should not change");
}
```

## Reading Reference Materials

Before writing a test, consult:
1. **rules.txt** - Comprehensive rule document covering all game mechanics
2. **qa_data.json** - Official Q&A with expected answers
3. **Card ability text** - The actual ability text on the card being tested
4. **Related cards** - Other cards mentioned in the Q&A that may be on stage/in hand

## Debugging Failed Tests

When a test fails, consider the following in order:
1. **Engine implementation** - The most likely issue is the engine not correctly implementing the game mechanic
2. **Ability parsing** - If the ability text is complex, parser.py in cards/ability_extraction/ may need to be updated to correctly parse the ability
3. **Card data** - Verify the card data in cards.json is correct
4. **Test setup** - Ensure the test correctly sets up the game state as described in the Q&A

**Note:** If the ability text is not correctly parsed by parser.py, it may need editing. However, the engine implementation is the most likely source of issues and should be investigated first.

## Starting Point

Begin with Q1, Q2, etc. (lowest numbers) and work upward. Prioritize:
- Simple mechanical tests first (cost payment, basic card movement)
- Complex interaction tests later (multi-card combos, timing issues)
- Edge cases last (empty zones, maximum limits, unusual states)

## Test Organization and Naming

### File Structure
Organize tests by category to maintain clarity:
```
tests/
├── qa/
│   ├── cost_payment.rs
│   ├── ability_triggering.rs
│   ├── card_counting.rs
│   ├── baton_touch.rs
│   ├── heart_score.rs
│   ├── deck_hand.rs
│   └── live_mechanics.rs
└── integration/
    └── full_game_scenarios.rs
```

### Test Naming Convention
Use descriptive names that indicate:
- The Q&A number being tested
- The mechanic being validated
- The expected outcome

**Good examples:**
```rust
fn test_q234_cost_payment_with_insufficient_energy()
fn test_q225_multi_member_card_counting()
fn test_q193_baton_touch_area_selection()
fn test_appearance_ability_draws_one_card()
```

**Bad examples:**
```rust
fn test_cost()
fn test_card_count()
fn test_baton()
fn test_ability()
```

## Test Data Management

### Card Loading Strategy
Load cards once per test module to avoid redundant I/O:
```rust
lazy_static! {
    static ref CARDS: Vec<Card> = load_cards();
    static ref CARD_DATABASE: Arc<CardDatabase> = Arc::new(CardDatabase::load_or_create(CARDS.clone()));
}
```

### Test-Specific Card Selection
Create helper functions to find specific cards:
```rust
fn find_card_by_number(card_no: &str) -> i16 {
    CARDS.iter()
        .find(|c| c.card_no == card_no)
        .map(|c| get_card_id(c, &CARD_DATABASE))
        .expect(&format!("Card {} not found", card_no))
}

fn find_cards_by_character(character_name: &str) -> Vec<i16> {
    CARDS.iter()
        .filter(|c| c.name.contains(character_name))
        .map(|c| get_card_id(c, &CARD_DATABASE))
        .collect()
}
```

### State Snapshot Helpers
Create reusable snapshot functions:
```rust
fn snapshot_game_state(game_state: &GameState) -> GameStateSnapshot {
    GameStateSnapshot {
        hand_size: game_state.player1.hand.cards.len(),
        deck_size: game_state.player1.main_deck.cards.len(),
        discard_size: game_state.player1.waitroom.cards.len(),
        energy_count: game_state.player1.energy_zone.cards.len(),
        stage_members: game_state.player1.stage.stage.iter()
            .filter(|&&id| id != -1)
            .count(),
        total_blades: game_state.player1.stage.total_blades(&game_state.card_database),
        total_hearts: game_state.player1.stage.all_heart_icons(&game_state.card_database).len(),
        total_score: calculate_total_score(&game_state.player1.live_card_area),
        current_phase: game_state.current_phase.clone(),
        turn_number: game_state.turn_number,
    }
}
```

## Regression Testing

### When to Add Regression Tests
Add regression tests when:
1. A bug is found and fixed
2. A Q&A reveals an edge case not previously covered
3. Complex interactions are discovered during gameplay
4. Engine behavior changes due to refactoring

### Regression Test Template
```rust
#[test]
fn test_regression_[issue_number]_[brief_description]() {
    // Issue: [description of the bug]
    // Fix: [description of the fix]
    // Q&A: [relevant Q&A number if applicable]

    let cards = load_cards();
    let card_database = Arc::new(CardDatabase::load_or_create(cards.clone()));

    // Setup the exact scenario that triggered the bug
    let (mut player1, mut player2) = create_test_players();
    let mut game_state = GameState::new(player1, player2, card_database.clone());

    // ... specific setup ...

    // Execute the action that previously failed
    let result = execute_action(&mut game_state, action);

    // Verify the fix works
    assert!(result.is_ok(), "Action should succeed after fix");
    // Verify concrete gameplay outcome
    assert_eq!(game_state.player1.hand.cards.len(), expected_hand_size);
}
```

## Test Maintenance

### Updating Tests When Abilities Change
When ability parsing or implementation changes:
1. Run the full test suite to identify failures
2. Check if the test expectation was wrong or the implementation changed
3. Update test expectations only if the Q&A confirms the new behavior
4. Document the reason for the change in comments

### Deprecating Tests
When a test is no longer relevant:
1. Add a comment explaining why it's deprecated
2. Mark with `#[ignore]` instead of deleting
3. Consider if the test can be repurposed for a different scenario

```rust
#[test]
#[ignore = "Q123 was superseded by Q456 - see test_q456 instead"]
fn test_q123_old_behavior() {
    // This test is kept for historical reference
}
```

## Integration with CI/CD

### Test Execution in CI
Configure CI to:
1. Run all QA tests on every commit
2. Run tests with `--release` for performance
3. Generate test reports for failed tests
4. Mark builds as failed if any QA test fails

### Performance Considerations
- Use `cargo test --release` for faster test execution
- Parallelize independent tests using `cargo test --test-threads=4`
- Cache card loading between tests using lazy_static
- Avoid expensive operations in test setup

## Common Pitfalls

### Pitfall 1: Testing Implementation Details
**Bad:** Testing internal function calls
```rust
assert!(game_state.ability_resolver.called);
```

**Good:** Testing observable outcomes
```rust
assert_eq!(game_state.player1.hand.cards.len(), expected);
```

### Pitfall 2: Fragile Test Setup
**Bad:** Hard-coded indices that break easily
```rust
game_state.player1.hand.cards[0] = card_id; // Assumes hand is empty
```

**Good:** Using helper functions that handle state
```rust
add_card_to_hand(&mut game_state.player1, card_id);
```

### Pitfall 3: Not Cleaning Up State
**Bad:** Tests that leave state dirty
```rust
// Test modifies game_state but doesn't reset
```

**Good:** Each test creates fresh state
```rust
let mut game_state = create_fresh_game_state();
```

### Pitfall 4: Overly Specific Assertions
**Bad:** Asserting exact card order when order doesn't matter
```rust
assert_eq!(game_state.player1.hand.cards[0], card_id);
```

**Good:** Asserting presence when order is irrelevant
```rust
assert!(game_state.player1.hand.cards.contains(&card_id));
```

## Advanced Testing Patterns

### Property-Based Testing
For rules that should hold across many scenarios:
```rust
#[test]
fn test_property_draw_increases_hand() {
    // Property: Drawing N cards always increases hand by N
    for n in 1..=5 {
        let initial_hand = game_state.player1.hand.cards.len();
        draw_cards(&mut game_state, n);
        assert_eq!(game_state.player1.hand.cards.len(), initial_hand + n);
    }
}
```

### State Transition Testing
Verify that state transitions are valid:
```rust
#[test]
fn test_phase_transitions() {
    let mut game_state = setup_game();
    
    // Can only go from Main to Live, not backwards
    game_state.current_phase = Phase::Main;
    transition_phase(&mut game_state, Phase::Live);
    assert_eq!(game_state.current_phase, Phase::Live);
    
    // Should not be able to transition backwards
    let result = transition_phase(&mut game_state, Phase::Main);
    assert!(result.is_err());
}
```

### Boundary Testing
Test edge cases and limits:
```rust
#[test]
fn test_maximum_hand_size() {
    // Test drawing when hand is at maximum
    fill_hand_to_max(&mut game_state);
    let result = draw_card(&mut game_state);
    assert!(result.is_err() || game_state.player1.waitroom.cards.len() > 0);
}
```

## Documentation and Comments

### Test Documentation
Each test should have:
1. A brief description of what it tests
2. Reference to relevant Q&A number
3. Expected behavior
4. Any special setup requirements

```rust
/// Tests Q193: Baton touch area selection
/// 
/// When using baton touch, you can choose any area that the touched member
/// occupied. This test verifies that the new member is placed in the same
/// area as the touched member.
#[test]
fn test_q193_baton_touch_area_selection() {
    // ...
}
```

### Complex Test Logic Comments
For complex test setup, add inline comments:
```rust
// Setup: Player has 3 energy cards in energy zone
// This is required to pay the cost of the member card
let energy_cards = vec![card_id_1, card_id_2, card_id_3];
game_state.player1.energy_zone.cards.extend(energy_cards);

// The member card costs 2 energy, so we expect 1 energy remaining
let expected_remaining_energy = 1;
```

## Continuous Improvement

### Reviewing Test Coverage
Regularly review:
1. Which Q&A questions are covered
2. Which game mechanics have tests
3. Areas with sparse test coverage
4. Tests that are frequently flaky

### Test Metrics to Track
- Number of Q&A questions covered
- Test execution time
- Test failure rate
- Number of regression tests added

### Updating the Framework
This document should be updated when:
1. New testing patterns are discovered
2. Common pitfalls are identified
3. Best practices evolve
4. New game mechanics are added
