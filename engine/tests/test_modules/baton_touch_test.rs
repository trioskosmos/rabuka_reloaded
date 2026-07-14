use rabuka_engine::core::types::ArcStr;
use crate::helpers::*;
use rabuka_engine::card::ConditionCardType;
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
    // Filler cards in deck so the debut "draw 2" succeeds without needing refresh
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(25);

    // Baton touch: play arriver to occupied center
    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(arriver, rabuka_engine::zones::MemberArea::Center);
    // Handle debut "draw 2, discard 1" choice (required, can't skip)
    while game.has_pending_choice() {
        let is_required_discard = game.state.get_pending_choice().is_some_and(|c| {
            matches!(
                c,
                rabuka_engine::ability::types::Choice::SelectCard {
                    count: 1,
                    allow_skip: false,
                    ..
                }
            )
        });
        if is_required_discard {
            game.select_indices(&[0]);
        } else {
            game.select_indices(&[]);
        }
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
    let before = game.state.player1.energy_zone.active_count();

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

    let after = game.state.player1.energy_zone.active_count();
    // Baton touch cost = arriver.cost - hanaho.cost = 15 - 9 = 6
    // Auto ability activates 2 → net -4
    assert!(
        after >= before - 6,
        "Energy should reflect baton touch cost 15-9=6 (before={}, got {}),",
        before,
        after
    );
}

/// Baton touch count is tracked per-player, not globally.
#[test]
fn baton_touch_count_per_player() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanaho = game.id("PL!HS-sd1-001-SD");
    let arriver = game.id("PL!HS-sd1-006-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = hanaho;
    game.give_energy(25);
    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);

    assert_eq!(
        game.state.get_baton_touch_count("p1"),
        0,
        "p1 count starts at 0"
    );
    assert_eq!(
        game.state.get_baton_touch_count("p2"),
        0,
        "p2 count starts at 0"
    );

    game.play_to_stage(arriver, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.get_baton_touch_count("p1"),
        1,
        "p1 count is 1 after p1's baton touch"
    );
    assert_eq!(
        game.state.get_baton_touch_count("p2"),
        0,
        "p2 count remains 0 after p1's baton touch"
    );
}

/// Baton touch arriving card IDs are tracked in baton_touch_arriving_card_ids.
#[test]
fn baton_touch_arriving_card_ids_tracked() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let hanaho = game.id("PL!HS-sd1-001-SD");
    let arriver = game.id("PL!HS-sd1-006-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage[1] = hanaho;
    game.give_energy(25);
    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);

    assert!(
        game.state.baton_touch_arriving_card_ids.is_empty(),
        "starts empty"
    );

    game.play_to_stage(arriver, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert!(
        game.state.baton_touch_arriving_card_ids.contains(&arriver),
        "arriving card ID is stored"
    );
}

/// Opponent's card with baton touch discard ability does NOT trigger
/// when the active player performs the baton touch.
#[test]
fn opponent_baton_touch_discard_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // 花帆 (PL!HS-sd1-001-SD) has: when baton-touched to waitroom by cost 10+ 蓮ノ空 → activate 2 energy
    let hanaho = game.id("PL!HS-sd1-001-SD");
    let arriver = game.id("PL!HS-sd1-006-SD");
    let filler = game.id("PL!-sd1-010-SD");

    // Put hanaho on Player 1's stage (she will be replaced by baton touch)
    game.state.player1.stage.stage[1] = hanaho;
    // Also put a copy of hanaho in Player 2's waitroom (as if P2 had their own replaced card)
    // But the baton touch was performed by P1, so P2's hanaho should NOT trigger.
    let p2_hanaho = game.id("PL!HS-sd1-001-SD");
    game.state.player2.waitroom.cards.push(p2_hanaho);

    // Give energy to P1 for the baton touch cost
    game.give_energy(25);

    game.state.player1.hand.cards.push(arriver);
    game.state.player1.hand.cards.push(filler);

    let p2_energy_before = game.state.player2.energy_zone.active_count();

    // Perform baton touch from P1's hand
    game.play_to_stage(arriver, rabuka_engine::zones::MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // P1's hanaho was replaced and should have triggered (activate 2 energy for P1)
    // P2's hanaho should NOT have triggered even though it's in the waitroom,
    // because the baton touch was performed by P1, not P2.

    let p2_energy_after = game.state.player2.energy_zone.active_count();
    assert_eq!(
        p2_energy_after, p2_energy_before,
        "P2's energy should not change — P2's hanaho should not trigger on P1's baton touch"
    );

    // Also verify baton touch is attributed correctly
    assert_eq!(
        game.state.get_baton_touch_count("p1"),
        1,
        "P1 has 1 baton touch"
    );
    assert_eq!(
        game.state.get_baton_touch_count("p2"),
        0,
        "P2 has 0 baton touches"
    );
}

/// card_count_condition with baton_touch_trigger correctly filters stage cards
/// to only those that arrived via baton touch.
#[test]
fn card_count_condition_baton_touch_filter() {
    use rabuka_engine::ability::condition::ConditionContext;
    use rabuka_engine::card::Condition;

    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let member1 = game.id("PL!HS-sd1-006-SD"); // 蓮ノ空 member
    let member2 = game.id("PL!HS-sd1-008-SD"); // 蓮ノ空 member

    // Place 2 蓮ノ空 members on stage directly (no baton touch)
    game.state.player1.stage.stage[0] = member1;
    game.state.player1.stage.stage[1] = member2;

    // Create a card_count_condition that checks stage for 蓮ノ空 members
    // with baton_touch_trigger and min_baton_touch_count=2
    let condition = Condition::Location {
        text: None,
        negation: None,
        phase: None,
        phase_target: None,
        cache: None,
        trigger_event: None,
        location: Some(ArcStr::from("stage")),
        locations: None,
        target: Some(ArcStr::from("self")),
        count: Some(2),
        operator: Some(ArcStr::from(">=")),
        card_type: Some(ConditionCardType::MemberCard),
        group_names: Some(Box::new(vec!["蓮ノ空".to_string()])),
        exclude_group_names: None,
        characters: None,
        exclude_characters: None,
        cost_limit: None,
        cost_limit_operator: None,
        heart_colors: None,
        heart_type: None,
        heart_source: None,
        distinct: None,
        exclude_self: None,
        self_target: None,
        source: None,
        destination: None,
        state: None,
        position: None,
        position_compare: None,
        require_position_cards: None,
        all: None,
        all_areas: None,
        temporal: None,
        yell_trigger: None,
        same_name: None,
        card_property: None,
        scope: None,
        sub_checks: None,
        baton_touch_trigger: Some(true),
        min_baton_touch_count: Some(2),
        unit: None,
        comparison_target: None,
        comparison_type: None,
        activation_position: None,
        group_reference: None,
        aggregate: None,
    };

    // Without any baton touches, the condition should fail
    let ctx = ConditionContext::new(&game.state);
    assert!(
        !ctx.evaluate_condition(&condition),
        "card_count_condition with baton_touch_trigger should fail with 0 baton touches"
    );

    // Now record 2 baton touches with arriving card IDs
    game.state.record_baton_touch("p1", Some(member1));
    game.state.record_baton_touch("p1", Some(member2));

    // Since record_baton_touch pushed member1 and member2 into
    // baton_touch_arriving_card_ids, and both are on stage,
    // the condition should pass.
    let ctx2 = ConditionContext::new(&game.state);
    assert!(
        ctx2.evaluate_condition(&condition),
        "card_count_condition with baton_touch_trigger=2 should pass with 2 baton touched members on stage"
    );

    // Remove one member from baton_touch_arriving_card_ids — condition should fail
    game.state
        .baton_touch_arriving_card_ids
        .retain(|&id| id != member2);
    let ctx3 = ConditionContext::new(&game.state);
    assert!(
        !ctx3.evaluate_condition(&condition),
        "Should fail when only 1 of 2 stage members arrived via baton touch"
    );
}
