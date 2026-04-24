/// Comprehensive gameplay tests for complex abilities
/// Tests full execution flow of abilities with multiple steps, user choices, and sequential actions

use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::player::Player;
use rabuka_engine::game_state::GameState;
use rabuka_engine::ability_resolver::{AbilityResolver, ChoiceResult};
use rabuka_engine::card::AbilityEffect;

fn load_all_cards() -> Vec<rabuka_engine::card::Card> {
    CardLoader::load_cards_from_file(
        std::path::Path::new("../cards/cards.json")
    ).expect("Failed to load cards")
}

fn create_card_database(cards: Vec<rabuka_engine::card::Card>) -> std::sync::Arc<CardDatabase> {
    std::sync::Arc::new(CardDatabase::load_or_create(cards))
}

fn get_card_id(card: &rabuka_engine::card::Card, card_database: &CardDatabase) -> i16 {
    card_database.get_card_id(&card.card_no).unwrap_or(0)
}

/// Test: Complex look_and_select ability with sequential actions
/// "自分のデッキの上からカードを3枚見る。その中から好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く。"
#[test]
fn test_look_and_select_sequential_actions() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    // Set up deck with known cards
    let member_cards: Vec<i16> = cards.iter()
        .filter(|c| c.is_member())
        .map(|c| get_card_id(c, &card_database))
        .take(10)
        .collect();

    player1.main_deck.cards = member_cards.clone().into();
    player1.hand.cards = vec![].into();

    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;

    let mut resolver = AbilityResolver::new(&mut game_state);

    // Create look_and_select effect with sequential actions
    let look_action = AbilityEffect {
        text: "自分のデッキの上からカードを3枚見る。".to_string(),
        action: "look_at".to_string(),
        source: Some("deck_top".to_string()),
        count: Some(3),
        target: Some("self".to_string()),
        ..Default::default()
    };

    let select_action = AbilityEffect {
        text: "好きな枚数を好きな順番でデッキの上に置き、残りを控え室に置く".to_string(),
        action: "sequential".to_string(),
        actions: Some(vec![
            AbilityEffect {
                text: "好きな枚数を好きな順番でデッキの上に置き".to_string(),
                placement_order: Some("any_order".to_string()),
                action: "move_cards".to_string(),
                destination: Some("deck_top".to_string()),
                any_number: Some(true),
                ..Default::default()
            },
            AbilityEffect {
                text: "残りを控え室に置く".to_string(),
                destination: Some("discard".to_string()),
                action: "move_cards".to_string(),
                ..Default::default()
            }
        ]),
        ..Default::default()
    };

    let look_and_select_effect = AbilityEffect {
        text: "look_and_select test".to_string(),
        action: "look_and_select".to_string(),
        look_action: Some(Box::new(look_action)),
        select_action: Some(Box::new(select_action)),
        ..Default::default()
    };

    // Execute the effect
    let result = resolver.execute_effect(&look_and_select_effect);
    assert!(result.is_ok(), "look_and_select should succeed");

    // Should have pending choice for card selection
    assert!(resolver.get_pending_choice().is_some(), "Should have pending choice after look_at");

    // Verify looked_at_cards has 3 cards
    assert_eq!(resolver.looked_at_cards.len(), 3, "Should have looked at 3 cards");

    // Simulate user selecting 2 cards to put back on deck
    let selected_indices = vec![0, 1];
    let choice_result = ChoiceResult::CardSelected { indices: selected_indices };

    // Provide choice and resume execution
    let result = resolver.provide_choice_result(choice_result);
    assert!(result.is_ok(), "Providing choice should succeed");

    // Verify execution completed
    assert!(resolver.get_pending_choice().is_none(), "Should have no pending choice after completion");

    // Verify cards were moved correctly
    // 2 cards should be on top of deck, 1 in discard
    let deck_size = game_state.player1.main_deck.cards.len();
    let discard_size = game_state.player1.waitroom.cards.len();

    assert!(deck_size >= 2, "Deck should have at least 2 cards");
    assert_eq!(discard_size, 1, "Discard should have 1 card");
}

/// Test: Look and select with optional cost
/// "手札を1枚控え室に置いてもよい：自分のデッキの上からカードを3枚見る。その中から1枚を手札に加え、残りを控え室に置く。"
#[test]
fn test_look_and_select_with_optional_cost() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    // Set up deck and hand
    let member_cards: Vec<i16> = cards.iter()
        .filter(|c| c.is_member())
        .map(|c| get_card_id(c, &card_database))
        .take(10)
        .collect();

    player1.main_deck.cards = member_cards.clone().into();
    player1.hand.cards = vec![member_cards[0]].into();

    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;

    let mut resolver = AbilityResolver::new(&mut game_state);

    // Execute optional cost directly
    let cost_effect = AbilityEffect {
        text: "手札を1枚控え室に置いてもよい".to_string(),
        action: "move_cards".to_string(),
        source: Some("hand".to_string()),
        destination: Some("discard".to_string()),
        count: Some(1),
        optional: Some(true),
        ..Default::default()
    };

    resolver.current_ability = Some(rabuka_engine::card::Ability {
        cost: Some(rabuka_engine::card::AbilityCost {
            text: "手札を1枚控え室に置いてもよい".to_string(),
            effect: Some(cost_effect),
            ..Default::default()
        }),
        effect: None,
        ..Default::default()
    });

    // Execute the cost
    let result = resolver.execute_cost(&resolver.current_ability.as_ref().unwrap().cost.as_ref().unwrap());
    assert!(result.is_ok(), "Cost execution should succeed");

    // Should have pending choice for optional cost
    assert!(resolver.get_pending_choice().is_some(), "Should have pending choice for optional cost");

    // Simulate user choosing to pay the cost
    let selected_indices = vec![0];
    let choice_result = ChoiceResult::CardSelected { indices: selected_indices };

    let result = resolver.provide_choice_result(choice_result);
    assert!(result.is_ok(), "Providing cost choice should succeed");

    // Verify cost was paid
    assert_eq!(game_state.player1.hand.cards.len(), 0, "Hand should be empty after paying cost");
    assert_eq!(game_state.player1.waitroom.cards.len(), 1, "Discard should have 1 card");
}

/// Test: Sequential effects in ability
#[test]
fn test_sequential_effects() {
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());

    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    // Set up deck
    let member_cards: Vec<i16> = cards.iter()
        .filter(|c| c.is_member())
        .map(|c| get_card_id(c, &card_database))
        .take(10)
        .collect();

    player1.main_deck.cards = member_cards.clone().into();
    player1.hand.cards = vec![].into();

    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_state.current_phase = rabuka_engine::game_state::Phase::Main;
    game_state.turn_number = 1;

    let mut resolver = AbilityResolver::new(&mut game_state);

    // Create sequential effect: draw 2, then discard 1
    let sequential_effect = AbilityEffect {
        text: "カードを2枚引き、手札を1枚控え室に置く".to_string(),
        action: "sequential".to_string(),
        actions: Some(vec![
            AbilityEffect {
                text: "カードを2枚引く".to_string(),
                action: "draw".to_string(),
                count: Some(2),
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

    // Execute the sequential effect
    let result = resolver.execute_effect(&sequential_effect);
    assert!(result.is_ok(), "Sequential effect should succeed");

    // Verify final state: drew 2, discarded 1 = 1 card in hand
    let hand_size = game_state.player1.hand.cards.len();
    let discard_size = game_state.player1.waitroom.cards.len();

    assert_eq!(hand_size, 1, "Hand should have 1 card (2 drawn - 1 discarded)");
    assert_eq!(discard_size, 1, "Discard should have 1 card");
}
