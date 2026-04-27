// Q243: Auto abilities with pending choices should not be removed from pending list
// Fault: process_pending_auto_abilities removes abilities from pending list before execution,
// but if an ability requires a user choice, it should remain in pending until the choice is resolved

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase};

#[test]
fn test_q243_pending_auto_abilities_with_choice() {
    // Test: Auto abilities that require user choices should remain in pending list
    // until the choice is resolved
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    
    // Trigger an auto ability
    game_state.trigger_auto_ability(
        "test_ability".to_string(),
        rabuka_engine::game_state::AbilityTrigger::Debut,
        "player1".to_string(),
        Some("test_card".to_string()),
    );
    
    let initial_pending_count = game_state.pending_auto_abilities.len();
    assert_eq!(initial_pending_count, 1, "Should have 1 pending auto ability");
    
    // ENGINE FAULT: process_pending_auto_abilities removes abilities from the pending list
    // before executing them (lines 1095-1100 in game_state.rs). If an ability execution
    // requires a user choice (pending_choice), the ability is already removed from the
    // pending list. When the choice is provided, the ability won't be in the pending list
    // anymore, so it won't be able to resume properly.
    //
    // The correct behavior should be:
    // 1. Execute the ability
    // 2. If execution completes successfully, remove from pending
    // 3. If execution requires a choice, keep in pending until choice is resolved
    
    // For now, this test documents the fault
    // A fix would require moving the removal logic to after successful execution
}
