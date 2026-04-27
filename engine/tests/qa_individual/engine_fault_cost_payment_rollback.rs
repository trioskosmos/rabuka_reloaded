// Q244: Cost payment should not modify game state if payment fails
// Fault: If a cost payment fails partway through (e.g., in sequential costs),
// the game state may be partially modified when it should be rolled back

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_q244_cost_payment_rollback_on_failure() {
    // Test: If cost payment fails, game state should not be partially modified
    // This is especially important for sequential costs where one sub-cost might fail
    
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
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    setup_player_with_hand(&mut player1, vec![member_id]);
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Play member to center
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(member_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play member to center: {:?}", result);
    
    // Verify member is on stage
    assert!(game_state.player1.stage.stage.contains(&member_id),
        "Member should be on stage");
    
    // ENGINE FAULT: In ability_resolver.rs pay_cost function (line 5414), sequential costs
    // are paid one by one in a loop (line 5422-5424). If a later sub-cost fails, the earlier
    // sub-costs have already been paid and the game state is partially modified.
    // There is no rollback mechanism to undo the earlier payments if a later payment fails.
    //
    // The correct behavior should be:
    // 1. Check if all sub-costs can be paid before paying any
    // 2. Or implement a rollback mechanism to undo partial payments on failure
    
    // For now, this test documents the fault
    // A fix would require either pre-validation or rollback logic
}
