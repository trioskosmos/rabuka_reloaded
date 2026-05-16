use rabuka_engine::core::card::Ability;
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct AbilitiesFile {
    unique_abilities: Vec<Ability>,
}

fn main() {
    let path = "../cards/abilities.json";
    let content = fs::read_to_string(path).expect("Failed to read abilities.json");
    
    match serde_json::from_str::<AbilitiesFile>(&content) {
        Ok(parsed) => {
            println!("Successfully parsed {} unique abilities into engine structs!", parsed.unique_abilities.len());
            // Let's do a quick check on the parsed data
            let with_cost = parsed.unique_abilities.iter().filter(|a| a.cost.is_some()).count();
            let with_effect = parsed.unique_abilities.iter().filter(|a| a.effect.is_some()).count();
            println!("- {} abilities have costs", with_cost);
            println!("- {} abilities have effects", with_effect);
        }
        Err(e) => {
            println!("Failed to parse abilities.json into engine structs:");
            println!("{}", e);
            std::process::exit(1);
        }
    }
}
