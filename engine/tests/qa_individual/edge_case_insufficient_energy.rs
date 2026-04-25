use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_edge_case_insufficient_energy() {
    // Edge case: Try to play a member card with insufficient energy
    // Expected: Should fail or handle gracefully
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card with high cost
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 5)
        .nth(0)
        .expect("Should have member card with cost > 5");
    let member_card_id = get_card_id(member_card, &card_database);
    
    // Set up player with the card but NO energy
    setup_player_with_hand(&mut player1, vec![member_card_id]);
    // No energy setup - intentionally empty
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Try to play the card with insufficient energy
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_card_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    // Verify the play failed
    assert!(result.is_err(), "Should fail to play member with insufficient energy: {:?}", result);
    
    // Verify card is still in hand
    assert!(game_state.player1.hand.cards.contains(&member_card_id),
        "Card should still be in hand when play fails");
    
    // Verify stage is still empty
    assert!(!game_state.player1.stage.stage.contains(&member_card_id),
        "Card should not be on stage when play fails");
    
    println!("Edge case test: Insufficient energy handled correctly");
}
