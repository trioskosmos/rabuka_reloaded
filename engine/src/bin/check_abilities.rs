use rabuka_engine::card_loader::CardLoader;
use std::path::Path;

fn main() {
    println!("Checking complex abilities...");
    
    // Use absolute path to cards directory
    let cards_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards");
    let cards = CardLoader::load_cards_from_file(cards_path).expect("Failed to load cards");
    
    println!("Loaded {} cards", cards.len());
    
    // Find complex abilities
    let mut look_and_select_count = 0;
    let mut conditional_alternative_count = 0;
    let mut sequential_count = 0;
    let mut sequential_cost_count = 0;
    let mut max_complexity = 0;
    let mut most_complex_name = String::new();
    let mut most_complex_text = String::new();
    
    for card in &cards {
        for ability in &card.abilities {
            let mut complexity = 0;
            
            if ability.cost.is_some() { complexity += 1; }
            if ability.effect.is_some() { complexity += 1; }
            
            if let Some(ref effect) = ability.effect {
                if effect.look_action.is_some() { complexity += 2; }
                if effect.select_action.is_some() { complexity += 2; }
                if let Some(ref actions) = effect.actions {
                    complexity += actions.len();
                }
                if effect.condition.is_some() { complexity += 2; }
                if effect.primary_effect.is_some() { complexity += 2; }
                if effect.alternative_condition.is_some() { complexity += 2; }
                if effect.alternative_effect.is_some() { complexity += 2; }
            }
            
            if let Some(ref cost) = ability.cost {
                if let Some(ref costs) = cost.costs {
                    complexity += costs.len();
                }
            }
            
            if complexity > max_complexity && complexity > 5 {
                max_complexity = complexity;
                most_complex_name = card.name.clone();
                most_complex_text = ability.full_text.clone();
            }
            
            // Count ability types
            if let Some(ref effect) = ability.effect {
                match effect.action.as_str() {
                    "look_and_select" => {
                        look_and_select_count += 1;
                        if look_and_select_count <= 3 {
                            println!("\n=== look_and_select #{} ===", look_and_select_count);
                            println!("Card: {}", card.name);
                            println!("Ability: {}", ability.full_text);
                            if let Some(ref look) = effect.look_action {
                                println!("Look: {}", look.text);
                            }
                            if let Some(ref select) = effect.select_action {
                                println!("Select action: {}", select.action);
                            }
                        }
                    }
                    "conditional_alternative" => {
                        conditional_alternative_count += 1;
                        if conditional_alternative_count <= 3 {
                            println!("\n=== conditional_alternative #{} ===", conditional_alternative_count);
                            println!("Card: {}", card.name);
                            println!("Ability: {}", ability.full_text);
                        }
                    }
                    "sequential" => {
                        sequential_count += 1;
                        if sequential_count <= 3 {
                            println!("\n=== sequential #{} ===", sequential_count);
                            println!("Card: {}", card.name);
                            println!("Ability: {}", ability.full_text);
                            if let Some(ref actions) = effect.actions {
                                println!("Actions: {}", actions.len());
                            }
                        }
                    }
                    _ => {}
                }
            }
            
            if let Some(ref cost) = ability.cost {
                if cost.cost_type.as_deref() == Some("sequential_cost") {
                    sequential_cost_count += 1;
                    if sequential_cost_count <= 3 {
                        println!("\n=== sequential_cost #{} ===", sequential_cost_count);
                        println!("Card: {}", card.name);
                        println!("Ability: {}", ability.full_text);
                        if let Some(ref costs) = cost.costs {
                            println!("Cost steps: {}", costs.len());
                        }
                    }
                }
            }
        }
    }
    
    println!("\n=== Summary ===");
    println!("look_and_select: {}", look_and_select_count);
    println!("conditional_alternative: {}", conditional_alternative_count);
    println!("sequential: {}", sequential_count);
    println!("sequential_cost: {}", sequential_cost_count);
    println!("Most complex ability (complexity {}): {}", max_complexity, most_complex_name);
    println!("{}", most_complex_text);
}
