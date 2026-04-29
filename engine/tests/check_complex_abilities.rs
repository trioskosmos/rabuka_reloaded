use std::fs;
use serde_json;

#[derive(Debug, serde::Deserialize)]
struct Ability {
    full_text: String,
    triggers: Option<String>,
    use_limit: Option<u32>,
    cost: Option<serde_json::Value>,
    effect: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct Card {
    name: String,
    card_no: String,
    abilities: Vec<Ability>,
}

#[test]
fn check_all_ability_combinations() {
    // Load abilities.json
    let abilities_path = "../cards/abilities.json";
    let content = fs::read_to_string(abilities_path).expect("Failed to read abilities.json");
    
    let abilities: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse abilities.json");
    
    let abilities_list = abilities.get("unique_abilities")
        .and_then(|v| v.as_array())
        .expect("Abilities should have 'unique_abilities' array");
    
    println!("Total abilities in database: {}", abilities_list.len());
    
    // Find complex abilities
    let mut look_and_select_count = 0;
    let mut conditional_alternative_count = 0;
    let mut sequential_count = 0;
    let mut sequential_cost_count = 0;
    
    for ability_entry in abilities_list {
        if let Some(effect) = ability_entry.get("effect") {
            if let Some(action) = effect.get("action") {
                let action_str = action.as_str().unwrap_or("");
                
                match action_str {
                    "look_and_select" => {
                        look_and_select_count += 1;
                        if look_and_select_count <= 3 {
                            println!("\n=== look_and_select ability #{} ===", look_and_select_count);
                            println!("Full text: {}", ability_entry.get("full_text").and_then(|v| v.as_str()).unwrap_or(""));
                            println!("Effect action: {}", action_str);
                            if let Some(look_action) = effect.get("look_action") {
                                println!("Look action: {}", look_action.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                            }
                            if let Some(select_action) = effect.get("select_action") {
                                println!("Select action: {:?}", select_action.get("action"));
                            }
                        }
                    }
                    "conditional_alternative" => {
                        conditional_alternative_count += 1;
                        if conditional_alternative_count <= 3 {
                            println!("\n=== conditional_alternative ability #{} ===", conditional_alternative_count);
                            println!("Full text: {}", ability_entry.get("full_text").and_then(|v| v.as_str()).unwrap_or(""));
                            println!("Effect action: {}", action_str);
                            if let Some(alt_condition) = effect.get("alternative_condition") {
                                println!("Alternative condition: {:?}", alt_condition.get("type"));
                            }
                            if let Some(alt_effect) = effect.get("alternative_effect") {
                                println!("Alternative effect: {:?}", alt_effect.get("action"));
                            }
                        }
                    }
                    "sequential" => {
                        sequential_count += 1;
                        if sequential_count <= 3 {
                            println!("\n=== sequential ability #{} ===", sequential_count);
                            println!("Full text: {}", ability_entry.get("full_text").and_then(|v| v.as_str()).unwrap_or(""));
                            println!("Effect action: {}", action_str);
                            if let Some(actions) = effect.get("actions") {
                                println!("Actions count: {}", actions.as_array().map(|a| a.len()).unwrap_or(0));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        if let Some(cost) = ability_entry.get("cost") {
            if let Some(cost_type) = cost.get("type") {
                if cost_type.as_str() == Some("sequential_cost") {
                    sequential_cost_count += 1;
                    if sequential_cost_count <= 3 {
                        println!("\n=== sequential_cost ability #{} ===", sequential_cost_count);
                        println!("Full text: {}", ability_entry.get("full_text").and_then(|v| v.as_str()).unwrap_or(""));
                        println!("Cost type: {:?}", cost_type);
                        if let Some(costs) = cost.get("costs") {
                            println!("Cost steps: {}", costs.as_array().map(|a| a.len()).unwrap_or(0));
                        }
                    }
                }
            }
        }
    }
    
    println!("\n=== Summary ===");
    println!("look_and_select abilities: {}", look_and_select_count);
    println!("conditional_alternative abilities: {}", conditional_alternative_count);
    println!("sequential abilities: {}", sequential_count);
    println!("sequential_cost abilities: {}", sequential_cost_count);
    
    // Find the most complex ability
    println!("\n=== Finding most complex ability ===");
    let mut max_complexity = 0;
    let mut most_complex_name = String::new();
    let mut most_complex_text = String::new();
    
    for ability_entry in abilities_list {
        let mut complexity = 0;
        
        if let Some(effect) = ability_entry.get("effect") {
            if let Some(action) = effect.get("action") {
                complexity += 1;
                
                if effect.get("look_action").is_some() { complexity += 2; }
                if effect.get("select_action").is_some() { complexity += 2; }
                if let Some(actions) = effect.get("actions") {
                    complexity += actions.as_array().map(|a| a.len()).unwrap_or(0);
                }
                if effect.get("condition").is_some() { complexity += 2; }
                if effect.get("primary_effect").is_some() { complexity += 2; }
                if effect.get("alternative_condition").is_some() { complexity += 2; }
                if effect.get("alternative_effect").is_some() { complexity += 2; }
            }
        }
        
        if let Some(cost) = ability_entry.get("cost") {
            complexity += 1;
            if let Some(costs) = cost.get("costs") {
                complexity += costs.as_array().map(|a| a.len()).unwrap_or(0);
            }
        }
        
        if complexity > max_complexity {
            max_complexity = complexity;
            most_complex_name = ability_entry.get("cards")
                .and_then(|v| v.as_array())
                .and_then(|a| a.get(0))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            most_complex_text = ability_entry.get("full_text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
        }
    }
    
    println!("Most complex ability (complexity: {}): {}", max_complexity, most_complex_name);
    println!("Full text: {}", most_complex_text);
}
