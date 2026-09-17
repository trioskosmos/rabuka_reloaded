use crate::helpers::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn wait_member_self_cost_grants_one_total_score_without_waiting_opponent() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    let opponent = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [-1, source, -1];
    game.state.player2.stage.stage = [-1, opponent, -1];

    game.activate_ability(source);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [-1, source, -1]);
    assert_eq!(game.state.player2.stage.stage, [-1, opponent, -1]);
    assert_eq!(game.state.mods.get_orientation_modifier(source), Some("wait"));
    assert_eq!(game.state.mods.get_orientation_modifier(opponent), None);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
}

#[test]
fn wait_member_baton_removes_recipient_and_immediately_loses_total_score() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    let recipient = game.id("PL!-sd1-010-SD");
    let replacement = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, replacement);
    game.state.player1.stage.stage = [recipient, source, -1];
    game.add_to_hand(replacement);
    game.give_energy(4);

    game.activate_ability(source);
    game.assert_select_card("stage", 1, false);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_orientation_modifier(recipient), Some("wait"));
    assert_eq!(game.state.mods.get_orientation_modifier(source), None);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(replacement),
        None,
        Some(MemberArea::LeftSide),
        Some(true),
    )
    .expect("Baton touch must replace the waited recipient");

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.current_phase, Phase::Main);
    assert_eq!(game.state.player1.stage.stage, [replacement, source, -1]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[recipient]);
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
}

#[test]
fn wait_member_baton_removes_source_but_recipient_keeps_total_score() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    let recipient = game.id("PL!-sd1-010-SD");
    let replacement = game.id("PL!-sd1-010-SD");
    fill_decks(&mut game, replacement);
    game.state.player1.stage.stage = [recipient, source, -1];
    game.add_to_hand(replacement);
    game.give_energy(4);

    game.activate_ability(source);
    game.assert_select_card("stage", 1, false);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.get_orientation_modifier(recipient), Some("wait"));
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);

    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(replacement),
        None,
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("Baton touch must replace the activating source");

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.current_phase, Phase::Main);
    assert_eq!(game.state.player1.stage.stage, [recipient, replacement, -1]);
    assert_eq!(game.state.player1.waitroom.cards.as_slice(), &[source]);
    assert!(game.state.player1.hand.cards.is_empty());
    assert_eq!(game.state.mods.get_orientation_modifier(recipient), Some("wait"));
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
}

#[test]
fn wait_member_total_score_persists_through_victory_and_expires_after_live() {
    assert_live_end_expiry(true);
}

#[test]
fn wait_member_total_score_expires_at_live_end_even_without_setting_live() {
    assert_live_end_expiry(false);
}

fn assert_live_end_expiry(set_live: bool) {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD");
    let live = game.id("PL!-sd1-019-SD");
    fill_decks(&mut game, filler);
    fill_energy_deck(&mut game, 0, 4);
    fill_energy_deck(&mut game, 1, 4);
    game.state.player1.stage.stage = [-1, source, -1];
    if set_live {
        game.add_to_hand(live);
    }

    game.activate_ability(source);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.current_phase, Phase::Main);

    for expected in [
        Phase::Active,
        Phase::Energy,
        Phase::Draw,
        Phase::Main,
        Phase::LiveCardSetFirstAttacker,
    ] {
        game.pass();
        assert!(!game.has_pending_choice());
        assert_eq!(game.state.current_phase, expected);
        assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    }
    if set_live {
        game.set_live_card(live);
        assert!(!game.has_pending_choice());
        assert_eq!(game.state.current_phase, Phase::LiveCardSetFirstAttacker);
    }

    for expected in [
        Phase::LiveCardSetSecondAttacker,
        Phase::FirstAttackerPerformance,
        Phase::SecondAttackerPerformance,
        Phase::LiveVictoryDetermination,
    ] {
        game.pass();
        assert!(!game.has_pending_choice());
        assert_eq!(game.state.current_phase, expected);
        assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    }

    game.pass();

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.current_phase, Phase::Active);
    assert_eq!(game.state.turn_number, 2);
    assert_eq!(game.state.player1.stage.stage, [-1, source, -1]);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
}

#[test]
fn wait_member_outside_center_cannot_pay_cost_or_gain_total_score() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    game.state.player1.stage.stage = [source, -1, -1];

    let _result = game.try_activate_ability(source);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [source, -1, -1]);
    assert_eq!(game.state.mods.get_orientation_modifier(source), None);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0);
}

#[test]
fn wait_member_second_activation_cannot_wait_another_member_or_stack_total_score() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    let other = game.id("PL!-sd1-010-SD");
    game.state.player1.stage.stage = [other, source, -1];

    game.activate_ability(source);
    game.assert_select_card("stage", 1, false);
    game.select_indices(&[0]);
    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.mods.get_orientation_modifier(other), Some("wait"));
    assert_eq!(game.state.mods.get_orientation_modifier(source), None);

    let _result = game.try_activate_ability(source);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [other, source, -1]);
    assert_eq!(game.state.mods.get_orientation_modifier(other), Some("wait"));
    assert_eq!(game.state.mods.get_orientation_modifier(source), None);
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
}

#[test]
fn wait_member_selected_cost_target_stays_on_stage_without_discarding() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    let other = game.id("PL!S-bp2-001-R");
    game.state.player1.stage.stage = [other, source, -1];

    game.activate_ability(source);
    game.assert_select_card("stage", 1, false);
    game.select_indices(&[0]);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [other, source, -1]);
    assert_eq!(game.state.mods.get_orientation_modifier(other), Some("wait"));
    assert_eq!(game.state.mods.get_orientation_modifier(source), None);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
}

#[test]
fn wait_member_selected_cost_target_immediately_grants_exactly_one_total_score() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    let other = game.id("PL!S-bp2-001-R");
    game.state.player1.stage.stage = [other, source, -1];

    game.activate_ability(source);
    game.assert_select_card("stage", 1, false);
    game.select_indices(&[0]);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
    assert_eq!(game.state.mods.p2_constant_total_score_bonus, 0);
}

#[test]
fn wait_member_right_target_waits_without_affecting_unselected_members() {
    let mut game = TestGame::new(load_real_database());
    let source = game.id("PL!S-bp3-001-R\u{ff0b}");
    let left = game.id("PL!S-bp2-001-R");
    let right = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [left, source, right];

    game.activate_ability(source);
    game.assert_select_card("stage", 1, false);
    game.select_indices(&[2]);

    assert!(!game.has_pending_choice());
    assert_eq!(game.state.player1.stage.stage, [left, source, right]);
    assert_eq!(game.state.mods.get_orientation_modifier(right), Some("wait"));
    assert_eq!(game.state.mods.get_orientation_modifier(left), None);
    assert_eq!(game.state.mods.get_orientation_modifier(source), None);
    assert!(game.state.player1.waitroom.cards.is_empty());
    assert_eq!(game.state.mods.p1_constant_total_score_bonus, 1);
}
