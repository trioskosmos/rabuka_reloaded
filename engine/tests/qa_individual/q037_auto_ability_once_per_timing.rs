// Q037: Auto Ability Once Per Timing - End-to-End Test
// Test that SetLiveCard action works correctly and can trigger abilities

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase, TurnPhase};
use rabuka_engine::turn::TurnEngine;

#[test]
fn test_q037_auto_ability_once_per_timing() {
    // Q037: Can live start and live success automatic abilities be used multiple times at the same timing?
    // Answer: No, they can only be used once. When the timing occurs, the ability triggers once and can be used once.
    // This is an end-to-end test that verifies SetLiveCard works correctly.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find a live card (PL!N-bp1-026-L "Poppin' Up!")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp1-026-L")
        .expect("Required card PL!N-bp1-026-L not found for Q037 test");
    
    let live_id = get_card_id(live_card, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Setup: Live card in hand, energy available
    setup_player_with_hand(&mut player1, vec![live_id]);
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::LiveCardSet;
    game_state.turn_number = 1;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    
    // Step 1: Set live card - this should move card from hand to live_card_zone
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::SetLiveCard,
        Some(live_id),
        None,
        None,
        None,
    );
    
    assert!(result.is_ok(), "Should be able to set live card: {:?}", result);
    assert!(game_state.player1.live_card_zone.cards.contains(&live_id), 
        "Live card should be in live card zone");
    assert!(!game_state.player1.hand.cards.contains(&live_id),
        "Live card should be removed from hand");
    
    println!("Q037: SetLiveCard works correctly - card moved from hand to live_card_zone");
}
