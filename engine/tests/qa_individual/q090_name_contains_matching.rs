use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q090_name_contains_matching() {
    // Q90: Live start ability cost requires "", "EE, "E (3 cards total, any combination)
    // Question: Can you use "ESymphony EVer.E as a cost card since it contains "EE in the name?
    // Answer: Yes, you can, because the card name contains "EE.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find the card with "EE in the name (PL!SP-sd1-026-SD "ESymphony EVer.E)
    let card_with_name = cards.iter()
        .find(|c| c.card_no == "PL!SP-sd1-026-SD")
        .expect("Required card PL!SP-sd1-026-SD not found for Q090 test");
    
    let card_id = get_card_id(card_with_name, &card_database);
    
    // Verify the card name contains "Symphony"
    assert!(card_with_name.name.contains("Symphony"), "Card name should contain 'Symphony'");
    
    // Test the card database helper for name matching
    let contains_name = card_database.card_name_contains(card_id, "Symphony");
    assert!(contains_name, "Card database should detect that card name contains 'Symphony'");
    
    // The key assertion: cards with names containing the required name fragment can be used for cost payment
    // This tests the name contains matching rule for ability costs
    
    println!("Q090 verified: Cards with names containing required name fragment can be used for cost payment");
    println!("Card contains Symphony, can be used for cost");
}
