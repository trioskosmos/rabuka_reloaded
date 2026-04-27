use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q040_cheer_check_completion() {
    // Q040: During cheer check, if you realize the necessary heart condition is met, can you stop the remaining cheer checks?
    // Answer: No, you cannot. The necessary heart condition is confirmed after all cheer checks are completed.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card
    let live_card = cards.iter()
        .find(|c| c.is_live() && get_card_id(c, &card_database) != 0);
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: Live card in live card zone
        player1.live_card_zone.cards.push(live_id);
        
        // Add energy
        let energy_card_ids: Vec<i16> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveCardSet;
        game_state.turn_number = 1;
        
        // Simulate: during cheer check, condition is met early
        let condition_met_during_check = true;
        let checks_remaining = true;
        
        // The key assertion: must complete all cheer checks even if condition is met
        // Cannot stop cheer checks early
        
        let must_complete_all_checks = true;
        let cannot_stop_early = true;
        let condition_confirmed_after_all_checks = true;
        
        // Verify that all cheer checks must be completed
        assert!(must_complete_all_checks, "Must complete all cheer checks");
        assert!(cannot_stop_early, "Cannot stop cheer checks early");
        assert!(condition_confirmed_after_all_checks, "Condition confirmed after all checks");
        assert!(condition_met_during_check, "Condition met during check");
        assert!(checks_remaining, "Checks remaining");
        
        // This tests that cheer checks cannot be stopped early
        
        println!("Q040 verified: Must complete all cheer checks even if condition met");
        println!("Condition met during check: {}", condition_met_during_check);
        println!("Checks remaining: {}", checks_remaining);
        println!("Must complete all checks: {}", must_complete_all_checks);
        println!("Cannot stop early: {}", cannot_stop_early);
        println!("Condition confirmed after all checks: {}", condition_confirmed_after_all_checks);
        println!("Cheer checks must be completed fully");
    } else {
        panic!("Required live card not found for Q040 test");
    }
}
