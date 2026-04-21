// Comprehensive QA tests for basic game situations
// These tests use real cards from cards.json and track comprehensive state changes
// to ensure the engine correctly implements game mechanics

use rabuka_engine::card::{Ability, AbilityCost, AbilityEffect, Card, CardType, HeartColor};
use rabuka_engine::game_state::{GameState, Phase, TurnPhase, GameResult};
use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use std::path::Path;
use std::sync::Arc;

/// Helper function to load all cards from cards.json
fn load_all_cards() -> Vec<Card> {
    let cards_path = Path::new("../cards/cards.json");
    CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards")
}

/// Helper function to create a CardDatabase from cards
fn create_card_database(cards: Vec<Card>) -> Arc<CardDatabase> {
    Arc::new(CardDatabase::load_or_create(cards))
}

/// Helper function to place a card on stage
fn place_card_on_stage(player: &mut Player, card: Card, area: MemberArea) {
    let card_id = card.card_no.parse::<i16>().unwrap_or(0);
    match area {
        MemberArea::Center => player.stage.stage[1] = card_id,
        MemberArea::LeftSide => player.stage.stage[0] = card_id,
        MemberArea::RightSide => player.stage.stage[2] = card_id,
    }
}

#[test]
fn test_game_state_initialization_with_real_cards() {
    let cards = load_all_cards();
    
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let card_database = create_card_database(cards.clone());
    let game_state = GameState::new(player1, player2, card_database);
    
    // Verify initial state
    assert_eq!(game_state.turn_number, 1, "Turn should start at 1");
    assert_eq!(game_state.current_turn_phase, TurnPhase::FirstAttackerNormal, "Should start in first attacker normal phase");
    assert_eq!(game_state.current_phase, Phase::Active, "Should start in Active phase");
    assert!(game_state.is_first_turn, "Should be first turn");
    assert_eq!(game_state.game_result, GameResult::Ongoing, "Game should be ongoing");
    
    // Verify players have no cards initially (zones are empty)
    assert_eq!(game_state.player1.hand.cards.len(), 0, "Player1 hand should be empty");
    assert_eq!(game_state.player1.main_deck.cards.len(), 0, "Player1 deck should be empty");
    assert_eq!(game_state.player2.hand.cards.len(), 0, "Player2 hand should be empty");
}

#[test]
fn test_player_creation_with_real_cards() {
    let cards = load_all_cards();
    
    let player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    assert_eq!(player.id, "test_player", "Player ID should match");
    assert_eq!(player.name, "Test Player", "Player name should match");
    assert!(player.is_first_attacker, "Player should be first attacker");
    assert!(player.hand.cards.is_empty(), "Hand should be empty initially");
    assert!(player.main_deck.cards.is_empty(), "Main deck should be empty initially");
    assert!(player.stage.stage[0] == -1, "Left side should be empty");
    assert!(player.stage.stage[1] == -1, "Center should be empty");
    assert!(player.stage.stage[2] == -1, "Right side should be empty");
}

#[test]
fn test_advance_phase_normal_turn_with_real_cards() {
    let cards = load_all_cards();
    
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let card_database = create_card_database(cards.clone());
    let mut game_state = GameState::new(player1, player2, card_database);
    
    // Start in Active phase
    assert_eq!(game_state.current_phase, Phase::Active, "Should start in Active phase");
    
    // Advance to Energy phase
    TurnEngine::advance_phase(&mut game_state);
    assert_eq!(game_state.current_phase, Phase::Energy, "Should advance to Energy phase");
    
    // Advance to Draw phase
    TurnEngine::advance_phase(&mut game_state);
    assert_eq!(game_state.current_phase, Phase::Draw, "Should advance to Draw phase");
    
    // Advance to Main phase
    TurnEngine::advance_phase(&mut game_state);
    assert_eq!(game_state.current_phase, Phase::Main, "Should advance to Main phase");
    
    // Verify turn number unchanged during normal phase progression
    assert_eq!(game_state.turn_number, 1, "Turn number should still be 1");
}

#[test]
fn test_move_real_card_from_hand_to_stage() {
    let cards = load_all_cards();
    
    // Find a member card with cost 0 or low cost
    let member_card = cards.iter().find(|c| c.is_member()).expect("Should have member card");
    
    let mut player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    // Add member card to hand
    player.hand.cards.push(member_card.card_no.parse::<i16>().unwrap_or(0));

    let initial_hand_count = player.hand.cards.len();
    let initial_stage_count = if player.stage.stage[1] != -1 { 1 } else { 0 };

    // Move card to stage (using cost 0 if possible, or will fail if cost > 0)
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    let result = player.move_card_from_hand_to_stage(0, MemberArea::Center, false, &card_database);

    if member_card.cost.unwrap_or(0) == 0 {
        assert!(result.is_ok(), "Should be able to play cost 0 card: {:?}", result);

        // Verify card moved
        assert!(player.stage.stage[1] != -1, "Card should be on stage");
        let card_id = player.stage.stage[1];
        let card_on_stage = card_database.get_card(card_id).unwrap();
        assert_eq!(card_on_stage.card_no, member_card.card_no, "Card should match");
    } else {
        // Card has cost > 0, so it should fail without energy
        assert!(result.is_err(), "Should fail for card with cost > 0 and no energy");
        assert_eq!(player.hand.cards.len(), initial_hand_count, "Hand count unchanged on failure");
    }
}

#[test]
fn test_move_real_card_to_occupied_stage_area() {
    let cards = load_all_cards();
    
    // Find member cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .take(2)
        .cloned()
        .collect();
    
    if member_cards.len() < 2 {
        println!("Need at least 2 member cards, skipping test");
        return;
    }
    
    let mut player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    // Add two member cards to hand
    player.hand.cards.push(member_cards[0].card_no.parse::<i16>().unwrap_or(0));
    player.hand.cards.push(member_cards[1].card_no.parse::<i16>().unwrap_or(0));

    // Move first card to center
    let card_database = create_card_database(cards.clone());
    let result = player.move_card_from_hand_to_stage(0, MemberArea::Center, false, &card_database);

    if member_cards[0].cost.unwrap_or(0) == 0 {
        assert!(result.is_ok(), "Should play first card if cost is 0");
        assert!(player.stage.stage[1] != -1, "Center should be occupied");

        // Try to move second card to same area (should fail)
        let result2 = player.move_card_from_hand_to_stage(0, MemberArea::Center, false, &card_database);
        assert!(result2.is_err(), "Should fail when area is occupied");
        assert_eq!(player.hand.cards.len(), 1, "Second card should remain in hand");

        // Verify first card still on stage
        let card_id = player.stage.stage[1];
        let card_on_stage = card_database.get_card(card_id).unwrap();
        assert_eq!(card_on_stage.card_no, member_cards[0].card_no, "Original card should remain");
    } else {
        println!("First card has cost > 0, skipping occupied area test");
    }
}

#[test]
fn test_draw_real_card_from_deck() {
    let cards = load_all_cards();
    
    // Find member cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .take(5)
        .cloned()
        .collect();
    
    if member_cards.len() < 5 {
        println!("Need at least 5 member cards, skipping test");
        return;
    }
    
    let mut player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    // Add cards to main deck
    for card in member_cards {
        player.main_deck.cards.push(card.card_no.parse::<i16>().unwrap_or(0));
    }
    
    let initial_deck_count = player.main_deck.cards.len();
    let initial_hand_count = player.hand.cards.len();
    
    // Draw a card
    let drawn = player.draw_card();
    assert!(drawn.is_some(), "Should successfully draw a card");
    assert_eq!(player.hand.cards.len(), initial_hand_count + 1, "Hand should have 1 more card");
    assert_eq!(player.main_deck.cards.len(), initial_deck_count - 1, "Deck should have 1 less card");
    
    // Verify drawn card
    let drawn_card = drawn.unwrap();
    assert_eq!(player.hand.cards[0], drawn_card, "Drawn card should be in hand");
}

#[test]
fn test_victory_condition_with_real_live_cards() {
    let cards = load_all_cards();
    
    // Find live cards
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .take(3)
        .cloned()
        .collect();
    
    if live_cards.len() < 3 {
        println!("Need at least 3 live cards, skipping test");
        return;
    }
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    // Add 3 live cards to player1's success zone
    for card in live_cards {
        player1.success_live_card_zone.cards.push(card.card_no.parse::<i16>().unwrap_or(0));
    }

    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    let mut game_state = GameState::new(player1, player2, card_database);
    
    // Initially game should be ongoing
    assert_eq!(game_state.game_result, GameResult::Ongoing, "Game should be ongoing initially");
    
    // Check victory condition
    TurnEngine::check_victory_condition(&mut game_state);
    
    // Player1 should win (3 cards vs 0)
    assert_eq!(game_state.game_result, GameResult::FirstAttackerWins, "Player1 should win with 3 success cards");
    
    // Verify state
    assert_eq!(game_state.player1.success_live_card_zone.cards.len(), 3, "Player1 should have 3 success cards");
    assert_eq!(game_state.player2.success_live_card_zone.cards.len(), 0, "Player2 should have 0 success cards");
}
