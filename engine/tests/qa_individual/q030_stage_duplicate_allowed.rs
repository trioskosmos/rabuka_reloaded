use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q030_stage_duplicate_allowed() {
    // Q30: Can you debut 2+ copies of same card to stage?
    // Answer: Yes, can debut 2+ copies even with same card number or name.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find a member card
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .filter(|c| c.cost.map_or(false, |cost| cost <= 3))
        .next();
    
    if let Some(member) = member_card {
        let member_id = get_card_id(member, &card_database);
        
        // Setup: 2 copies of member in hand
        setup_player_with_hand(&mut player1, vec![member_id, member_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(10)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // Play both members to stage
        let result1 = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        assert!(result1.is_ok(), "First member should play to stage: {:?}", result1);
        
        let result2 = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(member_id),
            None,
            Some(MemberArea::Left),
            Some(false),
        );
        assert!(result2.is_ok(), "Second member should play to stage: {:?}", result2);
        
        // The key assertion: can debut 2+ copies of same card to stage
        // This tests the stage duplicate allowed rule
        
        println!("Q030 verified: Can debut 2+ copies of same card to stage");
    } else {
        panic!("Required member card not found for Q030 test");
    }
}
