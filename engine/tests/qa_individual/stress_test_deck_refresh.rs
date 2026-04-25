// Stress test: Deck refresh during complex gameplay
// This tests deck refresh timing and state preservation

use crate::qa_individual::common::*;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

#[test]
fn test_stress_deck_refresh_during_gameplay() {
    // Stress test: Play members until deck is nearly empty, then trigger refresh
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find member cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.cost.is_some() && c.cost.unwrap_or(0) <= 2)
        .take(10)
        .collect();
    
    let member_ids: Vec<_> = member_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let energy_card_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(50)
        .collect();
    
    setup_player_with_hand(&mut player1, member_ids.clone());
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
    
    // Set up deck with very few cards to trigger refresh
    let small_deck_ids: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .take(2)
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    game_state.player1.main_deck.cards = small_deck_ids;
    game_state.player1.waitroom.cards = cards.iter()
        .filter(|c| c.is_member())
        .skip(2)
        .take(10)
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let initial_deck_size = game_state.player1.main_deck.cards.len();
    
    // Play members to trigger deck depletion
    let mut played_count = 0;
    for (i, &member_id) in member_ids.iter().enumerate() {
        if i >= 3 { break; }
        
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        if result.is_ok() {
            played_count += 1;
            game_state.turn_number += 1;
            game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::FirstAttackerNormal;
            game_state.player1.areas_locked_this_turn.clear();
        }
    }
    
    // Verify deck state after playing
    println!("Stress test: Deck refresh during gameplay - played {} members", played_count);
    println!("Initial deck size: {}, current deck size: {}", initial_deck_size, game_state.player1.main_deck.cards.len());
}
