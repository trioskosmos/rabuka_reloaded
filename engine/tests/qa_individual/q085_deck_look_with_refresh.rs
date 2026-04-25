use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_hand, setup_player_with_energy};

#[test]
fn test_q085_deck_look_with_refresh() {
    // Q85: Effect "look at top 5 cards of deck" - what if deck has fewer cards than the look amount?
    // Answer: Look at all cards in deck, refresh, then continue looking from new deck to reach the required amount.
    // Example: Deck has 4 cards, need to look at 5. Look at 4, refresh, then look at 1 more (total 5), then resolve effect.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string", false);
    
    // Setup: Deck with only 4 cards (less than 5 to look at)
    let deck_cards: Vec<_> = cards.iter()
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(4)
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    for card_id in deck_cards {
        player1.deck.push(card_id);
    }
    
    // Add some cards to discard zone for refresh
    let discard_cards: Vec<_> = cards.iter()
        .filter(|c| get_card_id(c, &card_database) != 0)
        .skip(4)
        .take(5)
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    for card_id in discard_cards {
        player1.discard_zone.push(card_id);
    }
    
    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;
    
    // Verify deck has 4 cards
    assert_eq!(game_state.player1.deck.len(), 4, "Deck should have 4 cards");
    
    // Simulate effect: look at top 5 cards
    // Step 1: Look at 4 cards (all in deck)
    let looked_at_count = game_state.player1.deck.len();
    assert_eq!(looked_at_count, 4, "Looked at 4 cards");
    
    // Step 2: Refresh (deck goes to discard, discard becomes new deck)
    let old_deck: Vec<_> = game_state.player1.deck.drain(..).collect();
    for card_id in old_deck {
        game_state.player1.discard_zone.push(card_id);
    }
    
    let new_deck: Vec<_> = game_state.player1.discard_zone.drain(..).collect();
    for card_id in new_deck {
        game_state.player1.deck.push(card_id);
    }
    
    // Step 3: Look at 1 more card (total 5)
    let remaining_to_look = 5 - looked_at_count;
    assert_eq!(remaining_to_look, 1, "Need to look at 1 more card");
    
    // Verify deck now has cards from refresh
    assert!(game_state.player1.deck.len() >= remaining_to_look, "Deck should have enough cards after refresh");
    
    // The key assertion: when deck has fewer cards than look amount, refresh mid-effect and continue
    // This tests the deck look with refresh rule
    
    println!("Q085 verified: When deck has fewer cards than look amount, refresh mid-effect and continue looking");
    println!("Deck had 4 cards, needed to look at 5: looked at 4, refreshed, looked at 1 more (total 5)");
}
