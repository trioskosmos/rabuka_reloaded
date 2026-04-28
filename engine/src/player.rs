use crate::zones::{

    EnergyDeck, EnergyZone, ExclusionZone, Hand, LiveCardZone, MainDeck, Stage,

    SuccessLiveCardZone, Waitroom,

};

use crate::card::CardDatabase;

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

    // Rule 9.6.2.1.2.1: Track areas where cards moved from non-stage to stage this turn

    // These areas cannot be targeted for baton touch

    pub areas_locked_this_turn: std::collections::HashSet<crate::zones::MemberArea>,

    // Rule 8.2: Track if player has finished their live card set

    pub has_finished_live_card_set: bool,

    // Track live score for the current live

    pub live_score: i32,

    // Track whether player has a valid live score (Q47, Q48)

    pub has_live_score: bool,

    // Track blade count for blade abilities

    pub blade: usize,

    // Track cards debuted this turn for debut-related abilities

    pub debuted_this_turn: Vec<i16>,

    // Track whether player cannot live (Q68)

    pub cannot_live: bool,

    // Track areas where cards were placed this turn (Q70, Q71)

    pub area_placed_this_turn: Vec<bool>,

    // Track constant abilities active on player (Q78)

    pub constant_abilities: Vec<String>,

    // Track energy wait zone (Q96, Q103)

    pub energy_wait: Vec<i16>,

    // Track cards moved this turn (Q99)

    pub moved_this_turn: Vec<i16>,

    // Track invalidated abilities (Q106)

    pub invalidated_abilities: std::collections::HashSet<i16>,

    // Track cheer-revealed cards (Q107)

    pub cheer_revealed: Vec<i16>,

    // Temporary storage for calculated stage hearts during live score calculation
    pub stage_hearts: Option<crate::card::BaseHeart>,

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

            areas_locked_this_turn: std::collections::HashSet::new(),

            has_finished_live_card_set: false,

            live_score: 0,

            has_live_score: false,

            blade: 0,

            energy_wait: Vec::new(),

            moved_this_turn: Vec::new(),

            debuted_this_turn: Vec::new(),

            cannot_live: false,

            area_placed_this_turn: vec![false; 3],

            constant_abilities: Vec::new(),

            invalidated_abilities: std::collections::HashSet::new(),

            cheer_revealed: Vec::new(),

            stage_hearts: None,

        }

    }



    pub fn set_main_deck(&mut self, cards: VecDeque<i16>) {

        self.main_deck.cards = cards.into_iter().collect();

    }



    pub fn set_energy_deck(&mut self, cards: VecDeque<i16>) {

        self.energy_deck.cards = cards.into_iter().collect();

    }



    // Helper method to get card index by card_id using linear search
    // Hands are small (5-10 cards), so O(n) is acceptable and simpler

    pub fn get_card_index_by_id(&self, card_id: i16) -> Option<usize> {

        self.hand.cards.iter().position(|&c| c == card_id)

    }



    // Helper method to add a card to the hand

    pub fn add_card_to_hand(&mut self, card_id: i16) {

        self.hand.cards.push(card_id);

    }



    // Helper method to remove a card from hand by index

    pub fn remove_card_from_hand(&mut self, index: usize) -> Option<i16> {

        if index >= self.hand.cards.len() {

            return None;

        }

        Some(self.hand.cards.remove(index))

    }

    

    pub fn move_card_from_hand_to_stage(&mut self, hand_index: usize, stage_area: crate::zones::MemberArea, use_baton_touch: bool, card_db: &CardDatabase) -> Result<(u32, bool), String> {

        // Rule 8.2: Main Phase - Play member card from hand to stage

        if hand_index >= self.hand.cards.len() {

            return Err("Invalid hand index".to_string());

        }



        let card_id = self.hand.cards.remove(hand_index);



        if let Some(card) = card_db.get_card(card_id) {

            // eprintln!("Retrieved card: {} (card_no: {})", card.name, card.card_no);

            if !card.is_member() {

                self.hand.cards.insert(hand_index, card_id);

                return Err("Only member cards can be placed on stage".to_string());

            }



            // Rule 9.6.2.3: Cost is equal to the card's cost value in energy

            let card_cost = card.cost.unwrap_or(0);

            // eprintln!("Playing card {} with cost {}", card.name, card_cost);



            // Rule 9.6.2.3: Determine cost and pay all costs

            let mut cost_to_pay = card_cost;



            // Rule 9.6.2.3.2: Baton touch - if 1+ energy to pay, can send member from target area to waitroom instead

            // Note: Baton touch sends member from the TARGET area (where you're playing the new member)

            let baton_touch_used = if use_baton_touch {

                if let Some(existing_member) = self.stage.get_area(stage_area) {

                    // Rule 9.6.2.1.2.1: Cannot baton touch to an area that had a card moved from non-stage to stage this turn

                    if self.areas_locked_this_turn.contains(&stage_area) {

                        false

                    } else {

                        // Check if player has 1+ active energy to pay (or if cost is 0 for equal/lower cost baton touch)

                        let active_energy_count = self.energy_zone.active_count();



                        // Allow baton touch if cost_to_pay is 0 (equal/lower cost) OR if there's energy to pay

                        if cost_to_pay == 0 || (cost_to_pay > 0 && active_energy_count >= 1) {

                            // Get the member card ID

                            let member_card_id = existing_member;

                            let cost = card_db.get_card(member_card_id).map(|c| c.cost.unwrap_or(1)).unwrap_or(1);



                            // Rule 9.6.2.3.2: Reduce cost by member's cost (baton touch)

                            cost_to_pay = cost_to_pay.saturating_sub(cost);

                            true

                        } else {

                            false

                        }

                    }

                } else {

                    // No member in target area, can't baton touch

                    self.hand.cards.insert(hand_index, card_id);

                    return Err("Cannot baton touch - no member in target area".to_string());

                }

            } else {

                false

            };

        

        // Rule 9.6.2.3.1: Pay energy equal to cost

        if cost_to_pay > 0 {

            // Use EnergyZone::pay_energy to actually tap energy cards

            if let Err(e) = self.energy_zone.pay_energy(cost_to_pay as usize) {

                self.hand.cards.insert(hand_index, card_id);

                return Err(e);

            }

        }



        // Store the replaced member card ID if using baton touch

        let replaced_member = if baton_touch_used {

            self.stage.get_area(stage_area)

        } else {

            None

        };



        match stage_area {

            crate::zones::MemberArea::LeftSide => {

                // If area is occupied and not using baton touch, send existing member to waitroom

                if !baton_touch_used && self.stage.stage[0] != -1 {

                    let existing_member = self.stage.stage[0];

                    self.waitroom.cards.push(existing_member);

                }

                self.stage.stage[0] = card_id;

                // Rule 9.6.2.1.2.1: Lock area when card moves from non-stage to stage (for baton touch restriction)

                if !baton_touch_used {

                    self.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);

                }

            }

            crate::zones::MemberArea::Center => {

                // If area is occupied and not using baton touch, send existing member to waitroom

                if !baton_touch_used && self.stage.stage[1] != -1 {

                    let existing_member = self.stage.stage[1];

                    self.waitroom.cards.push(existing_member);

                }

                self.stage.stage[1] = card_id;

                // Rule 9.6.2.1.2.1: Lock area when card moves from non-stage to stage

                if !baton_touch_used {

                    self.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);

                }

            }

            crate::zones::MemberArea::RightSide => {

                // If area is occupied and not using baton touch, send existing member to waitroom

                if !baton_touch_used && self.stage.stage[2] != -1 {

                    let existing_member = self.stage.stage[2];

                    self.waitroom.cards.push(existing_member);

                }

                self.stage.stage[2] = card_id;

                // Rule 9.6.2.1.2.1: Lock area when card moves from non-stage to stage (for baton touch restriction)

                if !baton_touch_used {

                    self.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);

                }

            }

        }



        // Send replaced member to waitroom if baton touch was used

        if let Some(member_id) = replaced_member {

            self.waitroom.cards.push(member_id);

        }

        

        // Rule 9.6.2.3.2.1: If baton touch performed, trigger 'baton touch' event

        // This is handled in turn.rs after the card is played to stage



        Ok((cost_to_pay, baton_touch_used))

    } else {

        self.hand.cards.insert(hand_index, card_id);

        Err("Card not found in database".to_string())

    }

    }

    

    pub fn move_card_from_hand_to_energy_zone(&mut self, hand_index: usize, card_db: &CardDatabase) -> Result<(), String> {

        // Rule 7.2: Energy Phase - Play energy card from hand to energy zone

        if hand_index >= self.hand.cards.len() {

            return Err("Invalid hand index".to_string());

        }



        let card_id = self.hand.cards.remove(hand_index);




        if let Some(card) = card_db.get_card(card_id) {

            if !card.is_energy() {

                // Card is not an energy card, put it back

                self.hand.cards.insert(hand_index, card_id);

                return Err("Card is not an energy card".to_string());

            }

            self.energy_zone.cards.push(card_id);

            Ok(())

        } else {

            self.hand.cards.insert(hand_index, card_id);

            Err("Card not found in database".to_string())

        }

    }



    pub fn move_card_from_hand_to_live_zone(&mut self, hand_index: usize, card_db: &CardDatabase) -> Result<(), String> {

        // Rule 9.1: Live Card Set Phase - Place card from hand to live card zone

        if hand_index >= self.hand.cards.len() {

            return Err("Invalid hand index".to_string());

        }



        let card_id = self.hand.cards.remove(hand_index);




        if !self.live_card_zone.can_place_card(card_db, card_id) {

            self.hand.cards.insert(hand_index, card_id);

            return Err("Card cannot be placed in live card zone".to_string());

        }



        self.live_card_zone.add_card(card_id, false, card_db)?;

        Ok(())

    }



    pub fn can_live(&self, card_db: &CardDatabase) -> bool {

        !self.live_card_zone.get_live_cards(card_db).is_empty()

    }



    pub fn total_live_score(&self, card_db: &CardDatabase, cheer_blade_heart_count: u32) -> u32 {
        // Note: This doesn't include heart satisfaction bonus - use calculate_live_score with stage_hearts for full calculation
        self.live_card_zone.calculate_live_score(card_db, cheer_blade_heart_count, None)
    }
    
    /// Calculate the total hearts provided by all members on stage
    /// Used for heart satisfaction bonus calculation during live
    pub fn calculate_stage_hearts(&self, card_db: &CardDatabase) -> crate::card::BaseHeart {
        use std::collections::HashMap;
        use crate::card::HeartColor;
        
        let mut total_hearts: HashMap<HeartColor, u32> = HashMap::new();
        
        // Collect hearts from all members on stage
        for &card_id in &self.stage.stage {
            if card_id == crate::constants::EMPTY_SLOT {
                continue;
            }
            if let Some(card) = card_db.get_card(card_id) {
                // Add base hearts from the card
                if let Some(ref base_heart) = card.base_heart {
                    for (color, count) in &base_heart.hearts {
                        *total_hearts.entry(*color).or_insert(0) += count;
                    }
                }
            }
        }
        
        crate::card::BaseHeart { hearts: total_hearts }
    }



    pub fn has_victory(&self) -> bool {

        // Rule 11.1: Victory Condition - Player wins when they have 3 cards in Success Live Card Zone

        self.success_live_card_zone.len() >= 3

    }



    pub fn activate_all_energy(&mut self) {

        // Rule 7.4.1: Activate all energy zone and member area wait cards

        self.energy_zone.activate_all();

        // Also activate member area wait cards (orientation tracking in GameState modifiers)

        // For now, this is a no-op as orientation is tracked differently

    }



    pub fn draw_card(&mut self) -> Option<i16> {

        // Rule 8.1: Draw Phase - Active player draws 1 card from main deck to hand

        self.main_deck.draw().map(|card_id| {

            self.add_card_to_hand(card_id);

            card_id

        })

    }



    pub fn draw_energy(&mut self) -> Option<i16> {

        self.energy_deck.draw().map(|card_id| {

            self.energy_zone.cards.push(card_id);

            self.energy_zone.active_energy_count += 1;

            card_id

        })

    }



// ... (rest of the code remains the same)

    pub fn refresh(&mut self) {

        // Rule 10.2: Refresh when main deck is empty and waitroom has cards

        // Rule 10.2.1: Condition - main deck is empty AND waitroom has cards

        // Rule 10.2.2: Shuffle waitroom cards and place them on top of main deck

        // Rule 10.2.3: This happens automatically during check timing

        if self.main_deck.is_empty() && !self.waitroom.cards.is_empty() {

            let mut waitroom_cards = self.waitroom.take_all();

            waitroom_cards.shuffle(&mut rand::thread_rng());

            for card in waitroom_cards {

                self.main_deck.cards.push(card);

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

                self.main_deck.cards.push(card);

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

            "deck" | "main_deck" => {

                self.main_deck.shuffle();

                Ok(())

            }

            "energy_deck" => {

                let mut cards: Vec<_> = self.energy_deck.cards.drain(..).collect();

                cards.shuffle(&mut rand::thread_rng());

                self.energy_deck.cards = cards.into();

                Ok(())

            }

            "waitroom" | "discard" => {

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

    pub fn look_at_top(&self, count: usize, _up_to: bool, _card_db: &CardDatabase) -> Vec<i16> {

        // Rule 5.7.2.1: If count is 0 or less, end the instruction

        if count == 0 {

            return Vec::new();

        }



        // Rule 10.2.2.2: If deck has fewer cards than needed, refresh first would be called

        // (This is handled by the caller via refresh_if_needed)

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

        self.stage.position_change(from_area, to_area)?;

        Ok(())

    }



    /// Rule 5.9: を支払う (Pay energy)

    /// Change active energy cards to wait state

    /// 5.9.1: Change 1 active energy in energy zone to wait state

    /// 5.9.1.1: Multiple icons = pay that many energy cards

    pub fn pay_energy(&mut self, amount: usize) -> Result<(), String> {

        self.energy_zone.pay_energy(amount)

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

        let energy_card_id = self.energy_zone.cards.remove(energy_index);



        // Energy tracking moved to GameState modifiers

        // For now, just return energy card to energy zone

        self.energy_zone.cards.insert(energy_index, energy_card_id);

        Err("Energy tracking moved to GameState modifiers".to_string())

    }



    // ============== LEGAL ACTION VALIDATION ==============



    /// Check if player can activate energy cards

    pub fn can_activate_energy(&self) -> bool {

        // Rule 7.1: Active Phase - Can activate if there are inactive energy cards

        self.energy_zone.active_energy_count < self.energy_zone.cards.len()

    }



    /// Check if player can draw a card

    pub fn can_draw_card(&self) -> bool {

        // Rule 8.1: Draw Phase - Can draw if main deck is not empty

        !self.main_deck.is_empty()

    }



    /// Check if player can play a member card from hand

    pub fn can_play_member_to_stage(&self, card_db: &CardDatabase) -> bool {

        // Rule 8.2: Main Phase - Can play if hand has member cards and stage has space

        let has_member = self.hand.cards.iter().any(|&card_id| {

            card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false)

        });

        let has_space = self.stage.stage[0] == -1 || self.stage.stage[1] == -1 || self.stage.stage[2] == -1;

        

        if !has_member || !has_space {

            return false;

        }



        // Check if there's any member with cost 0 (can play without energy)

        let has_cost_zero_member = self.hand.cards.iter()

            .filter(|&card_id| card_db.get_card(*card_id).map(|c| c.is_member()).unwrap_or(false))

            .any(|&card_id| card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0) == 0).unwrap_or(false));



        if has_cost_zero_member {

            return true;

        }



        // Rule 9.6.2.3: Check if player can afford to play a member

        // Find the cheapest member card in hand

        let cheapest_cost = self.hand.cards.iter()

            .filter(|&card_id| card_db.get_card(*card_id).map(|c| c.is_member()).unwrap_or(false))

            .map(|&card_id| card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0)).unwrap_or(0))

            .min()

            .unwrap_or(0);

        

        // Count active energy

        let active_energy_count = self.energy_zone.active_count() as u32;

        

        // Check if can use baton touch to reduce cost

        if active_energy_count >= 1 {

            // Check if any stage area has a member for baton touch

            if self.stage.stage[0] != -1 || self.stage.stage[1] != -1 || self.stage.stage[2] != -1 {

                // Can baton touch to reduce cost

                let member_cost = if self.stage.stage[0] != -1 {

                    card_db.get_card(self.stage.stage[0]).map(|c| c.cost.unwrap_or(1)).unwrap_or(1)

                } else if self.stage.stage[1] != -1 {

                    card_db.get_card(self.stage.stage[1]).map(|c| c.cost.unwrap_or(1)).unwrap_or(1)

                } else if self.stage.stage[2] != -1 {

                    card_db.get_card(self.stage.stage[2]).map(|c| c.cost.unwrap_or(1)).unwrap_or(1)

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

    pub fn can_place_in_live_zone(&self, card_db: &CardDatabase) -> bool {

        // Rule 9.1: Live Card Set Phase - Can place if hand has member or live cards

        self.hand.cards.iter().any(|&card_id| {

            card_db.get_card(card_id).map(|c| c.is_member() || c.is_live()).unwrap_or(false)

        })

    }



    /// Check if player can play energy card from hand

    pub fn can_play_energy_to_zone(&self, card_db: &CardDatabase) -> bool {

        // Rule 7.2: Energy Phase - Can play if hand has energy cards

        self.hand.cards.iter().any(|&card_id| {

            card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false)

        })

    }



    /// Check if player can shuffle a specific zone

    pub fn can_shuffle_zone(&self, zone: &str) -> bool {

        match zone {

            "deck" | "main_deck" => !self.main_deck.is_empty(),

            "energy_deck" => !self.energy_deck.is_empty(),

            "waitroom" | "discard" => !self.waitroom.cards.is_empty(),

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

        self.energy_zone.active_count() >= amount

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

            "hand" => self.hand.cards.len(),

            "deck" | "main_deck" => self.main_deck.len(),

            "energy_deck" => self.energy_deck.cards.len(),

            "waitroom" | "discard" => self.waitroom.cards.len(),

            "stage" => {

                (if self.stage.stage[0] != -1 { 1 } else { 0 })

                    + (if self.stage.stage[1] != -1 { 1 } else { 0 })

                    + (if self.stage.stage[2] != -1 { 1 } else { 0 })

            }

            "energy_zone" => self.energy_zone.cards.len(),

            "live_card_zone" => self.live_card_zone.len(),

            "success_live_card_zone" => self.success_live_card_zone.len(),

            "exclusion_zone" => self.exclusion_zone.cards.len(),

            _ => 0,

        }

    }



    /// Count cards of specific type in a zone

    pub fn count_cards_by_type_in_zone(&self, zone: &str, card_type: &str, card_db: &CardDatabase) -> usize {

        match zone {

            "hand" => {

                match card_type {

                    "member" | "member_card" => self.hand.cards.iter().filter(|&&card_id| {

                        card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false)

                    }).count(),

                    "live" | "live_card" => self.hand.cards.iter().filter(|&&card_id| {

                        card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false)

                    }).count(),

                    "energy" | "energy_card" => self.hand.cards.iter().filter(|&&card_id| {

                        card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false)

                    }).count(),

                    _ => 0,

                }

            }

            "waitroom" | "discard" => {

                match card_type {

                    "member" | "member_card" => self.waitroom.cards.iter().filter(|&&card_id| {

                        card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false)

                    }).count(),

                    "live" | "live_card" => self.waitroom.cards.iter().filter(|&&card_id| {

                        card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false)

                    }).count(),

                    _ => 0,

                }

            }

            "stage" => {

                match card_type {

                    "member" | "member_card" => {

                        (if self.stage.stage[0] != -1 { 1 } else { 0 })

                            + (if self.stage.stage[1] != -1 { 1 } else { 0 })

                            + (if self.stage.stage[2] != -1 { 1 } else { 0 })

                    }

                    _ => 0,

                }

            }

            _ => 0,

        }

    }



    /// Check if specific character is present on stage

    pub fn has_character_on_stage(&self, character_name: &str, card_db: &CardDatabase) -> bool {

        (self.stage.stage[0] != -1 && card_db.get_card(self.stage.stage[0]).map(|card| card.name == character_name).unwrap_or(false))

            || (self.stage.stage[1] != -1 && card_db.get_card(self.stage.stage[1]).map(|card| card.name == character_name).unwrap_or(false))

            || (self.stage.stage[2] != -1 && card_db.get_card(self.stage.stage[2]).map(|card| card.name == character_name).unwrap_or(false))

    }



    /// Check if any of the specified characters are present on stage

    pub fn has_any_character_on_stage(&self, character_names: &[String], card_db: &CardDatabase) -> bool {

        character_names.iter().any(|name| self.has_character_on_stage(name, card_db))

    }



    /// Check if specific group is present on stage

    pub fn has_group_on_stage(&self, group_name: &str, card_db: &CardDatabase) -> bool {

        (self.stage.stage[0] != -1 && card_db.get_card(self.stage.stage[0]).map(|card| card.group == group_name).unwrap_or(false))

            || (self.stage.stage[1] != -1 && card_db.get_card(self.stage.stage[1]).map(|card| card.group == group_name).unwrap_or(false))

            || (self.stage.stage[2] != -1 && card_db.get_card(self.stage.stage[2]).map(|card| card.group == group_name).unwrap_or(false))

    }



    /// Check if specific unit is present on stage

    pub fn has_unit_on_stage(&self, unit_name: &str, card_db: &CardDatabase) -> bool {

        (self.stage.stage[0] != -1 && card_db.get_card(self.stage.stage[0]).map(|card| card.unit.as_deref() == Some(unit_name)).unwrap_or(false))

            || (self.stage.stage[1] != -1 && card_db.get_card(self.stage.stage[1]).map(|card| card.unit.as_deref() == Some(unit_name)).unwrap_or(false))

            || (self.stage.stage[2] != -1 && card_db.get_card(self.stage.stage[2]).map(|card| card.unit.as_deref() == Some(unit_name)).unwrap_or(false))

    }



    /// Check if member is in specific position

    pub fn has_member_in_position(&self, position: &str) -> bool {

        match position {

            "center" | "センターエリア" => self.stage.stage[1] != -1,

            "left_side" | "左サイドエリア" => self.stage.stage[0] != -1,

            "right_side" | "右サイドエリア" => self.stage.stage[2] != -1,

            _ => false,

        }

    }



    /// Check if member in specific position is in specific state (active/wait)

    pub fn has_member_in_state_at_position(&self, position: &str, state: &str) -> bool {

        let card_id = match position {

            "center" | "センターエリア" => self.stage.stage[1],

            "left_side" | "左サイドエリア" => self.stage.stage[0],

            "right_side" | "右サイドエリア" => self.stage.stage[2],

            _ => return false,

        };



        match state {

            "active" | "アクティブ" => {

                card_id != -1  // Orientation tracking moved to GameState modifiers

            }

            "wait" | "ウェイト" => {

                card_id != -1  // Orientation tracking moved to GameState modifiers

            }

            _ => false,

        }

    }



    /// Count active energy cards

    pub fn count_active_energy(&self) -> usize {

        self.energy_zone.active_count()

    }



    /// Count wait energy cards

    pub fn count_wait_energy(&self) -> usize {

        self.energy_zone.cards.len() - self.energy_zone.active_count()

    }



    /// Get all cards in a zone

    pub fn get_cards_in_zone(&self, zone: &str) -> Vec<i16> {

        match zone {

            "hand" => self.hand.cards.iter().copied().collect(),

            "waitroom" | "discard" => self.waitroom.cards.iter().copied().collect(),

            "stage" => {

                let mut card_ids = Vec::new();

                if self.stage.stage[0] != -1 {

                    card_ids.push(self.stage.stage[0]);

                }

                if self.stage.stage[1] != -1 {

                    card_ids.push(self.stage.stage[1]);

                }

                if self.stage.stage[2] != -1 {

                    card_ids.push(self.stage.stage[2]);

                }

                card_ids

            }

            _ => Vec::new(),

        }

    }

}

