use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member
const NIJI_MEMBER_ID: &str = "PL!N-bp3-004-R";

// ====================================================================
// PL!N-bp3-004-R (起動 ターン1回):
// 「このメンバーをウェイトにし、手札を1枚控え室に置く：
//   自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える。」
// ====================================================================

#[test]
fn n_bp3_004_activation_rest_and_discard_recovers_niji_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(NIJI_MEMBER_ID);
    game.state.player1.stage.stage[0] = me;

    let live = game.id("PL!N-bp1-025-L"); // 虹ヶ咲 live card
    game.state.player1.waitroom.cards.push(live);

    let f1 = game.new_id(FILLER);
    game.state.player1.hand.cards.push(f1);

    game.activate_ability(me);
    assert!(
        game.has_pending_choice(),
        "1-card hand-discard cost must be prompted"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard discard-cost prompt"
    );
    game.select_indices(&[0]);

    assert_eq!(
        game.state.mods.orientation_modifiers.get(&me).copied(),
        Some(rabuka_engine::core::game_modifiers::CardOrientation::Wait),
        "this member was rested as part of the cost"
    );
    assert!(
        game.state.player1.hand.cards.contains(&live),
        "虹ヶ咲 live card retrieved to hand"
    );
}
