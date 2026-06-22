# Writing Gameplay Tests — Guide & Common Mistakes

## Philosophy

- **Real cards only.** Every test uses real card numbers from `cards/cards.json`.
- **Test what the Japanese text says.** The ability text is the spec, not the JSON fields.
- **Minimal injected functions.** Avoid adding test-only helper functions that mutate game state bypassing the normal action pipeline. Preference: `play_to_stage`, `activate_ability`, `pass`, `select_indices` — the same actions a real player takes.
- **One test per unique ability text pattern.** If the same text appears on 27 cards, test it once.

## TestGame Initial State

```rust
let db = load_real_database();
let mut game = TestGame::new(db);
```

`TestGame::new(db)` starts the game at:

| Field | Value |
|-------|-------|
| `current_phase` | `Phase::Main` |
| `current_turn_phase` | `TurnPhase::FirstAttackerNormal` |
| `turn_number` | 1 |
| `player1.is_first_attacker` | `true` |

RPS, mulligan, and initial draw (6 cards) are **skipped**. You must manually add cards to hand with `game.add_to_hand(id)`.

## What Each `pass()` Does

`game.pass()` calls `TurnEngine::execute_main_phase_action(Pass)` which calls `advance_phase()`. The phase transitions are deterministic. Learn them so you don't get surprised by cards being drawn from your carefully-prepared deck.

### Normal Turn: `FirstAttackerNormal` / `SecondAttackerNormal`

```
Active  →  Energy  →  Draw  →  Main  →  (cycle)
```

| Phase entered | Side effects |
|-----------|--------------|
| **Active→Energy** | Activates wait members, refreshes all energy to active |
| **Energy→Draw** | Draws 1 energy from energy deck |
| **Draw→Main** | **Draws 1 card from main deck** (`draw_card()` from index 0) |
| **Main→** | If `FirstAttackerNormal`: switches to `SecondAttackerNormal`, goes to Active. If `SecondAttackerNormal`: switches to `TurnPhase::Live`, goes to `LiveCardSetFirstAttacker` |

**Critical:** The Draw phase draws from **deck index 0** (the top). If you `insert(0, my_card)` to put a card on top and then `pass()` through the Draw phase, your card gets drawn.

### Live Turn Phase: `Live`

```
LiveCardSetFirstAttacker → LiveCardSetSecondAttacker → FirstAttackerPerformance → SecondAttackerPerformance → LiveVictoryDetermination
```

| Phase entered | Side effects |
|-----------|--------------|
| **LiveCardSetFirstAttacker** | Transitions to SecondAttacker (no draw) |
| **LiveCardSetSecondAttacker** | **Fires LiveStart abilities** for both players, processes auto-abilities |
| **FirstAttackerPerformance** | Resolves the live performance (cheer, scoring) |
| **SecondAttackerPerformance** | Same for second attacker |
| **LiveVictoryDetermination** | Compares scores, clears revealed cards, increments turn, resets to Active in `FirstAttackerNormal` |

### Special Initial Phases

RPS and mulligan phases are skipped by `TestGame::new`. The game starts at `Phase::Main` directly.

## Common Pattern: Advancing to LiveStart

The most common test pattern is setting up a board state and testing a LiveStart ability. Here's what the passes actually do:

```rust
// Starting state: Main phase, FirstAttackerNormal, turn 1
game.pass();  // Main → turn_phase=SecondAttackerNormal, phase=Active
game.pass();  // Active → Energy (energy refreshed)
game.pass();  // Energy → Draw (1 energy drawn)
game.pass();  // Draw → Main (1 CARD DRAWN FROM DECK — index 0 consumed!)
game.pass();  // Main → turn_phase=Live, phase=LiveCardSetFirstAttacker
// At this point: LiveCardSetFirstAttacker
game.set_live_card(live_card_id);
game.pass();  // LiveCardSetFirstAttacker → LiveCardSetSecondAttacker
// LiveCardSetSecondAttacker: fires LiveStart abilities, processes auto-abilities
```

Total: **1 card drawn** from the deck between setup and LiveStart firing.

If you use the helper functions:
```rust
advance_to_live_card_set_p1(&mut game);  // 5 passes → LiveCardSetFirstAttacker (1 draw)
game.set_live_card(live_card_id);
advance_to_live_start(&mut game);        // 2 passes → abilities fire (no draws)
```

## Common Mistakes

### 1. Deck-top card consumed by Draw phase — THE most common mistake

```rust
// ❌ WRONG: The card at index 0 gets drawn during advance_to_live_card_set_p1
game.state.player1.main_deck.cards.insert(0, my_test_card);
advance_to_live_card_set_p1(&mut game);  // 1 draw happens → my_test_card is in hand now!
// The ability fires and reveals the WRONG card (a filler)
```

**Fix:** Insert the test card at index 1, behind one filler card that will be drawn instead.

```rust
// ✅ CORRECT: One filler shields the test card
fill_decks(&mut game);
game.state.player1.main_deck.cards.insert(1, my_test_card);
// Deck: [filler_0, test_card, filler_1, filler_2, ...]
// Passes draw filler_0 → deck is now [test_card, filler_1, ...]
// Ability fires and reveals test_card ✓
```

If you need the card to NOT be drawn at all, account for each Draw phase. Every full turn cycle (Main→Active→Energy→Draw→Main→Live) draws exactly **1 card** per player.

For `advance_to_live_card_set_p1` + `advance_to_live_start` (7 passes), the answer is **1 draw**. So insert at index 1.

### 2. Insert vs push — deck orientation

```rust
// Index 0 = TOP of deck (drawn first)
game.state.player1.main_deck.cards.insert(0, card);   // puts on TOP
game.state.player1.main_deck.cards.push(card);         // puts on BOTTOM
```

`draw_card()` and `reveal deck_top` read from index 0.

### 3. Assuming `game.id()` vs `game.new_id()` return the same ID

```rust
let a = game.id("PL!-sd1-010-SD");    // pops from pre-created pool
let b = game.id("PL!-sd1-010-SD");    // different ID from same template
let c = game.new_id("PL!-sd1-010-SD"); // also different, from counter pool
```

- `id()` consumes from a pre-seeded pool (5 copies per template).
- `new_id()` falls back to a monotonically increasing counter.
- Use `id()` for cards you need to reference by variable.
- The IDs ARE distinct — `deck.contains(&a)` won't match `b` from the same card_no.

### 4. Skipping choice type assertions

```rust
// ❌ WRONG: silently tolerates wrong prompt types
if game.has_pending_choice() {
    game.select_indices(&[0]);
}

// ✅ CORRECT: assert the expected choice type
assert!(game.has_pending_choice(), "Expected SelectCard for discard");
assert_eq!(game.pending_choice_type(), Some("SelectCard"));
game.select_indices(&[0]);
```

### 5. Not clearing pending choices loop

```rust
// ❌ WRONG: might miss a prompt
game.activate_ability(card_id);
game.select_indices(&[0]);

// ✅ CORRECT: drain all prompts
game.activate_ability(card_id);
while game.has_pending_choice() {
    game.select_indices(&[0]); // or assert each one
}
```

### 6. Forgetting opponent's state

```rust
// ❌ WRONG: only checks own board
assert!(p1.hand.cards.contains(&card));

// ✅ CORRECT: verify both sides where applicable
assert!(p1.hand.cards.contains(&card), "P1 should have the card");
assert!(!p2.hand.cards.contains(&card), "P2 should NOT have the card");
```

### 7. Not accounting for `check_timing` triggers

`check_timing()` is called during every phase transition (inside `advance_phase`) and during `execute_effect`. It:
- Refreshes both players' decks from waitroom if empty
- Checks duplicate member rule (currently no-op safety check)
- Checks victory condition
- Checks invalid live/energy cards
- Checks orphaned under-member cards
- **Processes pending auto abilities** — so triggers may fire during phase transitions

This means that if you `play_to_stage` a card with a debut auto-ability, the ability may fire immediately during `play_to_stage`, not during a later `pass()`.

### 8. Adding debug-only injected functions

```rust
// ❌ WRONG: one-off helper that duplicates engine logic
fn my_setup(game: &mut TestGame) {
    game.state.player1.stage.stage[0] = my_card; // bypasses debut triggers!
}

// ✅ CORRECT: use standard actions
fn my_setup(game: &mut TestGame) {
    game.add_to_hand(my_card);
    game.give_energy(5);
    game.play_to_stage(my_card, MemberArea::LeftSide);
}
```

Prefer composing from existing helpers (`play_to_stage`, `activate_ability`, `pass`, `select_indices`) over poking raw game state. If you must manipulate state directly (e.g. setting up opponent's stage), add a comment explaining why.

## Phase Transition Reference Table

| `pass()` count | Phase after | Turn Phase | Side effect on P1 deck |
|---------------|-------------|------------|----------------------|
| 0 | Main | FirstAttackerNormal | — |
| 1 | Active | SecondAttackerNormal | — |
| 2 | Energy | SecondAttackerNormal | — |
| 3 | Draw | SecondAttackerNormal | — |
| 4 | Main | SecondAttackerNormal | **draw 1** |
| 5 | LiveCardSetFirstAttacker | Live | — |
| (set live card) | LiveCardSetFirstAttacker | Live | — |
| 6 | LiveCardSetSecondAttacker | Live | — |
| 7 | FirstAttackerPerformance | Live | — |

Total draws from deck during `advance_to_live_card_set_p1` (5 passes) → **1 draw**.

## Testing LiveStart Abilities — Correct Setup

```rust
fn test_live_start_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let karin = game.id("PL!N-pb1-004-R");
    let test_card = game.new_id("PL!-sd1-020-SD");  // live card, cost N/A
    let filler = game.id("PL!-sd1-010-SD");          // member, cost 4
    let live_card = game.id("PL!-sd1-020-SD");

    // Deck: 10 fillers from fill_decks + test_card at index 1
    fill_decks(&mut game);
    game.state.player1.main_deck.cards.insert(1, test_card);

    game.add_to_hand(live_card);
    game.add_to_hand(karin);
    game.add_to_hand(filler);
    game.give_energy(20);
    game.play_to_stage(filler, MemberArea::LeftSide);
    game.play_to_stage(karin, MemberArea::Center);

    // Advance through phases: 5 passes → LiveCardSetFirstAttacker (1 draw consumed filler_0)
    advance_to_live_card_set_p1(&mut game);
    // test_card is now at deck[0] (filler_0 was drawn)
    game.set_live_card(live_card);
    advance_to_live_start(&mut game);  // 2 passes → abilities fire

    // Handle all pending choices
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Assert results
    assert!(game.state.player1.waitroom.cards.contains(&test_card),
        "Non-member should be discarded");
    assert!(!game.state.player1.hand.cards.contains(&test_card),
        "Non-member should NOT be in hand");
}
```

## Filler Card Reference

Use `fill_decks()` to add 10 filler cards to each player's deck. Fillers have zero abilities and zero triggers — they won't interfere with your test.

| card_no | type | cost | notes |
|---------|------|------|-------|
| `PL!-sd1-010-SD` | member | 4 | most common filler |
| `PL!-sd1-020-SD` | live | N/A (score 2) | live card for tests |
| `PL!-sd1-021-SD` | live | N/A (score 3) | alternative live |

Always put at least 2 filler cards in zones (deck, discard, hand) to prevent edge cases with empty-zone detection, refresh mechanics, etc.

## Checking Your Test Count

Before submitting, run:
```bash
cargo test --test run_all 2>&1 | grep "test result"
```

You should see `1152 passed; 0 failed`. The exact count may vary as tests are added, but zero failures is the invariant.
