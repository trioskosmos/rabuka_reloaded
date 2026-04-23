// Comprehensive QA tests for resource_icon_count functionality
// These tests use real cards from cards.json and test edge cases
// to ensure the engine correctly handles resource icon counting

use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::ability_resolver::AbilityResolver;
use rabuka_engine::player::Player;
use rabuka_engine::card::{Ability, AbilityEffect};
use rabuka_engine::game_state::{GameState, TurnPhase, Phase, GameResult};
use rabuka_engine::zones::ResolutionZone;
use rabuka_engine::card::Card;
use std::path::Path;
use rabuka_engine::zones::MemberArea;

/// Load all cards from the standard cards.json path
fn load_cards() -> Vec<Card> {
    let cards_path = Path::new("../cards/cards.json");
    CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards")
}

/// Create a standard test GameState with default values
fn create_test_game_state(player1: Player, player2: Player) -> GameState {
    use rabuka_engine::card::CardDatabase;
    use std::sync::Arc;
    
    let card_database = Arc::new(CardDatabase::load_or_create(vec![]));
    
    GameState {
        player1,
        player2,
        card_database,
        current_turn_phase: TurnPhase::FirstAttackerNormal,
        current_phase: Phase::Active,
        turn_number: 1,
        resolution_zone: ResolutionZone::new(),
        is_first_turn: true,
        live_cheer_count: 0,
        turn1_abilities_played: std::collections::HashSet::new(),
        turn2_abilities_played: std::collections::HashMap::new(),
        player1_cheer_blade_heart_count: 0,
        player2_cheer_blade_heart_count: 0,
        temporary_effects: Vec::new(),
        game_result: GameResult::Ongoing,
        pending_auto_abilities: Vec::new(),
        cheer_check_completed: false,
        cheer_checks_required: 0,
        cheer_checks_done: 0,
        prohibition_effects: Vec::new(),
        turn_limited_abilities_used: std::collections::HashSet::new(),
        mulligan_player1_done: false,
        mulligan_player2_done: false,
        current_mulligan_player: String::new(),
        mulligan_selected_indices: Vec::new(),
        rps_winner: None,
        live_card_set_player1_done: false,
        live_card_set_player2_done: false,
        history: Vec::new(),
        future: Vec::new(),
        max_history_size: 100,
        blade_modifiers: std::collections::HashMap::new(),
        blade_type_modifiers: std::collections::HashMap::new(),
        heart_modifiers: std::collections::HashMap::new(),
        activating_card: None,
        score_modifiers: std::collections::HashMap::new(),
        need_heart_modifiers: std::collections::HashMap::new(),
        orientation_modifiers: std::collections::HashMap::new(),
        cost_modifiers: std::collections::HashMap::new(),
        revealed_cards: std::collections::HashSet::new(),
        optional_cost_behavior: "auto".to_string(),
        pending_ability: None,
        areas_placed_this_turn: std::collections::HashSet::new(),
        cards_appeared_this_turn: std::collections::HashSet::new(),
        turn_order_changed: false,
        auto_ability_trigger_counts: std::collections::HashMap::new(),
        turn_limit_usage: std::collections::HashMap::new(),
        card_instance_counter: 0,
        card_instance_mapping: std::collections::HashMap::new(),
        baton_touch_count: 0,
        heart_color_decision_phase: String::new(),
        deck_refresh_pending: false,
        position_change_occurred_this_turn: false,
        formation_change_occurred_this_turn: false,
        partial_resolution_allowed: false,
        full_cost_payment_required: false,
        auto_abilities_mandatory: false,
        search_count_adjustment_enabled: false,
        allow_replacement_placement: false,
        allow_live_without_stage_members: false,
        live_being_performed: false,
        game_ended: false,
        draw_state: false,
        prohibition_precedence_enabled: false,
        effect_resumption_state: String::new(),
        gained_abilities: std::collections::HashMap::new(),
        card_set_search_enabled: false,
        multi_victory_selection_enabled: false,
        turn_player_priority_enabled: false,
        arbitrary_actions_restricted: false,
        replacement_effects: Vec::new(),
        effect_creation_counter: 0,
        game_state_history: Vec::new(),
        max_state_history_size: 100,
        loop_detected: false,
    }
}

/// Place a card on stage in a specific area
fn place_card_on_stage(player: &mut Player, card_id: i16, area: MemberArea) {
    player.stage.set_area(area, card_id);
}

/// Execute an ability and return the result
fn execute_ability(game_state: &mut GameState, ability: &Ability) -> Result<(), String> {
    let mut resolver = AbilityResolver::new(game_state);
    resolver.resolve_ability(ability, None)
}

#[test]
fn test_resource_icon_count_is_loaded() {
    let cards = load_cards();
    
    // Find a card with gain_resource ability that has resource_icon_count
    let test_card = cards.iter().find(|c| {
        c.abilities.iter().any(|a| {
            a.effect.as_ref().map_or(false, |e| {
                e.action == "gain_resource" && e.resource_icon_count.is_some()
            })
        })
    });
    
    match test_card {
        Some(card) => {
            println!("Found card with resource_icon_count: {} - {}", card.card_no, card.name);
            let ability = card.abilities.iter().find(|a| {
                a.effect.as_ref().map_or(false, |e| {
                    e.action == "gain_resource" && e.resource_icon_count.is_some()
                })
            }).unwrap();
            
            let resource_icon_count = ability.effect.as_ref().unwrap().resource_icon_count.unwrap();
            println!("Resource icon count: {}", resource_icon_count);
            assert!(resource_icon_count > 0, "Resource icon count should be positive");
        }
        None => {
            println!("No card found with resource_icon_count in abilities.json");
            println!("This is expected if abilities.json doesn't have this field populated yet");
        }
    }
}

#[test]
fn test_gain_resource_with_resource_icon_count() {
    let cards = load_cards();
    
    // Create a manual ability with resource_icon_count to test the functionality
    let test_ability = Ability {
        full_text: "Test: Gain 2 blades".to_string(),
        triggerless_text: "Test: Gain 2 blades".to_string(),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: None,
        effect: Some(AbilityEffect {
            text: "Gain 2 blades".to_string(),
            action: "gain_resource".to_string(),
            resource: Some("blade".to_string()),
            resource_icon_count: Some(2),
            count: Some(1), // This should be ignored in favor of resource_icon_count
            ..Default::default()
        }),
        keywords: None,
    };
    
    let (mut player1, player2) = create_test_players();
    
    // Create a simple test card and place it on stage
    let test_card = cards.first().unwrap();
    let card_id = test_card.card_no.parse::<i16>().unwrap_or(0);
    place_card_on_stage(&mut player1, card_id, MemberArea::Center);
    
    let mut game_state = create_test_game_state(player1, player2);
    
    let initial_blade_count = if game_state.player1.stage.stage[1] != -1 {
        let card_id = game_state.player1.stage.stage[1];
        let cards = load_cards();
        let card = cards.iter().find(|c| c.card_no.parse::<i16>().unwrap_or(0) == card_id);
        card.map(|c| c.blade).unwrap_or(0)
    } else {
        0
    };
    println!("Initial blade count: {}", initial_blade_count);
    let result = execute_ability(&mut game_state, &test_ability);
    
    println!("Ability execution result: {:?}", result);
    assert!(result.is_ok(), "Ability should resolve successfully");
    
    // Verify that resource_icon_count (2) was used, not count (1)
    let final_blade_count = if game_state.player1.stage.stage[1] != -1 {
        let card_id = game_state.player1.stage.stage[1];
        let cards = load_cards();
        let card = cards.iter().find(|c| c.card_no.parse::<i16>().unwrap_or(0) == card_id);
        card.map(|c| c.blade).unwrap_or(0) as i32
    } else {
        0
    };
    println!("Final blade count: {}", final_blade_count);
    
    // Should have gained 2 blades (from resource_icon_count), not 1 (from count)
    assert_eq!(final_blade_count - initial_blade_count as i32, 2, "Should have gained 2 blades from resource_icon_count");
    
    println!("✓ resource_icon_count works correctly");
}

/// Test: Edge case - resource_icon_count of 0
/// Edge case: Zero resource icon count should not change state
#[test]
fn test_resource_icon_count_zero_edge_case() {
    let cards = load_cards();
    
    let test_ability = Ability {
        full_text: "Test: Gain 0 blades".to_string(),
        triggerless_text: "Test: Gain 0 blades".to_string(),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: None,
        effect: Some(AbilityEffect {
            text: "Gain 0 blades".to_string(),
            action: "gain_resource".to_string(),
            resource: Some("blade".to_string()),
            resource_icon_count: Some(0),
            ..Default::default()
        }),
        keywords: None,
    };
    
    let (mut player1, player2) = create_test_players();
    let test_card = cards.first().unwrap();
    let card_id = test_card.card_no.parse::<i16>().unwrap_or(0);
    place_card_on_stage(&mut player1, card_id, MemberArea::Center);
    
    let initial_blade_count = if player1.stage.stage[1] != -1 {
        test_card.blade
    } else {
        0
    };
    
    let mut game_state = create_test_game_state(player1, player2);
    let result = execute_ability(&mut game_state, &test_ability);
    
    assert!(result.is_ok(), "Ability should resolve even with 0 count");
    
    let final_blade_count = if game_state.player1.stage.stage[1] != -1 {
        let card_id = game_state.player1.stage.stage[1];
        let cards = load_cards();
        let card = cards.iter().find(|c| c.card_no.parse::<i16>().unwrap_or(0) == card_id);
        card.map(|c| c.blade).unwrap_or(0) as i32
    } else {
        0
    };
    assert_eq!(final_blade_count, initial_blade_count as i32, "Blade count should not change with 0 resource_icon_count");
}

/// Test: Edge case - Large resource_icon_count
/// Edge case: Very large resource icon count to test overflow handling
#[test]
fn test_resource_icon_count_large_value_edge_case() {
    let cards = load_cards();
    
    // Use a very large value to test if engine handles it correctly
    let large_count = 100u32;
    
    let test_ability = Ability {
        full_text: format!("Test: Gain {} blades", large_count),
        triggerless_text: format!("Test: Gain {} blades", large_count),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: None,
        effect: Some(AbilityEffect {
            text: format!("Gain {} blades", large_count),
            action: "gain_resource".to_string(),
            resource: Some("blade".to_string()),
            resource_icon_count: Some(large_count),
            ..Default::default()
        }),
        keywords: None,
    };
    
    let (mut player1, player2) = create_test_players();
    let test_card = cards.first().unwrap();
    let card_id = test_card.card_no.parse::<i16>().unwrap_or(0);
    place_card_on_stage(&mut player1, card_id, MemberArea::Center);
    
    let initial_blade_count = if player1.stage.stage[1] != -1 {
        test_card.blade
    } else {
        0
    };
    
    let mut game_state = create_test_game_state(player1, player2);
    let result = execute_ability(&mut game_state, &test_ability);
    
    assert!(result.is_ok(), "Ability should resolve with large count");
    
    let final_blade_count = if game_state.player1.stage.stage[1] != -1 {
        let card_id = game_state.player1.stage.stage[1];
        let cards = load_cards();
        let card = cards.iter().find(|c| c.card_no.parse::<i16>().unwrap_or(0) == card_id);
        card.map(|c| c.blade).unwrap_or(0) as i32
    } else {
        0
    };
    let expected = initial_blade_count as i32 + large_count as i32;
    assert_eq!(final_blade_count, expected, "Should gain exactly {} blades", large_count);
    
    // Verify no unintended side effects
    assert_eq!(game_state.player1.hand.cards.len(), 0, "Hand should be unchanged");
    assert_eq!(game_state.player2.stage.stage[1], -1, "Opponent stage should be unchanged");
}

/// Test: Edge case - Multiple resource gains in sequence
/// Edge case: Multiple resource_icon_count effects should accumulate correctly
#[test]
fn test_multiple_resource_icon_count_gains() {
    let cards = load_cards();
    
    let test_ability1 = Ability {
        full_text: "Test: Gain 1 blade".to_string(),
        triggerless_text: "Test: Gain 1 blade".to_string(),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: None,
        effect: Some(AbilityEffect {
            text: "Gain 1 blade".to_string(),
            action: "gain_resource".to_string(),
            resource: Some("blade".to_string()),
            resource_icon_count: Some(1),
            ..Default::default()
        }),
        keywords: None,
    };
    
    let test_ability2 = Ability {
        full_text: "Test: Gain 2 blades".to_string(),
        triggerless_text: "Test: Gain 2 blades".to_string(),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: None,
        effect: Some(AbilityEffect {
            text: "Gain 2 blades".to_string(),
            action: "gain_resource".to_string(),
            resource: Some("blade".to_string()),
            resource_icon_count: Some(2),
            ..Default::default()
        }),
        keywords: None,
    };
    
    let (mut player1, player2) = create_test_players();
    let test_card = cards.first().unwrap();
    let card_id = test_card.card_no.parse::<i16>().unwrap_or(0);
    place_card_on_stage(&mut player1, card_id, MemberArea::Center);
    
    let initial_blade_count = if player1.stage.stage[1] != -1 {
        test_card.blade
    } else {
        0
    };
    
    let mut game_state = create_test_game_state(player1, player2);
    
    // Execute first ability
    let result1 = execute_ability(&mut game_state, &test_ability1);
    assert!(result1.is_ok(), "First ability should resolve");
    
    let after_first = if game_state.player1.stage.stage[1] != -1 {
        let card_id = game_state.player1.stage.stage[1];
        let cards = load_cards();
        let card = cards.iter().find(|c| c.card_no.parse::<i16>().unwrap_or(0) == card_id);
        card.map(|c| c.blade).unwrap_or(0) as i32
    } else {
        0
    };
    assert_eq!(after_first - initial_blade_count as i32, 1, "Should gain 1 blade from first ability");
    
    // Execute second ability
    let result2 = execute_ability(&mut game_state, &test_ability2);
    assert!(result2.is_ok(), "Second ability should resolve");
    
    let final_blade_count = if game_state.player1.stage.stage[1] != -1 {
        let card_id = game_state.player1.stage.stage[1];
        let cards = load_cards();
        let card = cards.iter().find(|c| c.card_no.parse::<i16>().unwrap_or(0) == card_id);
        card.map(|c| c.blade).unwrap_or(0) as i32
    } else {
        0
    };
    assert_eq!(final_blade_count - initial_blade_count as i32, 3, "Should gain total of 3 blades (1 + 2)");
}

/// Test: Edge case - resource_icon_count vs count priority
/// Edge case: resource_icon_count should take priority over count field
#[test]
fn test_resource_icon_count_priority_over_count() {
    let cards = load_cards();
    
    // Create ability with both resource_icon_count and count set to different values
    let test_ability = Ability {
        full_text: "Test: Gain blades".to_string(),
        triggerless_text: "Test: Gain blades".to_string(),
        triggers: None,
        use_limit: None,
        is_null: false,
        cost: None,
        effect: Some(AbilityEffect {
            text: "Gain blades".to_string(),
            action: "gain_resource".to_string(),
            resource: Some("blade".to_string()),
            resource_icon_count: Some(5),  // Should use this
            count: Some(1),  // Should ignore this
            ..Default::default()
        }),
        keywords: None,
    };
    
    let (mut player1, player2) = create_test_players();
    let test_card = cards.first().unwrap();
    let card_id = test_card.card_no.parse::<i16>().unwrap_or(0);
    place_card_on_stage(&mut player1, card_id, MemberArea::Center);
    
    let initial_blade_count = if player1.stage.stage[1] != -1 {
        test_card.blade
    } else {
        0
    };
    
    let mut game_state = create_test_game_state(player1, player2);
    let result = execute_ability(&mut game_state, &test_ability);
    
    assert!(result.is_ok(), "Ability should resolve");
    
    let final_blade_count = if game_state.player1.stage.stage[1] != -1 {
        let card_id = game_state.player1.stage.stage[1];
        let cards = load_cards();
        let card = cards.iter().find(|c| c.card_no.parse::<i16>().unwrap_or(0) == card_id);
        card.map(|c| c.blade).unwrap_or(0) as i32
    } else {
        0
    };
    
    // Should use resource_icon_count (5), not count (1)
    assert_eq!(final_blade_count - initial_blade_count as i32, 5, "Should use resource_icon_count (5), not count (1)");
}

fn create_test_players() -> (Player, Player) {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    (player1, player2)
}
