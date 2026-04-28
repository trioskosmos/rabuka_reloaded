use std::path::Path;

// Simple test to check sequential ability parsing
fn main() {
    println!("Testing sequential ability parsing...");
    
    // Try to read abilities.json directly
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            println!("Successfully read abilities.json ({} bytes)", content.len());
            
            // Parse as JSON
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    println!("Successfully parsed JSON");
                    
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        println!("Found {} unique abilities", unique_abilities.len());
                        
                        let mut sequential_count = 0;
                        let mut complex_sequential_count = 0;
                        
                        for (i, ability) in unique_abilities.iter().enumerate() {
                            if let Some(effect) = ability.get("effect") {
                                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                    if action == "sequential" {
                                        sequential_count += 1;
                                        
                                        if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                            if actions.len() > 2 {
                                                complex_sequential_count += 1;
                                                println!("Complex sequential ability #{}: {} actions", i, actions.len());
                                                if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                                    println!("  Text: {}", text);
                                                }
                                                if complex_sequential_count <= 5 { // Show first 5 examples
                                                    for (j, sub_action) in actions.iter().enumerate() {
                                                        if let Some(sub_action_text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                                            println!("    Action {}: {}", j + 1, sub_action_text);
                                                        }
                                                    }
                                                    println!("  ---");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("Summary:");
                        println!("  Total sequential abilities: {}", sequential_count);
                        println!("  Complex sequential (>2 actions): {}", complex_sequential_count);
                    }
                }
                Err(e) => {
                    println!("Failed to parse JSON: {}", e);
                }
            }
        }
        Err(e) => {
            println!("Failed to read abilities.json: {}", e);
        }
    }
}
