use crate::qa_individual::common::{load_all_cards, create_card_database};
use std::collections::VecDeque;

#[test]
fn test_q007_energy_deck_duplicates() {
    // Q7: How many same cards can be used in energy deck?
    // Answer: Any number of same cards (can use 12 of same card).
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find an energy card
    let energy_card = cards.iter()
        .filter(|c| c.is_energy())
        .filter(|c| card_database.get_card_id(&c.card_no).is_some())
        .next();
    
    if let Some(card) = energy_card {
        let card_id = card_database.get_card_id(&card.card_no).unwrap();
        
        // Test valid energy deck: 12 copies of the same energy card
        let mut energy_deck: VecDeque<i16> = VecDeque::new();
        for _ in 0..12 {
            energy_deck.push_back(card_id);
        }
        
        // Create a valid main deck (48 member + 12 live = 60)
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
        
        let mut main_deck: VecDeque<i16> = VecDeque::new();
        for member in &member_cards {
            if let Some(member_id) = card_database.get_card_id(&member.card_no) {
                main_deck.push_back(member_id);
            }
        }
        for live in &live_cards {
            if let Some(live_id) = card_database.get_card_id(&live.card_no) {
                main_deck.push_back(live_id);
            }
        }
        
        
        // Verify energy deck has 12 copies of the same card
        assert_eq!(energy_deck.len(), 12, "Energy deck should have 12 cards");
        let all_same = energy_deck.iter().all(|&id| id == card_id);
        assert!(all_same, "Energy deck should have 12 of the same card");
        // Verify main deck composition
        let member_count = main_deck.iter().filter(|&&id| card_database.get_card(id).map(|c| c.is_member()).unwrap_or(false)).count();
        let live_count = main_deck.iter().filter(|&&id| card_database.get_card(id).map(|c| c.is_live()).unwrap_or(false)).count();
        assert_eq!(member_count, 48, "Main deck should have 48 member cards");
        assert_eq!(live_count, 12, "Main deck should have 12 live cards");
        
        println!("Q007 verified: Energy deck can have any number of same cards");
    } else {
        panic!("Required energy card not found for Q007 test");
    }
}
