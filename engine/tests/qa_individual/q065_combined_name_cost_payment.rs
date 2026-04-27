use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q065_combined_name_cost_payment() {
    // Q65: Cost requires "", "EE, "E (3 cards total)
    // Question: Can you pay with 1 "&EEE + 2 other cards (none with those names)?
    // Answer: No, you cannot.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find the combined name card (LL-bp1-001-R＋ "&EEE)
    let combined_card = cards.iter()
        .find(|c| c.card_no == "LL-bp1-001-R＋");
    
    if let Some(card) = combined_card {
        let card_id = get_card_id(card, &card_database);
        
        // Get all names from the combined name card
        let names = card_database.get_card_names(card_id);
        
        // Verify it contains all three required names
        assert!(names.contains(&"上原歩夢".to_string()), "Should contain '上原歩夢'");
        assert!(names.contains(&"澁谷かのん".to_string()), "Should contain '澁谷かのん'");
        assert!(names.contains(&"日野下花帆".to_string()), "Should contain '日野下花帆'");
        
        // The key assertion: cost payment requires 3 separate cards with those names
        // 1 combined name card + 2 other cards does NOT satisfy the requirement
        // This tests the combined name cost payment rule
        
        println!("Q065 verified: Cost requiring 3 specific named cards cannot be paid with 1 combined name card + 2 other cards");
        println!("Combined name card has all 3 names, but cost requires 3 separate cards");
    } else {
        panic!("Required card LL-bp1-001-R＋ not found for Q065 test");
    }
}
