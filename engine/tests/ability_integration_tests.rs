//! Integration tests for ability system improvements
//! These tests use actual cards and simulate real gameplay

use std::sync::Arc;
use std::path::PathBuf;
use rabuka_engine::ability_resolver::AbilityResolver;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;

fn setup_test_game_with_cards() -> GameState {
    // Load actual cards from the card database
    let cards_path = PathBuf::from("../cards/cards.json");
    let cards = CardLoader::load_cards_from_file(&cards_path).expect("Failed to load cards");
    let card_db = Arc::new(CardDatabase::load_or_create(cards));
    
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut game_state = GameState::new(player1, player2, card_db);
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    
    game_state
}

#[test]
fn test_actual_card_ability_execution() {
    // Test ability execution using an actual card from the database
    let mut game_state = setup_test_game_with_cards();
    
    // Find a card with abilities (星空 凛 - PL!-sd1-005-SD has an activation ability)
    let card_no = "PL!-sd1-005-SD";
    let card_id = game_state.card_database.get_card_id(card_no).expect("Card should exist in database");
    
    // Add the card to player's hand
    game_state.player1.add_card_to_hand(card_id);
    
    // Add a live card to waitroom (discard zone) for the ability to target
    let live_card_no = "PL!-sd1-019-SD"; // START:DASH!! is a live card
    if let Some(live_card_id) = game_state.card_database.get_card_id(live_card_no) {
        game_state.player1.waitroom.cards.push(live_card_id);
    }
    
    // Put the card on stage first (activation ability requires it to be on stage as cost)
    let hand_index = game_state.player1.get_card_index_by_id(card_id).expect("Card should be in hand");
    let stage_area = rabuka_engine::zones::MemberArea::Center;
    let _ = game_state.player1.move_card_from_hand_to_stage(
        hand_index,
        stage_area,
        false,
        &game_state.card_database
    );
    
    // Get the card and its abilities, clone the effect to avoid borrow conflict
    let card = game_state.card_database.get_card(card_id).expect("Card should exist");
    assert!(!card.abilities.is_empty(), "Card should have abilities");
    
    let effect = card.abilities[0].effect.clone().expect("Ability should have an effect");
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    // Execute the first ability's effect
    let result = resolver.execute_effect(&effect);
    assert!(result.is_ok(), "Ability effect execution should succeed: {:?}", result.err());
}

#[test]
fn test_play_card_and_activate_ability() {
    // Test playing a card and activating its ability naturally
    let mut game_state = setup_test_game_with_cards();
    
    // Find a member card with abilities
    let card_no = "PL!-sd1-001-SD"; // 高坂 穂乃果 with debut ability
    let card_id = game_state.card_database.get_card_id(card_no).expect("Card should exist in database");
    
    // Add the card to player's hand
    game_state.player1.add_card_to_hand(card_id);
    
    // Add energy cards to pay for the card's cost (11 energy)
    let energy_card_id = game_state.card_database.get_card_id("LL-E-001-SD").unwrap();
    for _ in 0..11 {
        game_state.player1.energy_zone.cards.push(energy_card_id);
    }
    game_state.player1.energy_zone.active_energy_count = 11;
    
    // Add some live cards to success zone to trigger debut ability
    let live_card_no = "PL!-sd1-004-SD";
    if let Some(live_card_id) = game_state.card_database.get_card_id(live_card_no) {
        game_state.player1.success_live_card_zone.add_card(live_card_id);
        game_state.player1.success_live_card_zone.add_card(live_card_id);
    }
    
    // Play the card to stage (simulating natural gameplay)
    let hand_index = game_state.player1.get_card_index_by_id(card_id).expect("Card should be in hand");
    let stage_area = rabuka_engine::zones::MemberArea::Center;
    
    let result = game_state.player1.move_card_from_hand_to_stage(
        hand_index,
        stage_area,
        false,
        &game_state.card_database
    );
    
    assert!(result.is_ok(), "Playing card to stage should succeed: {:?}", result.err());
    
    // Verify the card is now on stage (check all stage positions)
    let card_on_stage = game_state.player1.stage.stage.iter().any(|&id| id == card_id);
    assert!(card_on_stage, "Card should be on stage");
}

#[test]
fn test_natural_gameplay_with_choices() {
    // Test natural gameplay where player makes choices
    let mut game_state = setup_test_game_with_cards();
    
    // Add multiple cards to hand to simulate a real game
    let card_ids = vec![
        game_state.card_database.get_card_id("PL!-sd1-001-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-002-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-003-SD").unwrap(),
    ];
    
    for card_id in card_ids {
        game_state.player1.add_card_to_hand(card_id);
    }
    
    // Add energy cards to pay for the card's cost (11 energy)
    let energy_card_id = game_state.card_database.get_card_id("LL-E-001-SD").unwrap();
    for _ in 0..11 {
        game_state.player1.energy_zone.cards.push(energy_card_id);
    }
    game_state.player1.energy_zone.active_energy_count = 11;
    
    // Simulate playing a card to stage
    let hand_index = 0;
    let stage_area = rabuka_engine::zones::MemberArea::Center;
    
    let result = game_state.player1.move_card_from_hand_to_stage(
        hand_index,
        stage_area,
        false,
        &game_state.card_database
    );
    
    assert!(result.is_ok(), "Playing card should succeed");
    assert_eq!(game_state.player1.hand.cards.len(), 2, "Should have 2 cards remaining in hand");
}

#[test]
fn test_energy_management_in_gameplay() {
    // Test energy management during natural gameplay
    let mut game_state = setup_test_game_with_cards();
    
    // Add energy cards to energy zone (need 11 for the card's cost)
    let energy_card_id = game_state.card_database.get_card_id("LL-E-001-SD").unwrap();
    for _ in 0..11 {
        game_state.player1.energy_zone.cards.push(energy_card_id);
    }
    game_state.player1.energy_zone.active_energy_count = 11;
    
    // Add a member card to hand
    let member_card_id = game_state.card_database.get_card_id("PL!-sd1-001-SD").unwrap();
    game_state.player1.add_card_to_hand(member_card_id);
    
    // Verify energy state
    assert_eq!(game_state.player1.energy_zone.cards.len(), 11, "Should have 11 energy cards");
    assert_eq!(game_state.player1.energy_zone.active_energy_count, 11, "Should have 11 active energy");
    
    // Play the member card (which costs energy)
    let hand_index = game_state.player1.get_card_index_by_id(member_card_id).unwrap();
    let stage_area = rabuka_engine::zones::MemberArea::Center;
    
    let result = game_state.player1.move_card_from_hand_to_stage(
        hand_index,
        stage_area,
        false,
        &game_state.card_database
    );
    
    // This should succeed if we have enough energy
    assert!(result.is_ok(), "Playing card with energy should succeed");
}

#[test]
fn test_ability_with_duration() {
    // Test abilities with duration effects using actual cards
    let mut game_state = setup_test_game_with_cards();
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    // Add some duration effects (simulating what would come from actual card abilities)
    resolver.duration_effects.push(("gain_resource:blade:2".to_string(), "live_end".to_string()));
    resolver.duration_effects.push(("gain_resource:heart:1".to_string(), "live_end".to_string()));
    resolver.duration_effects.push(("permanent_effect".to_string(), "permanent".to_string()));
    
    // Expire live_end effects
    resolver.expire_live_end_effects();
    
    // Verify that live_end effects were removed
    assert_eq!(resolver.duration_effects.len(), 1, "Should have 1 effect remaining (permanent)");
    assert_eq!(resolver.duration_effects[0].0, "permanent_effect", "Remaining effect should be permanent");
}

#[test]
fn test_single_target_selection() {
    // Test that single target selection doesn't prompt user
    let mut game_state = setup_test_game_with_cards();
    
    // Add only 1 energy card
    let energy_card_id = game_state.card_database.get_card_id("LL-E-001-SD").unwrap();
    game_state.player1.energy_zone.cards.push(energy_card_id);
    game_state.player1.energy_zone.active_energy_count = 1;
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    // Create a change_state effect with count = total valid targets
    let effect = rabuka_engine::card::AbilityEffect {
        text: "Set 1 energy card to wait".to_string(),
        action: "change_state".to_string(),
        state_change: Some("wait".to_string()),
        count: Some(1),
        target: Some("self".to_string()),
        target_location: Some("energy_zone".to_string()),
        ..Default::default()
    };
    
    // Execute the effect
    let result = resolver.execute_effect(&effect);
    
    assert!(result.is_ok(), "Effect execution should succeed");
    
    // Verify that NO choice is pending (single target, no selection needed)
    let pending_choice = resolver.get_pending_choice();
    assert!(pending_choice.is_none(), "Should NOT prompt user when single valid target exists");
    
    // Verify that the card was deactivated
    assert_eq!(game_state.player1.energy_zone.active_energy_count, 0, "Should have 0 active energy cards");
}

#[test]
fn test_look_and_select_without_placement() {
    // Test that look_and_select without placement_order does NOT prompt user
    let mut game_state = setup_test_game_with_cards();
    
    // Add actual cards to deck
    let card_ids = vec![
        game_state.card_database.get_card_id("PL!-sd1-001-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-002-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-003-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-004-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-005-SD").unwrap(),
    ];
    
    for card_id in card_ids {
        game_state.player1.main_deck.cards.push(card_id);
    }
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    let look_effect = rabuka_engine::card::AbilityEffect {
        text: "Look at top 5 cards".to_string(),
        action: "look_at".to_string(),
        count: Some(5),
        source: Some("deck_top".to_string()),
        ..Default::default()
    };
    
    let select_effect = rabuka_engine::card::AbilityEffect {
        text: "Select cards".to_string(),
        action: "move_cards".to_string(),
        count: Some(2),
        source: Some("deck_top".to_string()),
        destination: Some("hand".to_string()),
        // NO placement_order specified
        ..Default::default()
    };
    
    let look_and_select_effect = rabuka_engine::card::AbilityEffect {
        text: "Look and select".to_string(),
        action: "look_and_select".to_string(),
        look_action: Some(Box::new(look_effect)),
        select_action: Some(Box::new(select_effect)),
        ..Default::default()
    };
    
    // Execute the effect
    let result = resolver.execute_effect(&look_and_select_effect);
    
    assert!(result.is_ok(), "Effect execution should succeed");
    
    // Verify that NO choice is pending (no placement_order)
    let pending_choice = resolver.get_pending_choice();
    assert!(pending_choice.is_none(), "Should NOT prompt user when placement_order is not specified");
}

#[test]
fn test_look_and_select_with_placement_order() {
    // Test that look_and_select with placement_order prompts user using real cards
    let mut game_state = setup_test_game_with_cards();
    
    // Add actual cards to deck
    let card_ids = vec![
        game_state.card_database.get_card_id("PL!-sd1-001-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-002-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-003-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-004-SD").unwrap(),
        game_state.card_database.get_card_id("PL!-sd1-005-SD").unwrap(),
    ];
    
    for card_id in card_ids {
        game_state.player1.main_deck.cards.push(card_id);
    }
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    let look_effect = rabuka_engine::card::AbilityEffect {
        text: "Look at top 5 cards".to_string(),
        action: "look_at".to_string(),
        count: Some(5),
        source: Some("deck_top".to_string()),
        ..Default::default()
    };
    
    let select_effect = rabuka_engine::card::AbilityEffect {
        text: "Select cards".to_string(),
        action: "move_cards".to_string(),
        count: Some(2),
        source: Some("deck_top".to_string()),
        destination: Some("hand".to_string()),
        placement_order: Some("any_order".to_string()), // This should trigger user choice
        ..Default::default()
    };
    
    let look_and_select_effect = rabuka_engine::card::AbilityEffect {
        text: "Look and select".to_string(),
        action: "look_and_select".to_string(),
        look_action: Some(Box::new(look_effect)),
        select_action: Some(Box::new(select_effect)),
        ..Default::default()
    };
    
    // Execute the effect - engine should set up pending choice
    let result = resolver.execute_effect(&look_and_select_effect);
    assert!(result.is_ok(), "Effect execution should succeed");
    
    // Verify that a choice IS pending (set by engine due to placement_order)
    let pending_choice = resolver.get_pending_choice();
    assert!(pending_choice.is_some(), "Engine should prompt user when placement_order is specified");
    
    // Verify that cards were stored in looked_at_cards by engine
    assert_eq!(resolver.looked_at_cards.len(), 5, "Engine should have stored looked-at cards");
}

#[test]
fn test_change_state_with_multiple_targets() {
    // Test that change_state with multiple valid targets prompts user using real cards
    let mut game_state = setup_test_game_with_cards();
    
    // Add multiple energy cards (use different card IDs to ensure valid targets are counted correctly)
    let energy_card_ids = vec![
        "LL-E-001-SD",
        "LL-E-002-SD", 
        "LL-E-003-SD",
        "LL-E-004-SD",
        "LL-E-005-SD"
    ];
    for card_no in energy_card_ids {
        if let Some(card_id) = game_state.card_database.get_card_id(card_no) {
            game_state.player1.energy_zone.cards.push(card_id);
        }
    }
    game_state.player1.energy_zone.active_energy_count = 5;
    
    let mut resolver = AbilityResolver::new(&mut game_state);
    
    // Create a change_state effect with count < total valid targets
    let effect = rabuka_engine::card::AbilityEffect {
        text: "Set 2 energy cards to wait".to_string(),
        action: "change_state".to_string(),
        state_change: Some("wait".to_string()),
        count: Some(2),
        target: Some("self".to_string()),
        source: Some("energy_zone".to_string()),
        card_type: Some("energy_card".to_string()),
        ..Default::default()
    };
    
    // Execute the effect - engine should set up pending choice when count < valid targets
    let result = resolver.execute_effect(&effect);
    assert!(result.is_ok(), "Effect execution should succeed");
    
    // Verify that a choice IS pending (set by engine due to multiple valid targets)
    let pending_choice = resolver.get_pending_choice();
    assert!(pending_choice.is_some(), "Engine should prompt user when multiple valid targets exist");
    
    // Verify that valid targets were stored in looked_at_cards by engine
    assert_eq!(resolver.looked_at_cards.len(), 5, "Engine should have stored valid targets");
}
