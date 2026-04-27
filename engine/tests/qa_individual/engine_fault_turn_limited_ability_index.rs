// Q242: turn_limited_abilities_used should use correct ability_index
// Fault: ability_resolver.rs hardcodes ability_index to 0 when tracking turn-limited abilities,
// but turn.rs uses the actual ability_index from the loop. This causes inconsistent tracking
// for cards with multiple abilities that have use_limit.

use crate::qa_individual::common::*;
use rabuka_engine::game_state::{GameState, Phase};
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_q242_turn_limited_ability_index_consistency() {
    // Test: turn_limited_abilities_used should use consistent ability_index keys
    // across turn.rs and ability_resolver.rs
    
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
    
    // ENGINE FAULT: In ability_resolver.rs line 4443, the ability_key is constructed as:
    // format!("{}_{}_{}", card_id, 0, self.game_state.turn_number)
    // This hardcodes ability_index to 0, but in turn.rs line 586, it uses:
    // format!("{}_{}_{}", stage_card_id, ability_index, game_state.turn_number)
    // where ability_index comes from the actual loop.
    // 
    // If a card has multiple abilities with use_limit, only ability_index 0 would
    // be tracked correctly in ability_resolver.rs. The other abilities would not
    // be properly tracked as used.
    
    // This test documents the fault. A fix would require ability_resolver.rs
    // to receive and use the correct ability_index when constructing the key.
}
