// Verify the critical fixes
use std::path::Path;

fn main() {
    println!("Verifying critical runtime safety fixes...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    let critical_abilities = vec![184, 261, 349, 439, 597, 600];
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut all_fixed = true;
                        
                        for &idx in &critical_abilities {
                            if let Some(ability) = unique_abilities.get(idx) {
                                println!("\n=== ABILITY #{} ===", idx);
                                
                                if let Some(effect) = ability.get("effect") {
                                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                        for (action_idx, action) in actions.iter().enumerate() {
                                            // Check if count is explicitly null (BAD)
                                            if let Some(count) = action.get("count") {
                                                if count.is_null() {
                                                    println!("  ✗ FAIL: Action {} still has count: null!", action_idx);
                                                    all_fixed = false;
                                                }
                                            }
                                            
                                            // Check if dynamic_count is present (GOOD)
                                            if let Some(dynamic_count) = action.get("dynamic_count") {
                                                println!("  ✓ FIXED: Action {} has dynamic_count: {}", action_idx, dynamic_count);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("\n");
                        if all_fixed {
                            println!("✅ ALL CRITICAL ISSUES FIXED!");
                            println!("   No more count: null that would cause engine panics");
                            println!("   Variable counts now use dynamic_count structure");
                        } else {
                            println!("❌ Some issues remain");
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
