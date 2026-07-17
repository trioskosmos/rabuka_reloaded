#[cfg(not(feature = "psp"))]
use std::fs;
#[cfg(not(feature = "psp"))]
use std::path::Path;

#[cfg(feature = "psp")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(not(feature = "psp"))]
use std::string::String;
#[cfg(not(feature = "psp"))]
use std::vec::Vec;

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
    #[cfg(not(feature = "psp"))]
    pub fn parse_deck_file(path: &Path) -> Result<DeckList, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read deck file: {}", e))?;

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut entries = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // Parse format: "card_no x quantity", "quantity x card_no", or single card per line
            let parts: Vec<&str> = line.split(" x ").collect();
            if parts.len() == 2 {
                // Try to parse first part as quantity (for "quantity x card_no" format)
                let (card_no, quantity) = if let Ok(q) = parts[0].trim().parse::<u32>() {
                    (Self::clean_card_no(parts[1].trim()), q)
                } else {
                    let q = parts[1]
                        .trim()
                        .parse::<u32>()
                        .map_err(|e| format!("Invalid quantity: {}", e))?;
                    (Self::clean_card_no(parts[0].trim()), q)
                };
                entries.push(DeckEntry { card_no, quantity });
            } else if line.contains('-') && !line.contains(' ') {
                // Single card per line (quantity defaults to 1)
                entries.push(DeckEntry {
                    card_no: Self::clean_card_no(line),
                    quantity: 1,
                });
            }
        }

        Ok(DeckList { name, entries })
    }

    #[cfg(not(feature = "psp"))]
    pub fn parse_all_decks_from_directory(dir_path: &Path) -> Result<Vec<DeckList>, String> {
        let mut decks = Vec::new();

        let dir_entries =
            fs::read_dir(dir_path).map_err(|e| format!("Failed to read directory: {}", e))?;

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

    #[cfg(not(feature = "psp"))]
    pub fn parse_all_decks() -> Result<Vec<DeckList>, String> {
        let decks_path = Path::new("../web_ui/decks");
        Self::parse_all_decks_from_directory(decks_path)
    }

    /// Parse deck content from HTML or plain text input.
    /// Strips HTML tags, then extracts card identifiers from each line,
    /// expanding quantities (e.g. "card_no x 3" produces three copies).
    pub fn parse_deck_content(content: &str) -> Vec<String> {
        // Strip HTML tags (rough but effective for deck table rows)
        let cleaned = content
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("</tr>", "\n")
            .replace("</div>", "\n")
            .replace("<td>", " ")
            .replace("</td>", " ")
            .replace("<th>", " ")
            .replace("</th>", " ");
        let stripped = cleaned.chars().fold(String::new(), |mut acc, c| {
            if c == '<' {
                acc.push('\n');
                acc
            } else if c == '>' {
                acc
            } else if acc.ends_with('\n') && c == '\n' {
                acc
            } else {
                acc.push(c);
                acc
            }
        });

        let mut card_numbers = Vec::new();
        for line in stripped.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Some((card_no, quantity)) = Self::parse_line(line) {
                for _ in 0..quantity {
                    card_numbers.push(card_no.clone());
                }
            }
        }
        card_numbers
    }

    pub fn normalize_card_no(raw: &str) -> String {
        raw.replace('+', "＋").replace('!', "！")
    }

    fn clean_card_no(raw: &str) -> String {
        Self::normalize_card_no(raw)
    }

    /// Parses a single line and returns (card_no, quantity).
    /// Supports "card_no x quantity", "quantity x card_no", or bare card_no (qty=1).
    fn parse_line(line: &str) -> Option<(String, u32)> {
        let line = line.trim();
        let parts: Vec<&str> = line.split(" x ").collect();
        if parts.len() == 2 {
            let (card_no, quantity) = if let Ok(q) = parts[0].trim().parse::<u32>() {
                (parts[1].trim(), q)
            } else if let Ok(q) = parts[1].trim().parse::<u32>() {
                (parts[0].trim(), q)
            } else {
                return None;
            };
            if card_no.contains('-') {
                return Some((Self::clean_card_no(card_no), quantity));
            }
        }
        // Single identifier per line (quantity defaults to 1)
        if line.contains('-') && !line.contains(' ') {
            return Some((Self::clean_card_no(line), 1));
        }
        None
    }

    pub fn deck_list_to_card_numbers(deck: &DeckList) -> Vec<String> {
        let mut card_numbers = Vec::new();

        for entry in &deck.entries {
            for _ in 0..entry.quantity {
                card_numbers.push(entry.card_no.to_string());
            }
        }

        card_numbers
    }
}
