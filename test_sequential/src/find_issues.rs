// Find specific parsing issues in sequential abilities
use std::path::Path;

fn main() {
    println!("Finding specific parsing issues in sequential abilities...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut issue_count = 0;
                        
                        for (i, ability) in unique_abilities.iter().enumerate() {
                            if let Some(effect) = ability.get("effect") {
                                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                    if action == "sequential" {
                                        if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                            for (j, sub_action) in actions.iter().enumerate() {
                                                let sub_action_type = sub_action.get("action").and_then(|a| a.as_str());
                                                
                                                if let Some(action_type) = sub_action_type {
                                                    if action_type == "move_cards" {
                                                        let has_count = sub_action.get("count").is_some();
                                                        let has_source = sub_action.get("source").is_some();
                                                        let has_destination = sub_action.get("destination").is_some();
                                                        
                                                        if !has_count || !has_source || !has_destination {
                                                            println!("Issue found in ability #{}, action #{}:", i, j);
                                                            if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                                                println!("  Effect text: {}", text);
                                                            }
                                                            println!("  Action type: {}", action_type);
                                                            println!("  Has count: {}", has_count);
                                                            println!("  Has source: {}", has_source);
                                                            println!("  Has destination: {}", has_destination);
                                                            
                                                            if let Some(action_text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                                                println!("  Action text: {}", action_text);
                                                            }
                                                            
                                                            // Show all fields for debugging
                                                            println!("  All fields:");
                                                            if let Some(obj) = sub_action.as_object() {
                                                                for (key, value) in obj {
                                                                    println!("    {}: {}", key, value);
                                                                }
                                                            }
                                                            
                                                            println!("  ---");
                                                            issue_count += 1;
                                                            
                                                            if issue_count >= 10 {
                                                                println!("... showing first 10 issues");
                                                                return;
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
                        
                        if issue_count == 0 {
                            println!("No issues found with move_cards actions");
                        } else {
                            println!("Found {} total issues", issue_count);
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
