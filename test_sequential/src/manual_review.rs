// Manual review of abilities.json for issues
use std::path::Path;

fn main() {
    println!("=== MANUAL REVIEW OF ABILITIES.JSON ===");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        println!("Total abilities: {}", abilities.len());
                        println!();
                        
                        let mut issues = Vec::new();
                        
                        for (idx, ability) in abilities.iter().enumerate() {
                            // Check 1: Effect is null but is_null is false (inconsistent)
                            let effect = ability.get("effect");
                            let is_null = ability.get("is_null").and_then(|v| v.as_bool()).unwrap_or(false);
                            
                            if effect.is_none() && !is_null {
                                let full_text = ability.get("full_text").and_then(|t| t.as_str()).unwrap_or("");
                                if !(full_text.starts_with('(') && full_text.ends_with(')')) {
                                    issues.push(format!("Ability {}: effect is null but is_null=false (not a rule ability)", idx));
                                }
                            }
                            
                            // Check 2: Effect has action but no text
                            if let Some(effect) = effect {
                                let has_action = effect.get("action").is_some();
                                let has_text = effect.get("text").and_then(|t| t.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
                                
                                if has_action && !has_text {
                                    issues.push(format!("Ability {}: effect has action but no text", idx));
                                }
                                
                                // Check 3: Sequential without actions array
                                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                    if action == "sequential" {
                                        let has_actions = effect.get("actions").and_then(|a| a.as_array()).map(|a| !a.is_empty()).unwrap_or(false);
                                        if !has_actions {
                                            issues.push(format!("Ability {}: sequential without actions array", idx));
                                        }
                                    }
                                }
                                
                                // Check 4: Individual action issues
                                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                    for (action_idx, action) in actions.iter().enumerate() {
                                        // Missing action field
                                        if action.get("action").is_none() {
                                            issues.push(format!("Ability {} action {}: missing action field", idx, action_idx));
                                        }
                                        
                                        // move_cards missing critical fields
                                        if let Some(action_type) = action.get("action").and_then(|a| a.as_str()) {
                                            if action_type == "move_cards" {
                                                if action.get("source").is_none() || action.get("destination").is_none() {
                                                    issues.push(format!("Ability {} action {}: move_cards missing source/destination", idx, action_idx));
                                                }
                                            }
                                        }
                                        
                                        // Count is explicitly null (BAD)
                                        if let Some(count) = action.get("count") {
                                            if count.is_null() {
                                                issues.push(format!("Ability {} action {}: count is explicitly null", idx, action_idx));
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Check 5: Text length issues
                            if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                let text_len = full_text.len();
                                if text_len < 20 {
                                    issues.push(format!("Ability {}: very short full_text ({} chars)", idx, text_len));
                                } else if text_len > 400 {
                                    issues.push(format!("Ability {}: very long full_text ({} chars)", idx, text_len));
                                }
                            }
                        }
                        
                        // Print results
                        if !issues.is_empty() {
                            println!("Found {} potential issues:", issues.len());
                            println!();
                            for (i, issue) in issues.iter().enumerate().take(30) {
                                println!("  {}. {}", i + 1, issue);
                            }
                            if issues.len() > 30 {
                                println!("  ... and {} more issues", issues.len() - 30);
                            }
                        } else {
                            println!("✅ No obvious issues found in manual review");
                        }
                        
                        // Also show some examples for verification
                        println!("\n=== SAMPLE ABILITIES FOR VERIFICATION ===");
                        for idx in [0, 100, 200, 300, 400, 500, 600].iter().filter(|&&i| i < abilities.len()) {
                            if let Some(ability) = abilities.get(*idx) {
                                println!("\nAbility #{}:", idx);
                                if let Some(text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                    println!("  Text: {}", text.chars().take(80).collect::<String>());
                                    if text.len() > 80 {
                                        println!("  ... ({} chars total)", text.len());
                                    }
                                }
                                if let Some(effect) = ability.get("effect") {
                                    if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                        println!("  Action: {}", action);
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
