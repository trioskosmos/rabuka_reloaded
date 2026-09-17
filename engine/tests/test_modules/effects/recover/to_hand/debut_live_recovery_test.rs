use crate::helpers::*;

const TRIPLE: &str = "LL-bp7-001-R＋";
const LIVE_CARD: &str = "PL!-sd1-020-SD";

// ====================================================================
// ab#1 (登場): add 1 live card from waitroom to hand
// ====================================================================

#[test]
fn debut_recovers_live_card_from_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    let live = game.id(LIVE_CARD);
    game.state.player1.waitroom.cards.push(live);
    game.state.player1.hand.cards.push(triple);
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "debut should add a live card from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "the live card should leave the waitroom"
    );
}

#[test]
fn debut_live_recovery_leaves_member_in_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let triple = game.id(TRIPLE);
    game.state.player1.hand.cards.push(triple);
    let member = game.id("PL!-sd1-010-SD");
    game.state.player1.waitroom.cards.push(member);
    game.give_energy(15);
    game.play_to_stage(triple, rabuka_engine::zones::MemberArea::Center);
    assert!(
        game.state.player1.waitroom.cards.contains(&member),
        "non-live cards in waitroom are not touched by ab#1"
    );
}
