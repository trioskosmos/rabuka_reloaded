use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;

use crate::helpers::{load_real_database, TestGame};

#[test]
fn repeated_action_protocol_routes_to_non_active_player_and_stops_only_after_exact_repetition() {
    let mut game = TestGame::new(load_real_database());
    let action = ActionType::Pass;

    game.state.record_action_boundary(action);
    game.state.record_action_boundary(action);
    assert!(!game.state.has_pending_choice());

    game.state.record_action_boundary(action);
    assert!(game.state.has_pending_choice());
    assert_eq!(game.state.get_pending_choice_player_id().as_deref(), Some("p2"));
    assert!(!game.state.game_ended);

    TurnEngine::resume_with_choice(&mut game.state, Some(0), None).unwrap();
    assert!(game.state.game_ended);
    assert_eq!(game.state.game_result, rabuka_engine::game_state::GameResult::Draw);
}

#[test]
fn protocol_continuation_and_different_action_clear_the_repetition_path() {
    let mut game = TestGame::new(load_real_database());
    let action = ActionType::Pass;
    game.state.record_action_boundary(action);
    game.state.record_action_boundary(action);
    game.state.record_action_boundary(action);

    TurnEngine::resume_with_choice(&mut game.state, Some(1), None).unwrap();
    assert!(!game.state.has_pending_choice());
    assert!(!game.state.game_ended);

    game.state.record_action_boundary(ActionType::PlayMemberToStage);
    game.state.record_action_boundary(ActionType::PlayMemberToStage);
    assert!(!game.state.has_pending_choice());
}

#[test]
fn fingerprint_distinguishes_ordered_card_ids_with_equal_zone_lengths() {
    let mut game = TestGame::new(load_real_database());
    let a = game.id("PL!N-bp1-001-R");
    let b = game.id("PL!N-bp1-001-R");
    game.state.player1.hand.cards.push(a);

    game.state.record_action_boundary(ActionType::Pass);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(b);
    game.state.record_action_boundary(ActionType::Pass);
    game.state.player1.hand.cards.clear();
    game.state.player1.hand.cards.push(a);
    game.state.record_action_boundary(ActionType::Pass);

    assert!(!game.state.has_pending_choice());
}
