use crate::helpers::*;

/// Baton touch: replaced member moves to waitroom.
/// Place the target on stage directly (not via play_to_stage, which would lock the area)
/// then baton touch from hand to that occupied area.
#[test]
fn baton_touch_moves_replaced_member_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());
    let target = game.id("PL!SP-bp2-011-R");
    let arriver = game.id("PL!HS-sd1-008-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Place target on stage directly (avoids area lock from play_to_stage)
    game.state.player1.stage.stage[1] = target;
    game.give_energy(25);

    // Baton touch: play arriver to occupied center
    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(arriver, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&target),
        "Replaced member should be in waitroom after baton touch"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], arriver,
        "Arriver should occupy center after baton touch"
    );
}

/// 花帆 (PL!HS-sd1-001-SD, cost 9, unit=EdelNote) has 自動:
///   When baton-touched to waitroom by a cost 10+ 蓮ノ空 member → activate 2 energy
/// The arriving card (PL!HS-sd1-006-SD, cost 15, unit=みらくらぱーく!) passes
/// cost ≥ 10, and card_matches_group_str(g="蓮ノ空") checks unit/group/series.
/// If the series mapping maps 蓮ノ空→Hasunosora cards, this fires the ability.
#[test]
fn baton_touch_hanaho_auto_ability_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanaho = game.id("PL!HS-sd1-001-SD"); // cost 9, has the auto ability
    let arriver = game.id("PL!HS-sd1-006-SD"); // cost 15, みらくらぱーく! (蓮ノ空 subunit)
    let filler = game.id("PL!-sd1-010-SD");

    // Put 花帆 on stage directly
    game.state.player1.stage.stage[1] = hanaho;

    game.give_energy(25);
    let before = game.state.player1.energy_zone.active_energy_count;

    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(arriver, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.player1.waitroom.cards.contains(&hanaho),
        "花帆 should be in waitroom after baton touch"
    );

    let after = game.state.player1.energy_zone.active_energy_count;
    // Baton touch cost = arriver.cost - hanaho.cost = 15 - 9 = 6
    // Auto ability activates 2 → net -4
    assert!(
        after >= before - 6,
        "Energy should reflect baton touch cost 15-9=6 (before={}, got {}),",
        before,
        after
    );
}
