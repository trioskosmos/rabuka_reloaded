// Basic ability resolution tests
// Tests fundamental ability resolver functionality without complex game scenarios

use rabuka_engine::card::{Ability, AbilityEffect, Condition};
use rabuka_engine::ability_resolver::AbilityResolver;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;

#[test]
fn test_basic_condition_evaluation_location() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let game_state = GameState::new(player1, player2);
    
    let resolver = AbilityResolver::new(&mut game_state);
    
    // Test location condition - check if hand is not empty
    let condition = Condition {
        text: "Hand is not empty".to_string(),
        condition_type: Some("location_condition".to_string()),
        location: Some("hand".to_string()),
        target: Some("self".to_string()),
        count: None,
        operator: None,
        card_type: None,
        group: None,
        group_names: None,
        characters: None,
        state: None,
        position: None,
        temporal_scope: None,
        distinct: None,
        unique: None,
        exclude_self: None,
        any_of: None,
        cost_limit: None,
        exact_match: None,
        negation: None,
        includes_pattern: None,
        movement_condition: None,
        baton_touch_trigger: None,
        movement_state: None,
        energy_state: None,
        aggregate_flags: None,
        comparison_target: None,
        comparison_operator: None,
        movement: None,
        heart_variety: None,
        activation_condition: None,
        activation_position: None,
        trigger_type: None,
        trigger_event: None,
        temporal: None,
        phase: None,
        aggregate: None,
        comparison_type: None,
        includes: None,
        appearance: None,
        conditions: None,
    };
    
    // With initial hand (6 cards from setup), condition should be true
    let result = resolver.evaluate_condition(&condition);
    assert!(result, "Hand should not be empty initially");
}

#[test]
fn test_basic_condition_evaluation_empty_location() {
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Clear hand to test empty condition
    player1.hand.cards.clear();
    
    let game_state = GameState::new(player1, player2);
    let resolver = AbilityResolver::new(&mut game_state);
    
    let condition = Condition {
        text: "Hand is empty".to_string(),
        condition_type: Some("location_condition".to_string()),
        location: Some("hand".to_string()),
        target: Some("self".to_string()),
        count: None,
        operator: None,
        card_type: None,
        group: None,
        group_names: None,
        characters: None,
        state: None,
        position: None,
        temporal_scope: None,
        distinct: None,
        unique: None,
        exclude_self: None,
        any_of: None,
        cost_limit: None,
        exact_match: None,
        negation: None,
        includes_pattern: None,
        movement_condition: None,
        baton_touch_trigger: None,
        movement_state: None,
        energy_state: None,
        aggregate_flags: None,
        comparison_target: None,
        comparison_operator: None,
        movement: None,
        heart_variety: None,
        activation_condition: None,
        activation_position: None,
        trigger_type: None,
        trigger_event: None,
        temporal: None,
        phase: None,
        aggregate: None,
        comparison_type: None,
        includes: None,
        appearance: None,
        conditions: None,
    };
    
    // With empty hand, condition should be false
    let result = resolver.evaluate_condition(&condition);
    assert!(!result, "Hand should be empty");
}

#[test]
fn test_basic_condition_comparison() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let game_state = GameState::new(player1, player2);
    
    let resolver = AbilityResolver::new(&mut game_state);
    
    // Test comparison condition: hand count >= 3
    let condition = Condition {
        text: "Hand count >= 3".to_string(),
        condition_type: Some("comparison_condition".to_string()),
        location: Some("hand".to_string()),
        target: Some("self".to_string()),
        count: Some(3),
        operator: Some(">=".to_string()),
        card_type: None,
        group: None,
        group_names: None,
        characters: None,
        state: None,
        position: None,
        temporal_scope: None,
        distinct: None,
        unique: None,
        exclude_self: None,
        any_of: None,
        cost_limit: None,
        exact_match: None,
        negation: None,
        includes_pattern: None,
        movement_condition: None,
        baton_touch_trigger: None,
        movement_state: None,
        energy_state: None,
        aggregate_flags: None,
        comparison_target: None,
        comparison_operator: None,
        movement: None,
        heart_variety: None,
        activation_condition: None,
        activation_position: None,
        trigger_type: None,
        trigger_event: None,
        temporal: None,
        phase: None,
        aggregate: None,
        comparison_type: None,
        includes: None,
        appearance: None,
        conditions: None,
    };
    
    // Initial hand has 6 cards from setup
    let result = resolver.evaluate_condition(&condition);
    assert!(result, "Hand count (6) should be >= 3");
}

#[test]
fn test_basic_condition_comparison_false() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let game_state = GameState::new(player1, player2);
    
    let resolver = AbilityResolver::new(&mut game_state);
    
    // Test comparison condition: hand count >= 100
    let condition = Condition {
        text: "Hand count >= 100".to_string(),
        condition_type: Some("comparison_condition".to_string()),
        location: Some("hand".to_string()),
        target: Some("self".to_string()),
        count: Some(100),
        operator: Some(">=".to_string()),
        card_type: None,
        group: None,
        group_names: None,
        characters: None,
        state: None,
        position: None,
        temporal_scope: None,
        distinct: None,
        unique: None,
        exclude_self: None,
        any_of: None,
        cost_limit: None,
        exact_match: None,
        negation: None,
        includes_pattern: None,
        movement_condition: None,
        baton_touch_trigger: None,
        movement_state: None,
        energy_state: None,
        aggregate_flags: None,
        comparison_target: None,
        comparison_operator: None,
        movement: None,
        heart_variety: None,
        activation_condition: None,
        activation_position: None,
        trigger_type: None,
        trigger_event: None,
        temporal: None,
        phase: None,
        aggregate: None,
        comparison_type: None,
        includes: None,
        appearance: None,
        conditions: None,
    };
    
    // Initial hand has 6 cards, not >= 100
    let result = resolver.evaluate_condition(&condition);
    assert!(!result, "Hand count (6) should not be >= 100");
}
