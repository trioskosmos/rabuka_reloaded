// Stress test: Baton touch with multiple members and energy edge cases
// This tests complex baton touch scenarios with multiple members on stage

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_baton_touch_multiple_members() {
    // Stress test: Baton touch with 3 different members, energy edge cases
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find 3 member cards with different costs
    let member1 = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) == 1)
        .nth(0)
        .expect("Should have member card with cost 1");
    let member1_id = get_card_id(member1, &card_database);
    
    let member2 = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != member1_id)
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) == 2)
        .nth(0)
        .expect("Should have member card with cost 2");
    let member2_id = get_card_id(member2, &card_database);
    
    let member3 = cards.iter()
        .filter(|c| c.is_member() && get_card_id(c, &card_database) != member1_id && get_card_id(c, &card_database) != member2_id)
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) == 3)
        .nth(0)
        .expect("Should have member card with cost 3");
    let member3_id = get_card_id(member3, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(50)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member1_id, member2_id, member3_id]);
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play member1 (cost 1) to center
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member1_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member1 to center: {:?}", result);
    
    // Advance turn
    game_state.turn_number = 2;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Play member2 (cost 2) to left side
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member2_id),
        None,
        Some(MemberArea::LeftSide),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member2 to left side: {:?}", result);
    
    // Advance turn
    game_state.turn_number = 3;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Play member3 (cost 3) to right side
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::PlayMemberToStage,
        Some(member3_id),
        None,
        Some(MemberArea::RightSide),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member3 to right side: {:?}", result);
    
    // Verify all 3 members are on stage
    let stage_members = game_state.player1.stage.stage.iter().filter(|&&id| *id != -1).count();
    assert_eq!(stage_members, 3, "Stage should have 3 members");
    
    // Advance turn to allow baton touch
    game_state.turn_number = 4;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    game_state.player1.areas_locked_this_turn.clear();
    
    // Add member3 back to hand for baton touch
    game_state.player1.hand.cards.push(member3_id);
    
    // Baton touch center (cost 1) with member3 (cost 3) - should pay 2 energy
    let initial_active_energy = game_state.player1.energy_zone.active_energy_count;
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &ActionType::BatonTouch,
        Some(member3_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should baton touch center with higher cost member: {:?}", result);
    
    // Verify energy was paid (cost 3 - cost 1 = 2 energy)
    assert_eq!(game_state.player1.energy_zone.active_energy_count, initial_active_energy - 2,
        "Should have paid 2 energy for baton touch (3 - 1 = 2)");
    
    // Verify center now has member3
    assert_eq!(game_state.player1.stage.stage[1], member3_id,
        "Center should have member3 after baton touch");
    
    println!("Stress test passed: Baton touch with multiple members and energy calculation");
}
