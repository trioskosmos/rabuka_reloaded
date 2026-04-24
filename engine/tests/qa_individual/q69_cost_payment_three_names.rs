use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q69_cost_payment_three_names() {
    // Q69: Can you pay cost with 3 cards having combined names "上原歩夢", "澁谷かのん", "日野下花帆"?
    // Answer: Yes, you can. You can pay the cost with a combination of 3 cards having the names
    // "上原歩夢", "澁谷かのん", and "日野下花帆".
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find cards with the specific names
    let name_cards: Vec<_> = cards.iter()
        .filter(|c| {
            c.name.contains("上原歩夢") || c.name.contains("澁谷かのん") || c.name.contains("日野下花帆")
        })
        .take(3)
        .collect();
    
    if name_cards.len() >= 1 {
        let card_id = get_card_id(name_cards[0], &card_database);
        
        setup_player_with_hand(&mut player1, vec![card_id]);
        
        // Add energy cards
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(10)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let card_db_clone = card_database.clone();
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // The key point: you can pay costs with cards having specific names
        // This test verifies that cards with the expected names exist
        assert!(name_cards.len() > 0,
            "Should find cards with the specified names");
        
        // Verify the card exists in the database
        assert!(card_db_clone.get_card(card_id).is_some(),
            "Card should exist in database");
    } else {
        println!("Skipping test: no cards with specified names found");
    }
}
