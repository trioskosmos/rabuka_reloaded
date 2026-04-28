// Analyze missing card_type issues in detail
use std::path::Path;
use std::collections::HashMap;

fn main() {
    println!("=== ANALYZING MISSING CARD_TYPE ISSUES ===");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut missing_card_type: Vec<(usize, usize, String, String)> = Vec::new(); // (ability_idx, action_idx, action_text, inferred_type)
                        
                        for (ability_idx, ability) in abilities.iter().enumerate() {
                            if let Some(effect) = ability.get("effect") {
                                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                    for (action_idx, action) in actions.iter().enumerate() {
                                        if let Some(action_type) = action.get("action").and_then(|a| a.as_str()) {
                                            if action_type == "move_cards" {
                                                if action.get("card_type").is_none() {
                                                    // Try to infer from text
                                                    let text = action.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                                    let inferred = infer_card_type(text);
                                                    missing_card_type.push((ability_idx, action_idx, text.to_string(), inferred));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("Found {} move_cards actions missing card_type:", missing_card_type.len());
                        println!();
                        
                        // Group by inferred type
                        let mut by_type: HashMap<String, Vec<(usize, usize, String)>> = HashMap::new();
                        for (ability_idx, action_idx, text, inferred) in &missing_card_type {
                            by_type.entry(inferred.clone()).or_default().push((*ability_idx, *action_idx, text.clone()));
                        }
                        
                        for (inferred_type, items) in &by_type {
                            println!("Inferred type '{}': {} occurrences", inferred_type, items.len());
                            for (ability_idx, action_idx, text) in items.iter().take(3) {
                                println!("  - Ability #{} action {}: {}", ability_idx, action_idx, 
                                    text.chars().take(50).collect::<String>());
                            }
                            if items.len() > 3 {
                                println!("  ... and {} more", items.len() - 3);
                            }
                            println!();
                        }
                        
                        // Show all for fixing
                        println!("\n=== DETAILED LIST FOR FIXING ===");
                        for (ability_idx, action_idx, text, inferred) in &missing_card_type {
                            println!("Ability #{} action {} -> inferred: '{}'", ability_idx, action_idx, inferred);
                            println!("  Text: {}", text);
                            println!();
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

fn infer_card_type(text: &str) -> String {
    if text.contains("メンバーカード") {
        "member_card".to_string()
    } else if text.contains("ライブカード") {
        "live_card".to_string()
    } else if text.contains("エネルギーカード") {
        "energy_card".to_string()
    } else if text.contains("カード") {
        "card".to_string()
    } else {
        "unknown".to_string()
    }
}
