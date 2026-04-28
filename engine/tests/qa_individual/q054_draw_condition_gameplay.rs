use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id, setup_player_with_energy};

/// Q54: Draw condition gameplay test
/// Question: What happens when 3+ success cards are in the success zone?
/// Answer: The game is a draw (in full deck format)
#[test]
fn test_q054_draw_condition_three_success_cards() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find a live card for testing
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(3)
        .collect();
    
    assert!(live_cards.len() >= 3, "Need at least 3 live cards for this test");
    
    // Setup: Player 1 has 3 success cards in success zone
    for live_card in &live_cards {
        let live_id = get_card_id(live_card, &card_database);
        player1.success_live_card_zone.cards.push(live_id);
    }
    
    // Add energy to both players
    let energy_card_ids: Vec<i16> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .map(|c| get_card_id(c, &card_database))
        .take(5)
        .collect();
    setup_player_with_energy(&mut player1, energy_card_ids.clone());
    setup_player_with_energy(&mut player2, energy_card_ids);
    
    let game_state = GameState::new(player1, player2, card_database.clone());
    
    // Record initial state
    let p1_success_count = game_state.player1.success_live_card_zone.cards.len();
    let p2_success_count = game_state.player2.success_live_card_zone.cards.len();
    
    // Verify: Player 1 has 3 success cards
    assert_eq!(p1_success_count, 3, "Player 1 should have 3 success cards");
    assert_eq!(p2_success_count, 0, "Player 2 should have 0 success cards");
    
    // Check draw condition for both players
    let p1_draw_condition = game_state.check_success_zone_draw_condition("player1");
    let p2_draw_condition = game_state.check_success_zone_draw_condition("player2");
    
    // Verify: Player 1 meets draw condition (3+ success cards)
    assert!(p1_draw_condition, "Player 1 with 3 success cards should meet draw condition");
    
    // Verify: Player 2 does not meet draw condition (0 success cards)
    assert!(!p2_draw_condition, "Player 2 with 0 success cards should not meet draw condition");
    
    println!("Q054 verified: Success zone draw condition");
    println!("Player 1 success cards: {}", p1_success_count);
    println!("Player 2 success cards: {}", p2_success_count);
    println!("Player 1 draw condition met: {}", p1_draw_condition);
    println!("Player 2 draw condition met: {}", p2_draw_condition);
    println!("Draw condition: 3+ success cards in success zone");
}

/// Q54: Edge case - exactly 2 success cards (should not be draw in full deck)
#[test]
fn test_q054_two_success_cards_not_draw() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find live cards for testing
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(2)
        .collect();
    
    assert!(live_cards.len() >= 2, "Need at least 2 live cards for this test");
    
    // Setup: Player 1 has exactly 2 success cards
    for live_card in &live_cards {
        let live_id = get_card_id(live_card, &card_database);
        player1.success_live_card_zone.cards.push(live_id);
    }
    
    let game_state = GameState::new(player1, player2, card_database.clone());
    
    // Verify: Player 1 has 2 success cards
    assert_eq!(game_state.player1.success_live_card_zone.cards.len(), 2, "Player 1 should have 2 success cards");
    
    // Check draw condition
    let draw_condition = game_state.check_success_zone_draw_condition("player1");
    
    // Verify: 2 success cards does NOT meet draw condition for full deck
    assert!(!draw_condition, "Player 1 with 2 success cards should NOT meet draw condition (full deck)");
    
    println!("Q054 verified: 2 success cards is not a draw in full deck format");
    println!("Success cards: 2, Draw condition met: {}", draw_condition);
}

/// Q54: Edge case - both players have 3+ success cards (draw game)
#[test]
fn test_q054_both_players_draw_condition() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Find live cards for testing (need 6 total)
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .filter(|c| get_card_id(c, &card_database) != 0)
        .take(6)
        .collect();
    
    assert!(live_cards.len() >= 6, "Need at least 6 live cards for this test");
    
    // Setup: Both players have 3 success cards each
    for i in 0..3 {
        let live_id = get_card_id(live_cards[i], &card_database);
        player1.success_live_card_zone.cards.push(live_id);
    }
    for i in 3..6 {
        let live_id = get_card_id(live_cards[i], &card_database);
        player2.success_live_card_zone.cards.push(live_id);
    }
    
    let game_state = GameState::new(player1, player2, card_database.clone());
    
    // Verify both players have 3 success cards
    assert_eq!(game_state.player1.success_live_card_zone.cards.len(), 3);
    assert_eq!(game_state.player2.success_live_card_zone.cards.len(), 3);
    
    // Check draw condition for both
    let p1_draw = game_state.check_success_zone_draw_condition("player1");
    let p2_draw = game_state.check_success_zone_draw_condition("player2");
    
    // Both should meet draw condition
    assert!(p1_draw, "Player 1 should meet draw condition");
    assert!(p2_draw, "Player 2 should meet draw condition");
    
    // When both players meet draw condition, game is a draw
    println!("Q054 verified: Both players with 3+ success cards = draw game");
    println!("Player 1 success cards: 3, meets draw: {}", p1_draw);
    println!("Player 2 success cards: 3, meets draw: {}", p2_draw);
    println!("Game result: Draw (both players have 3+ success cards)");
}
