use rabuka_engine::*;
use std::collections::HashMap;

/// Test check timing system with real cards
#[test]
fn test_check_timing_basic() {
    // Initialize game components
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create trigger cards
    let trigger_card1 = Card {
        card_no: "TRIGGER-001".to_string(),
        name: "Live Start Trigger".to_string(),
        card_type: CardType::Member,
        color: "Pink".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【ライブ開始時】:カードを1枚引く。".to_string(),
        full_text: "【ライブ開始時】:カードを1枚引く。".to_string(),
        ..Default::default()
    };
    
    let trigger_card2 = Card {
        card_no: "TRIGGER-002".to_string(),
        name: "Performance Trigger".to_string(),
        card_type: CardType::Member,
        color: "Blue".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【パフォーマンスフェイズの始めに】:相手に1ダメージ。".to_string(),
        full_text: "【パフォーマンスフェイズの始めに】:相手に1ダメージ。".to_string(),
        ..Default::default()
    };
    
    let trigger1_id = card_database.add_card(trigger_card1);
    let trigger2_id = card_database.add_card(trigger_card2);
    
    // Place triggers on stage
    player1.stage.set_area(crate::zones::MemberArea::LeftSide, trigger1_id);
    player1.stage.set_area(crate::zones::MemberArea::Center, trigger2_id);
    
    // Create check timing system
    let mut check_timing = CheckTimingSystem::new();
    
    println!("Testing basic check timing...");
    
    // Add triggers to queue
    check_timing.add_trigger(
        card_database.get_card(trigger1_id).unwrap().abilities[0].clone(),
        "player1".to_string(),
        "live_start".to_string(),
    );
    
    check_timing.add_trigger(
        card_database.get_card(trigger2_id).unwrap().abilities[0].clone(),
        "player1".to_string(),
        "performance_start".to_string(),
    );
    
    // Process check timing
    let resolved = check_timing.process_check_timing(&mut game_state, "player1").unwrap();
    
    // Verify results
    assert!(!resolved.is_empty(), "Should have resolved triggers");
    assert_eq!(resolved.len(), 2, "Should have resolved 2 triggers");
    
    // Check that triggers were processed in correct order
    assert!(resolved[0].contains("live_start"), "First trigger should be live_start");
    assert!(resolved[1].contains("performance_start"), "Second trigger should be performance_start");
    
    println!("✅ basic check timing test passed!");
}

#[test]
fn test_check_timing_priority() {
    // Test trigger priority (active player first)
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string());
    
    // Create multiple triggers for both players
    let p1_trigger = Card {
        card_no: "PRIORITY-001".to_string(),
        name: "Player 1 Trigger".to_string(),
        card_type: CardType::Member,
        color: "Red".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【ターンの始めに】:カードを1枚引く。".to_string(),
        full_text: "【ターンの始めに】:カードを1枚引く。".to_string(),
        ..Default::default()
    };
    
    let p2_trigger = Card {
        card_no: "PRIORITY-002".to_string(),
        name: "Player 2 Trigger".to_string(),
        card_type: CardType::Member,
        color: "Blue".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【ターンの始めに】:カードを1枚引く。".to_string(),
        full_text: "【ターンの始めに】:カードを1枚引く。".to_string(),
        ..Default::default()
    };
    
    let p1_id = card_database.add_card(p1_trigger);
    let p2_id = card_database.add_card(p2_trigger);
    
    // Place triggers on stage
    player1.stage.set_area(crate::zones::MemberArea::LeftSide, p1_id);
    player2.stage.set_area(crate::zones::MemberArea::LeftSide, p2_id);
    
    let mut check_timing = CheckTimingSystem::new();
    
    println!("Testing check timing priority...");
    
    // Add triggers (player1 is active player)
    check_timing.add_trigger(
        card_database.get_card(p1_id).unwrap().abilities[0].clone(),
        "player1".to_string(),
        "turn_start".to_string(),
    );
    
    check_timing.add_trigger(
        card_database.get_card(p2_id).unwrap().abilities[0].clone(),
        "player2".to_string(),
        "turn_start".to_string(),
    );
    
    // Process check timing with player1 as active player
    let resolved = check_timing.process_check_timing(&mut game_state, "player1").unwrap();
    
    // Verify results - active player's trigger should resolve first
    assert_eq!(resolved.len(), 2, "Should resolve both triggers");
    assert!(resolved[0].contains("player1"), "Active player's trigger should resolve first");
    assert!(resolved[1].contains("player2"), "Non-active player's trigger should resolve second");
    
    println!("✅ check timing priority test passed!");
}

#[test]
fn test_check_timing_multiple_same_ability() {
    // Test multiple triggers of same ability
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    
    // Create ability that can trigger multiple times
    let multi_trigger = Card {
        card_no: "MULTI-001".to_string(),
        name: "Multi-Trigger Member".to_string(),
        card_type: CardType::Member,
        color: "Green".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【ライブ成功時】:カードを1枚引く。".to_string(),
        full_text: "【ライブ成功時】:カードを1枚引く。".to_string(),
        ..Default::default()
    };
    
    let multi_id = card_database.add_card(multi_trigger);
    
    // Place on stage
    player1.stage.set_area(crate::zones::MemberArea::Center, multi_id);
    
    let mut check_timing = CheckTimingSystem::new();
    
    println!("Testing multiple same ability triggers...");
    
    // Add same trigger twice
    check_timing.add_trigger(
        card_database.get_card(multi_id).unwrap().abilities[0].clone(),
        "player1".to_string(),
        "live_success".to_string(),
    );
    
    check_timing.add_trigger(
        card_database.get_card(multi_id).unwrap().abilities[0].clone(),
        "player1".to_string(),
        "live_success".to_string(),
    );
    
    // Process check timing
    let resolved = check_timing.process_check_timing(&mut game_state, "player1").unwrap();
    
    // Verify results - should handle multiple triggers
    assert_eq!(resolved.len(), 2, "Should resolve both triggers");
    
    // Check that trigger count was tracked
    // (This depends on implementation details)
    
    println!("✅ multiple same ability triggers test passed!");
}

#[test]
fn test_check_timing_empty_queue() {
    // Test check timing with no pending triggers
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    
    let mut check_timing = CheckTimingSystem::new();
    
    println!("Testing empty check timing queue...");
    
    // Process check timing with no triggers
    let resolved = check_timing.process_check_timing(&mut game_state, "player1").unwrap();
    
    // Verify results
    assert!(resolved.is_empty(), "Should resolve no triggers when queue is empty");
    
    println!("✅ empty check timing queue test passed!");
}

#[test]
fn test_check_timing_clear() {
    // Test clearing check timing system
    let mut card_database = CardDatabase::new();
    let mut game_state = GameState::new();
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string());
    
    // Create and add a trigger
    let trigger_card = Card {
        card_no: "CLEAR-001".to_string(),
        name: "Clear Test Trigger".to_string(),
        card_type: CardType::Member,
        color: "Purple".to_string(),
        cost: Some(1),
        blade: Some(1),
        ability: "【登場時】:カードを1枚引く。".to_string(),
        full_text: "【登場時】:カードを1枚引く。".to_string(),
        ..Default::default()
    };
    
    let trigger_id = card_database.add_card(trigger_card);
    
    player1.stage.set_area(crate::zones::MemberArea::LeftSide, trigger_id);
    
    let mut check_timing = CheckTimingSystem::new();
    
    println!("Testing check timing clear...");
    
    // Add trigger
    check_timing.add_trigger(
        card_database.get_card(trigger_id).unwrap().abilities[0].clone(),
        "player1".to_string(),
        "debut".to_string(),
    );
    
    // Verify trigger was added
    assert_eq!(check_timing.pending_triggers.len(), 1, "Should have 1 pending trigger");
    
    // Clear triggers
    check_timing.clear_triggers();
    
    // Verify triggers were cleared
    assert!(check_timing.pending_triggers.is_empty(), "Should have no pending triggers after clear");
    assert!(check_timing.active_triggers.is_empty(), "Should have no active triggers after clear");
    
    println!("✅ check timing clear test passed!");
}
