//! GBA Deck Builder — custom deck creation saved to SRAM.
//!
//! Since GBA lacks the 3DS's QR reader, players build decks here. The
//! builder uses a 5-field input system (Name, Series, Rarity, Card, Qty)
//! with Left/Right to switch fields and Up/Down to change values. Decks
//! are written to SRAM via the `sav` codec and appear in DeckSelect.

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use rabuka_engine::card::CardType;
use rabuka_engine::core::card_binary::{decode_all_cards_from_slice, CARD_BLOB};
use rabuka_engine::game::sav::{encode_sav, SavDeck};

use crate::display::Display;
use crate::gba_ui::InputSource;
use crate::input::Button;
use crate::sram::write_sav;

/// Maximum cards in a deck (matches SAV format).
const MAX_DECK_CARDS: usize = 72;
/// Minimum cards for a legal deck.
const MIN_DECK_CARDS: usize = 40;
/// Maximum deck name length (matches SAV_NAME_LEN = 32, 31 chars + NUL).
const MAX_NAME_LEN: usize = 31;
/// Recent picks to show.
const RECENT_PICKS: usize = 8;
/// Maximum copies of a single card.
const MAX_COPIES: u8 = 4;
/// Maximum live cards in deck.
const MAX_LIVE_CARDS: u8 = 4;

/// Legality check result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Legality {
    Legal,
    TooFewCards { current: usize },
    TooManyCards { current: usize },
    TooManyLiveCards { count: u8 },
}

impl Legality {
    fn message(&self) -> &'static str {
        match self {
            Legality::Legal => "Legal",
            Legality::TooFewCards { .. } => "Too few cards (min 40)",
            Legality::TooManyCards { .. } => "Too many cards (max 72)",
            Legality::TooManyLiveCards { .. } => "Too many live cards (max 4)",
        }
    }

    fn is_legal(&self) -> bool {
        matches!(self, Legality::Legal)
    }
}

/// Check deck legality.
fn check_legality(cards: &[(String, u8)], all_cards: &[CardEntry]) -> Legality {
    let total: usize = cards.iter().map(|(_, q)| *q as usize).sum();
    
    // Min/max cards
    if total < MIN_DECK_CARDS {
        return Legality::TooFewCards { current: total };
    }
    if total > MAX_DECK_CARDS {
        return Legality::TooManyCards { current: total };
    }
    
    // Live card count
    let live_count: u8 = cards.iter()
        .filter_map(|(no, q)| all_cards.iter().find(|c| c.card_no == *no))
        .filter(|c| c.card_type == CardType::Live)
        .map(|c| *c.1)
        .sum();
    if live_count > MAX_LIVE_CARDS {
        return Legality::TooManyLiveCards { count: live_count };
    }
    
    Legality::Legal
}

/// Character set for deck name input (GBA has no keyboard).
const NAME_CHARSET: &[char] = &[
    'A','B','C','D','E','F','G','H','I','J','K','L','M','N','O','P','Q','R','S','T','U','V','W','X','Y','Z',
    'a','b','c','d','e','f','g','h','i','j','k','l','m','n','o','p','q','r','s','t','u','v','w','x','y','z',
    '0','1','2','3','4','5','6','7','8','9',
    '-','+','!','_',' ',
    'あ','い','う','え','お','か','き','く','け','こ','さ','し','す','せ','そ','た','ち','つ','て','と','な','に','ぬ','ね','の',
    'は','ひ','ふ','へ','ほ','ま','み','む','め','も','や','ゆ','よ','ら','り','る','れ','ろ','わ','を','ん',
    'ア','イ','ウ','エ','オ','カ','キ','ク','ケ','コ','サ','シ','ス','セ','ソ','タ','チ','ツ','テ','ト','ナ','ニ','ヌ','ネ','ノ',
    'ハ','ヒ','フ','ヘ','ホ','マ','ミ','ム','メ','モ','ヤ','ユ','ヨ','ラ','リ','ル','レ','ロ','ワ','ヲ','ン',
];

/// Current field being edited.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    DeckName,
    Series,
    Rarity,
    Card,
    Quantity,
}

/// Series groups (maps to Card.group).
const SERIES_LIST: &[&str] = &[
    "μ's", "Aqours", "虹ヶ咲", "Liella!", "蓮ノ空", "PR", "Other",
];

/// Rarity suffixes from card_no.
const RARITY_LIST: &[&str] = &[
    "N", "N＋", "R", "R＋", "SR", "SR＋", "SEC", "P", "P＋", "PR", "PR＋", "L", "PE＋", "SECL", "SRE", "SD", "SD2",
];

/// Pre-decoded card entry for the deck builder.
struct CardEntry {
    card_no: String,
    name: String,
    series: String,
    group: String,
    card_type: CardType,
}

/// Deck builder state machine.
pub struct DeckBuilder<'a, 'd, I: InputSource> {
    display: &'a mut Display<'d>,
    input: &'a mut I,

    // All non-energy cards decoded from ROM blob
    all_cards: Vec<CardEntry>,

    // Deck being built
    deck_name: String,
    cards: Vec<(String, u8)>, // (card_no, quantity)
    legality: Legality,        // Current legality status

    // UI state
    field: Field,
    series_idx: usize,
    rarity_idx: usize,
    card_idx: usize,
    quantity: u8,

    // Name input
    name_cursor: usize,
    name_char_idx: usize,

    // Recent picks (last 8)
    recent_picks: Vec<(String, u8)>,

    // Filtered card list (card numbers matching series+rarity)
    filtered_cards: Vec<String>,
}

impl<'a, 'd, I: InputSource> DeckBuilder<'a, 'd, I> {
    pub fn new(display: &'a mut Display<'d>, input: &'a mut I) -> Self {
        // Decode all cards from ROM blob once at startup
        let all_cards = Self::load_all_cards();

        let mut builder = Self {
            display,
            input,
            all_cards,
            deck_name: String::new(),
            cards: Vec::new(),
            legality: Legality::TooFewCards { current: 0 },
            field: Field::DeckName,
            series_idx: 0,
            rarity_idx: 0,
            card_idx: 0,
            quantity: 1,
            name_cursor: 0,
            name_char_idx: 0,
            recent_picks: Vec::new(),
            filtered_cards: Vec::new(),
        };
        builder.rebuild_filtered();
        builder
    }

    /// Load all non-energy cards from the ROM blob.
    fn load_all_cards() -> Vec<CardEntry> {
        let cards = decode_all_cards_from_slice(CARD_BLOB);
        let mut entries = Vec::new();
        for card in cards {
            if card.card_type != CardType::Energy {
                entries.push(CardEntry {
                    card_no: card.card_no.to_string(),
                    name: card.name.to_string(),
                    series: card.series.to_string(),
                    group: card.group.to_string(),
                    card_type: card.card_type,
                });
            }
        }
        entries
    }

    /// Run the deck builder loop. Returns true if a deck was saved.
    pub fn run(&mut self) -> bool {
        loop {
            self.render();
            self.input.poll();

            // Handle input based on current field
            match self.field {
                Field::DeckName => {
                    if self.handle_name_input() {
                        return true; // Saved
                    }
                }
                Field::Series => {
                    if self.handle_series_input() {
                        return true;
                    }
                }
                Field::Rarity => {
                    if self.handle_rarity_input() {
                        return true;
                    }
                }
                Field::Card => {
                    if self.handle_card_input() {
                        return true;
                    }
                }
                Field::Quantity => {
                    if self.handle_quantity_input() {
                        return true;
                    }
                }
            }

            self.display.wait();
        }
    }

    /// Rebuild filtered card list based on current series/rarity.
    fn rebuild_filtered(&mut self) {
        self.filtered_cards.clear();
        let series = SERIES_LIST[self.series_idx];
        let rarity = RARITY_LIST[self.rarity_idx];

        for entry in &self.all_cards {
            // Check series match
            let series_match = match series {
                "PR" => entry.series.contains("PR") || entry.group.contains("PR"),
                "Other" => {
                    !matches!(entry.group.as_str(), "μ's" | "Aqours" | "虹ヶ咲" | "Liella!" | "蓮ノ空")
                }
                _ => entry.group == series,
            };
            if !series_match {
                continue;
            }

            // Check rarity match (from card_no suffix)
            let rarity_match = entry.card_no.ends_with(&format!("-{}", rarity))
                || entry.card_no.ends_with(&format!("-{}", rarity.replace('＋', "+")));
            if !rarity_match {
                continue;
            }

            self.filtered_cards.push(entry.card_no.clone());
        }

        // Clamp card_idx
        if self.card_idx >= self.filtered_cards.len() {
            self.card_idx = self.filtered_cards.len().saturating_sub(1);
        }
    }

    /// Get total cards in deck (sum of quantities).
    fn total_cards(&self) -> usize {
        self.cards.iter().map(|(_, q)| *q as usize).sum()
    }

    /// Find card entry by card_no.
    fn find_card(&self, card_no: &str) -> Option<&CardEntry> {
        self.all_cards.iter().find(|e| e.card_no == card_no)
    }

    /// Show card detail screen using CardEntry data.
    fn show_card_detail(&mut self, card_no: &str) {
        if let Some(entry) = self.find_card(card_no) {
            // Build detail lines manually from CardEntry
            let mut lines: Vec<String> = Vec::new();
            lines.push(format!("[{}] {}", entry.card_no, entry.name));
            lines.push(format!("Series: {}  Group: {}", entry.series, entry.group));
            
            // Card type specific info
            match entry.card_type {
                rabuka_engine::card::CardType::Member => {
                    lines.push("Type: Member".to_string());
                    // Note: CardEntry doesn't have cost/hearts/blade - would need to add them
                }
                rabuka_engine::card::CardType::Live => {
                    lines.push("Type: Live".to_string());
                }
                rabuka_engine::card::CardType::Energy => {
                    lines.push("Type: Energy".to_string());
                }
            }
            lines.push("".to_string());
            lines.push("A/B/L/R/Start: Close".to_string());

            // Use the generic detail screen
            self.display.reset_vram();
            let mut scroll = 0usize;
            const VISIBLE: usize = 8;
            
            // Convert lines to wrapped format
            let wrapped_lines: Vec<String> = lines.iter()
                .flat_map(|l| Display::wrap_pane(l, 17))
                .collect();
            
            self.display.render_card_detail(None, &wrapped_lines, scroll);
            
            loop {
                self.input.poll();
                if self.input.just_pressed(Button::Up) && scroll > 0 {
                    scroll -= 1;
                    self.display.render_card_detail(None, &wrapped_lines, scroll);
                } else if self.input.just_pressed(Button::Down) && scroll + VISIBLE < wrapped_lines.len() {
                    scroll += 1;
                    self.display.render_card_detail(None, &wrapped_lines, scroll);
                } else if self.input.just_pressed(Button::A)
                    || self.input.just_pressed(Button::B)
                    || self.input.just_pressed(Button::L)
                    || self.input.just_pressed(Button::R)
                    || self.input.just_pressed(Button::Start)
                {
                    self.display.reset_vram();
                    return;
                }
                self.display.wait();
            }
        }
    }

    /// Add a card to the deck.
    fn add_card(&mut self, card_no: String, qty: u8) {
        // Check if already in deck
        for (existing_no, existing_qty) in &mut self.cards {
            if existing_no == &card_no {
                let new_qty = (*existing_qty + qty).min(4);
                *existing_qty = new_qty;
                // Update recent picks
                self.update_recent(card_no, new_qty);
                // Update legality
                self.legality = check_legality(&self.cards, &self.all_cards);
                return;
            }
        }
        // New card
        if self.cards.len() < MAX_DECK_CARDS && self.total_cards() + qty as usize <= MAX_DECK_CARDS {
            self.cards.push((card_no.clone(), qty));
            self.update_recent(card_no, qty);
            // Update legality
            self.legality = check_legality(&self.cards, &self.all_cards);
        }
    }

    fn update_recent(&mut self, card_no: String, qty: u8) {
        // Remove if already in recent
        self.recent_picks.retain(|(n, _)| n != &card_no);
        // Add to front
        self.recent_picks.insert(0, (card_no, qty));
        if self.recent_picks.len() > RECENT_PICKS {
            self.recent_picks.truncate(RECENT_PICKS);
        }
    }

    /// Try to save deck to SRAM.
    fn try_save(&mut self) -> bool {
        if self.cards.is_empty() {
            return false;
        }

        // Block saving illegal decks
        if !self.legality.is_legal() {
            return false;
        }

        let deck = SavDeck {
            name: self.deck_name.clone(),
            cards: {
                let mut v = Vec::new();
                for (no, qty) in &self.cards {
                    for _ in 0..*qty {
                        v.push(no.clone());
                    }
                }
                v
            },
        };

        match encode_sav(&[deck]) {
            Ok(sav_bytes) => {
                // Write to SRAM via GBA flash
                write_sav(&sav_bytes);
                true
            }
            Err(_) => false,
        }
    }

    // Input handlers return true if builder should exit (deck saved)
    fn handle_name_input(&mut self) -> bool {
        if self.input.just_pressed(Button::Left) {
            if self.name_cursor > 0 {
                self.name_cursor -= 1;
                self.name_char_idx = NAME_CHARSET.iter().position(|&c| {
                    self.deck_name.chars().nth(self.name_cursor) == Some(c)
                }).unwrap_or(0);
            } else {
                self.field = Field::Quantity; // Wrap to last field
            }
        } else if self.input.just_pressed(Button::Right) {
            if self.name_cursor < self.deck_name.len() {
                self.name_cursor += 1;
                self.name_char_idx = NAME_CHARSET.iter().position(|&c| {
                    self.deck_name.chars().nth(self.name_cursor) == Some(c)
                }).unwrap_or(0);
            } else if self.name_cursor < MAX_NAME_LEN {
                // Add new character at end
                self.deck_name.push(NAME_CHARSET[0]);
                self.name_cursor += 1;
                self.name_char_idx = 0;
            } else {
                self.field = Field::Series; // Wrap to next field
            }
        } else if self.input.just_pressed(Button::Up) {
            self.name_char_idx = (self.name_char_idx + 1) % NAME_CHARSET.len();
            if self.name_cursor < self.deck_name.len() {
                let mut chars: Vec<char> = self.deck_name.chars().collect();
                chars[self.name_cursor] = NAME_CHARSET[self.name_char_idx];
                self.deck_name = chars.into_iter().collect();
            }
        } else if self.input.just_pressed(Button::Down) {
            if self.name_char_idx == 0 {
                self.name_char_idx = NAME_CHARSET.len() - 1;
            } else {
                self.name_char_idx -= 1;
            }
            if self.name_cursor < self.deck_name.len() {
                let mut chars: Vec<char> = self.deck_name.chars().collect();
                chars[self.name_cursor] = NAME_CHARSET[self.name_char_idx];
                self.deck_name = chars.into_iter().collect();
            }
        } else if self.input.just_pressed(Button::B) {
            // Backspace
            if self.name_cursor > 0 {
                self.name_cursor -= 1;
                self.deck_name.remove(self.name_cursor);
                self.name_char_idx = NAME_CHARSET.iter().position(|&c| {
                    self.deck_name.chars().nth(self.name_cursor) == Some(c)
                }).unwrap_or(0);
            }
        } else if self.input.just_pressed(Button::A) {
            // Confirm name, move to next field
            self.field = Field::Series;
        } else if self.input.just_pressed(Button::L) || self.input.just_pressed(Button::R) {
            // Show detail for current card (if any selected)
            if !self.filtered_cards.is_empty() {
                let card_no = &self.filtered_cards[self.card_idx];
                self.show_card_detail(card_no);
            }
        } else if self.input.just_pressed(Button::Select) {
            // Open zone grid (not available without GameState, skip for now)
        } else if self.input.just_pressed(Button::Start) {
            // Start menu not available without GameState
        }
        false
    }

    fn handle_series_input(&mut self) -> bool {
        if self.input.just_pressed(Button::Left) {
            self.field = Field::DeckName;
        } else if self.input.just_pressed(Button::Right) {
            self.field = Field::Rarity;
        } else if self.input.just_pressed(Button::Up) {
            self.series_idx = (self.series_idx + 1) % SERIES_LIST.len();
            self.card_idx = 0;
            self.rebuild_filtered();
        } else if self.input.just_pressed(Button::Down) {
            self.series_idx = (self.series_idx + SERIES_LIST.len() - 1) % SERIES_LIST.len();
            self.card_idx = 0;
            self.rebuild_filtered();
        } else if self.input.just_pressed(Button::A) {
            self.field = Field::Rarity;
        } else if self.input.just_pressed(Button::L) || self.input.just_pressed(Button::R) {
            if !self.filtered_cards.is_empty() {
                let card_no = self.filtered_cards[self.card_idx].clone();
                self.show_card_detail(&card_no);
            }
        } else if self.input.just_pressed(Button::Select) {
        } else if self.input.just_pressed(Button::Start) {
        }
        false
    }

    fn handle_rarity_input(&mut self) -> bool {
        if self.input.just_pressed(Button::Left) {
            self.field = Field::Series;
        } else if self.input.just_pressed(Button::Right) {
            self.field = Field::Card;
        } else if self.input.just_pressed(Button::Up) {
            self.rarity_idx = (self.rarity_idx + 1) % RARITY_LIST.len();
            self.card_idx = 0;
            self.rebuild_filtered();
        } else if self.input.just_pressed(Button::Down) {
            self.rarity_idx = (self.rarity_idx + RARITY_LIST.len() - 1) % RARITY_LIST.len();
            self.card_idx = 0;
            self.rebuild_filtered();
        } else if self.input.just_pressed(Button::A) {
            self.field = Field::Card;
        } else if self.input.just_pressed(Button::L) || self.input.just_pressed(Button::R) {
            if !self.filtered_cards.is_empty() {
                let card_no = self.filtered_cards[self.card_idx].clone();
                show_card_detail(self.display, self.input, card_no);
            }
        } else if self.input.just_pressed(Button::Select) {
        } else if self.input.just_pressed(Button::Start) {
        }
        false
    }

    fn handle_card_input(&mut self) -> bool {
        if self.input.just_pressed(Button::Left) {
            self.field = Field::Rarity;
        } else if self.input.just_pressed(Button::Right) {
            self.field = Field::Quantity;
        } else if self.input.just_pressed(Button::Up) {
            if !self.filtered_cards.is_empty() {
                self.card_idx = (self.card_idx + 1) % self.filtered_cards.len();
            }
        } else if self.input.just_pressed(Button::Down) {
            if !self.filtered_cards.is_empty() {
                self.card_idx = (self.card_idx + self.filtered_cards.len() - 1) % self.filtered_cards.len();
            }
        } else if self.input.just_pressed(Button::A) {
            if !self.filtered_cards.is_empty() {
                let card_no = self.filtered_cards[self.card_idx].clone();
                // Show detail first
                self.show_card_detail(&card_no);
                // After detail returns, add the card
                self.add_card(card_no, self.quantity);
            }
        } else if self.input.just_pressed(Button::L) || self.input.just_pressed(Button::R) {
            if !self.filtered_cards.is_empty() {
                let card_no = self.filtered_cards[self.card_idx].clone();
                self.show_card_detail(&card_no);
            }
        } else if self.input.just_pressed(Button::Select) {
        } else if self.input.just_pressed(Button::Start) {
        }
        false
    }

    fn handle_quantity_input(&mut self) -> bool {
        if self.input.just_pressed(Button::Left) {
            self.field = Field::Card;
        } else if self.input.just_pressed(Button::Right) {
            self.field = Field::DeckName;
        } else if self.input.just_pressed(Button::Up) {
            self.quantity = (self.quantity % 4) + 1;
        } else if self.input.just_pressed(Button::Down) {
            self.quantity = if self.quantity == 1 { 4 } else { self.quantity - 1 };
        } else if self.input.just_pressed(Button::A) {
            // Try to save if deck has cards
            if !self.cards.is_empty() && self.try_save() {
                return true; // Exit builder
            }
        } else if self.input.just_pressed(Button::B) {
            // B on quantity = remove last added card
            if let Some((last_no, _)) = self.cards.pop() {
                self.recent_picks.retain(|(n, _)| n != &last_no);
                // Update legality
                self.legality = check_legality(&self.cards, &self.all_cards);
            }
        } else if self.input.just_pressed(Button::L) || self.input.just_pressed(Button::R) {
            if !self.filtered_cards.is_empty() {
                let card_no = self.filtered_cards[self.card_idx].clone();
                self.show_card_detail(&card_no);
            }
        } else if self.input.just_pressed(Button::Select) {
        } else if self.input.just_pressed(Button::Start) {
        }
        false
    }

    fn render(&mut self) {
        self.display.clear();

        // Header with legality indicator
        let legality_str = if self.legality.is_legal() { "✓ Legal" } else { "✗ Illegal" };
        self.display.println(&format!("DECK BUILDER [{}/{}] {} A:Done", self.total_cards(), MAX_DECK_CARDS, legality_str));

        // Field 1: Deck Name
        let name_prefix = if self.field == Field::DeckName { "> " } else { "  " };
        let mut name_display = format!("{}Name: {}", name_prefix, self.deck_name);
        if self.field == Field::DeckName && self.name_cursor <= self.deck_name.len() {
            // Show cursor as underscore at current position
            let cursor_pos = name_display.len() - (self.deck_name.len() - self.name_cursor);
            if cursor_pos < name_display.len() {
                name_display.insert(cursor_pos, '_');
            } else {
                name_display.push('_');
            }
        }
        self.display.println(&name_display);

        // Field 2: Series
        let series_prefix = if self.field == Field::Series { "> " } else { "  " };
        self.display.println(&format!("{}Series: {}", series_prefix, SERIES_LIST[self.series_idx]));

        // Field 3: Rarity
        let rarity_prefix = if self.field == Field::Rarity { "> " } else { "  " };
        self.display.println(&format!("{}Rarity: {}", rarity_prefix, RARITY_LIST[self.rarity_idx]));

        // Field 4: Card
        let card_prefix = if self.field == Field::Card { "> " } else { "  " };
        if !self.filtered_cards.is_empty() {
            let card_no = &self.filtered_cards[self.card_idx];
            if let Some(entry) = self.find_card(card_no) {
                self.display.println(&format!("{}Card: {}  {}", card_prefix, card_no, entry.name));
            } else {
                self.display.println(&format!("{}Card: {}", card_prefix, card_no));
            }
        } else {
            self.display.println(&format!("{}Card: (none)", card_prefix));
        }

        // Field 5: Quantity
        let qty_prefix = if self.field == Field::Quantity { "> " } else { "  " };
        self.display.println(&format!("{}Qty:   [{}]", qty_prefix, self.quantity));

        // Legality detail when illegal
        if !self.legality.is_legal() {
            self.display.println(&format!("  {}", self.legality.message()));
        }

        self.display.println(""); // spacer

        // Recent picks
        self.display.println("Recent:");
        for (i, (no, qty)) in self.recent_picks.iter().enumerate() {
            if let Some(entry) = self.find_card(no) {
                self.display.println(&format!("  {}. {}x{} {}", i + 1, no, qty, entry.name));
            } else {
                self.display.println(&format!("  {}. {}x{}", i + 1, no, qty));
            }
        }

        // Hint bar
        self.display.println("");
        match self.field {
            Field::DeckName => self.display.println("L/R:Detail"),
            Field::Series | Field::Rarity | Field::Card => self.display.println("L/R:Detail"),
            Field::Quantity => {
                if self.legality.is_legal() {
                    self.display.println("A:Save B:Remove L/R:Detail");
                } else {
                    self.display.println("B:Remove L/R:Detail (Fix errors to save)");
                }
            }
        }

        self.display.swap_buffers();
    }
}

/// Entry point called from main.
pub fn run_deck_builder<'a, 'd, I: InputSource>(
    display: &'a mut Display<'d>,
    input: &'a mut I,
) -> bool {
    let mut builder = DeckBuilder::new(display, input);
    builder.run()
}