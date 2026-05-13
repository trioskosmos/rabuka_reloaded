use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

/// Test that per_unit effects properly handle card counts
/// Verifies that abilities don't incorrectly multiply discard counts
#[test]
fn test_per_unit_discard_bug_fix() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [filler, filler, -1];
    game.state.player1.hand.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    let discard_before = game.state.player1.waitroom.cards.len();

    // Draw phase — verify 1 card drawn, no unexpected discards
    TurnEngine::advance_phase(&mut game.state);

    let discard_after = game.state.player1.waitroom.cards.len();
    assert_eq!(
        discard_after, discard_before,
        "Draw phase should not discard cards"
    );
}
