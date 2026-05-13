use crate::helpers::*;

/// Test that the draw phase doesn't cause unwanted discards.
/// During normal phase progression, the draw phase should only draw 1 card.
#[test]
fn test_draw_phase_no_unwanted_discards() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");

    // Both players need decks for phase progression
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // After TestGame::new, P1 is in Main phase (their turn is complete).
    // The next pass goes to P2's Active phase (SecondAttackerNormal).
    // Advance through P2's turn: Active → Energy → Draw → Main
    // P2 draws 1 card on the 4th pass (Draw→Main transition).
    let p2_deck_before = game.state.player2.main_deck.cards.len();
    let p2_discard_before = game.state.player2.waitroom.cards.len();

    for _ in 0..4 {
        game.pass();
    }

    let p2_deck_after = game.state.player2.main_deck.cards.len();
    let p2_discard_after = game.state.player2.waitroom.cards.len();

    assert_eq!(
        p2_deck_after,
        p2_deck_before - 1,
        "1 card drawn from deck during draw phase"
    );
    assert_eq!(
        p2_discard_after, p2_discard_before,
        "No discard during draw phase"
    );
}
