use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q83_multiple_live_cards() {
    // Q83: If you have multiple face-up live cards and win a live, can you put all of them in success live card zone?
    // Answer: No, you choose 1 card to place. When winning with multiple live cards, you choose 1 from among them
    // to place in the success live card zone. Also, the player chooses which card to place.
    // No specific cards mentioned in qa_data.json
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find live cards
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .take(3)
        .collect();
    
    if live_cards.len() >= 2 {
        let live_card1_id = get_card_id(live_cards[0], &card_database);
        let live_card2_id = get_card_id(live_cards[1], &card_database);
        
        // Add live cards to player1's live card zone
        let card_db_clone = card_database.clone();
        player1.live_card_zone.add_card(live_card1_id, false, &card_db_clone).expect("Failed to add live card");
        player1.live_card_zone.add_card(live_card2_id, false, &card_db_clone).expect("Failed to add live card");
        
        let mut game_state = GameState::new(player1, player2, card_database);
        game_state.current_phase = rabuka_engine::game_state::Phase::LiveVictoryDetermination;
        game_state.turn_number = 1;
        
        // The key point: when winning with multiple live cards, only 1 is placed in success live card zone
        // This test verifies that player has multiple live cards
        assert!(game_state.player1.live_card_zone.cards.contains(&live_card1_id),
            "Player 1 should have first live card");
        assert!(game_state.player1.live_card_zone.cards.contains(&live_card2_id),
            "Player 1 should have second live card");
    } else {
        panic!("Need at least 2 live cards to test Q83");
    }
}
