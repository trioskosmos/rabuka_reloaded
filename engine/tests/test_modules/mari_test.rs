/// Tests for 小原鞠莉 (PL!S-pb1-008-R) — LiveStart: choose self or opponent,
/// look at top 2 of chosen deck, reorder, rest to waitroom.
///
/// Q131: If deck has <2 cards, the ability cannot be used.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

/// Helper: answer "choose self or opponent" with "自分" (self).
fn choose_self(game: &mut TestGame) {
    if game.has_pending_choice() {
        game.select_option(0);
    }
}

/// Helper: answer "choose self or opponent" with "相手" (opponent).
fn choose_opponent(game: &mut TestGame) {
    if game.has_pending_choice() {
        game.select_option(1);
    }
}

/// Deck starts empty. LiveStart triggers → choose self → look fails silently.
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

    // Ability fires → choose self or opponent
    assert!(
        game.has_pending_choice(),
        "Choose-target choice should appear"
    );
    choose_self(&mut game);

    // After choose self, look fails silently since deck has 0 cards
    assert!(
        !game.has_pending_choice(),
        "No pending choices when deck has 0 cards"
    );
}

/// Deck has 1 card (< 2 needed). LiveStart fires → choose self → fails silently.
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

    // Ability fires → choose self or opponent
    assert!(
        game.has_pending_choice(),
        "Choose-target choice should appear"
    );
    choose_self(&mut game);

    // After choose self, look fails silently since deck has 1 card
    assert!(
        !game.has_pending_choice(),
        "No pending choices when deck has < 2 cards"
    );
}

/// Deck has enough cards: choose self, look at own deck, select, reorder.
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

    // Step 1: choose self or opponent
    assert!(
        game.has_pending_choice(),
        "Choose-target choice should appear"
    );
    choose_self(&mut game);

    // Step 2: look-and-select (which cards to keep)
    assert!(
        game.has_pending_choice(),
        "LiveStart ability should create look-and-select choice"
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

/// Choose opponent, look at opponent's deck, keep selected, discard rest.
#[test]
fn mari_choose_opponent_look_at_opponent_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // P2 deck: all filler cards
    game.state.player2.main_deck.cards.clear();
    let p2_initial = 12;
    for _ in 0..p2_initial {
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

    assert!(
        game.has_pending_choice(),
        "Choose-target choice should appear"
    );
    choose_opponent(&mut game);

    assert!(
        game.has_pending_choice(),
        "Look-and-select choice should appear"
    );

    let deck_before = game.state.player2.main_deck.cards.len();

    // Keep card 0, discard the rest
    game.select_indices(&[0]);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(!game.has_pending_choice(), "No more pending choices");

    // Deck should have 1 more card than after the look-at (1 was returned)
    let deck_after = game.state.player2.main_deck.cards.len();
    assert_eq!(
        deck_after,
        deck_before + 1,
        "Opponent deck should have 1 card returned ({} before selection, {} after)",
        deck_before,
        deck_after
    );

    // Waitroom should have the non-selected looked-at card (and any previously discarded)
    let waitroom_after = game.state.player2.waitroom.cards.len();
    assert!(
        waitroom_after >= 1,
        "Waitroom should contain the discarded looked-at card (has {})",
        waitroom_after
    );
}
/// Choose opponent but opponent's deck is empty → ability blocked.
#[test]
fn mari_choose_opponent_empty_deck_blocked() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // P2 deck starts empty
    game.state.player2.main_deck.cards.clear();

    game.state.player1.stage.stage[0] = game.id("PL!S-pb1-008-R");
    game.state.player1.stage.stage[1] = filler;

    let live = game.id("LL-bp5-001-L");
    game.state.player1.hand.cards.push(live);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    // Step 1: choose opponent
    assert!(
        game.has_pending_choice(),
        "Choose-target choice should appear"
    );
    choose_opponent(&mut game);

    // Step 2: opponent's deck empty → look fails silently
    assert!(
        !game.has_pending_choice(),
        "No pending choice when opponent's deck is empty"
    );
}
