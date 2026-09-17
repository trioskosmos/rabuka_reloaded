use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_s_bp5_004_r_blade_option_selects_one_other_aqours_member() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let dia = g.id("PL!S-bp5-004-R");
    let aqours1 = g.id("PL!S-sd1-002-SD");
    let aqours2 = g.id("PL!S-sd1-003-SD");
    g.state.player1.stage.set_area(MemberArea::Center, aqours1);
    g.state
        .player1
        .stage
        .set_area(MemberArea::LeftSide, aqours2);
    g.state.player1.hand.cards.push(dia);
    g.give_energy(5);
    g.play_to_stage(dia, MemberArea::RightSide);
    assert!(
        g.has_pending_choice(),
        "Dia debut should show choice with 2 options"
    );
    g.select_option(0);
    assert!(
        g.has_pending_choice(),
        "Should prompt for Aqours member selection"
    );
    g.select_indices(&[1]);
    let aqours1_blade = g.state.mods.get_blade_modifier(aqours1);
    assert!(
        aqours1_blade > 0,
        "Selected Aqours member should have blade modifier (got {})",
        aqours1_blade
    );
    let aqours2_blade = g.state.mods.get_blade_modifier(aqours2);
    assert_eq!(
        aqours2_blade, 0,
        "Unselected Aqours member should NOT have blade modifier"
    );
}

#[test]
fn pl_s_bp5_004_r_blade_option_no_other_aqours_skips_target_choice() {
    let db = load_real_database();
    let mut g = TestGame::new(db.clone());
    let dia = g.id("PL!S-bp5-004-R");
    g.state.player1.hand.cards.push(dia);
    g.give_energy(5);
    g.play_to_stage(dia, MemberArea::Center);
    assert!(
        g.has_pending_choice(),
        "Dia debut should show choice with 2 options"
    );
    g.select_option(0);
    assert!(
        !g.has_pending_choice(),
        "Should NOT prompt for target selection when no valid targets"
    );
    let dia_blade = g.state.mods.get_blade_modifier(dia);
    assert_eq!(
        dia_blade, 0,
        "Dia should not receive blade (excluded by exclude_self)"
    );
}
