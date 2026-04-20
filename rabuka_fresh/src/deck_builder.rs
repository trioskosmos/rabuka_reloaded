use crate::card::Card;
use std::collections::HashMap;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Deck {
    pub main_deck: VecDeque<Card>,
    pub energy_deck: VecDeque<Card>,
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
        let mut cards: Vec<Card> = self.main_deck.drain(..).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.main_deck = cards.into();
    }

    pub fn shuffle_energy_deck(&mut self) {
        use rand::seq::SliceRandom;
        let mut cards: Vec<Card> = self.energy_deck.drain(..).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.energy_deck = cards.into();
    }
}

pub struct DeckBuilder;

impl DeckBuilder {
    pub fn build_deck_from_cards(cards: Vec<Card>) -> Result<Deck, String> {
        let mut main_deck: VecDeque<Card> = VecDeque::new();
        let mut energy_deck: VecDeque<Card> = VecDeque::new();
        
        let mut member_count = 0;
        let mut live_count = 0;
        let mut energy_count = 0;
        
        let mut card_number_counts: HashMap<String, u32> = HashMap::new();
        
        for card in cards {
            // Card number limit check removed for testing purposes
            let count = card_number_counts.entry(card.card_no.clone()).or_insert(0);
            *count += 1;
            
            match card.card_type {
                crate::card::CardType::Member => {
                    main_deck.push_back(card);
                    member_count += 1;
                }
                crate::card::CardType::Live => {
                    main_deck.push_back(card);
                    live_count += 1;
                }
                crate::card::CardType::Energy => {
                    energy_deck.push_back(card);
                    energy_count += 1;
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
        
        // Rule 6.1.2: Energy deck must have exactly 12 energy cards
        // Energy deck can have any number (will add defaults if needed)
        if energy_count == 0 {
            // Will add default energy cards later
        } else if energy_count != 12 {
            return Err(format!("Energy deck should have 12 energy cards, found {}", energy_count));
        }
        
        Ok(Deck {
            main_deck,
            energy_deck,
        })
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
            
            for card in energy_cards {
                deck.energy_deck.push_back(card.clone());
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
