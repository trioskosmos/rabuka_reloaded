use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// 虹ヶ咲学園 (PL!N-bp1-006-R) ab0:
/// "{{E}}{{E}}、カードを1枚引く"
/// As written: pay exactly 2 energy, draw exactly 1 card.
#[test]
fn nijigasaki_bp1_006r_pay_two_draw_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let member = game.id("PL!N-bp1-006-R＋");
    let filler = game.id("PL!-sd1-010-SD");

    game.add_to_stage(MemberArea::Center, member);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(2);

    let before_hand = game.state.player1.hand.cards.len();
    let before_deck = game.state.player1.main_deck.cards.len();

    game.try_activate_ability(member)
        .expect("activation should succeed with 2 energy");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    game.assert_energy(0, "exactly 2 energy paid");
    assert_eq!(
        game.state.player1.hand.cards.len(),
        before_hand + 1,
        "drew exactly 1 card"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        before_deck - 1,
        "deck lost exactly 1 card"
    );
}
