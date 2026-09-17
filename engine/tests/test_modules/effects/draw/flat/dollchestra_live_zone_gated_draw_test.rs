use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

#[test]
fn hs_pb1_021_live_success_with_dollchestra_in_live_zone_draws_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-021-N");
    game.state.player1.stage.stage[0] = me;
    // A DOLLCHESTRA live card in the live card zone.
    let doll_live = game.id("PL!HS-bp2-020-L");
    game.state.player1.live_card_zone.cards.push(doll_live);
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before + 1,
        "DOLLCHESTRA in live zone -> draw 1"
    );
}

#[test]
fn hs_pb1_021_live_success_without_dollchestra_in_live_zone_does_not_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!HS-pb1-021-N");
    game.state.player1.stage.stage[0] = me;
    // Non-DOLLCHESTRA live card: Link to the FUTURE is DOLLCHESTRA, so use a
    // μ's live card instead.
    let other_live = game.id("PL!-sd1-020-SD");
    game.state.player1.live_card_zone.cards.push(other_live);
    let drawn = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(drawn);
    let hand_before = game.state.player1.hand.cards.len();

    fire_trigger(&mut game, me, AbilityTrigger::LiveSuccess, "ライブ成功時");

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "no DOLLCHESTRA in live zone -> no draw"
    );
    let _ = drawn;
}
