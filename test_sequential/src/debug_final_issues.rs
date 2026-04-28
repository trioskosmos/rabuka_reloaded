// Debug the final 3 remaining parsing issues
use std::path::Path;

fn main() {
    println!("Debugging the final 3 parsing issues...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let remaining_issues = vec![449, 575, 593];
                        
                        for &ability_index in &remaining_issues {
                            if let Some(ability) = unique_abilities.get(ability_index) {
                                println!("\n=== Ability #{} ===", ability_index);
                                if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                    println!("Full text: {}", full_text);
                                }
                                if let Some(effect) = ability.get("effect") {
                                    if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                        println!("Effect text: {}", text);
                                    }
                                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                        for (i, sub_action) in actions.iter().enumerate() {
                                            if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                                                let has_source = sub_action.get("source").is_some();
                                                let has_destination = sub_action.get("destination").is_some();
                                                let has_count = sub_action.get("count").is_some();
                                                
                                                if !has_source || !has_destination || !has_count {
                                                    println!("  Action {}: {} - source: {}, destination: {}, count: {}", 
                                                        i, action_type, has_source, has_destination, has_count);
                                                    if let Some(action_text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                                        println!("    Text: {}", action_text);
                                                    }
                                                    println!("    All fields:");
                                                    if let Some(obj) = sub_action.as_object() {
                                                        for (key, value) in obj {
                                                            println!("      {}: {}", key, value);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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
