use crate::zones::{
    EnergyDeck, EnergyZone, ExclusionZone, Hand, LiveCardZone, MainDeck, Stage,
    SuccessLiveCardZone, Waitroom,
};
use crate::card::Card;
use std::collections::VecDeque;
use rand::prelude::SliceRandom;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub is_first_attacker: bool,
    pub stage: Stage,
    pub live_card_zone: LiveCardZone,
    pub energy_zone: EnergyZone,
    pub main_deck: MainDeck,
    pub energy_deck: EnergyDeck,
    pub hand: Hand,
    pub waitroom: Waitroom,
    pub success_live_card_zone: SuccessLiveCardZone,
    pub exclusion_zone: ExclusionZone,
}

impl Player {
    pub fn new(id: String, name: String, is_first_attacker: bool) -> Self {
        Player {
            id,
            name,
            is_first_attacker,
            stage: Stage::new(),
            live_card_zone: LiveCardZone::new(),
            energy_zone: EnergyZone::new(),
            main_deck: MainDeck::new(),
            energy_deck: EnergyDeck::new(),
            hand: Hand::new(),
            waitroom: Waitroom::new(),
            success_live_card_zone: SuccessLiveCardZone::new(),
            exclusion_zone: ExclusionZone::new(),
        }
    }

    pub fn set_main_deck(&mut self, cards: VecDeque<Card>) {
        self.main_deck.cards = cards;
    }

    pub fn set_energy_deck(&mut self, cards: VecDeque<Card>) {
        self.energy_deck.cards = cards;
    }

    pub fn can_live(&self) -> bool {
        !self.live_card_zone.get_live_cards().is_empty()
    }

    pub fn total_live_score(&self) -> u32 {
        self.live_card_zone
            .get_live_cards()
            .iter()
            .map(|c| c.get_score())
            .sum()
    }

    pub fn has_victory(&self) -> bool {
        self.success_live_card_zone.len() >= 3
    }

    pub fn activate_all_energy(&mut self) {
        self.energy_zone.activate_all();
    }

    pub fn draw_card(&mut self) -> Option<Card> {
        self.main_deck.draw().map(|card| {
            self.hand.add_card(card.clone());
            card
        })
    }

    pub fn draw_energy(&mut self) -> Option<Card> {
        self.energy_deck.draw().map(|card| {
            let card_in_zone = crate::zones::CardInZone {
                card: card.clone(),
                orientation: Some(crate::zones::Orientation::Active),
                face_state: crate::zones::FaceState::FaceUp,
                energy_underneath: Vec::new(),
            };
            if let Err(e) = self.energy_zone.add_card(card_in_zone) {
                eprintln!("Error adding energy card: {}", e);
            }
            card
        })
    }

    pub fn refresh(&mut self) {
        // Rule 10.2: Refresh when main deck is empty and waitroom has cards
        if self.main_deck.is_empty() && !self.waitroom.cards.is_empty() {
            let mut waitroom_cards = self.waitroom.take_all();
            waitroom_cards.shuffle(&mut rand::thread_rng());
            for card in waitroom_cards {
                self.main_deck.cards.push_back(card);
            }
        }
    }
}
