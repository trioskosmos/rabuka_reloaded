use rabuka_engine::card_loader::CardLoader;
use std::path::Path;

#[test]
fn test_load_cards_with_abilities() {
    let cards_path = Path::new("../cards/cards.json");
    let cards = CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards");

    println!("Loaded {} cards", cards.len());

    // Find a card that should have an ability
    let cards_with_abilities: Vec<_> = cards.iter().filter(|c| !c.abilities.is_empty()).collect();
    println!("Cards with abilities: {}", cards_with_abilities.len());

    // Print first few cards with abilities
    for (i, card) in cards_with_abilities.iter().take(5).enumerate() {
        println!("{}: {} ({}) has {} abilities", i, card.name, card.card_no, card.abilities.len());
        for (j, ability) in card.abilities.iter().enumerate() {
            println!("  Ability {}: {}", j, ability.full_text);
        }
    }

    // Verify at least some cards have abilities
    assert!(cards_with_abilities.len() > 0, "Expected some cards to have abilities");
}
