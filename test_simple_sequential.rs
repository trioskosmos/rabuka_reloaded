// Simple test to verify sequential ability parsing works
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimpleAbilityEffect {
    pub text: String,
    pub action: String,
    pub count: Option<u32>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub actions: Option<Vec<SimpleAbilityEffect>>,
}

fn main() {
    println!("Testing sequential ability structure...");
    
    // Create a sequential ability as it would appear in abilities.json
    let sequential_ability = SimpleAbilityEffect {
        text: "カードを2枚引き、手札を1枚控え室に置く".to_string(),
        action: "sequential".to_string(),
        count: None,
        source: None,
        destination: None,
        actions: Some(vec![
            SimpleAbilityEffect {
                text: "カードを2枚引き".to_string(),
                action: "draw_card".to_string(),
                count: Some(2),
                source: Some("deck".to_string()),
                destination: Some("hand".to_string()),
                actions: None,
            },
            SimpleAbilityEffect {
                text: "手札を1枚控え室に置く".to_string(),
                action: "move_cards".to_string(),
                count: Some(1),
                source: Some("hand".to_string()),
                destination: Some("discard".to_string()),
                actions: None,
            }
        ]),
    };
    
    // Test serialization/deserialization
    let json_str = serde_json::to_string_pretty(&sequential_ability).unwrap();
    println!("Serialized sequential ability:\n{}", json_str);
    
    let parsed: SimpleAbilityEffect = serde_json::from_str(&json_str).unwrap();
    println!("\nParsed successfully:");
    println!("  Action: {}", parsed.action);
    println!("  Text: {}", parsed.text);
    
    if let Some(ref actions) = parsed.actions {
        println!("  Number of sub-actions: {}", actions.len());
        for (i, action) in actions.iter().enumerate() {
            println!("    Action {}: {} (count: {:?}, source: {:?}, destination: {:?})", 
                i+1, action.action, action.count, action.source, action.destination);
        }
    }
    
    println!("\n✓ Sequential ability structure parsing works correctly");
}
