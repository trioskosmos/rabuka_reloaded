use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member
const NIJI_MEMBER_ID: &str = "PL!N-bp3-004-R";

// ====================================================================
// PL!N-sd1-005-PRproteinbar (起動 ターン1回):
// 「手札を2枚控え室に置く：自分の控え室から『虹ヶ咲』のメンバーカードを1枚手札に加える。」
// ====================================================================

#[test]
fn n_sd1_005_proteinbar_activation_discard_two_recovers_niji_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!N-sd1-005-PRproteinbar");
    game.state.player1.stage.stage[0] = me;

    let niji = game.id(NIJI_MEMBER_ID);
    game.state.player1.waitroom.cards.push(niji);

    // Exactly two filler cards in hand — the discard cost.
    let f1 = game.new_id(FILLER);
    let f2 = game.new_id(FILLER);
    game.state.player1.hand.cards.push(f1);
    game.state.player1.hand.cards.push(f2);

    game.activate_ability(me);
    assert!(
        game.has_pending_choice(),
        "2-card hand-discard cost must be prompted"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard discard-cost prompt"
    );
    // Discard-cost selection: choose both hand cards.
    game.select_indices(&[0, 1]);

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "虹ヶ咲 member retrieved from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&niji),
        "retrieved member left the waitroom"
    );
}
