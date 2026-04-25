// Stress test: Energy management edge cases
// This tests energy exhaustion, refresh, and boundary conditions

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_energy_exhaustion() {
    // Stress test: Play members until energy is exhausted, then try more actions
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find high cost member cards
    let high_cost_members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) >= 3)
        .take(5)
        .collect();
    
    let member_ids: Vec<_> = high_cost_members.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(20)
        .collect();
    
    setup_player_with_hand(&mut player1, member_ids.clone());
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    let initial_energy = game_state.player1.energy_zone.active_energy_count;
    
    // Play members until energy is exhausted
    let mut played_count = 0;
    for (i, &member_id) in member_ids.iter().enumerate() {
        if i >= 3 { break; } // Only try to play 3 members
        
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        if result.is_ok() {
            played_count += 1;
            game_state.turn_number += 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            game_state.player1.areas_locked_this_turn.clear();
        }
    }
    
    // Try to play another member with insufficient energy - should fail
    if member_ids.len() > 3 {
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_ids[3]),
            None,
            Some(MemberArea::LeftSide),
            Some(false),
        );
        assert!(result.is_err(), "Should fail with insufficient energy: {:?}", result);
    }
    
    println!("Stress test passed: Energy exhaustion after playing {} members", played_count);
}

#[test]
fn test_stress_energy_zero_boundary() {
    // Stress test: Try to play member with exactly 0 energy
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0)
        .nth(0)
        .expect("Should have member card with cost > 0");
    let member_id = get_card_id(member_card, &card_database);
    
    // Setup with NO energy
    setup_player_with_hand(&mut player1, vec![member_id]);
    // Don't add any energy
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Try to play member with 0 energy - should fail
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_err(), "Should fail with 0 energy: {:?}", result);
    
    println!("Stress test passed: Cannot play member with 0 energy");
}
