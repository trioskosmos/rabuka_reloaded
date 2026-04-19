use crate::card::{Card, HeartColor, HeartIcon};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Orientation {
    Active,
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaceState {
    FaceUp,
    FaceDown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberArea {
    LeftSide,
    Center,
    RightSide,
}

#[derive(Debug, Clone)]
pub struct CardInZone {
    pub card: Card,
    pub orientation: Option<Orientation>,
    pub face_state: FaceState,
    pub energy_underneath: Vec<Card>,
}

#[derive(Debug, Clone)]
pub struct Stage {
    pub left_side: Option<CardInZone>,
    pub center: Option<CardInZone>,
    pub right_side: Option<CardInZone>,
}

impl Stage {
    pub fn new() -> Self {
        Stage {
            left_side: None,
            center: None,
            right_side: None,
        }
    }

    pub fn get_area(&self, area: MemberArea) -> Option<&CardInZone> {
        match area {
            MemberArea::LeftSide => self.left_side.as_ref(),
            MemberArea::Center => self.center.as_ref(),
            MemberArea::RightSide => self.right_side.as_ref(),
        }
    }

    pub fn get_area_mut(&mut self, area: MemberArea) -> Option<&mut CardInZone> {
        match area {
            MemberArea::LeftSide => self.left_side.as_mut(),
            MemberArea::Center => self.center.as_mut(),
            MemberArea::RightSide => self.right_side.as_mut(),
        }
    }

    pub fn total_blades(&self) -> u32 {
        let mut total = 0;
        if let Some(ref card) = self.left_side {
            total += card.card.total_blades();
        }
        if let Some(ref card) = self.center {
            total += card.card.total_blades();
        }
        if let Some(ref card) = self.right_side {
            total += card.card.total_blades();
        }
        total
    }

    pub fn can_place_card(&self, card: &Card) -> bool {
        // Rule: Live cards cannot be played on main stage
        !card.is_live()
    }

    pub fn set_area(&mut self, area: MemberArea, card: CardInZone) -> Result<(), String> {
        if !self.can_place_card(&card.card) {
            return Err(format!("Cannot place live card '{}' on main stage", card.card.name));
        }
        match area {
            MemberArea::LeftSide => self.left_side = Some(card),
            MemberArea::Center => self.center = Some(card),
            MemberArea::RightSide => self.right_side = Some(card),
        }
        Ok(())
    }

    pub fn clear_area(&mut self, area: MemberArea) -> Option<CardInZone> {
        match area {
            MemberArea::LeftSide => self.left_side.take(),
            MemberArea::Center => self.center.take(),
            MemberArea::RightSide => self.right_side.take(),
        }
    }

    pub fn all_heart_icons(&self) -> Vec<HeartIcon> {
        let mut hearts = Vec::new();
        if let Some(ref card) = self.left_side {
            if let Some(ref base_heart) = card.card.base_heart {
                for (color, count) in &base_heart.hearts {
                    hearts.push(HeartIcon {
                        color: Self::parse_heart_color(color),
                        count: *count,
                    });
                }
            }
        }
        if let Some(ref card) = self.center {
            if let Some(ref base_heart) = card.card.base_heart {
                for (color, count) in &base_heart.hearts {
                    hearts.push(HeartIcon {
                        color: Self::parse_heart_color(color),
                        count: *count,
                    });
                }
            }
        }
        if let Some(ref card) = self.right_side {
            if let Some(ref base_heart) = card.card.base_heart {
                for (color, count) in &base_heart.hearts {
                    hearts.push(HeartIcon {
                        color: Self::parse_heart_color(color),
                        count: *count,
                    });
                }
            }
        }
        hearts
    }

    fn parse_heart_color(s: &str) -> HeartColor {
        match s {
            "heart01" => HeartColor::Pink,
            "heart02" => HeartColor::Red,
            "heart03" => HeartColor::Yellow,
            "heart04" => HeartColor::Green,
            "heart05" => HeartColor::Blue,
            "heart06" => HeartColor::Purple,
            "heart00" => HeartColor::Wild,
            _ => HeartColor::Wild,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiveCardZone {
    pub cards: Vec<Card>,
}

impl LiveCardZone {
    pub fn new() -> Self {
        LiveCardZone { cards: Vec::new() }
    }

    pub fn can_place_card(&self, card: &Card) -> bool {
        // Rule: Only member and live cards can be placed in live card zone
        card.is_member() || card.is_live()
    }

    pub fn add_card(&mut self, card: Card, _face_down: bool) -> Result<(), String> {
        if !self.can_place_card(&card) {
            return Err(format!("Cannot place energy card '{}' in live card zone", card.name));
        }
        self.cards.push(card);
        Ok(())
    }

    pub fn get_live_cards(&self) -> Vec<&Card> {
        self.cards.iter().filter(|c| c.is_live()).collect()
    }

    pub fn clear(&mut self) -> Vec<Card> {
        std::mem::take(&mut self.cards)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Debug, Clone)]
pub struct EnergyZone {
    pub cards: Vec<CardInZone>,
}

impl EnergyZone {
    pub fn new() -> Self {
        EnergyZone { cards: Vec::new() }
    }

    pub fn can_place_card(&self, card: &Card) -> bool {
        // Rule: Only energy cards can be placed in energy zone
        card.is_energy()
    }

    pub fn add_card(&mut self, card: CardInZone) -> Result<(), String> {
        if !self.can_place_card(&card.card) {
            return Err(format!("Cannot place {} card '{}' in energy zone", 
                if card.card.is_live() { "live" } else { "member" }, card.card.name));
        }
        self.cards.push(card);
        Ok(())
    }

    pub fn active_count(&self) -> usize {
        self.cards
            .iter()
            .filter(|c| c.orientation == Some(Orientation::Active))
            .count()
    }

    pub fn activate_all(&mut self) {
        for card in &mut self.cards {
            card.orientation = Some(Orientation::Active);
        }
    }
}

#[derive(Debug, Clone)]
pub struct MainDeck {
    pub cards: VecDeque<Card>,
}

impl MainDeck {
    pub fn new() -> Self {
        MainDeck {
            cards: VecDeque::new(),
        }
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        let mut cards: Vec<Card> = self.cards.drain(..).collect();
        cards.shuffle(&mut rand::thread_rng());
        self.cards = cards.into();
    }

    pub fn draw(&mut self) -> Option<Card> {
        self.cards.pop_front()
    }

    pub fn draw_multiple(&mut self, count: usize) -> Vec<Card> {
        (0..count).filter_map(|_| self.draw()).collect()
    }

    pub fn peek_top(&self, count: usize) -> Vec<&Card> {
        self.cards.iter().take(count).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Debug, Clone)]
pub struct EnergyDeck {
    pub cards: VecDeque<Card>,
}

impl EnergyDeck {
    pub fn new() -> Self {
        EnergyDeck {
            cards: VecDeque::new(),
        }
    }

    pub fn draw(&mut self) -> Option<Card> {
        self.cards.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Hand {
    pub cards: Vec<Card>,
}

impl Hand {
    pub fn new() -> Self {
        Hand { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn remove_card(&mut self, index: usize) -> Option<Card> {
        if index < self.cards.len() {
            Some(self.cards.remove(index))
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Waitroom {
    pub cards: Vec<Card>,
}

impl Waitroom {
    pub fn new() -> Self {
        Waitroom { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn take_all(&mut self) -> Vec<Card> {
        std::mem::take(&mut self.cards)
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        self.cards.shuffle(&mut rand::thread_rng());
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Debug, Clone)]
pub struct SuccessLiveCardZone {
    pub cards: Vec<Card>,
}

impl SuccessLiveCardZone {
    pub fn new() -> Self {
        SuccessLiveCardZone { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Debug, Clone)]
pub struct ExclusionZone {
    pub cards: Vec<CardInZone>,
}

impl ExclusionZone {
    pub fn new() -> Self {
        ExclusionZone { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card, face_up: bool) {
        let face_state = if face_up {
            FaceState::FaceUp
        } else {
            FaceState::FaceDown
        };
        self.cards.push(CardInZone {
            card,
            orientation: None,
            face_state,
            energy_underneath: Vec::new(),
        });
    }
}

#[derive(Debug, Clone)]
pub struct ResolutionZone {
    pub cards: Vec<Card>,
}

impl ResolutionZone {
    pub fn new() -> Self {
        ResolutionZone { cards: Vec::new() }
    }

    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    pub fn clear(&mut self) -> Vec<Card> {
        std::mem::take(&mut self.cards)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}
