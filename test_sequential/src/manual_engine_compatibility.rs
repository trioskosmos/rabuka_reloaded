// Manual engine compatibility analysis - examine actual parser output vs engine expectations
use std::path::Path;

fn main() {
    println!("🔧 MANUAL ENGINE COMPATIBILITY ANALYSIS");
    println!("=======================================");
    println!("Examining parser output against actual engine struct requirements...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        // Focus on complex abilities that are likely to have compatibility issues
                        let complex_abilities = vec![57, 58, 144, 523, 256, 360, 367];
                        let sequential_abilities = vec![144, 449, 575, 593];
                        let conditional_abilities = vec![87, 258];
                        
                        println!("\n📊 COMPLEX ABILITY ANALYSIS");
                        println!("==============================");
                        for &ability_index in &complex_abilities {
                            analyze_complex_ability(ability_index, unique_abilities);
                        }
                        
                        println!("\n🔄 SEQUENTIAL ABILITY ANALYSIS");
                        println!("================================");
                        for &ability_index in &sequential_abilities {
                            analyze_sequential_ability(ability_index, unique_abilities);
                        }
                        
                        println!("\n🔀 CONDITIONAL ABILITY ANALYSIS");
                        println!("=================================");
                        for &ability_index in &conditional_abilities {
                            analyze_conditional_ability(ability_index, unique_abilities);
                        }
                        
                        println!("\n⚠️  FIELD COMPATIBILITY ISSUES");
                        println!("==============================");
                        analyze_field_compatibility_issues(unique_abilities);
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

fn analyze_complex_ability(index: usize, abilities: &[serde_json::Value]) {
    if let Some(ability) = abilities.get(index) {
        println!("\n--- Complex Ability #{} ---", index);
        if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
            println!("Text: {}", full_text);
        }
        
        if let Some(effect) = ability.get("effect") {
            if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                println!("Action type: {}", action);
                
                if action == "sequential" {
                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                        println!("Sequential actions: {}", actions.len());
                        for (i, sub_action) in actions.iter().enumerate() {
                            if let Some(sub_action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                                println!("  Action {}: {}", i, sub_action_type);
                                
                                // Check for engine compatibility issues
                                check_action_engine_compatibility(i, sub_action, sub_action_type);
                            }
                        }
                    }
                } else {
                    check_action_engine_compatibility(0, effect, action);
                }
            }
        }
    }
}

fn analyze_sequential_ability(index: usize, abilities: &[serde_json::Value]) {
    if let Some(ability) = abilities.get(index) {
        println!("\n--- Sequential Ability #{} ---", index);
        if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
            println!("Text: {}", full_text);
        }
        
        if let Some(effect) = ability.get("effect") {
            if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                println!("Action type: {}", action);
                
                if action == "sequential" {
                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                        println!("Sequential actions: {}", actions.len());
                        for (i, sub_action) in actions.iter().enumerate() {
                            if let Some(sub_action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                                println!("  Action {}: {}", i, sub_action_type);
                                check_action_engine_compatibility(i, sub_action, sub_action_type);
                            }
                        }
                    }
                } else {
                    check_action_engine_compatibility(0, effect, action);
                }
            }
        }
    }
}

fn analyze_conditional_ability(index: usize, abilities: &[serde_json::Value]) {
    if let Some(ability) = abilities.get(index) {
        println!("\n--- Conditional Ability #{} ---", index);
        if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
            println!("Text: {}", full_text);
        }
        
        if let Some(effect) = ability.get("effect") {
            if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                println!("Action type: {}", action);
                
                // Check for conditional_alternative structure
                if action == "conditional_alternative" {
                    println!("Conditional alternative structure:");
                    if let Some(primary_effect) = effect.get("primary_effect") {
                        println!("  ✓ Has primary_effect");
                    } else {
                        println!("  ⚠️  Missing primary_effect");
                    }
                    if let Some(alternative_effect) = effect.get("alternative_effect") {
                        println!("  ✓ Has alternative_effect");
                    } else {
                        println!("  ⚠️  Missing alternative_effect");
                    }
                    if let Some(condition) = effect.get("condition") {
                        println!("  ✓ Has condition: {:?}", condition);
                    } else {
                        println!("  ⚠️  Missing condition");
                    }
                } else {
                    check_action_engine_compatibility(0, effect, action);
                }
            }
        }
    }
}

fn check_action_engine_compatibility(action_index: usize, action: &serde_json::Value, action_type: &str) {
    let mut issues = Vec::new();
    
    // Check for common engine compatibility issues
    match action_type {
        "move_cards" => {
            // Engine expects these fields for move_cards
            let required_fields = vec!["source", "destination", "count"];
            for field in &required_fields {
                if action.get(field).is_none() {
                    issues.push(format!("Missing required field '{}'", field));
                }
            }
            
            // Check for optional but important fields
            if action.get("card_type").is_none() {
                issues.push("Missing card_type (usually required)".to_string());
            }
            
            // Check for state_change for energy cards
            if let Some(text) = action.get("text").and_then(|t| t.as_str()) {
                if text.contains("ウェイト状態") && action.get("state_change").is_none() {
                    issues.push("Missing state_change for energy card placement".to_string());
                }
            }
        }
        "draw_card" => {
            // Engine expects source/destination for draw_card
            if action.get("source").is_none() {
                issues.push("Missing source for draw_card".to_string());
            }
            if action.get("destination").is_none() {
                issues.push("Missing destination for draw_card".to_string());
            }
            if action.get("count").is_none() {
                issues.push("Missing count for draw_card".to_string());
            }
        }
        "gain_resource" => {
            // Engine expects resource and count for gain_resource
            if action.get("resource").is_none() {
                issues.push("Missing resource for gain_resource".to_string());
            }
            if action.get("count").is_none() {
                issues.push("Missing count for gain_resource".to_string());
            }
        }
        "look_and_select" => {
            // Engine expects look_action and select_action
            if action.get("look_action").is_none() {
                issues.push("Missing look_action for look_and_select".to_string());
            }
            if action.get("select_action").is_none() {
                issues.push("Missing select_action for look_and_select".to_string());
            }
        }
        _ => {}
    }
    
    // Check for text field (important for debugging)
    if action.get("text").is_none() {
        issues.push("Missing text field (important for debugging)".to_string());
    }
    
    // Report issues
    if !issues.is_empty() {
        println!("    ⚠️  Compatibility issues for action {}:", action_index);
        for issue in issues {
            println!("      - {}", issue);
        }
    } else {
        println!("    ✓ No compatibility issues detected");
    }
}

fn analyze_field_compatibility_issues(abilities: &[serde_json::Value]) {
    let mut total_issues = 0;
    let mut field_issues = Vec::new();
    
    for (index, ability) in abilities.iter().enumerate() {
        if let Some(effect) = ability.get("effect") {
            // Check for missing text field in effect
            if effect.get("text").is_none() {
                field_issues.push(format!("Ability #{}: missing effect text", index));
                total_issues += 1;
            }
            
            // Check for actions array
            if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                for (action_index, sub_action) in actions.iter().enumerate() {
                    if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                        // Check for problematic field values
                        if let Some(source) = sub_action.get("source").and_then(|s| s.as_str()) {
                            if source.is_empty() {
                                field_issues.push(format!("Ability #{} action {}: empty source field", index, action_index));
                                total_issues += 1;
                            }
                        }
                        
                        if let Some(destination) = sub_action.get("destination").and_then(|d| d.as_str()) {
                            if destination.is_empty() {
                                field_issues.push(format!("Ability #{} action {}: empty destination field", index, action_index));
                                total_issues += 1;
                            }
                        }
                        
                        // Check for inconsistent count types
                        if let Some(count) = sub_action.get("count") {
                            if count.is_string() {
                                field_issues.push(format!("Ability #{} action {}: count is string instead of number", index, action_index));
                                total_issues += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("Total field compatibility issues: {}", total_issues);
    
    if !field_issues.is_empty() {
        println!("Sample issues (first 10):");
        for issue in field_issues.iter().take(10) {
            println!("  - {}", issue);
        }
        if field_issues.len() > 10 {
            println!("  ... and {} more", field_issues.len() - 10);
        }
    } else {
        println!("✅ No field compatibility issues detected");
    }
}
