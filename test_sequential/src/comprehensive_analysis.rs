// Comprehensive parsing analysis for all ability types
use std::path::Path;
use std::collections::HashMap;

fn main() {
    println!("Running comprehensive parsing analysis for all ability types...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        println!("✓ Successfully loaded {} unique abilities", unique_abilities.len());
                        
                        let mut action_counts = HashMap::new();
                        let mut parsing_issues = Vec::new();
                        let mut complex_abilities = Vec::new();
                        
                        for (i, ability) in unique_abilities.iter().enumerate() {
                            if let Some(effect) = ability.get("effect") {
                                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                    // Count action types
                                    *action_counts.entry(action.to_string()).or_insert(0) += 1;
                                    
                                    // Check for complex abilities
                                    match action {
                                        "sequential" => check_sequential_ability(i, effect, &mut parsing_issues, &mut complex_abilities),
                                        "conditional_alternative" => check_conditional_alternative(i, effect, &mut parsing_issues, &mut complex_abilities),
                                        "look_and_select" => check_look_and_select(i, effect, &mut parsing_issues, &mut complex_abilities),
                                        "choice" => check_choice_ability(i, effect, &mut parsing_issues, &mut complex_abilities),
                                        _ => check_simple_ability(i, effect, &mut parsing_issues),
                                    }
                                }
                            }
                        }
                        
                        // Print action type distribution
                        println!("\n=== ACTION TYPE DISTRIBUTION ===");
                        let mut sorted_counts: Vec<_> = action_counts.iter().collect();
                        sorted_counts.sort_by(|a, b| b.1.cmp(a.1));
                        for (action, count) in sorted_counts {
                            println!("  {}: {}", action, count);
                        }
                        
                        // Print complex abilities summary
                        println!("\n=== COMPLEX ABILITIES SUMMARY ===");
                        println!("Sequential abilities: {}", action_counts.get("sequential").unwrap_or(&0));
                        println!("Conditional alternative abilities: {}", action_counts.get("conditional_alternative").unwrap_or(&0));
                        println!("Look and select abilities: {}", action_counts.get("look_and_select").unwrap_or(&0));
                        println!("Choice abilities: {}", action_counts.get("choice").unwrap_or(&0));
                        
                        // Print parsing issues
                        println!("\n=== PARSING ISSUES ===");
                        if parsing_issues.is_empty() {
                            println!("✓ No parsing issues found across all ability types");
                        } else {
                            println!("✗ Found {} parsing issues:", parsing_issues.len());
                            for issue in parsing_issues.iter().take(20) {
                                println!("  - {}", issue);
                            }
                            if parsing_issues.len() > 20 {
                                println!("  ... and {} more", parsing_issues.len() - 20);
                            }
                        }
                        
                        // Show examples of complex abilities
                        println!("\n=== COMPLEX ABILITY EXAMPLES ===");
                        for (i, (ability_type, text)) in complex_abilities.iter().take(5).enumerate() {
                            println!("{}. [{}] {}", i+1, ability_type, text);
                        }
                        if complex_abilities.len() > 5 {
                            println!("... and {} more complex abilities", complex_abilities.len() - 5);
                        }
                    }
                }
                Err(e) => {
                    println!("✗ Failed to parse JSON: {}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to read abilities.json: {}", e);
        }
    }
}

fn check_sequential_ability(index: usize, effect: &serde_json::Value, issues: &mut Vec<String>, complex: &mut Vec<(String, String)>) {
    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
        if actions.len() > 2 {
            complex.push(("sequential".to_string(), effect.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string()));
        }
        
        for (i, sub_action) in actions.iter().enumerate() {
            if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                match action_type {
                    "move_cards" => {
                        let has_count = sub_action.get("count").is_some();
                        let has_source = sub_action.get("source").is_some();
                        let has_destination = sub_action.get("destination").is_some();
                        
                        if !has_count || !has_source || !has_destination {
                            issues.push(format!("Sequential ability #{} action #{} (move_cards) missing fields - count: {}, source: {}, destination: {}", 
                                index, i, has_count, has_source, has_destination));
                        }
                    }
                    "draw_card" => {
                        let has_source = sub_action.get("source").is_some();
                        let has_destination = sub_action.get("destination").is_some();
                        
                        if !has_source || !has_destination {
                            issues.push(format!("Sequential ability #{} action #{} (draw_card) missing fields - source: {}, destination: {}", 
                                index, i, has_source, has_destination));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn check_conditional_alternative(index: usize, effect: &serde_json::Value, issues: &mut Vec<String>, complex: &mut Vec<(String, String)>) {
    complex.push(("conditional_alternative".to_string(), effect.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string()));
    
    // Check for required fields in conditional alternatives
    let has_primary = effect.get("primary_effect").is_some();
    let has_alternative = effect.get("alternative_effect").is_some();
    let has_condition = effect.get("condition").is_some() || effect.get("alternative_condition").is_some();
    
    if !has_primary || !has_alternative || !has_condition {
        issues.push(format!("Conditional alternative #{} missing required fields - primary: {}, alternative: {}, condition: {}", 
            index, has_primary, has_alternative, has_condition));
    }
}

fn check_look_and_select(index: usize, effect: &serde_json::Value, issues: &mut Vec<String>, complex: &mut Vec<(String, String)>) {
    complex.push(("look_and_select".to_string(), effect.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string()));
    
    // Check for required fields in look_and_select
    let has_look_action = effect.get("look_action").is_some();
    let has_select_action = effect.get("select_action").is_some();
    
    if !has_look_action || !has_select_action {
        issues.push(format!("Look and select #{} missing required fields - look_action: {}, select_action: {}", 
            index, has_look_action, has_select_action));
    }
}

fn check_choice_ability(index: usize, effect: &serde_json::Value, issues: &mut Vec<String>, complex: &mut Vec<(String, String)>) {
    complex.push(("choice".to_string(), effect.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string()));
    
    // Check for required fields in choice abilities
    let has_options = effect.get("options").is_some() || effect.get("choice_options").is_some();
    
    if !has_options {
        issues.push(format!("Choice ability #{} missing options field", index));
    }
}

fn check_simple_ability(index: usize, effect: &serde_json::Value, issues: &mut Vec<String>) {
    if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
        match action {
            "move_cards" => {
                let has_count = effect.get("count").is_some();
                let has_source = effect.get("source").is_some();
                let has_destination = effect.get("destination").is_some();
                
                // Some move_cards might not need all fields (e.g., when source/destination are inferred)
                if !has_source && !has_destination {
                    issues.push(format!("Simple ability #{} (move_cards) missing both source and destination", index));
                }
            }
            "draw_card" => {
                let has_source = effect.get("source").is_some();
                let has_destination = effect.get("destination").is_some();
                
                if !has_source || !has_destination {
                    issues.push(format!("Simple ability #{} (draw_card) missing fields - source: {}, destination: {}", 
                        index, has_source, has_destination));
                }
            }
            _ => {}
        }
    }
}
