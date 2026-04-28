// Debug ability #144 to understand why selected_cards count is not being set
use std::path::Path;

fn main() {
    println!("Debugging ability #144...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        if let Some(ability) = unique_abilities.get(144) {
                            println!("Found ability #144:");
                            if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                println!("  Full text: {}", full_text);
                            }
                            if let Some(effect) = ability.get("effect") {
                                if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                    println!("  Effect text: {}", text);
                                }
                                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                    for (i, sub_action) in actions.iter().enumerate() {
                                        println!("  Action {}:", i);
                                        if let Some(obj) = sub_action.as_object() {
                                            for (key, value) in obj {
                                                println!("    {}: {}", key, value);
                                            }
                                        }
                                        println!("  ---");
                                    }
                                }
                            }
                        } else {
                            println!("Ability #144 not found");
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
