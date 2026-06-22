use crate::ability::enums::Zone;
use crate::zones::{
    EnergyDeck, EnergyZone, ExclusionZone, Hand, LiveCardZone, MainDeck, Stage,
    SuccessLiveCardZone, Waitroom,
};

use crate::card::CardDatabase;
use crate::core::game_modifiers::ModifierEntry;

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

    pub stage_hearts: Option<crate::card::BaseHeart>,

    pub debut_count_this_turn: u32,

    pub live_card_set_limit_reduction: u32,

    pub last_resolution_cards: Vec<i16>,
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

            stage_hearts: None,

            debut_count_this_turn: 0,

            live_card_set_limit_reduction: 0,

            last_resolution_cards: Vec::new(),
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

    /// Rule 10.5.3-10.5.4: Remove a member from stage and recycle its under-cards.
    /// Member cards under → waitroom. Energy cards under → energy deck.
    /// Returns the removed member card ID.
    pub fn remove_member_from_stage_with_recycling(
        &mut self,
        index: usize,
        card_db: &CardDatabase,
    ) -> Option<i16> {
        if index >= 3 || self.stage.stage[index] == -1 {
            return None;
        }
        let card_id = self.stage.stage[index];
        self.stage.stage[index] = crate::constants::EMPTY_SLOT;
        // Recycle under-cards
        let area = match index {
            0 => crate::zones::MemberArea::LeftSide,
            1 => crate::zones::MemberArea::Center,
            _ => crate::zones::MemberArea::RightSide,
        };
        let (member_under, energy_under) = self.stage.recycle_under_cards(area, card_db);
        self.areas_locked_this_turn.remove(&area);
        for cid in member_under {
            self.waitroom.add_card(cid);
        }
        for cid in energy_under {
            self.energy_deck.cards.push(cid);
        }
        Some(card_id)
    }

    pub fn move_card_from_hand_to_stage(
        &mut self,
        hand_index: usize,
        stage_area: crate::zones::MemberArea,
        use_baton_touch: bool,
        card_db: &CardDatabase,
    ) -> Result<(u32, bool, Option<u32>, Option<i16>), String> {
        // Rule 8.2: Main Phase - Play member card from hand to stage

        if hand_index >= self.hand.cards.len() {
            return Err("Invalid hand index".to_string());
        }

        let card_id = self.hand.cards.remove(hand_index);

        if let Some(card) = card_db.get_card(card_id) {
            // log::debug!("Retrieved card: {} (card_no: {})", card.name, card.card_no);

            if !card.is_member() {
                self.hand.cards.insert(hand_index, card_id);

                return Err("Only member cards can be placed on stage".to_string());
            }

            // Rule 9.6.2.3: Cost is equal to the card's cost value in energy
            let card_cost = card.cost.unwrap_or(0);

            // Rule 9.6.2.3: Determine cost and pay all costs

            // Rule: Cost reduction from 常時 abilities (parsed as modify_cost/subtract/hand)
            // Card was already removed from hand, so add 1 to get true hand count
            let hand_count = self.hand.cards.len() + 1;
            let cost_reduction = crate::ability::util::calculate_play_cost_reduction(
                &self.stage,
                &self.success_live_card_zone.cards,
                hand_count,
                card_id,
                card_db,
            );
            // Rule: Cost increase from 常時 abilities (e.g. success_live_zone cards → +cost)
            let mut cost_increase: u32 = 0;
            for ability in &card.abilities {
                if let Some(ref effect) = ability.effect {
                    if crate::ability::enums::ActionType::from_str(&effect.action)
                        == Some(crate::ability::enums::ActionType::ModifyCost)
                        && matches!(effect.operation.as_deref(), Some("increase") | Some("add"))
                        && Zone::from_str(effect.location.as_deref().unwrap_or(""))
                            == Some(Zone::SuccessLiveZone)
                    {
                        let per_unit_count = effect.per_unit_count.unwrap_or(1) as usize;
                        let success_count = self.success_live_card_zone.cards.len();
                        let multiplier = effect.count.unwrap_or(1);
                        cost_increase = ((success_count / per_unit_count) as u32) * multiplier;
                    }
                }
            }
            let mut cost_to_pay = card_cost
                .saturating_sub(cost_reduction)
                .saturating_add(cost_increase);

            // Rule 9.6.2.3.2: Baton touch - if 1+ energy to pay, can send member from target area to waitroom instead

            // Note: Baton touch sends member from the TARGET area (where you're playing the new member)

            // Track baton touch state and replaced member cost
            let mut baton_touch_replaced_cost: Option<u32> = None;

            // Determine if baton touch should be used:
            // 1. If use_baton_touch parameter is explicitly set to true, OR
            // 2. If the target area is occupied (auto-detect baton touch scenario)
            let should_use_baton_touch =
                use_baton_touch || self.stage.get_area(stage_area).is_some();

            let baton_touch_used = if should_use_baton_touch {
                if let Some(existing_member) = self.stage.get_area(stage_area) {
                    // Rule 9.6.2.1.2.1: Cannot baton touch to an area that had a card moved from non-stage to stage this turn

                    if self.areas_locked_this_turn.contains(&stage_area) {
                        self.hand.cards.insert(hand_index, card_id);
                        return Err("Cannot baton touch: area is locked this turn".to_string());
                    } else {
                        // Get the member card ID and cost first
                        let member_card_id = existing_member;
                        let replaced_member_cost = card_db
                            .get_card(member_card_id)
                            .map(|c| c.cost.unwrap_or(1))
                            .unwrap_or(1);

                        // Store the replaced member cost for later use
                        baton_touch_replaced_cost = Some(replaced_member_cost);

                        // Rule 9.6.2.3.2: Reduce cost by member's cost (baton touch)
                        cost_to_pay = cost_to_pay.saturating_sub(replaced_member_cost);

                        // Check if player has sufficient active energy to pay the reduced cost (or if cost is 0 for equal/lower cost baton touch)
                        let active_energy_count = self.energy_zone.active_count();

                        // Allow baton touch if cost_to_pay is 0 (equal/lower cost) OR if there's sufficient energy to pay the reduced cost
                        cost_to_pay == 0
                            || (cost_to_pay > 0 && active_energy_count >= cost_to_pay as usize)
                    }
                } else {
                    // No member in target area, can't baton touch

                    self.hand.cards.insert(hand_index, card_id);

                    return Err("Cannot baton touch - no member in target area".to_string());
                }
            } else {
                false
            };

            // Check cannot_baton_touch protection BEFORE paying energy
            if baton_touch_used {
                if let Some(member_id) = self.stage.get_area(stage_area) {
                    let has_protection = card_db.get_card(member_id).is_some_and(|existing_card| {
                        existing_card.abilities.iter().any(|a| {
                            a.effect.as_ref().is_some_and(|ef| {
                                ef.restriction_type.as_deref() == Some("cannot_baton_touch")
                            })
                        })
                    });
                    if has_protection {
                        self.hand.cards.insert(hand_index, card_id);
                        return Err(
                            "Cannot baton touch: member has baton touch discard protection"
                                .to_string(),
                        );
                    }
                }
            }

            // Rule 9.6.2.3.1: Pay energy equal to cost

            if cost_to_pay > 0 {
                // Use EnergyZone::pay_energy to actually tap energy cards

                if let Err(e) = self.energy_zone.pay_energy(cost_to_pay as usize) {
                    self.hand.cards.insert(hand_index, card_id);

                    return Err(e);
                }
            }

            let index = match stage_area {
                crate::zones::MemberArea::LeftSide => 0,
                crate::zones::MemberArea::Center => 1,
                crate::zones::MemberArea::RightSide => 2,
            };

            let replaced_member = if self.stage.stage[index] != -1 {
                let old_card_opt = self.remove_member_from_stage_with_recycling(index, card_db);
                if let Some(old_card) = old_card_opt {
                    self.waitroom.cards.push(old_card);
                }
                old_card_opt
            } else {
                None
            };

            self.stage.stage[index] = card_id;
            self.areas_locked_this_turn.insert(stage_area);

            // Rule 9.6.2.3.2.1: If baton touch performed, trigger 'baton touch' event

            // This is handled in turn.rs after the card is played to stage

            let replaced_id = replaced_member;
            Ok((
                cost_to_pay,
                baton_touch_used,
                baton_touch_replaced_cost,
                replaced_id,
            ))
        } else {
            self.hand.cards.insert(hand_index, card_id);

            Err("Card not found in database".to_string())
        }
    }

    pub fn move_card_from_hand_to_energy_zone(
        &mut self,
        hand_index: usize,
        card_db: &CardDatabase,
    ) -> Result<(), String> {
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

    pub fn move_card_from_hand_to_live_zone(
        &mut self,
        hand_index: usize,
        card_db: &CardDatabase,
    ) -> Result<(), String> {
        // Rule 9.1: Live Card Set Phase - Place card from hand to live card zone

        if hand_index >= self.hand.cards.len() {
            return Err("Invalid hand index".to_string());
        }

        let card_id = self.hand.cards.remove(hand_index);

        if !self.live_card_zone.can_place_card(card_db, card_id) {
            self.hand.cards.insert(hand_index, card_id);

            return Err("Card cannot be placed in live card zone".to_string());
        }

        self.live_card_zone.add_card(card_id, card_db)?;

        Ok(())
    }

    /// Calculate the total hearts provided by all members on stage
    /// Used for heart satisfaction bonus calculation during live
    pub fn calculate_stage_hearts(
        &self,
        card_db: &CardDatabase,
        heart_color_multiplier: &std::collections::HashMap<i16, crate::card::HeartColor>,
        heart_override: &std::collections::HashMap<i16, (crate::card::HeartColor, u32)>,
        heart_modifiers: &std::collections::HashMap<
            i16,
            std::collections::HashMap<crate::card::HeartColor, ModifierEntry>,
        >,
    ) -> crate::card::BaseHeart {
        use crate::card::HeartColor;
        use std::collections::HashMap;

        let mut total_hearts: HashMap<HeartColor, u32> = HashMap::new();

        for &card_id in &self.stage.stage {
            if card_id == crate::constants::EMPTY_SLOT {
                continue;
            }

            // heart_override: replace card's hearts entirely
            if let Some(&(override_color, override_count)) = heart_override.get(&card_id) {
                *total_hearts.entry(override_color).or_insert(0) += override_count;
                continue;
            }

            if let Some(card) = card_db.get_card(card_id) {
                if let Some(ref base_heart) = card.base_heart {
                    if let Some(override_color) = heart_color_multiplier.get(&card_id) {
                        let total: u32 = base_heart.hearts.values().sum();
                        *total_hearts.entry(*override_color).or_insert(0) += total;
                    } else {
                        for (color, count) in &base_heart.hearts {
                            *total_hearts.entry(*color).or_insert(0) += count;
                        }
                    }
                }
            }

            // heart_modifiers: additive per-color adjustments
            if let Some(mods) = heart_modifiers.get(&card_id) {
                for (color, entry) in mods {
                    let delta = entry.total();
                    if delta != 0 {
                        let new_val =
                            (*total_hearts.get(color).unwrap_or(&0) as i32 + delta).max(0) as u32;
                        if new_val > 0 {
                            total_hearts.insert(*color, new_val);
                        } else {
                            total_hearts.remove(color);
                        }
                    }
                }
            }
        }

        crate::card::BaseHeart {
            hearts: total_hearts,
        }
    }

    pub fn activate_all_energy(&mut self) {
        // Rule 7.4.1: Activate all energy zone and member area wait cards

        self.energy_zone.activate_all();

        // Also activate member area wait cards (orientation tracking in GameState modifiers)

        // For now, this is a no-op as orientation is tracked differently
    }

    pub fn draw_card(&mut self) -> Option<i16> {
        // Rule 8.1: Draw Phase - Active player draws 1 card from main deck to hand
        self.main_deck.draw().inspect(|&card_id| {
            self.add_card_to_hand(card_id);
        })
    }

    pub fn draw_energy(&mut self) -> Option<i16> {
        self.energy_deck.draw().inspect(|&card_id| {
            self.energy_zone.cards.push(card_id);

            self.energy_zone.active_energy_count += 1;
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
}
