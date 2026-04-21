#![allow(dead_code)]

use crate::card::{Ability, AbilityCost, AbilityEffect, Condition, Keyword};

use crate::game_state::GameState;
use crate::zones::MemberArea;
use std::vec::Vec;
use std::string::String;

#[allow(dead_code)]
pub struct AbilityResolver<'a> {
    game_state: &'a mut GameState,
}

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    pub fn new(game_state: &'a mut GameState) -> Self {
        AbilityResolver { game_state }
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
            _ => {}
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
        if let Some(ref actions) = effect.actions {
            for action in actions {
                self.execute_effect(action)?;
            }
        }
        Ok(())
    }

    fn execute_conditional_alternative(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Check alternative condition
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
        // Execute look action first
        if let Some(ref look_action) = effect.look_action {
            self.execute_effect(look_action)?;
        }

        // Then execute select action
        if let Some(ref select_action) = effect.select_action {
            self.execute_effect(select_action)?;
        }

        Ok(())
    }

    fn execute_draw(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let source = effect.source.as_deref().unwrap_or("deck");
        let destination = effect.destination.as_deref().unwrap_or("hand");
        let card_type_filter = effect.card_type.as_deref();
        let resource_icon_count = effect.resource_icon_count;
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

        match source {
            "deck" | "deck_top" => {
                let mut drawn = 0;
                while drawn < count {
                    if let Some(card) = player.main_deck.draw() {
                        if matches_card_type(card, card_type_filter) && matches_group(card, group_filter) {
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

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Get energy cards from energy zone
        let mut energy_cards = Vec::new();
        for _ in 0..count {
            if let Some(energy_card) = player.energy_zone.cards.pop() {
                energy_cards.push(energy_card); // energy_card is now i16 directly
            } else {
                break; // No more energy cards available
            }
        }

        // Place energy cards under stage members
        // Energy tracking moved to GameState modifiers
        if player.stage.stage[1] != -1 {
            // Energy tracking moved to GameState modifiers
        } else if player.stage.stage[0] != -1 {
            // Energy tracking moved to GameState modifiers
        } else if player.stage.stage[2] != -1 {
            // Energy tracking moved to GameState modifiers
        } else {
            // No member on stage, put energy cards back
            for card in energy_cards {
                player.energy_zone.cards.push(card);
            }
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
        let source = effect.source.as_deref().unwrap_or("");
        let destination = effect.destination.as_deref().unwrap_or("");
        let card_type_filter = effect.card_type.as_deref();
        let target = effect.target.as_deref().unwrap_or("self");
        let optional = effect.optional.unwrap_or(false);
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));

        // Handle optional costs based on game state behavior flag
        if optional {
            match self.game_state.optional_cost_behavior.as_str() {
                "never_pay" => {
                    eprintln!("Move cards is optional - skipping (never_pay mode)");
                    return Ok(());
                }
                "always_pay" => {
                    // Proceed with the move
                }
                "auto" => {
                    // For auto mode, always pay to make abilities functional
                    // In a real UI, this would prompt the player
                }
                _ => {
                    eprintln!("Unknown optional_cost_behavior: {}, defaulting to always_pay", self.game_state.optional_cost_behavior);
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

        match source {
            "stage" => {
                // Move card from stage to destination
                let mut moved = 0;
                // For now, move from center position - would need position parameter
                if player.stage.stage[1] != -1 {
                    let center_card = player.stage.stage[1];
                    player.stage.stage[1] = -1;
                    if matches_card_type(center_card, card_type_filter) && matches_group(center_card, group_filter) {
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
                        if matches_card_type(left_card_id, card_type_filter) && matches_group(left_card_id, group_filter) {
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
                        if matches_card_type(right_card_id, card_type_filter) && matches_group(right_card_id, group_filter) {
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
                        if matches_card_type(card, card_type_filter) && matches_group(card, group_filter) {
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
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            player.waitroom.add_card(card);
                        }
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            player.hand.add_card(card);
                        }
                    }
                    "deck_bottom" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card, card_type_filter) && matches_group(*card, group_filter) {
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
                            if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) {
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
                    }
                    "discard" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card_id) in player.energy_zone.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) {
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
                            if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) {
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
                            if matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) {
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
                        .filter(|&card_id| matches_card_type(card_id, card_type_filter) && matches_group(card_id, group_filter))
                        .count() as u32
                }
                Some("hand") => {
                    player.hand.cards.iter()
                        .filter(|&card_id| matches_card_type(card_id, card_type_filter) && matches_group(card_id, group_filter))
                        .count() as u32
                }
                _ => {
                    // Default to stage count
                    player.stage.stage.iter()
                        .filter(|&&card_id| card_id != -1)
                        .filter(|&card_id| matches_card_type(card_id, card_type_filter) && matches_group(card_id, group_filter))
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
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        match state_change {
            "active" => {
                if count == 2 {
                    player.energy_zone.activate_all();
                } else {
                    // Activate specific energy cards based on count
                    let count_usize = count as usize;
                    let max_to_activate = player.energy_zone.cards.len().min(count_usize);
                    player.energy_zone.active_energy_count = max_to_activate;
                }
            }
            "wait" => {
                // Set energy cards to wait state
                let count_usize = count as usize;
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

        // Collect card_ids first to avoid borrow conflicts
        let card_ids: Vec<i16> = match target {
            "self" => self.game_state.player1.stage.stage.iter()
                .filter(|&&card_id| card_id != -1)
                .copied()
                .collect(),
            "opponent" => self.game_state.player2.stage.stage.iter()
                .filter(|&&card_id| card_id != -1)
                .copied()
                .collect(),
            _ => self.game_state.player1.stage.stage.iter()
                .filter(|&&card_id| card_id != -1)
                .copied()
                .collect(),
        };

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
        let _value = effect.value.unwrap_or(0);

        // This would need to target specific live cards
        match operation {
            "decrease" => {
                // Decrease required hearts
            }
            "increase" => {
                // Increase required hearts
            }
            _ => {}
        }

        Ok(())
    }

    fn execute_set_cost(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Set card cost
        Ok(())
    }

    fn execute_set_blade_type(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Set blade type
        Ok(())
    }

    fn execute_set_heart_type(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Set heart type
        Ok(())
    }

    fn execute_activate_ability(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Activate an ability
        Ok(())
    }

    fn execute_invalidate_ability(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Invalidate an ability
        Ok(())
    }

    fn execute_play_baton_touch(&mut self, _effect: &AbilityEffect) -> Result<(), String> {
        // Execute baton touch
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
        // For now, automatically select the first matching card
        let count = effect.count.unwrap_or(1);
        let source = effect.source.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let _card_type_filter = effect.card_type.as_deref();

        let _player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Helper function to check if card matches type filter
        let _matches_card_type = |card: &crate::card::Card, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card.is_live(),
                Some("member_card") => card.is_member(),
                Some("energy_card") => card.is_energy(),
                None => true,
                _ => true,
            }
        };

        // For now, just log the selection - actual selection would need to be tracked
        // In a full implementation, this would mark cards as "selected" for subsequent actions
        eprintln!("Select {} cards from {} for {} (automatic selection)", count, source, target);
        Ok(())
    }

    fn execute_look_at(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Look at cards from a location without revealing to opponent
        let count = effect.count.unwrap_or(1);
        let source = effect.source.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        let _player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // For now, just log that cards are being looked at
        // In a full implementation, this would track which cards have been looked at
        // and potentially store them in a temporary buffer for selection
        match source {
            "deck" | "deck_top" => {
                eprintln!("Look at top {} cards of deck for {}", count, target);
            }
            "hand" => {
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
        // First, pay the cost if there is one
        if let Some(ref cost) = ability.cost {
            self.pay_cost(cost)?;
        }

        // Then execute the effect
        if let Some(ref effect) = ability.effect {
            self.execute_effect(effect)?;
        }

        Ok(())
    }

    fn execute_position_change(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Swap positions between stage areas
        // For now, swap center with left_side if both have cards
        if player.stage.stage[1] != -1 && player.stage.stage[0] != -1 {
            let center = player.stage.stage[1];
            let left = player.stage.stage[0];
            player.stage.stage[1] = left;
            player.stage.stage[0] = center;
        }

        eprintln!("Position change executed for {}", target);
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
            if let Some(first_option) = options.first() {
                // For now, just log the choice since options are strings, not effects
                // A real implementation would need UI for player choice and parsing
                eprintln!("Choice made: {}", first_option);
            }
        }

        eprintln!("Choice executed");
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
        // Set a card's identity (name, group, etc.)
        // This is complex - would need to create new card instances or track identity overrides
        // For now, track identity changes in GameState modifiers
        if let Some(ref group) = effect.group {
            // Track group change - this would need a new modifier type
            eprintln!("Set card identity: group={}", group.name);
            // For now, just log - full implementation would need identity override tracking
        }

        eprintln!("Set card identity: {:?}", effect);
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

        // Discard cards from hand until hand has target_count cards
        while player.hand.len() > target_count {
            if let Some(card) = player.hand.cards.pop() {
                player.waitroom.add_card(card);
            } else {
                break;
            }
        }

        eprintln!("Discard until count: {} for {}", target_count, target);
        Ok(())
    }

    fn execute_restriction(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Apply a restriction effect
        // Add to prohibition_effects in GameState
        let restriction_text = effect.text.clone();
        if !restriction_text.is_empty() {
            self.game_state.prohibition_effects.push(restriction_text);
        }

        eprintln!("Restriction applied: {:?}", effect);
        Ok(())
    }

    fn execute_re_yell(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Re-yell action - increment yell count
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");

        let _player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Increment yell count - this would need a yell counter in Player
        // For now, just log
        eprintln!("Re-yell executed: {} times for {}", count, target);
        Ok(())
    }

    fn execute_modify_cost(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        // Track cost modifiers in GameState
        // This would need a new modifier type: cost_modifiers HashMap
        // For now, just log
        eprintln!("Modify cost: {} by {} for {}", operation, value, target);
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
        }
    }
}
