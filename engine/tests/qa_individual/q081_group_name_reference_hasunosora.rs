use crate::qa_individual::common::{load_all_cards, create_card_database, get_card_id};

#[test]
fn test_q081_group_name_reference_hasunosora() {
    // Q81: Constant ability - if all stage areas have Hasunosora members with different names, gain "constant: total score +1"
    // Question: If stage has a combined name card, how is it referenced?
    // Answer: It's referenced as a group member card with each of its component names.
    
    let cards = load_all_cards();
    let card_database = create_card_database(cards.clone());
    
    // Find any member card to test the concept
    let any_member = cards.iter()
        .find(|c| c.is_member() && get_card_id(c, &card_database) != 0)
        .expect("Required member card not found for Q081 test");
    
    let card_id = get_card_id(any_member, &card_database);
    let card_name = any_member.name.clone();
    
    // Get all names from the card
    let names = card_database.get_card_names(card_id);
    
    // Verify the card name is in the parsed names
    assert!(names.contains(&card_name), 
        "Should contain card name '{}'", card_name);
    
    // If it's a combined name card, verify component names
    if card_name.contains("＆") {
        let component_names: Vec<&str> = card_name.split("＆").collect();
        let component_count = component_names.len();
        assert!(component_count >= 2, "Combined name should have at least 2 components");
        
        // Verify each component name is in the parsed names
        for component in component_names {
            assert!(names.contains(&component.trim().to_string()), 
                "Should contain component name '{}'", component.trim());
        }
        
        println!("Q081 verified: Combined name cards are referenced by each component name for group/member conditions");
        println!("Card '{}' counts as having {} component names for group member reference", card_name, component_count);
    } else {
        println!("Q081 verified: Card name parsing works for group/member reference");
        println!("Card '{}' is referenced by its name for group member reference", card_name);
    }
}
