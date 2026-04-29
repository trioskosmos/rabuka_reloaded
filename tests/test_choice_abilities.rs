use rabuka_engine::*;
use std::collections::HashMap;

/// Test choice-based abilities with real cards
#[test]
fn test_choice_abilities() {
    // Initialize game components
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with choice ability
    let test_card = Card {
        card_no: "CHOICE-001".to_string(),
        name: "Strategic Choice".to_string(),
        card_type: CardType::Member,
        color: "Blue".to_string(),
        cost: Some(3),
        blade: Some(2),
        ability: "【自】:カードを1枚引く。その後、以下から1つ選ぶ：A)相手に2ダメージ、B)自分の手札を1枚捨てる。".to_string(),
        full_text: "【自】:カードを1枚引く。その後、以下から1つ選ぶ：A)相手に2ダメージ、B)自分の手札を1枚捨てる。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to player's deck for drawing
    for i in 10..=15 {
        player1.main_deck.cards.push(i);
    }
    
    // Add cards to player's hand for discarding option
    for i in 20..=22 {
        player1.hand.add_card(i);
    }
    
    // Add cards to opponent's stage for damage option
    player2.stage.set_area(crate::zones::MemberArea::LeftSide, 100);
    player2.stage.set_area(crate::zones::MemberArea::LeftSide, 100);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing choice ability...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "choice ability should execute successfully");
    
    // Should have drawn 1 card from deck
    assert_eq!(player1.main_deck.cards.len(), 4, "Should have drawn 1 card from deck");
    
    // Should have executed one of the choice options
    // (This depends on the specific choice implementation)
    
    println!("✅ choice ability test passed!");
}

#[test]
fn test_multiple_choice_abilities() {
    // Test ability with multiple complex choices
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with multiple choices
    let test_card = Card {
        card_no: "CHOICE-002".to_string(),
        name: "Tactical Decision".to_string(),
        card_type: CardType::Member,
        color: "Red".to_string(),
        cost: Some(4),
        blade: Some(3),
        ability: "【自】:以下から2つ選ぶ：A)カードを2枚引く、B)相手のメンバーを1体選び、手札に加える。".to_string(),
        full_text: "【自】:以下から2つ選ぶ：A)カードを2枚引く、B)相手のメンバーを1体選び、手札に加える。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to player's deck for drawing
    for i in 10..=16 {
        player1.main_deck.cards.push(i);
    }
    
    // Add cards to opponent's stage for stealing option
    player2.stage.set_area(crate::zones::MemberArea::Center, 200);
    player2.stage.set_area(crate::zones::MemberArea::RightSide, 201);
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing multiple choice ability...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "multiple choice ability should execute successfully");
    
    // Should have executed one of the choice options
    // Either drawn 2 cards OR stolen 1 opponent member
    
    println!("✅ multiple choice ability test passed!");
}

#[test]
fn test_conditional_choice() {
    // Test choice that depends on game state
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with conditional choice
    let test_card = Card {
        card_no: "CHOICE-003".to_string(),
        name: "Adaptive Strategy".to_string(),
        card_type: CardType::Member,
        color: "Green".to_string(),
        cost: Some(2),
        blade: Some(1),
        ability: "【自】:自分の手札が3枚以上の場合、カードを2枚引く。3枚未満の場合、カードを1枚引く。".to_string(),
        full_text: "【自】:自分の手札が3枚以上の場合、カードを2枚引く。3枚未満の場合、カードを1枚引く。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to player's deck for drawing
    for i in 10..=15 {
        player1.main_deck.cards.push(i);
    }
    
    // Test case 1: Hand has 3+ cards
    player1.hand.add_card(100);
    player1.hand.add_card(101);
    player1.hand.add_card(102);
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing conditional choice (3+ cards)...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "conditional choice should execute successfully");
    
    // Should have drawn 2 cards (since hand had 3+ cards)
    assert_eq!(player1.main_deck.cards.len(), 3, "Should have drawn 2 cards from deck");
    
    // Test case 2: Reset and test with <3 cards
    player1.main_deck.cards.clear();
    for i in 10..=15 {
        player1.main_deck.cards.push(i);
    }
    player1.hand.clear();
    player1.hand.add_card(103);
    player1.hand.add_card(104);
    player1.hand.add_card(card_id);
    
    println!("Testing conditional choice (<3 cards)...");
    
    // Execute the ability again
    let result2 = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result2.is_ok(), "conditional choice should execute successfully");
    
    // Should have drawn 1 card (since hand had <3 cards)
    assert_eq!(player1.main_deck.cards.len(), 4, "Should have drawn 1 card from deck");
    
    println!("✅ conditional choice test passed!");
}

#[test]
fn test_choice_with_cost() {
    // Test choice ability that requires additional cost
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test card with costly choice
    let test_card = Card {
        card_no: "CHOICE-004".to_string(),
        name: "Expensive Decision".to_string(),
        card_type: CardType::Member,
        color: "Purple".to_string(),
        cost: Some(5),
        blade: Some(4),
        ability: "【自】:相手のメンバーを1体選び、手札に加える。その後、自分のエネルギーを2払う。".to_string(),
        full_text: "【自】:相手のメンバーを1体選び、手札に加える。その後、自分のエネルギーを2払う。".to_string(),
        ..Default::default()
    };
    
    let card_id = card_database.add_card(test_card);
    
    // Add cards to opponent's stage
    player2.stage.set_area(crate::zones::MemberArea::LeftSide, 300);
    
    // Add cards to player's deck
    for i in 10..=20 {
        player1.main_deck.cards.push(i);
    }
    
    // Give player energy for cost payment
    player1.energy_zone.add_energy(10);
    
    player1.hand.add_card(card_id);
    
    let mut executor = AbilityExecutor::new();
    
    println!("Testing choice with additional cost...");
    
    // Execute the ability
    let result = executor.execute_ability(
        &card_database.get_card(card_id).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(card_id),
    );
    
    // Verify results
    assert!(result.is_ok(), "choice with cost should execute successfully");
    
    // Should have stolen 1 opponent member
    assert_eq!(player1.hand.cards.len(), 2, "Should have stolen 1 opponent member");
    
    // Should have paid 2 energy
    assert_eq!(player1.energy_zone.get_energy_count(), 8, "Should have paid 2 energy");
    
    println!("✅ choice with cost test passed!");
}
