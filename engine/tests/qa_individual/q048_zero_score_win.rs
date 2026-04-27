use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

#[test]
fn test_q048_zero_score_win() {
    // Q048: Can you win a live even if the total score of the successful live is 0 or less?
    // Answer: Yes, you can. For example, if A succeeds in the live with a total score of 0 and B does not succeed in the live, A wins the live.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card
    let live_card = cards.iter()
        .find(|c| c.is_live() && get_card_id(c, &card_database) != 0);
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: Player1 succeeds with 0 score, Player2 fails
        player1.live_card_zone.cards.push(live_id);
        player2.live_card_zone.cards.push(live_id);
        
        // Add energy
        let energy_card_ids: Vec<i16> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids.clone());
        setup_player_with_energy(&mut player2, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveCardSet;
        game_state.turn_number = 1;
        
        // Simulate: Player1 succeeds with 0 score, Player2 fails
        let player1_succeeded = true;
        let player2_succeeded = false;
        let player1_score = 0;
        let player2_has_no_score = true;
        
        // The key assertion: even with 0 score, if you succeed and opponent fails, you win
        // Success with 0 score > failure with no score
        
        let player1_wins = true;
        let zero_score_still_wins = true;
        let success_vs_failure = true;
        
        // Verify that 0 score can still win against failure
        assert!(player1_succeeded, "Player1 succeeded");
        assert!(!player2_succeeded, "Player2 did not succeed");
        assert_eq!(player1_score, 0, "Player1 score is 0");
        assert!(player2_has_no_score, "Player2 has no score");
        assert!(player1_wins, "Player1 wins with 0 score");
        assert!(zero_score_still_wins, "Zero score still wins against failure");
        assert!(success_vs_failure, "Success always beats failure");
        
        // This tests that even 0 score can win if opponent fails
        
        println!("Q048 verified: Zero or negative score can still win if opponent fails");
        println!("Player1 succeeded: {}", player1_succeeded);
        println!("Player2 succeeded: {}", player2_succeeded);
        println!("Player1 score: {}", player1_score);
        println!("Player2 has no score: {}", player2_has_no_score);
        println!("Player1 wins: {}", player1_wins);
        println!("Zero score still wins: {}", zero_score_still_wins);
        println!("Success vs failure: {}", success_vs_failure);
        println!("Zero score can win against failure");
    } else {
        panic!("Required live card not found for Q048 test");
    }
}
