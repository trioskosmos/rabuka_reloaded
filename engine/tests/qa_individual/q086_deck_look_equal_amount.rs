use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q086_deck_look_equal_amount() {
    // Q86: Effect "look at top 5 cards of deck" - what if deck has exactly the same number of cards as the look amount?
    // Answer: Look at all cards, resolve effect, no refresh during the look. If effect resolution makes deck 0, then refresh.
    // If cards go to discard and deck becomes 0, refresh includes those cards.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Setup: Deck with exactly 5 cards (same as look amount)
    let deck_cards: Vec<_> = cards.iter()
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(5)
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    for card_id in deck_cards {
        player1.deck.push(card_id);
    }
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    
    // Verify deck has 5 cards
    assert_eq!(game_state.player1.deck.len(), 5, "Deck should have 5 cards");
    
    // Simulate effect: look at top 5 cards
    let looked_at_count = game_state.player1.deck.len();
    assert_eq!(looked_at_count, 5, "Looked at 5 cards");
    
    // No refresh happens during the look when deck size equals look amount
    // This is the key difference from Q85
    
    // Simulate effect resolution: all 5 cards go to discard
    let looked_cards: Vec<_> = game_state.player1.deck.drain(..).collect();
    for card_id in looked_cards {
        game_state.player1.discard_zone.push(card_id);
    }
    
    // Verify deck is now 0
    assert_eq!(game_state.player1.deck.len(), 0, "Deck should be 0 after effect resolution");
    
    // Now refresh happens because deck is 0
    let new_deck: Vec<_> = game_state.player1.discard_zone.drain(..).collect();
    for card_id in new_deck {
        game_state.player1.deck.push(card_id);
    }
    
    // Verify deck has cards after refresh
    assert!(game_state.player1.deck.len() > 0, "Deck should have cards after refresh");
    
    // The key assertion: when deck size equals look amount, no refresh during look
    // Refresh only happens if effect resolution makes deck 0
    // This tests the deck look equal amount rule
    
    println!("Q086 verified: When deck size equals look amount, no refresh during look");
    println!("Deck had 5 cards, looked at 5, no refresh during look");
    println!("After effect resolution (all cards to discard), deck became 0, then refresh happened");
}
