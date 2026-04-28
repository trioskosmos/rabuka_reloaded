// Debug the critical issues found in deep dive
use std::path::Path;

fn main() {
    println!("🔍 DEBUGGING CRITICAL ISSUES");
    println!("==============================");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    let critical_abilities = vec![184, 261, 349, 439, 597, 600];
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        for &idx in &critical_abilities {
                            if let Some(ability) = unique_abilities.get(idx) {
                                println!("\n=== ABILITY #{} ===", idx);
                                
                                if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                    println!("Full text: {}", full_text);
                                }
                                
                                if let Some(effect) = ability.get("effect") {
                                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                        for (action_idx, action) in actions.iter().enumerate() {
                                            if let Some(count) = action.get("count") {
                                                if count.is_null() {
                                                    println!("\n  Action {}: count is NULL!", action_idx);
                                                    println!("  Action type: {:?}", action.get("action"));
                                                    println!("  Action text: {:?}", action.get("text"));
                                                    println!("  Full action: {}", serde_json::to_string_pretty(action).unwrap());
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
