/// Tests for 宮下 愛 (PL!N-pb1-005-R) — Q197
///
/// 自動/ターン1: When a cost-10 member appears on your stage, draw 1.
use crate::helpers::*;

/// Cost-10 member appears on a DIFFERENT area (not replacing 宮下 愛).
/// Her auto ability SHOULD trigger → draw 1 card.
#[test]
fn miyashita_ai_cost10_appears_elsewhere_triggers_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ai = game.id("PL!N-pb1-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    let cost10 = game.id("PL!SP-bp2-006-P");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // Ai on LeftSide, filler on Center, RightSide open
    game.state.player1.stage.stage = [ai, filler, -1];
    game.state.player1.hand.cards.push(cost10);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(12);

    game.play_to_stage(cost10, rabuka_engine::zones::MemberArea::RightSide);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Hand started at 2 cards [cost10, filler].
    // After playing cost10 to empty slot: hand = [filler] (1 card).
    // Cost10 debuted WITHOUT baton touch → its "baton touch only" debut does NOT fire.
    // Ai's auto ability should see cost-10 member appear → draw 1.
    // Final hand: [filler, drawn] = 2 cards.
    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "Ai should draw 1 when cost-10 member appears on a different area"
    );
}

/// Cost-10 member REPLACES 宮下 愛 via baton touch.
/// Q197: Her auto ability should NOT trigger when she is the one being replaced.
#[test]
fn miyashita_ai_q197_baton_touch_replaced_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ai = game.id("PL!N-pb1-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    let cost10 = game.id("PL!SP-bp2-006-P");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    // Ai on LeftSide, filler on Center
    game.state.player1.stage.stage = [ai, filler, -1];
    game.state.player1.hand.cards.push(cost10);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(12);

    game.play_to_stage(cost10, rabuka_engine::zones::MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Hand started at 2 cards [cost10, filler].
    // After playing cost10 via baton touch (replaces Ai): hand = [filler] (1 card).
    // Cost10's debut triggers (baton touch used) but Ai is not Liella!, so no return.
    // Ai's auto ability should NOT trigger (Q197: she was replaced).
    // If buggy, Ai's draw would add another card → 2 cards.
    // Correct behavior: 1 card (no draw from Ai).
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Q197: Ai's auto should NOT trigger when she is replaced by baton touch"
    );
}
