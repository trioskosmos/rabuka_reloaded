use rabuka_engine::*;
use std::collections::HashMap;

/// Test dynamic count abilities with real cards
#[test]
fn test_dynamic_count_player_choice() {
    // Test abilities that use dynamic_count: PlayerChoice
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with PlayerChoice dynamic count
    let test_card = Card {
        card_no: "DYN-001".to_string(),
        name: "Flexible Scout".to_string(),
        card_type: CardType::Member,
        color: "Blue".to_string(),
        cost: Some(2),
        blade: Some(1),
        ability: "【自】:相手のデッキの上から好きな枚数選び、手札に加える。".to_string(),
        full_text: "【自】:相手のデッキの上から好きな枚数選び、手札に加える。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to opponent's deck
    for i in 1..=8 {
        player2.main_deck.cards.push(card_id + i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing PlayerChoice dynamic count...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "PlayerChoice dynamic count should execute successfully");
    
    // Should prompt for player choice (this depends on implementation)
    // For now, we just verify it doesn't crash
    
    println!("✅ PlayerChoice dynamic count test passed!");
}

#[test]
fn test_dynamic_count_remaining_looked_at() {
    // Test abilities that use dynamic_count: RemainingLookedAt
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with RemainingLookedAt dynamic count
    let test_card = Card {
        card_no: "DYN-002".to_string(),
        name: "Efficient Scout".to_string(),
        card_type: CardType::Member,
        color: "Red".to_string(),
        cost: Some(3),
        blade: Some(2),
        ability: "【自】:相手のデッキの上からカードを2枚見て、1枚選び、手札に加える。残りのカードをすべて捨てる。".to_string(),
        full_text: "【自】:相手のデッキの上からカードを2枚見て、1枚選び、手札に加える。残りのカードをすべて捨てる。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to opponent's deck
    for i in 1..=6 {
        player2.main_deck.cards.push(card_id + i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing RemainingLookedAt dynamic count...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "RemainingLookedAt dynamic count should execute successfully");
    
    // Should have looked at 2 cards
    assert_eq!(executor.looked_at_cards.len(), 2, "Should have looked at 2 cards");
    
    // Should have 1 card selected to hand
    assert!(player1.hand.cards.len() > 1, "Should have selected at least 1 card to hand");
    
    // Should have 1 remaining card in discard
    // (This depends on implementation)
    
    println!("✅ RemainingLookedAt dynamic count test passed!");
}

#[test]
fn test_dynamic_count_revealed_cards() {
    // Test abilities that use dynamic_count: RevealedCards
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with RevealedCards dynamic count
    let test_card = Card {
        card_no: "DYN-003".to_string(),
        name: "Revealing Scout".to_string(),
        card_type: CardType::Member,
        color: "Green".to_string(),
        cost: Some(4),
        blade: Some(3),
        ability: "【自】:自分のデッキの上からカードを3枚公開する。公開したカードの枚数だけ、相手はカードを引く。".to_string(),
        full_text: "【自】:自分のデッキの上からカードを3枚公開する。公開したカードの枚数だけ、相手はカードを引く。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to player's deck
    for i in 10..=20 {
        player1.main_deck.cards.push(i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing RevealedCards dynamic count...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "RevealedCards dynamic count should execute successfully");
    
    // Should have revealed 3 cards from deck
    assert_eq!(player1.main_deck.cards.len(), 6, "Should have revealed 3 cards from deck");
    
    // Should allow opponent to draw 3 cards (this depends on implementation)
    
    println!("✅ RevealedCards dynamic count test passed!");
}

#[test]
fn test_dynamic_count_hand_size() {
    // Test abilities that use dynamic_count: HandSize
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with HandSize dynamic count
    let test_card = Card {
        card_no: "DYN-004".to_string(),
        name: "Hand-Size Scout".to_string(),
        card_type: CardType::Member,
        color: "Yellow".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【自】:自分の手札の枚数だけ、カードを引く。".to_string(),
        full_text: "【自】:自分の手札の枚数だけ、カードを引く。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to player's deck
    for i in 10..=15 {
        player1.main_deck.cards.push(i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing HandSize dynamic count...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "HandSize dynamic count should execute successfully");
    
    // Should draw cards equal to hand size (1 card)
    assert_eq!(player1.main_deck.cards.len(), 4, "Should have drawn 1 card from deck");
    
    println!("✅ HandSize dynamic count test passed!");
}

#[test]
fn test_dynamic_count_deck_size() {
    // Test abilities that use dynamic_count: DeckSize
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with DeckSize dynamic count
    let test_card = Card {
        card_no: "DYN-005".to_string(),
        name: "Deck-Size Scout".to_string(),
        card_type: CardType::Member,
        color: "Purple".to_string(),
        cost: Some(2),
        blade: Some(1),
        ability: "【自】:自分のデッキの枚数だけ、カードを引く。".to_string(),
        full_text: "【自】:自分のデッキの枚数だけ、カードを引く。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to player's deck
    for i in 10..=20 {
        player1.main_deck.cards.push(i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing DeckSize dynamic count...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "DeckSize dynamic count should execute successfully");
    
    // Should draw cards equal to deck size (10 cards)
    assert_eq!(player1.main_deck.cards.len(), 0, "Should have drawn 10 cards from deck");
    
    println!("✅ DeckSize dynamic count test passed!");
}

#[test]
fn test_any_number_with_dynamic_count() {
    // Test any_number: true with dynamic count
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with any_number and dynamic count
    let test_card = Card {
        card_no: "DYN-006".to_string(),
        name: "Ultimate Scout".to_string(),
        card_type: CardType::Member,
        color: "Blue".to_string(),
        cost: Some(5),
        blade: Some(4),
        ability: "【自】:相手のデッキの上から好きな枚数選び、手札に加える。".to_string(),
        full_text: "【自】:相手のデッキの上から好きな枚数選び、手札に加える。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to opponent's deck
    for i in 1..=10 {
        player2.main_deck.cards.push(card_id + i);
    }
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing any_number with dynamic count...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "any_number with dynamic count should execute successfully");
    
    // Should prompt for player choice (this depends on implementation)
    
    println!("✅ any_number with dynamic count test passed!");
}
