# How to Write Engine Tests

The goal: every test should exercise real card behavior, not bypass it.

## Test Setup

```rust
use crate::helpers::*;

#[test]
fn my_card_ability_does_what_it_should() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Fill decks so draws don't panic
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler_id); }
    // Use abilityless filler cards unless the filler's own ability is being tested
    let filler = game.id("PL!-sd1-010-SD");
}
```

## Card Placement — Always use `play_to_stage` for debuts

Do NOT directly set `stage.stage[X] = card_id` or use `add_to_stage()` unless you're specifically testing engine infrastructure (modifier cleanup on card leave, etc.). For debut triggers, ALWAYS use `game.play_to_stage(card_id, MemberArea::Center)` — this properly fires the debut ability.

```rust
// GOOD — fires debut trigger
game.play_to_stage(card, MemberArea::Center);

// BAD — bypasses triggers (only use for manual setup)
// game.state.player1.stage.stage = [card, -1, -1];
```

## Ability Activation — Use `activate_ability`

For activation (起動) abilities, use `game.activate_ability(card_id)`. This properly checks use_limit, position restrictions, and processes the cost sequence.

```rust
game.activate_ability(card_id);
while game.has_pending_choice() {
    game.select_indices(&[0]);
}
```

## Choice Resolution

Resolve all pending choices, don't leave them dangling:

```rust
while game.has_pending_choice() {
    game.select_indices(&[0]);  // picks first option
    // or
    game.select_option(1);  // picks card_id=1 (e.g. for pay/skip cost)
}
```

## Filters — Verify Non-Matching Cards Are Excluded

When a card has group, cost_limit, or character filters, the choice should ONLY show matching cards. Test that non-matching cards are not selectable:

```rust
let matching = game.id("PL!SP-pb1-014-N");  // matches filter
let non_matching = game.id("PL!-sd1-010-SD");  // doesn't match

game.state.player1.hand.cards.push(matching);
game.state.player1.hand.cards.push(non_matching);

game.activate_ability(card_id);
// Choice should only show matching: non_matching should NOT be an option
let choice = game.state.ability_queue.is_waiting_for_choice().unwrap();
// Verify non-matching cards are filtered out
```

## `target_player_id` on Choices

Every `Choice::select_cards` must include `.target_player_id()` so the correct player's
zone is displayed. If this is missing, Player 2 abilities will show Player 1's hand/discard/etc.

## Area Selection

When a card says "select an area different from current", the engine creates an
`area_select` choice. The selected area is then used by the subsequent `position_change`:

```rust
// After play_to_stage that triggers an area select choice
game.select_option(0);  // picks first available position
```

## Center Position Requirement

Cards with `{{center.png|センター}}` in the cost text require the member to be at
Center to activate. The engine checks this in `handle_use_ability`. Test both cases:

```rust
// SUCCESS: at Center
game.state.player1.stage.stage = [-1, card_id, -1];
game.activate_ability(card_id);  // succeeds

// FAILURE: at Left
game.state.player1.stage.stage = [card_id, -1, -1];
let err = game.try_activate_ability(card_id).unwrap_err();
assert!(err.contains("position"));
```

## Use Limit Testing

Activation abilities with `use_limit: 1` can only be used once per turn:

```rust
game.activate_ability(card_id);
while game.has_pending_choice() { game.select_indices(&[0]); }

let err = game.try_activate_ability(card_id).unwrap_err();
assert!(err.contains("use_limit") || err.contains("already used"));
```

## Reveal-Until Pattern

Cards like "reveal from deck until [card type] is found" use a sequential with
`reveal(deck_top, multiple_targets=true)` then `move_cards(looked_at → hand)` and
`move_cards(looked_at_remaining → discard)`:

```rust
// After activating the ability and resolving the initial choice,
// the ability will reveal until the matching card is found.
// The revealed card should be in hand, others in discard.
```

## Rotation (003-R pattern)

For `position_change` with `multiple_targets=true` and a `position` field, the
engine applies rotation: left→right, center→left, right→center:

```rust
// Before: [A, B, C] at [left, center, right]
// After:  [B, C, A]
```

## `same_area` Destination in position_change

When a second `position_change` has `destination: "same_area"`, the engine
skips it (the swap was already done by the first position_change). This is
a no-op.

## Mulligan Selection

The engine tracks mulligan selections in `game_state.mulligan_selected_indices`.
During MulliganP1Turn/P2Turn, the `select_mulligan` action toggles indices.
The display sends `mulligan_selection` to the UI; tests should verify the
bitmask matches selected indices.

## Phase Flow

After skipping all main phases (pass P1 Main → P2 Main → LiveCardSet → Performances →
LiveVictoryDet), the next turn starts. The e2e test (`e2e_basic_game_test.rs`) covers
the complete flow from RPS through turn 3.

## Common Gotchas

- **Decks must have filler cards** — `draw_card` panics on empty deck.
- **`same_area` needs `last_vacated_stage_area`** — only set by prior `self_cost`.
- **Active phase activates ALL energy** — both players get active energy refreshed.
- **LiveSuccess fires once per turn** — flagged by `live_success_triggered_this_turn`.
- **Full-width characters** — use `\u{ff0b}` for `＋` in Rust strings.
- **`target_player_id` on all `Choice::select_cards`** — required for cross-player ability resolution.
