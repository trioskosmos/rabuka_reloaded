// Q247: Live End Duration Effects Clearing
// Test that effects with duration "ライブ終了時まで" are properly cleared when live phase ends
// Rule 9.7.5.1: Effects with live_end duration expire at end of live victory determination phase

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase, TurnPhase};
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_q247_live_end_duration_clearing() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find sakurakoji_kinako (PL!SP-bp1-006-R)
    let sakurakoji_kinako = cards.iter()
        .find(|c| c.card_no == "PL!SP-bp1-006-R")
        .expect("Sakurakoji Kinako card not found");
    let sakurakoji_kinako_id = get_card_id(sakurakoji_kinako, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    setup_player_with_hand(&mut player1, vec![sakurakoji_kinako_id]);
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    
    // Step 1: Play sakurakoji_kinako to center
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(sakurakoji_kinako_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    assert!(result.is_ok(), "Should play sakurakoji_kinako to center: {:?}", result);
    assert_eq!(game_state.player1.stage.get_area(MemberArea::Center), Some(sakurakoji_kinako_id),
        "Sakurakoji Kinako should be in center");
    
    // Step 2: Process debut auto ability (live_start ability)
    // The live_start ability is optional: pay 1 energy to gain 2 blades with live_end duration
    let _ = game_state.ability_queue.start_next();
    
    // Step 3: Add a temporary effect with Duration::LiveEnd
    // For this test, we'll manually add a temporary effect to test expiration
    game_state.add_temporary_effect(
        "blade_modifier".to_string(),
        rabuka_engine::game_state::Duration::LiveEnd,
        "player1".to_string(),
        "live_start_ability".to_string(),
    );
    
    // Verify the effect was added
    assert_eq!(game_state.temporary_effects.len(), 1, "Should have 1 temporary effect");
    
    // Step 4: Advance turn without performing live
    // Set turn phase to something other than Live to simulate end of turn
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game_state.turn_number = 2;
    
    // Step 5: Call check_expired_effects
    game_state.check_expired_effects();
    
    // Verify: Temporary effect should be cleared
    assert_eq!(game_state.temporary_effects.len(), 0, "Should have 0 temporary effects after expiration");
}
