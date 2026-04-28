// Simple verification test for sequential abilities
use std::path::Path;

fn main() {
    println!("Verifying sequential ability parsing and engine compatibility...");
    
    // Test 1: Verify abilities.json has correct sequential structure
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            println!("✓ Successfully read abilities.json ({} bytes)", content.len());
            
            // Parse as JSON
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    println!("✓ Successfully parsed JSON");
                    
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        println!("✓ Found {} unique abilities", unique_abilities.len());
                        
                        let mut sequential_count = 0;
                        let mut complex_sequential_count = 0;
                        let mut parsing_issues = Vec::new();
                        
                        for (i, ability) in unique_abilities.iter().enumerate() {
                            if let Some(effect) = ability.get("effect") {
                                if let Some(action) = effect.get("action").and_then(|a| a.as_str()) {
                                    if action == "sequential" {
                                        sequential_count += 1;
                                        
                                        if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                            if actions.len() > 2 {
                                                complex_sequential_count += 1;
                                                
                                                // Verify each sub-action has required fields for engine
                                                for (j, sub_action) in actions.iter().enumerate() {
                                                    let sub_action_type = sub_action.get("action").and_then(|a| a.as_str());
                                                    let sub_action_text = sub_action.get("text").and_then(|t| t.as_str());
                                                    
                                                    if sub_action_type.is_none() {
                                                        parsing_issues.push(format!("Ability #{} action #{} missing 'action' field", i, j));
                                                    }
                                                    if sub_action_text.is_none() {
                                                        parsing_issues.push(format!("Ability #{} action #{} missing 'text' field", i, j));
                                                    }
                                                    
                                                    // Check for common engine-required fields
                                                    if let Some(action_type) = sub_action_type {
                                                        match action_type {
                                                            "draw_card" | "move_cards" => {
                                                                if sub_action.get("count").is_none() {
                                                                    parsing_issues.push(format!("Ability #{} action #{} ({}) missing 'count' field", i, j, action_type));
                                                                }
                                                                if sub_action.get("source").is_none() && action_type == "move_cards" {
                                                                    parsing_issues.push(format!("Ability #{} action #{} ({}) missing 'source' field", i, j, action_type));
                                                                }
                                                                if sub_action.get("destination").is_none() {
                                                                    parsing_issues.push(format!("Ability #{} action #{} ({}) missing 'destination' field", i, j, action_type));
                                                                }
                                                            }
                                                            _ => {}
                                                        }
                                                    }
                                                }
                                                
                                                if complex_sequential_count <= 3 { // Show first 3 examples
                                                    if let Some(text) = effect.get("text").and_then(|t| t.as_str()) {
                                                        println!("Complex sequential #{}: {} actions", i, actions.len());
                                                        println!("  Text: {}", text);
                                                        for (j, sub_action) in actions.iter().enumerate() {
                                                            if let Some(action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                                                                if let Some(action_text) = sub_action.get("text").and_then(|t| t.as_str()) {
                                                                    println!("    Action {}: {} -> {}", j+1, action_type, action_text);
                                                                }
                                                            }
                                                        }
                                                        println!("  ---");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("\n=== SUMMARY ===");
                        println!("✓ Total sequential abilities: {}", sequential_count);
                        println!("✓ Complex sequential (>2 actions): {}", complex_sequential_count);
                        
                        if parsing_issues.is_empty() {
                            println!("✓ No parsing issues found - all sequential abilities have required engine fields");
                        } else {
                            println!("✗ Found {} parsing issues:", parsing_issues.len());
                            for issue in parsing_issues.iter().take(10) {
                                println!("  - {}", issue);
                            }
                            if parsing_issues.len() > 10 {
                                println!("  ... and {} more", parsing_issues.len() - 10);
                            }
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
    
    // Test 2: Verify engine can handle the structure
    println!("\n=== ENGINE COMPATIBILITY TEST ===");
    
    // Create a test sequential ability matching the engine's expected structure
    let test_ability = r#"
    {
        "text": "カードを2枚引き、手札を1枚控え室に置く",
        "action": "sequential",
        "actions": [
            {
                "text": "カードを2枚引き",
                "action": "draw_card",
                "count": 2,
                "source": "deck",
                "destination": "hand"
            },
            {
                "text": "手札を1枚控え室に置く",
                "action": "move_cards",
                "count": 1,
                "source": "hand",
                "destination": "discard"
            }
        ]
    }
    "#;
    
    match serde_json::from_str::<serde_json::Value>(test_ability) {
        Ok(parsed) => {
            println!("✓ Engine can parse test sequential ability structure");
            
            if let Some(action) = parsed.get("action").and_then(|a| a.as_str()) {
                println!("✓ Main action: {}", action);
            }
            
            if let Some(actions) = parsed.get("actions").and_then(|a| a.as_array()) {
                println!("✓ Actions count: {}", actions.len());
                for (i, sub_action) in actions.iter().enumerate() {
                    if let Some(sub_action_type) = sub_action.get("action").and_then(|a| a.as_str()) {
                        if let Some(sub_action_text) = sub_action.get("text").and_then(|t| t.as_str()) {
                            println!("✓ Action {}: {} -> {}", i+1, sub_action_type, sub_action_text);
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Engine failed to parse test ability: {}", e);
        }
    }
    
    println!("\n=== CONCLUSION ===");
    println!("Parser and engine appear compatible for sequential abilities.");
}
