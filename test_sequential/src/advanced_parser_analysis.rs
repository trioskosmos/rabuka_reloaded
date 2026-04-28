// Advanced parser analysis to look for edge cases and potential improvements
use std::path::Path;

fn main() {
    println!("Running advanced parser analysis for edge cases and improvements...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut potential_issues = Vec::new();
                        let mut field_anomalies = Vec::new();
                        let mut unusual_patterns = Vec::new();
                        
                        for (index, ability) in unique_abilities.iter().enumerate() {
                            analyze_ability(index, ability, &mut potential_issues, &mut field_anomalies, &mut unusual_patterns);
                        }
                        
                        println!("\n=== ADVANCED ANALYSIS RESULTS ===");
                        println!("Total abilities analyzed: {}", unique_abilities.len());
                        
                        if !potential_issues.is_empty() {
                            println!("\n⚠️  POTENTIAL ISSUES ({}):", potential_issues.len());
                            for issue in &potential_issues {
                                println!("  - {}", issue);
                            }
                        } else {
                            println!("\n✓ No potential issues detected");
                        }
                        
                        if !field_anomalies.is_empty() {
                            println!("\n🔍 FIELD ANOMALIES ({}):", field_anomalies.len());
                            for anomaly in &field_anomalies {
                                println!("  - {}", anomaly);
                            }
                        } else {
                            println!("\n✓ No field anomalies detected");
                        }
                        
                        if !unusual_patterns.is_empty() {
                            println!("\n📊 UNUSUAL PATTERNS ({}):", unusual_patterns.len());
                            for pattern in &unusual_patterns {
                                println!("  - {}", pattern);
                            }
                        } else {
                            println!("\n✓ No unusual patterns detected");
                        }
                        
                        // Check for specific edge cases
                        check_edge_cases(unique_abilities);
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

fn analyze_ability(index: usize, ability: &serde_json::Value, potential_issues: &mut Vec<String>, field_anomalies: &mut Vec<String>, unusual_patterns: &mut Vec<String>) {
    if let Some(effect) = ability.get("effect") {
        // Check for missing critical fields in move_cards actions
        if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
            for (action_index, sub_action) in actions.iter().enumerate() {
                if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                    match action_type {
                        "move_cards" => {
                            let has_source = sub_action.get("source").is_some();
                            let has_destination = sub_action.get("destination").is_some();
                            let has_count = sub_action.get("count").is_some();
                            let has_text = sub_action.get("text").is_some();
                            
                            if !has_text {
                                potential_issues.push(format!("Ability #{} action #{}: move_cards missing text field", index, action_index));
                            }
                            
                            // Check for unusual count values
                            if let Some(count) = sub_action.get("count") {
                                if let Some(count_val) = count.as_u64() {
                                    if count_val > 10 {
                                        unusual_patterns.push(format!("Ability #{} action #{}: unusually high count ({})", index, action_index, count_val));
                                    }
                                }
                            }
                            
                            // Check for missing optional but important fields
                            if !has_source && !has_destination {
                                field_anomalies.push(format!("Ability #{} action #{}: move_cards missing both source and destination", index, action_index));
                            }
                        }
                        "draw_card" => {
                            let has_source = sub_action.get("source").is_some();
                            let has_destination = sub_action.get("destination").is_some();
                            
                            if !has_source || !has_destination {
                                field_anomalies.push(format!("Ability #{} action #{}: draw_card missing source/destination", index, action_index));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn check_edge_cases(abilities: &[serde_json::Value]) {
    println!("\n=== EDGE CASE ANALYSIS ===");
    
    let mut very_long_effects = 0;
    let mut very_short_effects = 0;
    let mut high_complexity = 0;
    let mut unusual_card_types = Vec::new();
    
    for (index, ability) in abilities.iter().enumerate() {
        if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
            // Check for unusually long or short effects
            if full_text.len() > 200 {
                very_long_effects += 1;
            } else if full_text.len() < 50 {
                very_short_effects += 1;
            }
        }
        
        if let Some(effect) = ability.get("effect") {
            if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                if actions.len() > 4 {
                    high_complexity += 1;
                    println!("  High complexity ability #{}: {} actions", index, actions.len());
                }
                
                // Check for unusual card types
                for action in actions {
                    if let Some(card_type) = action.get("card_type").and_then(|t| t.as_str()) {
                        match card_type {
                            "live_card" | "member_card" | "energy_card" => {},
                            other => {
                                unusual_card_types.push(format!("Ability #{}: unusual card type '{}'", index, other));
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("Very long effects (>200 chars): {}", very_long_effects);
    println!("Very short effects (<50 chars): {}", very_short_effects);
    println!("High complexity abilities (>4 actions): {}", high_complexity);
    
    if !unusual_card_types.is_empty() {
        println!("Unusual card types:");
        for card_type in unusual_card_types {
            println!("  {}", card_type);
        }
    } else {
        println!("✓ No unusual card types detected");
    }
}
