// Examine the specific subtle issues found by deep analysis
use std::path::Path;

fn main() {
    println!("Examining subtle parser issues found by deep analysis...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        // Examine abilities with missing effect text
                        let missing_text_abilities = vec![5, 126];
                        
                        for &ability_index in &missing_text_abilities {
                            if let Some(ability) = unique_abilities.get(ability_index) {
                                println!("\n=== ABILITY #{} (MISSING EFFECT TEXT) ===", ability_index);
                                if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                    println!("Full text: {}", full_text);
                                }
                                if let Some(effect) = ability.get("effect") {
                                    println!("Effect structure:");
                                    if let Some(obj) = effect.as_object() {
                                        for (key, value) in obj {
                                            println!("  {}: {}", key, value);
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Examine a few abilities with potential missing conditions
                        let condition_abilities = vec![57, 58, 144, 523];
                        
                        println!("\n=== ABILITIES WITH POTENTIAL MISSING CONDITIONS ===");
                        for &ability_index in &condition_abilities {
                            if let Some(ability) = unique_abilities.get(ability_index) {
                                println!("\n--- Ability #{} ---", ability_index);
                                if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                    println!("Full text: {}", full_text);
                                }
                                if let Some(effect) = ability.get("effect") {
                                    if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                        println!("Effect text: {}", text);
                                    }
                                    if let Some(condition) = effect.get("condition") {
                                        println!("Has condition: {}", condition);
                                    } else {
                                        println!("⚠️  No explicit condition field");
                                    }
                                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                        println!("Actions: {}", actions.len());
                                        for (i, action) in actions.iter().enumerate() {
                                            if let Some(action_type) = action.get("action").and_then(|a| a.as_str()) {
                                                println!("  Action {}: {}", i, action_type);
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
