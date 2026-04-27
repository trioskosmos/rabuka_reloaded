// Stress test: Live card interactions edge cases
// This tests multiple live cards, heart conditions, and live outcomes

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_multiple_live_cards() {
    // Stress test: Multiple live cards in live card zone
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find live cards
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .take(3)
        .collect();
    
    let live_card_ids: Vec<_> = live_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    setup_player_with_hand(&mut player1, live_card_ids.clone());
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::LiveCardSet;
    game_state.turn_number = 1;
    
    // Add multiple live cards to live card zone
    for &live_id in &live_card_ids {
        game_state.player1.live_card_zone.add_card(live_id, false, &card_database)
            .expect("Failed to add live card");
    }
    
    // Verify multiple live cards are in zone
    assert_eq!(game_state.player1.live_card_zone.cards.len(), live_card_ids.len(),
        "Should have all live cards in live card zone");
    
    println!("Stress test passed: Multiple live cards in live card zone");
}

#[test]
fn test_stress_live_card_with_member_conditions() {
    // Stress test: Live card with specific member conditions on stage
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card
    let live_card = cards.iter()
        .filter(|c| c.is_live())
        .nth(0)
        .expect("Should have live card");
    let live_card_id = get_card_id(live_card, &card_database);
    
    // Find member cards
    let member1 = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have member card");
    let member1_id = get_card_id(member1, &card_database);
    
    let member2 = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != member1_id)
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have second member card");
    let member2_id = get_card_id(member2, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![live_card_id, member1_id, member2_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play members to stage
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member1_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member1 to center: {:?}", result);
    
    game_state.turn_number = 2;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member2_id),
        None,
        Some(MemberArea::LeftSide),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member2 to left side: {:?}", result);
    
    // Verify both members are on stage
    let stage_members = game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count();
    assert_eq!(stage_members, 2, "Stage should have 2 members");
    
    // Add live card to live card zone
    game_state.player1.live_card_zone.add_card(live_card_id, false, &card_database)
        .expect("Failed to add live card");
    
    // Verify live card is in zone
    assert!(game_state.player1.live_card_zone.cards.contains(&live_card_id),
        "Live card should be in live card zone");
    
    println!("Stress test passed: Live card with members on stage");
}
