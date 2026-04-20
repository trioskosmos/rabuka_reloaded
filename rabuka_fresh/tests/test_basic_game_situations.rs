use rabuka_engine::card::{Ability, AbilityCost, AbilityEffect, Card, CardType, HeartColor};
use rabuka_engine::game_state::{GameState, Phase, TurnPhase};
use rabuka_engine::player::Player;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;
use std::collections::HashMap;

#[test]
fn test_game_state_initialization() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let game_state = GameState::new(player1, player2);
    
    assert_eq!(game_state.turn_number, 1);
    assert_eq!(game_state.current_turn_phase, TurnPhase::FirstAttackerNormal);
    assert_eq!(game_state.current_phase, Phase::Active);
    assert!(game_state.is_first_turn);
}

#[test]
fn test_player_creation() {
    let player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    assert_eq!(player.id, "test_player");
    assert_eq!(player.name, "Test Player");
    assert!(player.is_first_attacker);
    assert!(player.hand.cards.is_empty());
    assert!(player.main_deck.is_empty());
    assert!(player.stage.left_side.is_none());
    assert!(player.stage.center.is_none());
    assert!(player.stage.right_side.is_none());
}

#[test]
fn test_advance_phase_normal_turn() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let mut game_state = GameState::new(player1, player2);
    
    // Start in Active phase
    assert_eq!(game_state.current_phase, Phase::Active);
    
    // Advance to Energy phase
    TurnEngine::advance_phase(&mut game_state);
    assert_eq!(game_state.current_phase, Phase::Energy);
    
    // Advance to Draw phase
    TurnEngine::advance_phase(&mut game_state);
    assert_eq!(game_state.current_phase, Phase::Draw);
    
    // Advance to Main phase
    TurnEngine::advance_phase(&mut game_state);
    assert_eq!(game_state.current_phase, Phase::Main);
}

#[test]
fn test_move_card_from_hand_to_stage() {
    let mut player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    // Add a member card to hand
    let mut hearts = HashMap::new();
    hearts.insert("heart01".to_string(), 2);
    
    let card = Card {
        card_no: "TEST-001".to_string(),
        img: None,
        name: "Test Member".to_string(),
        product: "Test".to_string(),
        card_type: CardType::Member,
        series: "Test".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: Some(rabuka_engine::card::BaseHeart { hearts }),
        need_heart: None,
        special_heart: None,
        blade: 1,
        blade_heart: None,
        score: Some(1000),
        rare: "R".to_string(),
        ability: String::new(),
        abilities: vec![],
        faq: vec![],
        _img: None,
    };
    
    player.hand.cards.push(card);
    
    // Move card to stage
    let result = player.move_card_from_hand_to_stage(0, MemberArea::Center);
    assert!(result.is_ok());
    
    // Verify card is on stage
    assert!(player.stage.center.is_some());
    assert_eq!(player.stage.center.as_ref().unwrap().card.name, "Test Member");
    assert!(player.hand.cards.is_empty());
}

#[test]
fn test_move_card_to_stage_occupied_area() {
    let mut player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    // Add two member cards to hand
    let mut hearts = HashMap::new();
    hearts.insert("heart01".to_string(), 2);
    
    let card1 = Card {
        card_no: "TEST-001".to_string(),
        img: None,
        name: "Test Member 1".to_string(),
        product: "Test".to_string(),
        card_type: CardType::Member,
        series: "Test".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: Some(rabuka_engine::card::BaseHeart { hearts: hearts.clone() }),
        need_heart: None,
        special_heart: None,
        blade: 1,
        blade_heart: None,
        score: Some(1000),
        rare: "R".to_string(),
        ability: String::new(),
        abilities: vec![],
        faq: vec![],
        _img: None,
    };
    
    let card2 = Card {
        card_no: "TEST-002".to_string(),
        img: None,
        name: "Test Member 2".to_string(),
        product: "Test".to_string(),
        card_type: CardType::Member,
        series: "Test".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(1),
        base_heart: Some(rabuka_engine::card::BaseHeart { hearts }),
        need_heart: None,
        special_heart: None,
        blade: 1,
        blade_heart: None,
        score: Some(1000),
        rare: "R".to_string(),
        ability: String::new(),
        abilities: vec![],
        faq: vec![],
        _img: None,
    };
    
    player.hand.cards.push(card1);
    player.hand.cards.push(card2);
    
    // Move first card to center
    let result = player.move_card_from_hand_to_stage(0, MemberArea::Center);
    assert!(result.is_ok());
    assert!(player.stage.center.is_some());
    
    // Try to move second card to same area (should fail)
    let result = player.move_card_from_hand_to_stage(0, MemberArea::Center);
    assert!(result.is_err());
}

#[test]
fn test_can_play_member_to_stage() {
    let mut player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    // No cards in hand
    assert!(!player.can_play_member_to_stage());
    
    // Add a member card with cost 0
    let mut hearts = HashMap::new();
    hearts.insert("heart01".to_string(), 2);
    
    let card = Card {
        card_no: "TEST-001".to_string(),
        img: None,
        name: "Test Member".to_string(),
        product: "Test".to_string(),
        card_type: CardType::Member,
        series: "Test".to_string(),
        group: "Test Group".to_string(),
        unit: None,
        cost: Some(0),
        base_heart: Some(rabuka_engine::card::BaseHeart { hearts }),
        need_heart: None,
        special_heart: None,
        blade: 1,
        blade_heart: None,
        score: Some(1000),
        rare: "R".to_string(),
        ability: String::new(),
        abilities: vec![],
        faq: vec![],
        _img: None,
    };
    
    player.hand.cards.push(card);
    assert!(player.can_play_member_to_stage());
}

#[test]
fn test_draw_card() {
    let mut player = Player::new("test_player".to_string(), "Test Player".to_string(), true);
    
    // Add cards to main deck
    let mut hearts = HashMap::new();
    hearts.insert("heart01".to_string(), 2);
    
    for i in 0..5 {
        let card = Card {
            card_no: format!("TEST-{:03}", i),
            img: None,
            name: format!("Test Card {}", i),
            product: "Test".to_string(),
            card_type: CardType::Member,
            series: "Test".to_string(),
            group: "Test Group".to_string(),
            unit: None,
            cost: Some(1),
            base_heart: Some(rabuka_engine::card::BaseHeart { hearts: hearts.clone() }),
            need_heart: None,
            special_heart: None,
            blade: 1,
            blade_heart: None,
            score: Some(1000),
            rare: "R".to_string(),
            ability: String::new(),
            abilities: vec![],
            faq: vec![],
            _img: None,
        };
        player.main_deck.cards.push_back(card);
    }
    
    // Draw a card
    let drawn = player.draw_card();
    assert!(drawn.is_some());
    assert_eq!(player.hand.cards.len(), 1);
    assert_eq!(player.main_deck.cards.len(), 4);
}

#[test]
fn test_victory_condition() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let mut game_state = GameState::new(player1, player2);
    
    // Initially no victory
    assert_eq!(game_state.check_victory(), rabuka_engine::game_state::GameResult::Ongoing);
    
    // Add 3 success cards to player 1
    let mut hearts = HashMap::new();
    hearts.insert("heart01".to_string(), 2);
    
    for i in 0..3 {
        let card = Card {
            card_no: format!("WIN-{:03}", i),
            img: None,
            name: format!("Win Card {}", i),
            product: "Test".to_string(),
            card_type: CardType::Live,
            series: "Test".to_string(),
            group: "Test Group".to_string(),
            unit: None,
            cost: Some(1),
            base_heart: Some(rabuka_engine::card::BaseHeart { hearts: hearts.clone() }),
            need_heart: None,
            special_heart: None,
            blade: 1,
            blade_heart: None,
            score: Some(1000),
            rare: "R".to_string(),
            ability: String::new(),
            abilities: vec![],
            faq: vec![],
            _img: None,
        };
        game_state.player1.success_live_card_zone.add_card(card);
    }
    
    // Player 1 should win (3 cards vs 0)
    assert_eq!(game_state.check_victory(), rabuka_engine::game_state::GameResult::FirstAttackerWins);
}
