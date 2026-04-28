// Analyze very long effects to identify potential edge cases and improvements
use std::path::Path;

fn main() {
    println!("Analyzing very long effects for edge cases and improvements...");
    
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        let mut long_effects = Vec::new();
                        let mut very_long_effects = Vec::new();
                        let mut complex_patterns = Vec::new();
                        
                        for (index, ability) in unique_abilities.iter().enumerate() {
                            if let Some(full_text) = ability.get("full_text").and_then(|t| t.as_str()) {
                                if full_text.len() > 200 {
                                    long_effects.push((index, full_text.len(), full_text.to_string()));
                                }
                                if full_text.len() > 300 {
                                    very_long_effects.push((index, full_text.len(), full_text.to_string()));
                                }
                                
                                // Look for complex patterns that might challenge the parser
                                check_complex_patterns(index, full_text, &mut complex_patterns);
                            }
                        }
                        
                        // Sort by length
                        long_effects.sort_by(|a, b| b.1.cmp(&a.1));
                        very_long_effects.sort_by(|a, b| b.1.cmp(&a.1));
                        
                        println!("\n=== LONG EFFECTS ANALYSIS ===");
                        println!("Total effects > 200 chars: {}", long_effects.len());
                        println!("Total effects > 300 chars: {}", very_long_effects.len());
                        
                        if !very_long_effects.is_empty() {
                            println!("\n📏 TOP 10 LONGEST EFFECTS:");
                            for (i, (index, length, _text)) in very_long_effects.iter().take(10).enumerate() {
                                println!("  {}. Ability #{} - {} chars", i + 1, index, length);
                            }
                        }
                        
                        if !complex_patterns.is_empty() {
                            println!("\n🔍 COMPLEX PATTERNS FOUND:");
                            for pattern in &complex_patterns {
                                println!("  {}", pattern);
                            }
                        } else {
                            println!("\n✓ No complex patterns detected");
                        }
                        
                        // Analyze specific examples of the longest effects
                        if !very_long_effects.is_empty() {
                            println!("\n📝 ANALYSIS OF LONGEST EFFECT:");
                            let (index, length, text) = &very_long_effects[0];
                            println!("Ability #{} - {} chars:", index, length);
                            println!("{}", text);
                            println!();
                            
                            // Analyze its structure
                            if let Some(ability) = unique_abilities.get(*index) {
                                if let Some(effect) = ability.get("effect") {
                                    if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                        println!("Structure: {} actions", actions.len());
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

fn check_complex_patterns(index: usize, text: &str, complex_patterns: &mut Vec<String>) {
    // Check for patterns that might be challenging for the parser
    let patterns = [
        ("multiple conditionals", text.matches("場合").count() > 2),
        ("nested parentheses", text.matches("（").count() > 2),
        ("complex icons", text.matches("{{").count() > 5),
        ("multiple actions", text.matches("、").count() > 3),
        ("complex sequencing", text.contains("その後") && text.contains("そうした場合")),
        ("multiple choices", text.matches("好きな").count() > 2),
        ("complex conditions", text.contains("～") || text.contains("…")),
    ];
    
    for (pattern_name, condition) in &patterns {
        if *condition {
            complex_patterns.push(format!("Ability #{}: {}", index, pattern_name));
        }
    }
}
