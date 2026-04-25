use rabuka_engine::deck_builder::DeckBuilder;
use crate::qa_individual::common::{load_all_cards, create_card_database};
use std::collections::VecDeque;

#[test]
fn test_q003_main_deck_composition() {
    // Q3: Can member and live cards be combined in any ratio for main deck?
    // Answer: No, must be specific counts. 48 member, 12 live, total 60 (half deck: 24 member, 6 live, total 30).
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find member and live cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| create_card_database(cards.clone()).get_card_id(&c.card_no).is_some())
        .take(48)
        .collect();
    
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .filter(|c| create_card_database(cards.clone()).get_card_id(&c.card_no).is_some())
        .take(12)
        .collect();
    
    let energy_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| create_card_database(cards.clone()).get_card_id(&c.card_no).is_some())
        .take(12)
        .collect();
    
    // Test valid full deck (48 member + 12 live = 60)
    let mut valid_main_deck: VecDeque<i16> = VecDeque::new();
    for member in &member_cards {
        if let Some(card_id) = card_database.get_card_id(&member.card_no) {
            valid_main_deck.push_back(card_id);
        }
    }
    for live in &live_cards {
        if let Some(card_id) = card_database.get_card_id(&live.card_no) {
            valid_main_deck.push_back(card_id);
        }
    }
    
    let mut valid_energy_deck: VecDeque<i16> = VecDeque::new();
    for energy in &energy_cards {
        if let Some(card_id) = card_database.get_card_id(&energy.card_no) {
            valid_energy_deck.push_back(card_id);
        }
    }
    
    let validation = DeckBuilder::validate_deck(&card_database, &valid_main_deck, &valid_energy_deck);
    assert!(validation.is_valid, "Valid deck should pass validation: {:?}", validation.errors);
    
    // Test invalid deck (wrong composition)
    let mut invalid_main_deck: VecDeque<i16> = VecDeque::new();
    for member in member_cards.iter().take(50) {
        if let Some(card_id) = card_database.get_card_id(&member.card_no) {
            invalid_main_deck.push_back(card_id);
        }
    }
    for live in live_cards.iter().take(10) {
        if let Some(card_id) = card_database.get_card_id(&live.card_no) {
            invalid_main_deck.push_back(card_id);
        }
    }
    
    let invalid_validation = DeckBuilder::validate_deck(&card_database, &invalid_main_deck, &valid_energy_deck);
    assert!(!invalid_validation.is_valid, "Invalid deck should fail validation");
    
    // Test valid half deck (24 member + 6 live = 30)
    let mut half_main_deck: VecDeque<i16> = VecDeque::new();
    for member in member_cards.iter().take(24) {
        if let Some(card_id) = card_database.get_card_id(&member.card_no) {
            half_main_deck.push_back(card_id);
        }
    }
    for live in live_cards.iter().take(6) {
        if let Some(card_id) = card_database.get_card_id(&live.card_no) {
            half_main_deck.push_back(card_id);
        }
    }
    
    let half_validation = DeckBuilder::validate_deck(&card_database, &half_main_deck, &valid_energy_deck);
    assert!(half_validation.is_valid, "Valid half deck should pass validation: {:?}", half_validation.errors);
    
    println!("Q003 verified: Main deck must be 48 member + 12 live = 60 total (half deck: 24 + 6 = 30)");
}
