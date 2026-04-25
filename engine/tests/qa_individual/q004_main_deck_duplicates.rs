use rabuka_engine::deck_builder::DeckBuilder;
use crate::qa_individual::common::{load_all_cards, create_card_database};
use std::collections::VecDeque;

#[test]
fn test_q004_main_deck_duplicates() {
    // Q4: How many same cards can be used in main deck?
    // Answer: Cards with same card number are same card, max 4 of same card. Card number excludes rarity symbol.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find a member card with multiple rarities
    let member_card = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| c.card_no.contains("-R") || c.card_no.contains("-P"))
        .next();
    
    if let Some(card) = member_card {
        let card_id = card_database.get_card_id(&card.card_no).unwrap();
        
        // Test valid deck with 4 copies of same card number
        let mut valid_deck: VecDeque<i16> = VecDeque::new();
        for _ in 0..4 {
            valid_deck.push_back(card_id);
        }
        
        // Fill with other cards to reach 60
        let other_cards: Vec<_> = cards.iter()
            .filter(|c| c.is_member() && c.card_no != card.card_no)
            .filter(|c| card_database.get_card_id(&c.card_no).is_some())
            .take(56)
            .collect();
        
        for other in &other_cards {
            if let Some(other_id) = card_database.get_card_id(&other.card_no) {
                valid_deck.push_back(other_id);
            }
        }
        
        let energy_deck: VecDeque<i16> = VecDeque::new();
        let validation = DeckBuilder::validate_deck(&card_database, &valid_deck, &energy_deck);
        assert!(validation.is_valid, "Deck with 4 copies should be valid: {:?}", validation.errors);
        
        // Test invalid deck with 5 copies of same card number
        let mut invalid_deck: VecDeque<i16> = VecDeque::new();
        for _ in 0..5 {
            invalid_deck.push_back(card_id);
        }
        
        for other in other_cards.iter().take(55) {
            if let Some(other_id) = card_database.get_card_id(&other.card_no) {
                invalid_deck.push_back(other_id);
            }
        }
        
        let invalid_validation = DeckBuilder::validate_deck(&card_database, &invalid_deck, &energy_deck);
        assert!(!invalid_validation.is_valid, "Deck with 5 copies should be invalid");
        assert!(invalid_validation.errors.iter().any(|e| e.contains("maximum is 4")));
        
        println!("Q004 verified: Same card number means same card, max 4 of same card in main deck");
    } else {
        panic!("Required member card not found for Q004 test");
    }
}
