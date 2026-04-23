// Integration tests for card movement abilities
// These tests verify that cards move to the correct zones during normal gameplay
// Testing: self-cost, exclude-self, discard-to-hand, and stage-to-discard mechanics

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::game_state::{GameState, Phase, TurnPhase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::game_setup::{self, ActionType};
use rabuka_engine::ability_resolver::ChoiceResult;
use std::path::Path;
use std::sync::Arc;

fn load_all_cards() -> Vec<rabuka_engine::card::Card> {
    let cards_path = Path::new("../cards/cards.json");
    CardLoader::load_cards_from_file(cards_path)
        .expect("Failed to load cards")
}

fn create_card_database(cards: Vec<rabuka_engine::card::Card>) -> Arc<CardDatabase> {
    Arc::new(CardDatabase::load_or_create(cards))
}

fn get_card_id(card: &rabuka_engine::card::Card, card_db: &CardDatabase) -> i16 {
    card_db.card_no_to_id.get(&card.card_no)
        .copied()
        .expect("Card ID not found")
}

fn setup_player_with_energy(player: &mut Player, card_ids: Vec<i16>) {
    let count = card_ids.len();
    player.energy_zone.cards = card_ids.into_iter().collect();
    // Set half of energy to active, half to wait (so ability can activate)
    player.energy_zone.active_energy_count = count / 2;
}

#[test]
fn test_self_cost_discard_activating_card() {
    println!("\n=== Test: Self-Cost Discard Activating Card ===");
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find a card with self-cost ability: "このメンバーをステージから控え室に置く"
    // Example: PL!-sd1-005-RM 星空 凛 (ab#0) - has ability to discard itself to add live card to hand
    let activation_card = cards.iter()
        .find(|c| c.card_no == "PL!-sd1-005-RM")
        .expect("星空 凛 not found");
    let activation_id = get_card_id(activation_card, &card_database);
    
    // Find live cards for discard pile
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .take(2)
        .collect();
    let live_card_ids: Vec<_> = live_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Place activation card on Player 1's stage
    player1.stage.stage[1] = activation_id;
    
    // Put live cards in Player 1's discard pile
    player1.waitroom.cards = live_card_ids.clone().into();
    
    // Add energy to Player 1
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10)
        .map(|c| get_card_id(c, &card_database)).collect();
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, create_card_database(cards.clone()));
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 2;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game_state.player1.is_first_attacker = true;
    game_state.player2.is_first_attacker = false;
    
    // Clear locked areas to allow ability activation
    game_state.player1.areas_locked_this_turn.clear();
    
    println!("Initial state:");
    println!("  Player 1 stage center: {} (activation card)", activation_card.name);
    println!("  Player 1 discard: {} live cards", live_card_ids.len());
    println!("  Player 1 hand: {} cards", game_state.player1.hand.len());
    
    // Step 1: Generate actions for Player 1
    let actions = game_setup::generate_possible_actions(&game_state);
    println!("\nStep 1: Available actions: {}", actions.len());
    
    // Find the UseAbility action for the card
    let activation_action = actions.iter()
        .find(|a| a.action_type == ActionType::UseAbility && 
                 a.parameters.as_ref().and_then(|p| p.card_id) == Some(activation_id))
        .expect("Activation action not found");
    
    println!("  Found activation action for {}", activation_card.name);
    
    // Step 2: Execute the activation
    let card_id = activation_action.parameters.as_ref().and_then(|p| p.card_id);
    match TurnEngine::execute_main_phase_action(
        &mut game_state,
        &activation_action.action_type,
        card_id,
        None, // card_indices
        None, // stage_area
        None, // use_baton_touch
    ) {
        Ok(_) => println!("  Activation executed successfully"),
        Err(e) => panic!("Activation failed: {}", e),
    }
    
    println!("\nAfter activation:");
    println!("  Player 1 stage center: {}", if game_state.player1.stage.stage[1] != -1 { "card present" } else { "empty" });
    println!("  Player 1 discard: {} cards", game_state.player1.waitroom.len());
    println!("  Player 1 hand: {} cards", game_state.player1.hand.len());
    
    // Verify the activating card was discarded (self-cost)
    assert_eq!(game_state.player1.stage.stage[1], -1, "Activating card should be removed from stage");
    assert!(game_state.player1.waitroom.cards.contains(&activation_id), "Activating card should be in discard");
    
    // Verify a live card was added to hand (effect)
    let hand_has_live = game_state.player1.hand.cards.iter()
        .any(|&id| card_database.get_card(id).map(|c| c.is_live()).unwrap_or(false));
    assert!(hand_has_live, "Hand should contain a live card");
    
    println!("\n✓ Self-cost test passed: Activating card correctly discarded, live card added to hand");
}

#[test]
fn test_exclude_self_discard_other_card() {
    println!("\n=== Test: Exclude-Self Discard Other Card ===");
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find a card with exclude-self cost: "このメンバー以外の..."
    // Example: PL!SP-pb1-011-R 鬼塚冬毬 - can discard other Liella members
    let activation_card = cards.iter()
        .find(|c| c.card_no == "PL!SP-pb1-011-R")
        .expect("鬼塚冬毬 not found");
    let activation_id = get_card_id(activation_card, &card_database);
    
    // Find other Liella members for stage
    let other_members: Vec<_> = cards.iter()
        .filter(|c| c.is_member() && c.group == "Liella!" && c.card_no != "PL!SP-pb1-011-R")
        .take(2)
        .collect();
    let other_member_ids: Vec<_> = other_members.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Place activation card on Player 1's stage
    player1.stage.stage[1] = activation_id;
    
    // Place other members on Player 1's stage
    player1.stage.stage[0] = other_member_ids[0];
    player1.stage.stage[2] = other_member_ids[1];
    
    // Add energy to Player 1
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10)
        .map(|c| get_card_id(c, &card_database)).collect();
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, create_card_database(cards.clone()));
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 2;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game_state.player1.is_first_attacker = true;
    
    // Clear locked areas to allow ability activation
    game_state.player1.areas_locked_this_turn.clear();
    
    println!("Initial state:");
    println!("  Player 1 stage center: {} (activation card)", activation_card.name);
    println!("  Player 1 stage left: {}", card_database.get_card(other_member_ids[0]).unwrap().name);
    println!("  Player 1 stage right: {}", card_database.get_card(other_member_ids[1]).unwrap().name);
    
    // Step 1: Generate actions for Player 1
    let actions = game_setup::generate_possible_actions(&game_state);
    
    // Find the UseAbility action for the card
    let activation_action = actions.iter()
        .find(|a| a.action_type == ActionType::UseAbility && 
                 a.parameters.as_ref().and_then(|p| p.card_id) == Some(activation_id))
        .expect("Activation action not found");
    
    // Step 2: Execute the activation
    let card_id = activation_action.parameters.as_ref().and_then(|p| p.card_id);
    match TurnEngine::execute_main_phase_action(
        &mut game_state,
        &activation_action.action_type,
        card_id,
        None, // card_indices
        None, // stage_area
        None, // use_baton_touch
    ) {
        Ok(_) => println!("  Activation executed"),
        Err(e) => panic!("Activation failed: {}", e),
    }
    
    println!("\nAfter activation:");
    println!("  Player 1 stage center: {} (should still be present)", 
             if game_state.player1.stage.stage[1] == activation_id { "present" } else { "missing" });
    println!("  Player 1 discard: {} cards", game_state.player1.waitroom.len());
    
    // Verify the activating card is NOT discarded (exclude_self)
    assert_eq!(game_state.player1.stage.stage[1], activation_id, "Activating card should remain on stage");
    
    // Verify one of the other members was discarded
    let other_discarded = game_state.player1.waitroom.cards.contains(&other_member_ids[0]) || 
                         game_state.player1.waitroom.cards.contains(&other_member_ids[1]);
    assert!(other_discarded, "One of the other members should be in discard");
    
    println!("\n✓ Exclude-self test passed: Activating card remains, other member discarded");
}

#[test]
fn test_discard_to_hand_with_selection() {
    println!("\n=== Test: Discard to Hand with User Selection ===");
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find a card that moves live card from discard to hand
    // Example: Any card with ability "自分の控え室からライブカードを1枚手札に加える"
    let activation_card = cards.iter()
        .find(|c| c.card_no == "PL!-sd1-005-RM")
        .expect("星空 凛 not found");
    let activation_id = get_card_id(activation_card, &card_database);
    
    // Find multiple live cards for discard pile
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .take(3)
        .collect();
    let live_card_ids: Vec<_> = live_cards.iter()
        .map(|c| get_card_id(c, &card_database))
        .collect();
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Place activation card on Player 1's stage
    player1.stage.stage[1] = activation_id;
    
    // Put multiple live cards in Player 1's discard pile
    player1.waitroom.cards = live_card_ids.clone().into();
    
    // Add energy to Player 1
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10)
        .map(|c| get_card_id(c, &card_database)).collect();
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, create_card_database(cards.clone()));
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 2;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game_state.player1.is_first_attacker = true;
    
    // Clear locked areas to allow ability activation
    game_state.player1.areas_locked_this_turn.clear();
    
    println!("Initial state:");
    println!("  Player 1 stage center: {} (activation card)", activation_card.name);
    println!("  Player 1 discard: {} live cards", live_card_ids.len());
    println!("  Player 1 hand: {} cards", game_state.player1.hand.len());
    
    // Step 1: Generate actions for Player 1
    let actions = game_setup::generate_possible_actions(&game_state);
    
    // Find the UseAbility action for the card
    let activation_action = actions.iter()
        .find(|a| a.action_type == ActionType::UseAbility && 
                 a.parameters.as_ref().and_then(|p| p.card_id) == Some(activation_id))
        .expect("Activation action not found");
    
    // Step 2: Execute the activation (should trigger cost payment first)
    let card_id = activation_action.parameters.as_ref().and_then(|p| p.card_id);
    match TurnEngine::execute_main_phase_action(
        &mut game_state,
        &activation_action.action_type,
        card_id,
        None, // card_indices
        None, // stage_area
        None, // use_baton_touch
    ) {
        Ok(_) => println!("  Cost payment executed"),
        Err(e) => panic!("Cost payment failed: {}", e),
    }
    
    println!("\nAfter cost payment:");
    println!("  Player 1 stage center: {}", if game_state.player1.stage.stage[1] != -1 { "card present" } else { "empty" });
    println!("  Player 1 discard: {} cards", game_state.player1.waitroom.len());
    
    // Verify self-cost was paid (activating card discarded)
    assert_eq!(game_state.player1.stage.stage[1], -1, "Activating card should be removed from stage");
    assert!(game_state.player1.waitroom.cards.contains(&activation_id), "Activating card should be in discard");
    
    // Step 3: Check if there's a pending choice for discard-to-hand selection
    if let Some(ref pending) = game_state.pending_ability {
        println!("  Pending ability detected (waiting for user choice)");
        
        // Simulate user selecting the first live card from discard
        let selected_index = game_state.player1.waitroom.cards.iter()
            .position(|&id| card_database.get_card(id).map(|c| c.is_live()).unwrap_or(false))
            .expect("No live card in discard");
        
        let choice_result = ChoiceResult::CardSelected {
            indices: vec![selected_index],
        };
        
        match game_state.provide_ability_choice_result(choice_result) {
            Ok(_) => println!("  User choice processed"),
            Err(e) => panic!("Choice processing failed: {}", e),
        }
    }
    
    println!("\nAfter effect execution:");
    println!("  Player 1 discard: {} cards (should be 2 less)", game_state.player1.waitroom.len());
    println!("  Player 1 hand: {} cards (should have 1 more)", game_state.player1.hand.len());
    
    // Verify a live card was moved from discard to hand
    let hand_has_live = game_state.player1.hand.cards.iter()
        .any(|&id| card_database.get_card(id).map(|c| c.is_live()).unwrap_or(false));
    assert!(hand_has_live, "Hand should contain a live card");
    
    // Verify discard has one fewer live card
    let discard_live_count = game_state.player1.waitroom.cards.iter()
        .filter(|&&id| card_database.get_card(id).map(|c| c.is_live()).unwrap_or(false))
        .count();
    assert_eq!(discard_live_count, live_card_ids.len() - 1, "Discard should have one fewer live card");
    
    println!("\n✓ Discard-to-hand test passed: User selection triggered, card moved correctly");
}

#[test]
fn test_normal_stage_to_discard_no_selection() {
    println!("\n=== Test: Normal Stage to Discard (No Selection Needed) ===");
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find a card with simple self-cost (no selection needed)
    let activation_card = cards.iter()
        .find(|c| c.card_no == "PL!-sd1-005-RM")
        .expect("星空 凛 not found");
    let activation_id = get_card_id(activation_card, &card_database);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Place activation card on Player 1's stage (only card on stage)
    player1.stage.stage[1] = activation_id;
    player1.stage.stage[0] = -1;
    player1.stage.stage[2] = -1;
    
    // Add energy to Player 1
    let energy_card_ids: Vec<_> = cards.iter().filter(|c| c.is_energy()).take(10)
        .map(|c| get_card_id(c, &card_database)).collect();
    setup_player_with_energy(&mut player1, energy_card_ids);
    
    let mut game_state = GameState::new(player1, player2, create_card_database(cards.clone()));
    game_state.current_phase = Phase::Main;
    game_state.turn_number = 2;
    game_state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game_state.player1.is_first_attacker = true;
    
    // Clear locked areas to allow ability activation
    game_state.player1.areas_locked_this_turn.clear();
    
    println!("Initial state:");
    println!("  Player 1 stage center: {} (only card on stage)", activation_card.name);
    
    // Step 1: Generate actions for Player 1
    let actions = game_setup::generate_possible_actions(&game_state);
    
    // Find the UseAbility action for the card
    let activation_action = actions.iter()
        .find(|a| a.action_type == ActionType::UseAbility && 
                 a.parameters.as_ref().and_then(|p| p.card_id) == Some(activation_id))
        .expect("Activation action not found");
    
    // Step 2: Execute the activation
    let card_id = activation_action.parameters.as_ref().and_then(|p| p.card_id);
    match TurnEngine::execute_main_phase_action(
        &mut game_state,
        &activation_action.action_type,
        card_id,
        None, // card_indices
        None, // stage_area
        None, // use_baton_touch
    ) {
        Ok(_) => println!("  Activation executed"),
        Err(e) => panic!("Activation failed: {}", e),
    }
    
    println!("\nAfter activation:");
    println!("  Player 1 stage center: {}", if game_state.player1.stage.stage[1] != -1 { "card present" } else { "empty" });
    println!("  Player 1 discard: {} cards", game_state.player1.waitroom.len());
    
    // Verify the activating card was discarded without user selection
    assert_eq!(game_state.player1.stage.stage[1], -1, "Activating card should be removed from stage");
    assert!(game_state.player1.waitroom.cards.contains(&activation_id), "Activating card should be in discard");
    
    // Verify no pending choice (auto-executed)
    assert!(game_state.pending_ability.is_none(), "No pending choice should remain");
    
    println!("\n✓ Normal stage-to-discard test passed: Card discarded automatically without selection");
}
