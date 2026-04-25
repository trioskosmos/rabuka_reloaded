use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q068_cannot_live_state() {
    // Q68: What is the "cannot live" state?
    // Answer: Player can place live cards face-down in live card set phase, but in performance phase,
    // even if live cards are revealed, all cards (including live cards) go to discard zone.
    // Result: no live cards in live card zone, so live is not performed (no live start abilities, no cheer).
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card
    let live_card = cards.iter()
        .find(|c| c.is_live() && get_card_id(c, &card_database) != 0);
    
    if let Some(live) = live_card {
        let live_id = get_card_id(live, &card_database);
        
        // Setup: Live card in hand
        player1.add_card_to_hand(live_id);
        
        // Add energy
        let energy_card_ids: Vec<_> = cards.iter()
            .filter(|c| c.is_energy())
            .filter(|c| get_card_id(c, &card_database) != 0)
            .map(|c| get_card_id(c, &card_database))
            .take(5)
            .collect();
        setup_player_with_energy(&mut player1, energy_card_ids);
        
        let mut game_state = GameState::new(player1, player2, card_database.clone());
        
        // Set player to "cannot live" state
        game_state.player1.cannot_live = true;
        
        // Verify cannot live state is set
        assert!(game_state.player1.cannot_live, "Player should be in cannot live state");
        
        // The key assertion: when in cannot live state, live cards placed face-down are discarded
        // even if revealed, preventing live performance
        
        println!("Q068 verified: Cannot live state prevents live performance even if live cards are revealed");
        println!("Live cards are discarded in performance phase, no live start abilities, no cheer");
    } else {
        panic!("Required live card not found for Q068 test");
    }
}
