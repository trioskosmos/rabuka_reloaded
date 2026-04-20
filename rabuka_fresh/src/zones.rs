use crate::card::{Card, HeartColor, HeartIcon, Keyword};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Active,
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaceState {
    FaceUp,
    FaceDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl CardInZone {
    pub fn total_blades(&self) -> u32 {
        // Rule 8.2.3: Calculate total blades from card and energy underneath
        let mut total = self.card.blade;
        for energy in &self.energy_underneath {
            total += energy.blade;
        }
        total
    }
    
    pub fn can_pay_cost(&self, cost: u32) -> bool {
        // Rule 8.2.4: Check if card has enough blades to pay cost
        self.total_blades() >= cost
    }
    
    pub fn pay_cost(&mut self, cost: u32) -> bool {
        // Rule 8.2.5: Pay cost by reducing blades
        // For now, this is a placeholder - actual blade payment needs more complex logic
        if self.can_pay_cost(cost) {
            // TODO: Implement actual blade payment logic
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct Stage {
    // Rule 5.3: Stage - Where member cards are placed during Main Phase
    // Has three areas: Left Side, Center, Right Side
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

    pub fn member_in_position(&self, position: Keyword) -> bool {
        // Check if a member is in the specified position (Center, LeftSide, RightSide)
        match position {
            Keyword::Center => self.center.is_some(),
            Keyword::LeftSide => self.left_side.is_some(),
            Keyword::RightSide => self.right_side.is_some(),
            _ => false,
        }
    }

    pub fn position_change(&mut self, from_area: MemberArea, to_area: MemberArea) -> Result<(), String> {
        // Rule 11.10: Position Change - move member to different area
        // Rule 11.10.2: If destination has a member, it swaps positions
        if from_area == to_area {
            return Err("Cannot move to same area".to_string());
        }
        
        // Take the card from source area
        let card = match from_area {
            MemberArea::LeftSide => self.left_side.take(),
            MemberArea::Center => self.center.take(),
            MemberArea::RightSide => self.right_side.take(),
        };
        
        if card.is_none() {
            return Err("No card in source area".to_string());
        }
        
        let card = card.unwrap();
        
        // Check if destination has a card
        let has_dest = match to_area {
            MemberArea::LeftSide => self.left_side.is_some(),
            MemberArea::Center => self.center.is_some(),
            MemberArea::RightSide => self.right_side.is_some(),
        };
        
        if has_dest {
            // Swap: move destination card to source
            let dest_card = match to_area {
                MemberArea::LeftSide => self.left_side.take(),
                MemberArea::Center => self.center.take(),
                MemberArea::RightSide => self.right_side.take(),
            };
            
            // Place source card in destination
            match to_area {
                MemberArea::LeftSide => self.left_side = Some(card),
                MemberArea::Center => self.center = Some(card),
                MemberArea::RightSide => self.right_side = Some(card),
            }
            
            // Place destination card in source
            match from_area {
                MemberArea::LeftSide => self.left_side = dest_card,
                MemberArea::Center => self.center = dest_card,
                MemberArea::RightSide => self.right_side = dest_card,
            }
        } else {
            // Move: place source card in destination
            match to_area {
                MemberArea::LeftSide => self.left_side = Some(card),
                MemberArea::Center => self.center = Some(card),
                MemberArea::RightSide => self.right_side = Some(card),
            }
        }
        
        Ok(())
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
        // Rule 8.2.2: Only member cards can be placed on the stage
        // Live cards cannot be played on main stage
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
    
    pub fn get_available_hearts(&self) -> crate::card::BaseHeart {
        // Rule 8.2.7: Calculate available hearts from stage for heart requirement checking
        let mut hearts = std::collections::HashMap::new();
        
        if let Some(ref card) = self.left_side {
            if let Some(ref base_heart) = card.card.base_heart {
                for (color, count) in &base_heart.hearts {
                    *hearts.entry(color.clone()).or_insert(0) += count;
                }
            }
        }
        if let Some(ref card) = self.center {
            if let Some(ref base_heart) = card.card.base_heart {
                for (color, count) in &base_heart.hearts {
                    *hearts.entry(color.clone()).or_insert(0) += count;
                }
            }
        }
        if let Some(ref card) = self.right_side {
            if let Some(ref base_heart) = card.card.base_heart {
                for (color, count) in &base_heart.hearts {
                    *hearts.entry(color.clone()).or_insert(0) += count;
                }
            }
        }
        
        crate::card::BaseHeart { hearts }
    }
    
    pub fn activate_all_cards(&mut self) {
        // Rule 7.1: Change orientation of all stage cards from Wait to Active
        if let Some(ref mut card) = self.left_side {
            card.orientation = Some(Orientation::Active);
        }
        if let Some(ref mut card) = self.center {
            card.orientation = Some(Orientation::Active);
        }
        if let Some(ref mut card) = self.right_side {
            card.orientation = Some(Orientation::Active);
        }
    }

    // ============== STATE CHANGE METHODS ==============

    /// Set orientation of card in specific position
    pub fn set_card_orientation(&mut self, area: MemberArea, orientation: Orientation) -> Result<(), String> {
        let mut card_in_zone = match area {
            MemberArea::LeftSide => self.left_side.as_mut(),
            MemberArea::Center => self.center.as_mut(),
            MemberArea::RightSide => self.right_side.as_mut(),
        };

        if let Some(ref mut card) = card_in_zone {
            card.orientation = Some(orientation);
            Ok(())
        } else {
            Err(format!("No card in {:?}", area))
        }
    }

    /// Set face state of card in specific position
    pub fn set_card_face_state(&mut self, area: MemberArea, face_state: FaceState) -> Result<(), String> {
        let mut card_in_zone = match area {
            MemberArea::LeftSide => self.left_side.as_mut(),
            MemberArea::Center => self.center.as_mut(),
            MemberArea::RightSide => self.right_side.as_mut(),
        };

        if let Some(ref mut card) = card_in_zone {
            card.face_state = face_state;
            Ok(())
        } else {
            Err(format!("No card in {:?}", area))
        }
    }

    /// Set orientation of all stage cards
    pub fn set_all_orientation(&mut self, orientation: Orientation) {
        if let Some(ref mut card) = self.left_side {
            card.orientation = Some(orientation.clone());
        }
        if let Some(ref mut card) = self.center {
            card.orientation = Some(orientation.clone());
        }
        if let Some(ref mut card) = self.right_side {
            card.orientation = Some(orientation.clone());
        }
    }

    fn parse_heart_color(s: &str) -> HeartColor {
        match s {
            "heart00" => HeartColor::Heart00,  // Wildcard - can substitute for heart01-heart06
            "heart01" => HeartColor::Heart01,
            "heart02" => HeartColor::Heart02,
            "heart03" => HeartColor::Heart03,
            "heart04" => HeartColor::Heart04,
            "heart05" => HeartColor::Heart05,
            "heart06" => HeartColor::Heart06,
            _ => HeartColor::Heart00,  // Default to wildcard
        }
    }
}

#[derive(Debug, Clone)]
pub struct LiveCardZone {
    // Rule 5.2: Live Card Zone - Where member and live cards are placed during Live Card Set Phase
    pub cards: Vec<Card>,
}

impl LiveCardZone {
    pub fn new() -> Self {
        LiveCardZone { cards: Vec::new() }
    }

    pub fn can_place_card(&self, card: &Card) -> bool {
        // Rule 9.1: During Live Card Set Phase, member and live cards can be placed in Live Card Zone
        // Energy cards cannot be placed here
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
    
    pub fn calculate_live_score(&self, cheer_blade_heart_count: u32) -> u32 {
        // Rule 8.4.2.1: Add 1 to score for each cheer blade heart icon
        // Rule 9.2.1: Calculate total live score from cards in Live Card Zone
        // Score = sum of card scores + bonus from heart satisfaction + cheer blade hearts
        let mut total_score = 0;
        for card in &self.cards {
            total_score += card.get_score();
            // TODO: Add heart satisfaction bonus calculation
        }
        total_score + cheer_blade_heart_count
    }
    
    pub fn get_top_card(&self) -> Option<&Card> {
        // Rule 9.3.1: Get top card from Live Card Zone for victory determination
        self.cards.first()
    }
    
    pub fn remove_top_card(&mut self) -> Option<Card> {
        // Rule 9.3.1: Remove top card from Live Card Zone
        if self.cards.is_empty() {
            None
        } else {
            Some(self.cards.remove(0))
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnergyZone {
    // Rule 5.1: Energy Zone - Where energy cards are placed and activated
    pub cards: Vec<CardInZone>,
}

impl EnergyZone {
    pub fn new() -> Self {
        EnergyZone { cards: Vec::new() }
    }

    pub fn can_place_card(&self, card: &Card) -> bool {
        // Rule 7.2: Only energy cards can be placed in Energy Zone
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
        self.cards.iter().filter(|c| c.orientation == Some(Orientation::Active)).count()
    }
    
    pub fn total_blades(&self) -> u32 {
        // Rule 8.2.4: Calculate total blades from all energy cards in Energy Zone
        self.cards.iter().map(|c| c.total_blades()).sum()
    }
    
    pub fn can_pay_blades(&self, amount: u32) -> bool {
        // Rule 8.2.5: Check if Energy Zone has enough blades to pay cost
        self.total_blades() >= amount
    }
    
    pub fn pay_blades(&mut self, amount: u32) -> bool {
        // Rule 8.2.6: Pay blades by deactivating energy cards
        // This is a simplified version - actual blade payment needs more complex logic
        if !self.can_pay_blades(amount) {
            return false;
        }
        
        // TODO: Implement actual blade payment logic
        // For now, just return true if we have enough blades
        true
    }

    pub fn activate_all(&mut self) {
        for card in &mut self.cards {
            card.orientation = Some(Orientation::Active);
        }
    }

    // ============== STATE CHANGE METHODS ==============

    /// Set orientation of card at specific index
    pub fn set_card_orientation(&mut self, index: usize, orientation: Orientation) -> Result<(), String> {
        if index >= self.cards.len() {
            return Err(format!("Invalid energy index: {}", index));
        }
        self.cards[index].orientation = Some(orientation);
        Ok(())
    }

    /// Set face state of card at specific index
    pub fn set_card_face_state(&mut self, index: usize, face_state: FaceState) -> Result<(), String> {
        if index >= self.cards.len() {
            return Err(format!("Invalid energy index: {}", index));
        }
        self.cards[index].face_state = face_state;
        Ok(())
    }

    /// Set orientation of all energy cards
    pub fn set_all_orientation(&mut self, orientation: Orientation) {
        for card in &mut self.cards {
            card.orientation = Some(orientation.clone());
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
    // Rule 5.4: Hand - Where cards drawn from main deck are held
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
    // Rule 5.5: Waitroom - Where used cards are placed
    // Used for refresh when main deck is empty
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
    // Rule 5.6: Success Live Card Zone - Where won live cards are placed
    // Victory condition: 3 cards in this zone
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
    // Rule 5.7: Exclusion Zone - Where excluded cards are placed
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

#[derive(Debug, Clone, Default)]
pub struct ResolutionZone {
    // Rule 5.8: Resolution Zone - Temporary holding area for cards being resolved
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
