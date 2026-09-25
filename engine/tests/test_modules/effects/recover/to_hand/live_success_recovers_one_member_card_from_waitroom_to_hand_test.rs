use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const TRIPLE: &str = "LL-bp7-001-R＋";

// ====================================================================
// ab#2 (ライブ成功時): add 1 member card from waitroom to hand
// ====================================================================

fn trigger_live_success(game: &mut TestGame, card_id: i16) {
    fire_trigger(
        game,
        card_id,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    game.drain_auto_ability_choices();
}

#[test]
fn live_success_recovers_member_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    game.state.player1.stage.stage = [-1, triple, -1];
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(member);
    trigger_live_success(&mut game, triple);
    assert!(
        game.state.player1.hand.cards.contains(&member),
        "live success should add a member card from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&member),
        "the member card should leave the waitroom"
    );
}

#[test]
fn live_success_member_recovery_ignores_live_cards() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    game.state.player1.stage.stage = [-1, triple, -1];
    let live_in_waitroom = game.id("PL!-sd1-020-SD");
    game.state.player1.waitroom.cards.push(live_in_waitroom);
    trigger_live_success(&mut game, triple);
    assert!(
        game.state
            .player1
            .waitroom
            .cards
            .contains(&live_in_waitroom),
        "live cards in waitroom are not touched by ab#2"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&live_in_waitroom),
        "live card must not be added to hand by ab#2"
    );
}
