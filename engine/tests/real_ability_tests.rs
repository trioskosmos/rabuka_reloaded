// Tests based on actual abilities from abilities.json
// These tests verify that real ability patterns work correctly

use rabuka_engine::ability_resolver::AbilityResolver;
use rabuka_engine::game_state::GameState;
use rabuka_engine::card_loader::CardDatabase;
use rabuka_engine::card::{Ability, AbilityEffect, AbilityCost, Card};

#[test]
fn test_real_ability_self_cost_move_cards() {
    // Based on ability: "このメンバーをステージから控え室に置く：自分の控え室からライブカードを1枚手札に加える"
    // Cost: self_cost move_cards from stage to discard
    // Effect: move_cards from discard to hand, card_type: live_card
    
    let mut game_state = GameState::new();
    let mut card_db = CardDatabase::new();
    
    // Create test cards
    let member_card = Card {
        card_no: "TEST-MEMBER-001".to_string(),
        name: "Test Member".to_string(),
        group: "Test".to_string(),
        unit: "Test".to_string(),
        cost: Some(1),
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: None,
        required_hearts: vec![],
        card_type: "member".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let live_card = Card {
        card_no: "TEST-LIVE-001".to_string(),
        name: "Test Live".to_string(),
        group: "Test".to_string(),
        unit: "Test".to_string(),
        cost: None,
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: Some(10),
        required_hearts: vec![],
        card_type: "live".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let member_card_id = card_db.add_card(member_card);
    let live_card_id = card_db.add_card(live_card);
    
    let mut resolver = AbilityResolver::new(&mut game_state, &card_db);
    
    // Setup: Member on stage, live card in discard
    game_state.player1.stage.stage[1] = member_card_id;
    game_state.player1.waitroom.add_card(live_card_id);
    game_state.activating_card = Some(member_card_id);
    
    // Create cost matching real ability
    let cost = AbilityCost {
        text: "このメンバーをステージから控え室に置く".to_string(),
        cost_type: Some("move_cards".to_string()),
        source: Some("stage".to_string()),
        destination: Some("discard".to_string()),
        card_type: Some("member_card".to_string()),
        self_cost: Some(true),
        ..Default::default()
    };
    
    // Create effect matching real ability
    let effect = AbilityEffect {
        text: "自分の控え室からライブカードを1枚手札に加える".to_string(),
        action: "move_cards".to_string(),
        source: Some("discard".to_string()),
        destination: Some("hand".to_string()),
        count: Some(1),
        card_type: Some("live_card".to_string()),
        target: Some("self".to_string()),
        ..Default::default()
    };
    
    // Execute cost
    let cost_result = resolver.execute_cost(&cost);
    assert!(cost_result.is_ok());
    assert!(game_state.player1.stage.stage[1] == -1); // Member removed from stage
    assert!(game_state.player1.waitroom.cards.contains(&member_card_id)); // Member in discard
    
    // Execute effect
    let effect_result = resolver.execute_move_cards(&effect);
    assert!(effect_result.is_ok());
    assert!(game_state.player1.hand.cards.contains(&live_card_id)); // Live card in hand
    assert!(!game_state.player1.waitroom.cards.contains(&live_card_id)); // Live card removed from discard
}

#[test]
fn test_real_ability_optional_cost_with_effect() {
    // Based on ability: "手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る"
    // Cost: optional move_cards from hand to discard
    // Effect: look_at from deck_top
    
    let mut game_state = GameState::new();
    let mut card_db = CardDatabase::new();
    
    // Create test cards
    let hand_card = Card {
        card_no: "TEST-001".to_string(),
        name: "Test Card".to_string(),
        group: "Test".to_string(),
        unit: "Test".to_string(),
        cost: Some(1),
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: None,
        required_hearts: vec![],
        card_type: "member".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let deck_card1 = Card {
        card_no: "TEST-002".to_string(),
        name: "Test Card 2".to_string(),
        group: "Test".to_string(),
        unit: "Test".to_string(),
        cost: Some(1),
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: None,
        required_hearts: vec![],
        card_type: "member".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let hand_card_id = card_db.add_card(hand_card);
    let deck_card1_id = card_db.add_card(deck_card1);
    
    let mut resolver = AbilityResolver::new(&mut game_state, &card_db);
    
    // Setup: Card in hand, card in deck
    game_state.player1.hand.add_card(hand_card_id);
    game_state.player1.main_deck.cards.push(deck_card1_id);
    
    // Create optional cost matching real ability
    let cost = AbilityCost {
        text: "手札を1枚控え室に置いてもよい".to_string(),
        cost_type: Some("move_cards".to_string()),
        source: Some("hand".to_string()),
        destination: Some("discard".to_string()),
        count: Some(1),
        optional: Some(true),
        ..Default::default()
    };
    
    // Execute cost - should set pending choice
    let cost_result = resolver.execute_cost(&cost);
    // This will set pending_choice for user to decide whether to pay
    // For now, just verify it doesn't error
    assert!(cost_result.is_ok());
}

#[test]
fn test_real_ability_change_state_with_cost_limit() {
    // Based on ability: "相手のステージにいるコスト4以下のメンバー1人をウェイトにする"
    // Effect: change_state to wait, target: opponent, cost_limit: 4, card_type: member_card
    
    let mut game_state = GameState::new();
    let mut card_db = CardDatabase::new();
    
    // Create test member cards with different costs
    let cost3_card = Card {
        card_no: "TEST-MEMBER-C3".to_string(),
        name: "Cost 3 Member".to_string(),
        group: "Test".to_string(),
        unit: "Test".to_string(),
        cost: Some(3),
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: None,
        required_hearts: vec![],
        card_type: "member".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let cost5_card = Card {
        card_no: "TEST-MEMBER-C5".to_string(),
        name: "Cost 5 Member".to_string(),
        group: "Test".to_string(),
        unit: "Test".to_string(),
        cost: Some(5),
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: None,
        required_hearts: vec![],
        card_type: "member".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let cost3_id = card_db.add_card(cost3_card);
    let cost5_id = card_db.add_card(cost5_card);
    
    let mut resolver = AbilityResolver::new(&mut game_state, &card_db);
    
    // Setup: Opponent has both cards on stage
    game_state.player2.stage.stage[0] = cost3_id;
    game_state.player2.stage.stage[1] = cost5_id;
    
    // Create effect matching real ability
    let effect = AbilityEffect {
        text: "相手のステージにいるコスト4以下のメンバー1人をウェイトにする".to_string(),
        action: "change_state".to_string(),
        state_change: Some("wait".to_string()),
        count: Some(1),
        card_type: Some("member_card".to_string()),
        target: Some("opponent".to_string()),
        cost_limit: Some(4),
        ..Default::default()
    };
    
    // Execute effect
    let result = resolver.execute_change_state(&effect);
    // Should prompt for choice (only cost3 card is valid)
    assert!(result.is_ok());
    // Cost 3 card should be selectable, cost 5 card should not
}

#[test]
fn test_real_ability_group_filter() {
    // Based on ability: "自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える"
    // Effect: move_cards from discard to hand, card_type: live_card, group: 虹ヶ咲
    
    let mut game_state = GameState::new();
    let mut card_db = CardDatabase::new();
    
    // Create test live cards from different groups
    let nijigaku_live = Card {
        card_no: "TEST-LIVE-NIJIGAKU".to_string(),
        name: "Nijigaku Live".to_string(),
        group: "虹ヶ咲".to_string(),
        unit: "Test".to_string(),
        cost: None,
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: Some(10),
        required_hearts: vec![],
        card_type: "live".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let mus_live = Card {
        card_no: "TEST-LIVE-MUS".to_string(),
        name: "Muse Live".to_string(),
        group: "μ's".to_string(),
        unit: "Test".to_string(),
        cost: None,
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: Some(10),
        required_hearts: vec![],
        card_type: "live".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let nijigaku_id = card_db.add_card(nijigaku_live);
    let mus_id = card_db.add_card(mus_live);
    
    let mut resolver = AbilityResolver::new(&mut game_state, &card_db);
    
    // Setup: Both live cards in discard
    game_state.player1.waitroom.add_card(nijigaku_id);
    game_state.player1.waitroom.add_card(mus_id);
    
    // Create effect matching real ability
    let effect = AbilityEffect {
        text: "自分の控え室から『虹ヶ咲』のライブカードを1枚手札に加える".to_string(),
        action: "move_cards".to_string(),
        source: Some("discard".to_string()),
        destination: Some("hand".to_string()),
        count: Some(1),
        card_type: Some("live_card".to_string()),
        target: Some("self".to_string()),
        group: Some(rabuka_engine::card::GroupInfo {
            name: "虹ヶ咲".to_string(),
        }),
        ..Default::default()
    };
    
    // Execute effect
    let result = resolver.execute_move_cards(&effect);
    assert!(result.is_ok());
    // Only Nijigaku card should be in hand
    assert!(game_state.player1.hand.cards.contains(&nijigaku_id));
    assert!(!game_state.player1.hand.cards.contains(&mus_id));
}

#[test]
fn test_real_ability_sequential_actions() {
    // Based on ability: "カードを1枚引き、手札を1枚控え室に置く"
    // Effect: sequential with two actions: draw_card, then move_cards
    
    let mut game_state = GameState::new();
    let mut card_db = CardDatabase::new();
    
    // Create test cards
    let deck_card = Card {
        card_no: "TEST-DECK-001".to_string(),
        name: "Deck Card".to_string(),
        group: "Test".to_string(),
        unit: "Test".to_string(),
        cost: Some(1),
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: None,
        required_hearts: vec![],
        card_type: "member".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let hand_card = Card {
        card_no: "TEST-HAND-001".to_string(),
        name: "Hand Card".to_string(),
        group: "Test".to_string(),
        unit: "Test".to_string(),
        cost: Some(1),
        blade_heart: None,
        blade: None,
        heart: vec![],
        score: None,
        required_hearts: vec![],
        card_type: "member".to_string(),
        text: "".to_string(),
        ..Default::default()
    };
    
    let deck_card_id = card_db.add_card(deck_card);
    let hand_card_id = card_db.add_card(hand_card);
    
    let mut resolver = AbilityResolver::new(&mut game_state, &card_db);
    
    // Setup: Card in deck, card in hand
    game_state.player1.main_deck.cards.push(deck_card_id);
    game_state.player1.hand.add_card(hand_card_id);
    
    // Create sequential effect matching real ability
    let effect = AbilityEffect {
        text: "カードを1枚引き、手札を1枚控え室に置く".to_string(),
        action: "sequential".to_string(),
        actions: Some(vec![
            AbilityEffect {
                text: "カードを1枚引き".to_string(),
                action: "draw_card".to_string(),
                count: Some(1),
                source: Some("deck".to_string()),
                destination: Some("hand".to_string()),
                ..Default::default()
            },
            AbilityEffect {
                text: "手札を1枚控え室に置く".to_string(),
                action: "move_cards".to_string(),
                source: Some("hand".to_string()),
                destination: Some("discard".to_string()),
                count: Some(1),
                ..Default::default()
            }
        ]),
        ..Default::default()
    };
    
    // Execute effect
    let result = resolver.execute_sequential(&effect);
    assert!(result.is_ok());
    // Should have drawn 1 card and discarded 1 card
    // Note: The discard will prompt for user choice
}
