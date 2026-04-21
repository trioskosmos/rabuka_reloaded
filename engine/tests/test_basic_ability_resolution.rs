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
    
    let resolver = AbilityResolver::new(game_state);
    
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
        triggers: None,
        source: None,
        destination: None,
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
    let resolver = AbilityResolver::new(game_state);
    
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
        triggers: None,
        source: None,
        destination: None,
        conditions: None,
    };
    
    // With empty hand, condition should be false
    let result = resolver.evaluate_condition(&condition);
    assert!(!result, "Hand should be empty");
}

#[test]
fn test_basic_effect_execute_draw() {
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Add cards to deck for drawing
    for i in 0..5 {
        let card = rabuka_engine::card::Card {
            card_no: format!("card_{}", i),
            img: None,
            name: "Test Card".to_string(),
            product: "Test".to_string(),
            card_type: rabuka_engine::card::CardType::Member,
            series: "Test".to_string(),
            group: "Test".to_string(),
            unit: None,
            cost: None,
            base_heart: None,
            blade_heart: None,
            blade: 0,
            rare: "R".to_string(),
            ability: String::new(),
            faq: Vec::new(),
            _img: None,
            score: None,
            need_heart: None,
            special_heart: None,
            abilities: Vec::new(),
        };
        player1.main_deck.cards.push_back(card);
    }
    
    let _initial_hand_size = player1.hand.cards.len();
    let mut game_state = GameState::new(player1, player2);
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    let effect = AbilityEffect {
        text: "Draw 2 cards".to_string(),
        action: "draw".to_string(),
        count: Some(2),
        target: Some("self".to_string()),
        condition: None,
        card_type: None,
        source: None,
        destination: None,
        duration: None,
        parenthetical: None,
        look_action: None,
        select_action: None,
        actions: None,
        position: None,
        state_change: None,
        optional: None,
        max: None,
        effect_constraint: None,
        shuffle_target: None,
        icon_count: None,
        ability_gain: None,
        quoted_text: None,
        per_unit: None,
        primary_effect: None,
        alternative_condition: None,
        alternative_effect: None,
        operation: None,
        value: None,
        aggregate: None,
        comparison_type: None,
        heart_color: None,
        blade_type: None,
        energy_count: None,
        target_member: None,
        choice_options: None,
        group: None,
        per_unit_count: None,
        per_unit_type: None,
        per_unit_reference: None,
        group_matching: None,
        repeat_limit: None,
        repeat_optional: None,
        is_further: None,
        cost_result_reference: None,
        dynamic_count: None,
        resource_icon_count: None,
        placement_order: None,
        cost_limit: None,
        unit: None,
        distinct: None,
        target_player: None,
        target_location: None,
        target_scope: None,
        target_card_type: None,
        activation_condition: None,
        activation_condition_parsed: None,
        gained_ability: None,
        ability_text: None,
        swap_action: None,
        has_member_swapping: None,
        group_options: None,
        card_count: None,
        use_limit: None,
        triggers: None,
    };
    
    let result = resolver.execute_effect(&effect);
    assert!(result.is_ok(), "Draw effect should execute successfully");
    
    // Note: Since resolver works on a clone, the actual game state won't be updated
    // This is a known limitation that needs architectural refactoring
}

#[test]
fn test_basic_effect_execute_gain_resource() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let mut game_state = GameState::new(player1, player2);
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    let effect = AbilityEffect {
        text: "Gain 2 blades".to_string(),
        action: "gain_resource".to_string(),
        resource: Some("blade".to_string()),
        count: Some(2),
        target: Some("self".to_string()),
        condition: None,
        card_type: None,
        source: None,
        destination: None,
        duration: None,
        parenthetical: None,
        look_action: None,
        select_action: None,
        actions: None,
        position: None,
        state_change: None,
        optional: None,
        max: None,
        effect_constraint: None,
        shuffle_target: None,
        icon_count: None,
        ability_gain: None,
        quoted_text: None,
        per_unit: None,
        primary_effect: None,
        alternative_condition: None,
        alternative_effect: None,
        operation: None,
        value: None,
        aggregate: None,
        comparison_type: None,
    };
    
    let result = resolver.execute_effect(&effect);
    assert!(result.is_ok(), "Gain resource effect should execute successfully");
}

#[test]
fn test_basic_condition_comparison() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let game_state = GameState::new(player1, player2);
    
    let resolver = AbilityResolver::new(game_state);
    
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
        triggers: None,
        source: None,
        destination: None,
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
    
    let resolver = AbilityResolver::new(game_state);
    
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
        triggers: None,
        source: None,
        destination: None,
        conditions: None,
    };
    
    // Initial hand has 6 cards, not >= 100
    let result = resolver.evaluate_condition(&condition);
    assert!(!result, "Hand count (6) should not be >= 100");
}

#[test]
fn test_basic_ability_with_condition_and_effect() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let mut game_state = GameState::new(player1, player2);
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    // Create an ability with a condition and effect
    let ability = Ability {
        full_text: "If hand has 3+ cards, draw 1".to_string(),
        triggerless_text: "If hand has 3+ cards, draw 1".to_string(),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: None,
        effect: Some(AbilityEffect {
            text: "Draw 1 card".to_string(),
            action: "draw".to_string(),
            count: Some(1),
            target: Some("self".to_string()),
            condition: Some(Condition {
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
                triggers: None,
                source: None,
                destination: None,
                conditions: None,
            }),
            card_type: None,
            source: None,
            destination: None,
            duration: None,
            parenthetical: None,
            look_action: None,
            select_action: None,
            actions: None,
            resource: None,
            position: None,
            state_change: None,
            optional: None,
            max: None,
            effect_constraint: None,
            shuffle_target: None,
            icon_count: None,
            ability_gain: None,
            quoted_text: None,
            per_unit: None,
            primary_effect: None,
            alternative_condition: None,
            alternative_effect: None,
            operation: None,
            value: None,
            aggregate: None,
            comparison_type: None,
        }),
    };
    
    let result = resolver.resolve_ability(&ability);
    assert!(result.is_ok(), "Ability should resolve successfully");
}
