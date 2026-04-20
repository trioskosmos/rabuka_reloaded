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
    
    pub fn move_card_from_hand_to_stage(&mut self, hand_index: usize, stage_area: crate::zones::MemberArea) -> Result<(), String> {
        // Rule 8.2: Main Phase - Play member card from hand to stage
        if hand_index >= self.hand.cards.len() {
            return Err("Invalid hand index".to_string());
        }
        
        let card = self.hand.cards.remove(hand_index);
        
        if !card.is_member() {
            self.hand.cards.insert(hand_index, card);
            return Err("Only member cards can be placed on stage".to_string());
        }
        
        // Rule 9.6.2.3.1: Cost is equal to the card's cost value in energy
        let card_cost = card.cost.unwrap_or(0);
        
        // Rule 9.6.2.3: Determine cost and pay all costs
        let mut cost_to_pay = card_cost;
        
        // Rule 9.6.2.3.2: Baton touch - if 1+ energy to pay, can send member from target area to waitroom instead
        // Note: Baton touch sends member from the TARGET area (where you're playing the new member)
        if cost_to_pay > 0 && self.energy_zone.cards.len() >= 1 {
            // Check if there's a member in the target area to baton touch
            let member_cost = if let Some(existing_member) = self.stage.get_area(stage_area) {
                // Clone the member card before clearing the area
                let member_card = existing_member.card.clone();
                let cost = existing_member.card.cost.unwrap_or(1);
                
                // Clear the target area since member will be sent to waitroom
                match stage_area {
                    crate::zones::MemberArea::LeftSide => self.stage.left_side = None,
                    crate::zones::MemberArea::Center => self.stage.center = None,
                    crate::zones::MemberArea::RightSide => self.stage.right_side = None,
                }
                
                // Send member to waitroom
                self.waitroom.cards.push(member_card);
                cost
            } else {
                0
            };
            
            // Rule 9.6.2.3.2: Reduce cost by member's cost
            cost_to_pay = cost_to_pay.saturating_sub(member_cost);
        }
        
        // Rule 9.6.2.3.1: Pay energy equal to cost
        if cost_to_pay > 0 {
            // Check if player has enough active energy
            let active_energy_count = self.energy_zone.cards.iter()
                .filter(|c| c.orientation == Some(crate::zones::Orientation::Active))
                .count() as u32;
            
            if active_energy_count < cost_to_pay {
                self.hand.cards.insert(hand_index, card);
                return Err(format!("Not enough energy to pay cost: need {}, have {}", cost_to_pay, active_energy_count));
            }
            
            // Pay the cost by activating energy cards
            let mut paid = 0;
            for energy_card in self.energy_zone.cards.iter_mut() {
                if paid >= cost_to_pay {
                    break;
                }
                if energy_card.orientation == Some(crate::zones::Orientation::Active) {
                    energy_card.orientation = Some(crate::zones::Orientation::Wait);
                    paid += 1;
                }
            }
        }
        
        let card_in_zone = crate::zones::CardInZone {
            card: card,
            orientation: Some(crate::zones::Orientation::Wait),
            face_state: crate::zones::FaceState::FaceUp,
            energy_underneath: Vec::new(),
        };
        
        match stage_area {
            crate::zones::MemberArea::LeftSide => {
                if self.stage.left_side.is_some() {
                    self.hand.cards.insert(hand_index, card_in_zone.card);
                    return Err("Left side already occupied".to_string());
                }
                self.stage.left_side = Some(card_in_zone);
            }
            crate::zones::MemberArea::Center => {
                if self.stage.center.is_some() {
                    self.hand.cards.insert(hand_index, card_in_zone.card);
                    return Err("Center already occupied".to_string());
                }
                self.stage.center = Some(card_in_zone);
            }
            crate::zones::MemberArea::RightSide => {
                if self.stage.right_side.is_some() {
                    self.hand.cards.insert(hand_index, card_in_zone.card);
                    return Err("Right side already occupied".to_string());
                }
                self.stage.right_side = Some(card_in_zone);
            }
        }
        
        // Rule 9.6.2.3.2.1: If baton touch performed, trigger 'baton touch' event
        // TODO: Implement baton touch event triggering
        
        Ok(())
    }
    
    pub fn move_card_from_hand_to_energy_zone(&mut self, hand_index: usize) -> Result<(), String> {
        // Rule 7.2: Energy Phase - Play energy card from hand to energy zone
        if hand_index >= self.hand.cards.len() {
            return Err("Invalid hand index".to_string());
        }
        
        let card = self.hand.cards.remove(hand_index);
        
        if !card.is_energy() {
            self.hand.cards.insert(hand_index, card);
            return Err("Only energy cards can be placed in energy zone".to_string());
        }
        
        let card_in_zone = crate::zones::CardInZone {
            card: card,
            orientation: Some(crate::zones::Orientation::Wait),
            face_state: crate::zones::FaceState::FaceUp,
            energy_underneath: Vec::new(),
        };
        
        self.energy_zone.add_card(card_in_zone)?;
        Ok(())
    }
    
    pub fn move_card_from_hand_to_live_zone(&mut self, hand_index: usize) -> Result<(), String> {
        // Rule 9.1: Live Card Set Phase - Place card from hand to live card zone
        if hand_index >= self.hand.cards.len() {
            return Err("Invalid hand index".to_string());
        }
        
        let card = self.hand.cards.remove(hand_index);
        
        if !self.live_card_zone.can_place_card(&card) {
            self.hand.cards.insert(hand_index, card);
            return Err("Card cannot be placed in live card zone".to_string());
        }
        
        self.live_card_zone.add_card(card, false)?;
        Ok(())
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
        // Rule 11.1: Victory Condition - Player wins when they have 3 cards in Success Live Card Zone
        self.success_live_card_zone.len() >= 3
    }

    pub fn activate_all_energy(&mut self) {
        self.energy_zone.activate_all();
    }

    pub fn draw_card(&mut self) -> Option<Card> {
        // Rule 8.1: Draw Phase - Active player draws 1 card from main deck to hand
        self.main_deck.draw().map(|card| {
            self.hand.add_card(card.clone());
            card
        })
    }

    pub fn draw_energy(&mut self) -> Option<Card> {
        self.energy_deck.draw().map(|card| {
            let card_in_zone = crate::zones::CardInZone {
                card: card.clone(),
                orientation: Some(crate::zones::Orientation::Wait),
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
        // Rule 10.2.1: Condition - main deck is empty AND waitroom has cards
        // Rule 10.2.2: Shuffle waitroom cards and place them on top of main deck
        // Rule 10.2.3: This happens automatically during check timing
        if self.main_deck.is_empty() && !self.waitroom.cards.is_empty() {
            let mut waitroom_cards = self.waitroom.take_all();
            waitroom_cards.shuffle(&mut rand::thread_rng());
            for card in waitroom_cards {
                self.main_deck.cards.push_back(card);
            }
        }
        
        // Rule 10.2.2.2: Refresh when looking at top cards and deck is too small
        // If deck has fewer cards than needed to look at, refresh first
        // This would be called before look_at_top operations
        
        // Energy deck does NOT refresh like main deck
        // Energy cards are recycled via Rule 10.5.3 (energy without member above -> energy deck)
        // and Rule 10.5.4 (energy going to waitroom -> energy deck instead)
        // These are handled in check_timing/check_invalid_cards in turn.rs
    }
    
    pub fn refresh_if_needed(&mut self, needed: usize) {
        // Rule 10.2.2.2: Refresh when looking at top cards and deck is too small
        if self.main_deck.cards.len() < needed && !self.waitroom.cards.is_empty() {
            let mut waitroom_cards = self.waitroom.take_all();
            waitroom_cards.shuffle(&mut rand::thread_rng());
            for card in waitroom_cards {
                self.main_deck.cards.push_back(card);
            }
        }
    }

    // ============== SPECIFIC ACTIONS BASED ON RULES.TXT ==============

    /// Rule 5.5: シャッフルする (Shuffle)
    /// Randomly change card order in a specified zone
    /// 5.5.1.1: If zone name is specified, shuffle all cards in that zone
    /// 5.5.1.2: If zone has 0 or 1 cards, shuffle is considered performed even though order doesn't change
    pub fn shuffle_zone(&mut self, zone: &str) -> Result<(), String> {
        match zone {
            "deck" | "main_deck" | "メインデッキ" => {
                self.main_deck.shuffle();
                Ok(())
            }
            "energy_deck" | "エネルギーデッキ" => {
                let mut cards: Vec<_> = self.energy_deck.cards.drain(..).collect();
                cards.shuffle(&mut rand::thread_rng());
                self.energy_deck.cards = cards.into();
                Ok(())
            }
            "waitroom" | "控え室" => {
                self.waitroom.shuffle();
                Ok(())
            }
            _ => Err(format!("Unknown zone for shuffle: {}", zone)),
        }
    }

    /// Rule 5.7: 上から見る (Look at top)
    /// Look at top N cards from main deck
    /// 5.7.1: Look at top (number) cards from main deck
    /// 5.7.2: Look at top (number) cards until (up to) - player can stop early
    pub fn look_at_top(&self, count: usize, _up_to: bool) -> Vec<&crate::card::Card> {
        // Rule 5.7.2.1: If count is 0 or less, end the instruction
        if count == 0 {
            return Vec::new();
        }
        
        // Rule 10.2.2.2: If deck has fewer cards than needed, refresh first would be called
        // (This is handled by the caller via refresh_if_needed)
        
        // Return top count cards (or fewer if deck is smaller)
        self.main_deck.peek_top(count)
    }

    /// Rule 5.8: 入れ替える (Swap)
    /// Exchange two cards between areas
    /// 5.8.1: Move first card to second card's area, and second card to first card's area simultaneously
    /// 5.8.2: If either card cannot move, the swap does not execute
    pub fn swap_cards(
        &mut self,
        from_area: crate::zones::MemberArea,
        to_area: crate::zones::MemberArea,
    ) -> Result<(), String> {
        // Use the existing Stage::position_change method which handles swapping
        self.stage.position_change(from_area, to_area)
    }

    /// Rule 5.9: を支払う (Pay energy)
    /// Change active energy cards to wait state
    /// 5.9.1: Change 1 active energy in energy zone to wait state
    /// 5.9.1.1: Multiple icons = pay that many energy cards
    pub fn pay_energy(&mut self, amount: usize) -> Result<(), String> {
        let mut paid = 0;
        
        for card_in_zone in &mut self.energy_zone.cards {
            if paid >= amount {
                break;
            }
            
            // Rule 5.9.1: Only active energy can be paid
            if card_in_zone.orientation == Some(crate::zones::Orientation::Active) {
                card_in_zone.orientation = Some(crate::zones::Orientation::Wait);
                paid += 1;
            }
        }
        
        if paid < amount {
            Err(format!(
                "Could not pay {} energy (only {} active energy available)",
                amount, paid
            ))
        } else {
            Ok(())
        }
    }

    /// Rule 5.10: （エネルギーをメンバーの）下に置く (Place energy under member)
    /// Move energy card to member area and place it under the member
    /// 5.10.1: Move energy card to member's area and place it under the member (4.5.5)
    /// 4.5.5.3: Energy underneath moves with member when member moves to another member area
    /// 4.5.5.4: When member moves to non-member area, only member card moves, energy stays and goes to energy deck via rule processing
    pub fn place_energy_under_member(
        &mut self,
        energy_index: usize,
        target_area: crate::zones::MemberArea,
    ) -> Result<(), String> {
        // Check if energy index is valid
        if energy_index >= self.energy_zone.cards.len() {
            return Err("Invalid energy index".to_string());
        }
        
        // Check if target area has a member
        let target_card = self.stage.get_area(target_area);
        if target_card.is_none() {
            return Err("Target area has no member".to_string());
        }
        
        // Remove energy from energy zone
        let energy_card_in_zone = self.energy_zone.cards.remove(energy_index);
        let energy_card = energy_card_in_zone.card.clone();
        
        // Add energy underneath the member in target area
        if let Some(ref mut member_in_zone) = self.stage.get_area_mut(target_area) {
            member_in_zone.energy_underneath.push(energy_card);
            Ok(())
        } else {
            // Put the energy back if something went wrong
            self.energy_zone.cards.insert(energy_index, energy_card_in_zone);
            Err("Failed to access target member".to_string())
        }
    }

    // ============== LEGAL ACTION VALIDATION ==============

    /// Check if player can activate energy cards
    pub fn can_activate_energy(&self) -> bool {
        // Rule 7.1: Active Phase - Can activate if there are wait energy cards
        self.energy_zone.cards.iter().any(|c| c.orientation == Some(crate::zones::Orientation::Wait))
    }

    /// Check if player can draw a card
    pub fn can_draw_card(&self) -> bool {
        // Rule 8.1: Draw Phase - Can draw if main deck is not empty
        !self.main_deck.is_empty()
    }

    /// Check if player can play a member card from hand
    pub fn can_play_member_to_stage(&self) -> bool {
        // Rule 8.2: Main Phase - Can play if hand has member cards and stage has space
        let has_member = self.hand.cards.iter().any(|c| c.is_member());
        let has_space = self.stage.left_side.is_none() || self.stage.center.is_none() || self.stage.right_side.is_none();
        
        if !has_member || !has_space {
            return false;
        }
        
        // Check if there's any member with cost 0 (can play without energy)
        let has_cost_zero_member = self.hand.cards.iter()
            .filter(|c| c.is_member())
            .any(|c| c.cost.unwrap_or(0) == 0);
        
        if has_cost_zero_member {
            return true;
        }
        
        // Rule 9.6.2.3: Check if player can afford to play a member
        // Find the cheapest member card in hand
        let cheapest_cost = self.hand.cards.iter()
            .filter(|c| c.is_member())
            .map(|c| c.cost.unwrap_or(0))
            .min()
            .unwrap_or(0);
        
        // Count active energy
        let active_energy_count = self.energy_zone.cards.iter()
            .filter(|c| c.orientation == Some(crate::zones::Orientation::Active))
            .count() as u32;
        
        // Check if can use baton touch to reduce cost
        if active_energy_count >= 1 {
            // Check if any stage area has a member for baton touch
            if self.stage.left_side.is_some() || self.stage.center.is_some() || self.stage.right_side.is_some() {
                // Can baton touch to reduce cost
                let member_cost = if let Some(member) = self.stage.left_side.as_ref() {
                    member.card.cost.unwrap_or(1)
                } else if let Some(member) = self.stage.center.as_ref() {
                    member.card.cost.unwrap_or(1)
                } else if let Some(member) = self.stage.right_side.as_ref() {
                    member.card.cost.unwrap_or(1)
                } else {
                    1
                };
                let reduced_cost = cheapest_cost.saturating_sub(member_cost);
                return active_energy_count >= reduced_cost;
            }
        }
        
        active_energy_count >= cheapest_cost
    }

    /// Check if player can place a card in live zone
    pub fn can_place_in_live_zone(&self) -> bool {
        // Rule 9.1: Live Card Set Phase - Can place if hand has member or live cards
        self.hand.cards.iter().any(|c| c.is_member() || c.is_live())
    }

    /// Check if player can play energy card from hand
    pub fn can_play_energy_to_zone(&self) -> bool {
        // Rule 7.2: Energy Phase - Can play if hand has energy cards
        self.hand.cards.iter().any(|c| c.is_energy())
    }

    /// Check if player can shuffle a specific zone
    pub fn can_shuffle_zone(&self, zone: &str) -> bool {
        match zone {
            "deck" | "main_deck" | "メインデッキ" => !self.main_deck.is_empty(),
            "energy_deck" | "エネルギーデッキ" => !self.energy_deck.is_empty(),
            "waitroom" | "控え室" => !self.waitroom.cards.is_empty(),
            _ => false,
        }
    }

    /// Check if player can look at top cards
    pub fn can_look_at_top(&self, count: usize) -> bool {
        // Rule 5.7: Can look if deck has enough cards (or can refresh)
        count > 0 && (self.main_deck.len() >= count || !self.waitroom.cards.is_empty())
    }

    /// Check if player can swap cards between areas
    pub fn can_swap_cards(&self, from_area: crate::zones::MemberArea, to_area: crate::zones::MemberArea) -> bool {
        // Rule 5.8: Can swap if both areas have cards (or one is empty for move)
        let from_has_card = self.stage.get_area(from_area).is_some();
        let to_has_card = self.stage.get_area(to_area).is_some();
        from_has_card && (to_has_card || from_area != to_area)
    }

    /// Check if player can pay energy
    pub fn can_pay_energy(&self, amount: usize) -> bool {
        // Rule 5.9: Can pay if enough active energy available
        let active_count = self.energy_zone.cards.iter()
            .filter(|c| c.orientation == Some(crate::zones::Orientation::Active))
            .count();
        active_count >= amount
    }

    /// Check if player can place energy under member
    pub fn can_place_energy_under_member(&self, target_area: crate::zones::MemberArea) -> bool {
        // Rule 5.10: Can place if energy zone has cards and target area has a member
        !self.energy_zone.cards.is_empty() && self.stage.get_area(target_area).is_some()
    }

    // ============== ABILITY QUERY METHODS ==============

    /// Count cards in a specific zone
    pub fn count_cards_in_zone(&self, zone: &str) -> usize {
        match zone {
            "hand" | "手札" => self.hand.cards.len(),
            "deck" | "main_deck" | "メインデッキ" => self.main_deck.len(),
            "energy_deck" | "エネルギーデッキ" => self.energy_deck.cards.len(),
            "waitroom" | "discard" | "控え室" => self.waitroom.cards.len(),
            "stage" | "ステージ" => {
                self.stage.left_side.as_ref().map_or(0, |_| 1)
                    + self.stage.center.as_ref().map_or(0, |_| 1)
                    + self.stage.right_side.as_ref().map_or(0, |_| 1)
            }
            "energy_zone" | "エネルギー置き場" => self.energy_zone.cards.len(),
            "live_card_zone" | "ライブカード置き場" => self.live_card_zone.len(),
            "success_live_card_zone" | "成功ライブカード置き場" => self.success_live_card_zone.len(),
            "exclusion_zone" | "除外領域" => self.exclusion_zone.cards.len(),
            _ => 0,
        }
    }

    /// Count cards of specific type in a zone
    pub fn count_cards_by_type_in_zone(&self, zone: &str, card_type: &str) -> usize {
        match zone {
            "hand" | "手札" => {
                match card_type {
                    "member" | "member_card" | "メンバー" => self.hand.cards.iter().filter(|c| c.is_member()).count(),
                    "live" | "live_card" | "ライブ" => self.hand.cards.iter().filter(|c| c.is_live()).count(),
                    "energy" | "energy_card" | "エネルギー" => self.hand.cards.iter().filter(|c| c.is_energy()).count(),
                    _ => 0,
                }
            }
            "waitroom" | "discard" | "控え室" => {
                match card_type {
                    "member" | "member_card" | "メンバー" => self.waitroom.cards.iter().filter(|c| c.is_member()).count(),
                    "live" | "live_card" | "ライブ" => self.waitroom.cards.iter().filter(|c| c.is_live()).count(),
                    _ => 0,
                }
            }
            "stage" | "ステージ" => {
                match card_type {
                    "member" | "member_card" | "メンバー" => {
                        self.stage.left_side.as_ref().map_or(0, |_| 1)
                            + self.stage.center.as_ref().map_or(0, |_| 1)
                            + self.stage.right_side.as_ref().map_or(0, |_| 1)
                    }
                    _ => 0,
                }
            }
            _ => 0,
        }
    }

    /// Check if specific character is present on stage
    pub fn has_character_on_stage(&self, character_name: &str) -> bool {
        self.stage.left_side.as_ref().map_or(false, |c| c.card.name == character_name)
            || self.stage.center.as_ref().map_or(false, |c| c.card.name == character_name)
            || self.stage.right_side.as_ref().map_or(false, |c| c.card.name == character_name)
    }

    /// Check if any of the specified characters are present on stage
    pub fn has_any_character_on_stage(&self, character_names: &[String]) -> bool {
        character_names.iter().any(|name| self.has_character_on_stage(name))
    }

    /// Check if specific group is present on stage
    pub fn has_group_on_stage(&self, group_name: &str) -> bool {
        self.stage.left_side.as_ref().map_or(false, |c| c.card.group == group_name)
            || self.stage.center.as_ref().map_or(false, |c| c.card.group == group_name)
            || self.stage.right_side.as_ref().map_or(false, |c| c.card.group == group_name)
    }

    /// Check if specific unit is present on stage
    pub fn has_unit_on_stage(&self, unit_name: &str) -> bool {
        self.stage.left_side.as_ref().map_or(false, |c| c.card.unit.as_deref() == Some(unit_name))
            || self.stage.center.as_ref().map_or(false, |c| c.card.unit.as_deref() == Some(unit_name))
            || self.stage.right_side.as_ref().map_or(false, |c| c.card.unit.as_deref() == Some(unit_name))
    }

    /// Check if member is in specific position
    pub fn has_member_in_position(&self, position: &str) -> bool {
        match position {
            "center" | "センターエリア" => self.stage.center.is_some(),
            "left_side" | "左サイドエリア" => self.stage.left_side.is_some(),
            "right_side" | "右サイドエリア" => self.stage.right_side.is_some(),
            _ => false,
        }
    }

    /// Check if member in specific position is in specific state (active/wait)
    pub fn has_member_in_state_at_position(&self, position: &str, state: &str) -> bool {
        let card_in_zone = match position {
            "center" | "センターエリア" => self.stage.center.as_ref(),
            "left_side" | "左サイドエリア" => self.stage.left_side.as_ref(),
            "right_side" | "右サイドエリア" => self.stage.right_side.as_ref(),
            _ => return false,
        };

        match state {
            "active" | "アクティブ" => {
                card_in_zone.map_or(false, |c| c.orientation == Some(crate::zones::Orientation::Active))
            }
            "wait" | "ウェイト" => {
                card_in_zone.map_or(false, |c| c.orientation == Some(crate::zones::Orientation::Wait))
            }
            _ => false,
        }
    }

    /// Count active energy cards
    pub fn count_active_energy(&self) -> usize {
        self.energy_zone.cards.iter()
            .filter(|c| c.orientation == Some(crate::zones::Orientation::Active))
            .count()
    }

    /// Count wait energy cards
    pub fn count_wait_energy(&self) -> usize {
        self.energy_zone.cards.iter()
            .filter(|c| c.orientation == Some(crate::zones::Orientation::Wait))
            .count()
    }

    /// Get all cards in a zone
    pub fn get_cards_in_zone(&self, zone: &str) -> Vec<&crate::card::Card> {
        match zone {
            "hand" | "手札" => self.hand.cards.iter().collect(),
            "waitroom" | "discard" | "控え室" => self.waitroom.cards.iter().collect(),
            "stage" | "ステージ" => {
                let mut cards = Vec::new();
                if let Some(ref c) = self.stage.left_side {
                    cards.push(&c.card);
                }
                if let Some(ref c) = self.stage.center {
                    cards.push(&c.card);
                }
                if let Some(ref c) = self.stage.right_side {
                    cards.push(&c.card);
                }
                cards
            }
            _ => Vec::new(),
        }
    }
}
