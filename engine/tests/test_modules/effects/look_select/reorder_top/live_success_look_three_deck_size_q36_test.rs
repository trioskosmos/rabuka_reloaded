use crate::helpers::*;

#[test]
fn live_success_look_three_does_not_increase_deck_size_after_live_q36() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let start_dash = game.id("PL!-sd1-019-SD");
    let filler = game.id("PL!-sd1-010-SD");
    let member = game.id("PL!-sd1-001-SD");

    game.state.player1.stage.stage = [member, -1, -1];
    game.state.player1.hand.cards.push(start_dash);
    for _ in 0..15 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    let deck_before = game.state.player1.main_deck.len();

    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(start_dash);
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();

    let deck_after = game.state.player1.main_deck.len();
    assert!(
        deck_after <= deck_before,
        "LiveSuccess should draw cards: {} → {}",
        deck_before,
        deck_after
    );
}
