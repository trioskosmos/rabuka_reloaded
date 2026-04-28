use rabuka_engine::card_loader::CardLoader;
use std::path::Path;

fn main() {
    println!("Testing sequential ability execution...");
    
    // Load cards from the actual cards directory
    let cards_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards");
    
    match CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            println!("Successfully loaded {} cards", cards.len());
            
            // Find cards with sequential abilities
            let mut sequential_cards = Vec::new();
            
            for card in &cards {
                for ability in &card.abilities {
                    if let Some(ref effect) = ability.effect {
                        if effect.action == "sequential" {
                            sequential_cards.push((card.name.clone(), effect.text.clone(), effect.actions.as_ref().map(|a| a.len()).unwrap_or(0)));
                        }
                    }
                }
            }
            
            println!("Found {} cards with sequential abilities:", sequential_cards.len());
            
            // Show some examples
            for (i, (name, text, action_count)) in sequential_cards.iter().take(10).enumerate() {
                println!("{}. {} ({} actions)", i+1, name, action_count);
                println!("   Text: {}", text);
            }
            
            if sequential_cards.len() > 10 {
                println!("... and {} more", sequential_cards.len() - 10);
            }
        }
        Err(e) => {
            println!("Failed to load cards: {}", e);
        }
    }
}
