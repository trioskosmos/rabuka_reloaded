use crate::helpers::*;
use crate::test_modules::support::bp7_wait_immunity_helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn debut_all_low_blade_members_wait_respects_opponent_effect_immunity() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());

    // Player2 protects their 果南 (Aqours, blade 2).
    let p2_kanan = p2_establish_wait_immunity(&mut g);

    // Player1 plays 桂城 (PL!HS-pb1-008-R) → waits ALL blade≤3 members on both stages.
    let katsuraki = g.id("PL!HS-pb1-008-R");
    g.state.player1.hand.cards.push(katsuraki);
    g.give_energy(20);
    g.play_to_stage(katsuraki, MemberArea::Center);

    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }

    assert!(
        !is_waited(&g, p2_kanan),
        "桂城's all-members wait must be blocked for the protected member"
    );
}
