use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";

#[test]
fn go_master_start_fewer_success_cards_scores() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp2-023-L");
    game.state.player1.live_card_zone.cards.push(live);

    let s1 = game.new_id(FILLER);
    game.state.player1.success_live_card_zone.cards.push(s1);
    let o1 = game.new_id(FILLER);
    let o2 = game.new_id(FILLER);
    game.state.player2.success_live_card_zone.cards.push(o1);
    game.state.player2.success_live_card_zone.cards.push(o2);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        1,
        "1 < 2 success cards -> score +1"
    );
}

#[test]
fn go_master_start_equal_success_cards_no_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp2-023-L");
    game.state.player1.live_card_zone.cards.push(live);

    let s1 = game.new_id(FILLER);
    game.state.player1.success_live_card_zone.cards.push(s1);
    let o1 = game.new_id(FILLER);
    game.state.player2.success_live_card_zone.cards.push(o1);

    fire_trigger(&mut game, live, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "1 vs 1 is not fewer -> no score"
    );
}
