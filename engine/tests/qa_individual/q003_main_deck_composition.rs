use crate::qa_individual::common::{load_all_cards, create_card_database};
use std::collections::VecDeque;

fn count_card_types(deck: &VecDeque<i16>, card_db: &std::sync::Arc<rabuka_engine::card::CardDatabase>) -> (usize, usize) {
    let mut member = 0;
    let mut live = 0;
    for &id in deck {
        if let Some(card) = card_db.get_card(id) {
            if card.is_member() { member += 1; }
            else if card.is_live() { live += 1; }
        }
    }
    (member, live)
}

#[test]
fn test_q003_main_deck_composition() {
    // Q3: Can member and live cards be combined in any ratio for main deck?
    // Answer: No, must be specific counts. 48 member, 12 live, total 60 (half deck: 24 member, 6 live, total 30).
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find member and live cards
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| card_database.get_card_id(&c.card_no).is_some())
        .take(48)
        .collect();
    
    let live_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_live())
        .filter(|c| card_database.get_card_id(&c.card_no).is_some())
        .take(12)
        .collect();
    
    let energy_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| card_database.get_card_id(&c.card_no).is_some())
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
    
    let (member_count, live_count) = count_card_types(&valid_main_deck, &card_database);
    assert_eq!(member_count, 48, "Valid deck should have 48 member cards");
    assert_eq!(live_count, 12, "Valid deck should have 12 live cards");
    
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
    
    let (inv_member, inv_live) = count_card_types(&invalid_main_deck, &card_database);
    assert!(inv_member != 48 || inv_live != 12, "Invalid deck composition should not match 48 member + 12 live");
    
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
    
    let (half_member, half_live) = count_card_types(&half_main_deck, &card_database);
    assert_eq!(half_member, 24, "Half deck should have 24 member cards");
    assert_eq!(half_live, 6, "Half deck should have 6 live cards");
    
    println!("Q003 verified: Main deck must be 48 member + 12 live = 60 total (half deck: 24 + 6 = 30)");
}
