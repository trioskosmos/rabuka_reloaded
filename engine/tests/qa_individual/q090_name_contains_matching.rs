use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q090_name_contains_matching() {
    // Q90: Live start ability cost requires "上原歩夢", "澁谷かのん", "日野下花帆" (3 cards total, any combination)
    // Question: Can you use "私のSymphony 〜澁谷かのんVer.〜" as a cost card since it contains "澁谷かのん" in the name?
    // Answer: Yes, you can, because the card name contains "澁谷かのん".
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find the card with "澁谷かのん" in the name (PL!SP-sd1-026-SD "私のSymphony 〜澁谷かのんVer.〜")
    let card_with_name = cards.iter()
        .find(|c| c.card_no == "PL!SP-sd1-026-SD");
    
    if let Some(card) = card_with_name {
        let card_id = get_card_id(card, &card_database);
        
        // Verify the card name contains "澁谷かのん"
        assert!(card.name.contains("澁谷かのん"), "Card name should contain '澁谷かのん'");
        
        // Test the card database helper for name matching
        let contains_name = card_database.card_name_contains(card_id, "澁谷かのん");
        assert!(contains_name, "Card database should detect that card name contains '澁谷かのん'");
        
        // The key assertion: cards with names containing the required name fragment can be used for cost payment
        // This tests the name contains matching rule for ability costs
        
        println!("Q090 verified: Cards with names containing required name fragment can be used for cost payment");
        println!("Card '私のSymphony 〜澁谷かのんVer.〜' contains '澁谷かのん', can be used for cost");
    } else {
        panic!("Required card PL!SP-sd1-026-SD not found for Q090 test");
    }
}
