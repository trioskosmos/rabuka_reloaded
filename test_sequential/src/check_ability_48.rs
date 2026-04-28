// Check ability #48 card_type
use std::path::Path;

fn main() {
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        if let Some(ability) = abilities.get(48) {
                            println!("Ability #48 effect:");
                            if let Some(effect) = ability.get("effect") {
                                if let Some(actions) = effect.get("actions").and_then(|a| a.as_array()) {
                                    if let Some(action1) = actions.get(1) {
                                        println!("{}", serde_json::to_string_pretty(action1).unwrap());
                                        
                                        if action1.get("card_type").is_some() {
                                            println!("\n✅ card_type IS present!");
                                        } else {
                                            println!("\n❌ card_type is still missing");
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
