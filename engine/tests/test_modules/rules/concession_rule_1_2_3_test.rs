use rabuka_engine::ability::types::Choice;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::game_setup::{execute_action_for_player, Action, ActionType};
use rabuka_engine::game_state::{GameResult, GameState};
use rabuka_engine::player::Player;
use std::str::FromStr;
use std::sync::Arc;

fn game_state() -> GameState {
    let p1 = Player::new("player1".into(), "Player 1".into(), true);
    let p2 = Player::new("player2".into(), "Player 2".into(), false);
    GameState::new(p1, p2, Arc::new(CardDatabase::new()))
}

fn concede_action() -> Action {
    Action {
        description: "Concede".to_string(),
        description_ja: None,
        action_type: ActionType::Concede,
        parameters: None,
        selected: None,
    }
}

#[test]
fn concession_action_is_wire_representable() {
    assert_eq!(ActionType::Concede.to_string(), "concede");
    assert_eq!(ActionType::from_str("concede"), Ok(ActionType::Concede));
    assert_eq!(
        ActionType::from_tag(ActionType::Concede.to_tag()),
        ActionType::Concede
    );
}

#[test]
fn concession_gives_the_game_to_the_opponent() {
    let mut p1_concedes = game_state();
    p1_concedes.concede(0).unwrap();
    assert_eq!(p1_concedes.game_result, GameResult::SecondAttackerWins);
    assert!(p1_concedes.game_ended);

    let mut p2_concedes = game_state();
    p2_concedes.concede(1).unwrap();
    assert_eq!(p2_concedes.game_result, GameResult::FirstAttackerWins);
    assert!(p2_concedes.game_ended);
}

#[test]
fn concession_dispatch_bypasses_pending_choice_and_replacement() {
    let mut state = game_state();
    state.ability_queue.pause_for_choice(
        Choice::select_cards("hand", 1, "Choose a card", false)
            .picker(Some("player1".to_string()))
            .build(),
    );
    state.pending_success_replacement_card_id = Some(7);
    state.pending_success_replacement_player_id = Some("player1".to_string());
    assert!(!state.can_player_act(1));

    execute_action_for_player(&mut state, &concede_action(), 1).unwrap();

    assert_eq!(state.game_result, GameResult::FirstAttackerWins);
    assert!(!state.has_pending_choice());
    assert_eq!(state.pending_success_replacement_card_id, None);
    assert_eq!(state.pending_success_replacement_player_id, None);
}

#[test]
fn simultaneous_concessions_produce_a_draw() {
    let mut state = game_state();
    state.concede_both().unwrap();
    assert_eq!(state.game_result, GameResult::Draw);
    assert!(state.game_ended);
}

#[test]
fn concession_is_rejected_after_the_game_ends() {
    let mut state = game_state();
    state.game_result = GameResult::Draw;
    state.game_ended = true;

    assert!(state.concede(0).is_err());
    assert!(state.concede(1).is_err());
    assert!(state.concede_both().is_err());
    assert_eq!(state.game_result, GameResult::Draw);
}
