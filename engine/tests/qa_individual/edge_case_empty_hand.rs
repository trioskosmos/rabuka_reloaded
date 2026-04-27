use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_edge_case_empty_hand() {
    // Edge case: Try to play a card when hand is empty
    // Expected: Should fail gracefully
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Set up player with energy but NO cards in hand
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(10)
        .collect();
    setup_player_with_energy(&mut player1, energy_card_ids);
    // No hand setup - intentionally empty
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Try to play a card that doesn't exist in hand
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(9999), // Invalid card ID
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    // Verify the play failed
    assert!(result.is_err(), "Should fail to play non-existent card: {:?}", result);
    
    println!("Edge case test: Empty hand handled correctly");
}
