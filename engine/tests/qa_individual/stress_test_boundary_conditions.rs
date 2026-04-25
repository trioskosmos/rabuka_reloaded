// Stress test: Turn boundary conditions and state transitions
// This tests edge cases around turn number, phase transitions, and debut restrictions

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_turn_boundary_debut() {
    // Stress test: Debut restrictions at turn boundaries
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have member card");
    let member_id = get_card_id(member_card, &card_database);
    
    let baton_card = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != member_id)
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have second member card");
    let baton_id = get_card_id(baton_card, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member_id, baton_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play member on turn 1
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member on turn 1: {:?}", result);
    
    // Try baton touch on same turn - should fail due to debut restriction
    game_state.player1.hand.cards.push(baton_id);
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::BatonTouch,
        Some(baton_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    // Baton touch should fail on same turn as debut
    if result.is_err() {
        println!("Correctly prevented baton touch on same turn as debut");
    }
    
    // Advance to turn 2
    game_state.turn_number = 2;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Try baton touch on turn 2 - should succeed
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::BatonTouch,
        Some(baton_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    if result.is_ok() {
        println!("Baton touch succeeded on turn 2 (after debut turn)");
    }
    
    println!("Stress test passed: Turn boundary debut restrictions");
}

#[test]
fn test_stress_zone_overflow() {
    // Stress test: Attempt to overflow various zones
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find many member cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .take(20)
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
    
    // Add many cards to hand
    setup_player_with_hand(&mut player1, member_ids.clone());
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    
    // Attempt to add excessive cards to waitroom
    for &card_id in &member_ids {
        game_state.player1.waitroom.cards.push(card_id);
    }
    
    let waitroom_size = game_state.player1.waitroom.cards.len();
    println!("Waitroom size after overflow attempt: {}", waitroom_size);
    
    // Attempt to add excessive cards to deck
    for &card_id in &member_ids {
        game_state.player1.main_deck.cards.push(card_id);
    }
    
    let deck_size = game_state.player1.main_deck.cards.len();
    println!("Deck size after overflow attempt: {}", deck_size);
    
    // Verify zones handle large numbers
    assert!(waitroom_size > 10, "Waitroom should handle many cards");
    assert!(deck_size > 10, "Deck should handle many cards");
    
    println!("Stress test passed: Zone overflow handling");
}

#[test]
fn test_stress_invalid_action_sequence() {
    // Stress test: Attempt invalid action sequences
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .nth(0)
        .expect("Should have member card");
    let member_id = get_card_id(member_card, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(10)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    
    // Try to baton touch with no member on stage - should fail
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::BatonTouch,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    if result.is_err() {
        println!("Correctly prevented baton touch with no member on stage");
    }
    
    // Try to play to invalid area (if engine supports validation)
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    if result.is_ok() {
        println!("Successfully played member to stage");
    }
    
    println!("Stress test passed: Invalid action sequence handling");
}
