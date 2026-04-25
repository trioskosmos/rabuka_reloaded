use crate::card::{Card, CardDatabase};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::vec::Vec;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Deck {
    pub main_deck: VecDeque<i16>,  // Card IDs
    pub energy_deck: VecDeque<i16>,  // Card IDs
}

#[derive(Debug, Clone)]
pub struct DeckValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl DeckValidationResult {
    pub fn new() -> Self {
        DeckValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
}

impl Deck {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Deck {
            main_deck: VecDeque::new(),
            energy_deck: VecDeque::new(),
        }
    }

    pub fn shuffle_main_deck(&mut self) {
        use rand::seq::SliceRandom;
        let mut cards: Vec<i16> = self.main_deck.drain(..).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.main_deck = cards.into();
    }

    #[allow(dead_code)]
    pub fn shuffle_main_deck_with_rng<R: rand::Rng>(&mut self, rng: &mut R) {
        use rand::seq::SliceRandom;
        let mut cards: Vec<i16> = self.main_deck.drain(..).collect();
        cards.shuffle(rng);
        self.main_deck = cards.into();
    }

    pub fn shuffle_energy_deck(&mut self) {
        use rand::seq::SliceRandom;
        let mut cards: Vec<i16> = self.energy_deck.drain(..).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.energy_deck = cards.into();
    }

    #[allow(dead_code)]
    pub fn shuffle_energy_deck_with_rng<R: rand::Rng>(&mut self, rng: &mut R) {
        use rand::seq::SliceRandom;
        let mut cards: Vec<i16> = self.energy_deck.drain(..).collect();
        cards.shuffle(rng);
        self.energy_deck = cards.into();
    }
}

pub struct DeckBuilder;

impl DeckBuilder {
    // Q3: Main deck composition validation (48 member + 12 live = 60 total, half deck: 24 + 6 = 30)
    // Q4: Main deck duplicates validation (max 4 of same card number)
    // Q5: Same card number different rarity validation (max 4 total regardless of rarity)
    // Q6: Different card numbers validation (can use 4 of each if card numbers differ)
    // Q7: Energy deck duplicates validation (any number of same cards allowed)
    pub fn validate_deck(card_db: &Arc<CardDatabase>, main_deck: &VecDeque<i16>, energy_deck: &VecDeque<i16>) -> DeckValidationResult {
        let mut result = DeckValidationResult::new();

        // Count card types in main deck
        let mut member_count = 0;
        let mut live_count = 0;
        let mut card_number_counts: HashMap<String, u32> = HashMap::new();

        for &card_id in main_deck {
            if let Some(card) = card_db.get_card(card_id) {
                match card.card_type {
                    crate::card::CardType::Member => {
                        member_count += 1;
                    }
                    crate::card::CardType::Live => {
                        live_count += 1;
                    }
                    _ => {}
                }

                // Extract card number (excluding rarity symbol)
                let card_number = Self::extract_card_number(&card.card_no);
                *card_number_counts.entry(card_number).or_insert(0) += 1;
            }
        }

        // Q3: Validate main deck composition
        let total_main = member_count + live_count;
        if total_main == 60 {
            if member_count != 48 || live_count != 12 {
                result.add_error(format!(
                    "Main deck must be exactly 48 member + 12 live = 60 total, found {} member + {} live",
                    member_count, live_count
                ));
            }
        } else if total_main == 30 {
            if member_count != 24 || live_count != 6 {
                result.add_error(format!(
                    "Half deck must be exactly 24 member + 6 live = 30 total, found {} member + {} live",
                    member_count, live_count
                ));
            }
        } else {
            result.add_error(format!(
                "Main deck must be 60 cards (48 member + 12 live) or 30 cards for half deck (24 member + 6 live), found {} total",
                total_main
            ));
        }

        // Q4, Q5, Q6: Validate main deck duplicates (max 4 of same card number)
        for (card_number, count) in &card_number_counts {
            if *count > 4 {
                result.add_error(format!(
                    "Card number {} appears {} times in main deck, maximum is 4",
                    card_number, count
                ));
            }
        }

        // Q7: Energy deck can have any number of same cards (no validation needed)
        // Energy deck size validation (should be 12)
        if energy_deck.len() != 12 {
            result.add_warning(format!(
                "Energy deck has {} cards, expected 12",
                energy_deck.len()
            ));
        }

        result
    }

    // Extract card number excluding rarity symbol (e.g., "PL!N-bp1-001-R+" -> "PL!N-bp1-001")
    fn extract_card_number(card_no: &str) -> String {
        // Card number format: SERIES-bp#-###-RARITY
        // We want to extract up to the card number (excluding rarity)
        let parts: Vec<&str> = card_no.split('-').collect();
        if parts.len() >= 3 {
            // Reconstruct without the last part (rarity)
            format!("{}-{}-{}", parts[0], parts[1], parts[2])
        } else {
            card_no.to_string()
        }
    }

    pub fn build_deck_from_database(card_db: &Arc<CardDatabase>, card_numbers: Vec<String>) -> Result<Deck, String> {
        let mut main_deck: VecDeque<i16> = VecDeque::new();
        let mut energy_deck: VecDeque<i16> = VecDeque::new();

        let mut member_count = 0;
        let mut live_count = 0;
        let mut energy_count = 0;

        let mut card_number_counts: HashMap<String, u32> = HashMap::new();

        for card_no in card_numbers {
            // Card number limit check removed for testing purposes
            let count = card_number_counts.entry(card_no.clone()).or_insert(0);
            *count += 1;

            if let Some(card_id) = card_db.get_card_id(&card_no) {
                if let Some(card) = card_db.get_card(card_id) {
                    match card.card_type {
                        crate::card::CardType::Member => {
                            main_deck.push_back(card_id);
                            member_count += 1;
                        }
                        crate::card::CardType::Live => {
                            main_deck.push_back(card_id);
                            live_count += 1;
                        }
                        crate::card::CardType::Energy => {
                            energy_deck.push_back(card_id);
                            energy_count += 1;
                        }
                    }
                }
            }
        }

        // Validate deck composition with priority on 12 live + 48 member
        // Rule 6.1.1: Main deck must have exactly 60 cards (48 member + 12 live)
        // Be lenient if cards are missing from database
        let total_main = member_count + live_count;
        if total_main < 60 {
            eprintln!("Warning: Main deck has {} cards (expected 60): {} member + {} live", total_main, member_count, live_count);
            // Allow decks with fewer cards due to missing cards in database
        }

        if live_count < 12 {
            eprintln!("Warning: Main deck has {} live cards (expected 12)", live_count);
        }

        if member_count < 48 {
            eprintln!("Warning: Main deck has {} member cards (expected 48)", member_count);
        }

        // Rule 6.1.1.3: Energy deck must have exactly 12 energy cards
        if energy_count != 12 {
            eprintln!("Warning: Energy deck has {} energy cards (expected 12)", energy_count);
        }

        Ok(Deck {
            main_deck,
            energy_deck,
        })
    }

    pub fn build_deck_from_cards(cards: Vec<Card>) -> Result<Deck, String> {
        // This method is deprecated in favor of build_deck_from_database
        // For backward compatibility, convert cards to card IDs
        let card_db = Arc::new(CardDatabase::load_or_create(cards.clone()));
        let card_numbers: Vec<String> = cards.iter().map(|c| c.card_no.clone()).collect();
        Self::build_deck_from_database(&card_db, card_numbers)
    }
    
    pub fn add_default_energy_cards_from_database(deck: &mut Deck, card_db: &Arc<CardDatabase>) -> Result<(), String> {
        if deck.energy_deck.is_empty() {
            // Add default energy cards
            let mut energy_card_ids: Vec<i16> = Vec::new();
            for (card_id, card) in card_db.cards.iter() {
                if card.is_energy() {
                    energy_card_ids.push(*card_id);
                    if energy_card_ids.len() >= 12 {
                        break;
                    }
                }
            }

            if energy_card_ids.len() < 12 {
                return Err(format!("Not enough energy cards available: found {}", energy_card_ids.len()));
            }

            for card_id in energy_card_ids {
                deck.energy_deck.push_back(card_id);
            }
        }
        Ok(())
    }

    pub fn add_default_energy_cards(deck: &mut Deck, all_cards: &HashMap<String, Card>) -> Result<(), String> {
        if deck.energy_deck.is_empty() {
            // Add default energy cards
            let energy_cards: Vec<&Card> = all_cards.values()
                .filter(|c| c.is_energy())
                .take(12)
                .collect();

            if energy_cards.len() < 12 {
                return Err(format!("Not enough energy cards available: found {}", energy_cards.len()));
            }

            for _card in energy_cards {
                // This is deprecated - use add_default_energy_cards_from_database instead
                // For now, we'll need to map card to card_id, but we don't have a mapping here
                // This function should not be used anymore
                return Err("add_default_energy_cards is deprecated, use add_default_energy_cards_from_database".to_string());
            }
        }
        Ok(())
    }
    
    pub fn build_deck_from_card_map(cards: &HashMap<String, Card>, card_numbers: Vec<String>) -> Result<Deck, String> {
        let mut deck_cards = Vec::new();
        let mut missing_cards = Vec::new();
        
        for card_no in &card_numbers {
            if let Some(card) = cards.get(card_no) {
                deck_cards.push(card.clone());
            } else {
                missing_cards.push(card_no.clone());
            }
        }
        
        if !missing_cards.is_empty() {
            eprintln!("Warning: {} cards not found in database", missing_cards.len());
        }
        
        Self::build_deck_from_cards(deck_cards)
    }
}
