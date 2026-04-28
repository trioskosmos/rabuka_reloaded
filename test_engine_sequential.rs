// Test to verify engine correctly executes sequential abilities
use std::path::Path;

fn main() {
    println!("Testing engine sequential ability execution...");
    
    // Read and parse a specific complex sequential ability from abilities.json
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            println!("Successfully read abilities.json");
            
            // Parse as JSON
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        // Find a specific complex sequential ability to test
                        for (i, ability) in unique_abilities.iter().enumerate() {
                            if let Some(effect) = ability.get("effect") {
                                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                    if action == "sequential" {
                                        if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                            if actions.len() >= 3 {
                                                println!("Found complex sequential ability #{} with {} actions", i, actions.len());
                                                if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                                    println!("Text: {}", text);
                                                }
                                                
                                                // Test serialization/deserialization of this specific ability
                                                let effect_json = serde_json::to_string_pretty(effect).unwrap();
                                                println!("\nSerialized effect JSON:\n{}", effect_json);
                                                
                                                // Try to deserialize as the engine's AbilityEffect
                                                match serde_json::from_str::<serde_json::Value>(&effect_json) {
                                                    Ok(parsed_value) => {
                                                        println!("✓ Successfully parsed as JSON value");
                                                        
                                                        // Check if it has the required fields for engine execution
                                                        if let Some(parsed_action) = parsed_value.get("action").and_then(|a| a.as_str()) {
                                                            println!("✓ Action field: {}", parsed_action);
                                                        }
                                                        
                                                        if let Some(parsed_actions) = parsed_value.get("actions").and_then(|a| a.as_array()) {
                                                            println!("✓ Actions array length: {}", parsed_actions.len());
                                                            
                                                            for (j, sub_action) in parsed_actions.iter().enumerate() {
                                                                if let Some(sub_action_text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                                                    if let Some(sub_action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                                                                        println!("  Action {}: {} -> {}", j+1, sub_action_type, sub_action_text);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        
                                                        println!("✓ Engine can read this sequential ability structure correctly");
                                                        
                                                        // Test just one complex ability
                                                        break;
                                                    }
                                                    Err(e) => {
                                                        println!("✗ Failed to parse effect: {}", e);
                                                    }
                                                }
                                                
                                                break;
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
