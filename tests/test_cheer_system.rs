use rabuka_engine::*;
use std::collections::HashMap;

/// Test cheer system with real cards
#[test]
fn test_cheer_basic() {
    // Initialize game components
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create test member cards with blades
    let member1 = Card {
        card_no: "MEMBER-001".to_string(),
        name: "Singing Member".to_string(),
        card_type: CardType::Member,
        color: "Pink".to_string(),
        cost: Some(1),
        blade: Some(2), // 2 blades
        blade_heart: Some(BladeHeart {
            hearts: HashMap::from([
                (crate::card::BladeColor::Peach, 1),
                (crate::card::BladeColor::Red, 1),
            ]),
        }),
        ..Default::default()
    };
    
    let member2 = Card {
        card_no: "MEMBER-002".to_string(),
        name: "Dancing Member".to_string(),
        card_type: CardType::Member,
        color: "Blue".to_string(),
        cost: Some(2),
        blade: Some(1), // 1 blade
        blade_heart: Some(BladeHeart {
            hearts: HashMap::from([
                (crate::card::BladeColor::Yellow, 2),
            ]),
        }),
        ..Default::default()
    };
    
    let member3 = Card {
        card_no: "MEMBER-003".to_string(),
        name: "Acting Member".to_string(),
        card_type: CardType::Member,
        color: "Green".to_string(),
        cost: Some(3),
        blade: Some(0), // 0 blades
        blade_heart: Some(BladeHeart {
            hearts: HashMap::from([
                (crate::card::BladeColor::Purple, 1),
            ]),
        }),
        ..Default::default()
    };
    
    // Add cards to database
    let member1_id = card_database.add_card(member1);
    let member2_id = card_database.add_card(member2);
    let member3_id = card_database.add_card(member3);
    
    // Place members on stage
    player1.stage.set_area(crate::zones::MemberArea::LeftSide, member1_id);
    player1.stage.set_area(crate::zones::MemberArea::Center, member2_id);
    player1.stage.set_area(crate::zones::MemberArea::RightSide, member3_id);
    
    // Add cards to deck for cheering
    for i in 100..=120 {
        player1.main_deck.cards.push(i);
    }
    
    // Create cheer system
    let mut cheer_system = CheerSystem::new();
    
    println!("Testing basic cheer system...");
    
    // Execute cheer
    let result = cheer_system.execute_cheer(
        &mut player1,
        &mut game_state,
        &card_database,
    );
    
    // Verify results
    assert!(result.is_ok(), "cheer should execute successfully");
    
    // Should have counted total blades (2 + 1 + 0 = 3)
    assert_eq!(cheer_system.total_blade_count, 3, "Should count 3 total blades");
    
    // Should have revealed 3 cards from deck
    assert_eq!(cheer_system.cheer_cards.len(), 3, "Should have revealed 3 cards");
    
    // Should have extracted heart icons
    assert!(!cheer_system.heart_icons_extracted.is_empty(), "Should have extracted heart icons");
    
    // Should have extracted hearts: 1 Peach, 1 Red, 2 Yellow, 1 Purple = 5 total
    assert_eq!(cheer_system.get_total_heart_count(), 5, "Should have 5 total hearts");
    
    // Should have drawn 5 cards from deck
    assert_eq!(player1.main_deck.cards.len(), 115, "Should have drawn 5 cards from deck (120 - 5)");
    
    println!("✅ basic cheer test passed!");
}
