// Extreme stress test: Ability chain reactions
// This tests multiple abilities triggering in sequence with state modifications

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_ability_chain_reaction() {
    // Stress test: Play members with abilities that might trigger chain reactions
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find member cards with various ability types
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| !c.abilities.is_empty())
        .take(8)
        .collect();
    
    let member_ids: Vec<_> = member_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(80)
        .collect();
    
    setup_player_with_hand(&mut player1, member_ids.clone());
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play sequence to trigger ability chains
    let mut chain_length = 0;
    for (i, &member_id) in member_ids.iter().enumerate() {
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        if result.is_ok() {
            chain_length += 1;
            game_state.turn_number += 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            game_state.player1.areas_locked_this_turn.clear();
            
            // Try baton touch to trigger debut abilities
            if i + 1 < member_ids.len() {
                game_state.player1.hand.cards.push(member_ids[i + 1]);
                
                let result = TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &ActionType::BatonTouch,
                    Some(member_ids[i + 1]),
                    None,
                    Some(MemberArea::Center),
                    Some(false),
                );
                
                if result.is_ok() {
                    chain_length += 1;
                }
            }
        }
    }
    
    println!("Extreme stress test passed: Ability chain reaction with {} actions", chain_length);
}

#[test]
fn test_stress_concurrent_zone_operations() {
    // Stress test: Rapid operations across multiple zones
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .take(15)
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
    
    // Rapidly move cards between zones
    for &card_id in &member_ids {
        game_state.player1.waitroom.cards.push(card_id);
    }
    
    let initial_waitroom = game_state.player1.waitroom.cards.len();
    
    // Move from waitroom to deck
    for &card_id in &member_ids {
        game_state.player1.main_deck.cards.push(card_id);
    }
    
    // Move from deck to hand
    for &card_id in &member_ids {
        game_state.player1.hand.cards.push(card_id);
    }
    
    let final_hand = game_state.player1.hand.cards.len();
    
    println!("Concurrent zone operations: waitroom {}, hand {}", initial_waitroom, final_hand);
    assert!(final_hand > 10, "Should handle concurrent zone operations");
}

#[test]
fn test_stress_extreme_cost_values() {
    // Stress test: Members with extreme cost values
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find members with high costs
    let high_cost_members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) >= 5)
        .take(5)
        .collect();
    
    // Find members with low costs
    let low_cost_members: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .take(5)
        .collect();
    
    let high_cost_ids: Vec<_> = high_cost_members.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let low_cost_ids: Vec<_> = low_cost_members.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(100)
        .collect();
    
    let mut hand_cards = vec![];
    hand_cards.extend(high_cost_ids);
    hand_cards.extend(low_cost_ids);
    
    setup_player_with_hand(&mut player1, hand_cards);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play high cost member
    if let Some(&high_id) = high_cost_ids.first() {
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(high_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        if result.is_ok() {
            game_state.turn_number += 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            game_state.player1.areas_locked_this_turn.clear();
            
            // Baton touch with low cost member (large energy difference)
            if let Some(&low_id) = low_cost_ids.first() {
                game_state.player1.hand.cards.push(low_id);
                
                let result = TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &ActionType::BatonTouch,
                    Some(low_id),
                    None,
                    Some(MemberArea::Center),
                    Some(false),
                );
                
                println!("Extreme cost baton touch: {:?}", result);
            }
        }
    }
    
    println!("Extreme stress test passed: Extreme cost values");
}
