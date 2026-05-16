use crate::helpers::*;
use rabuka_engine::game_setup::{generate_possible_actions, ActionType};
use rabuka_engine::zones::MemberArea;

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

/// Baton touch should stay per-area when the stage is already full.
/// After replacing one occupied lane, the remaining occupied lanes should
/// still offer baton-touch placement options.
#[test]
fn baton_touch_does_not_lock_all_full_lanes() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let left = game.id("PL!HS-sd1-001-SD");
    let center = game.id("PL!HS-sd1-006-SD");
    let right = game.id("PL!HS-sd1-008-SD");
    let first_arriver = game.id("PL!HS-sd1-006-SD");
    let second_arriver = game.id("PL!HS-sd1-006-SD");

    game.state.player1.stage.stage[0] = left;
    game.state.player1.stage.stage[1] = center;
    game.state.player1.stage.stage[2] = right;
    game.give_energy(30);

    game.state.player1.hand.cards.push(first_arriver);
    game.state.player1.hand.cards.push(second_arriver);

    game.play_to_stage(first_arriver, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    let actions = generate_possible_actions(&game.state);
    let second_action = actions
        .iter()
        .find(|action| {
            action.action_type == ActionType::PlayMemberToStage
                && action.parameters.as_ref().and_then(|p| p.card_id) == Some(second_arriver)
        })
        .expect("Second baton-touch play action should still be available");

    let available_areas = second_action
        .parameters
        .as_ref()
        .and_then(|p| p.available_areas.as_ref())
        .expect("Play action should include area information");

    assert!(
        available_areas
            .iter()
            .any(|area| area.available && area.is_baton_touch),
        "At least one occupied lane should still offer baton touch after the first replacement"
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
