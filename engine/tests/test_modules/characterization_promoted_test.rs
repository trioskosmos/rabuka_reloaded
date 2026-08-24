//! PROMOTED characterization tests (upgraded from generated stubs).
//! These pin actual gameplay for previously-untested abilities.
//! The generator will not re-create stubs for cards referenced here.

use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// 村野さやか (PL!HS-bp1-002-R):
/// "{{E}}{{E}}、このメンバーをステージから控え室に置く：自分の控え室から
/// コスト15以下の『蓮ノ空』のメンバーカードを1枚、このメンバーがいたエリアに登場させる。"
///
/// As written: pay 2 energy + bounce self to waitroom; a 蓮ノ空 member
/// costing <=15 debuts from the waitroom into the SAME area.
#[test]
fn sayaka_self_bounce_retrieves_hasunosora_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let sayaka = game.id("PL!HS-bp1-002-R");
    let retrieved = game.id("PL!HS-sd1-006-SD"); // 蓮ノ空 member

    game.add_to_stage(MemberArea::Center, sayaka);
    game.add_to_discard(retrieved);
    game.give_energy(2);

    // Activate: pay 2E + self to waitroom, then debut `retrieved` here.
    game.try_activate_ability(sayaka).expect("activation should succeed");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Cost paid exactly.
    game.assert_energy(0, "2 energy activation cost");
    // Self bounced to the waitroom.
    assert!(
        game.state.player1.waitroom.cards.contains(&sayaka),
        "さやか should be in the waitroom after her own activation cost"
    );
    // The retrieved member debuted into the SAME area (center).
    game.assert_stage_pos(MemberArea::Center, retrieved, "retrieved member debuts in さやか's old area");
    assert!(
        !game.state.player1.waitroom.cards.contains(&retrieved),
        "retrieved member must leave the waitroom"
    );
}

// NOTE: 松浦果南 (PL!S-bp6-003-R) conditional swap remains on the stub ladder
// (characterization_test.rs) — promotion needs multi-member cost fixtures.
// Its parsed structure (sequential + cost_offset=2 retrieval) is verified by
// pipeline_report and the corpus smoke today.
