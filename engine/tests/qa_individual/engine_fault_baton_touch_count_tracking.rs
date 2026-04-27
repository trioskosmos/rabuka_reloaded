// Q238: Baton touch count should be tracked correctly
// Fault: Engine may not properly increment baton_touch_count when baton touch occurs

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_q238_baton_touch_count_tracking() {
    // Test: Baton touch count should increment when baton touch is used
    // This tests the engine's baton touch tracking for condition evaluation
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card with cost
    let member = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0)
        .nth(0)
        .expect("Should have member card with cost > 0");
    let member_id = get_card_id(member, &card_database);
    
    // Find another member card for baton touch
    let member2 = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != member_id)
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0)
        .nth(0)
        .expect("Should have second member card");
    let member2_id = get_card_id(member2, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(20)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member_id, member2_id]);
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Verify initial baton touch count is 0
    assert_eq!(game_state.get_baton_touch_count(), 0,
        "Initial baton touch count should be 0");
    
    // Play member to center (NOT baton touch)
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false), // NOT baton touch
    );
    assert!(result.is_ok(), "Should play member to center: {:?}", result);
    
    // Verify baton touch count is still 0 (no baton touch used)
    assert_eq!(game_state.get_baton_touch_count(), 0,
        "Baton touch count should still be 0 after normal play");
    
    // Advance turn to allow baton touch
    game_state.turn_number = 2;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Activate all energy for the baton touch
    game_state.player1.activate_all_energy();
    
    // Baton touch to replace member1 with member2
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(member2_id),
        None,
        Some(MemberArea::Center),
        Some(true), // use baton touch
    );
    
    // ENGINE FAULT: The engine may not increment baton_touch_count
    // This should increment the count when baton touch is used
    assert!(result.is_ok(), "Should perform baton touch: {:?}", result);
    
    // Verify baton touch count is now 1
    assert_eq!(game_state.get_baton_touch_count(), 1,
        "Baton touch count should be 1 after baton touch");
    
    // Verify member1 is no longer on stage
    assert!(!game_state.player1.stage.stage.contains(&member_id),
        "Member1 should not be on stage after baton touch");
    
    // Verify member2 is on stage
    assert!(game_state.player1.stage.stage.contains(&member2_id),
        "Member2 should be on stage after baton touch");
}
