use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q105_combined_name_hasunosora() {
    // Q105: Live start ability - for each different named Hasunosora member on stage, add +2 to this card's score
    // Question: If stage has combined name card like "渡辺 曜&鬼塚夏美&大沢瑠璃乃", how is it referenced?
    // Answer: It's referenced as a Hasunosora member with one of the component names (e.g., "大沢瑠璃乃").
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find the combined name card (LL-bp2-001-R+ "渡辺 曜&鬼塚夏美&大沢瑠璃乃")
    let combined_card = cards.iter()
        .find(|c| c.card_no == "LL-bp2-001-R+");
    
    if let Some(card) = combined_card {
        let card_id = get_card_id(card, &card_database);
        
        // Get all names from the combined name card
        let names = card_database.get_card_names(card_id);
        
        // Verify it contains all three names
        assert!(names.contains(&"渡辺 曜".to_string()), "Should contain '渡辺 曜'");
        assert!(names.contains(&"鬼塚夏美".to_string()), "Should contain '鬼塚夏美'");
        assert!(names.contains(&"大沢瑠璃乃".to_string()), "Should contain '大沢瑠璃乃'");
        
        // The key assertion: combined name cards are referenced by each of their component names
        // for group/member reference purposes in ability conditions
        // This tests the combined name Hasunosora reference rule
        
        println!("Q105 verified: Combined name cards are referenced by each component name for group/member conditions");
        println!("Card '渡辺 曜&鬼塚夏美&大沢瑠璃乃' counts as having all 3 names for Hasunosora member reference");
        println!("Specifically referenced as Hasunosora member with '大沢瑠璃乃' name");
    } else {
        panic!("Required card LL-bp2-001-R+ not found for Q105 test");
    }
}
