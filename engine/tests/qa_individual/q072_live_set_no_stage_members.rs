// Test plan: tests/qa_individual/q072_live_set_no_stage_members.md
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::game_setup::ActionType;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q072_live_set_no_stage_members() {
    // Q72: Can you place cards in live card zone during live card set phase when you have no members on stage?
    // Answer: Yes, you can.
    // See test plan for detailed methodology
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card
    let live_card = cards.iter()
        .find(|c| c.is_live() && get_card_id(c, &card_database) != 0);
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: Live card in hand, no members on stage
        player1.add_card_to_hand(live_id);
        
        // Verify stage is empty
        assert_eq!(player1.stage.stage[0], -1, "Left area should be empty");
        assert_eq!(player1.stage.stage[1], -1, "Center area should be empty");
        assert_eq!(player1.stage.stage[2], -1, "Right area should be empty");
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveCardSet;
        game_state.turn_number = 1;
        
        // Place live card in live card zone using real engine function
        let result = TurnEngine::execute_main_phase_action(
            &mut game_state,
            &ActionType::SetLiveCard,
            Some(live_id),
            None,
            None,
            None,
        );
        
        assert!(result.is_ok(), "Live card placement should succeed: {:?}", result);
        
        // Verify live card was placed
        assert!(game_state.player1.live_card_zone.cards.contains(&live_id), "Live card should be in live card zone");
        assert!(!game_state.player1.hand.cards.contains(&live_id), "Live card should not be in hand");
        
        // Verify stage is still empty
        assert_eq!(game_state.player1.stage.stage[0], -1, "Left area should still be empty");
        assert_eq!(game_state.player1.stage.stage[1], -1, "Center area should still be empty");
        assert_eq!(game_state.player1.stage.stage[2], -1, "Right area should still be empty");
        
        // The key assertion: can place live cards even with no members on stage
        // This tests the live set with no stage members rule
        
        println!("Q072 verified: Can place live cards in live card zone even with no members on stage");
        println!("Stage is empty, live card placed successfully");
    } else {
        panic!("Required live card not found for Q072 test");
    }
}
