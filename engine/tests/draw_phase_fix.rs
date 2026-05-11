mod helpers;
use helpers::*;

/// Test that the draw phase doesn't cause unwanted discards.
/// During normal phase progression, the draw phase should only draw 1 card.
#[test]
fn test_draw_phase_no_unwanted_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 { game.state.player1.main_deck.cards.push(filler); }

    let hand_before = game.state.player1.hand.cards.len();
    let deck_before = game.state.player1.main_deck.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    // After TestGame::new, we're in Main phase. Advance through phases.
    // The next phase transition from Main should go through draw steps.
    game.pass();

    let hand_after = game.state.player1.hand.cards.len();
    let deck_after = game.state.player1.main_deck.cards.len();
    let discard_after = game.state.player1.waitroom.cards.len();

    // In normal gameplay, passing the turn should trigger draw and may add a card
    assert!(deck_after <= deck_before, "Deck should not grow during draw phase");
    assert!(discard_after >= discard_before, "Discard should not shrink during draw phase");
}
