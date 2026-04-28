// Verify card_type inference fixes
use std::path::Path;

fn main() {
    println!("Verifying card_type inference fixes...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    // Abilities that previously had missing card_type
    let check_abilities = vec![48, 260, 280, 290, 439, 593];
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut all_fixed = true;
                        
                        for &idx in &check_abilities {
                            if let Some(ability) = abilities.get(idx) {
                                println!("\n=== ABILITY #{} ===", idx);
                                
                                if let Some(effect) = ability.get("effect") {
                                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                        for (action_idx, action) in actions.iter().enumerate() {
                                            if action.get("action").and_then(|a| a.as_str()) == Some("move_cards") {
                                                let text = action.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                                
                                                if let Some(card_type) = action.get("card_type") {
                                                    println!("  ✓ Action {} has card_type: {:?}", action_idx, card_type);
                                                    println!("    Text: {}", text.chars().take(50).collect::<String>());
                                                } else {
                                                    println!("  ✗ Action {} MISSING card_type!", action_idx);
                                                    println!("    Text: {}", text);
                                                    all_fixed = false;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("\n");
                        if all_fixed {
                            println!("✅ ALL CHECKED ABILITIES HAVE card_type!");
                        } else {
                            println!("❌ Some card_type issues remain");
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
