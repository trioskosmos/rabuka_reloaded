use crate::card::{CardDatabase, HeartColor, HeartIcon, Keyword};
use serde::{Serialize, Deserialize};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    Active,
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FaceState {
    FaceUp,
    FaceDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberArea {
    LeftSide,
    Center,
    RightSide,
}

impl std::fmt::Display for MemberArea {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MemberArea::LeftSide => write!(f, "left"),
            MemberArea::Center => write!(f, "center"),
            MemberArea::RightSide => write!(f, "right"),
        }
    }
}

impl std::str::FromStr for MemberArea {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(MemberArea::LeftSide),
            "center" => Ok(MemberArea::Center),
            "right" => Ok(MemberArea::RightSide),
            _ => Err(format!("Invalid area: {}", s)),
        }
    }
}

// CardInZone removed for performance - use i16 IDs directly
use crate::constants::{STAGE_SIZE, EMPTY_SLOT};

// Orientation and other state tracked in GameState modifiers

#[derive(Debug, Clone)]
pub struct Stage {
    // Rule 5.3: Stage - Where member cards are placed during Main Phase
    // Has three areas: Left Side, Center, Right Side
    // Use EMPTY_SLOT to indicate empty slot (like old engine)
    pub stage: [i16; STAGE_SIZE],  // [left_side, center, right_side]
}

impl Stage {
    pub fn new() -> Self {
        Stage {
            stage: [EMPTY_SLOT, EMPTY_SLOT, EMPTY_SLOT],  // [left_side, center, right_side], EMPTY_SLOT indicates empty
        }
    }

    /// Invariant check: stage must always have exactly STAGE_SIZE positions
    pub fn invariant(&self) -> bool {
        self.stage.len() == STAGE_SIZE
    }

    pub fn get_area(&self, area: MemberArea) -> Option<i16> {
        debug_assert!(self.invariant(), "Stage invariant violated");
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        let card_id = self.stage[index];
        if card_id == EMPTY_SLOT { None } else { Some(card_id) }
    }

    pub fn set_area(&mut self, area: MemberArea, card_id: i16) {
        debug_assert!(self.invariant(), "Stage invariant violated before set");
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        self.stage[index] = card_id;
        debug_assert!(self.invariant(), "Stage invariant violated after set");
    }

    pub fn clear_area(&mut self, area: MemberArea) {
        debug_assert!(self.invariant(), "Stage invariant violated before clear");
        let index = match area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        self.stage[index] = EMPTY_SLOT;
        debug_assert!(self.invariant(), "Stage invariant violated after clear");
    }

    pub fn member_in_position(&self, position: Keyword) -> bool {
        // Check if a member is in the specified position (Center, LeftSide, RightSide)
        let index = match position {
            Keyword::Center => 1,
            Keyword::LeftSide => 0,
            Keyword::RightSide => 2,
            _ => return false,
        };
        self.stage[index] != -1
    }

    pub fn position_change(&mut self, from_area: MemberArea, to_area: MemberArea) -> Result<i16, String> {
        // Rule 11.10: Position Change - move member to different area
        // Rule 11.10.2: If destination has a member, it swaps positions
        if from_area == to_area {
            return Err("Cannot move to same area".to_string());
        }

        let from_index = match from_area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };
        let to_index = match to_area {
            MemberArea::LeftSide => 0,
            MemberArea::Center => 1,
            MemberArea::RightSide => 2,
        };

        let card_id = self.stage[from_index];
        if card_id == -1 {
            return Err("No card in source area".to_string());
        }

        let dest_card_id = self.stage[to_index];

        if dest_card_id != -1 {
            // Swap: move destination card to source
            self.stage[from_index] = dest_card_id;
            self.stage[to_index] = card_id;
        } else {
            // Move: place source card in destination
            self.stage[to_index] = card_id;
            self.stage[from_index] = -1;
        }

        Ok(card_id)
    }

    pub fn formation_change(&mut self, assignments: Vec<(MemberArea, MemberArea)>) -> Result<(), String> {
        // Rule 11.11: Formation Change - move all members to specified areas
        // Rule 11.11.2: Cannot move multiple members to same area
        let mut target_areas = std::collections::HashSet::new();
        for (_, target) in &assignments {
            if !target_areas.insert(target) {
                return Err("Cannot move multiple members to same area".to_string());
            }
        }

        for (from, to) in assignments {
            self.position_change(from, to)?;
        }

        Ok(())
    }

    pub fn total_blades(&self, card_db: &CardDatabase) -> u32 {
        let mut total = 0;
        for &card_id in &self.stage {
            if card_id != -1 {
                if let Some(card) = card_db.get_card(card_id) {
                    total += card.blade;
                }
            }
        }
        total
    }

    pub fn can_place_card(&self, card_db: &CardDatabase, card_id: i16) -> bool {
        // Rule 8.2.2: Only member cards can be placed on the stage
        // Live cards cannot be played on main stage
        if let Some(card) = card_db.get_card(card_id) {
            !card.is_live()
        } else {
            false
        }
    }

    pub fn all_heart_icons(&self, card_db: &CardDatabase) -> Vec<HeartIcon> {
        let mut hearts = Vec::new();
        for &card_id in &self.stage {
            if card_id != -1 {
                if let Some(card) = card_db.get_card(card_id) {
                    if let Some(ref base_heart) = card.base_heart {
                        for (color, count) in &base_heart.hearts {
                            hearts.push(HeartIcon {
                                color: *color,
                                count: *count,
                            });
                        }
                    }
                }
            }
        }
        hearts
    }

    pub fn get_available_hearts(&self, card_db: &CardDatabase) -> crate::card::BaseHeart {
        // Rule 8.2.7: Calculate available hearts from stage for heart requirement checking
        let mut hearts = std::collections::HashMap::new();

        for &card_id in &self.stage {
            if card_id != -1 {
                if let Some(card) = card_db.get_card(card_id) {
                    if let Some(ref base_heart) = card.base_heart {
                        for (color, count) in &base_heart.hearts {
                            *hearts.entry(*color).or_insert(0) += count;
                        }
                    }
                }
            }
        }

        crate::card::BaseHeart { hearts }
    }
}

pub fn parse_heart_color(s: &str) -> HeartColor {
    match s {
        "heart00" => HeartColor::Heart00,
        "heart01" => HeartColor::Heart01,
        "heart02" => HeartColor::Heart02,
        "heart03" => HeartColor::Heart03,
        "heart04" => HeartColor::Heart04,
        "heart05" => HeartColor::Heart05,
        "heart06" => HeartColor::Heart06,
        "b_all" => HeartColor::BAll,
        "draw" => HeartColor::Draw,
        "score" => HeartColor::Score,
        _ => HeartColor::Heart00,
    }
}

pub fn parse_blade_color(s: &str) -> crate::card::BladeColor {
    match s {
        "桃" => crate::card::BladeColor::Peach,
        "赤" => crate::card::BladeColor::Red,
        "黄" => crate::card::BladeColor::Yellow,
        "緑" => crate::card::BladeColor::Green,
        "青" => crate::card::BladeColor::Blue,
        "紫" => crate::card::BladeColor::Purple,
        "all" | "ALL" => crate::card::BladeColor::All,
        _ => crate::card::BladeColor::All,
    }
}

#[derive(Debug, Clone)]
pub struct LiveCardZone {
    // Rule 5.2: Live Card Zone - Where member and live cards are placed during Live Card Set Phase
    pub cards: SmallVec<[i16; MAX_LIVE_CARDS]>,  // Card IDs - stack-allocated for up to MAX_LIVE_CARDS cards
}

impl LiveCardZone {
    pub fn new() -> Self {
        LiveCardZone { cards: SmallVec::new() }
    }

    pub fn can_place_card(&self, _card_db: &CardDatabase, _card_id: i16) -> bool {
        // Rule 8.2: During Live Card Set Phase, any card from hand can be placed in Live Card Zone
        true
    }

    pub fn add_card(&mut self, card_id: i16, _face_down: bool, _card_db: &CardDatabase) -> Result<(), String> {
        if !self.can_place_card(_card_db, card_id) {
            if let Some(card) = _card_db.get_card(card_id) {
                return Err(format!("Cannot place energy card '{}' in live card zone", card.name));
            }
            return Err("Cannot place unknown card in live card zone".to_string());
        }
        self.cards.push(card_id);
        Ok(())
    }

    pub fn get_live_cards(&self, card_db: &CardDatabase) -> Vec<i16> {
        self.cards.iter().filter(|&&card_id| {
            card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false)
        }).copied().collect()
    }

    pub fn clear(&mut self) -> SmallVec<[i16; 3]> {
        std::mem::take(&mut self.cards)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn calculate_live_score(&self, card_db: &CardDatabase, cheer_blade_heart_count: u32) -> u32 {
        // Rule 8.4.2.1: Add 1 to score for each cheer blade heart icon
        // Rule 9.2.1: Calculate total live score from cards in Live Card Zone
        // Score = sum of card scores + bonus from heart satisfaction + cheer blade hearts
        let mut total_score = 0;
        for card_id in &self.cards {
            if let Some(card) = card_db.get_card(*card_id) {
                total_score += card.get_score();
                // Heart satisfaction bonus: if card's need_heart is satisfied, add bonus
                // For now, add 1 bonus if card has need_heart and it's satisfied
                // This is a simplified implementation - full implementation would check actual heart satisfaction
                if let Some(ref need_heart) = card.need_heart {
                    if !need_heart.hearts.is_empty() {
                        total_score += 1; // Simplified: add 1 bonus for any heart requirement
                    }
                }
            }
        }
        total_score + cheer_blade_heart_count
    }

    pub fn get_top_card(&self) -> Option<i16> {
        // Rule 9.3.1: Get top card from Live Card Zone for victory determination
        self.cards.first().copied()
    }

    pub fn remove_top_card(&mut self) -> Option<i16> {
        // Rule 9.3.1: Remove top card from Live Card Zone
        if self.cards.is_empty() {
            None
        } else {
            self.cards.drain(..1).next()
        }
    }
}

use crate::constants::{MAX_ENERGY_CARDS, MAX_LIVE_CARDS};

#[derive(Debug, Clone)]
pub struct EnergyZone {
    // Rule 5.1: Energy Zone - Where energy cards are placed and activated
    pub cards: SmallVec<[i16; MAX_ENERGY_CARDS]>,  // Card IDs - stack-allocated for up to MAX_ENERGY_CARDS energy cards
    pub active_energy_count: usize,  // Simple count of active energy cards (simpler than HashSet)
}

impl EnergyZone {
    pub fn new() -> Self {
        EnergyZone { 
            cards: SmallVec::new(),
            active_energy_count: 0,
        }
    }

    pub fn can_place_card(&self, card_db: &CardDatabase, card_id: i16) -> bool {
        // Rule 7.2: Only energy cards can be placed in Energy Zone
        card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or_else(|| false)
    }

    pub fn add_card(&mut self, card_id: i16, card_db: &CardDatabase) -> Result<(), String> {
        // Rule 7.2: Only energy cards can be placed in Energy Zone
        if !card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or_else(|| false) {
            return Err("Only energy cards can be placed in Energy Zone".to_string());
        }
        
        // New energy cards start in Active state (Rule 7.4)
        self.cards.push(card_id);
        self.active_energy_count += 1;
        Ok(())
    }

    pub fn active_count(&self) -> usize {
        self.active_energy_count
    }

    pub fn total_blades(&self, card_db: &CardDatabase) -> u32 {
        // Rule 8.2.4: Calculate total blades from all energy cards in Energy Zone
        self.cards.iter().filter_map(|&card_id| card_db.get_card(card_id)).map(|c| c.blade).sum()
    }

    pub fn can_pay_blades(&self, card_db: &CardDatabase, amount: u32) -> bool {
        // Rule 8.2.5: Check if Energy Zone has enough blades to pay cost
        self.total_blades(card_db) >= amount
    }

    pub fn pay_blades(&mut self, card_db: &CardDatabase, amount: u32) -> bool {
        // Rule 8.2.6: Pay blades by deactivating energy cards
        if !self.can_pay_blades(card_db, amount) {
            return false;
        }

        // Decrement active energy count
        if self.active_energy_count >= amount as usize {
            self.active_energy_count -= amount as usize;
            true
        } else {
            false
        }
    }
    
    pub fn can_pay_energy(&self, amount: usize) -> bool {
        // Rule 5.9: Check if player has enough active energy cards
        self.active_energy_count >= amount
    }

    pub fn pay_energy_count(&mut self, amount: usize) -> bool {
        // Rule 5.9: Pay energy by decrementing active count
        if self.active_energy_count >= amount {
            self.active_energy_count -= amount;
            true
        } else {
            false
        }
    }
    
    pub fn pay_energy(&mut self, amount: usize) -> Result<(), String> {
        // Rule 5.9: Pay energy by decrementing active count
        // eprintln!("pay_energy called: amount={}, active_energy_count={}", amount, self.active_energy_count);
        
        if self.active_energy_count >= amount {
            self.active_energy_count -= amount;
            // eprintln!("pay_energy result: success, remaining active_energy_count={}", self.active_energy_count);
            Ok(())
        } else {
            // eprintln!("pay_energy result: failed, active_energy_count={}", self.active_energy_count);
            Err(format!("Could not pay {} energy (only {} active energy available, {} total energy cards)", amount, self.active_energy_count, self.cards.len()))
        }
    }

    pub fn activate_all(&mut self) {
        // Set all energy cards to active state
        self.active_energy_count = self.cards.len();
        // eprintln!("Activated {} energy cards (active_energy_count={})", self.cards.len(), self.active_energy_count);
    }
}

#[derive(Debug, Clone)]
pub struct MainDeck {
    pub cards: SmallVec<[i16; 60]>,  // Card IDs - stack-allocated for up to 60 cards
}

impl MainDeck {
    pub fn new() -> Self {
        MainDeck {
            cards: SmallVec::new(),
        }
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        self.cards.shuffle(&mut rand::thread_rng());
    }

    pub fn draw(&mut self) -> Option<i16> {
        if self.cards.is_empty() {
            None
        } else {
            Some(self.cards.remove(0))
        }
    }

    pub fn draw_multiple(&mut self, count: usize) -> Vec<i16> {
        (0..count).filter_map(|_| self.draw()).collect()
    }

    pub fn peek_top(&self, count: usize) -> Vec<i16> {
        self.cards.iter().take(count).copied().collect()
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
    pub cards: SmallVec<[i16; 20]>,  // Card IDs - stack-allocated for up to 20 energy cards
}

impl EnergyDeck {
    pub fn new() -> Self {
        EnergyDeck {
            cards: SmallVec::new(),
        }
    }

    pub fn draw(&mut self) -> Option<i16> {
        if self.cards.is_empty() {
            None
        } else {
            Some(self.cards.remove(0))
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct Hand {
    // Rule 5.4: Hand - Where cards drawn from main deck are held
    pub cards: SmallVec<[i16; 7]>,  // Card IDs - stack-allocated for up to 7 cards
}

impl Hand {
    pub fn new() -> Self {
        Hand { cards: SmallVec::new() }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    pub fn remove_card(&mut self, index: usize) -> Option<i16> {
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
    // Rule 5.5: Waitroom - Where used cards are placed
    // Used for refresh when main deck is empty
    pub cards: SmallVec<[i16; 30]>,  // Card IDs - stack-allocated for typical sizes
}

impl Waitroom {
    pub fn new() -> Self {
        Waitroom { cards: SmallVec::new() }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    pub fn take_all(&mut self) -> SmallVec<[i16; 30]> {
        std::mem::take(&mut self.cards)
    }

    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        self.cards.shuffle(&mut rand::thread_rng());
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn remove_card(&mut self, card_id: i16) {
        self.cards.retain(|c| *c != card_id);
    }
}

#[derive(Debug, Clone)]
pub struct SuccessLiveCardZone {
    // Rule 5.6: Success Live Card Zone - Where won live cards are placed
    // Victory condition: 3 cards in this zone
    pub cards: SmallVec<[i16; 3]>,  // Card IDs - stack-allocated for victory condition (max 3)
}

impl SuccessLiveCardZone {
    pub fn new() -> Self {
        SuccessLiveCardZone { cards: SmallVec::new() }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

#[derive(Debug, Clone)]
pub struct ExclusionZone {
    // Rule 5.7: Exclusion Zone - Where excluded cards are placed
    pub cards: SmallVec<[i16; 10]>,  // Card IDs - stack-allocated for up to 10 cards
}

impl ExclusionZone {
    pub fn new() -> Self {
        ExclusionZone { cards: SmallVec::new() }
    }

    pub fn add_card(&mut self, card_id: i16, _face_up: bool) {
        // Face state tracking moved to GameState modifiers
        self.cards.push(card_id);
    }
}

#[derive(Debug, Clone, Default)]
pub struct ResolutionZone {
    // Rule 5.8: Resolution Zone - Temporary holding area for cards being resolved
    pub cards: SmallVec<[i16; 10]>,  // Card IDs - stack-allocated for up to 10 cards
}

impl ResolutionZone {
    pub fn new() -> Self {
        ResolutionZone { cards: SmallVec::new() }
    }

    pub fn add_card(&mut self, card_id: i16) {
        self.cards.push(card_id);
    }

    pub fn clear(&mut self) -> SmallVec<[i16; 10]> {
        std::mem::take(&mut self.cards)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }
}
