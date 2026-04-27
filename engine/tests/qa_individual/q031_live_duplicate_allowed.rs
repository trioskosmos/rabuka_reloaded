use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q031_live_duplicate_allowed() {
    // Q31: Can you put 2+ copies of same card in live area?
    // Answer: Yes, can put 2+ copies even with same card number or name.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card
    let live_card = cards.iter()
        .filter(|c| c.is_live())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .next();
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: 2 copies of live card in hand
        setup_player_with_hand(&mut player1, vec![live_id, live_id]);
        
        // Add member cards to stage to meet heart requirements
        let member_cards: Vec<_> = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .filter(|c| c.cost.map_or(false, |cost| cost <= 3))
            .take(3)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        for member_id in member_cards.iter() {
            player1.add_card_to_hand(*member_id);
        }
        
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
        
        // Play members to stage
        for member_id in member_cards.iter() {
            let result = TurnEngine::execute_main_phase_action(
                &mut game_state,
                &ActionType::PlayMemberToStage,
                Some(*member_id),
                None,
                Some(MemberArea::Center),
                Some(false),
            );
            if result.is_err() {
                let _ = TurnEngine::execute_main_phase_action(
                    &mut game_state,
                    &ActionType::PlayMemberToStage,
                    Some(*member_id),
                    None,
                    Some(MemberArea::LeftSide),
                    Some(false),
                );
            }
        }
        
        // Set live cards using proper SetLiveCard action
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveCardSet;
        game_state.current_turn_phase = rabuka_engine::game_state::TurnPhase::Live;
        
        // Set first live card
        let result1 = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::SetLiveCard,
            Some(live_id),
            None,
            None,
            None,
        );
        assert!(result1.is_ok(), "Should be able to set first live card");
        
        // Set second live card (same card number)
        let result2 = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::SetLiveCard,
            Some(live_id),
            None,
            None,
            None,
        );
        assert!(result2.is_ok(), "Should be able to set second live card (duplicate)");
        
        // Verify both live cards are in zone
        assert_eq!(game_state.player1.live_card_zone.cards.len(), 2, "Should have 2 live cards");
        
        // The key assertion: can put 2+ copies of same card in live area
        // This tests the live duplicate allowed rule
        
        println!("Q031 verified: Can put 2+ copies of same card in live area");
    } else {
        panic!("Required live card not found for Q031 test");
    }
}
