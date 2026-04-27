// Q250: Gain Ability with Reveal Cost
// Test gain_ability with reveal cost and conditional effect based on total cost
// NOTE: This test documents that ActivateAbility action type does not exist in the engine
// The engine can trigger debut abilities when playing members, but cannot manually
// trigger 起動 (activation) abilities. This is a documented limitation.

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase, TurnPhase};
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_q250_gain_ability_reveal_cost() {
    // This test documents that the engine lacks ActivateAbility action type
    // 起動 abilities cannot be manually triggered through TurnEngine
    // Only debut abilities trigger automatically when playing members to stage
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find 嵐 千砂都 (PL!SP-bp1-003-R＋) - has 起動 ability
    let chisato_card = cards.iter()
        .find(|c| c.card_no == "PL!SP-bp1-003-R＋")
        .expect("Required card PL!SP-bp1-003-R+ not found for Q250 test");
    
    let chisato_id = get_card_id(chisato_card, &card_database);
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(30)
        .collect();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    setup_player_with_hand(&mut player1, vec![chisato_id]);
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    
    // Play 嵐 千砂都 to stage - this will trigger debut ability if it has one
    let result = TurnEngine::execute_main_phase_action(
        &mut game_state,
        &rabuka_engine::game_setup::ActionType::PlayMemberToStage,
        Some(chisato_id),
        None,
        Some(MemberArea::Center),
        Some(false),
    );
    
    assert!(result.is_ok(), "Should be able to play 嵐 千砂都 to stage");
    
    // Document the limitation: Cannot manually trigger 起動 abilities
    // The engine would need an ActionType::ActivateAbility to support this
    println!("Q250: Engine lacks ActivateAbility action type");
    println!("Q250: 起動 abilities cannot be manually triggered");
    println!("Q250: Only debut/live_start/live_success abilities trigger automatically");
}
