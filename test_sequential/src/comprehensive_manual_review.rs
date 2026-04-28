// Comprehensive manual review for subtle issues
use std::path::Path;
use std::collections::HashSet;

fn main() {
    println!("=== COMPREHENSIVE MANUAL REVIEW ===");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        
                        let mut issues: Vec<String> = Vec::new();
                        let mut action_types: HashSet<String> = HashSet::new();
                        
                        // Valid action types from engine
                        let valid_actions: HashSet<&str> = [
                            "move_cards", "draw_card", "gain_resource", "look_and_select",
                            "reveal", "select", "appear", "choice", "sequential", "modify_score",
                            "look", "change_state", "place_energy_under_member", "set_card_identity",
                            "custom", "conditional_alternative", "modify_yell_count", "null",
                            "draw", "position_change", "temporal", "energy_active"
                        ].iter().cloned().collect();
                        
                        for (idx, ability) in abilities.iter().enumerate() {
                            // Check effect structure
                            if let Some(effect) = ability.get("effect") {
                                let action = effect.get("action").and_then(|a| a.as_str()).unwrap_or("");
                                action_types.insert(action.to_string());
                                
                                // Check for non-standard action types
                                if !action.is_empty() && !valid_actions.contains(action) {
                                    issues.push(format!("Ability {}: non-standard action '{}'", idx, action));
                                }
                                
                                // Check for empty strings in critical fields
                                for field in &["source", "destination", "target", "resource"] {
                                    if let Some(val) = effect.get(field) {
                                        if val.as_str() == Some("") {
                                            issues.push(format!("Ability {}: empty '{}'", idx, field));
                                        }
                                    }
                                }
                                
                                // Check actions array
                                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                    for (i, action) in actions.iter().enumerate() {
                                        let act_type = action.get("action").and_then(|a| a.as_str()).unwrap_or("");
                                        
                                        // Empty action type
                                        if act_type.is_empty() {
                                            issues.push(format!("Ability {} action {}: empty action type", idx, i));
                                        }
                                        
                                        // Non-standard action
                                        if !act_type.is_empty() && !valid_actions.contains(act_type) {
                                            issues.push(format!("Ability {} action {}: non-standard '{}'", idx, i, act_type));
                                        }
                                        
                                        // Count is 0
                                        if let Some(count) = action.get("count") {
                                            if count.as_i64() == Some(0) {
                                                issues.push(format!("Ability {} action {}: count=0", idx, i));
                                            }
                                        }
                                        
                                        // Source == destination (no-op)
                                        let src = action.get("source").and_then(|s| s.as_str()).unwrap_or("");
                                        let dst = action.get("destination").and_then(|d| d.as_str()).unwrap_or("");
                                        if !src.is_empty() && src == dst {
                                            issues.push(format!("Ability {} action {}: source==destination ('{}')", idx, i, src));
                                        }
                                        
                                        // Very high count
                                        if let Some(count) = action.get("count") {
                                            if let Some(c) = count.as_i64() {
                                                if c > 10 {
                                                    issues.push(format!("Ability {} action {}: very high count ({})", idx, i, c));
                                                }
                                            }
                                        }
                                        
                                        // Negative count
                                        if let Some(count) = action.get("count") {
                                            if let Some(c) = count.as_i64() {
                                                if c < 0 {
                                                    issues.push(format!("Ability {} action {}: negative count ({})", idx, i, c));
                                                }
                                            }
                                        }
                                        
                                        // Check for null values where they shouldn't be
                                        for field in &["source", "destination", "count"] {
                                            if let Some(val) = action.get(field) {
                                                if val.is_null() {
                                                    issues.push(format!("Ability {} action {}: null '{}'", idx, i, field));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Check cost structure
                            if let Some(cost) = ability.get("cost") {
                                if let Some(cost_type) = cost.get("type").and_then(|t| t.as_str()) {
                                    if cost_type == "move_cards" {
                                        // move_cards cost should have source and destination
                                        if cost.get("source").is_none() {
                                            issues.push(format!("Ability {}: move_cards cost missing source", idx));
                                        }
                                        if cost.get("destination").is_none() {
                                            issues.push(format!("Ability {}: move_cards cost missing destination", idx));
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Print results
                        println!("Total abilities: {}", abilities.len());
                        println!("Found {} potential issues:", issues.len());
                        
                        for issue in issues.iter().take(50) {
                            println!("  {}", issue);
                        }
                        
                        if issues.len() > 50 {
                            println!("  ... and {} more", issues.len() - 50);
                        }
                        
                        println!("\nAll action types found:");
                        let mut sorted_types: Vec<&String> = action_types.iter().collect();
                        sorted_types.sort();
                        for act_type in sorted_types {
                            if !act_type.is_empty() {
                                println!("  {}", act_type);
                            }
                        }
                        
                        // Summary
                        if issues.is_empty() {
                            println!("\n✅ No issues found!");
                        } else {
                            println!("\n⚠️  {} issues found", issues.len());
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
