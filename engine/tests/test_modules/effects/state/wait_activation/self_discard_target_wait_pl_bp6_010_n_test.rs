use crate::helpers::*;
use crate::test_modules::support::bp7_wait_immunity_helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_bp6_010_n_cost_limit_wait_blocked_by_wait_immunity() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let p2_kanan = p2_establish_wait_immunity(&mut g);
    let honoka = g.id("PL!-bp6-010-N");
    g.state.player1.hand.cards.push(honoka);
    g.give_energy(10);
    g.play_to_stage(honoka, MemberArea::RightSide);
    g.activate_ability(honoka);
    while g.has_pending_choice() {
        g.select_indices(&[0]);
    }
    assert!(
        !is_waited(&g, p2_kanan),
        "穂乃果's cost-limit wait must be blocked by wait-immunity"
    );
}

#[test]
fn pl_bp6_010_n_self_discard_waits_selected_target_keeps_stage() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let honoka = g.id("PL!-bp6-010-N");
    let target1 = g.id("PL!-sd1-010-SD");
    let target2 = g.id("PL!-sd1-013-SD");
    g.state.player2.stage.set_area(MemberArea::Center, target1);
    g.state
        .player2
        .stage
        .set_area(MemberArea::LeftSide, target2);
    g.state.player1.hand.cards.push(honoka);
    g.give_energy(10);
    g.play_to_stage(honoka, MemberArea::RightSide);
    g.activate_ability(honoka);
    let p1_right = g.state.player1.stage.get_area(MemberArea::RightSide);
    assert_eq!(p1_right, None, "Honoka should have left stage (self_cost)");
    assert!(
        g.state.player1.waitroom.cards.contains(&honoka),
        "Honoka should be in waitroom"
    );
    assert!(
        g.has_pending_choice(),
        "Should prompt for change_state target selection"
    );
    g.select_indices(&[1]);
    let p2_center = g.state.player2.stage.get_area(MemberArea::Center);
    assert_eq!(p2_center, Some(target1), "Target should remain on stage");
    let target1_ori = g.state.mods.get_orientation_modifier(target1);
    assert_eq!(target1_ori, Some("wait"), "Target should be in wait state");
    let target2_ori = g.state.mods.get_orientation_modifier(target2);
    assert_eq!(
        target2_ori, None,
        "Unselected target should have no orientation modifier"
    );
}
