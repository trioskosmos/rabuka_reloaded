// Deep parser analysis to find any remaining subtle issues
use std::path::Path;

fn main() {
    println!("Running deep parser analysis for any remaining issues...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut subtle_issues = Vec::new();
                        let missing_optional_fields: Vec<String> = Vec::new();
                        let mut inconsistent_naming = Vec::new();
                        let mut potential_enhancements = Vec::new();
                        
                        for (index, ability) in unique_abilities.iter().enumerate() {
                            analyze_ability_deeply(index, ability, &mut subtle_issues, &mut inconsistent_naming, &mut potential_enhancements);
                        }
                        
                        println!("\n=== DEEP PARSER ANALYSIS RESULTS ===");
                        println!("Total abilities analyzed: {}", unique_abilities.len());
                        
                        if !subtle_issues.is_empty() {
                            println!("\n⚠️  SUBTLE ISSUES ({}):", subtle_issues.len());
                            for issue in &subtle_issues {
                                println!("  - {}", issue);
                            }
                        } else {
                            println!("\n✓ No subtle issues detected");
                        }
                        
                        if !inconsistent_naming.is_empty() {
                            println!("\n🔤 INCONSISTENT NAMING ({}):", inconsistent_naming.len());
                            for naming in &inconsistent_naming {
                                println!("  - {}", naming);
                            }
                        } else {
                            println!("\n✓ No inconsistent naming detected");
                        }
                        
                        if !potential_enhancements.is_empty() {
                            println!("\n💡 POTENTIAL ENHANCEMENTS ({}):", potential_enhancements.len());
                            for enhancement in &potential_enhancements {
                                println!("  - {}", enhancement);
                            }
                        } else {
                            println!("\n✓ No obvious enhancements needed");
                        }
                        
                        // Check for specific edge cases
                        check_specific_edge_cases(unique_abilities);
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

fn analyze_ability_deeply(index: usize, ability: &serde_json::Value, subtle_issues: &mut Vec<String>, inconsistent_naming: &mut Vec<String>, potential_enhancements: &mut Vec<String>) {
    if let Some(effect) = ability.get("effect") {
        // Check for missing text fields in actions
        if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
            for (action_index, sub_action) in actions.iter().enumerate() {
                if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                    // Check for missing text field
                    if sub_action.get("text").is_none() {
                        subtle_issues.push(format!("Ability #{} action #{}: missing text field for action '{}'", index, action_index, action_type));
                    }
                    
                    // Check for inconsistent field naming
                    check_field_consistency(index, action_index, sub_action, inconsistent_naming);
                    
                    // Check for potential enhancements
                    check_enhancement_opportunities(index, action_index, sub_action, potential_enhancements);
                }
            }
        }
        
        // Check for missing effect text
        if effect.get("text").is_none() {
            subtle_issues.push(format!("Ability #{}: missing effect text", index));
        }
    }
}

fn check_field_consistency(index: usize, action_index: usize, action: &serde_json::Value, inconsistent_naming: &mut Vec<String>) {
    // Check for consistent naming conventions
    if let Some(obj) = action.as_object() {
        for (key, _) in obj {
            // Check for snake_case vs camelCase inconsistencies
            if key.contains("_") && key.chars().any(|c| c.is_uppercase()) {
                inconsistent_naming.push(format!("Ability #{} action #{}: mixed case field '{}'", index, action_index, key));
            }
        }
    }
}

fn check_enhancement_opportunities(index: usize, action_index: usize, action: &serde_json::Value, potential_enhancements: &mut Vec<String>) {
    if let Some(action_type) = action.get("action").and_then(|a| a.as_str()) {
        match action_type {
            "move_cards" => {
                // Check for missing optional flags that could be inferred
                if action.get("optional").is_none() {
                    if let Some(text) = action.get("text").and_then(|t| t.as_str()) {
                        if text.contains("てもよい") {
                            potential_enhancements.push(format!("Ability #{} action #{}: could infer optional=true", index, action_index));
                        }
                    }
                }
                
                // Check for missing shuffle flags that could be inferred
                if action.get("shuffle").is_none() {
                    if let Some(text) = action.get("text").and_then(|t| t.as_str()) {
                        if text.contains("シャッフル") {
                            potential_enhancements.push(format!("Ability #{} action #{}: could infer shuffle=true", index, action_index));
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn check_specific_edge_cases(abilities: &[serde_json::Value]) {
    println!("\n=== SPECIFIC EDGE CASE ANALYSIS ===");
    
    let mut empty_actions = 0;
    let mut missing_conditions = 0;
    let mut unusual_counts = 0;
    let mut very_high_costs = 0;
    
    for (index, ability) in abilities.iter().enumerate() {
        if let Some(effect) = ability.get("effect") {
            if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                // Check for empty actions
                for (action_index, sub_action) in actions.iter().enumerate() {
                    if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                        if action_type == "custom" && sub_action.get("text").and_then(|t| t.as_str()).unwrap_or("").trim().is_empty() {
                            empty_actions += 1;
                            println!("  Empty custom action in ability #{} action #{}", index, action_index);
                        }
                        
                        // Check for unusual counts
                        if let Some(count) = sub_action.get("count") {
                            if let Some(count_val) = count.as_u64() {
                                if count_val > 20 {
                                    unusual_counts += 1;
                                    println!("  Unusual count {} in ability #{} action #{}", count_val, index, action_index);
                                }
                            }
                        }
                    }
                }
                
                // Check for conditions that might be missing
                if effect.get("condition").is_none() {
                    if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                        if text.contains("場合") || text.contains("～") {
                            missing_conditions += 1;
                            println!("  Possible missing condition in ability #{}", index);
                        }
                    }
                }
            }
            
            // Check for very high costs
            if let Some(cost) = ability.get("cost") {
                if let Some(cost_val) = cost.as_u64() {
                    if cost_val > 15 {
                        very_high_costs += 1;
                        println!("  Very high cost {} in ability #{}", cost_val, index);
                    }
                }
            }
        }
    }
    
    println!("Empty custom actions: {}", empty_actions);
    println!("Possible missing conditions: {}", missing_conditions);
    println!("Unusual counts (>20): {}", unusual_counts);
    println!("Very high costs (>15): {}", very_high_costs);
    
    if empty_actions == 0 && missing_conditions == 0 && unusual_counts == 0 && very_high_costs == 0 {
        println!("✓ No edge cases detected");
    }
}
