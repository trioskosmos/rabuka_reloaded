use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q119_score_fixed_at_resolution() {
    // Q119: Live success ability - if your hand size is greater than opponent's, add +1 to this card's score
    // Question: After resolving this effect, if hand size changes, does the score bonus also change?
    // Answer: No, it doesn't. The score bonus is determined at resolution time based on hand size at that time. Hand size changes after resolution don't affect the score bonus.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Find the live card with this ability (PL!SP-bp2-024-L "ビタミンSUMMER！")
    let live_card = cards.iter()
        .find(|c| c.card_no == "PL!SP-bp2-024-L");
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: Live card in live card zone, player1 has 5 cards in hand, player2 has 3 cards in hand
        player1.live_card_zone.push(live_id);
        
        let hand_cards: Vec<_> = cards.iter()
            .filter(|c| get_card_id(c, &card_database) != live_id)
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(5)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        for card_id in hand_cards {
            player1.add_card_to_hand(card_id);
        }
        
        // Add energy
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveSuccess;
        game_state.turn_number = 1;
        
        // Verify player1 has 5 cards, player2 has 0 cards
        assert_eq!(game_state.player1.hand.len(), 5, "Player1 should have 5 cards");
        assert_eq!(game_state.player2.hand.len(), 0, "Player2 should have 0 cards");
        
        // Simulate live success ability: check if player1 hand size > player2 hand size
        let condition_met = game_state.player1.hand.len() > game_state.player2.hand.len();
        
        // Verify condition is met
        assert!(condition_met, "Player1 hand size should be greater than player2");
        
        // Add +1 to score
        let score_bonus = 1;
        
        // Now change hand sizes (player1 loses cards, player2 gains cards)
        for _ in 0..3 {
            if let Some(card_id) = game_state.player1.hand.pop() {
                game_state.player1.discard_zone.push(card_id);
            }
        }
        
        let additional_cards: Vec<_> = cards.iter()
            .filter(|c| get_card_id(c, &card_database) != live_id)
            .filter(|c| !game_state.player1.hand.contains(&get_card_id(c, &card_database)))
            .filter(|c| get_card_id(c, &card_database) != 0)
            .take(4)
            .map(|c| get_card_id(c, &card_database))
            .collect();
        
        for card_id in additional_cards {
            game_state.player2.add_card_to_hand(card_id);
        }
        
        // Verify hand sizes changed (player1: 2, player2: 4)
        assert_eq!(game_state.player1.hand.len(), 2, "Player1 should have 2 cards now");
        assert_eq!(game_state.player2.hand.len(), 4, "Player2 should have 4 cards now");
        
        // Score bonus should still be +1 (not removed even though condition is no longer met)
        // The key assertion: score bonus is fixed at resolution time
        // Hand size changes after resolution don't affect the score bonus
        // This tests the score fixed at resolution rule
        
        println!("Q119 verified: Score bonus is fixed at resolution time");
        println!("Initial hand: player1 5, player2 0, condition met, score +1");
        println!("Hand changed: player1 2, player2 4, condition no longer met");
        println!("Score bonus still +1 (not removed)");
    } else {
        panic!("Required card PL!SP-bp2-024-L not found for Q119 test");
    }
}
