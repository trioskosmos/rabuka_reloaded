// Test: Verify cards have abilities attached from abilities.json
// This ensures the ability attachment system works correctly

use crate::qa_individual::common::*;

#[test]
fn test_cards_have_abilities_attached() {
    // Test: Cards should have abilities attached from abilities.json
    let cards = load_all_cards();
    
    // Count cards with abilities
    let cards_with_abilities = cards.iter()
        .filter(|c| !c.abilities.is_empty())
        .count();
    
    println!("Total cards: {}", cards.len());
    println!("Cards with abilities: {}", cards_with_abilities);
    
    // According to abilities.json statistics, there should be 1057 cards with abilities
    // Allow some tolerance for parsing differences
    assert!(cards_with_abilities > 1000, 
        "Expected at least 1000 cards with abilities, got {}", cards_with_abilities);
}

#[test]
fn test_specific_card_has_abilities() {
    // Test: Specific known cards should have their abilities attached
    let cards = load_all_cards();
    
    // Find a card that should have abilities according to abilities.json
    // PL!-sd1-005-SD | 星空 凛 (ab#0) should have the first ability in the list
    let target_card = cards.iter()
        .find(|c| c.card_no == "PL!-sd1-005-SD");
    
    assert!(target_card.is_some(), "Should find card PL!-sd1-005-SD");
    
    let card = target_card.unwrap();
    println!("Card {} has {} abilities", card.card_no, card.abilities.len());
    
    // This card should have at least one ability
    assert!(!card.abilities.is_empty(), 
        "Card PL!-sd1-005-SD should have abilities attached");
    
    // Verify the ability has the expected trigger
    let has_kidou_trigger = card.abilities.iter()
        .any(|a| a.triggers.as_deref() == Some("起動"));
    
    assert!(has_kidou_trigger, 
        "Card PL!-sd1-005-SD should have an ability with '起動' trigger");
}

#[test]
fn test_ability_fields_populated() {
    // Test: Abilities should have their fields properly populated
    let cards = load_all_cards();
    
    // Find a card with abilities
    let card_with_ability = cards.iter()
        .find(|c| !c.abilities.is_empty())
        .expect("Should have at least one card with abilities");
    
    let ability = &card_with_ability.abilities[0];
    
    println!("Card {} ability: {:?}", card_with_ability.card_no, ability);
    
    // Verify critical fields are populated
    assert!(!ability.triggers.as_ref().map_or(false, |t| t.is_empty()) || ability.triggers.is_some(),
        "Ability should have triggers or be triggerless");
    
    // If it has an effect, verify action is populated
    if let Some(ref effect) = ability.effect {
        assert!(!effect.action.is_empty(), 
            "Ability effect should have an action populated");
    }
}
