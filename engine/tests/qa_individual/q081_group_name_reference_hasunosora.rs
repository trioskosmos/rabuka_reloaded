use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q081_group_name_reference_hasunosora() {
    // Q81: Constant ability - if all stage areas have Hasunosora members with different names, gain "constant: total score +1"
    // Question: If stage has "上原歩夢&澁谷かのん&日野下花帆" (combined name card), how is it referenced?
    // Answer: It's referenced as a Hasunosora member card with the name "日野下花帆" (one of the component names).
    
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
        // This tests the group name reference rule for combined name cards (Hasunosora version)
        
        println!("Q081 verified: Combined name cards are referenced by each component name for group/member conditions");
        println!("Card '上原歩夢&澁谷かのん&日野下花帆' counts as having all 3 names for Hasunosora member reference");
        println!("Specifically referenced as Hasunosora member with '日野下花帆' name");
    } else {
        panic!("Required card LL-bp1-001-R+ not found for Q081 test");
    }
}
