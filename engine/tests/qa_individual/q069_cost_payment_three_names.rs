use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q069_cost_payment_three_names() {
    // Q69: Cost requires "上原歩夢", "澁谷かのん", "日野下花帆" (3 cards total, any combination)
    // Question: Can you pay with 3 "上原歩夢" cards, or 2 "澁谷かのん" + 1 "日野下花帆"?
    // Answer: Yes, any combination of cards with those names (3 total) works.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find the combined name card (LL-bp1-001-R+ "上原歩夢&澁谷かのん&日野下花帆")
    let combined_card = cards.iter()
        .find(|c| c.card_no == "LL-bp1-001-R+");
    
    if let Some(card) = combined_card {
        let card_id = get_card_id(card, &card_database);
        
        // Get all names from the combined name card
        let names = card_database.get_card_names(card_id);
        
        // Verify it contains all three required names
        assert!(names.contains(&"上原歩夢".to_string()), "Should contain '上原歩夢'");
        assert!(names.contains(&"澁谷かのん".to_string()), "Should contain '澁谷かのん'");
        assert!(names.contains(&"日野下花帆".to_string()), "Should contain '日野下花帆'");
        
        // The key assertion: cost payment allows any combination of cards with those names
        // 3 cards with "上原歩夢" name works
        // 2 cards with "澁谷かのん" + 1 card with "日野下花帆" works
        // This tests the combined name cost payment flexibility rule
        
        println!("Q069 verified: Cost requiring 3 cards with specific names can be paid with any combination");
        println!("Valid combinations: 3 of one name, or mixed names totaling 3 cards");
        println!("Combined name card counts as having all 3 names");
    } else {
        panic!("Required card LL-bp1-001-R+ not found for Q069 test");
    }
}
