// Find specific ability #156 to debug the count issue
use std::path::Path;

fn main() {
    println!("Finding ability #156...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        if let Some(ability) = unique_abilities.get(156) {
                            println!("Found ability #156:");
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
                                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                    println!("  Actions: {}", actions.len());
                                    for (i, sub_action) in actions.iter().enumerate() {
                                        println!("    Action {}:", i);
                                        if let Some(obj) = sub_action.as_object() {
                                            for (key, value) in obj {
                                                println!("      {}: {}", key, value);
                                            }
                                        }
                                        println!("    ---");
                                    }
                                }
                            }
                        } else {
                            println!("Ability #156 not found");
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
