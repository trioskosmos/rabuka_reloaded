/// Tests for PL!SP-bp5-004-R+ (平安名すみれ) ab#0 — Q220
///
/// Ability (自動[ターン1]):
///   このカードの効果によって、このメンバーがエリアを移動するか
///   自分のエネルギー置き場にエネルギーが置かれた時、
///   カードを1枚引き、ライブ終了時まで、heart02を得る。
///
/// Q220: ポジションチェンジによりこのメンバーが移動する場合、
///        自動能力は発動するか？
/// Answer: はい、発動する。
use crate::helpers::*;

/// Sumire at P1's Center. Wien in hand. Play Wien → Wien's debut triggers
/// position_change for both players' center members.
/// P1 chooses RightSide for Sumire → Sumire moves from Center to RightSide.
/// Q220: After position change, verify Sumire moved (the auto ability
/// trigger depends on engine's movement_condition evaluation).
#[test]
fn sumire_bp5_q220_position_change_moves_sumire() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let sumire = game.id("PL!SP-bp5-004-R\u{ff0b}");
    let wien = game.id("PL!SP-bp5-010-R");
    let filler = game.id("PL!-sd1-010-SD");
    let p2_center = game.id("PL!-sd1-013-SD");

    // Sumire at P1 Center, P2 also has a center member
    game.state.player1.stage.stage = [-1, sumire, -1];
    game.state.player2.stage.stage = [-1, p2_center, -1];

    game.state.player1.hand.cards.push(wien);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(13);

    // Play Wien — debut triggers position_change for both
    game.play_to_stage(wien, rabuka_engine::zones::MemberArea::LeftSide);

    // Opponent's choice first
    if game.has_pending_choice() {
        game.select_option(0); // move P2 center to LeftSide
    }

    // P1's choice: Sumire at Center → choose destination
    assert!(
        game.has_pending_choice(),
        "P1 should get choice for Sumire's destination"
    );
    game.select_option(2); // RightSide

    // Verify Sumire moved
    assert_eq!(
        game.state.player1.stage.stage[2], sumire,
        "Q220: Sumire should be at RightSide after position change"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], -1,
        "P1 Center empty after Sumire moved"
    );
}
