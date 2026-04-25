use rabuka_engine::deck_builder::DeckBuilder;
use crate::qa_individual::common::{load_all_cards, create_card_database};
use std::collections::VecDeque;

#[test]
fn test_q006_different_card_numbers() {
    // Q6: Can cards with same name/ability but different card numbers be 4 each in main deck?
    // Answer: Yes, if card numbers differ, can use 4 of each.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find two different member cards with different card numbers
    let member_cards: Vec<_> = cards.iter()
        .filter(|c| c.is_member())
        .filter(|c| card_database.get_card_id(&c.card_no).is_some())
        .take(2)
        .collect();
    
    if member_cards.len() >= 2 {
        let card1_id = card_database.get_card_id(&member_cards[0].card_no).unwrap();
        let card2_id = card_database.get_card_id(&member_cards[1].card_no).unwrap();
        
        // Test valid deck: 4 copies of card1 + 4 copies of card2 (different card numbers)
        let mut valid_deck: VecDeque<i16> = VecDeque::new();
        for _ in 0..4 {
            valid_deck.push_back(card1_id);
        }
        for _ in 0..4 {
            valid_deck.push_back(card2_id);
        }
        
        // Fill with other cards to reach 60
        let other_cards: Vec<_> = cards.iter()
            .filter(|c| c.is_member() && c.card_no != member_cards[0].card_no && c.card_no != member_cards[1].card_no)
            .filter(|c| card_database.get_card_id(&c.card_no).is_some())
            .take(52)
            .collect();
        
        for other in &other_cards {
            if let Some(other_id) = card_database.get_card_id(&other.card_no) {
                valid_deck.push_back(other_id);
            }
        }
        
        let energy_deck: VecDeque<i16> = VecDeque::new();
        let validation = DeckBuilder::validate_deck(&card_database, &valid_deck, &energy_deck);
        assert!(validation.is_valid, "Deck with 4 of each different card number should be valid: {:?}", validation.errors);
        
        println!("Q006 verified: Cards with different card numbers can be 4 of each in main deck");
    } else {
        panic!("Required member cards not found for Q006 test");
    }
}
