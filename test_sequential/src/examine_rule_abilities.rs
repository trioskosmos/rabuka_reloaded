// Examine the specific rule abilities #5 and #126
use std::path::Path;

fn main() {
    println!("Examining rule abilities #5 and #126...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let rule_abilities = vec![5, 126];
                        
                        for &ability_index in &rule_abilities {
                            if let Some(ability) = unique_abilities.get(ability_index) {
                                println!("\n=== RULE ABILITY #{} ===", ability_index);
                                
                                // Check card information
                                if let Some(card_name) = ability.get("name").and_then(|n| n.as_str()) {
                                    println!("Card name: {}", card_name);
                                }
                                if let Some(card_type) = ability.get("type").and_then(|t| t.as_str()) {
                                    println!("Card type: {}", card_type);
                                }
                                
                                if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                    println!("Full text: {}", full_text);
                                    
                                    // Check if this is a rule ability (starts with parentheses)
                                    if full_text.starts_with('(') && full_text.ends_with(')') {
                                        println!("✓ This is a rule ability (enclosed in parentheses)");
                                        let rule_text = &full_text[1..full_text.len()-1]; // Remove parentheses
                                        println!("Rule text: {}", rule_text);
                                    }
                                }
                                
                                if let Some(effect) = ability.get("effect") {
                                    println!("Current effect structure:");
                                    if let Some(obj) = effect.as_object() {
                                        if obj.is_empty() {
                                            println!("  ⚠️  Empty effect object");
                                        } else {
                                            for (key, value) in obj {
                                                println!("  {}: {}", key, value);
                                            }
                                        }
                                    } else {
                                        println!("  ⚠️  Effect is not an object: {:?}", effect);
                                    }
                                } else {
                                    println!("  ⚠️  No effect field found");
                                }
                                
                                // Check if this ability has any actions
                                if let Some(parsed) = ability.get("parsed") {
                                    if let Some(parsed_effect) = parsed.get("effect") {
                                        println!("Parsed effect: {:?}", parsed_effect);
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
