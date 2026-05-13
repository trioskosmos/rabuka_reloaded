/// Q188: 近江彼方 (PL!N-bp4-018-N) — Auto: when active->wait during main, draw 1 drop 1.
use crate::helpers::*;

#[test]
fn konata_bp4_q188_placed_in_wait_no_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let konata = game.id("PL!N-bp4-018-N");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [konata, filler, filler];
    game.state.mods.add_orientation_modifier(konata, "wait");

    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, "p1");
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.hand.cards.len(),
        0,
        "No cards drawn — condition not met"
    );
}
