use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";

#[test]
fn pl_sp_bp5_023_l_live_success_two_success_cards_and_revealed_score_live_grant_two_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp5-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    for no in ["PL!-sd1-019-SD", "PL!HS-bp2-020-L"] {
        let s = game.new_id(no);
        game.state.player1.success_live_card_zone.cards.push(s);
    }
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(game.new_id("PL!SP-bp1-023-L"));

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "both gates met -> live +2"
    );
}

#[test]
fn pl_sp_bp5_023_l_live_success_single_success_card_grants_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let live = game.id("PL!SP-bp5-023-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    let s = game.new_id("PL!-sd1-019-SD");
    game.state.player1.success_live_card_zone.cards.push(s);
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(game.new_id("PL!SP-bp1-023-L"));

    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(game.state.mods.get_score_modifier(live), 0);
}
