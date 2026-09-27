#[cfg(not(feature = "no_std"))]
use std::fs;
#[cfg(not(feature = "no_std"))]
use std::path::Path;

#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[cfg(not(feature = "no_std"))]
use std::string::String;
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

#[derive(Debug, Clone)]
pub struct DeckEntry {
    pub card_no: String,
    pub quantity: u8,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(serde::Deserialize))]
pub struct DeckListEntry {
    pub name: String,
    pub cards: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DeckList {
    pub name: String,
    pub entries: Vec<DeckEntry>,
}

/// Load and merge two deck JSON files from DECK_CARD_FILES, deduplicating by card_no.
/// JSON parsing requires the `serde_support` feature; on no-serde targets (DS/PS1/etc)
/// this returns cards from the embedded compact blob when available, else empty.
pub fn load_two_decks(deck1_idx: usize, deck2_idx: usize) -> Vec<crate::card::Card> {
    #[cfg(feature = "serde_support")]
    {
        let json1 = DECK_CARD_FILES[deck1_idx];
        let mut merged: Vec<crate::card::Card> = serde_json::from_str(json1).unwrap_or_default();
        if deck1_idx != deck2_idx && deck2_idx < DECK_CARD_FILES.len() {
            let json2 = DECK_CARD_FILES[deck2_idx];
            let cards2: Vec<crate::card::Card> = serde_json::from_str(json2).unwrap_or_default();
            for c in cards2 {
                if !merged.iter().any(|m| m.card_no == c.card_no) {
                    merged.push(c);
                }
            }
        }
        merged
    }
    #[cfg(not(feature = "serde_support"))]
    {
        // Compact builds: decode only the two selected decks' cards from the
        // engine-baked per-deck blobs. No serde, no full-database load.
        let _ = (deck1_idx, deck2_idx);
        let mut merged: Vec<crate::card::Card> = Vec::new();
        let mut seen: crate::HashSet<String> = crate::HashSet::default();
        for idx in [deck1_idx, deck2_idx] {
            if idx >= crate::decks_cards_gen::DECK_CARD_BLOBS.len() {
                continue;
            }
            for card in crate::core::card_binary::decode_all_cards_from_slice(
                crate::decks_cards_gen::DECK_CARD_BLOBS[idx],
            ) {
                let no = card.card_no.to_string();
                if seen.insert(no.clone()) {
                    merged.push(card);
                }
            }
        }
        merged
    }
}

/// Load two decks' cards with abilities attached — the one-liner every
/// console port's boot flow inlines (load_two_decks + attach_abilities).
/// `no_std`-safe.
pub fn load_two_decks_with_abilities(deck1_idx: usize, deck2_idx: usize) -> Vec<crate::card::Card> {
    let mut cards = load_two_decks(deck1_idx, deck2_idx);
    crate::card_loader::CardLoader::attach_abilities(&mut cards);
    cards
}

pub struct DeckParser;

#[cfg(feature = "serde_support")]
pub const DECK_CARD_FILES: &[&str] = &[
    include_str!("../../baked/deck_0_cards.json"),
    include_str!("../../baked/deck_1_cards.json"),
    include_str!("../../baked/deck_2_cards.json"),
    include_str!("../../baked/deck_3_cards.json"),
    include_str!("../../baked/deck_4_cards.json"),
    include_str!("../../baked/deck_5_cards.json"),
    include_str!("../../baked/deck_6_cards.json"),
    include_str!("../../baked/deck_7_cards.json"),
    include_str!("../../baked/deck_8_cards.json"),
    include_str!("../../baked/deck_9_cards.json"),
    include_str!("../../baked/deck_10_cards.json"),
    include_str!("../../baked/deck_11_cards.json"),
    include_str!("../../baked/deck_12_cards.json"),
    include_str!("../../baked/deck_13_cards.json"),
    include_str!("../../baked/deck_14_cards.json"),
    include_str!("../../baked/deck_15_cards.json"),
];

impl DeckParser {
    #[cfg(not(feature = "no_std"))]
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
            if let Some((card_no, quantity)) = Self::parse_line(line) {
                entries.push(DeckEntry { card_no, quantity });
            }
        }

        Ok(DeckList { name, entries })
    }

    #[cfg(not(feature = "no_std"))]
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

    #[cfg(not(feature = "no_std"))]
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
            } else if c == '>' || (acc.ends_with('\n') && c == '\n') {
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
        let mut s = raw.replace('+', "＋").replace('!', "！");
        // Strip trailing fullwidth plus/exclamation for canonical lookup
        // since some cards exist with/without the rarity suffix (e.g. P vs P＋)
        while s.ends_with('＋') || s.ends_with('！') {
            s.pop();
        }
        s
    }

    /// Parses a single line and returns (card_no, quantity).
    /// Supports "card_no x quantity", "quantity x card_no", or bare card_no (qty=1).
    fn parse_line(line: &str) -> Option<(String, u8)> {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            return None;
        }

        if let Some((first, second)) = line.split_once(" x ") {
            let (card_no, quantity) = first
                .trim()
                .parse::<u8>()
                .ok()
                .map(|quantity| (second.trim(), quantity))
                .or_else(|| {
                    second
                        .trim()
                        .parse::<u8>()
                        .ok()
                        .map(|quantity| (first.trim(), quantity))
                })?;
            return Some((Self::normalize_card_no(card_no), quantity));
        }

        (line.contains('-') && !line.contains(' ')).then(|| (Self::normalize_card_no(line), 1))
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
