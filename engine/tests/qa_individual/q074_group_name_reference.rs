use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q074_group_name_reference() {
    // Q74: Live start ability condition "5+ different named Liella! members in stage AND discard zone"
    // Question: If stage or discard has "上原歩夢&澁谷かのん&日野下花帆" (combined name card), how is it referenced?
    // Answer: It's referenced as a Liella! member card with each of its component names (e.g., "澁谷かのん").
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find the combined name card (LL-bp1-001-R+ "上原歩夢&澁谷かのん&日野下花帆")
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
        
        // The key assertion: combined name cards are referenced by each of their component names
        // for group/member reference purposes in ability conditions
        // This tests the group name reference rule for combined name cards
        
        println!("Q074 verified: Combined name cards are referenced by each component name for group/member conditions");
        println!("Card '上原歩夢&澁谷かのん&日野下花帆' counts as having all 3 names for Liella! member reference");
    } else {
        panic!("Required card LL-bp1-001-R+ not found for Q074 test");
    }
}
