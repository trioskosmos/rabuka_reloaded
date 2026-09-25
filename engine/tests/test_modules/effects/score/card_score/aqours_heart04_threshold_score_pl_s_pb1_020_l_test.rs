use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

#[test]
fn pl_s_pb1_020_l_aqours_heart04_total_ten_grants_two_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-020-L");
    game.state.player1.live_card_zone.cards.push(live);

    let a1 = game.id("PL!S-bp5-007-R");
    let a2 = game.new_id("PL!S-bp5-007-R");
    game.state.player1.stage.stage[0] = a1;
    game.state.player1.stage.stage[1] = a2;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "combined printed heart04 = 10 -> score +2"
    );
}

#[test]
fn pl_s_pb1_020_l_aqours_heart04_total_five_grants_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!S-pb1-020-L");
    game.state.player1.live_card_zone.cards.push(live);

    let a1 = game.id("PL!S-bp5-007-R");
    game.state.player1.stage.stage[0] = a1;

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "combined printed heart04 = 5 -> no score"
    );
}
