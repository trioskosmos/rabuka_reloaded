// Show full ability #593 structure
use std::path::Path;

fn main() {
    let abilities_path = Path::new("c:\\Users\\trios\\OneDrive\\Documents\\rabuka_reloaded\\cards\\abilities.json");
    
    match std::fs::read_to_string(abilities_path) {
        Ok(content) => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(data) => {
                    if let Some(unique_abilities) = data.get("unique_abilities").and_then(|v| v.as_array()) {
                        if let Some(ability) = unique_abilities.get(593) {
                            println!("========================================");
                            println!("FULL ABILITY #593 STRUCTURE");
                            println!("========================================");
                            
                            // Pretty print the entire ability
                            println!("{}", serde_json::to_string_pretty(ability).unwrap());
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
