// Examine specific complex abilities to identify potential parser improvements
use std::path::Path;

fn main() {
    println!("Examining complex abilities for potential parser improvements...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        // Focus on abilities with multiple conditionals and complex patterns
                        let complex_abilities = vec![35, 63, 258, 360, 367, 523];
                        
                        for &ability_index in &complex_abilities {
                            if let Some(ability) = unique_abilities.get(ability_index) {
                                println!("\n=== COMPLEX ABILITY #{} ===", ability_index);
                                if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                    println!("Full text: {}", full_text);
                                    println!("Length: {} chars", full_text.len());
                                }
                                if let Some(effect) = ability.get("effect") {
                                    if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                        println!("Effect text: {}", text);
                                    }
                                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                        println!("Structure: {} actions", actions.len());
                                        for (i, sub_action) in actions.iter().enumerate() {
                                            if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                                                println!("  Action {}: {}", i, action_type);
                                                
                                                // Check for missing fields in complex abilities
                                                if action_type == "move_cards" {
                                                    let has_source = sub_action.get("source").is_some();
                                                    let has_destination = sub_action.get("destination").is_some();
                                                    let has_count = sub_action.get("count").is_some();
                                                    
                                                    if !has_source || !has_destination || !has_count {
                                                        println!("    ⚠️  Missing fields - source: {}, destination: {}, count: {}", 
                                                            has_source, has_destination, has_count);
                                                    }
                                                }
                                                
                                                // Show all fields for debugging
                                                if let Some(obj) = sub_action.as_object() {
                                                    for (key, value) in obj {
                                                        if key != "text" { // Skip text to reduce noise
                                                            println!("    {}: {}", key, value);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Look for potential improvements in parsing patterns
                        println!("\n=== POTENTIAL PARSER IMPROVEMENTS ===");
                        check_for_improvements(unique_abilities);
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

fn check_for_improvements(abilities: &[serde_json::Value]) {
    let mut potential_improvements = Vec::new();
    
    for (index, ability) in abilities.iter().enumerate() {
        if let Some(effect) = ability.get("effect") {
            if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                for (action_index, sub_action) in actions.iter().enumerate() {
                    if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                        // Check for potential improvements
                        match action_type {
                            "move_cards" => {
                                // Check for missing optional flags that could be inferred
                                if sub_action.get("optional").is_none() {
                                    if let Some(text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                        if text.contains("てもよい") {
                                            potential_improvements.push(format!(
                                                "Ability #{} action #{}: Could infer optional=true from 'てもよい'", 
                                                index, action_index
                                            ));
                                        }
                                    }
                                }
                                
                                // Check for missing state_change that could be inferred
                                if sub_action.get("state_change").is_none() {
                                    if let Some(text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                        if text.contains("ウェイト状態") {
                                            potential_improvements.push(format!(
                                                "Ability #{} action #{}: Could infer state_change=wait from 'ウェイト状態'", 
                                                index, action_index
                                            ));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    
    if !potential_improvements.is_empty() {
        println!("Found {} potential improvements:", potential_improvements.len());
        for improvement in &potential_improvements {
            println!("  - {}", improvement);
        }
    } else {
        println!("✓ No obvious improvements found");
    }
}
