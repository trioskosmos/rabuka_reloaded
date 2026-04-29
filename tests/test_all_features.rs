//! Integration test for all implemented features
//! This test validates that all of implemented systems work together end-to-end

use rabuka_engine::card::{CardDatabase, Card, CardType};
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::ability::AbilityExecutor;
use rabuka_engine::cheer_system::CheerSystem;
use rabuka_engine::selection_system::SelectionSystem;
use rabuka_engine::check_timing::CheckTimingSystem;
use std::collections::HashMap;

/// Comprehensive end-to-end test
#[test]
fn test_all_features_integration() {
    println!("🧪 Running comprehensive integration test...");
    
    // Initialize all game components
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create comprehensive test cards
    let cards = create_test_cards(&mut card_database);
    
    // Setup game state
    setup_game_state(&mut game_state, &mut player1, &mut player2, &cards);
    
    // Initialize all systems
    let mut executor = AbilityExecutor::new();
    let mut selection_system = SelectionSystem::new();
    let mut cheer_system = CheerSystem::new();
    let mut check_timing = CheckTimingSystem::new();
    
    println!("✅ All systems initialized successfully");
    
    // Test 1: Execute look_and_select ability
    println!("🔍 Test 1: look_and_select ability");
    let result1 = executor.execute_ability(
        &card_database.get_card(cards.look_and_select).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(cards.look_and_select),
    );
    assert!(result1.is_ok(), "look_and_select should work");
    println!("✅ look_and_select test passed");
    
    // Test 2: Execute choice ability
    println!("🔍 Test 2: choice ability");
    let result2 = executor.execute_ability(
        &card_database.get_card(cards.choice).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(cards.choice),
    );
    assert!(result2.is_ok(), "choice should work");
    println!("✅ choice test passed");
    
    // Test 3: Execute dynamic count ability
    println!("🔍 Test 3: dynamic count ability");
    let result3 = executor.execute_ability(
        &card_database.get_card(cards.dynamic_count).unwrap().abilities[0].clone(),
        &mut player1,
        &mut game_state,
        "player1",
        Some(cards.dynamic_count),
    );
    assert!(result3.is_ok(), "dynamic count should work");
    println!("✅ dynamic count test passed");
    
    // Test 4: Execute cheer system
    println!("🔍 Test 4: cheer system");
    let result4 = cheer_system.execute_cheer(
        &mut player1,
        &mut game_state,
        &card_database,
    );
    assert!(result4.is_ok(), "cheer system should work");
    assert_eq!(cheer_system.total_blade_count, 3, "Should count 3 blades");
    println!("✅ cheer system test passed");
    
    // Test 5: Execute check timing
    println!("🔍 Test 5: check timing system");
    check_timing.add_trigger(
        card_database.get_card(cards.trigger).unwrap().abilities[0].clone(),
        "player1".to_string(),
        "live_start".to_string(),
    );
    
    let resolved = check_timing.process_check_timing(&mut game_state, "player1").unwrap();
    assert!(!resolved.is_empty(), "check timing should resolve triggers");
    println!("✅ check timing test passed");
    
    println!("🎉 ALL INTEGRATION TESTS PASSED!");
    println!("📊 Summary:");
    println!("   ✅ look_and_select abilities working");
    println!("   ✅ choice abilities working");
    println!("   ✅ dynamic count abilities working");
    println!("   ✅ cheer system working");
    println!("   ✅ check timing system working");
    println!("   ✅ All systems integrated successfully");
}

fn create_test_cards(card_database: &mut CardDatabase) -> TestCards {
    // Create cards for comprehensive testing
    let look_and_select = create_look_and_select_card(card_database);
    let choice = create_choice_card(card_database);
    let dynamic_count = create_dynamic_count_card(card_database);
    let trigger = create_trigger_card(card_database);
    
    TestCards {
        look_and_select,
        choice,
        dynamic_count,
        trigger,
    }
}

fn create_look_and_select_card(card_database: &mut CardDatabase) -> i16 {
    let card = Card {
        card_no: "INTEGRATION-001".to_string(),
        name: "Integration Scout".to_string(),
        card_type: CardType::Member,
        color: "Blue".to_string(),
        cost: Some(2),
        blade: Some(1),
        ability: "【自】:相手のデッキの上からカードを2枚見て、1枚選び、手札に加える。".to_string(),
        full_text: "【自】:相手のデッキの上からカードを2枚見て、1枚選び、手札に加える。".to_string(),
        ..Default::default()
    };
    card_database.add_card(card)
}

fn create_choice_card(card_database: &mut CardDatabase) -> i16 {
    let card = Card {
        card_no: "INTEGRATION-002".to_string(),
        name: "Integration Choice".to_string(),
        card_type: CardType::Member,
        color: "Red".to_string(),
        cost: Some(3),
        blade: Some(2),
        ability: "【自】:カードを1枚引く。その後、以下から1つ選ぶ：A)相手に2ダメージ、B)自分の手札を1枚捨てる。".to_string(),
        full_text: "【自】:カードを1枚引く。その後、以下から1つ選ぶ：A)相手に2ダメージ、B)自分の手札を1枚捨てる。".to_string(),
        ..Default::default()
    };
    card_database.add_card(card)
}

fn create_dynamic_count_card(card_database: &mut CardDatabase) -> i16 {
    let card = Card {
        card_no: "INTEGRATION-003".to_string(),
        name: "Integration Dynamic".to_string(),
        card_type: CardType::Member,
        color: "Green".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【自】:相手のデッキの上から好きな枚数選び、手札に加える。".to_string(),
        full_text: "【自】:相手のデッキの上から好きな枚数選び、手札に加える。".to_string(),
        ..Default::default()
    };
    card_database.add_card(card)
}

fn create_trigger_card(card_database: &mut CardDatabase) -> i16 {
    let card = Card {
        card_no: "INTEGRATION-004".to_string(),
        name: "Integration Trigger".to_string(),
        card_type: CardType::Member,
        color: "Purple".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【ライブ開始時】:カードを1枚引く。".to_string(),
        full_text: "【ライブ開始時】:カードを1枚引く。".to_string(),
        ..Default::default()
    };
    card_database.add_card(card)
}

fn setup_game_state(game_state: &mut GameState, player1: &mut Player, player2: &mut Player, cards: &TestCards) {
    // Add cards to opponent's deck for looking at
    for i in 1..=5 {
        player2.main_deck.cards.push(cards.look_and_select + i);
    }
    
    // Add cards to player's deck for drawing
    for i in 100..=120 {
        player1.main_deck.cards.push(i);
    }
    
    // Add cards to player's hand for discarding
    for i in 200..=205 {
        player1.hand.add_card(i);
    }
    
    // Add opponent member for targeting
    player2.stage.set_area(crate::zones::MemberArea::LeftSide, 500);
    
    // Place trigger card on stage
    player1.stage.set_area(crate::zones::MemberArea::Center, cards.trigger);
    
    // Add cards to player's hand
    player1.hand.add_card(cards.look_and_select);
    player1.hand.add_card(cards.choice);
    player1.hand.add_card(cards.dynamic_count);
}

struct TestCards {
    look_and_select: i16,
    choice: i16,
    dynamic_count: i16,
    trigger: i16,
}
