# Test Assertions Guidelines

## Hard assertions: never use `if` guards to silently skip prompt handling

### ❌ Wrong
```rust
if game.has_pending_choice() {
    game.select_indices(&[0]);
}
if game.pending_choice_type() == Some("SelectPosition".to_string()) {
    game.select_option(1);
}
```
These silently tolerate wrong prompt types, missing prompts, and extra prompts.

### ✅ Correct
```rust
assert!(game.has_pending_choice(), "Expected pending choice for ...");
assert_eq!(game.pending_choice_type(), Some("SelectCard"), "Expected SelectCard for ...");
game.select_indices(&[0]);
```

## Always verify card locations after effects

After an effect that moves cards:
```rust
// Card is on stage (not in hand, not in discard)
assert!(player.stage.stage.contains(&card_id), "...");
assert!(!player.hand.cards.contains(&card_id), "...");
assert!(!player.waitroom.cards.contains(&card_id), "...");
```

## Always verify state changes (wait/active)

```rust
assert_eq!(
    game.state.mods.get_orientation_modifier(card_id),
    Some(&"wait".to_string()),
    "..."
);
```

## Verify both players in both-target effects

```rust
assert!(p1.stage.stage.contains(&p1_card), "P1 should get their card");
assert!(p2.stage.stage.contains(&p2_card), "P2 should get their card");
```

## Verify prompt chain exhaustively

Check every prompt in sequence with no silent skips:
```rust
// Prompt 1: P1 selects card from discard
assert_eq!(game.pending_choice_type(), Some("SelectCard"), "P1: discard select");
game.select_indices(&[0]);

// Prompt 2: P1 chooses position
assert_eq!(game.pending_choice_type(), Some("SelectPosition"), "P1: position");
game.select_option(1);

// Prompt 3: P2 selects card from discard
assert_eq!(game.pending_choice_type(), Some("SelectCard"), "P2: discard select");
game.select_indices(&[0]);

// Prompt 4: P2 chooses position
assert_eq!(game.pending_choice_type(), Some("SelectPosition"), "P2: position");
game.select_option(2);

// No more prompts
assert!(!game.has_pending_choice(), "No remaining prompts");
```

## Helper: check state at the end

```rust
fn assert_member_on_stage_in_wait(game: &TestGame, card_id: i16, player: &crate::player::Player) {
    assert!(player.stage.stage.contains(&card_id));
    assert_eq!(
        game.state.mods.get_orientation_modifier(card_id),
        Some(&"wait".to_string())
    );
}
```
