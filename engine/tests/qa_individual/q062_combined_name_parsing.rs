use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q062_combined_name_parsing() {
    // Q62: Do cards with names like "A&B" have both names "A" and "B"?
    // Answer: Yes, they have each name.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find LL-bp1-001-R+ "上原歩夢&澁谷かのん&日野下花帆"
    let combined_card = cards.iter()
        .find(|c| c.card_no == "LL-bp1-001-R+");
    
    if let Some(card) = combined_card {
        let card_id = get_card_id(card, &card_database);
        
        // Get all names from the combined name card
        let names = card_database.get_card_names(card_id);
        
        // Verify it contains all three names
        assert!(names.contains(&"上原歩夢".to_string()), "Should contain '上原歩夢'");
        assert!(names.contains(&"澁谷かのん".to_string()), "Should contain '澁谷かのん'");
        assert!(names.contains(&"日野下花帆".to_string()), "Should contain '日野下花帆'");
        assert_eq!(names.len(), 3, "Should have exactly 3 names");
        
        println!("Q062 verified: Combined name card '上原歩夢&澁谷かのん&日野下花帆' correctly parsed as 3 separate names");
    } else {
        panic!("Required card LL-bp1-001-R+ not found for Q062 test");
    }
}
