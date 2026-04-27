/// Comprehensive gameplay tests for complex abilities
/// Tests full execution flow of abilities with multiple steps, user choices, and sequential actions
/// NOTE: These tests are currently disabled as placeholders because the required
/// engine methods are private or don't exist. They document test cases that need
/// proper engine API access to be implemented.

use rabuka_engine::card_loader::CardLoader;
use rabuka_engine::card::CardDatabase;
use rabuka_engine::player::Player;
use rabuka_engine::game_state::GameState;
use rabuka_engine::ability_resolver::AbilityResolver;

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
    // Note: execute_effect is not a public method on AbilityResolver
    // This test documents that the ability execution API needs to be made public
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let mut game_state = GameState::new(player1, player2, card_database);
    let resolver = AbilityResolver::new(&mut game_state);
    assert!(resolver.looked_at_cards.is_empty(), "Should have no looked_at cards initially");
}

/// Test: Optional cost with user choice
#[test]
fn test_optional_cost_with_choice() {
    // Note: execute_cost is not a public method on AbilityResolver
    // This test documents that the ability execution API needs to be made public
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let mut game_state = GameState::new(player1, player2, card_database);
    let resolver = AbilityResolver::new(&mut game_state);
    assert!(resolver.current_ability.is_none(), "Current ability should be None initially");
}

/// Test: Sequential effects in ability
#[test]
fn test_sequential_effects() {
    // Note: execute_effect is not a public method on AbilityResolver
    // This test documents that the ability execution API needs to be made public
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    let mut game_state = GameState::new(player1, player2, card_database);
    let resolver = AbilityResolver::new(&mut game_state);
    assert!(resolver.current_ability.is_none(), "Current ability should be None initially");
}
