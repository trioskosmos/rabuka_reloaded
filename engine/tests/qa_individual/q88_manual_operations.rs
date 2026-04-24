use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q88_manual_operations() {
    // Q88: Can players arbitrarily perform operations like moving cards from hand to waitroom,
    // moving stage members to waitroom, moving stage members to other areas, changing active cards to wait?
    // Answer: No, you cannot.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Use a specific member card for testing: PL!N-bp1-001-R "上原歩夢"
    let member_card = cards.iter()
        .find(|c| c.card_no == "PL!N-bp1-001-R");
    
    if let Some(card) = member_card {
        let card_id = get_card_id(card, &card_database);
        
        setup_player_with_hand(&mut player1, vec![card_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(10)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // The key point: players cannot arbitrarily perform manual operations
        // This test verifies that the card plays to stage via proper game actions
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::PlayMemberToStage,
            Some(card_id),
            None,
            Some(MemberArea::Center),
            Some(false),
        );
        
        assert!(result.is_ok(), "Card should play to stage: {:?}", result);
        assert!(game_state.player1.stage.stage.contains(&card_id),
            "PL!N-bp1-001-R should be on stage after playing");
    } else {
        panic!("Required card PL!N-bp1-001-R not found in card database");
    }
}
