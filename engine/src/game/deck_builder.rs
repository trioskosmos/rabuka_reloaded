use crate::card::CardDatabase;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Deck {
    pub main_deck: VecDeque<i16>,   // Card IDs
    pub energy_deck: VecDeque<i16>, // Card IDs
}

impl Deck {
    pub fn shuffle_main_deck(&mut self) {
        use rand::seq::SliceRandom;
        let mut cards: Vec<i16> = self.main_deck.drain(..).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.main_deck = cards.into();
    }

    pub fn shuffle_energy_deck(&mut self) {
        use rand::seq::SliceRandom;
        let mut cards: Vec<i16> = self.energy_deck.drain(..).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.energy_deck = cards.into();
    }
}

pub struct DeckBuilder;

impl DeckBuilder {
    pub fn build_deck_from_database(
        card_db: &mut Arc<CardDatabase>,
        card_numbers: Vec<String>,
    ) -> Result<Deck, String> {
        let mut main_deck: VecDeque<i16> = VecDeque::new();
        let mut energy_deck: VecDeque<i16> = VecDeque::new();

        let mut member_count = 0;
        let mut live_count = 0;
        let mut energy_count = 0;
        let mut missing_cards: Vec<String> = Vec::new();

        for card_no in card_numbers {
            // Try to find card template ID
            let template_id = card_db.get_card_id(&card_no);

            if let Some(template_id) = template_id {
                // Create a unique instance ID for this card copy
                let card_id = Arc::make_mut(card_db).create_copy(template_id);
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
            } else {
                missing_cards.push(card_no.clone());
            }
        }

        // Log missing cards for debugging
        if !missing_cards.is_empty() {
            eprintln!(
                "Warning: {} cards not found in database:",
                missing_cards.len()
            );
            for card_no in &missing_cards {
                eprintln!("  - {}", card_no);
            }
        }

        // Validate deck composition with priority on 12 live + 48 member
        let total_main = member_count + live_count;
        if total_main < 60 {
            eprintln!(
                "Warning: Main deck has {} cards (expected 60): {} member + {} live",
                total_main, member_count, live_count
            );
        }

        if live_count < 12 {
            eprintln!(
                "Warning: Main deck has {} live cards (expected 12)",
                live_count
            );
        }

        if member_count < 48 {
            eprintln!(
                "Warning: Main deck has {} member cards (expected 48)",
                member_count
            );
        }

        if energy_count != 12 {
            eprintln!(
                "Warning: Energy deck has {} energy cards (expected 12)",
                energy_count
            );
        }

        Ok(Deck {
            main_deck,
            energy_deck,
        })
    }

    pub fn add_default_energy_cards_from_database(
        deck: &mut Deck,
        card_db: &mut Arc<CardDatabase>,
    ) -> Result<(), String> {
        let current_count = deck.energy_deck.len();
        let needed = if current_count < 12 {
            12 - current_count
        } else {
            0
        };

        if needed > 0 {
            // Find a template energy card
            let template_energy_id = card_db.cards.iter()
                .find(|(_, card)| card.is_energy())
                .map(|(id, _)| *id);

            if let Some(template_id) = template_energy_id {
                for _ in 0..needed {
                    let card_id = Arc::make_mut(card_db).create_copy(template_id);
                    deck.energy_deck.push_back(card_id);
                }
            } else {
                return Err("No energy cards found in database".to_string());
            }
        }
        Ok(())
    }
}
