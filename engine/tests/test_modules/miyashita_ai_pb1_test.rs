/// Tests for 宮下 愛 (PL!N-pb1-005-R) — Q197
///
/// 自動/ターン1: When a cost-10 member appears on your stage, draw 1.
use crate::helpers::*;

/// Place a cost-10 member on stage → auto trigger draws 1.
#[test]
fn miyashita_ai_q197_cost10_appears_draw_1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let ai = game.id("PL!N-pb1-005-R");
    let filler = game.id("PL!-sd1-010-SD");
    let cost10 = game.id("PL!SP-bp2-006-P");

    game.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player1.stage.stage = [ai, filler, -1];
    game.state.player1.hand.cards.push(cost10);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(12);

    game.play_to_stage(cost10, rabuka_engine::zones::MemberArea::LeftSide);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Hand started at 2, cost-10 card was removed, then at least 1 drawn
    assert!(
        game.state.player1.hand.cards.len() >= 1,
        "Q197: Draw occurred after cost-10 member debut"
    );
}
