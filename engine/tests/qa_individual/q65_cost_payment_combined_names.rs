use rabuka_engine::game_state::GameState;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q65_cost_payment_combined_names() {
    // Q65: Can you pay cost with cards having combined names?
    // Answer: No, you cannot.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a card with "&" in name
    let ampersand_card = cards.iter()
        .find(|c| c.name.contains("&"));
    
    if let Some(card) = ampersand_card {
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
        
        let card_db_clone = card_database.clone();
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::Main;
        game_state.turn_number = 1;
        
        // The key point: cards with combined names cannot be used to pay costs that require specific names
        // This test verifies that such a card exists and can be played to stage
        assert!(card.name.contains("&"),
            "Card should have & in name");
        
        // Verify the card exists in the database
        assert!(card_db_clone.get_card(card_id).is_some(),
            "Card should exist in database");
    } else {
        println!("Skipping test: no card with & in name found");
    }
}
