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

    // P2's Draw phase BY NAME, then one step out of it. The draw happens on the
    // Draw → Main transition, so stopping ON Draw measures nothing — and
    // "4 passes" only happened to be that transition today. Two facts now:
    // the deck is untouched on ARRIVAL at Draw, and exactly one card leaves on
    // the way out.
    game.advance_to_phase(rabuka_engine::game_state::Phase::Draw);
    assert_eq!(
        game.state.current_phase,
        rabuka_engine::game_state::Phase::Draw,
        "the draw must be measured at the Draw phase, not an arbitrary pass"
    );
    assert_eq!(
        game.state.player2.main_deck.cards.len(),
        p2_deck_before,
        "no card leaves the deck BEFORE the draw phase"
    );
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
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
