// Common helper functions for QA tests
// This module provides shared utilities for setting up game state and testing scenarios

use rabuka_engine::card::{Card, CardDatabase};
use rabuka_engine::card_loader::CardLoader;
use std::path::Path;
use std::sync::Arc;

// Re-export types for use in test files
pub use rabuka_engine::player::Player;

/// Load all cards from cards.json
pub fn load_all_cards() -> Vec<Card> {
    let cards_path = Path::new("../cards/cards.json");
    CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards")
}

/// Create CardDatabase from loaded cards
pub fn create_card_database(cards: Vec<Card>) -> Arc<CardDatabase> {
    Arc::new(CardDatabase::load_or_create(cards))
}

/// Get card ID from card using CardDatabase
pub fn get_card_id(card: &Card, card_database: &Arc<CardDatabase>) -> i16 {
    card_database.get_card_id(&card.card_no).unwrap_or(0)
}

/// Set up a player with specific cards in hand
pub fn setup_player_with_hand(player: &mut Player, card_ids: Vec<i16>) {
    player.hand.cards = card_ids.into_iter().collect();
}

/// Set up a player with specific energy cards (all active)
pub fn setup_player_with_energy(player: &mut Player, card_ids: Vec<i16>) {
    let count = card_ids.len();
    player.energy_zone.cards = card_ids.into_iter().collect();
    player.energy_zone.active_energy_count = count;
}

/// Set up a player with specific energy cards (mixed active/wait)
#[allow(dead_code)]
pub fn setup_player_with_mixed_energy(player: &mut Player, card_ids: Vec<i16>, active_count: usize) {
    player.energy_zone.cards = card_ids.into_iter().collect();
    player.energy_zone.active_energy_count = active_count;
}

/// Find a card by card number from the loaded cards
#[allow(dead_code)]
pub fn find_card_by_no<'a>(cards: &'a [Card], card_no: &str) -> Option<&'a Card> {
    cards.iter().find(|c| c.card_no == card_no)
}

/// Find a member card with a specific cost
#[allow(dead_code)]
pub fn find_member_card_with_cost(cards: &[Card], cost: u32) -> Option<&Card> {
    cards.iter()
        .filter(|c| c.is_member())
        .find(|c| c.cost == Some(cost))
}

/// Find an energy card
#[allow(dead_code)]
pub fn find_energy_card(cards: &[Card]) -> Option<&Card> {
    cards.iter().find(|c| c.is_energy())
}

/// Find a live card
#[allow(dead_code)]
pub fn find_live_card(cards: &[Card]) -> Option<&Card> {
    cards.iter().find(|c| c.is_live())
}

/// Set up a player with specific cards in deck
#[allow(dead_code)]
pub fn setup_player_with_deck(player: &mut Player, card_ids: Vec<i16>) {
    player.main_deck.cards = card_ids.into_iter().collect();
}

/// Set up a player with specific cards on stage
pub fn setup_player_with_stage(player: &mut Player, card_ids: Vec<i16>) {
    for (i, card_id) in card_ids.iter().enumerate() {
        if i < 3 {
            player.stage.stage[i] = *card_id;
        }
    }
}

