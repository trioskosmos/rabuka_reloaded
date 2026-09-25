use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_hs_bp2_003_r_live_start_skip_reorder_sends_all_three_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!HS-bp2-003-R");
    game.state.player1.stage.stage[1] = me;
    game.add_to_hand(game.new_id(FILLER));
    let d1 = game.new_id("PL!N-sd1-001-SD");
    let d2 = game.new_id("PL!N-sd1-002-SD");
    let d3 = game.new_id("PL!N-sd1-003-SD");
    for c in [d1, d2, d3] {
        game.state.player1.main_deck.cards.insert(0, c);
    }

    fire_trigger(&mut game, me, AbilityTrigger::LiveStart, "ライブ開始時");
    game.select_option(0);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        game.select_indices(&[]);
    }

    let wr = &game.state.player1.waitroom.cards;
    assert!(
        wr.contains(&d1) && wr.contains(&d2) && wr.contains(&d3),
        "skipping placement sends the looked cards to the waitroom"
    );
}
