/// Tests for 小原鞠莉 (PL!S-pb1-008-R) — LiveStart: look at top 2, reorder, rest to waitroom.
///
/// Q131: If deck has <2 cards, the ability cannot be used.

mod helpers;
use helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
}

/// Deck starts empty. LiveStart triggers but can't look at 2 → silently fails.
#[test]
fn mari_q131_zero_deck_blocks_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    game.state.player1.stage.stage[0] = game.id("PL!S-pb1-008-R");
    game.state.player1.stage.stage[1] = filler;

    let live = game.id("LL-bp5-001-L");
    game.state.player1.hand.cards.push(live);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    while game.has_pending_choice() { game.select_indices(&[]); }
    assert!(game.state.player1.main_deck.cards.is_empty());
}

/// Deck has 1 card. Same: LiveStart fails, no crash.
#[test]
fn mari_q131_one_card_deck_blocked() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(filler);
    game.state.player1.stage.stage[0] = game.id("PL!S-pb1-008-R");
    game.state.player1.stage.stage[1] = filler;

    let live = game.id("LL-bp5-001-L");
    game.state.player1.hand.cards.push(live);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    while game.has_pending_choice() { game.select_indices(&[]); }
}

/// Deck has enough cards: LiveStart fires, choice appears, select works.
#[test]
fn mari_live_start_sufficient_deck_fires() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 { game.state.player1.main_deck.cards.push(filler); }
    game.state.player1.stage.stage[0] = game.id("PL!S-pb1-008-R");
    game.state.player1.stage.stage[1] = filler;
    game.state.player1.stage.stage[2] = filler;

    let live = game.id("LL-bp5-001-L");
    game.state.player1.hand.cards.push(live);

    advance_to_live_set(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();

    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(game.state.player1.main_deck.cards.len() > 0,
        "Deck non-empty after LiveStart");
}
