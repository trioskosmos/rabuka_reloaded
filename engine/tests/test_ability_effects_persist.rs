// Test that ability effects actually persist to the game state
// This tests the fix for the cloning issue where changes were lost

use rabuka_engine::card::{Ability, AbilityEffect, Card, CardType};
use rabuka_engine::ability_resolver::AbilityResolver;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;

#[test]
fn test_gain_resource_persists() {
    // Create players with cards on stage
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    // Add a member card to stage
    let stage_card = Card {
        card_no: "test-001".to_string(),
        img: None,
        name: "Test Member".to_string(),
        product: "Test".to_string(),
        card_type: CardType::Member,
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

    use rabuka_engine::zones::{CardInZone, Orientation, FaceState};
    player1.stage.left_side = Some(CardInZone {
        card: stage_card,
        orientation: Some(Orientation::Active),
        face_state: FaceState::FaceUp,
        energy_underneath: Vec::new(),
        played_via_ability: false,
        turn_played: 0,
    });

    let initial_blade_count = player1.stage.left_side.as_ref().unwrap().card.blade;

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
    assert!(result.is_ok(), "Gain resource effect should execute successfully");

    // Verify the blade count increased (changes are direct since resolver uses mutable reference)
    let new_blade_count = game_state.player1.stage.left_side.as_ref().unwrap().card.blade;
    assert_eq!(new_blade_count, initial_blade_count + 2, "Blade count should have increased by 2");

    // Verify the change persisted
    let final_blade_count = game_state.player1.stage.left_side.as_ref().unwrap().card.blade;
    assert_eq!(final_blade_count, initial_blade_count + 2, "Blade count should persist");
}

#[test]
fn test_modify_score_persists() {
    // Create players with cards on stage
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    // Add a member card to stage with initial score
    let stage_card = Card {
        card_no: "test-002".to_string(),
        img: None,
        name: "Test Member".to_string(),
        product: "Test".to_string(),
        card_type: CardType::Member,
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
        score: Some(100),
        need_heart: None,
        special_heart: None,
        abilities: Vec::new(),
    };

    use rabuka_engine::zones::{CardInZone, Orientation, FaceState};
    player1.stage.center = Some(CardInZone {
        card: stage_card,
        orientation: Some(Orientation::Active),
        face_state: FaceState::FaceUp,
        energy_underneath: Vec::new(),
        played_via_ability: false,
        turn_played: 0,
    });

    let initial_score = player1.stage.center.as_ref().unwrap().card.score.unwrap_or(0);

    let mut game_state = GameState::new(player1, player2);
    let mut resolver = AbilityResolver::new(&mut game_state);

    let effect = AbilityEffect {
        text: "Add 50 to score".to_string(),
        action: "modify_score".to_string(),
        operation: Some("add".to_string()),
        value: Some(50),
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
        count: None,
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
    assert!(result.is_ok(), "Modify score effect should execute successfully");

    // Verify the score increased (changes are direct since resolver uses mutable reference)
    let new_score = game_state.player1.stage.center.as_ref().unwrap().card.score.unwrap_or(0);
    assert_eq!(new_score, initial_score + 50, "Score should have increased by 50");
}
