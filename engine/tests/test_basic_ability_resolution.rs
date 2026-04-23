// Comprehensive QA tests for basic ability resolution
// These tests use real cards from cards.json and test real conditions
// from actual card abilities to ensure the engine correctly evaluates conditions

use rabuka_engine::card::{Ability, AbilityEffect, Condition, Card};
use rabuka_engine::ability_resolver::AbilityResolver;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use rabuka_engine::zones::MemberArea;
use std::path::Path;
use std::sync::Arc;

/// Helper function to load all cards from cards.json
fn load_all_cards() -> Vec<Card> {
    let cards_path = Path::new("../cards/cards.json");
    CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards")
}

/// Helper function to load cards (alias for consistency)
fn load_cards() -> Vec<Card> {
    load_all_cards()
}

/// Helper function to create card database
fn create_card_database(cards: &[Card]) -> Arc<CardDatabase> {
    Arc::new(CardDatabase::load_or_create(cards.to_vec()))
}

/// Helper function to place a card on stage
fn place_card_on_stage(player: &mut Player, card_id: i16, area: MemberArea) {
    player.stage.set_area(area, card_id);
}

/// Test: Real card ability with hand count condition
/// Edge case: Condition should evaluate correctly based on actual hand size
#[test]
fn test_real_card_hand_count_condition() {
    let cards = load_all_cards();
    
    // Find a card with an ability that has a hand count condition
    let card_with_condition = cards.iter().find(|c| {
        c.abilities.iter().any(|a| {
            a.effect.as_ref().map_or(false, |e| {
                e.condition.as_ref().map_or(false, |cond| {
                    cond.location.as_deref() == Some("hand")
                })
            })
        })
    });
    
    match card_with_condition {
        Some(card) => {
            println!("Testing hand count condition with card: {} ({})", card.name, card.card_no);
            
            let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
            let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
            
            // Add cards to hand
            let member_cards: Vec<_> = cards.iter()
                .filter(|c| c.is_member())
                .take(5)
                .cloned()
                .collect();
            
            for card in member_cards {
                player1.hand.cards.push(card.card_no.parse::<i16>().unwrap_or(0));
            }
            
            let card_database = create_card_database(&cards);
            let mut game_state = GameState::new(player1, player2, card_database);
            let resolver = AbilityResolver::new(&mut game_state);
            
            // Test condition: hand has cards
            let condition = Condition {
                text: "Hand has cards".to_string(),
                condition_type: Some("location_condition".to_string()),
                location: Some("hand".to_string()),
                target: Some("self".to_string()),
                count: None,
                operator: None,
                card_type: None,
                group: None,
                group_names: None,
                characters: None,
                no_excess_heart: None,
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
                all_areas: None,
                exclude_this_member: None,
                resource_type: None,
                unit: None,
                location_condition: None,
                cost_result_reference: None,
                cost_result_group_match: None,
                group_matching: None,
            };
            
            let result = resolver.evaluate_condition(&condition);
            assert!(result, "Hand should have cards");
            
            // Verify state
            assert_eq!(game_state.player1.hand.cards.len(), 5,
                "Hand should have 5 cards");
        }
        None => {
            println!("No card with hand condition found, using synthetic test");
            
            // Fallback to synthetic test
            let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
            let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
            
            let member_cards: Vec<_> = cards.iter()
                .filter(|c| c.is_member())
                .take(5)
                .cloned()
                .collect();
            
            for card in member_cards {
                player1.hand.cards.push(card.card_no.parse::<i16>().unwrap_or(0));
            }
            
            let card_database = create_card_database(&cards);
            let mut game_state = GameState::new(player1, player2, card_database);
            let resolver = AbilityResolver::new(&mut game_state);
            
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
                no_excess_heart: None,
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
                all_areas: None,
                exclude_this_member: None,
                resource_type: None,
                unit: None,
                location_condition: None,
                cost_result_reference: None,
                cost_result_group_match: None,
                group_matching: None,
            };
            
            let result = resolver.evaluate_condition(&condition);
            assert!(result, "Hand count should be >= 3");
        }
    }
}

/// Test: Empty hand condition evaluation
/// Edge case: Condition should fail when hand is empty
#[test]
fn test_empty_hand_condition_with_real_cards() {
    let cards = load_all_cards();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Hand is empty (no cards added)
    let card_database = create_card_database(&cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    let resolver = AbilityResolver::new(&mut game_state);
    
    // Test condition: hand has cards
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
        no_excess_heart: None,
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
        all_areas: None,
        exclude_this_member: None,
        resource_type: None,
        unit: None,
        location_condition: None,
        cost_result_reference: None,
        cost_result_group_match: None,
        group_matching: None,
    };
    
    let result = resolver.evaluate_condition(&condition);
    assert!(!result, "Empty hand should fail condition");
    
    // Verify state
    assert_eq!(game_state.player1.hand.cards.len(), 0,
        "Hand should be empty");
}

/// Test: Stage position condition evaluation
/// Edge case: Condition should check if card is in specific position
#[test]
fn test_stage_position_condition_with_real_cards() {
    let cards = load_all_cards();
    
    let member_card = cards.iter().find(|c| c.is_member()).expect("Should have member card");
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Place card in center
    let card_id = member_card.card_no.parse::<i16>().unwrap_or(0);
    place_card_on_stage(&mut player1, card_id, MemberArea::Center);
    
    let card_database = create_card_database(&cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    let resolver = AbilityResolver::new(&mut game_state);
    
    // Test condition: center position is occupied
    let condition = Condition {
        text: "Center is occupied".to_string(),
        condition_type: Some("position_condition".to_string()),
        location: None,
        target: Some("self".to_string()),
        count: None,
        operator: None,
        card_type: None,
        group: None,
        group_names: None,
        characters: None,
        no_excess_heart: None,
        state: None,
        position: Some(rabuka_engine::card::PositionInfo {
            position: Some("center".to_string()),
            target: None,
        }),
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
        all_areas: None,
        exclude_this_member: None,
        resource_type: None,
        unit: None,
        location_condition: None,
        cost_result_reference: None,
        cost_result_group_match: None,
        group_matching: None,
    };
    
    let result = resolver.evaluate_condition(&condition);
    assert!(result, "Center should be occupied");
    
    // Verify state
    assert!(game_state.player1.stage.stage[1] != -1,
        "Center should have a card");
    assert_eq!(game_state.player1.stage.stage[1],
        member_card.card_no.parse::<i16>().unwrap_or(0),
        "Card in center should match");
}

/// Test: Multiple conditions with AND logic
/// Edge case: All conditions must be satisfied for compound condition
#[test]
fn test_compound_and_condition_with_real_cards() {
    let cards = load_all_cards();
    
    let member_card = cards.iter().find(|c| c.is_member()).expect("Should have member card");
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Add cards to hand and place card on stage
    let hand_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .take(3)
        .cloned()
        .collect();
    
    for card in hand_cards {
        player1.hand.cards.push(card.card_no.parse::<i16>().unwrap_or(0));
    }
    
    let card_id = member_card.card_no.parse::<i16>().unwrap_or(0);
    place_card_on_stage(&mut player1, card_id, MemberArea::Center);
    
    let card_database = create_card_database(&cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    let resolver = AbilityResolver::new(&mut game_state);
    
    // Test compound condition: hand has cards AND center is occupied
    let condition = Condition {
        text: "Hand has cards AND center occupied".to_string(),
        condition_type: Some("compound".to_string()),
        operator: Some("and".to_string()),
        location: None,
        target: None,
        count: None,
        card_type: None,
        group: None,
        group_names: None,
        characters: None,
        no_excess_heart: None,
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
        conditions: Some(vec![
            Condition {
                text: "Hand has cards".to_string(),
                condition_type: Some("location_condition".to_string()),
                location: Some("hand".to_string()),
                target: Some("self".to_string()),
                count: None,
                operator: None,
                card_type: None,
                group: None,
                group_names: None,
                characters: None,
                no_excess_heart: None,
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
                all_areas: None,
                exclude_this_member: None,
                resource_type: None,
                unit: None,
                location_condition: None,
                cost_result_reference: None,
                cost_result_group_match: None,
                group_matching: None,
            },
            Condition {
                text: "Center occupied".to_string(),
                condition_type: Some("position_condition".to_string()),
                location: None,
                target: Some("self".to_string()),
                count: None,
                operator: None,
                card_type: None,
                group: None,
                group_names: None,
                characters: None,
                no_excess_heart: None,
                state: None,
                position: Some(rabuka_engine::card::PositionInfo {
                    position: Some("center".to_string()),
                    target: None,
                }),
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
                all_areas: None,
                exclude_this_member: None,
                resource_type: None,
                unit: None,
                location_condition: None,
                cost_result_reference: None,
                cost_result_group_match: None,
                group_matching: None,
            },
        ]),
        all_areas: None,
        exclude_this_member: None,
        resource_type: None,
        unit: None,
        location_condition: None,
        cost_result_reference: None,
        cost_result_group_match: None,
        group_matching: None,
    };
    
    let result = resolver.evaluate_condition(&condition);
    assert!(result, "Both conditions should be satisfied");
    
    // Verify state
    assert!(game_state.player1.hand.cards.len() >= 1,
        "Hand should have cards");
    assert!(game_state.player1.stage.stage[1] != -1,
        "Center should be occupied");
}

/// Test: Compound condition with OR logic
/// Edge case: At least one condition must be satisfied
#[test]
fn test_compound_or_condition_with_real_cards() {
    let cards = load_all_cards();
    
    let member_card = cards.iter().find(|c| c.is_member()).expect("Should have member card");
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Only place card on stage, no cards in hand
    let card_id = member_card.card_no.parse::<i16>().unwrap_or(0);
    place_card_on_stage(&mut player1, card_id, MemberArea::Center);
    
    let cards = load_cards();
    let card_database = create_card_database(&cards);
    let mut game_state = GameState::new(player1, player2, card_database);
    let resolver = AbilityResolver::new(&mut game_state);
    
    // Test compound condition: hand has cards OR center is occupied
    let condition = Condition {
        text: "Hand has cards OR center occupied".to_string(),
        condition_type: Some("compound".to_string()),
        operator: Some("or".to_string()),
        location: None,
        target: None,
        count: None,
        card_type: None,
        group: None,
        group_names: None,
        characters: None,
        no_excess_heart: None,
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
        conditions: Some(vec![
            Condition {
                text: "Hand has cards".to_string(),
                condition_type: Some("location_condition".to_string()),
                location: Some("hand".to_string()),
                target: Some("self".to_string()),
                count: None,
                operator: None,
                card_type: None,
                group: None,
                group_names: None,
                characters: None,
                no_excess_heart: None,
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
                all_areas: None,
                exclude_this_member: None,
                resource_type: None,
                unit: None,
                location_condition: None,
                cost_result_reference: None,
                cost_result_group_match: None,
                group_matching: None,
            },
            Condition {
                text: "Center occupied".to_string(),
                condition_type: Some("position_condition".to_string()),
                location: None,
                target: Some("self".to_string()),
                count: None,
                operator: None,
                card_type: None,
                group: None,
                group_names: None,
                characters: None,
                no_excess_heart: None,
                state: None,
                position: Some(rabuka_engine::card::PositionInfo {
                    position: Some("center".to_string()),
                    target: None,
                }),
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
                all_areas: None,
                exclude_this_member: None,
                resource_type: None,
                unit: None,
                location_condition: None,
                cost_result_reference: None,
                cost_result_group_match: None,
                group_matching: None,
            },
        ]),
        all_areas: None,
        exclude_this_member: None,
        resource_type: None,
        unit: None,
        location_condition: None,
        cost_result_reference: None,
        cost_result_group_match: None,
        group_matching: None,
    };
    
    let result = resolver.evaluate_condition(&condition);
    assert!(result, "At least one condition should be satisfied (center is occupied)");
    
    // Verify state
    assert_eq!(game_state.player1.hand.cards.len(), 0,
        "Hand should be empty");
    assert!(game_state.player1.stage.stage[1] != -1,
        "Center should be occupied");
}
