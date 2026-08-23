/// Untested-abilities batch 16 — 起動 (activation) abilities with waitroom retrieval:
/// - PL!N-sd1-005-PRproteinbar: discard 2 from hand → retrieve 虹ヶ咲 member
/// - PL!N-bp3-004-R: rest self + discard 1 → retrieve 虹ヶ咲 live card
/// - PL!SP-bp4-018-N: rest self (to waitroom) → retrieve Liella! card
use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member
const NIJI_MEMBER_ID: &str = "PL!N-bp3-004-R";

// ====================================================================
// PL!N-sd1-005-PRproteinbar (起動 ターン1回):
// 「手札を2枚控え室に置く：自分の控え室から『虹ヶ咲』のメンバーカードを1枚手札に加える。」
// ====================================================================

#[test]
fn proteinbar_discard2_retrieves_niji_member() {
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
    if game.has_pending_choice() {
        // Discard-cost selection: choose both hand cards.
        game.select_indices(&[0, 1]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&niji),
        "虹ヶ咲 member retrieved from waitroom to hand"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&niji),
        "retrieved member left the waitroom"
    );
}

// ====================================================================
// PL!N-bp3-004-R (起動 ターン1回):
// 「このメンバーをウェイトにし、手札を1枚控え室に置く：
//   自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える。」
// ====================================================================

#[test]
fn bp3004_rest_self_and_discard_retrieves_niji_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id(NIJI_MEMBER_ID);
    game.state.player1.stage.stage[0] = me;

    let live = game.id("PL!N-bp1-025-L"); // 虹ヶ咲 live card
    game.state.player1.waitroom.cards.push(live);

    let f1 = game.new_id(FILLER);
    game.state.player1.hand.cards.push(f1);

    game.activate_ability(me);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

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

// ====================================================================
// PL!SP-bp4-018-N (起動):
// 「このメンバーをステージから控え室に置く：
//   自分の控え室から『Liella!』のカードを1枚手札に加える。」
// ====================================================================

#[test]
fn bp4018_rests_self_to_waitroom_retrieves_liella() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = game.id("PL!SP-bp4-018-N"); // CatChu! member
    game.state.player1.stage.stage[0] = me;

    // A Liella! member (澁谷かのん, CatChu! sub-unit) sits in the waitroom.
    let liella = game.id("PL!SP-pb1-001-PR");
    game.state.player1.waitroom.cards.push(liella);

    game.activate_ability(me);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&me),
        "this member moved to the waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&liella),
        "Liella! card retrieved to hand"
    );
}
