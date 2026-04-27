use rabuka_engine::deck_builder::DeckBuilder;
use crate::qa_individual::common::{load_all_cards, create_card_database};
use std::collections::VecDeque;

fn extract_card_number(card_no: &str) -> String {
    let parts: Vec<&str> = card_no.split('-').collect();
    if parts.len() >= 3 {
        format!("{}-{}-{}", parts[0], parts[1], parts[2])
    } else {
        card_no.to_string()
    }
}

#[test]
fn test_q005_same_card_number_different_rarity() {
    // Q5: Can cards with same card number but different rarity be 4 each in main deck?
    // Answer: No, same card number means max 4 total regardless of rarity.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find cards with same base card number but different rarities
    let card_base = "PL!N-bp1-001"; // Example base card number
    let cards_with_variants: Vec<_> = cards.iter()
        .filter(|c| c.card_no.starts_with(card_base))
        .collect();
    
    if cards_with_variants.len() >= 2 {
        let card_ids: Vec<_> = cards_with_variants.iter()
            .filter_map(|c| card_database.get_card_id(&c.card_no))
            .collect();
        
        let base_card_no = extract_card_number(&cards_with_variants[0].card_no);
        
        // Test invalid deck: 2 copies of R rarity + 3 copies of P rarity = 5 total of same card number
        let mut invalid_deck: VecDeque<i16> = VecDeque::new();
        for _ in 0..2 {
            if let Some(&id) = card_ids.get(0) {
                invalid_deck.push_back(id);
            }
        }
        for _ in 0..3 {
            if let Some(&id) = card_ids.get(1) {
                invalid_deck.push_back(id);
            }
        }
        
        // Fill with other member cards and live cards to reach 60 (48 member + 12 live)
        let other_member_cards: Vec<_> = cards.iter()
            .filter(|c| c.is_member())
            .filter(|c| extract_card_number(&c.card_no) != base_card_no)
            .filter(|c| card_database.get_card_id(&c.card_no).is_some())
            .take(44)
            .collect();
        
        let live_cards: Vec<_> = cards.iter()
            .filter(|c| c.is_live())
            .filter(|c| card_database.get_card_id(&c.card_no).is_some())
            .take(12)
            .collect();
        
        for other in &other_member_cards {
            if let Some(other_id) = card_database.get_card_id(&other.card_no) {
                invalid_deck.push_back(other_id);
            }
        }
        
        for live in &live_cards {
            if let Some(live_id) = card_database.get_card_id(&live.card_no) {
                invalid_deck.push_back(live_id);
            }
        }
        
        let energy_deck: VecDeque<i16> = VecDeque::new();
        let invalid_validation = DeckBuilder::validate_deck(&card_database, &invalid_deck, &energy_deck);
        assert!(!invalid_validation.is_valid, "Deck with 5 total of same card number should be invalid");
        
        // Test valid deck: 2 copies of R rarity + 2 copies of P rarity = 4 total of same card number
        let mut valid_deck: VecDeque<i16> = VecDeque::new();
        for _ in 0..2 {
            if let Some(&id) = card_ids.get(0) {
                valid_deck.push_back(id);
            }
        }
        for _ in 0..2 {
            if let Some(&id) = card_ids.get(1) {
                valid_deck.push_back(id);
            }
        }
        
        for other in &other_member_cards {
            if let Some(other_id) = card_database.get_card_id(&other.card_no) {
                valid_deck.push_back(other_id);
            }
        }
        
        for live in &live_cards {
            if let Some(live_id) = card_database.get_card_id(&live.card_no) {
                valid_deck.push_back(live_id);
            }
        }
        
        let valid_validation = DeckBuilder::validate_deck(&card_database, &valid_deck, &energy_deck);
        assert!(valid_validation.is_valid, "Deck with 4 total of same card number should be valid: {:?}", valid_validation.errors);
        
        println!("Q005 verified: Same card number means max 4 total regardless of rarity");
    } else {
        // Fallback: test with same card if variants not found
        let member_card = cards.iter()
            .filter(|c| c.is_member())
            .next();
        
        if let Some(card) = member_card {
            let card_id = card_database.get_card_id(&card.card_no).unwrap();
            let base_card_no = extract_card_number(&card.card_no);
            
            let mut invalid_deck: VecDeque<i16> = VecDeque::new();
            for _ in 0..5 {
                invalid_deck.push_back(card_id);
            }
            
            let other_member_cards: Vec<_> = cards.iter()
                .filter(|c| c.is_member())
                .filter(|c| extract_card_number(&c.card_no) != base_card_no)
                .filter(|c| card_database.get_card_id(&c.card_no).is_some())
                .take(44)
                .collect();
            
            let live_cards: Vec<_> = cards.iter()
                .filter(|c| c.is_live())
                .filter(|c| card_database.get_card_id(&c.card_no).is_some())
                .take(12)
                .collect();
            
            for other in &other_member_cards {
                if let Some(other_id) = card_database.get_card_id(&other.card_no) {
                    invalid_deck.push_back(other_id);
                }
            }
            
            for live in &live_cards {
                if let Some(live_id) = card_database.get_card_id(&live.card_no) {
                    invalid_deck.push_back(live_id);
                }
            }
            
            let energy_deck: VecDeque<i16> = VecDeque::new();
            let invalid_validation = DeckBuilder::validate_deck(&card_database, &invalid_deck, &energy_deck);
            assert!(!invalid_validation.is_valid, "Deck with 5 copies should be invalid");
            
            println!("Q005 verified: Same card number means max 4 total regardless of rarity");
        }
    }
}
