# Ability Field Requirements and Defaults

This document describes the required fields and default values for each ability action type in the Rabuka game engine.

## AbilityEffect Required Fields by Action Type

### `move_cards`
**Purpose**: Move cards from one location to another
**Required Fields**:
- `action`: "move_cards"
- `source`: Source location (e.g., "deck", "hand", "stage", "discard", "waitroom")
- `destination`: Destination location (e.g., "hand", "stage", "discard", "waitroom")
- `count`: Number of cards to move (optional, defaults to 1)
- `card_type`: Type of card to move (e.g., "member_card", "live_card", "energy_card")
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "move_cards".to_string(),
    source: Some("deck".to_string()),
    destination: Some("hand".to_string()),
    count: Some(1),
    card_type: Some("member_card".to_string()),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `draw`
**Purpose**: Draw cards from deck to hand
**Required Fields**:
- `action`: "draw"
- `count`: Number of cards to draw (optional, defaults to 1)
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "draw".to_string(),
    count: Some(1),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `draw_until_count`
**Purpose**: Draw cards until hand reaches a specific count
**Required Fields**:
- `action`: "draw_until_count"
- `count`: Target hand count
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "draw_until_count".to_string(),
    count: Some(5),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `gain_resource`
**Purpose**: Add resources (blade, heart, energy) to cards
**Required Fields**:
- `action`: "gain_resource"
- `resource`: Resource type ("blade", "heart", "energy")
- `count`: Number of resources to add (optional, defaults to 1)
- `target`: Target player ("self" or "opponent", defaults to "self")
- `duration`: How long the resource lasts (optional, e.g., "live_end")

**Example**:
```rust
AbilityEffect {
    action: "gain_resource".to_string(),
    resource: Some("blade".to_string()),
    count: Some(2),
    target: Some("self".to_string()),
    duration: Some("live_end".to_string()),
    ..Default::default()
}
```

### `change_state`
**Purpose**: Change card state (active, wait, etc.)
**Required Fields**:
- `action`: "change_state"
- `state_change`: State to change to ("active", "wait")
- `count`: Number of cards to affect (optional, defaults to 1)
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "change_state".to_string(),
    state_change: Some("active".to_string()),
    count: Some(2),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `modify_score`
**Purpose**: Modify card scores
**Required Fields**:
- `action`: "modify_score"
- `operation`: Operation type ("add", "remove", "set")
- `value`: Score value to modify
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "modify_score".to_string(),
    operation: Some("add".to_string()),
    value: Some(1),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `pay_energy`
**Purpose**: Pay energy cost by deactivating energy cards
**Required Fields**:
- `action`: "pay_energy"
- `count`: Number of energy cards to deactivate (optional, defaults to 1)
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "pay_energy".to_string(),
    count: Some(1),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `discard_until_count`
**Purpose**: Discard cards from hand until hand reaches a specific count
**Required Fields**:
- `action`: "discard_until_count"
- `count`: Target hand count
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "discard_until_count".to_string(),
    count: Some(3),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `position_change`
**Purpose**: Swap card positions on stage
**Required Fields**:
- `action`: "position_change"
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "position_change".to_string(),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `reveal`
**Purpose**: Reveal hidden cards
**Required Fields**:
- `action`: "reveal"
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "reveal".to_string(),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `choice`
**Purpose**: Execute one of multiple choice options
**Required Fields**:
- `action`: "choice"
- `choice_options`: Vec<String> of choice text options
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
AbilityEffect {
    action: "choice".to_string(),
    choice_options: Some(vec!["Draw a card".to_string(), "Gain blade".to_string()]),
    target: Some("self".to_string()),
    ..Default::default()
}
```

### `sequential`
**Purpose**: Execute multiple effects in sequence
**Required Fields**:
- `action`: "sequential"
- `actions`: Vec<AbilityEffect> of effects to execute in order

**Example**:
```rust
AbilityEffect {
    action: "sequential".to_string(),
    actions: Some(vec![
        AbilityEffect { action: "draw".to_string(), count: Some(1), ..Default::default() },
        AbilityEffect { action: "gain_resource".to_string(), resource: Some("blade".to_string()), ..Default::default() },
    ]),
    ..Default::default()
}
```

### `conditional_alternative`
**Purpose**: Execute alternative effect if condition is met
**Required Fields**:
- `action`: "conditional_alternative"
- `condition`: Condition to check
- `alternative_condition`: Alternative condition
- `primary_effect`: Effect if primary condition met
- `alternative_effect`: Effect if alternative condition met

### `look_and_select`
**Purpose**: Look at cards and select one
**Required Fields**:
- `action`: "look_and_select"
- `look_action`: AbilityEffect for looking
- `select_action`: AbilityEffect for selecting
- `target`: Target player ("self" or "opponent", defaults to "self")

### `look_at`
**Purpose**: Look at cards without selecting
**Required Fields**:
- `action`: "look_at"
- `source`: Source location to look at
- `count`: Number of cards to look at (optional)
- `target`: Target player ("self" or "opponent", defaults to "self")

### `select`
**Purpose**: Select cards from a location
**Required Fields**:
- `action`: "select"
- `source`: Source location to select from
- `count`: Number of cards to select
- `target`: Target player ("self" or "opponent", defaults to "self")

## Condition Field Requirements

### `card_count_condition`
**Purpose**: Check if player has a specific number of cards of a type
**Required Fields**:
- `condition_type`: "card_count_condition"
- `card_type`: Type of card to count ("member_card", "live_card", "energy_card")
- `count`: Required count
- `operator`: Comparison operator (">=", ">", "<=", "<", "==", "!=")
- `target`: Target player ("self" or "opponent", defaults to "self")

**Example**:
```rust
Condition {
    text: "メンバーカードがある場合".to_string(),
    condition_type: Some("card_count_condition".to_string()),
    card_type: Some("member_card".to_string()),
    count: Some(1),
    operator: Some(">=".to_string()),
    target: Some("self".to_string()),
    // ... all other fields set to None
}
```

### `comparison_condition`
**Purpose**: Compare values
**Required Fields**:
- `condition_type`: "comparison_condition"
- `count`: Value to compare against
- `operator`: Comparison operator (">=", ">", "<=", "<", "==", "!=")

### `group_condition`
**Purpose**: Check if cards belong to a specific group
**Required Fields**:
- `condition_type`: "group_condition"
- `group`: Group information (e.g., group name)
- `card_type`: Type of card to check
- `count`: Required count
- `operator`: Comparison operator

### `location_condition`
**Purpose**: Check if card is in a specific location
**Required Fields**:
- `condition_type`: "location_condition"
- `location`: Location to check ("stage", "hand", "deck", etc.)
- `card_type`: Type of card to check (optional)

### `temporal_condition`
**Purpose**: Check temporal state (this turn, this live, etc.)
**Required Fields**:
- `condition_type`: "temporal_condition"
- `temporal`: Temporal scope ("this_turn", "this_live")
- `event`: Event type ("appearance", etc.)
- `location`: Location to check
- `card_type`: Type of card to check

## Common Default Values

### AbilityEffect Fields
- `text`: Empty string (for display purposes)
- `action`: Empty string (must be set for execution)
- `source`: None
- `destination`: None
- `count`: None (defaults to 1 for most actions)
- `card_type`: None
- `target`: None (defaults to "self" in most handlers)
- `duration`: None
- `condition`: None
- `resource`: None
- `operation`: None
- `value`: None
- `choice_options`: None
- `actions`: None

### Ability Fields
- `full_text`: Empty string
- `triggerless_text`: Empty string
- `triggers`: None
- `use_limit`: None
- `is_null`: false
- `cost`: None
- `effect`: None

### AbilityCost Fields
- `text`: Empty string
- `cost_type`: None
- `source`: None
- `destination`: None
- `count`: None
- `card_type`: None
- `target`: None
- `action`: None
- `optional`: None
- `energy`: None
- `state_change`: None
- `position`: None

## Important Notes

1. **Condition Checking**: Abilities with conditions will skip execution if the condition is not met. The resolver returns `Ok(())` without executing the effect when conditions fail.

2. **Target Default**: Most ability actions default to "self" if target is not specified.

3. **Count Default**: Most actions default to count=1 if not specified.

4. **Condition Types**: Unknown condition types will fail-open (return true) with a warning logged.

5. **Action Types**: Unknown action types will log a warning but return `Ok(())` to prevent crashes.

6. **Card Types**: Common card types include:
   - "member_card"
   - "live_card"
   - "energy_card"
   - "event_card"

7. **Locations**: Common locations include:
   - "deck"
   - "hand"
   - "stage"
   - "discard" / "waitroom"
   - "energy_zone"
   - "live_card_zone"
