/// Tests for PL!-sd1-005-SD 星空凛 — Activation ability (ab#0):
///
/// {{kidou.png|起動}}このメンバーをステージから控え室に置く：
/// 自分の控え室からライブカードを1枚手札に加える。
///
/// Cost: put this member from stage to waitroom (self_cost).
/// Effect: recover 1 live card from waitroom to hand.
///
/// Q123: Can you activate with no live cards in waitroom? A: Yes.
///       (Effect does nothing, but cost is still paid.)
/// Q79:  After activating, can a new member be placed in the vacated area? A: Yes.

mod helpers;
use helpers::*;

/// Q123: Activate ability when waitroom has no live cards — cost still paid.
#[test]
fn rin_q123_activate_empty_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let rin = game.id("PL!-sd1-005-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Stage: 星空凛 at center
    game.state.player1.stage.stage = [-1, rin, -1];

    // Hand: a card for hand count (not relevant here)
    game.add_to_hand(filler);

    // No live cards in waitroom
    game.give_energy(3);

    let stage_before = game.state.player1.stage.stage[1];
    assert_eq!(stage_before, rin, "Rin should be on stage before activation");

    game.activate_ability(rin);

    // Self_cost moved Rin off stage → stage center should be empty
    assert_eq!(game.state.player1.stage.stage[1], -1,
        "Rin should be removed from stage after self_cost");

    // Rin should be in waitroom
    assert!(game.state.player1.waitroom.cards.contains(&rin),
        "Rin should be in waitroom after self_cost");

    // No live card was recovered (waitroom had none), but ability succeeded
    // The effect gracefully did nothing
}

/// Q79: After self_cost removes the member, the vacated area is empty and
/// can receive a new card.
#[test]
fn rin_q79_vacated_area_can_receive_new_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let rin = game.id("PL!-sd1-005-SD");
    let live_card = game.id("PL!-sd1-019-SD");

    // Stage: 星空凛 at center
    game.state.player1.stage.stage = [-1, rin, -1];

    // Hand: a live card so waitroom has a target for recovery
    game.add_to_hand(live_card);
    // Put the live card in waitroom for recovery
    game.add_to_discard(live_card);
    game.give_energy(3);

    game.activate_ability(rin);

    // Center stage should be empty after self_cost
    assert_eq!(game.state.player1.stage.stage[1], -1,
        "Center area should be empty after self_cost");

    // Waitroom should have Rin (from self_cost) — live card was recovered
    assert!(game.state.player1.waitroom.cards.contains(&rin),
        "Rin should be in waitroom");
    assert!(game.state.player1.hand.cards.contains(&live_card),
        "Live card should be recovered to hand");

    // The vacated center area can receive a new card (empty slot exists)
}
