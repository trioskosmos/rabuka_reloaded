use rabuka_engine::*;
use std::collections::HashMap;

/// Test look_and_select abilities with real cards
#[test]
fn test_look_and_select_abilities() {
    // Initialize game components
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test cards with look_and_select abilities
    let test_card = Card {
        card_no: "TEST-001".to_string(),
        name: "Scout Member".to_string(),
        card_type: CardType::Member,
        color: "Blue".to_string(),
        cost: Some(2),
        blade: Some(1),
        // Add look_and_select ability
        ability: "【自】:相手のデッキの上からカードを2枚見て、1枚選び、手札に加える。".to_string(),
        full_text: "【自】:相手のデッキの上からカードを2枚見て、1枚選び、手札に加える。".to_string(),
        ..Default::default()
    };
    
    // Add cards to database
    let card_id = card_database.add_card(test_card);
    
    // Add cards to opponent's deck (for looking at)
    player2.main_deck.cards.push(card_id + 1);
    player2.main_deck.cards.push(card_id + 2);
    player2.main_deck.cards.push(card_id + 3);
    
    // Add ability card to player's hand
    player1.hand.add_card(card_id);
    
    // Create ability executor
    let mut executor = AbilityExecutor::new();
    
    println!("Testing look_and_select ability...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "look_and_select ability should execute successfully");
    
    // Check that looked_at_cards were stored
    assert!(!executor.looked_at_cards.is_empty(), "Should have looked at cards");
    assert_eq!(executor.looked_at_cards.len(), 2, "Should have looked at 2 cards");
    
    // Check that cards were moved to hand
    assert_eq!(player1.hand.cards.len(), 2, "Should have 2 cards in hand (1 original + 1 selected)");
    
    // Check that remaining looked-at cards are handled correctly
    // (This depends on the specific ability implementation)
    
    println!("✅ look_and_select test passed!");
}

#[test]
fn test_look_and_select_with_any_number() {
    // Test look_and_select with any_number: true
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with any_number look_and_select
    let test_card = Card {
        card_no: "TEST-002".to_string(),
        name: "Flexible Scout".to_string(),
        card_type: CardType::Member,
        color: "Red".to_string(),
        cost: Some(3),
        blade: Some(2),
        ability: "【自】:相手のデッキの上からカードを3枚見て、好きな枚数選び、手札に加える。".to_string(),
        full_text: "【自】:相手のデッキの上からカードを3枚見て、好きな枚数選び、手札に加える。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to opponent's deck
    for i in 1..=5 {
        player2.main_deck.cards.push(card_id + i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing look_and_select with any_number...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "any_number look_and_select should execute successfully");
    
    // Should have looked at 3 cards
    assert_eq!(executor.looked_at_cards.len(), 3, "Should have looked at 3 cards");
    
    println!("✅ look_and_select any_number test passed!");
}

#[test]
fn test_look_and_select_with_looked_at_remaining() {
    // Test look_and_select that uses looked_at_remaining source
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card that uses looked_at_remaining
    let test_card = Card {
        card_no: "TEST-003".to_string(),
        name: "Efficient Scout".to_string(),
        card_type: CardType::Member,
        color: "Green".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【自】:相手のデッキの上からカードを2枚見て、1枚選び、手札に加える。残りのカードをすべて捨てる。".to_string(),
        full_text: "【自】:相手のデッキの上からカードを2枚見て、1枚選び、手札に加える。残りのカードをすべて捨てる。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to opponent's deck
    for i in 1..=4 {
        player2.main_deck.cards.push(card_id + i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing look_and_select with looked_at_remaining...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "look_and_select with looked_at_remaining should execute successfully");
    
    // Should have looked at 2 cards
    assert_eq!(executor.looked_at_cards.len(), 2, "Should have looked at 2 cards");
    
    // Should have 1 card selected to hand
    assert!(player1.hand.cards.len() > 1, "Should have selected at least 1 card to hand");
    
    // Remaining cards should be in discard (this depends on implementation)
    
    println!("✅ look_and_select looked_at_remaining test passed!");
}

#[test]
fn test_sequential_look_and_select() {
    // Test complex look_and_select with sequential effects
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with complex look_and_select
    let test_card = Card {
        card_no: "TEST-004".to_string(),
        name: "Strategic Scout".to_string(),
        card_type: CardType::Member,
        color: "Purple".to_string(),
        cost: Some(4),
        blade: Some(3),
        ability: "【自】:相手のデッキの上からカードを3枚見て、1枚選び、手札に加える。その後、自分のデッキの上からカードを1枚引く。".to_string(),
        full_text: "【自】:相手のデッキの上からカードを3枚見て、1枚選び、手札に加える。その後、自分のデッキの上からカードを1枚引く。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to opponent's deck
    for i in 1..=5 {
        player2.main_deck.cards.push(card_id + i);
    }
    
    // Add cards to player's deck for drawing
    for i in 10..=15 {
        player1.main_deck.cards.push(i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing sequential look_and_select...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "sequential look_and_select should execute successfully");
    
    // Should have looked at 3 cards
    assert_eq!(executor.looked_at_cards.len(), 3, "Should have looked at 3 cards");
    
    // Should have 1 card selected to hand
    assert!(player1.hand.cards.len() > 1, "Should have selected at least 1 card to hand");
    
    // Should have drawn 1 card from deck
    assert!(player1.main_deck.cards.len() < 6, "Should have drawn 1 card from deck");
    
    println!("✅ sequential look_and_select test passed!");
}
