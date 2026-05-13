/// Live timing definition QAs — verify LiveSuccess fires at the correct phase.

mod helpers;
use helpers::*;

fn advance_to_live_success(game: &mut TestGame) {
    for _ in 0..5 { game.pass(); }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.pass(); game.pass(); game.pass(); game.pass(); game.pass();
}

/// Q36: LiveSuccess fires during LiveVictoryDetermination, after both performances.
/// Verify by checking the phase after the final pass.
#[test]
fn q36_live_success_timing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let live = game.id("PL!-sd1-019-SD");
    let member = game.id("PL!-sd1-001-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(live);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    advance_to_live_success(&mut game);

    // After LiveVictoryDetermination processes, the turn advances to Active
    // (or back to Main if second turn). LiveSuccess abilities already fired.
    let phase = game.state.current_phase.to_string();
    assert_eq!(phase, "Active",
        "After LiveVictoryDetermination, phase should be Active, got {}", phase);
}
