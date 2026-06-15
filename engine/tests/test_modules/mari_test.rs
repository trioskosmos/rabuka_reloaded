/// Tests for 小原鞠莉 (PL!S-pb1-008-R) — LiveStart: look at top 2, reorder, rest to waitroom.
///
/// Q131: If deck has <2 cards, the ability cannot be used.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Deck starts empty. LiveStart triggers but can't look at 2 → silently fails.
#[test]
fn mari_q131_zero_deck_blocks_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    // Give 2 cards: one consumed by set_live_card's replacement draw,
    // leaving 0 — not enough for the 2-card look needed by the ability
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.main_deck.cards.push(filler);
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = game.id("PL!S-pb1-008-R");
    game.state.player1.stage.stage[1] = filler;

    let live = game.id("LL-bp5-001-L");
    game.state.player1.hand.cards.push(live);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // After replacement draw, deck has 0 (< 2 needed) → ability blocked
    assert!(
        !game.has_pending_choice(),
        "No pending choices when deck has 0 cards (set_live_card consumed the only 2)"
    );
}

/// Deck has 1 card (< 2 needed). LiveStart fails, no choices created.
#[test]
fn mari_q131_one_card_deck_blocked() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler);
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = game.id("PL!S-pb1-008-R");
    game.state.player1.stage.stage[1] = filler;

    let live = game.id("LL-bp5-001-L");
    game.state.player1.hand.cards.push(live);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // Ability blocked: no pending choices
    assert!(
        !game.has_pending_choice(),
        "No pending choices when deck has < 2 cards"
    );
}

/// Deck has enough cards: LiveStart fires, choice appears, select works.
#[test]
fn mari_live_start_sufficient_deck_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage[0] = game.id("PL!S-pb1-008-R");
    game.state.player1.stage.stage[1] = filler;
    game.state.player1.stage.stage[2] = filler;

    let live = game.id("LL-bp5-001-L");
    game.state.player1.hand.cards.push(live);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // Ability should have fired: choice for look-and-select (which cards to keep)
    assert!(
        game.has_pending_choice(),
        "LiveStart ability should create a choice when deck has >= 2 cards"
    );

    // Select card 0 to keep (reorder on top), unselected card goes to waitroom
    game.select_indices(&[0]);

    // Resolve any remaining prompts (finish selection, order, etc.)
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        !game.has_pending_choice(),
        "No more pending choices after look-and-select resolves"
    );
}
