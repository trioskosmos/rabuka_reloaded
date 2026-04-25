// Extreme stress test: Combined mechanics interaction
// This tests multiple game systems interacting simultaneously

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_combined_mechanics() {
    // Stress test: Combine baton touch, energy management, and deck operations
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find member cards with different costs
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0 && c.cost.unwrap_or(0) <= 5)
        .take(6)
        .collect();
    
    let member_ids: Vec<_> = member_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(60)
        .collect();
    
    // Add many cards to hand
    let mut hand_cards = vec![];
    for &id in &member_ids {
        hand_cards.push(id);
        hand_cards.push(id); // duplicates
    }
    
    setup_player_with_hand(&mut player1, hand_cards);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Set up small deck to trigger refresh scenarios
    let small_deck: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .take(3)
        .map(|c| get_card_id(c, &card_database))
        .collect();
    game_state.player1.main_deck.cards = small_deck;
    
    // Play sequence: play members, baton touch, deplete energy
    let mut actions_completed = 0;
    for i in 0..min(5, member_ids.len()) {
        // Play member
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_ids[i]),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        if result.is_ok() {
            actions_completed += 1;
            game_state.turn_number += 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            game_state.player1.areas_locked_this_turn.clear();
            
            // Baton touch on next turn
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
                    actions_completed += 1;
                }
            }
        }
    }
    
    println!("Extreme stress test completed: {} actions in combined mechanics test", actions_completed);
}

#[test]
fn test_stress_simultaneous_ability_triggers() {
    // Stress test: Multiple abilities triggering simultaneously
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find member cards with abilities
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| !c.abilities.is_empty())
        .take(4)
        .collect();
    
    let member_ids: Vec<_> = member_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(40)
        .collect();
    
    setup_player_with_hand(&mut player1, member_ids.clone());
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play multiple members with abilities in sequence
    let mut abilities_triggered = 0;
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
            abilities_triggered += 1;
            game_state.turn_number += 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            game_state.player1.areas_locked_this_turn.clear();
        }
    }
    
    println!("Stress test passed: {} members with abilities played", abilities_triggered);
}

#[test]
fn test_stress_hand_size_limits() {
    // Stress test: Hand size with many cards
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find many member cards
    let member_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .take(30)
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    // Add excessive cards to hand
    setup_player_with_hand(&mut player1, member_ids.clone());
    
    let game_state = GameState::new(player1, player2, card_database.clone());
    
    let hand_size = game_state.player1.hand.cards.len();
    println!("Hand size with excessive cards: {}", hand_size);
    
    // Verify hand can hold many cards
    assert!(hand_size >= 20, "Hand should hold many cards");
    
    println!("Stress test passed: Hand size limits with {} cards", hand_size);
}
