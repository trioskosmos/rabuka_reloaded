// Debug ability #523 to understand the nested structure
use std::path::Path;

fn main() {
    println!("Debugging ability #523 nested structure...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        if let Some(ability) = unique_abilities.get(523) {
                            println!("=== ABILITY #523 ===");
                            if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                println!("Full text: {}", full_text);
                            }
                            
                            if let Some(effect) = ability.get("effect") {
                                println!("Effect structure:");
                                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                    println!("  Action: {}", action);
                                }
                                
                                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                    println!("  Actions: {}", actions.len());
                                    for (i, sub_action) in actions.iter().enumerate() {
                                        println!("  Action {}:", i);
                                        if let Some(sub_action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                                            println!("    Type: {}", sub_action_type);
                                            
                                            if sub_action_type == "sequential" {
                                                if let Some(nested_actions) = sub_action.get("actions").and_then(|a| a.as_array()) {
                                                    println!("    Nested actions: {}", nested_actions.len());
                                                    for (j, nested_action) in nested_actions.iter().enumerate() {
                                                        println!("    Nested Action {}:", j);
                                                        if let Some(nested_type) = nested_action.get("action").and_then(|a| a.as_str()) {
                                                            println!("      Type: {}", nested_type);
                                                        }
                                                        if let Some(text) = nested_action.get("text").and_then(|t| t.as_str()) {
                                                            println!("      Text: {}", text);
                                                        }
                                                        if let Some(resource) = nested_action.get("resource") {
                                                            println!("      Resource: {:?}", resource);
                                                        } else if nested_action.get("action").and_then(|a| a.as_str()) == Some("gain_resource") {
                                                            println!("      ⚠️  Missing resource field!");
                                                        }
                                                        
                                                        // Show all fields
                                                        if let Some(obj) = nested_action.as_object() {
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
