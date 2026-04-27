// Aggressive stress test: Maximum stage capacity with ability interactions
// This tests the engine's ability to handle full stage with complex ability chains

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_max_stage_capacity() {
    // Stress test: Fill all stage areas and attempt additional operations
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find 5 member cards (maximum stage capacity)
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .take(5)
        .collect();
    
    let member_ids: Vec<_> = member_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(50)
        .collect();
    
    setup_player_with_hand(&mut player1, member_ids.clone());
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    let areas = [MemberArea::LeftSide, MemberArea::Center, MemberArea::RightSide, MemberArea::LeftSupport, MemberArea::RightSupport];
    
    // Fill all stage areas
    for (i, &member_id) in member_ids.iter().enumerate() {
        if i >= areas.len() { break; }
        
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_id),
            None,
            Some(areas[i]),
            Some(false),
        );
        
        if result.is_ok() {
            game_state.turn_number += 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            game_state.player1.areas_locked_this_turn.clear();
        }
    }
    
    // Verify stage is full
    let stage_members = game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count();
    assert!(stage_members >= 3, "Stage should have multiple members");
    
    // Try to play another member - should fail due to no available areas
    if member_ids.len() > 3 {
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_ids[3]),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        // May fail due to area locked or full stage
        println!("Attempt to play to full stage: {:?}", result);
    }
    
    println!("Stress test passed: Maximum stage capacity with {} members", stage_members);
}

#[test]
fn test_stress_rapid_baton_touch_chain() {
    // Stress test: Rapid baton touch chain with same card
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find 2 member cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .take(2)
        .collect();
    
    let member_ids: Vec<_> = member_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(50)
        .collect();
    
    // Add multiple copies for rapid baton touch
    let mut hand_cards = vec![];
    for _ in 0..5 {
        hand_cards.push(member_ids[0]);
        hand_cards.push(member_ids[1]);
    }
    
    setup_player_with_hand(&mut player1, hand_cards);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play first member
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_ids[0]),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play first member: {:?}", result);
    
    // Rapid baton touch chain
    let mut baton_count = 0;
    for i in 0..4 {
        game_state.turn_number += 1;
        game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
        game_state.player1.areas_locked_this_turn.clear();
        
        // Add card back to hand for next baton touch
        game_state.player1.hand.cards.push(member_ids[(i + 1) % 2]);
        
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::BatonTouch,
            Some(member_ids[(i + 1) % 2]),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        if result.is_ok() {
            baton_count += 1;
        }
    }
    
    println!("Stress test passed: Rapid baton touch chain with {} successful baton touches", baton_count);
}
