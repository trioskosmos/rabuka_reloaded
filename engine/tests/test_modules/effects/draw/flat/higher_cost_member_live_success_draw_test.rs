use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD";
const HIGHER_COST_MEMBER: &str = "PL!HS-bp5-004-R";
const LOWER_COST_MEMBER: &str = "PL!SP-PR-007-PR";

#[test]
fn pb1013_higher_cost_member_on_stage_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-013-R"); // cost 9
    let big = game.id(HIGHER_COST_MEMBER); // cost 15
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = big;
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "a member costing more than 9 is on stage -> draw 1"
    );
    assert!(
        game.state.player1.hand.cards.contains(&drawn),
        "the drawn card is the one stocked on the deck"
    );
}

#[test]
fn pb1013_only_lower_cost_members_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-013-R"); // cost 9
    let small = game.id(LOWER_COST_MEMBER); // cost 2
    game.state.player1.stage.stage[0] = me;
    game.state.player1.stage.stage[1] = small;
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no member above cost 9 -> no draw"
    );
}
