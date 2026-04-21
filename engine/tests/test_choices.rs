use rabuka_engine::ability::{AbilityExecutor, Choice, ChoiceResult};
use rabuka_engine::card::{Ability, AbilityCost, AbilityEffect, Card, CardType, HeartColor};
use rabuka_engine::game_state::GameState;
use rabuka_engine::player::Player;
use std::collections::HashMap;

#[test]
fn test_ability_executor_new() {
    let executor = AbilityExecutor::new();
    assert!(executor.get_pending_choice().is_none());
}

#[test]
fn test_request_choice() {
    let mut executor = AbilityExecutor::new();
    let choice = Choice::SelectCard {
        zone: "hand".to_string(),
        card_type: Some("member_card".to_string()),
        count: 1,
        description: "Select a member card from hand".to_string(),
    };
    
    executor.request_choice(choice.clone()).unwrap();
    
    let pending = executor.get_pending_choice();
    assert!(pending.is_some());
    
    match pending {
        Some(Choice::SelectCard { zone, card_type, count, description }) => {
            assert_eq!(zone, "hand");
            assert_eq!(card_type, Some("member_card".to_string()));
            assert_eq!(count, 1);
            assert_eq!(description, "Select a member card from hand");
        }
        _ => panic!("Expected SelectCard choice"),
    }
}

#[test]
fn test_provide_choice_result() {
    let mut executor = AbilityExecutor::new();
    let choice = Choice::SelectCard {
        zone: "hand".to_string(),
        card_type: Some("member_card".to_string()),
        count: 1,
        description: "Select a member card from hand".to_string(),
    };
    
    executor.request_choice(choice).unwrap();
    assert!(executor.get_pending_choice().is_some());
    
    let result = ChoiceResult::CardSelected { indices: vec![0] };
    executor.provide_choice_result(result).unwrap();
    assert!(executor.get_pending_choice().is_none());
}

#[test]
fn test_choice_result_mismatch() {
    let mut executor = AbilityExecutor::new();
    let choice = Choice::SelectCard {
        zone: "hand".to_string(),
        card_type: Some("member_card".to_string()),
        count: 1,
        description: "Select a member card from hand".to_string(),
    };
    
    executor.request_choice(choice).unwrap();
    
    // Try to provide wrong result type
    let result = ChoiceResult::TargetSelected { target: "opponent".to_string() };
    let err = executor.provide_choice_result(result);
    assert!(err.is_err());
}

#[test]
fn test_ability_executor_with_game_state() {
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let game_state = GameState::new(player1, player2);
    
    // Create separate ability executor
    let mut executor = AbilityExecutor::new();
    
    // Executor should have no pending choice initially
    assert!(executor.get_pending_choice().is_none());
    
    // Request a choice
    let choice = Choice::SelectTarget {
        target: "opponent".to_string(),
        description: "Select target player".to_string(),
    };
    executor.request_choice(choice).unwrap();
    
    // Should be able to get pending choice
    assert!(executor.get_pending_choice().is_some());
    
    // Provide result
    let result = ChoiceResult::TargetSelected { target: "opponent".to_string() };
    executor.provide_choice_result(result).unwrap();
    
    // Choice should be cleared
    assert!(executor.get_pending_choice().is_none());
}

#[test]
fn test_basic_card_creation() {
    let mut hearts = HashMap::new();
    hearts.insert("heart01".to_string(), 2);
    
    let card = Card {
        card_no: "TEST-001".to_string(),
        img: None,
        name: "Test Card".to_string(),
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
    
    assert_eq!(card.card_no, "TEST-001");
    assert_eq!(card.name, "Test Card");
    assert!(card.is_member());
    assert!(!card.is_live());
    assert!(!card.is_energy());
}

