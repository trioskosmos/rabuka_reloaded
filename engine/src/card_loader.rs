use crate::card::Card;
use std::collections::HashMap;
use std::path::Path;

pub struct CardLoader;

impl CardLoader {
    pub fn load_cards_from_file(path: &Path) -> Result<HashMap<String, Card>, Box<dyn std::error::Error>> {
        let file_content = std::fs::read_to_string(path)?;
        let cards_json: serde_json::Value = serde_json::from_str(&file_content)?;
        
        let mut cards_map = HashMap::new();
        
        if let Some(obj) = cards_json.as_object() {
            for (card_no, card_data) in obj {
                if let Ok(card) = serde_json::from_value::<Card>(card_data.clone()) {
                    cards_map.insert(card_no.clone(), card);
                }
            }
        }
        
        Ok(cards_map)
    }
    
    pub fn load_all_cards_from_directory(dir_path: &Path) -> Result<HashMap<String, Card>, Box<dyn std::error::Error>> {
        let mut all_cards = HashMap::new();
        
        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
                let cards = Self::load_cards_from_file(&path)?;
                for (card_no, card) in cards {
                    all_cards.insert(card_no, card);
                }
            }
        }
        
        Ok(all_cards)
    }
}
