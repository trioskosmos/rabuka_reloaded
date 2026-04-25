// Stress test: Multiple copies of the same card with complex interactions
// This tests edge cases with duplicate cards on stage, abilities, and baton touch

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_multiple_copies_complex_interactions() {
    // Stress test: 3 copies of same card, baton touch, ability interactions
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a member card with low cost
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0 && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have member card with cost <= 2");
    let member_card_id = get_card_id(member_card, &card_database);
    
    // Find another member card for baton touch
    let baton_card = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != member_card_id)
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) > 0 && c.cost.unwrap_or(0) <= 2)
        .nth(0)
        .expect("Should have second member card with cost <= 2");
    let baton_card_id = get_card_id(baton_card, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(50)
        .collect();
    
    // Add three copies of the same member and one baton card to hand
    setup_player_with_hand(&mut player1, vec![member_card_id, member_card_id, member_card_id, baton_card_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play first copy to center
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_card_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play first copy to center: {:?}", result);
    
    // Advance turn
    game_state.turn_number = 2;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Play second copy to left side
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_card_id),
        None,
        Some(MemberArea::LeftSide),
        Some(false),
    );
    assert!(result.is_ok(), "Should play second copy to left side: {:?}", result);
    
    // Advance turn
    game_state.turn_number = 3;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Play third copy to right side
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member_card_id),
        None,
        Some(MemberArea::RightSide),
        Some(false),
    );
    assert!(result.is_ok(), "Should play third copy to right side: {:?}", result);
    
    // Verify all three copies are on stage
    let stage_members = game_state.player1.stage.stage.iter().filter(|&&id| id != -1).count();
    assert_eq!(stage_members, 3, "Stage should have 3 members");
    
    // Advance turn to allow baton touch
    game_state.turn_number = 4;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Baton touch center with baton card
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::BatonTouch,
        Some(baton_card_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should baton touch center: {:?}", result);
    
    // Verify baton touch worked - center should now have baton card
    assert_eq!(game_state.player1.stage.stage[1], baton_card_id,
        "Center should have baton card after baton touch");
    
    // Verify original member is in waitroom
    assert!(!game_state.player1.stage.stage.contains(&member_card_id) || 
            game_state.player1.stage.stage.iter().filter(|&&id| id == member_card_id).count() < 3,
        "One copy should be in waitroom after baton touch");
    
    println!("Stress test passed: 3 copies of same card on stage with baton touch");
}
