use std::fs;
use std::path::Path;
use std::vec::Vec;
use std::string::String;

#[derive(Debug, Clone)]
pub struct DeckEntry {
    pub card_no: String,
    pub quantity: u32,
}

#[derive(Debug, Clone)]
pub struct DeckList {
    pub name: String,
    pub entries: Vec<DeckEntry>,
}

pub struct DeckParser;

impl DeckParser {
    pub fn parse_deck_file(path: &Path) -> Result<DeckList, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read deck file: {}", e))?;
        
        let name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let mut entries = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            
            // Parse format: "card_no x quantity" or "quantity x card_no"
            let parts: Vec<&str> = line.split(" x ").collect();
            if parts.len() != 2 {
                return Err(format!("Invalid line format: {}", line));
            }
            
            // Try to parse first part as quantity (for "quantity x card_no" format)
            let (card_no, quantity) = if let Ok(q) = parts[0].trim().parse::<u32>() {
                // Format: "quantity x card_no"
                (parts[1].trim().to_string(), q)
            } else {
                // Format: "card_no x quantity"
                let q = parts[1].trim().parse::<u32>()
                    .map_err(|e| format!("Invalid quantity: {}", e))?;
                (parts[0].trim().to_string(), q)
            };
            
            entries.push(DeckEntry { card_no, quantity });
        }
        
        Ok(DeckList { name, entries })
    }
    
    pub fn parse_all_decks_from_directory(dir_path: &Path) -> Result<Vec<DeckList>, String> {
        let mut decks = Vec::new();
        
        let dir_entries = fs::read_dir(dir_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;
        
        for entry in dir_entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map(|e| e == "txt").unwrap_or(false) {
                let deck = Self::parse_deck_file(&path)?;
                decks.push(deck);
            }
        }
        
        Ok(decks)
    }
    
    pub fn parse_all_decks() -> Result<Vec<DeckList>, String> {
        let decks_path = Path::new("../game/decks");
        Self::parse_all_decks_from_directory(decks_path)
    }
    
    pub fn deck_list_to_card_numbers(deck: &DeckList) -> Vec<String> {
        let mut card_numbers = Vec::new();
        
        for entry in &deck.entries {
            for _ in 0..entry.quantity {
                card_numbers.push(entry.card_no.clone());
            }
        }
        
        card_numbers
    }
}
