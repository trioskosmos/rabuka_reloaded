#![allow(dead_code)]

use crate::card::{Ability, AbilityCost, AbilityEffect, Condition, Keyword};

use crate::game_state::GameState;
use crate::zones::MemberArea;
use std::vec::Vec;
use std::string::String;

#[derive(Debug, Clone)]
pub enum Choice {
    SelectCard {
        zone: String,
        card_type: Option<String>,
        count: usize,
        description: String,
        allow_skip: bool,
    },
    SelectTarget {
        target: String,
        description: String,
    },
    SelectPosition {
        position: String,
        description: String,
    },
}

#[derive(Debug, Clone)]
pub enum ChoiceResult {
    CardSelected { indices: Vec<usize> },
    TargetSelected { target: String },
    PositionSelected { position: String },
    Skip,
}

#[allow(dead_code)]
pub struct AbilityResolver<'a> {
    game_state: &'a mut GameState,
    pub pending_choice: Option<Choice>,
    // Temporary storage for looked-at cards for selection
    pub looked_at_cards: Vec<i16>,
    // Track effects with duration for expiration
    pub duration_effects: Vec<(String, String)>, // (effect_description, duration)
    // Track current ability being executed for optional cost handling
    pub current_ability: Option<crate::card::Ability>,
}

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    pub fn new(game_state: &'a mut GameState) -> Self {
        AbilityResolver { 
            game_state,
            pending_choice: None,
            looked_at_cards: Vec::new(),
            duration_effects: Vec::new(),
            current_ability: None,
        }
    }

    /// Get pending choice (if any)
    pub fn get_pending_choice(&self) -> Option<&Choice> {
        self.pending_choice.as_ref()
    }

    /// Expire all effects with duration "live_end"
    pub fn expire_live_end_effects(&mut self) {
        let initial_count = self.duration_effects.len();
        self.duration_effects.retain(|(_, duration)| duration != "live_end");
        let expired_count = initial_count - self.duration_effects.len();
        
        if expired_count > 0 {
            eprintln!("Expired {} effects with duration 'live_end'", expired_count);
        }
    }

    /// Provide choice result and continue execution
    pub fn provide_choice_result(&mut self, result: ChoiceResult) -> Result<(), String> {
        let choice = self.pending_choice.clone();
        match (&choice, result) {
            (Some(Choice::SelectCard { zone, card_type, count, description: _, allow_skip: _ }), ChoiceResult::CardSelected { indices }) => {
                match zone.as_str() {
                    "hand" => self.execute_selected_cards_from_hand(indices.as_slice(), *count, card_type.as_deref())?,
                    "deck" => self.execute_selected_cards_from_deck(indices.as_slice(), *count, card_type.as_deref())?,
                    "discard" => self.execute_selected_cards_from_discard(indices.as_slice(), *count, card_type.as_deref())?,
                    "stage" => self.execute_selected_cards_from_stage(indices.as_slice(), *count, card_type.as_deref())?,
                    "looked_at" => self.execute_selected_looked_at_cards(indices.as_slice())?,
                    "energy_zone" => self.execute_selected_energy_zone_cards(indices.as_slice(), *count)?,
                    _ => eprintln!("Card selection from zone '{}' not yet implemented", zone),
                }
                self.pending_choice = None;
                Ok(())
            }
            (Some(Choice::SelectCard { .. }), ChoiceResult::Skip) => {
                // User chose to skip optional cost - don't execute cost, proceed to effect
                eprintln!("User skipped optional cost");
                self.pending_choice = None;
                Ok(())
            }
            (Some(Choice::SelectTarget { target, .. }), ChoiceResult::TargetSelected { target: selected }) => {
                // Check if this is an optional cost choice
                if target == "pay_optional_cost:skip_optional_cost" {
                    // If user chose to pay, continue with the effect
                    // If user chose to skip, skip the effect
                    if selected == "skip_optional_cost" {
                        eprintln!("User chose to skip optional cost");
                        self.pending_choice = None;
                        return Ok(());
                    } else if selected == "pay_optional_cost" {
                        eprintln!("User chose to pay optional cost - continuing with effect");
                        // Retrieve the pending ability and continue execution
                        if let Some(pending) = self.game_state.pending_ability.clone() {
                            // Re-execute the effect without the optional check
                            // For now, just clear and continue - the actual execution happens when the ability is resolved
                            self.pending_choice = None;
                            return Ok(());
                        }
                    }
                }
                // Check if this is a conditional_alternative choice
                if target == "primary|alternative" {
                    // Store the choice and execute the appropriate effect
                    // This would need to be stored and retrieved when resuming execution
                    // For now, just clear the pending choice
                    self.pending_choice = None;
                    return Ok(());
                }
                // Handle other target selections
                self.pending_choice = None;
                Ok(())
            }
            (Some(Choice::SelectPosition { .. }), ChoiceResult::PositionSelected { .. }) => {
                self.pending_choice = None;
                Ok(())
            }
            _ => Err("Choice result does not match pending choice".to_string()),
        }
    }

    /// Execute card selection from zone based on user selection
    fn execute_selected_cards_from_zone(&mut self, zone: &str, indices: &[usize], _count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        let player = &mut self.game_state.player1;
        let card_db = self.game_state.card_database.clone();

        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true,
            }
        };

        match zone {
            "hand" => {
                // Remove selected cards from hand and add to waitroom
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

                for i in indices_to_remove {
                    if i < player.hand.cards.len() {
                        let card_id = player.hand.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) {
                            player.waitroom.add_card(card_id);
                        } else {
                            // Card doesn't match filter, put it back
                            player.hand.cards.insert(i, card_id);
                        }
                    }
                }
            }
            "deck" => {
                // Remove selected cards from deck and add to hand
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

                for i in indices_to_remove {
                    if i < player.main_deck.cards.len() {
                        let card_id = player.main_deck.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) {
                            player.hand.add_card(card_id);
                        } else {
                            // Card doesn't match filter, put it back
                            player.main_deck.cards.insert(i, card_id);
                        }
                    }
                }
            }
            "discard" => {
                // Remove selected cards from discard and add to hand
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

                for i in indices_to_remove {
                    if i < player.waitroom.cards.len() {
                        let card_id = player.waitroom.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) {
                            player.hand.add_card(card_id);
                        } else {
                            // Card doesn't match filter, put it back
                            player.waitroom.cards.insert(i, card_id);
                        }
                    }
                }
            }
            "stage" => {
                // Map indices to stage positions (0=left, 1=center, 2=right)
                let stage_positions = [0, 1, 2];
                for &idx in indices {
                    if idx < stage_positions.len() {
                        let pos = stage_positions[idx];
                        if player.stage.stage[pos] != -1 {
                            let card_id = player.stage.stage[pos];
                            if matches_card_type(card_id, card_type_filter) {
                                player.stage.stage[pos] = -1;
                                player.hand.add_card(card_id);
                            }
                        }
                    }
                }
            }
            _ => return Err(format!("Unknown zone: {}", zone)),
        }

        Ok(())
    }

    // Convenience wrappers for backward compatibility
    fn execute_selected_cards_from_hand(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("hand", indices, count, card_type_filter)
    }

    fn execute_selected_cards_from_deck(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("deck", indices, count, card_type_filter)
    }

    fn execute_selected_cards_from_discard(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("discard", indices, count, card_type_filter)
    }

    fn execute_selected_cards_from_stage(&mut self, indices: &[usize], count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        self.execute_selected_cards_from_zone("stage", indices, count, card_type_filter)
    }

    /// Execute card selection from looked_at cards buffer
    fn execute_selected_looked_at_cards(&mut self, indices: &[usize]) -> Result<(), String> {
        let player = &mut self.game_state.player1;

        // Remove selected cards from looked_at buffer and add to hand
        let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
        indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

        for i in indices_to_remove {
            if i < self.looked_at_cards.len() {
                let card_id = self.looked_at_cards.remove(i);
                player.hand.add_card(card_id);
            }
        }

        // Move remaining cards to discard (selection_remaining logic)
        for card_id in self.looked_at_cards.drain(..) {
            player.waitroom.add_card(card_id);
        }

        Ok(())
    }

    /// Execute card selection from energy_zone for change_state
    fn execute_selected_energy_zone_cards(&mut self, indices: &[usize], _count: usize) -> Result<(), String> {
        let player = &mut self.game_state.player1;

        // Deactivate selected energy cards
        let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
        indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

        for i in indices_to_remove {
            if i < self.looked_at_cards.len() {
                // Card is in looked_at_cards buffer, just track it
                let _card_id = self.looked_at_cards.remove(i);
            }
        }

        // Decrease active energy count by the number of selected cards
        let deactivated_count = indices.len();
        if player.energy_zone.active_energy_count >= deactivated_count {
            player.energy_zone.active_energy_count -= deactivated_count;
        }

        eprintln!("Deactivated {} energy cards (now {} active)", deactivated_count, player.energy_zone.active_energy_count);

        Ok(())
    }

    /// Evaluate a condition against the current game state
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        match condition.condition_type.as_deref() {
            Some("compound") => self.evaluate_compound_condition(condition),
            Some("comparison_condition") => self.evaluate_comparison_condition(condition),
            Some("location_condition") => self.evaluate_location_condition(condition),
            Some("position_condition") => self.evaluate_position_condition(condition),
            Some("group_condition") => self.evaluate_group_condition(condition),
            Some("card_count_condition") => self.evaluate_card_count_condition(condition),
            Some("appearance_condition") => self.evaluate_appearance_condition(condition),
            Some("temporal_condition") => self.evaluate_temporal_condition(condition),
            Some("state_condition") => self.evaluate_state_condition(condition),
            Some("energy_state_condition") => self.evaluate_energy_state_condition(condition),
            Some("movement_condition") => self.evaluate_movement_condition(condition),
            Some("ability_negation_condition") => self.evaluate_ability_negation_condition(condition),
            Some("or_condition") => self.evaluate_or_condition(condition),
            Some("any_of_condition") => self.evaluate_any_of_condition(condition),
            Some("score_threshold_condition") => self.evaluate_score_threshold_condition(condition),
            _ => {
                // Default: unknown condition type, return true (fail-open)
                eprintln!("Unknown condition type: {:?}", condition.condition_type);
                true
            }
        }
    }

    fn evaluate_compound_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            match condition.operator.as_deref() {
                Some("and") => conditions.iter().all(|c| self.evaluate_condition(c)),
                Some("or") => conditions.iter().any(|c| self.evaluate_condition(c)),
                _ => true,
            }
        } else {
            true
        }
    }

    fn evaluate_comparison_condition(&self, condition: &Condition) -> bool {
        let count = self.get_count_for_condition(condition);
        let target_count = condition.count.unwrap_or(0);

        match condition.operator.as_deref() {
            Some(">=") => count >= target_count,
            Some(">") => count > target_count,
            Some("<=") => count <= target_count,
            Some("<") => count < target_count,
            Some("==") | Some("=") => count == target_count,
            Some("!=") => count != target_count,
            _ => true,
        }
    }

    fn evaluate_location_condition(&self, condition: &Condition) -> bool {
        // Check if cards exist in specified location with optional comparison
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let card_type_filter = condition.card_type.as_deref();
        let aggregate = condition.aggregate.as_deref(); // e.g., "total"
        let comparison_type = condition.comparison_type.as_deref(); // e.g., "score"
        let operator = condition.operator.as_deref(); // e.g., ">=", "=="
        let count_threshold = condition.count.unwrap_or(0);
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        let card_db = &self.game_state.card_database;

        let matches_card_type = |card_id: i16| -> bool {
            if let Some(card) = card_db.get_card(card_id) {
                match card_type_filter.unwrap_or("") {
                    "member_card" | "メンバー" => card.is_member(),
                    "live_card" | "ライブ" => card.is_live(),
                    _ => true,
                }
            } else {
                false
            }
        };

        // Calculate the value based on location and comparison type
        let location_value = match location {
            "stage" => {
                if comparison_type == Some("score") {
                    // Calculate total score of cards on stage
                    let mut total_score = 0u32;
                    let card_db = &self.game_state.card_database;
                    if player.stage.stage[1] != -1 {
                        if let Some(card) = card_db.get_card(player.stage.stage[1]) {
                            total_score += card.get_score() + self.game_state.get_score_modifier(player.stage.stage[1]) as u32;
                        }
                    }
                    if player.stage.stage[0] != -1 {
                        if let Some(card) = card_db.get_card(player.stage.stage[0]) {
                            total_score += card.get_score() + self.game_state.get_score_modifier(player.stage.stage[0]) as u32;
                        }
                    }
                    if player.stage.stage[2] != -1 {
                        if let Some(card) = card_db.get_card(player.stage.stage[2]) {
                            total_score += card.get_score() + self.game_state.get_score_modifier(player.stage.stage[2]) as u32;
                        }
                    }
                    total_score
                } else {
                    // Count cards on stage
                    let count = player.stage.stage.iter().filter(|&&card_id| card_id != -1).count();
                    count as u32
                }
            }
            "hand" => {
                if comparison_type == Some("score") {
                    // Calculate total score of cards in hand
                    let card_db = &self.game_state.card_database;
                    player.hand.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(0, |c| c.get_score() + self.game_state.get_score_modifier(id) as u32)
                    }).sum()
                } else if card_type_filter.is_some() {
                    player.hand.cards.iter().filter(|&id| {
                        matches_card_type(*id)
                    }).count() as u32
                } else {
                    player.hand.cards.len() as u32
                }
            }
            "deck" => player.main_deck.cards.len() as u32,
            "discard" => {
                if comparison_type == Some("score") {
                    let card_db = &self.game_state.card_database;
                    player.waitroom.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(0, |c| c.get_score() + self.game_state.get_score_modifier(id) as u32)
                    }).sum()
                } else if card_type_filter.is_some() {
                    player.waitroom.cards.iter().filter(|&id| {
                        matches_card_type(*id)
                    }).count() as u32
                } else {
                    player.waitroom.cards.len() as u32
                }
            }
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => {
                if comparison_type == Some("score") {
                    let card_db = &self.game_state.card_database;
                    player.live_card_zone.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(0, |c| c.get_score() + self.game_state.get_score_modifier(id) as u32)
                    }).sum()
                } else {
                    player.live_card_zone.cards.len() as u32
                }
            }
            "success_live_zone" => {
                if comparison_type == Some("score") {
                    let card_db = &self.game_state.card_database;
                    player.success_live_card_zone.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(0, |c| c.get_score() + self.game_state.get_score_modifier(id) as u32)
                    }).sum()
                } else {
                    player.success_live_card_zone.cards.len() as u32
                }
            }
            _ => 0,
        };

        // Apply aggregate if specified (e.g., "total" for sum)
        let final_value = match aggregate {
            Some("total") => location_value, // Already summed
            None => location_value,
            _ => location_value,
        };

        // Apply operator comparison if specified
        match operator {
            Some(">=") => final_value >= count_threshold,
            Some(">") => final_value > count_threshold,
            Some("<=") => final_value <= count_threshold,
            Some("<") => final_value < count_threshold,
            Some("==") | Some("=") => final_value == count_threshold,
            Some("!=") => final_value != count_threshold,
            None => final_value > 0, // Default: just check if non-zero
            _ => final_value > 0,
        }
    }

    fn evaluate_position_condition(&self, condition: &Condition) -> bool {
        // Check position conditions (center, left_side, right_side)
        let target = condition.target.as_deref().unwrap_or("self");
        let position = condition.position.as_ref().and_then(|p| p.position.as_deref()).unwrap_or("");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        match position {
            "center" => player.stage.stage[1] != -1,
            "left_side" => player.stage.stage[0] != -1,
            "right_side" => player.stage.stage[2] != -1,
            "any" => player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1,
            _ => true,
        }
    }

    fn evaluate_group_condition(&self, condition: &Condition) -> bool {
        let count = self.get_group_card_count(condition);
        let target_count = condition.count.unwrap_or(0);

        match condition.operator.as_deref() {
            Some(">=") => count >= target_count,
            Some(">") => count > target_count,
            Some("<=") => count <= target_count,
            Some("<") => count < target_count,
            Some("==") | Some("=") => count == target_count,
            Some("!=") => count != target_count,
            _ => true,
        }
    }

    fn evaluate_card_count_condition(&self, condition: &Condition) -> bool {
        let card_type = condition.card_type.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let count = condition.count.unwrap_or(0);

        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        let actual_count = match card_type {
            "live_card" => player.live_card_zone.len(),
            "member_card" => player.stage.total_blades(&self.game_state.card_database) as usize, // Approximate
            "energy_card" => player.energy_zone.cards.len(),
            _ => 0,
        };

        match condition.operator.as_deref() {
            Some(">=") => actual_count as u32 >= count,
            Some(">") => actual_count as u32 > count,
            Some("<=") => actual_count as u32 <= count,
            Some("<") => (actual_count as u32) < count,
            Some("==") | Some("=") => actual_count as u32 == count,
            Some("!=") => actual_count as u32 != count,
            _ => true,
        }
    }

    fn evaluate_appearance_condition(&self, condition: &Condition) -> bool {
        // Check if a card has appeared (moved to a location)
        let appearance = condition.appearance.unwrap_or(false);
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
        
        if appearance {
            // Check if cards have appeared in the specified location
            match location {
                "stage" => {
                    player.stage.stage.iter().any(|&card_id| card_id != -1)
                }
                "hand" => {
                    !player.hand.cards.is_empty()
                }
                "discard" => {
                    !player.waitroom.cards.is_empty()
                }
                _ => true,
            }
        } else {
            // Check if cards have NOT appeared in the specified location
            match location {
                "stage" => {
                    player.stage.stage[0] == -1 && player.stage.stage[1] == -1 && player.stage.stage[2] == -1
                }
                "hand" => {
                    player.hand.cards.is_empty()
                }
                "discard" => {
                    player.waitroom.cards.is_empty()
                }
                _ => true,
            }
        }
    }

    fn evaluate_temporal_condition(&self, condition: &Condition) -> bool {
        // Check temporal conditions (this turn, live_end, etc.)
        let temporal = condition.temporal.as_deref().unwrap_or("");
        let phase = condition.phase.as_deref();
        
        match temporal {
            "this_turn" => {
                // Check if condition is for the current turn
                if let Some(created_turn) = condition.temporal_scope.as_ref().and_then(|s| s.parse::<u32>().ok()) {
                    created_turn == self.game_state.turn_number
                } else {
                    // If no specific turn, assume it's for current turn
                    true
                }
            }
            "live_end" => {
                // Check if live has ended (after live performance phase)
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "this_live" => {
                // Check if during the current live (live card set to live victory determination)
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSet) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance) ||
                matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "before_live" => {
                // Check if before live starts
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSet) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance) &&
                !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination)
            }
            "first_turn" => {
                self.game_state.is_first_turn
            }
            _ => {
                // Check phase if specified
                if let Some(phase_str) = phase {
                    match phase_str {
                        "active" => matches!(self.game_state.current_phase, crate::game_state::Phase::Active),
                        "live_card_set" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSet),
                        "live_performance" => matches!(self.game_state.current_phase, crate::game_state::Phase::FirstAttackerPerformance) ||
                                               matches!(self.game_state.current_phase, crate::game_state::Phase::SecondAttackerPerformance),
                        "live_victory" => matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination),
                        _ => true, // Unknown phase, allow
                    }
                } else {
                    true // Unknown temporal condition, allow
                }
            }
        }
    }

    fn evaluate_state_condition(&self, condition: &Condition) -> bool {
        // Check card state (active, wait, face_up, face_down)
        let state = condition.state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        match state {
            "active" => {
                // Orientation tracking moved to GameState modifiers
                // For now, return true if any card is on stage
                player.stage.stage.iter().any(|&card_id| card_id != -1)
            }
            "wait" => {
                // Orientation tracking moved to GameState modifiers
                // For now, return true if any card is on stage
                player.stage.stage.iter().any(|&card_id| card_id != -1)
            }
            _ => true,
        }
    }

    fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        // Check energy card states
        let energy_state = condition.energy_state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        match energy_state {
            "active" => player.energy_zone.active_count() > 0,
            _ => true,
        }
    }

    fn evaluate_movement_condition(&self, condition: &Condition) -> bool {
        // Check movement-related conditions
        let movement = condition.movement.as_deref().unwrap_or("");
        let movement_state = condition.movement_state.as_deref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
        
        match movement {
            "moved" => {
                // Check if a card has moved to/from a location
                if let Some(state) = movement_state {
                    match state {
                        "to_stage" => {
                            // Check if any card moved to stage this turn
                            // This would need to track movement events - for now check if stage has cards
                            player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1
                        }
                        "from_stage" => {
                            // Check if any card moved from stage this turn
                            // This would need to track movement events - for now assume true if waitroom has cards
                            !player.waitroom.cards.is_empty()
                        }
                        "to_discard" => {
                            // Check if card moved to discard this turn
                            !player.waitroom.cards.is_empty()
                        }
                        _ => true, // Unknown movement state, allow
                    }
                } else {
                    // No specific movement state, just check if movement occurred
                    true // Placeholder - would need proper movement tracking
                }
            }
            "notmoved" => {
                // Check if a card has not moved
                if let Some(state) = movement_state {
                    match state {
                        "this_turn" => {
                            // Check if card has not moved this turn
                            // This would need to track movement events per card
                            true // Placeholder
                        }
                        _ => true,
                    }
                } else {
                    true
                }
            }
            "baton_touch" => {
                // Check if baton touch occurred
                condition.baton_touch_trigger.unwrap_or(false)
            }
            _ => {
                // Check location-based movement conditions
                match location {
                    "stage" => {
                        // Check if movement involves stage
                        player.stage.stage[0] != -1 || player.stage.stage[1] != -1 || player.stage.stage[2] != -1
                    }
                    "hand" => {
                        // Check if movement involves hand
                        !player.hand.cards.is_empty()
                    }
                    "discard" => {
                        // Check if movement involves discard
                        !player.waitroom.cards.is_empty()
                    }
                    _ => true,
                }
            }
        }
    }

    fn evaluate_ability_negation_condition(&self, condition: &Condition) -> bool {
        // Check if abilities are negated
        let negation = condition.negation.unwrap_or(false);
        
        if negation {
            // Check if there are any prohibition effects that negate abilities
            // For now, check if the prohibition_effects list is not empty
            self.game_state.prohibition_effects.is_empty()
        } else {
            true
        }
    }

    fn evaluate_or_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            conditions.iter().any(|c| self.evaluate_condition(c))
        } else {
            true
        }
    }

    fn evaluate_any_of_condition(&self, condition: &Condition) -> bool {
        // Check if any condition is met
        if let Some(ref any_of) = condition.any_of {
            // any_of is a list of condition types that should be evaluated
            // For now, this is a simplified implementation
            // A full implementation would evaluate each condition type and return true if any match
            !any_of.is_empty()
        } else {
            true
        }
    }

    fn evaluate_score_threshold_condition(&self, condition: &Condition) -> bool {
        // Check score thresholds (cheer blade heart counts)
        let count = condition.count.unwrap_or(0);
        let operator = condition.operator.as_deref();
        let target = condition.target.as_deref().unwrap_or("self");
        
        let cheer_count = match target {
            "self" => self.game_state.player1_cheer_blade_heart_count,
            "opponent" => self.game_state.player2_cheer_blade_heart_count,
            _ => self.game_state.player1_cheer_blade_heart_count,
        };
        
        match operator {
            Some(">=") => cheer_count >= count,
            Some(">") => cheer_count > count,
            Some("<=") => cheer_count <= count,
            Some("<") => cheer_count < count,
            Some("==") | Some("=") => cheer_count == count,
            Some("!=") => cheer_count != count,
            _ => true,
        }
    }

    fn get_count_for_condition(&self, condition: &Condition) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        match location {
            "stage" => player.stage.total_blades(&self.game_state.card_database),
            "hand" => player.hand.len() as u32,
            "deck" => player.main_deck.len() as u32,
            "discard" => player.waitroom.len() as u32,
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => player.live_card_zone.len() as u32,
            "success_live_zone" => player.success_live_card_zone.len() as u32,
            _ => 0,
        }
    }

    fn get_group_card_count(&self, condition: &Condition) -> u32 {
        // Count cards of a specific group
        let group_filter = condition.group_names.as_ref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
        
        let mut count = 0;
        
        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();
        
        // Helper function to check if card matches group
        let matches_group = |card_id: i16, groups: Option<&Vec<String>>| -> bool {
            match groups {
                Some(group_names) => card_db.get_card(card_id).map(|c| group_names.iter().any(|g| c.group == *g)).unwrap_or(false),
                None => true,
            }
        };
        
        match location {
            "stage" => {
                if player.stage.stage[1] != -1 {
                    if matches_group(player.stage.stage[1], group_filter) {
                        count += 1;
                    }
                }
                if player.stage.stage[0] != -1 {
                    if matches_group(player.stage.stage[0], group_filter) {
                        count += 1;
                    }
                }
                if player.stage.stage[2] != -1 {
                    if matches_group(player.stage.stage[2], group_filter) {
                        count += 1;
                    }
                }
                if player.stage.stage[0] != -1 {
                    if matches_group(player.stage.stage[0], group_filter) {
                        count += 1;
                    }
                }
                if player.stage.stage[2] != -1 {
                    if matches_group(player.stage.stage[2], group_filter) {
                        count += 1;
                    }
                }
            }
            "hand" => {
                for card in &player.hand.cards {
                    if matches_group(*card, group_filter) {
                        count += 1;
                    }
                }
            }
            "discard" | "waitroom" => {
                for card in &player.waitroom.cards {
                    if matches_group(*card, group_filter) {
                        count += 1;
                    }
                }
            }
            _ => {
                // Do nothing for unknown zones
            }
        }

        count
    }

    fn infer_action_from_text(&self, text: &str) -> String {
        // Simple text-based inference for action types from Japanese text
        if text.contains("手札に加える") || text.contains("手札に加えてもよい") {
            // Add to hand - could be draw or move_cards
            if text.contains("デッキの上から") || text.contains("カードを1枚引く") {
                "draw".to_string()
            } else if text.contains("控え室から") || text.contains("discard") {
                "move_cards".to_string()
            } else {
                "draw".to_string() // Default to draw for adding to hand
            }
        } else if text.contains("控え室に置く") || text.contains("discard") {
            "move_cards".to_string()
        } else if text.contains("ステージに登場") || text.contains("stage") {
            "move_cards".to_string()
        } else if text.contains("公開する") || text.contains("reveal") {
            "reveal".to_string()
        } else if text.contains("アクティブ") || text.contains("ウェイト") {
            "change_state".to_string()
        } else if text.contains("ブレード") || text.contains("blade") {
            "gain_resource".to_string()
        } else if text.contains("スコア") || text.contains("score") {
            "modify_score".to_string()
        } else {
            // Default: try to infer from common patterns
            if text.contains("加える") {
                "draw".to_string()
            } else if text.contains("置く") {
                "move_cards".to_string()
            } else {
                "".to_string() // Unknown
            }
        }
    }

    /// Check if an effect can be activated based on its activation conditions
    pub fn can_activate_effect(&self, effect: &AbilityEffect) -> bool {
        // Check activation_condition_parsed first (structured condition)
        if let Some(ref activation_condition) = effect.activation_condition_parsed {
            if !self.evaluate_condition(activation_condition) {
                return false;
            }
        }

        // Check activation_condition (string-based, for logging/debugging)
        if let Some(ref _activation_text) = effect.activation_condition {
            // For now, we just log it - full parsing would be needed
            // The parsed version should handle the actual logic
            eprintln!("Activation condition: {}", _activation_text);
        }

        true
    }

    /// Check if keywords are satisfied for an ability
    /// Keywords include position restrictions (Center, LeftSide, RightSide) and timing restrictions (Turn1, Turn2)
    pub fn check_keywords(&self, keywords: &[Keyword], card_position: Option<MemberArea>) -> bool {
        for keyword in keywords {
            match keyword {
                Keyword::Center => {
                    if card_position != Some(MemberArea::Center) {
                        return false;
                    }
                }
                Keyword::LeftSide => {
                    if card_position != Some(MemberArea::LeftSide) {
                        return false;
                    }
                }
                Keyword::RightSide => {
                    if card_position != Some(MemberArea::RightSide) {
                        return false;
                    }
                }
                Keyword::Turn1 => {
                    // Only valid on turn 1
                    if self.game_state.turn_number != 1 {
                        return false;
                    }
                }
                Keyword::Turn2 => {
                    // Only valid on turn 2
                    if self.game_state.turn_number != 2 {
                        return false;
                    }
                }
                Keyword::Debut => {
                    // Only valid when this is the first time the member is on stage
                    // Check if the card at this position has turn_played equal to current turn
                    if let Some(pos) = card_position {
                        let card_id = match pos {
                            MemberArea::Center => self.game_state.player1.stage.stage[1],
                            MemberArea::LeftSide => self.game_state.player1.stage.stage[0],
                            MemberArea::RightSide => self.game_state.player1.stage.stage[2],
                        };
                        if card_id != -1 {
                            // Debut tracking moved to GameState modifiers
                            // For now, always return true
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                Keyword::LiveStart => {
                    // Only valid at live card set phase
                    if !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveCardSet) {
                        return false;
                    }
                }
                Keyword::LiveSuccess => {
                    // Only valid at live victory determination phase
                    if !matches!(self.game_state.current_phase, crate::game_state::Phase::LiveVictoryDetermination) {
                        return false;
                    }
                }
                Keyword::PositionChange => {
                    // Only valid when a position change has occurred this turn
                    // This would need to track position changes - for now return true
                    // A proper implementation would check if a card moved to a different stage area
                }
                Keyword::FormationChange => {
                    // Only valid when a formation change has occurred this turn
                    // This would need to track when multiple members move simultaneously
                    // For now return true - proper implementation would check formation changes
                }
            }
        }
        true
    }

    /// Execute an ability effect
    pub fn execute_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // First, check activation conditions (gates whether ability can be used)
        if !self.can_activate_effect(effect) {
            return Ok(()); // Activation condition not met, skip effect
        }

        // Then, check if there's a condition for the effect itself
        if let Some(ref condition) = effect.condition {
            if !self.evaluate_condition(condition) {
                return Ok(()); // Condition not met, skip effect
            }
        }

        // Infer action and other fields from text if action field is empty
        let action_to_use = if effect.action.is_empty() {
            // Try to infer from text at runtime
            self.infer_action_from_text(&effect.text)
        } else {
            effect.action.clone()
        };

        match action_to_use.as_str() {
            "sequential" => self.execute_sequential_effect(effect),
            "conditional_alternative" => self.execute_conditional_alternative(effect),
            "look_and_select" => self.execute_look_and_select(effect),
            "draw" | "draw_card" => self.execute_draw(effect),
            "draw_until_count" => self.execute_draw_until_count(effect),
            "move_cards" => self.execute_move_cards(effect),
            "gain_resource" => self.execute_gain_resource(effect),
            "change_state" => self.execute_change_state(effect),
            "modify_score" => self.execute_modify_score(effect),
            "modify_required_hearts" => self.execute_modify_required_hearts(effect),
            "set_cost" => self.execute_set_cost(effect),
            "set_blade_type" => self.execute_set_blade_type(effect),
            "set_heart_type" => self.execute_set_heart_type(effect),
            "activate_ability" => self.execute_activate_ability(effect),
            "invalidate_ability" => self.execute_invalidate_ability(effect),
            "gain_ability" => self.execute_gain_ability(effect),
            "play_baton_touch" => self.execute_play_baton_touch(effect),
            "reveal" => self.execute_reveal(effect),
            "select" => self.execute_select(effect),
            "look_at" => self.execute_look_at(effect),
            "modify_required_hearts_global" => self.execute_modify_required_hearts_global(effect),
            "modify_yell_count" => self.execute_modify_yell_count(effect),
            "place_energy_under_member" => self.execute_place_energy_under_member(effect),
            "activation_cost" => self.execute_activation_cost(effect),
            "position_change" => self.execute_position_change(effect),
            "appear" => self.execute_appear(effect),
            "choice" => self.execute_choice(effect),
            "pay_energy" => self.execute_pay_energy(effect),
            "set_card_identity" => self.execute_set_card_identity(effect),
            "discard_until_count" => self.execute_discard_until_count(effect),
            "restriction" => self.execute_restriction(effect),
            "re_yell" => self.execute_re_yell(effect),
            "modify_cost" => self.execute_modify_cost(effect),
            "activation_restriction" => self.execute_activation_restriction(effect),
            "choose_required_hearts" => self.execute_choose_required_hearts(effect),
            "modify_limit" => self.execute_modify_limit(effect),
            "set_blade_count" => self.execute_set_blade_count(effect),
            "set_required_hearts" => self.execute_set_required_hearts(effect),
            "set_score" => self.execute_set_score(effect),
            "custom" => {
                // Custom actions are card-specific effects not yet implemented
                // For now, skip them without error
                Ok(())
            }
            _ => {
                eprintln!("Unknown action type: {}", effect.action);
                Ok(())
            }
        }
    }

    fn execute_sequential_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let conditional = effect.condition.is_some();
        let condition = effect.condition.as_ref();

        // Check condition if this is a conditional sequential effect
        if conditional {
            if let Some(cond) = condition {
                // For now, we'll execute the actions regardless of condition
                // A full implementation would evaluate the condition
                eprintln!("Conditional sequential effect with condition: {:?}", cond);
            }
        }

        if let Some(ref actions) = effect.actions {
            for action in actions {
                self.execute_effect(action)?;
            }
        }
        Ok(())
    }

    fn execute_conditional_alternative(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // If the effect has a choice flag or if both effects are available, request user choice
        // For now, check if both primary and alternative effects exist
        let has_primary = effect.primary_effect.is_some();
        let has_alternative = effect.alternative_effect.is_some();
        
        if has_primary && has_alternative {
            // Request user to choose between primary and alternative
            // Show the actual effect texts to make the choice clear
            let primary_text = effect.primary_effect.as_ref().and_then(|e| Some(e.text.clone()))
                .unwrap_or_else(|| "Primary effect".to_string());
            let alternative_text = effect.alternative_effect.as_ref().and_then(|e| Some(e.text.clone()))
                .unwrap_or_else(|| "Alternative effect".to_string());
            
            let description = format!("Choose effect:\nPrimary: {}\nAlternative: {}", primary_text, alternative_text);
            
            self.pending_choice = Some(Choice::SelectTarget {
                target: "primary|alternative".to_string(),
                description: description,
            });
            // Return early - execution will continue after user provides choice
            return Ok(());
        }
        
        // Otherwise, check condition and execute appropriate effect
        if let Some(ref alt_condition) = effect.alternative_condition {
            if self.evaluate_condition(alt_condition) {
                // Execute alternative effect
                if let Some(ref alt_effect) = effect.alternative_effect {
                    return self.execute_effect(alt_effect);
                }
            }
        }

        // Otherwise execute primary effect
        if let Some(ref primary_effect) = effect.primary_effect {
            self.execute_effect(primary_effect)
        } else {
            Ok(())
        }
    }

    fn execute_look_and_select(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Execute look action first (stores cards in looked_at_cards)
        if let Some(ref look_action) = effect.look_action {
            self.execute_effect(look_action)?;
        }

        // Check if select_action has placement_order parameter
        if let Some(ref select_action) = effect.select_action {
            let placement_order = select_action.placement_order.as_deref();
            let count = select_action.count.unwrap_or(1);
            let optional = select_action.optional.unwrap_or(false);

            if placement_order.is_some() || optional {
                // Need user choice for selection when placement_order is specified or optional
                // When placement_order is specified, user can select up to the number of looked-at cards
                let available_count = self.looked_at_cards.len();
                
                let description = if optional {
                    format!("Select up to {} card(s) from the {} looked-at cards (or skip) (placement_order: {})", 
                        count, available_count, placement_order.unwrap_or("default"))
                } else {
                    format!("Select up to {} card(s) from the {} looked-at cards (placement_order: {})", 
                        count, available_count, placement_order.unwrap_or("default"))
                };

                self.pending_choice = Some(Choice::SelectCard {
                    zone: "looked_at".to_string(),
                    card_type: None,
                    count: available_count,
                    description,
                    allow_skip: optional,
                });
                // Return early - execution will continue after user provides choice
                return Ok(());
            }

            // Otherwise execute select action normally
            self.execute_effect(select_action)?;
        }

        Ok(())
    }

    fn execute_draw(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let max = effect.max.unwrap_or(false);
        let target = effect.target.as_deref().unwrap_or("self");
        let source = effect.source.as_deref().unwrap_or("deck");
        let destination = effect.destination.as_deref().unwrap_or("hand");
        let card_type_filter = effect.card_type.as_deref();
        let resource_icon_count = effect.resource_icon_count;
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));
        let cost_limit = effect.cost_limit;
        let per_unit = effect.per_unit;
        let per_unit_count = effect.per_unit_count.unwrap_or(1);
        let per_unit_type = effect.per_unit_type.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();

        // Helper function to check if card matches type filter
        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true, // Unknown type, allow all
            }
        };

        // Helper function to check if card matches group filter
        let matches_group = |card_id: i16, filter: Option<&String>| -> bool {
            match filter {
                Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                None => true,
            }
        };

        // Helper function to check if card matches cost limit
        let matches_cost_limit = |card_id: i16, limit: Option<u32>| -> bool {
            match limit {
                Some(max_cost) => card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0) <= max_cost).unwrap_or(false),
                None => true,
            }
        };

        // Calculate final count with per-unit scaling
        let final_count = if per_unit.unwrap_or(false) {
            // Calculate based on per_unit_count and per_unit_type
            let multiplier = match per_unit_type {
                Some("member") => player.stage.stage.iter().filter(|&&c| c != -1).count() as u32,
                Some("energy") => player.energy_zone.cards.len() as u32,
                Some("hand") => player.hand.cards.len() as u32,
                _ => 1,
            };
            count * multiplier * per_unit_count
        } else {
            count
        };

        match source {
            "deck" | "deck_top" => {
                let mut drawn = 0;
                while drawn < final_count {
                    if let Some(card) = player.main_deck.draw() {
                        if matches_card_type(card, card_type_filter) && matches_group(card, group_filter) && matches_cost_limit(card, cost_limit) {
                            match destination {
                                "hand" => player.hand.add_card(card),
                                _ => {
                                    eprintln!("Draw destination '{}' not yet implemented", destination);
                                    player.hand.add_card(card); // Default to hand
                                }
                            }
                            drawn += 1;
                        } else {
                            // Card doesn't match filter, put it back on bottom of deck
                            player.main_deck.cards.push(card);
                            break; // Stop if we encounter non-matching card
                        }
                    } else {
                        break; // Deck empty
                    }
                }
            }
            _ => {
                eprintln!("Draw from source '{}' not yet implemented", source);
            }
        }

        // Log resource_icon_count if present (for debugging/verification)
        if let Some(ric) = resource_icon_count {
            eprintln!("Draw action had resource_icon_count: {}", ric);
        }

        Ok(())
    }

    fn execute_draw_until_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target_count = effect.count.unwrap_or(1) as usize;
        let target = effect.target.as_deref().unwrap_or("self");
        let _source = effect.source.as_deref().unwrap_or("deck");
        let destination = effect.destination.as_deref().unwrap_or("hand");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let current_count = match destination {
            "hand" => player.hand.len(),
            _ => {
                eprintln!("Draw until count for destination '{}' not yet implemented", destination);
                return Ok(());
            }
        };

        let to_draw = target_count.saturating_sub(current_count);

        // Use execute_draw with the calculated count
        let mut draw_effect = effect.clone();
        draw_effect.count = Some(to_draw as u32);
        self.execute_draw(&draw_effect)
    }

    fn execute_place_energy_under_member(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let count = effect.count.unwrap_or(1);
        let position = effect.position.as_ref().and_then(|p| p.position.as_deref());

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Get energy cards from energy zone
        let mut energy_cards = Vec::new();
        for _ in 0..count {
            if let Some(energy_card) = player.energy_zone.cards.pop() {
                energy_cards.push(energy_card);
            } else {
                break; // No more energy cards available
            }
        }

        // Determine target stage position
        let target_index = match position {
            Some("center") | Some("中央") => 1,
            Some("left") | Some("左側") => 0,
            Some("right") | Some("右側") => 2,
            None => {
                // Default to center if available, otherwise first occupied
                if player.stage.stage[1] != -1 {
                    1
                } else if player.stage.stage[0] != -1 {
                    0
                } else if player.stage.stage[2] != -1 {
                    2
                } else {
                    // No member on stage, put energy cards back
                    for card in energy_cards {
                        player.energy_zone.cards.push(card);
                    }
                    return Ok(());
                }
            }
            _ => 1,
        };

        // Check if there's a member at the target position
        if player.stage.stage[target_index] == -1 {
            // No member, put energy cards back
            for card in energy_cards {
                player.energy_zone.cards.push(card);
            }
            return Ok(());
        }

        // Place energy cards under the member
        // For now, track this as a blade modifier on the member card
        let member_card_id = player.stage.stage[target_index];
        for _ in energy_cards {
            self.game_state.add_blade_modifier(member_card_id, 1);
        }

        Ok(())
    }

    fn execute_activation_cost(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // This action modifies the activation cost of abilities
        let operation = effect.operation.as_deref().unwrap_or("increase");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let duration = effect.duration.as_deref();
        
        // Track the cost modification as a prohibition effect
        // This is a simplified implementation - a full implementation would modify actual ability costs
        let prohibition_text = format!("activation_cost_{}_{}", operation, value);
        
        match target {
            "self" => {
                self.game_state.prohibition_effects.push(prohibition_text);
            }
            "opponent" => {
                self.game_state.prohibition_effects.push(prohibition_text);
            }
            _ => {}
        }
        
        // Handle duration for temporary cost modifications
        if let Some(duration_str) = duration {
            if duration_str != "permanent" {
                let duration_enum = match duration_str {
                    "live_end" => crate::game_state::Duration::LiveEnd,
                    "this_turn" => crate::game_state::Duration::ThisTurn,
                    "this_live" => crate::game_state::Duration::ThisLive,
                    _ => crate::game_state::Duration::ThisLive,
                };
                
                let temp_effect = crate::game_state::TemporaryEffect {
                    effect_type: format!("activation_cost_{}_{}", operation, value),
                    duration: duration_enum,
                    created_turn: self.game_state.turn_number,
                    created_phase: self.game_state.current_phase.clone(),
                    target_player_id: target.to_string(),
                    description: format!("Modify activation cost by {} {}", operation, value),
                };
                self.game_state.temporary_effects.push(temp_effect);
            }
        }
        
        Ok(())
    }

    fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let max = effect.max.unwrap_or(false);
        let source = effect.source.as_deref().unwrap_or("");
        let destination = effect.destination.as_deref().unwrap_or("");
        let card_type_filter = effect.card_type.as_deref();
        let target = effect.target.as_deref().unwrap_or("self");
        let optional = effect.optional.unwrap_or(false);
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));
        let cost_limit = effect.cost_limit;
        let placement_order = effect.placement_order.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();

        // Helper function to check if card matches cost limit
        let matches_cost_limit = |card_id: i16, limit: Option<u32>| -> bool {
            match limit {
                Some(max_cost) => card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0) <= max_cost).unwrap_or(false),
                None => true,
            }
        };

        // Handle optional costs - only for AUTO abilities, not ACTIVATION abilities
        // Rule 9.4.2.2: Activation ability costs are mandatory
        // Rule 9.7.3.1.1: Auto abilities can have optional costs
        if optional {
            // Check if this is an activation ability - if so, cost is mandatory
            if let Some(current_ability) = &self.current_ability {
                if current_ability.triggers.as_ref().map_or(false, |t| t == "起動") {
                    // Activation ability - cost is mandatory, ignore optional flag
                    eprintln!("Activation ability - cost is mandatory, proceeding with payment");
                } else {
                    // Auto ability - optional cost, show card selection with skip option
                    // For costs that require card selection (e.g., discard from hand), show the selection UI directly
                    // with a skip option, rather than a separate pay/skip prompt
                    if source == "hand" || source == "deck" || source == "discard" || source == "energy_zone" {
                        // Cost requires card selection - show selection UI with skip option
                        let count_to_select = count;
                        let card_type_filter = card_type_filter;
                        
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: source.to_string(),
                            card_type: card_type_filter.map(|s| s.to_string()),
                            count: count_to_select as usize,
                            description: format!("Select card(s) to pay optional cost (or skip): {}", effect.text),
                            allow_skip: true,
                        });
                        
                        // Store effect for resuming after choice
                        self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                            card_no: "optional_cost".to_string(),
                            player_id: "self".to_string(),
                            action_index: 0,
                            effect: effect.clone(),
                            conditional_choice: None,
                        });
                        
                        return Ok(());
                    } else {
                        // Non-card-selection cost (e.g., state change, energy) - show pay/skip prompt
                        // Show more descriptive message about what the cost does
                        let cost_description = if effect.text.contains("ウェイト") {
                            "Put this member to wait state"
                        } else if effect.text.contains("エネルギー") || effect.text.contains("E") {
                            "Pay energy"
                        } else if effect.text.contains("控え室") {
                            "Send card to discard"
                        } else {
                            "Pay cost"
                        };
                        
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "pay_optional_cost:skip_optional_cost".to_string(),
                            description: format!("Pay optional cost: {}? (pay or skip)", cost_description),
                        });
                        
                        // Store effect for resuming after choice
                        self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                            card_no: "optional_cost".to_string(),
                            player_id: "self".to_string(),
                            action_index: 0,
                            effect: effect.clone(),
                            conditional_choice: None,
                        });
                        
                        return Ok(());
                    }
                }
            } else {
                // No current ability context, assume auto ability with optional cost
                if source == "hand" || source == "deck" || source == "discard" || source == "energy_zone" {
                    // Cost requires card selection - show selection UI with skip option
                    let count_to_select = count;
                    let card_type_filter = card_type_filter;
                    
                    self.pending_choice = Some(Choice::SelectCard {
                        zone: source.to_string(),
                        card_type: card_type_filter.map(|s| s.to_string()),
                        count: count_to_select as usize,
                        description: format!("Select card(s) to pay optional cost (or skip): {}", effect.text),
                        allow_skip: true,
                    });
                    
                    // Store effect for resuming after choice
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: effect.clone(),
                        conditional_choice: None,
                    });
                    
                    return Ok(());
                } else {
                    // Non-card-selection cost - show pay/skip prompt
                    // Show more descriptive message about what the cost does
                    let cost_description = if effect.text.contains("ウェイト") {
                        "Put this member to wait state"
                    } else if effect.text.contains("エネルギー") || effect.text.contains("E") {
                        "Pay energy"
                    } else if effect.text.contains("控え室") {
                        "Send card to discard"
                    } else {
                        "Pay cost"
                    };
                    
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay optional cost: {}? (pay or skip)", cost_description),
                    });
                    
                    // Store effect for resuming after choice
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: effect.clone(),
                        conditional_choice: None,
                    });
                    
                    return Ok(());
                }
            }
        }

        // Helper function to check if card matches type filter
        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true, // Unknown type, allow all
            }
        };

        // Helper function to check if card matches group filter
        let matches_group = |card_id: i16, filter: Option<&String>| -> bool {
            match filter {
                Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                None => true,
            }
        };

        match source {
            "stage" => {
                // Move card from stage to destination
                let mut moved = 0;
                // For now, move from center position - would need position parameter
                if player.stage.stage[1] != -1 {
                    let center_card = player.stage.stage[1];
                    player.stage.stage[1] = -1;
                    if matches_card_type(center_card, card_type_filter) && matches_group(center_card, group_filter) && matches_cost_limit(center_card, cost_limit) {
                        match destination {
                            "discard" => {
                                player.waitroom.add_card(center_card);
                                moved += 1;
                            }
                            "hand" => {
                                player.hand.add_card(center_card);
                                moved += 1;
                            }
                            "deck_bottom" => {
                                player.main_deck.cards.push(center_card);
                                moved += 1;
                            }
                            "deck_top" => {
                                player.main_deck.cards.insert(0, center_card);
                                moved += 1;
                            }
                            "live_card_zone" => {
                                player.live_card_zone.cards.push(center_card);
                                moved += 1;
                            }
                            "success_live_zone" => {
                                player.success_live_card_zone.cards.push(center_card);
                                moved += 1;
                            }
                            "under_member" => {
                                // Place card underneath a member on stage (as energy)
                                // For now, place under center - would need position parameter
                                // Energy tracking moved to GameState modifiers
                                // For now, just put card back to hand
                                player.hand.add_card(center_card);
                            }
                            "empty_area" => {
                                // Place in first empty stage area
                                if player.stage.stage[1] == -1 {
                                    player.stage.stage[1] = center_card;
                                    moved += 1;
                                } else if player.stage.stage[0] == -1 {
                                    player.stage.stage[0] = center_card;
                                    moved += 1;
                                } else if player.stage.stage[2] == -1 {
                                    player.stage.stage[2] = center_card;
                                    moved += 1;
                                } else {
                                    // No empty area, put card back to hand
                                    player.hand.add_card(center_card);
                                }
                            }
                            "same_area" => {
                                // Move to same area (no-op for stage to stage)
                                player.hand.add_card(center_card);
                            }
                            _ => {
                                // Put card back to hand
                                player.hand.add_card(center_card);
                            }
                        }
                    } else {
                        // Put card back if it doesn't match
                        player.hand.add_card(center_card);
                    }
                }
                // Also check left and right sides
                if moved < count {
                    if player.stage.stage[0] != -1 {
                        let left_card_id = player.stage.stage[0];
                        if matches_card_type(left_card_id, card_type_filter) && matches_group(left_card_id, group_filter) && matches_cost_limit(left_card_id, cost_limit) {
                            match destination {
                                "discard" => {
                                    player.waitroom.add_card(left_card_id);
                                    moved += 1;
                                }
                                "hand" => {
                                    player.hand.add_card(left_card_id);
                                    moved += 1;
                                }
                                "deck_bottom" => {
                                    player.main_deck.cards.push(left_card_id);
                                    moved += 1;
                                }
                                "deck_top" => {
                                    player.main_deck.cards.insert(0, left_card_id);
                                    moved += 1;
                                }
                                "live_card_zone" => {
                                    player.live_card_zone.cards.push(left_card_id);
                                    moved += 1;
                                }
                                "success_live_zone" => {
                                    player.success_live_card_zone.cards.push(left_card_id);
                                    moved += 1;
                                }
                                "under_member" => {
                                    // Place card underneath a member on stage (as energy)
                                    // Energy tracking moved to GameState modifiers
                                    player.hand.add_card(left_card_id);
                                }
                                "empty_area" => {
                                    // Place in first empty stage area
                                    if player.stage.stage[1] == -1 {
                                        player.stage.stage[1] = left_card_id;
                                        moved += 1;
                                    } else if player.stage.stage[0] == -1 {
                                        player.stage.stage[0] = left_card_id;
                                        moved += 1;
                                    } else if player.stage.stage[2] == -1 {
                                        player.stage.stage[2] = left_card_id;
                                        moved += 1;
                                    } else {
                                        // No empty area, put card back to hand
                                        player.hand.add_card(left_card_id);
                                    }
                                }
                                "same_area" => {
                                    // Move to same area (no-op for stage to stage)
                                    player.hand.add_card(left_card_id);
                                }
                                _ => {
                                    player.hand.add_card(left_card_id);
                                }
                            }
                        } else {
                            // Card doesn't match, put it back (already in stage)
                        }
                    }
                }
                if moved < count {
                    if player.stage.stage[2] != -1 {
                        let right_card_id = player.stage.stage[2];
                        if matches_card_type(right_card_id, card_type_filter) && matches_group(right_card_id, group_filter) && matches_cost_limit(right_card_id, cost_limit) {
                            match destination {
                                "discard" => {
                                    player.waitroom.add_card(right_card_id);
                                }
                                "hand" => {
                                    player.hand.add_card(right_card_id);
                                }
                                "deck_bottom" => {
                                    player.main_deck.cards.push(right_card_id);
                                }
                                "deck_top" => {
                                    player.main_deck.cards.insert(0, right_card_id);
                                }
                                "live_card_zone" => {
                                    player.live_card_zone.cards.push(right_card_id);
                                }
                                "success_live_zone" => {
                                    player.success_live_card_zone.cards.push(right_card_id);
                                }
                                "under_member" => {
                                    // Place card underneath a member on stage (as energy)
                                    // Energy tracking moved to GameState modifiers
                                    player.hand.add_card(right_card_id);
                                }
                                "empty_area" => {
                                    // Place in first empty stage area
                                    if player.stage.stage[1] == -1 {
                                        player.stage.stage[1] = right_card_id;
                                    } else if player.stage.stage[0] == -1 {
                                        player.stage.stage[0] = right_card_id;
                                    } else if player.stage.stage[2] == -1 {
                                        player.stage.stage[2] = right_card_id;
                                    } else {
                                        // No empty area, put card back to hand
                                        player.hand.add_card(right_card_id);
                                    }
                                }
                                "same_area" => {
                                    // Move to same area (no-op for stage to stage)
                                    player.hand.add_card(right_card_id);
                                }
                                _ => {
                                    player.hand.add_card(right_card_id);
                                }
                            }
                        } else {
                            // Card doesn't match, put it back (already in stage)
                        }
                    }
                }
            }
            "deck" | "deck_top" => {
                let mut moved = 0;
                while moved < count {
                    if let Some(card) = player.main_deck.draw() {
                        if matches_card_type(card, card_type_filter) && matches_group(card, group_filter) && matches_cost_limit(card, cost_limit) {
                            match destination {
                                "hand" => player.hand.add_card(card),
                                "discard" => player.waitroom.add_card(card),
                                "stage" => {
                                    // Place in first available stage area
                                    if player.stage.stage[1] == -1 {
                                        player.stage.stage[1] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                                    } else if player.stage.stage[0] == -1 {
                                        player.stage.stage[0] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                                    } else if player.stage.stage[2] == -1 {
                                        player.stage.stage[2] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                                    } else {
                                        player.hand.add_card(card); // Fallback to hand
                                    }
                                }
                                "live_card_zone" => {
                                    player.live_card_zone.cards.push(card);
                                }
                                "success_live_zone" => {
                                    player.success_live_card_zone.cards.push(card);
                                }
                                "deck_top" => {
                                    player.main_deck.cards.insert(0, card);
                                }
                                _ => {}
                            }
                            moved += 1;
                        } else {
                            // Card doesn't match filter, put it back
                            player.main_deck.cards.push(card);
                            break; // Stop if we encounter non-matching card
                        }
                    } else {
                        break; // Deck empty
                    }
                }
            }
            "hand" => {
                match destination {
                    "discard" => {
                        // Request user selection for cards to discard
                        let card_type_desc = if let Some(ct) = card_type_filter {
                            format!("{} ", ct)
                        } else {
                            "".to_string()
                        };
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "hand".to_string(),
                            card_type: card_type_filter.map(|s| s.to_string()),
                            count: count as usize,
                            description: format!("Select {} {}card(s) from hand to discard", count, card_type_desc),
                            allow_skip: false,
                        });
                        // Return early - execution will continue after user provides choice
                        return Ok(());
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            player.main_deck.cards.push(card);
                        }
                    }
                    "deck_top" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            player.main_deck.cards.insert(0, card);
                        }
                    }
                    "stage" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            // Place in first available stage area
                            if player.stage.stage[1] == -1 {
                                player.stage.stage[1] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                            } else if player.stage.stage[0] == -1 {
                                player.stage.stage[0] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                            } else if player.stage.stage[2] == -1 {
                                player.stage.stage[2] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                            } else {
                                player.hand.add_card(card); // Fallback
                            }
                        }
                    }
                    "live_card_zone" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            player.live_card_zone.cards.push(card);
                        }
                    }
                    _ => {}
                }
            }
            "discard" => {
                match destination {
                    "hand" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            player.hand.add_card(card);
                        }
                        player.rebuild_hand_index_map();
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            player.main_deck.cards.push(card);
                        }
                    }
                    "deck_top" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            player.main_deck.cards.insert(0, card);
                        }
                    }
                    "stage" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            let card_id = card;
                            // Try to place card in first available stage area (center, left, right)
                            if player.stage.stage[1] == -1 {
                                player.stage.stage[1] = card_id;
                                // Rule 9.6.2.1.2.1: Lock area when card moves from non-stage to stage via ability
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                            } else if player.stage.stage[0] == -1 {
                                player.stage.stage[0] = card_id;
                                // Rule 9.6.2.1.2.1: Lock area when card moves from non-stage to stage via ability
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                            } else if player.stage.stage[2] == -1 {
                                player.stage.stage[2] = card_id;
                                // Rule 9.6.2.1.2.1: Lock area when card moves from non-stage to stage via ability
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                            } else {
                                player.hand.add_card(card_id); // Fallback
                            }
                        }
                    }
                    "live_card_zone" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            player.live_card_zone.cards.push(card);
                        }
                    }
                    "success_live_zone" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            player.success_live_card_zone.cards.push(card);
                        }
                    }
                    _ => {}
                }
            }
            "success_live_zone" => {
                match destination {
                    "hand" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.success_live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            player.hand.add_card(card);
                        }
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.success_live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            player.main_deck.cards.push(card);
                        }
                    }
                    "deck_top" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.success_live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            player.main_deck.cards.insert(0, card);
                        }
                    }
                    "stage" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.success_live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            let card_id = card;
                            // Try to place card in first available stage area (center, left, right)
                            if player.stage.stage[1] == -1 {
                                player.stage.stage[1] = card_id;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                            } else if player.stage.stage[0] == -1 {
                                player.stage.stage[0] = card_id;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                            } else if player.stage.stage[2] == -1 {
                                player.stage.stage[2] = card_id;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                            } else {
                                player.hand.add_card(card_id); // Fallback
                            }
                        }
                    }
                    _ => {}
                }
            }
            "live_card_zone" => {
                match destination {
                    "hand" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            player.hand.add_card(card);
                        }
                    }
                    "discard" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            player.waitroom.add_card(card);
                        }
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            player.main_deck.cards.push(card);
                        }
                    }
                    "deck_top" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            player.main_deck.cards.insert(0, card);
                        }
                    }
                    "success_live_zone" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.live_card_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            player.success_live_card_zone.cards.push(card);
                        }
                    }
                    _ => {}
                }
            }
            "energy_zone" => {
                match destination {
                    "hand" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) && matches_cost_limit(*card_id, cost_limit) {
                                indices_to_remove.push((i, *card_id));
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for (i, card_id) in indices_to_remove.into_iter().rev() {
                            player.energy_zone.cards.remove(i);
                            // Decrement active_energy_count if the removed card was active
                            if player.energy_zone.active_energy_count > 0 {
                                player.energy_zone.active_energy_count -= 1;
                            }
                            player.hand.add_card(card_id);
                        }
                        player.rebuild_hand_index_map();
                    }
                    "discard" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) && matches_cost_limit(*card_id, cost_limit) {
                                indices_to_remove.push((i, *card_id));
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for (i, card_id) in indices_to_remove.into_iter().rev() {
                            player.energy_zone.cards.remove(i);
                            // Decrement active_energy_count if the removed card was active
                            if player.energy_zone.active_energy_count > 0 {
                                player.energy_zone.active_energy_count -= 1;
                            }
                            player.waitroom.cards.push(card_id);
                        }
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) && matches_cost_limit(*card_id, cost_limit) {
                                indices_to_remove.push((i, *card_id));
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for (i, card_id) in indices_to_remove.into_iter().rev() {
                            player.energy_zone.cards.remove(i);
                            // Decrement active_energy_count if the removed card was active
                            if player.energy_zone.active_energy_count > 0 {
                                player.energy_zone.active_energy_count -= 1;
                            }
                            player.main_deck.cards.push(card_id);
                        }
                    }
                    "deck_top" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) && matches_cost_limit(*card_id, cost_limit) {
                                indices_to_remove.push((i, *card_id));
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for (i, card_id) in indices_to_remove.into_iter().rev() {
                            player.energy_zone.cards.remove(i);
                            // Decrement active_energy_count if the removed card was active
                            if player.energy_zone.active_energy_count > 0 {
                                player.energy_zone.active_energy_count -= 1;
                            }
                            player.main_deck.cards.insert(0, card_id);
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                eprintln!("Unknown source for move_cards: {}", source);
            }
        }

        Ok(())
    }

    fn execute_gain_resource(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let resource = effect.resource.as_deref().unwrap_or("");
        // Use resource_icon_count if available, otherwise fall back to count
        let count = effect.resource_icon_count.unwrap_or(effect.count.unwrap_or(1));
        let target = effect.target.as_deref().unwrap_or("self");
        let duration = effect.duration.as_deref();
        let card_type_filter = effect.card_type.as_deref();
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));
        let per_unit = effect.per_unit;
        let per_unit_count = effect.per_unit_count.unwrap_or(1);
        let per_unit_type = effect.per_unit_type.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();

        // Helper function to check if card matches type filter
        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true, // Unknown type, allow all
            }
        };

        // Helper function to check if card matches group filter
        let matches_group = |card_id: i16, filter: Option<&String>| -> bool {
            match filter {
                Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                None => true,
            }
        };

        // Calculate count based on per_unit if specified
        let final_count = if per_unit == Some(true) {
            // Count matching cards and multiply by per_unit_count
            let matching_count = match per_unit_type {
                Some("stage") => {
                    player.stage.stage.iter()
                        .filter(|&&card_id| card_id != -1)
                        .filter(|&card_id| matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter))
                        .count() as u32
                }
                Some("hand") => {
                    player.hand.cards.iter()
                        .filter(|&card_id| matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter))
                        .count() as u32
                }
                _ => {
                    // Default to stage count
                    player.stage.stage.iter()
                        .filter(|&&card_id| card_id != -1)
                        .filter(|&card_id| matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter))
                        .count() as u32
                }
            };
            matching_count * per_unit_count
        } else {
            count
        };

        // Handle duration - if specified, create temporary effect
        let is_temporary = duration.is_some() && duration != Some("permanent");
        if is_temporary {
            let duration_enum = match duration {
                Some("live_end") => crate::game_state::Duration::LiveEnd,
                Some("this_turn") => crate::game_state::Duration::ThisTurn,
                Some("this_live") => crate::game_state::Duration::ThisLive,
                Some("as_long_as") => crate::game_state::Duration::ThisLive, // Treat as this_live for now
                _ => crate::game_state::Duration::ThisLive,
            };

            let temp_effect = crate::game_state::TemporaryEffect {
                effect_type: format!("gain_resource_{}", resource),
                duration: duration_enum,
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: player.id.clone(),
                description: format!("Gain {} {} for {}", final_count, resource, target),
            };
            self.game_state.temporary_effects.push(temp_effect);
        }

        match resource {
            "blade" => {
                // Add blades to stage cards using modifier tracking, with filters
                let card_ids: Vec<i16> = vec![
                    player.stage.stage[0],
                    player.stage.stage[1],
                    player.stage.stage[2],
                ].into_iter().filter(|&card_id| card_id != -1)
                 .filter(|&card_id| matches_card_type(card_id, card_type_filter) && matches_group(card_id, group_filter))
                 .collect();
                for card_id in card_ids {
                    self.game_state.add_blade_modifier(card_id, final_count as i32);
                }
            }
            "heart" => {
                // Add hearts to stage cards using modifier tracking, with filters
                let color = crate::card::HeartColor::Heart01; // Default to heart01
                let card_ids: Vec<i16> = vec![
                    player.stage.stage[0],
                    player.stage.stage[1],
                    player.stage.stage[2],
                ].into_iter().filter(|&card_id| card_id != -1)
                 .filter(|&card_id| matches_card_type(card_id, card_type_filter) && matches_group(card_id, group_filter))
                 .collect();
                for card_id in card_ids {
                    self.game_state.add_heart_modifier(card_id, color, final_count as i32);
                }
            }
            "energy" => {
                // Add energy cards to energy zone
                // This would need to actually create/add energy cards
                eprintln!("Gain resource: energy (not fully implemented)");
            }
            _ => {
                eprintln!("Unknown resource type: {}", resource);
            }
        }

        Ok(())
    }

    fn execute_change_state(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let state_change = effect.state_change.as_deref().unwrap_or("");
        let count = effect.count.unwrap_or(1);
        let max = effect.max.unwrap_or(false);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));
        let cost_limit = effect.cost_limit;
        let optional = effect.optional.unwrap_or(false);
        let destination = effect.destination.as_deref();
        let source = effect.source.as_deref().unwrap_or("");

        // Handle optional costs - only for AUTO abilities, not ACTIVATION abilities
        // Rule 9.4.2.2: Activation ability costs are mandatory
        // Rule 9.7.3.1.1: Auto abilities can have optional costs
        if optional {
            // Check if this is an activation ability - if so, cost is mandatory
            if let Some(current_ability) = &self.current_ability {
                if current_ability.triggers.as_ref().map_or(false, |t| t == "起動") {
                    // Activation ability - cost is mandatory, ignore optional flag
                    eprintln!("Activation ability - cost is mandatory, proceeding with payment");
                } else {
                    // Auto ability - optional cost, show card selection with skip option
                    // For costs that require card selection (e.g., discard from hand), show the selection UI directly
                    // with a skip option, rather than a separate pay/skip prompt
                    if source == "hand" || source == "deck" || source == "discard" || source == "energy_zone" {
                        // Cost requires card selection - show selection UI with skip option
                        let count_to_select = count;
                        let card_type_filter = card_type_filter;
                        
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: source.to_string(),
                            card_type: card_type_filter.map(|s| s.to_string()),
                            count: count_to_select as usize,
                            description: format!("Select card(s) to pay optional cost (or skip): {}", effect.text),
                            allow_skip: true,
                        });
                        
                        // Store effect for resuming after choice
                        self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                            card_no: "optional_cost".to_string(),
                            player_id: "self".to_string(),
                            action_index: 0,
                            effect: effect.clone(),
                            conditional_choice: None,
                        });
                        
                        return Ok(());
                    } else {
                        // Non-card-selection cost (e.g., state change, energy) - show pay/skip prompt
                        // Show more descriptive message about what the cost does
                        let cost_description = if effect.text.contains("ウェイト") {
                            "Put this member to wait state"
                        } else if effect.text.contains("エネルギー") || effect.text.contains("E") {
                            "Pay energy"
                        } else if effect.text.contains("控え室") {
                            "Send card to discard"
                        } else {
                            "Pay cost"
                        };
                        
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "pay_optional_cost:skip_optional_cost".to_string(),
                            description: format!("Pay optional cost: {}? (pay or skip)", cost_description),
                        });
                        
                        // Store effect for resuming after choice
                        self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                            card_no: "optional_cost".to_string(),
                            player_id: "self".to_string(),
                            action_index: 0,
                            effect: effect.clone(),
                            conditional_choice: None,
                        });
                        
                        return Ok(());
                    }
                }
            } else {
                // No current ability context, assume auto ability with optional cost
                if source == "hand" || source == "deck" || source == "discard" || source == "energy_zone" {
                    // Cost requires card selection - show selection UI with skip option
                    let count_to_select = count;
                    let card_type_filter = card_type_filter;
                    
                    self.pending_choice = Some(Choice::SelectCard {
                        zone: source.to_string(),
                        card_type: card_type_filter.map(|s| s.to_string()),
                        count: count_to_select as usize,
                        description: format!("Select card(s) to pay optional cost (or skip): {}", effect.text),
                        allow_skip: true,
                    });
                    
                    // Store effect for resuming after choice
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: effect.clone(),
                        conditional_choice: None,
                    });
                    
                    return Ok(());
                } else {
                    // Non-card-selection cost - show pay/skip prompt
                    // Show more descriptive message about what the cost does
                    let cost_description = if effect.text.contains("ウェイト") {
                        "Put this member to wait state"
                    } else if effect.text.contains("エネルギー") || effect.text.contains("E") {
                        "Pay energy"
                    } else if effect.text.contains("控え室") {
                        "Send card to discard"
                    } else {
                        "Pay cost"
                    };
                    
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay optional cost: {}? (pay or skip)", cost_description),
                    });
                    
                    // Store effect for resuming after choice
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: effect.clone(),
                        conditional_choice: None,
                    });
                    
                    return Ok(());
                }
            }
        }

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();

        // Helper function to check if card matches type filter
        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true, // Unknown type, allow all
            }
        };

        // Helper function to check if card matches group filter
        let matches_group = |card_id: i16, filter: Option<&String>| -> bool {
            match filter {
                Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                None => true,
            }
        };

        // Helper function to check if card matches cost limit
        let matches_cost_limit = |card_id: i16, limit: Option<u32>| -> bool {
            match limit {
                Some(max_cost) => card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0) <= max_cost).unwrap_or(false),
                None => true,
            }
        };

        match state_change {
            "active" => {
                if count == 2 {
                    player.energy_zone.activate_all();
                } else {
                    // Activate specific energy cards based on count and filters
                    let count_usize = count as usize;
                    let mut activated = 0;
                    for card_id in player.energy_zone.cards.iter() {
                        if activated >= count_usize {
                            break;
                        }
                        if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) && matches_cost_limit(*card_id, cost_limit) {
                            if activated < player.energy_zone.active_energy_count {
                                // Already active, skip
                                continue;
                            }
                            activated += 1;
                        }
                    }
                    let max_to_activate = player.energy_zone.cards.len().min(count_usize);
                    player.energy_zone.active_energy_count = max_to_activate;
                }
            }
            "wait" => {
                // Set energy cards to wait state
                let count_usize = count as usize;
                
                // Find all valid targets
                let valid_targets: Vec<i16> = player.energy_zone.cards.iter()
                    .filter(|&card_id| matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) && matches_cost_limit(*card_id, cost_limit))
                    .copied()
                    .collect();
                
                // If multiple valid targets and count < valid_targets.len(), need user choice
                if valid_targets.len() > count_usize && count_usize > 0 {
                    let description = format!("Select {} energy card(s) to set to wait state from {} valid targets", count_usize, valid_targets.len());
                    
                    // Store valid targets temporarily for selection
                    self.looked_at_cards = valid_targets;
                    
                    self.pending_choice = Some(Choice::SelectCard {
                        zone: "energy_zone".to_string(),
                        card_type: card_type_filter.map(|s| s.to_string()),
                        count: count_usize,
                        description,
                        allow_skip: false,
                    });
                    // Return early - execution will continue after user provides choice
                    return Ok(());
                }
                
                // Otherwise, deactivate the first count_usize cards
                let mut deactivated = 0;
                for card_id in valid_targets.iter().take(count_usize) {
                    deactivated += 1;
                }
                let max_to_wait = player.energy_zone.active_energy_count.min(count_usize);
                player.energy_zone.active_energy_count -= max_to_wait;
            }
            _ => {}
        }

        Ok(())
    }

    fn execute_modify_score(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let duration = effect.duration.as_deref();
        let card_type_filter = effect.card_type.as_deref();
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();

        // Helper function to check if card matches type filter
        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true, // Unknown type, allow all
            }
        };

        // Helper function to check if card matches group filter
        let matches_group = |card_id: i16, filter: Option<&String>| -> bool {
            match filter {
                Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                None => true,
            }
        };

        // Collect card_ids first to avoid borrow conflicts
        let card_ids: Vec<i16> = player.stage.stage.iter()
            .filter(|&&card_id| card_id != -1)
            .filter(|&&card_id| matches_card_type(card_id, card_type_filter))
            .filter(|&&card_id| matches_group(card_id, group_filter))
            .copied()
            .collect();

        // Handle duration - if specified, create temporary effect
        let is_temporary = duration.is_some() && duration != Some("permanent");
        if is_temporary {
            let duration_enum = match duration {
                Some("live_end") => crate::game_state::Duration::LiveEnd,
                Some("this_turn") => crate::game_state::Duration::ThisTurn,
                Some("this_live") => crate::game_state::Duration::ThisLive,
                _ => crate::game_state::Duration::ThisLive,
            };

            let temp_effect = crate::game_state::TemporaryEffect {
                effect_type: format!("modify_score_{}_{}", operation, value),
                duration: duration_enum,
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: player.id.clone(),
                description: format!("Modify score by {} {} for {}", operation, value, target),
            };
            self.game_state.temporary_effects.push(temp_effect);
        }

        // Apply modifiers after releasing borrow
        match operation {
            "add" => {
                for card_id in card_ids {
                    self.game_state.add_score_modifier(card_id, value as i32);
                }
            }
            "remove" => {
                for card_id in card_ids {
                    self.game_state.add_score_modifier(card_id, -(value as i32));
                }
            }
            "set" => {
                for card_id in card_ids {
                    self.game_state.score_modifiers.insert(card_id, value as i32);
                }
            }
            _ => return Err(format!("Unknown operation: {}", operation)),
        }

        Ok(())
    }

// ...
    fn execute_modify_required_hearts(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("decrease");
        let value = effect.value.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Get card IDs from live card zone
        let card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();

        if card_ids.is_empty() {
            return Ok(()); // No live cards to modify
        }

        let delta = match operation {
            "decrease" => -(value as i8),
            "increase" => value as i8,
            "set" => value as i8,
            _ => return Err(format!("Unknown operation: {}", operation)),
        };

        let color = crate::zones::parse_heart_color(heart_color);

        for card_id in card_ids {
            if operation == "set" {
                self.game_state.set_need_heart_modifier(card_id, color, delta as i32);
            } else {
                self.game_state.add_need_heart_modifier(card_id, color, delta as i32);
            }
        }

        Ok(())
    }

    fn execute_set_cost(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("set");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Target specific cards based on card_type_filter
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type_filter {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else if let Some("energy_card") = card_type_filter {
            player.energy_zone.cards.iter().copied().collect()
        } else {
            // Default to hand
            player.hand.cards.iter().copied().collect()
        };

        let delta = match operation {
            "add" => value as i32,
            "subtract" => -(value as i32),
            "set" => value as i32,
            _ => return Err(format!("Unknown operation: {}", operation)),
        };

        for card_id in card_ids {
            if operation == "set" {
                self.game_state.set_cost_modifier(card_id, delta);
            } else {
                self.game_state.add_cost_modifier(card_id, delta);
            }
        }

        Ok(())
    }

    fn execute_set_blade_type(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let blade_type_str = effect.blade_type.as_deref().unwrap_or("all");
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let duration = effect.duration.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Parse blade color
        let blade_color = crate::zones::parse_blade_color(blade_type_str);

        // Target specific cards based on card_type_filter
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type_filter {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else if let Some("energy_card") = card_type_filter {
            player.energy_zone.cards.iter().copied().collect()
        } else {
            // Default to all cards in resolution zone (cheer cards)
            self.game_state.resolution_zone.cards.iter().copied().collect()
        };

        // Set blade type modifier for each card
        for card_id in card_ids {
            self.game_state.set_blade_type_modifier(card_id, blade_color);
        }

        // Track as temporary effect if duration is specified
        if let Some(dur) = duration {
            let duration_enum = match dur {
                "live_end" => crate::game_state::EffectDuration::LiveEnd,
                "as_long_as" => crate::game_state::EffectDuration::AsLongAs,
                "this_turn" => crate::game_state::EffectDuration::ThisTurn,
                _ => crate::game_state::EffectDuration::LiveEnd,
            };

            let temp_effect = crate::game_state::TemporaryEffect {
                effect_type: format!("set_blade_type_{}", blade_type_str),
                duration: duration_enum,
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: target.to_string(),
                description: format!("Set blade type to {}", blade_type_str),
            };
            self.game_state.temporary_effects.push(temp_effect);
        }

        Ok(())
    }

    fn execute_set_heart_type(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let value = effect.value.unwrap_or(1);

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Target specific cards based on card_type_filter
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type_filter {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else if let Some("energy_card") = card_type_filter {
            player.energy_zone.cards.iter().copied().collect()
        } else {
            // Default to hand
            player.hand.cards.iter().copied().collect()
        };

        let color = crate::zones::parse_heart_color(heart_color);

        for card_id in card_ids {
            self.game_state.add_heart_modifier(card_id, color, value as i32);
        }

        Ok(())
    }

    fn execute_activate_ability(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let ability_text = effect.text.clone();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Collect card IDs based on card_type_filter
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type_filter {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else if let Some("energy_card") = card_type_filter {
            player.energy_zone.cards.iter().copied().collect()
        } else {
            // Default to hand
            player.hand.cards.iter().copied().collect()
        };

        // For each card, find abilities matching the text and execute them
        let mut effects_to_execute: Vec<crate::card::AbilityEffect> = Vec::new();

        for card_id in card_ids {
            if let Some(card) = self.game_state.card_database.get_card(card_id) {
                for ability in &card.abilities {
                    if ability_text.is_empty() || ability.full_text.contains(&ability_text) {
                        // Clone the effect to avoid borrow conflicts
                        if let Some(ref effect_data) = ability.effect {
                            effects_to_execute.push(effect_data.clone());
                        }
                    }
                }
            }
        }

        // Execute collected effects
        for effect_data in effects_to_execute {
            self.execute_effect(&effect_data)?;
        }

        Ok(())
    }

    fn execute_invalidate_ability(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let ability_text = effect.text.clone();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Collect card IDs from stage (member cards)
        let card_ids: Vec<i16> = player.stage.stage.iter().filter(|&&id| id != -1).copied().collect();

        // Mark abilities as invalid by adding to prohibition effects
        for card_id in card_ids {
            if let Some(card) = self.game_state.card_database.get_card(card_id) {
                for ability in &card.abilities {
                    if ability_text.is_empty() || ability.full_text.contains(&ability_text) {
                        // Add ability prohibition - use full_text as identifier since Ability has no id field
                        let prohibition = format!("ability_invalid:{}", ability.full_text);
                        self.game_state.prohibition_effects.push(prohibition);
                    }
                }
            }
        }

        Ok(())
    }

    fn execute_gain_ability(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let duration = effect.duration.as_deref();
        let ability_text = effect.text.clone();

        // Track granted ability as a temporary effect
        // A full implementation would parse the ability and add it to the card
        // For now, track it as a prohibition effect to indicate the ability exists
        let granted_ability = format!("gain_ability:{}:{}:{}", target, ability_text, duration.unwrap_or("permanent"));
        self.game_state.prohibition_effects.push(granted_ability);

        Ok(())
    }

    fn execute_play_baton_touch(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let position = effect.position.as_ref().and_then(|p| p.position.as_deref());

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Unlock the specified stage area to allow baton touch
        if let Some(pos) = position {
            match pos {
                "center" | "中央" => {
                    player.areas_locked_this_turn.remove(&crate::zones::MemberArea::Center);
                }
                "left" | "左側" => {
                    player.areas_locked_this_turn.remove(&crate::zones::MemberArea::LeftSide);
                }
                "right" | "右側" => {
                    player.areas_locked_this_turn.remove(&crate::zones::MemberArea::RightSide);
                }
                _ => {
                    // If no specific position, unlock all areas
                    player.areas_locked_this_turn.clear();
                }
            }
        } else {
            // Unlock all areas if no position specified
            player.areas_locked_this_turn.clear();
        }

        Ok(())
    }

    fn execute_reveal(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let source = effect.source.as_deref().unwrap_or("");
        let count = effect.count.unwrap_or(1);
        let card_type_filter = effect.card_type.as_deref();
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Helper function to check if card matches type filter
        let matches_card_type = |card: &crate::card::Card, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card.is_live(),
                Some("member_card") => card.is_member(),
                Some("energy_card") => card.is_energy(),
                None => true,
                _ => true, // Unknown type, allow all
            }
        };

        match source {
            "hand" => {
                // Mark cards as revealed in hand
                let mut revealed_count = 0;
                for card_id in &player.hand.cards {
                    if revealed_count >= count {
                        break;
                    }
                    if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                        if matches_card_type(card, card_type_filter) {
                            // Mark card as revealed - for now we just log it
                            // In a full implementation, this would track revealed state
                            // and make the card visible to the opponent
                            println!("Revealed card: {} from hand", card.name);
                            revealed_count += 1;
                        }
                    }
                }
                if revealed_count < count {
                    eprintln!("Warning: Only {} cards revealed, requested {}", revealed_count, count);
                }
            }
            _ => {
                eprintln!("Reveal from source '{}' not yet implemented", source);
            }
        }

        Ok(())
    }

    fn execute_select(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Select cards from a location (typically after looking)
        let count = effect.count.unwrap_or(1);
        let source = effect.source.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let optional = effect.optional.unwrap_or(false);

        // Build more descriptive message
        let card_type_desc = if let Some(ct) = card_type_filter {
            format!("{} ", ct)
        } else {
            "".to_string()
        };
        
        let target_desc = if target == "self" { String::new() } else { format!("for {} ", target) };
        let description = if optional {
            format!("Select {} {}card(s) from {} {}(or skip)", count, card_type_desc, source, target_desc)
        } else {
            format!("Select {} {}card(s) from {} {}", count, card_type_desc, source, target_desc)
        };

        // Request user selection for cards
        self.pending_choice = Some(Choice::SelectCard {
            zone: source.to_string(),
            card_type: card_type_filter.map(|s| s.to_string()),
            count: count as usize,
            description: description,
            allow_skip: optional,
        });
        // Return early - execution will continue after user provides choice
        Ok(())
    }

    fn execute_look_at(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Look at cards from a location without revealing to opponent
        let count = effect.count.unwrap_or(1) as usize;
        let source = effect.source.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Clear previous looked-at cards
        self.looked_at_cards.clear();

        // Store looked-at cards for subsequent selection
        match source {
            "deck" | "deck_top" => {
                // Look at top count cards of deck
                let cards_to_look: Vec<i16> = player.main_deck.cards.iter()
                    .take(count)
                    .copied()
                    .collect();
                self.looked_at_cards = cards_to_look;
                eprintln!("Look at top {} cards of deck for {}", count, target);
            }
            "hand" => {
                // Look at count cards in hand
                let cards_to_look: Vec<i16> = player.hand.cards.iter()
                    .take(count)
                    .copied()
                    .collect();
                self.looked_at_cards = cards_to_look;
                eprintln!("Look at {} cards in hand for {}", count, target);
            }
            _ => {
                eprintln!("Look at {} cards from {} for {}", count, source, target);
            }
        }
        Ok(())
    }

    fn execute_modify_required_hearts_global(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Modify required hearts for all live cards in a zone
        let operation = effect.operation.as_deref().unwrap_or("increase");
        let _value = effect.value.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");

        let _player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // For now, just log the operation - would need target_location field
        eprintln!("Modify required hearts globally for player: {}", operation);
        Ok(())
    }

    fn execute_modify_yell_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Modify yell count (cheer blade/heart count)
        let operation = effect.operation.as_deref().unwrap_or("subtract");
        let count = effect.count.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        let _player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        // This would need to modify the cheer blade/heart count
        // For now, just log the operation
        match operation {
            "add" => {
                eprintln!("Add {} to yell count for player", count);
            }
            "subtract" => {
                eprintln!("Subtract {} from yell count for player", count);
            }
            _ => {
                eprintln!("Unknown modify_yell_count operation: {}", operation);
            }
        }

        Ok(())
    }

    /// Resolve a complete ability
    pub fn resolve_ability(&mut self, ability: &Ability) -> Result<(), String> {
        // Set current ability for optional cost handling
        self.current_ability = Some(ability.clone());
        
        // First, pay the cost if there is one
        if let Some(ref cost) = ability.cost {
            self.pay_cost(cost)?;
        }

        // Then execute the effect
        if let Some(ref effect) = ability.effect {
            self.execute_effect(effect)?;
        }

        // Clear current ability after execution
        self.current_ability = None;
        
        Ok(())
    }

    fn execute_position_change(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // position_change can be both:
        // 1. A condition trigger (when a member moves to a different area)
        // 2. An actual effect that moves a member to a different area

        // Check if it has a condition - if so, it's a trigger that executes nested effects
        if effect.condition.is_some() {
            if let Some(ref nested_effect) = effect.primary_effect {
                self.execute_effect(nested_effect)?;
            }
            return Ok(());
        }

        // Otherwise, it's an actual movement effect
        // Move a member to a different area
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));
        let count = effect.count.unwrap_or(1);

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();

        // Helper function to check if card matches type filter
        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true,
            }
        };

        // Helper function to check if card matches group filter
        let matches_group = |card_id: i16, filter: Option<&String>| -> bool {
            match filter {
                Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                None => true,
            }
        };

        // If destination is present, request user to select destination
        if let Some(ref destination) = effect.destination {
            // Parse destination options (if multiple destinations are available)
            let destinations: Vec<&str> = destination.split("|").collect();
            
            let description = if destinations.len() > 1 {
                // Multiple destinations available - show them in a numbered list
                let dest_display = destinations.iter()
                    .enumerate()
                    .map(|(i, dest)| format!("{}. {}", i + 1, dest))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("Select destination area:\n{}", dest_display)
            } else if effect.text.is_empty() {
                format!("Select destination area: {}", destination)
            } else {
                format!("{} (destination: {})", effect.text, destination)
            };
            
            self.pending_choice = Some(Choice::SelectTarget {
                target: destination.clone(),
                description: description,
            });
            // Store effect for resuming after choice
            self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                card_no: "position_change".to_string(),
                player_id: target.to_string(),
                action_index: 0,
                effect: effect.clone(),
                conditional_choice: None,
            });
            return Ok(());
        }

        // For now, implement simple position change: swap center and left
        // A full implementation would need to handle various source/destination combinations
        let mut moved = 0;
        if moved < count && player.stage.stage[1] != -1 && matches_card_type(player.stage.stage[1], card_type_filter) && matches_group(player.stage.stage[1], group_filter) {
            if player.stage.stage[0] != -1 {
                let center = player.stage.stage[1];
                let left = player.stage.stage[0];
                player.stage.stage[1] = left;
                player.stage.stage[0] = center;
                moved += 1;
            }
        }

        eprintln!("Position change executed: moved {} cards for {}", moved, target);
        Ok(())
    }

    fn execute_appear(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let source = effect.source.as_deref().unwrap_or("hand");
        let destination = effect.destination.as_deref().unwrap_or("stage");
        let target = effect.target.as_deref().unwrap_or("self");
        let count = effect.count.unwrap_or(1);

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        match source {
            "hand" => {
                match destination {
                    "stage" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, _card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            indices_to_remove.push(i);
                            moved += 1;
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            // Place in first available stage area (center, left, right)
                            if player.stage.stage[1] == -1 {
                                player.stage.stage[1] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                            } else if player.stage.stage[0] == -1 {
                                player.stage.stage[0] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                            } else if player.stage.stage[2] == -1 {
                                player.stage.stage[2] = card;
                                player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                            } else {
                                player.hand.add_card(card); // Fallback
                            }
                        }
                    }
                    _ => {
                        // Use move_cards for other destinations
                        let move_effect = AbilityEffect {
                            action: "move_cards".to_string(),
                            source: Some(source.to_string()),
                            destination: Some(destination.to_string()),
                            count: Some(count),
                            target: Some(target.to_string()),
                            ..Default::default()
                        };
                        return self.execute_move_cards(&move_effect);
                    }
                }
            }
            _ => {
                // Use move_cards for other sources
                let move_effect = AbilityEffect {
                    action: "move_cards".to_string(),
                    source: Some(source.to_string()),
                    destination: Some(destination.to_string()),
                    count: Some(count),
                    target: Some(target.to_string()),
                    ..Default::default()
                };
                return self.execute_move_cards(&move_effect);
            }
        }

        eprintln!("Appear executed: moved {} cards from {} to {} for {}", count, source, destination, target);
        Ok(())
    }

    fn execute_choice(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Execute one of the choice options
        if let Some(ref options) = effect.choice_options {
            // Request user to make a choice
            let description = if effect.text.is_empty() {
                "Make a choice".to_string()
            } else {
                effect.text.clone()
            };

            // Show options in a more readable format instead of joined by "|"
            let options_display = options.iter()
                .enumerate()
                .map(|(i, opt)| format!("{}. {}", i + 1, opt))
                .collect::<Vec<_>>()
                .join("\n");
            
            let full_description = format!("{}\n{}", description, options_display);

            self.pending_choice = Some(Choice::SelectTarget {
                target: options.join("|"),
                description: full_description,
            });
            // Store effect for resuming after choice
            self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                card_no: "choice".to_string(),
                player_id: "self".to_string(),
                action_index: 0,
                effect: effect.clone(),
                conditional_choice: None,
            });
            return Ok(());
        }

        Ok(())
    }

    fn execute_pay_energy(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let amount = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Use EnergyZone::pay_energy to actually tap energy cards
        if let Err(e) = player.energy_zone.pay_energy(amount as usize) {
            eprintln!("Failed to pay energy: {}", e);
            return Err(e);
        }

        eprintln!("Paid {} energy for {}", amount, target);
        Ok(())
    }

    fn execute_set_card_identity(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let group_name = effect.group.as_ref().map(|g| g.name.clone());

        // Track identity change as a prohibition effect
        // A full implementation would need identity override tracking in GameState
        let identity_change = if let Some(gn) = group_name {
            format!("set_card_identity:{}:group={}", target, gn)
        } else {
            format!("set_card_identity:{}", target)
        };
        self.game_state.prohibition_effects.push(identity_change);

        Ok(())
    }

    fn execute_discard_until_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target_count = effect.count.unwrap_or(0) as usize;
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let current_count = player.hand.len();
        if current_count > target_count {
            let cards_to_discard = current_count - target_count;
            
            // Request user selection for cards to discard
            self.pending_choice = Some(Choice::SelectCard {
                zone: "hand".to_string(),
                card_type: None,
                count: cards_to_discard,
                description: format!("Select {} card(s) from hand to discard until you have {} cards", cards_to_discard, target_count),
                allow_skip: false,
            });
            // Return early - execution will continue after user provides choice
            return Ok(());
        }

        Ok(())
    }

    fn execute_restriction(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Apply a restriction effect
        let restriction_type = effect.restriction_type.as_deref().unwrap_or("");
        let restricted_destination = effect.restricted_destination.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        // Store restriction in a format that can be checked later
        // Format: "restriction:<restriction_type>:<restricted_destination>:<target>"
        if !restriction_type.is_empty() {
            let restriction = if !restricted_destination.is_empty() {
                format!("restriction:{}:{}:{}", restriction_type, restricted_destination, target)
            } else {
                format!("restriction:{}:{}", restriction_type, target)
            };
            self.game_state.prohibition_effects.push(restriction);
        }

        eprintln!("Restriction applied: type={}, destination={}, target={}", 
            restriction_type, restricted_destination, target);
        Ok(())
    }

    fn execute_re_yell(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");

        let _player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Track re-yell as a temporary effect
        for _ in 0..count {
            self.game_state.prohibition_effects.push(format!("re_yell:{}", target));
        }

        Ok(())
    }

    fn execute_activation_restriction(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let restriction_type = effect.text.clone();
        let target = effect.target.as_deref().unwrap_or("self");

        // Track activation restriction as a prohibition effect
        let restriction = format!("activation_restriction:{}:{}", target, restriction_type);
        self.game_state.prohibition_effects.push(restriction);

        Ok(())
    }

    fn execute_choose_required_hearts(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Request user to choose required hearts
        let description = if effect.text.is_empty() {
            "Choose required hearts".to_string()
        } else {
            effect.text.clone()
        };

        if let Some(ref options) = effect.choice_options {
            // Show options in a more readable format instead of joined by "|"
            let options_display = options.iter()
                .enumerate()
                .map(|(i, opt)| format!("{}. {}", i + 1, opt))
                .collect::<Vec<_>>()
                .join("\n");
            
            let full_description = format!("{}\n{}", description, options_display);

            self.pending_choice = Some(Choice::SelectTarget {
                target: options.join("|"),
                description: full_description,
            });
            // Store effect for resuming after choice
            self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                card_no: "choose_required_hearts".to_string(),
                player_id: "self".to_string(),
                action_index: 0,
                effect: effect.clone(),
                conditional_choice: None,
            });
            return Ok(());
        }

        Ok(())
    }

    fn execute_modify_limit(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("decrease");
        let value = effect.value.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type = effect.card_type.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let delta = match operation {
            "decrease" => -(value as i32),
            "increase" => value as i32,
            "set" => value as i32,
            _ => return Err(format!("Unknown operation: {}", operation)),
        };

        // Track limit modification as a prohibition effect
        let limit_text = if let Some(ct) = card_type {
            format!("modify_limit:{}:{}:{}", operation, ct, delta)
        } else {
            format!("modify_limit:{}:{}", operation, delta)
        };
        self.game_state.prohibition_effects.push(limit_text);

        Ok(())
    }

    fn execute_set_blade_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let group = effect.group.as_ref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Get all stage member cards
        let card_ids: Vec<i16> = player.stage.stage.iter().filter(|&&id| id != -1).copied().collect();

        if let Some(group_info) = group {
            // Filter by group and set blade count
            let card_db = self.game_state.card_database.clone();
            for card_id in card_ids {
                if let Some(card) = card_db.get_card(card_id) {
                    if card.group == group_info.name {
                        // Set blade count - for now add blade modifier
                        // A full implementation would need a group-specific blade counter
                        self.game_state.add_blade_modifier(card_id, 1);
                    }
                }
            }
        }

        Ok(())
    }

    fn execute_set_required_hearts(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let value = effect.value.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type_filter {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else {
            player.hand.cards.iter().copied().collect()
        };

        let color = crate::zones::parse_heart_color(heart_color);

        for card_id in card_ids {
            self.game_state.set_need_heart_modifier(card_id, color, value as i32);
        }

        Ok(())
    }

    fn execute_set_score(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Set the live score for the current live
        player.live_score = value as i32;

        Ok(())
    }

    fn execute_modify_cost(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Target specific cards based on card_type_filter
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type_filter {
            player.stage.stage.iter().filter(|&&id| id != -1).copied().collect()
        } else if let Some("energy_card") = card_type_filter {
            player.energy_zone.cards.iter().copied().collect()
        } else {
            // Default to hand
            player.hand.cards.iter().copied().collect()
        };

        let delta = match operation {
            "add" => value as i32,
            "subtract" => -(value as i32),
            "set" => value as i32,
            _ => return Err(format!("Unknown operation: {}", operation)),
        };

        for card_id in card_ids {
            if operation == "set" {
                self.game_state.set_cost_modifier(card_id, delta);
            } else {
                self.game_state.add_cost_modifier(card_id, delta);
            }
        }

        Ok(())
    }

    fn pay_cost(&mut self, cost: &AbilityCost) -> Result<(), String> {
        eprintln!("PAY_COST: cost_type={:?}, source={:?}, destination={:?}, card_type={:?}", cost.cost_type, cost.source, cost.destination, cost.card_type);
        match cost.cost_type.as_deref() {
            Some("move_cards") => {
                // Execute the move action as a cost
                let effect = AbilityEffect {
                    text: cost.text.clone(),
                    action: cost.cost_type.clone().unwrap_or_default(),
                    source: cost.source.clone(),
                    destination: cost.destination.clone(),
                    count: cost.count,
                    card_type: cost.card_type.clone(),
                    target: cost.target.clone(),
                    ..Default::default()
                };
                self.execute_move_cards(&effect)
            }
            Some("pay_energy") => {
                // Pay energy cost
                let energy = cost.energy.unwrap_or(0);
                let target = cost.target.as_deref().unwrap_or("self");
                
                let player = match target {
                    "self" => &mut self.game_state.player1,
                    "opponent" => &mut self.game_state.player2,
                    _ => &mut self.game_state.player1,
                };
                
                if energy > 0 {
                    // Use EnergyZone::pay_energy to actually tap energy cards
                    if let Err(e) = player.energy_zone.pay_energy(energy as usize) {
                        return Err(e);
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

// Implement Default for AbilityEffect
impl Default for AbilityEffect {
    fn default() -> Self {
        AbilityEffect {
            heart_color: None,
            blade_type: None,
            energy_count: None,
            target_member: None,
            text: String::new(),
            action: String::new(),
            source: None,
            destination: None,
            count: None,
            card_type: None,
            target: None,
            duration: None,
            parenthetical: None,
            look_action: None,
            select_action: None,
            actions: None,
            resource: None,
            position: None,
            state_change: None,
            optional: None,
            max: None,
            effect_constraint: None,
            shuffle_target: None,
            icon_count: None,
            ability_gain: None,
            quoted_text: None,
            per_unit: None,
            condition: None,
            primary_effect: None,
            alternative_condition: None,
            alternative_effect: None,
            operation: None,
            value: None,
            aggregate: None,
            comparison_type: None,
            choice_options: None,
            group: None,
            per_unit_count: None,
            per_unit_type: None,
            per_unit_reference: None,
            group_matching: None,
            repeat_limit: None,
            repeat_optional: None,
            is_further: None,
            cost_result_reference: None,
            dynamic_count: None,
            resource_icon_count: None,
            placement_order: None,
            cost_limit: None,
            unit: None,
            distinct: None,
            target_player: None,
            target_location: None,
            target_scope: None,
            target_card_type: None,
            activation_condition: None,
            activation_condition_parsed: None,
            gained_ability: None,
            ability_text: None,
            swap_action: None,
            has_member_swapping: None,
            group_options: None,
            card_count: None,
            use_limit: None,
            triggers: None,
            restricted_destination: None,
            restriction_type: None,
        }
    }
}
