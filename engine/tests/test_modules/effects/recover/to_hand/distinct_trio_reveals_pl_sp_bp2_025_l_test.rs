use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!N-sd1-010-SD";
const KANON_SD: &str = "PL!SP-sd1-002-SD";
const WIEN_SD: &str = "PL!SP-sd2-010-SD2";

#[test]
fn pl_sp_bp2_025_l_two_distinct_trio_names_retrieve_revealed_card_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let live = game.id("PL!SP-bp2-025-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    let kanon = game.new_id(KANON_SD);
    let wien = game.new_id(WIEN_SD);
    game.state.player1.stage.stage[0] = kanon;
    game.state.player1.stage.stage[2] = wien;
    let prize = game.new_id("PL!SP-PR-003-PR");
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(prize);
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        game.state.player1.hand.cards.contains(&prize),
        "gate met -> revealed card retrieved to hand"
    );
}

#[test]
fn pl_sp_bp2_025_l_one_trio_name_does_not_retrieve_revealed_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);
    let live = game.id("PL!SP-bp2-025-L");
    game.add_to_hand(live);
    game.set_live_card(live);
    let kanon = game.new_id(KANON_SD);
    game.state.player1.stage.stage[0] = kanon;
    let prize = game.new_id("PL!SP-PR-003-PR");
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(prize);
    fire_trigger(&mut game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    assert!(
        !game.state.player1.hand.cards.contains(&prize),
        "<2 distinct names -> nothing retrieved"
    );
}
