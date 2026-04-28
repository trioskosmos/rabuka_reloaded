// Find conditional alternative ability #87 to debug the missing primary/alternative effects
use std::path::Path;

fn main() {
    println!("Finding conditional alternative ability #87...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        if let Some(ability) = unique_abilities.get(87) {
                            println!("Found ability #87:");
                            if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                println!("  Full text: {}", full_text);
                            }
                            if let Some(effect) = ability.get("effect") {
                                if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                    println!("  Effect text: {}", text);
                                }
                                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                    println!("  Action: {}", action);
                                }
                                
                                // Show all fields
                                println!("  All effect fields:");
                                if let Some(obj) = effect.as_object() {
                                    for (key, value) in obj {
                                        println!("    {}: {}", key, value);
                                    }
                                }
                            }
                        } else {
                            println!("Ability #87 not found");
                        }
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
