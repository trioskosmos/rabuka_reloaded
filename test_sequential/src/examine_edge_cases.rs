// Examine specific edge case abilities in detail
use std::path::Path;

fn main() {
    println!("=== EXAMINING EDGE CASE ABILITIES ===");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    // Abilities that might have edge cases
    let edge_case_abilities = vec![
        5,    // Rule ability
        126,  // Rule ability
        144,  // Sequential with cost
        184,  // Variable count (dynamic_count)
        261,  // Variable count (dynamic_count)
        360,  // Choice ability (Emma Punch)
        449,  // Sequential with condition
        523,  // Multi-step sequential
        593,  // Sequential with optional
        600,  // Variable count (dynamic_count)
    ];
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        for &idx in &edge_case_abilities {
                            if let Some(ability) = abilities.get(idx) {
                                println!("\n========================================");
                                println!("ABILITY #{} - DETAILED EXAMINATION", idx);
                                println!("========================================");
                                
                                // Full text
                                if let Some(text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                    println!("Full text: {}", text);
                                }
                                
                                // Effect structure
                                if let Some(effect) = ability.get("effect") {
                                    println!("\nEffect structure:");
                                    
                                    if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                        println!("  Main action: {}", action);
                                    }
                                    
                                    if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                        println!("  Effect text: {}", text);
                                    }
                                    
                                    if let Some(conditional) = effect.get("conditional").and_then(|c| c.as_bool()) {
                                        println!("  Conditional: {}", conditional);
                                    }
                                    
                                    // Check for dynamic_count
                                    if let Some(dynamic_count) = effect.get("dynamic_count") {
                                        println!("  ⚠️  Has dynamic_count at effect level: {:?}", dynamic_count);
                                    }
                                    
                                    // Actions
                                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                        println!("\n  Actions ({} total):", actions.len());
                                        for (i, action) in actions.iter().enumerate() {
                                            println!("\n  Action {}:", i);
                                            examine_action(action, i);
                                        }
                                    }
                                }
                                
                                // Check cost
                                if let Some(cost) = ability.get("cost") {
                                    if !cost.is_null() {
                                        println!("\n  Has cost structure");
                                        if let Some(cost_text) = cost.get("text").and_then(|t| t.as_str()) {
                                            println!("    Cost text: {}", cost_text);
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

fn examine_action(action: &serde_json::Value, idx: usize) {
    // Action type
    if let Some(action_type) = action.get("action").and_then(|a| a.as_str()) {
        println!("    Type: {}", action_type);
    }
    
    // Text
    if let Some(text) = action.get("text").and_then(|t| t.as_str()) {
        println!("    Text: {}", text.chars().take(60).collect::<String>());
        if text.len() > 60 {
            println!("          ... ({} chars)", text.len());
        }
    }
    
    // Count fields
    if let Some(count) = action.get("count") {
        if count.is_null() {
            println!("    ⚠️  count: null (should use dynamic_count)");
        } else if let Some(n) = count.as_u64() {
            println!("    count: {}", n);
        } else if let Some(s) = count.as_str() {
            println!("    ⚠️  count is string: '{}'", s);
        }
    }
    
    // Dynamic count
    if let Some(dynamic_count) = action.get("dynamic_count") {
        println!("    ✅ dynamic_count: {:?}", dynamic_count);
    }
    
    // Source/destination
    if let Some(source) = action.get("source").and_then(|s| s.as_str()) {
        println!("    source: {}", source);
    }
    if let Some(dest) = action.get("destination").and_then(|d| d.as_str()) {
        println!("    destination: {}", dest);
    }
    
    // Card type
    if let Some(card_type) = action.get("card_type").and_then(|c| c.as_str()) {
        println!("    card_type: {}", card_type);
    }
    
    // Resource
    if let Some(resource) = action.get("resource").and_then(|r| r.as_str()) {
        println!("    resource: {}", resource);
    }
    
    // Optional
    if let Some(optional) = action.get("optional").and_then(|o| o.as_bool()) {
        if optional {
            println!("    optional: true");
        }
    }
    
    // Nested actions for sequential
    if let Some(nested) = action.get("actions").and_then(|a| a.as_array()) {
        println!("    ⚠️  Has {} nested actions", nested.len());
        for (j, nested_action) in nested.iter().enumerate() {
            if let Some(nested_type) = nested_action.get("action").and_then(|a| a.as_str()) {
                println!("      Nested {}: {}", j, nested_type);
            }
        }
    }
}
