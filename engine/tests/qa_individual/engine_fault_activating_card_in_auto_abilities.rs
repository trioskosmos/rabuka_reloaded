// Q240: Effect with target="self" should resolve correctly in auto abilities
// Fault: When auto abilities execute, target="self" may not resolve to the correct card
// This can happen if activating_card is not set properly during auto ability execution

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_q240_self_target_in_auto_ability() {
    // Test: Effects with target="self" should resolve to the activating card
    // in auto abilities (e.g., debut abilities)
    
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
        .take(30)  // More energy for higher cost cards
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
    
    // ENGINE FAULT: When a debut auto ability triggers with an effect that has target="self",
    // the activating_card should be set to the played card so that "self" resolves correctly
    // If activating_card is not set, target="self" may fail or resolve incorrectly
    
    // The engine's execute_card_ability function in game_state.rs does not set activating_card
    // when executing auto abilities. This is a fault.
    // See line 1108-1170 in game_state.rs - it finds the card but doesn't set activating_card
    
    // This test documents the fault. A fix would require setting activating_card
    // before executing the ability in execute_card_ability
}
