#![allow(dead_code)]

use crate::card::{Ability, AbilityCost, AbilityEffect, Condition, Keyword};

use crate::game_state::{GameState, Phase};

use crate::zones::MemberArea;
use std::vec::Vec;
use std::string::String;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    SelectHeartColor {
        count: usize,
        options: Vec<String>,
        description: String,
    },
    SelectHeartType {
        count: usize,
        options: Vec<String>,
        description: String,
    },
}

#[derive(Debug, Clone)]
pub enum ChoiceResult {
    CardSelected { indices: Vec<usize> },
    TargetSelected { target: String },
    PositionSelected { position: String },
    HeartColorSelected { colors: Vec<String> },
    HeartTypeSelected { types: Vec<String> },
    Skip,
}

#[derive(Debug, Clone)]
pub enum ExecutionContext {
    /// No execution in progress
    None,
    /// Executing a single effect
    SingleEffect { effect_index: usize },
    /// Executing sequential effects
    SequentialEffects { current_index: usize, effects: Vec<AbilityEffect> },
    /// Executing look_and_select pattern
    LookAndSelect { step: LookAndSelectStep },
}

#[derive(Debug, Clone)]
pub enum LookAndSelectStep {
    /// Initial look_at step - draw cards to looked_at zone
    LookAt { count: usize, source: String },
    /// Selection step - user selects cards from looked_at
    Select { count: usize },
    /// Final step - move selected cards to destination, rest to discard
    Finalize { destination: String },
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
    // Track the card currently activating an ability
    pub activating_card_id: Option<i16>,
    // Execution context for resuming after user choices
    pub execution_context: ExecutionContext,
    // Store the current effect being executed for resuming
    pub current_effect: Option<AbilityEffect>,
}

#[allow(dead_code)]
impl<'a> AbilityResolver<'a> {
    pub fn new(game_state: &'a mut GameState) -> Self {
        let pending_choice = game_state.pending_choice.clone();
        let activating_card_id = game_state.activating_card;
        AbilityResolver {
            game_state,
            pending_choice,
            looked_at_cards: Vec::new(),
            duration_effects: Vec::new(),
            current_ability: None,
            activating_card_id,
            execution_context: ExecutionContext::None,
            current_effect: None,
        }
    }

    /// Get pending choice (if any)
    pub fn get_pending_choice(&self) -> Option<&Choice> {
        self.game_state.pending_choice.as_ref()
    }

    /// Resume execution after user provides choice
    fn resume_execution(&mut self, context: ExecutionContext) -> Result<(), String> {
        match context {
            ExecutionContext::None => {
                // No context to resume from
                Ok(())
            }
            ExecutionContext::LookAndSelect { step } => {
                match step {
                    LookAndSelectStep::Select { count: _ } => {
                        // After selecting from looked_at, execute the select_action (which may be sequential)
                        let select_action_to_execute = if let Some(ref effect) = self.current_effect {
                            effect.select_action.clone()
                        } else {
                            None
                        };
                        
                        if let Some(select_action) = select_action_to_execute {
                            // Execute the select_action (may be sequential with multiple actions)
                            self.execute_effect(&select_action)?;
                        }
                        self.execution_context = ExecutionContext::None;
                        Ok(())
                    }
                    LookAndSelectStep::LookAt { .. } => {
                        // After look_at, proceed to selection
                        self.execution_context = ExecutionContext::None;
                        Ok(())
                    }
                    LookAndSelectStep::Finalize { .. } => {
                        // After finalizing, execution is complete
                        self.execution_context = ExecutionContext::None;
                        Ok(())
                    }
                }
            }
            ExecutionContext::SingleEffect { .. } => {
                // After single effect, execution is complete
                self.execution_context = ExecutionContext::None;
                Ok(())
            }
            ExecutionContext::SequentialEffects { current_index, effects } => {
                // Continue with next effect in sequence
                if current_index + 1 < effects.len() {
                    self.execution_context = ExecutionContext::SequentialEffects {
                        current_index: current_index + 1,
                        effects: effects.clone(),
                    };
                    self.execute_effect(&effects[current_index + 1])
                } else {
                    // All effects executed
                    self.execution_context = ExecutionContext::None;
                    Ok(())
                }
            }
        }
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
        let context = self.execution_context.clone();
        match (&choice, result) {
            (Some(Choice::SelectCard { zone, card_type, count, description: _, allow_skip }), ChoiceResult::CardSelected { indices }) => {
                // Check if this is an optional cost payment
                if *allow_skip {
                    eprintln!("Detected optional cost choice (allow_skip=true)");
                    // User chose to pay the optional cost
                    if !indices.is_empty() {
                        eprintln!("User chose to pay optional cost with {} cards", indices.len());
                        // Execute the cost payment by moving the selected cards
                        // The indices are from the zone specified in the choice (usually "hand")
                        match zone.as_str() {
                            "hand" => {
                                // Move selected cards from hand to discard
                                let player = self.game_state.active_player_mut();
                                // Collect cards to move (in reverse order to maintain indices)
                                for &idx in indices.iter().rev() {
                                    if idx < player.hand.cards.len() {
                                        let card_id = player.hand.cards[idx];
                                        player.hand.remove_card(idx);
                                        player.waitroom.add_card(card_id);
                                        eprintln!("Moved card {} from hand to waitroom for optional cost", card_id);
                                    }
                                }
                            }
                            "stage" => {
                                // Move selected member cards from stage to waitroom
                                let player = self.game_state.active_player_mut();
                                let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                                for &idx in indices.iter().rev() {
                                    if idx < areas.len() {
                                        let area = areas[idx];
                                        if let Some(card_id) = player.stage.get_area(area) {
                                            player.stage.clear_area(area);
                                            player.waitroom.add_card(card_id);
                                            eprintln!("Moved card {} from stage to waitroom for optional cost", card_id);
                                        }
                                    }
                                }
                            }
                            "energy_zone" => {
                                // Move selected energy cards from energy zone to waitroom
                                let player = self.game_state.active_player_mut();
                                for &idx in indices.iter().rev() {
                                    if idx < player.energy_zone.cards.len() {
                                        let card_id = player.energy_zone.cards.remove(idx);
                                        player.waitroom.add_card(card_id);
                                        eprintln!("Moved card {} from energy zone to waitroom for optional cost", card_id);
                                    }
                                }
                            }
                            _ => {
                                eprintln!("Optional cost payment from zone '{}' not supported - only hand, stage, and energy_zone are supported", zone);
                            }
                        }
                    } else {
                        eprintln!("User chose to skip optional cost");
                    }
                    self.pending_choice = None;
                    self.game_state.pending_choice = None;
                    self.game_state.pending_ability = None;
                    // Continue with effect execution
                    eprintln!("Checking if pending_current_ability exists for effect execution");
                    if let Some(ref ability) = self.game_state.pending_current_ability {
                        eprintln!("pending_current_ability exists, checking effect");
                        self.current_ability = Some(ability.clone());
                        if let Some(ref effect) = ability.effect {
                            eprintln!("Continuing with effect execution after optional cost payment");
                            eprintln!("Effect to execute: action={}, look_action={:?}", effect.action, effect.look_action);
                            let effect_clone = effect.clone();
                            if let Err(e) = self.execute_effect(&effect_clone) {
                                eprintln!("Failed to execute effect after optional cost: {}", e);
                            } else {
                                eprintln!("Effect execution succeeded");
                            }
                            // Copy any new pending choice to game_state
                            eprintln!("After effect execution, pending_choice: {:?}", self.pending_choice);
                            eprintln!("After effect execution, looked_at_cards: {:?}", self.looked_at_cards);
                            if self.pending_choice.is_some() {
                                eprintln!("Copying new pending choice to game_state after effect execution");
                                self.game_state.pending_choice = self.pending_choice.clone();
                            }
                        } else {
                            eprintln!("No effect in pending_current_ability");
                        }
                        // Clear pending_current_ability AFTER effect execution
                        self.game_state.pending_current_ability = None;
                        self.current_ability = None;
                    } else {
                        eprintln!("No pending_current_ability after optional cost payment");
                    }
                    return Ok(());
                }

                // Normal card selection (not optional cost)
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
                self.game_state.pending_choice = None;
                // Resume execution based on context
                self.resume_execution(context)?;
                // Resume pending sequential actions if any
                if let Some(ref pending_actions) = self.game_state.pending_sequential_actions.clone() {
                    eprintln!("Resuming {} pending sequential actions after choice", pending_actions.len());
                    for action in pending_actions {
                        self.execute_effect(action)?;
                    }
                    self.game_state.pending_sequential_actions = None;
                }
                Ok(())
            }
            (Some(Choice::SelectCard { .. }), ChoiceResult::Skip) => {
                // User chose to skip optional cost - don't execute cost, proceed to effect
                eprintln!("User skipped optional cost");
                self.pending_choice = None;
                self.game_state.pending_choice = None;
                // Resume execution based on context
                self.resume_execution(context)
            }
            (Some(Choice::SelectTarget { target, .. }), ChoiceResult::TargetSelected { target: selected }) => {
                // Check if this is a choice effect (from execute_choice)
                if let Some(ref pending) = self.game_state.pending_ability.clone() {
                    if pending.card_no == "choice" {
                        if let Some(ref options_json) = pending.conditional_choice {
                            // Deserialize the options from JSON (Vec<AbilityEffect>)
                            if let Ok(options) = serde_json::from_str::<Vec<AbilityEffect>>(options_json) {
                                // Parse the selected option (target is JSON array of options, selected is the chosen index as string)
                                let selected_index: usize = selected.parse().unwrap_or(0);
                                
                                if selected_index < options.len() {
                                    let selected_effect = &options[selected_index];
                                    eprintln!("User selected option {}: {}", selected_index, selected_effect.text);
                                    
                                    // Execute the selected effect
                                    if let Err(e) = self.execute_effect(selected_effect) {
                                        eprintln!("Failed to execute selected choice effect: {}", e);
                                        return Err(e);
                                    }
                                    
                                    eprintln!("Choice effect executed successfully");
                                } else {
                                    eprintln!("Invalid choice index: {}", selected_index);
                                }
                            } else {
                                eprintln!("Failed to deserialize choice options");
                            }
                        }
                        
                        self.pending_choice = None;
                        self.game_state.pending_choice = None;
                        self.game_state.pending_ability = None;
                        return Ok(());
                    }
                    
                    // Check if this is a string choice (heart options, etc.)
                    if pending.card_no == "choice_string" {
                        if let Some(ref options_json) = pending.conditional_choice {
                            // Deserialize the options from JSON (Vec<String>)
                            if let Ok(options) = serde_json::from_str::<Vec<String>>(options_json) {
                                eprintln!("User selected string option: {} from {:?}", selected, options);
                                
                                // Apply the selected option based on context
                                if let Ok(selected_idx) = selected.parse::<usize>() {
                                    if selected_idx > 0 && selected_idx <= options.len() {
                                        let selected_value = &options[selected_idx - 1];
                                        eprintln!("Applying selected option: {}", selected_value);
                                        
                                        // Store the selected heart color in game state for later use
                                        if selected_value.starts_with("heart") || 
                                           selected_value == "赤" || selected_value == "桃" || 
                                           selected_value == "緑" || selected_value == "青" ||
                                           selected_value == "黄" || selected_value == "紫" {
                                            self.game_state.prohibition_effects.push(format!("selected_heart_color:{}", selected_value));
                                            eprintln!("Stored selected heart color: {}", selected_value);
                                        }
                                    }
                                }
                            } else {
                                eprintln!("Failed to deserialize string choice options");
                            }
                        }
                        
                        self.pending_choice = None;
                        self.game_state.pending_choice = None;
                        self.game_state.pending_ability = None;
                        return Ok(());
                    }
                }

                // Check if this is a position_change choice
                if let Some(ref pending) = self.game_state.pending_ability.clone() {
                    if pending.card_no == "position_change" {
                        eprintln!("User selected position change destination: {}", selected);

                        // Execute the position change with the selected destination
                        // Create a modified effect with the selected destination
                        let mut modified_effect = pending.effect.clone();
                        modified_effect.destination = Some(selected.clone());

                        // Execute the position change with the destination specified
                        if let Err(e) = self.execute_position_change_with_destination(&modified_effect, &selected) {
                            eprintln!("Failed to execute position change: {}", e);
                        }

                        self.pending_choice = None;
                        self.game_state.pending_choice = None;
                        self.game_state.pending_ability = None;
                        return Ok(());
                    }
                }

                // Check if this is an optional cost choice
                if target == "pay_optional_cost:skip_optional_cost" {
                    // If user chose to pay, continue with the effect
                    // If user chose to skip, skip the effect
                    if selected == "skip_optional_cost" {
                        eprintln!("User chose to skip optional cost");
                        self.pending_choice = None;
                        self.game_state.pending_choice = None;
                        return Ok(());
                    } else if selected == "pay_optional_cost" {
                        eprintln!("User chose to pay optional cost - paying energy and continuing with effect");
                        // Retrieve the pending ability and pay the cost first
                        if let Some(pending) = self.game_state.pending_ability.clone() {
                            // Pay the energy cost first
                            if let Some(ref cost) = pending.cost {
                                if let Some(energy) = cost.energy {
                                    if energy > 0 {
                                        let target = cost.target.as_deref().unwrap_or("self");
                                        let player = match target {
                                            "self" => &mut self.game_state.player1,
                                            "opponent" => &mut self.game_state.player2,
                                            _ => &mut self.game_state.player1,
                                        };
                                        
                                        if let Err(e) = player.energy_zone.pay_energy(energy as usize) {
                                            eprintln!("Failed to pay energy cost: {}", e);
                                            return Err(e);
                                        }
                                        eprintln!("Paid {} energy for optional cost", energy);
                                    }
                                }
                            }
                            
                            eprintln!("Executing effect from pending ability: {:?}", pending.effect);
                            // Execute the effect
                            if let Err(e) = self.execute_effect(&pending.effect) {
                                eprintln!("Failed to execute effect after optional cost: {}", e);
                            }
                            self.pending_choice = None;
                            self.game_state.pending_choice = None;
                            return Ok(());
                        }
                    }
                }
                // Check if this is a conditional_alternative choice
                if target == "primary|alternative" {
                    // Store the choice and execute the appropriate effect
                    let choice_made = selected.as_str();
                    
                    // Execute the chosen effect (primary or alternative)
                    if let Some(ref pending) = self.game_state.pending_ability.clone() {
                        if let Some(ref ability) = self.current_ability.clone() {
                            if let Some(ref effect) = ability.effect {
                                if effect.action == "conditional_alternative" {
                                    let player = self.game_state.active_player_mut();
                                    let perspective_player_id = player.id.clone();
                                    
                                    match choice_made {
                                        "primary" => {
                                            if let Some(ref primary) = effect.primary_effect {
                                                if let Err(e) = self.execute_effect(primary) {
                                                    eprintln!("Failed to execute primary effect: {}", e);
                                                }
                                            }
                                        }
                                        "alternative" => {
                                            if let Some(ref alternative) = effect.alternative_effect {
                                                if let Err(e) = self.execute_effect(alternative) {
                                                    eprintln!("Failed to execute alternative effect: {}", e);
                                                }
                                            }
                                        }
                                        _ => {
                                            eprintln!("Unknown choice for conditional_alternative: {}", choice_made);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    self.pending_choice = None;
                    self.game_state.pending_choice = None;
                    self.game_state.pending_ability = None;
                    return Ok(());
                }
                // Handle other target selections
                self.pending_choice = None;
                self.game_state.pending_choice = None;
                Ok(())
            }
            (Some(Choice::SelectPosition { .. }), ChoiceResult::PositionSelected { position }) => {
                // Handle stage area selection
                if let ExecutionContext::LookAndSelect { step } = context {
                    if let LookAndSelectStep::Finalize { destination } = step {
                        if destination == "stage" {
                            // Place the card in the selected area
                            if let Some(&card_id) = self.looked_at_cards.last() {
                                let player = &mut self.game_state.player1;
                                match position.as_str() {
                                    "center" => {
                                        player.stage.stage[1] = card_id;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                                    }
                                    "left_side" => {
                                        player.stage.stage[0] = card_id;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                                    }
                                    "right_side" => {
                                        player.stage.stage[2] = card_id;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                                    }
                                    _ => {
                                        eprintln!("Unknown position: {}", position);
                                        player.hand.add_card(card_id);
                                    }
                                }
                                self.looked_at_cards.clear();
                            }
                        }
                    }
                }
                self.pending_choice = None;
                self.game_state.pending_choice = None;
                self.execution_context = ExecutionContext::None;
                Ok(())
            }
            _ => Err("Choice result does not match pending choice".to_string()),
        }
    }

    /// Execute card selection from zone based on user selection
    fn execute_selected_cards_from_zone(&mut self, zone: &str, indices: &[usize], _count: usize, card_type_filter: Option<&str>) -> Result<(), String> {
        let player = &mut self.game_state.player1;
        let card_db = self.game_state.card_database.clone();

        // Get character name filter from pending ability cost if available
        let character_filter = self.game_state.pending_ability.as_ref()
            .and_then(|p| p.cost.as_ref())
            .and_then(|c| c.characters.as_ref());

        let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true,
            }
        };

        let matches_character_names = |card_id: i16, names: Option<&Vec<String>>| -> bool {
            if let Some(required_names) = names {
                if let Some(card) = card_db.get_card(card_id) {
                    required_names.iter().any(|name| card.name.contains(name) || card.name == *name)
                } else {
                    false
                }
            } else {
                true
            }
        };

        match zone {
            "hand" => {
                // Remove selected cards from hand and add to waitroom
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

                let mut cards_moved: Vec<i16> = Vec::new();
                for i in indices_to_remove {
                    if i < player.hand.cards.len() {
                        let card_id = player.hand.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) && matches_character_names(card_id, character_filter) {
                            player.waitroom.add_card(card_id);
                            cards_moved.push(card_id);
                        } else {
                            // Card doesn't match filter, put it back
                            player.hand.cards.insert(i, card_id);
                        }
                    }
                }
                // Clear modifiers for cards moved to waitroom
                for card_id in cards_moved {
                    self.game_state.clear_modifiers_for_card(card_id);
                }
            }
            "deck" => {
                // Remove selected cards from deck and add to hand
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

                let mut cards_moved: Vec<i16> = Vec::new();
                for i in indices_to_remove {
                    if i < player.main_deck.cards.len() {
                        let card_id = player.main_deck.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) && matches_character_names(card_id, character_filter) {
                            player.hand.add_card(card_id);
                            cards_moved.push(card_id);
                        } else {
                            // Card doesn't match filter, put it back
                            player.main_deck.cards.insert(i, card_id);
                        }
                    }
                }
                // Clear modifiers for cards moved to hand
                for card_id in cards_moved {
                    self.game_state.clear_modifiers_for_card(card_id);
                }
            }
            "discard" => {
                // Remove selected cards from discard and add to hand
                let mut indices_to_remove: Vec<usize> = indices.iter().copied().collect();
                indices_to_remove.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

                let mut cards_moved: Vec<i16> = Vec::new();
                for i in indices_to_remove {
                    if i < player.waitroom.cards.len() {
                        let card_id = player.waitroom.cards.remove(i);
                        if matches_card_type(card_id, card_type_filter) && matches_character_names(card_id, character_filter) {
                            player.hand.add_card(card_id);
                            cards_moved.push(card_id);
                        } else {
                            // Card doesn't match filter, put it back
                            player.waitroom.cards.insert(i, card_id);
                        }
                    }
                }
                // Modifiers stay when moving to hand (card still in play)
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
        let result = match condition.condition_type.as_deref() {
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
            Some("choice_condition") => self.evaluate_choice_condition(condition),
            Some("position_change_condition") => self.evaluate_position_change_condition(condition),
            Some("state_change_condition") => self.evaluate_state_change_condition(condition),
            Some("opponent_choice_condition") => self.evaluate_opponent_choice_condition(condition),
            _ => {
                // Default: unknown condition type, return true (fail-open)
                eprintln!("Unknown condition type: {:?}", condition.condition_type);
                true
            }
        };

        // Apply negation if specified
        if condition.negation.unwrap_or(false) {
            !result
        } else {
            result
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
        
        // Handle "values" field - check if count is one of the specified values
        if let Some(ref values) = condition.values {
            return values.contains(&(count as u32));
        }
        
        // Handle comparison_target - compare against opponent's value
        let target_count = if let Some(ref comparison_target) = condition.comparison_target {
            if comparison_target == "opponent" {
                // Get the same count for opponent
                self.get_count_for_target(condition, "opponent")
            } else {
                condition.count.unwrap_or(0)
            }
        } else {
            condition.count.unwrap_or(0)
        };

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
        let distinct = condition.distinct.unwrap_or(false); // Check for distinct names
        let all_areas = condition.all_areas.unwrap_or(false); // Check all areas (e.g., all stage areas)
        let no_excess_heart = condition.no_excess_heart.unwrap_or(false); // Check opponent has no excess hearts
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false); // Check if triggered by baton touch

        // If baton_touch_trigger is true, check if a baton touch occurred
        if baton_touch_trigger {
            // Check if a baton touch occurred this turn
            // Note: This checks turn-level baton touch count, not whether this specific
            // ability was triggered by baton touch. Full context tracking would require
            // passing baton touch context through the ability resolution pipeline.
            if self.game_state.baton_touch_count == 0 {
                // No baton touch occurred this turn, condition fails
                return false;
            }
        }
        
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

        // Helper function to check if cards have distinct names
        let check_distinct_names = |card_ids: &[i16]| -> bool {
            let mut names = std::collections::HashSet::new();
            for &card_id in card_ids {
                if card_id == -1 {
                    continue;
                }
                if card_db.get_card(card_id).is_some() {
                    // For multi-name cards (e.g., "A&B&C"), check all names
                    let card_names = card_db.get_card_names(card_id);
                    for name in card_names {
                        if !names.insert(name) {
                            return false; // Duplicate name found
                        }
                    }
                }
            }
            true
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
                    
                    // Apply all_areas check: if all_areas is true, all stage areas must be occupied
                    if all_areas && count != 3 {
                        return false;
                    }
                    
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
            "revealed_cards" => {
                // Check cards in the revealed_cards set
                let revealed_count = self.game_state.revealed_cards.len() as u32;
                
                // If card_property is specified (e.g., has_blade_heart), filter by that
                if let Some(property) = condition.card_property.as_deref() {
                    let card_db = &self.game_state.card_database;
                    match property {
                        "has_blade_heart" => {
                            self.game_state.revealed_cards.iter()
                                .filter(|&&card_id| {
                                    if let Some(card) = card_db.get_card(card_id) {
                                        card.has_blade_heart()
                                    } else {
                                        false
                                    }
                                })
                                .count() as u32
                        }
                        _ => revealed_count
                    }
                } else {
                    revealed_count
                }
            }
            _ => 0,
        };

        // Apply distinct check: if distinct is true, verify cards have distinct names
        if distinct {
            let card_ids: Vec<i16> = match location {
                "stage" => player.stage.stage.to_vec(),
                "hand" => player.hand.cards.to_vec(),
                "discard" => player.waitroom.cards.to_vec(),
                "energy_zone" => player.energy_zone.cards.to_vec(),
                "live_card_zone" => player.live_card_zone.cards.to_vec(),
                "success_live_zone" => player.success_live_card_zone.cards.to_vec(),
                _ => Vec::new(),
            };
            
            if !check_distinct_names(&card_ids) {
                return false;
            }
        }

        // Apply no_excess_heart check: if true, verify opponent has no excess hearts
        // This is used in conditions like "相手が余剰のハートを持たずにライブを成功させていた"
        if no_excess_heart {
            // Check if opponent (target) has excess hearts
            // Excess hearts are hearts beyond what's needed for live cards
            let opponent = if target == "self" {
                &self.game_state.player2
            } else {
                &self.game_state.player1
            };
            
            // Calculate opponent's excess hearts
            let total_hearts: u32 = opponent.stage.stage.iter()
                .filter(|&&card_id| card_id != -1)
                .map(|&card_id| {
                    if let Some(card) = card_db.get_card(card_id) {
                        card.total_hearts()
                    } else {
                        0
                    }
                })
                .sum();
            
            // Calculate hearts needed for live cards
            let needed_hearts: u32 = opponent.live_card_zone.cards.iter()
                .map(|&card_id| {
                    if let Some(card) = card_db.get_card(card_id) {
                        card.total_hearts()
                    } else {
                        0
                    }
                })
                .sum();
            
            // If total hearts > needed hearts, opponent has excess hearts
            if total_hearts > needed_hearts {
                return false;
            }
        }

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
        let position = condition.position.as_ref().and_then(|p| p.get_position()).unwrap_or("");
        
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
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false);
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };
        
        // Check baton touch condition (Q229)
        if baton_touch_trigger {
            // This ability only triggers when the card appeared via baton touch
            // Check if the activating card was played via baton touch
            if let Some(ref activating_card) = self.game_state.activating_card {
                if let Some(_card) = self.game_state.card_database.get_card(*activating_card) {
                    // Check if a baton touch occurred this turn
                    // Note: This checks turn-level baton touch count, not whether this specific
                    // card was played via baton touch. Card-specific tracking would require
                    // storing baton touch origin for each card on stage.
                    return self.game_state.baton_touch_count > 0;
                }
            }
            return false;
        }
        
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
                    // Check nested condition for this_turn (e.g., not_moved, has_moved)
                    if let Some(nested_condition) = &condition.condition {
                        match nested_condition.condition_type.as_deref() {
                            Some("not_moved") => {
                                // Check if activating card has not moved this turn
                                if let Some(activating_card_id) = self.activating_card_id {
                                    !self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else {
                                    true // No activating card, assume not moved
                                }
                            }
                            Some("has_moved") => {
                                // Check if activating card has moved this turn
                                if let Some(activating_card_id) = self.activating_card_id {
                                    self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else {
                                    false // No activating card, assume not moved
                                }
                            }
                            _ => {
                                // Evaluate nested condition normally
                                self.evaluate_condition(nested_condition)
                            }
                        }
                    } else {
                        true // No nested condition, assume valid for current turn
                    }
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
            // any_of is a list of condition type strings that should be evaluated
            // Evaluate each condition type and return true if any match
            any_of.iter().any(|condition_type| {
                match condition_type.as_str() {
                    "has_member" => {
                        // Check if player has at least one member on stage
                        !self.game_state.player1.stage.stage.iter().all(|&id| id == crate::constants::EMPTY_SLOT)
                    }
                    "has_energy" => {
                        // Check if player has energy in energy zone
                        !self.game_state.player1.energy_zone.cards.is_empty()
                    }
                    "has_hand" => {
                        // Check if player has cards in hand
                        !self.game_state.player1.hand.cards.is_empty()
                    }
                    "has_blade_heart" => {
                        // Check if any member on stage has blade heart
                        self.game_state.player1.stage.stage.iter().any(|&id| {
                            if id != crate::constants::EMPTY_SLOT {
                                if let Some(card) = self.game_state.card_database.get_card(id) {
                                    card.has_blade_heart()
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        })
                    }
                    "has_live_card" => {
                        // Check if player has live cards set
                        !self.game_state.player1.live_card_zone.cards.is_empty()
                    }
                    "is_active_phase" => {
                        // Check if current phase is Active
                        matches!(self.game_state.current_phase, crate::game_state::Phase::Active)
                    }
                    "is_main_phase" => {
                        // Check if current phase is Main
                        matches!(self.game_state.current_phase, crate::game_state::Phase::Main)
                    }
                    _ => {
                        // Unknown condition type, log and return false
                        eprintln!("Unknown any_of condition type: {}", condition_type);
                        false
                    }
                }
            })
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

    fn evaluate_choice_condition(&self, _condition: &Condition) -> bool {
        // Choice conditions represent player choice in cost payment
        // This is handled during cost resolution, not condition evaluation
        // For now, return true to allow the ability to be considered valid
        // The actual choice logic should be implemented in the cost payment system
        true
    }

    fn evaluate_position_change_condition(&self, condition: &Condition) -> bool {
        // Check if a position change occurred or is requested
        // This condition type handles "ポジションチェンジしてもよい" patterns
        let optional = condition.options.as_ref().map(|_| true).unwrap_or(false);
        let _position = condition.position.as_ref().and_then(|p| p.get_position()).unwrap_or("");
        
        if optional {
            // For optional position changes, check if position_change_occurred_this_turn
            // and if the action was actually taken
            if self.game_state.position_change_occurred_this_turn {
                // Check if it was this specific member that changed position
                // For now, we track that a position change happened
                return true;
            }
            // Optional position change not taken yet - condition is not met
            return false;
        }
        
        // Non-optional position change condition
        // Check if position change tracking is enabled and occurred
        self.game_state.position_change_occurred_this_turn
    }

    fn evaluate_state_change_condition(&self, condition: &Condition) -> bool {
        // Check for state transitions like active -> wait
        let _text = &condition.text;
        let _during_main_phase = condition.text.contains("main_phase");
        
        // Check if we're in main phase (if specified)
        if _during_main_phase && self.game_state.current_phase != Phase::Main {
            return false;
        }
        
        // Check if state transition tracking exists
        // For now, we check if any member recently changed from active to wait
        // This would need proper tracking in game_state
        // Return true as placeholder - proper implementation needs state transition history
        true
    }

    fn evaluate_opponent_choice_condition(&self, condition: &Condition) -> bool {
        // Check if opponent made a specific choice
        // Pattern: "相手は手札を1枚控え室に置いてもよい。そうしなかった"
        // The "そうしなかった" means the opponent chose NOT to do the optional action
        
        let _target = condition.target.as_deref().unwrap_or("opponent");
        let negation = condition.negation.unwrap_or(false);
        // Note: action type is determined from condition text parsing, not a separate field
        
        // Check if opponent_choice_declined flag is set in game_state
        // This would be set when opponent declines an optional action
        let opponent_declined = self.game_state.opponent_choice_declined;
        
        // If negation is true ("そうしなかった"), we return true when opponent declined
        // If negation is false, we return true when opponent accepted
        if negation {
            opponent_declined
        } else {
            !opponent_declined
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

    fn get_count_for_target(&self, condition: &Condition, target: &str) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let comparison_type = condition.comparison_type.as_deref(); // e.g., "score", "cost", "energy"

        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        // Handle comparison_type for more specific counts
        if let Some(comp_type) = comparison_type {
            match comp_type {
                "score" => {
                    // Get total score from success_live_card_zone
                    let mut total_score = 0;
                    for card_id in &player.success_live_card_zone.cards {
                        if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                            total_score += card.score.unwrap_or(0);
                        }
                    }
                    total_score
                }
                "cost" => {
                    // Get total cost from stage
                    let mut total_cost = 0;
                    for card_id in &player.stage.stage {
                        if *card_id != -1 {
                            if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                                total_cost += card.cost.unwrap_or(0);
                            }
                        }
                    }
                    total_cost
                }
                "energy" => {
                    // Get energy count
                    player.energy_zone.cards.len() as u32
                }
                _ => {
                    // Default to location-based count
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
            }
        } else {
            // No comparison_type, use location-based count
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
                    return self.game_state.position_change_occurred_this_turn;
                }
                Keyword::FormationChange => {
                    // Only valid when a formation change has occurred this turn
                    return self.game_state.formation_change_occurred_this_turn;
                }
            }
        }
        true
    }

    /// Extract the original event from replacement effect text
    /// For example, "draw card" from "instead of drawing, do X"
    fn extract_original_event_from_text(&self, text: &str) -> Option<String> {
        // Extract the original event type from replacement effect text
        // Handles both Japanese and English patterns
        let text_lower = text.to_lowercase();
        
        // Draw patterns
        if text.contains("引く時") || text.contains("引く場合") || 
           text_lower.contains("when you draw") || text_lower.contains("instead of drawing") {
            Some("draw".to_string())
        } 
        // Energy payment patterns
        else if text.contains("支払う時") || text.contains("支払う場合") ||
                text_lower.contains("when you pay") || text_lower.contains("instead of paying") {
            Some("pay_energy".to_string())
        } 
        // Card placement/movement patterns
        else if text.contains("置く時") || text.contains("置く場合") ||
                text.contains("出す時") || text.contains("出す場合") ||
                text_lower.contains("when you place") || text_lower.contains("instead of placing") ||
                text_lower.contains("when you move") || text_lower.contains("instead of moving") {
            Some("move_cards".to_string())
        } 
        // Live patterns
        else if text.contains("ライブする時") || text.contains("ライブする場合") ||
                text_lower.contains("when you live") || text_lower.contains("instead of playing a live") {
            Some("live".to_string())
        }
        // Appear/Debut patterns
        else if text.contains("登場する時") || text.contains("デビューする時") ||
                text_lower.contains("when it appears") || text_lower.contains("instead of debuting") {
            Some("appear".to_string())
        }
        // Damage patterns
        else if text.contains("ダメージを受ける時") || text.contains("ダメージが与えられる時") ||
                text_lower.contains("when you take damage") || text_lower.contains("instead of taking damage") {
            Some("take_damage".to_string())
        }
        // Score patterns
        else if text.contains("得点が増える時") || text.contains("スコアが増える時") ||
                text_lower.contains("when you gain score") || text_lower.contains("instead of gaining score") {
            Some("gain_score".to_string())
        }
        else {
            // Unknown pattern - log for debugging
            eprintln!("Unknown replacement effect pattern: {}", text);
            None
        }
    }

    /// Execute an ability effect
    pub fn execute_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        eprintln!("DEBUG: EXECUTING EFFECT - action: {}, text: {}, target: {:?}", 
            effect.action, effect.text, effect.target);
        
        // First, check activation conditions (gates whether ability can be used)
        if !self.can_activate_effect(effect) {
            eprintln!("DEBUG: Activation condition not met, skipping effect");
            return Ok(()); // Activation condition not met, skip effect
        }

        // Then, check if there's a condition for the effect itself
        if let Some(ref condition) = effect.condition {
            eprintln!("DEBUG: Checking effect condition: {:?}", condition);
            if !self.evaluate_condition(condition) {
                eprintln!("DEBUG: Effect condition not met, skipping effect");
                return Ok(()); // Condition not met, skip effect
            }
            eprintln!("DEBUG: Effect condition met");
        } else {
            eprintln!("DEBUG: No effect condition to check");
        }

        // Check for opponent action (action_by field)
        if effect.action_by.as_deref() == Some("opponent") {
            eprintln!("DEBUG: Opponent action detected");
            if let Some(ref opponent_action) = effect.opponent_action {
                eprintln!("DEBUG: Executing opponent action: {:?}", opponent_action);
                // Execute the opponent action on the opponent's game state
                // For now, execute it directly - full implementation would need proper opponent state management
                self.execute_effect(opponent_action)?;
            }
        }

        // Rule 9.10: Check for replacement effects before executing the action
        // Reset replacement effect flags for this new event
        self.game_state.reset_replacement_effect_flags();

        // Use the action from the effect (abilities.json should have this populated)
        let action_to_use = effect.action.clone();

        // Check for replacement effects for this action
        let replacement_effects: Vec<crate::game_state::ReplacementEffect> = self.game_state.get_replacement_effects_for_event(&action_to_use)
            .iter()
            .map(|r| (*r).clone())
            .collect();
        if !replacement_effects.is_empty() {
            eprintln!("DEBUG: Found {} replacement effects for action '{}'", replacement_effects.len(), action_to_use);
            // Rule 9.10.2: If multiple replacement effects, affected player decides order
            // Apply all replacement effects in order
            for replacement in &replacement_effects {
                // Rule 9.10.3: Choice-based replacement effects require the choice to be executable
                if replacement.is_choice_based {
                    eprintln!("DEBUG: Choice-based replacement effect found, prompting user");
                    // Create a choice for the player to decide whether to apply the replacement
                    let description = format!("Apply replacement effect for action '{}'?", action_to_use);
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "apply_replacement".to_string(),
                        description,
                    });
                    self.game_state.pending_choice = self.pending_choice.clone();
                    return Err("Pending choice required: apply replacement effect".to_string());
                } else {
                    // Apply the replacement effects instead of the original action
                    eprintln!("Applying replacement effect for action: {}", action_to_use);
                    for replacement_effect in &replacement.replacement_effects {
                        self.execute_effect(replacement_effect)?;
                    }
                    // Mark as applied (Rule 9.10.2.3)
                    self.game_state.mark_replacement_effect_applied(replacement.card_id);
                }
            }
            return Ok(());
        }

        // Rule 9.2.1: If this effect is a replacement effect, register it instead of executing
        if let Some(ref effect_type) = effect.effect_type {
            if effect_type == "replacement" {
                // Parse the replacement effect to determine what event it replaces
                // Extract the original event from the effect text (supports Japanese and English)
                let original_event = self.extract_original_event_from_text(&effect.text);
                let is_choice_based = effect.text.contains("してよい") || effect.text.contains("may");
                
                // Get the current card being activated
                let card_id = self.game_state.activating_card.unwrap_or(-1);
                let player_id = if self.game_state.current_turn_phase == crate::game_state::TurnPhase::FirstAttackerNormal {
                    self.game_state.player1.id.clone()
                } else {
                    self.game_state.player2.id.clone()
                };
                
                // Register the replacement effect
                if let Some(event) = original_event {
                    self.game_state.add_replacement_effect(
                        card_id,
                        player_id,
                        event.clone(),
                        vec![effect.clone()],
                        is_choice_based,
                    );
                    eprintln!("Registered replacement effect for event: {}", event);
                }
                return Ok(());
            }
        }

        eprintln!("DEBUG: Executing effect action: '{}'", action_to_use);
        match action_to_use.as_str() {
            "sequential" => {
                eprintln!("DEBUG: Executing sequential effect");
                self.execute_sequential_effect(effect)
            },
            "conditional_alternative" => {
                eprintln!("DEBUG: Executing conditional_alternative effect");
                self.execute_conditional_alternative(effect)
            },
            "look_and_select" => {
                eprintln!("DEBUG: Executing look_and_select effect");
                self.execute_look_and_select(effect)
            },
            "draw" | "draw_card" => {
                eprintln!("DEBUG: Executing draw effect");
                self.execute_draw(effect)
            },
            "draw_until_count" => {
                eprintln!("DEBUG: Executing draw_until_count effect");
                self.execute_draw_until_count(effect)
            },
            "move_cards" => {
                eprintln!("DEBUG: Executing move_cards effect");
                self.execute_move_cards(effect)
            },
            "gain_resource" => {
                eprintln!("DEBUG: Executing gain_resource effect");
                self.execute_gain_resource(effect)
            },
            "change_state" => {
                eprintln!("DEBUG: Executing change_state effect");
                self.execute_change_state(effect)
            },
            "modify_score" => {
                eprintln!("DEBUG: Executing modify_score effect");
                self.execute_modify_score(effect)
            },
            "modify_required_hearts" => {
                eprintln!("DEBUG: Executing modify_required_hearts effect");
                self.execute_modify_required_hearts(effect)
            },
            "set_cost" => {
                eprintln!("DEBUG: Executing set_cost effect");
                self.execute_set_cost(effect)
            },
            "set_blade_type" => {
                eprintln!("DEBUG: Executing set_blade_type effect");
                self.execute_set_blade_type(effect)
            },
            "set_heart_type" => {
                eprintln!("DEBUG: Executing set_heart_type effect");
                self.execute_set_heart_type(effect)
            },
            "activate_ability" => {
                eprintln!("DEBUG: Executing activate_ability effect");
                self.execute_activate_ability(effect)
            },
            "invalidate_ability" => {
                eprintln!("DEBUG: Executing invalidate_ability effect");
                self.execute_invalidate_ability(effect)
            },
            "gain_ability" => {
                eprintln!("DEBUG: Executing gain_ability effect");
                self.execute_gain_ability(effect)
            },
            "play_baton_touch" => {
                eprintln!("DEBUG: Executing play_baton_touch effect");
                self.execute_play_baton_touch(effect)
            },
            "reveal" => {
                eprintln!("DEBUG: Executing reveal effect");
                self.execute_reveal(effect)
            },
            "select" => {
                eprintln!("DEBUG: Executing select effect");
                self.execute_select(effect)
            },
            "look_at" => {
                eprintln!("DEBUG: Executing look_at effect");
                self.execute_look_at(effect)
            },
            "modify_required_hearts_global" => {
                eprintln!("DEBUG: Executing modify_required_hearts_global effect");
                self.execute_modify_required_hearts_global(effect)
            },
            "modify_yell_count" => {
                eprintln!("DEBUG: Executing modify_yell_count effect");
                self.execute_modify_yell_count(effect)
            },
            "place_energy_under_member" => {
                eprintln!("DEBUG: Executing place_energy_under_member effect");
                self.execute_place_energy_under_member(effect)
            },
            "activation_cost" => {
                eprintln!("DEBUG: Executing activation_cost effect");
                self.execute_activation_cost(effect)
            },
            "position_change" => {
                eprintln!("DEBUG: Executing position_change effect");
                self.execute_position_change(effect)
            },
            "appear" => {
                eprintln!("DEBUG: Executing appear effect");
                self.execute_appear(effect)
            },
            "choice" => {
                eprintln!("DEBUG: Executing choice effect");
                self.execute_choice(effect)
            },
            "pay_energy" => {
                eprintln!("DEBUG: Executing pay_energy effect");
                self.execute_pay_energy(effect)
            },
            "set_card_identity" => {
                eprintln!("DEBUG: Executing set_card_identity effect");
                self.execute_set_card_identity(effect)
            },
            "discard_until_count" => {
                eprintln!("DEBUG: Executing discard_until_count effect");
                self.execute_discard_until_count(effect)
            },
            "restriction" => {
                eprintln!("DEBUG: Executing restriction effect");
                self.execute_restriction(effect)
            },
            "re_yell" => {
                eprintln!("DEBUG: Executing re_yell effect");
                self.execute_re_yell(effect)
            },
            "modify_cost" => {
                eprintln!("DEBUG: Executing modify_cost effect");
                self.execute_modify_cost(effect)
            },
            "activation_restriction" => {
                eprintln!("DEBUG: Executing activation_restriction effect");
                self.execute_activation_restriction(effect)
            },
            "choose_required_hearts" => {
                eprintln!("DEBUG: Executing choose_required_hearts effect");
                self.execute_choose_required_hearts(effect)
            },
            "modify_limit" => {
                eprintln!("DEBUG: Executing modify_limit effect");
                self.execute_modify_limit(effect)
            },
            "set_blade_count" => {
                eprintln!("DEBUG: Executing set_blade_count effect");
                self.execute_set_blade_count(effect)
            },
            "set_required_hearts" => {
                eprintln!("DEBUG: Executing set_required_hearts effect");
                self.execute_set_required_hearts(effect)
            },
            "set_score" => {
                eprintln!("DEBUG: Executing set_score effect");
                self.execute_set_score(effect)
            },
            "specify_heart_color" => {
                eprintln!("DEBUG: Executing specify_heart_color effect");
                self.execute_specify_heart_color(effect)
            },
            "modify_required_hearts_success" => {
                eprintln!("DEBUG: Executing modify_required_hearts_success effect");
                self.execute_modify_required_hearts_success(effect)
            },
            "set_cost_to_use" => {
                eprintln!("DEBUG: Executing set_cost_to_use effect");
                self.execute_set_cost_to_use(effect)
            },
            "all_blade_timing" => {
                eprintln!("DEBUG: Executing all_blade_timing effect");
                self.execute_all_blade_timing(effect)
            },
            "set_card_identity_all_regions" => {
                eprintln!("DEBUG: Executing set_card_identity_all_regions effect");
                self.execute_set_card_identity_all_regions(effect)
            },
            "shuffle" => {
                eprintln!("DEBUG: Executing shuffle effect");
                self.execute_shuffle(effect)
            },
            "reveal_per_group" => {
                eprintln!("DEBUG: Executing reveal_per_group effect");
                self.execute_reveal_per_group(effect)
            },
            "conditional_on_result" => {
                eprintln!("DEBUG: Executing conditional_on_result effect");
                self.execute_conditional_on_result(effect)
            },
            "conditional_on_optional" => {
                eprintln!("DEBUG: Executing conditional_on_optional effect");
                self.execute_conditional_on_optional(effect)
            },
            "custom" => {
                eprintln!("DEBUG: Executing custom effect (not implemented)");
                // Custom actions are card-specific effects not yet implemented
                // For now, skip them without error
                Ok(())
            }
            unknown_action => {
                eprintln!("DEBUG: Unknown action type: '{}', skipping", unknown_action);
                Ok(())
            }
        }
    }

    fn execute_sequential_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let conditional = effect.conditional.unwrap_or(false) || effect.condition.is_some();
        let condition = effect.condition.as_ref();
        let is_further = effect.is_further.unwrap_or(false);

        // Check condition if this is a conditional sequential effect
        if conditional {
            if let Some(cond) = condition {
                // Evaluate the condition before executing actions
                let condition_met = self.evaluate_condition(cond);
                eprintln!("Conditional sequential effect with condition: {:?}, met: {}", cond, condition_met);
                
                // If condition is not met, skip execution
                if !condition_met {
                    eprintln!("Condition not met, skipping sequential actions");
                    return Ok(());
                }
            }
        }

        // Handle "further" (さらに) pattern - this indicates additional conditional effects
        if is_further {
            eprintln!("Further conditional effect (さらに) - executing additional actions");
        }

        if let Some(ref actions) = effect.actions {
            for (i, action) in actions.iter().enumerate() {
                // Check if this is an opponent action
                let is_opponent_action = action.target.as_deref() == Some("opponent");
                if is_opponent_action {
                    eprintln!("Opponent action in sequence: {}", action.action);
                    // For now, execute normally - full implementation would need opponent interaction
                }

                // Propagate per_unit information from parent to child actions
                let mut action_to_execute = action.clone();
                if action_to_execute.per_unit.is_none() && effect.per_unit.is_some() {
                    action_to_execute.per_unit = effect.per_unit;
                }
                if action_to_execute.per_unit_count.is_none() && effect.per_unit_count.is_some() {
                    action_to_execute.per_unit_count = effect.per_unit_count;
                }
                if action_to_execute.per_unit_type.is_none() && effect.per_unit_type.is_some() {
                    action_to_execute.per_unit_type = effect.per_unit_type.clone();
                }
                
                // Handle pending choices in sequential effects
                match self.execute_effect(&action_to_execute) {
                    Ok(_) => {},
                    Err(e) if e.contains("Pending choice required") => {
                        // Save the remaining actions to resume after user choice
                        let remaining_actions: Vec<AbilityEffect> = actions[i + 1..].to_vec();
                        if !remaining_actions.is_empty() {
                            self.game_state.pending_sequential_actions = Some(remaining_actions);
                        }
                        return Err(e);
                    }
                    Err(e) => return Err(e),
                }
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
            let primary_text = effect.primary_effect.as_ref().map(|e| e.text.as_str())
                .unwrap_or("Primary effect");
            let alternative_text = effect.alternative_effect.as_ref().map(|e| e.text.as_str())
                .unwrap_or("Alternative effect");
            
            let description = format!("Choose effect:\nPrimary: {}\nAlternative: {}", primary_text, alternative_text);
            
            self.pending_choice = Some(Choice::SelectTarget {
                target: "primary|alternative".to_string(),
                description: description,
            });
            self.game_state.pending_choice = self.pending_choice.clone();
            // Store execution context to resume after user choice
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
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
        // Store current effect for resuming after user choice
        self.current_effect = Some(effect.clone());

        // Execute look action first (stores cards in looked_at_cards)
        if let Some(ref look_action) = effect.look_action {
            self.execute_effect(look_action)?;
        }

        // Check if select_action has placement_order or any_number parameter
        if let Some(ref select_action) = effect.select_action {
            let placement_order = select_action.placement_order.as_deref();
            let count = select_action.count.unwrap_or(1);
            let optional = select_action.optional.unwrap_or(false);
            let any_number = select_action.any_number.unwrap_or(false);

            // look_and_select ALWAYS requires user choice - the user must select which cards to keep
            // The flags (optional, any_number, placement_order) just modify the selection behavior
            let available_count = self.looked_at_cards.len();

            let max_select = if any_number {
                available_count // Can select up to all available cards
            } else {
                count as usize // Limited to specific count
            };

            let description = if any_number {
                if optional {
                    format!("Select any number of cards from the {} looked-at cards (or skip) (placement_order: {})",
                        available_count, placement_order.unwrap_or("default"))
                } else {
                    format!("Select any number of cards from the {} looked-at cards (placement_order: {})",
                        available_count, placement_order.unwrap_or("default"))
                }
            } else if optional {
                format!("Select up to {} card(s) from the {} looked-at cards (or skip) (placement_order: {})",
                    count, available_count, placement_order.unwrap_or("default"))
            } else {
                format!("Select {} card(s) from the {} looked-at cards (placement_order: {})",
                    count, available_count, placement_order.unwrap_or("default"))
            };

            let choice = Choice::SelectCard {
                zone: "looked_at".to_string(),
                card_type: None,
                count: max_select,
                description,
                allow_skip: optional || any_number, // Allow skip if optional or any_number
            };
            self.pending_choice = Some(choice.clone());
            self.game_state.pending_choice = Some(choice);
            // Store execution context to resume after user choice
            self.execution_context = ExecutionContext::LookAndSelect {
                step: LookAndSelectStep::Select { count: max_select },
            };
            // Return early - execution will continue after user provides choice
            return Ok(());
        }

        // Otherwise execute select action normally (shouldn't happen for look_and_select)
        if let Some(ref select_action) = effect.select_action {
            self.execute_effect(select_action)?;
        }

        // Clear current effect after execution
        self.current_effect = None;
        Ok(())
    }

    fn execute_draw(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let _max = effect.max.unwrap_or(false);
        let target = effect.target.as_deref().unwrap_or("self");
        let source = effect.source.as_deref().unwrap_or("deck");
        let destination = effect.destination.as_deref().unwrap_or("hand");
        let card_type_filter = effect.card_type.as_deref();
        let resource_icon_count = effect.resource_icon_count;
        let _group_filter: Option<&String> = None;
        let _cost_limit: Option<u32> = None;
        let per_unit = effect.per_unit;
        let per_unit_count = effect.per_unit_count.unwrap_or(1);
        let per_unit_type = effect.per_unit_type.as_deref();

        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();

        // Handle "both" target (Q229)
        if target == "both" {
            // Apply to both players - need to clone card_db for each call
            let card_db1 = card_db.clone();
            let card_db2 = card_db.clone();
            {
                let p1 = &mut self.game_state.player1;
                Self::draw_cards_for_player(p1, count, source, destination, card_type_filter, &card_db1)?;
            }
            {
                let p2 = &mut self.game_state.player2;
                Self::draw_cards_for_player(p2, count, source, destination, card_type_filter, &card_db2)?;
            }
            return Ok(());
        }

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Helper function to check if card matches type filter
        let _matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
            match filter {
                Some("live_card") => card_db.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                Some("member_card") => card_db.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                Some("energy_card") => card_db.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                None => true,
                _ => true, // Unknown type, allow all
            }
        };

        // Helper function to check if card matches group filter
        let _matches_group = |card_id: i16, filter: Option<&String>| -> bool {
            match filter {
                Some(group_name) => card_db.get_card(card_id).map(|c| c.group == *group_name).unwrap_or(false),
                None => true,
            }
        };

        // Helper function to check if card matches cost limit
        let _matches_cost_limit = |card_id: i16, limit: Option<u32>| -> bool {
            match limit {
                Some(max_cost) => card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0) <= max_cost).unwrap_or(false),
                None => true,
            }
        };

        // Calculate final count with per-unit scaling
        let final_count = if per_unit.unwrap_or(false) {
            // Calculate based on per_unit_count and per_unit_type
            let multiplier = match per_unit_type {
                Some("member") | Some("人") => player.stage.stage.iter().filter(|&&c| c != -1).count() as u32,
                Some("energy") => player.energy_zone.cards.len() as u32,
                Some("hand") => player.hand.cards.len() as u32,
                _ => 1,
            };
            count * multiplier * per_unit_count
        } else {
            count
        };

        match source.as_ref() {
            "deck" | "deck_top" => {
                // Clone card_db to avoid borrow issues
                let card_db_clone = card_db.clone();
                Self::draw_cards_for_player(player, final_count, source, destination, card_type_filter, &card_db_clone)?;
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

    fn draw_cards_for_player(player: &mut crate::player::Player, count: u32, _source: &str, destination: &str, card_type_filter: Option<&str>, card_db: &crate::card::CardDatabase) -> Result<(), String> {
        let mut drawn = 0;
        while drawn < count {
            if let Some(card) = player.main_deck.draw() {
                let matches_type = match card_type_filter {
                    Some("live_card") => card_db.get_card(card).map(|c| c.is_live()).unwrap_or(false),
                    Some("member_card") => card_db.get_card(card).map(|c| c.is_member()).unwrap_or(false),
                    Some("energy_card") => card_db.get_card(card).map(|c| c.is_energy()).unwrap_or(false),
                    None => true,
                    _ => true,
                };
                
                if matches_type {
                    match destination.as_ref() {
                        "hand" => player.hand.add_card(card),
                        _ => {
                            eprintln!("Draw destination '{}' not yet implemented", destination);
                            player.hand.add_card(card);
                        }
                    }
                    drawn += 1;
                } else {
                    player.main_deck.cards.push(card);
                }
            } else {
                break;
            }
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
        let position = effect.position.as_ref().and_then(|p| p.get_position());

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
        
        // Track the cost modification using temporary effects system
        // This properly tracks cost modifications with duration support
        let prohibition_text = format!("activation_cost_{}_{}", operation, value);
        
        match target {
            "self" => {
                self.game_state.prohibition_effects.push(prohibition_text);
            }
            "opponent" => {
                self.game_state.prohibition_effects.push(prohibition_text);
            }
            _ => {
                // Do nothing for other targets
            }
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
                    creation_order: 0,
                    effect_data: None,
                };
                self.game_state.temporary_effects.push(temp_effect);
            }
        }
        
        Ok(())
    }

    fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let _max = effect.max.unwrap_or(false);
        let source = effect.source.as_deref().unwrap_or("").to_string();
        let destination = effect.destination.as_deref().unwrap_or("").to_string();
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let _placement_order = effect.placement_order.as_deref();

        // Handle effect_constraint (e.g., minimum/maximum value constraints)
        if let Some(ref constraint) = effect.effect_constraint {
            eprintln!("Effect constraint: {}", constraint);
            // Parse constraint format: "constraint_type:value"
            // For now, just log - full implementation would enforce the constraint
        }

        // Handle deck_position (e.g., "4th from top")
        if let Some(ref position_info) = effect.position {
            if let Some(deck_pos) = position_info.get_position() {
                eprintln!("Deck position: {}", deck_pos);
                // For now, just log - full implementation would place card at specific position
            }
        }

        let _optional = effect.optional.unwrap_or(false);
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));
        let cost_limit = effect.cost_limit;
        let _placement_order = effect.placement_order.as_deref();

        // Source and destination should be populated from abilities.json
        // No text-based fallback - data should be clean

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Clone card_database to avoid borrow conflicts
        let card_db = self.game_state.card_database.clone();

        // Zone count validation - check if source has enough cards
        // Note: This is a basic count check; actual matching may reduce available cards
        // Per rules 1.3.2, we should do as much as possible, so this is just a warning
        let source_card_count = match source.as_str() {
            "stage" => player.stage.stage.iter().filter(|&&x| x != -1).count(),
            "deck" | "deck_top" => player.main_deck.cards.len(),
            "hand" => player.hand.cards.len(),
            "discard" => player.waitroom.cards.len(),
            "energy_zone" => player.energy_zone.cards.len(),
            "live_card_zone" => player.live_card_zone.cards.len(),
            "success_live_zone" => player.success_live_card_zone.cards.len(),
            _ => 0,
        };

        if source_card_count < (count as usize) {
            eprintln!("Warning: Source '{}' has only {} cards, but trying to move {} cards. Will do as much as possible per rules 1.3.2", source, source_card_count, count);
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

        // Helper function to check if card matches cost limit
        let matches_cost_limit = |card_id: i16, limit: Option<u32>| -> bool {
            match limit {
                Some(max_cost) => card_db.get_card(card_id).map(|c| c.cost.unwrap_or(0) <= max_cost).unwrap_or(false),
                None => true,
            }
        };

        match source.as_ref() {
            "stage" => {
                // Handle self-cost and exclude_self for stage moves
                let is_self_cost = effect.self_cost.unwrap_or(false);
                let exclude_self = effect.exclude_self.unwrap_or(false);
                
                if is_self_cost {
                    // Self-cost: the activating card itself is the cost
                    if let Some(activating_card_id) = self.game_state.activating_card {
                        let mut found = false;
                        for i in 0..3 {
                            if player.stage.stage[i] == activating_card_id {
                                if matches_card_type(activating_card_id, card_type_filter) &&
                                   matches_group(activating_card_id, group_filter) &&
                                   matches_cost_limit(activating_card_id, cost_limit) {
                                    let card_id = player.stage.stage[i];
                                    
                                    // Skip if destination is stage (no-op) - don't remove from stage
                                    if destination == "stage" {
                                        eprintln!("Self-cost: skipping move of activating card {} from stage to stage (no-op)", activating_card_id);
                                        found = true;
                                        break;
                                    }
                                    
                                    player.stage.stage[i] = -1;
                                    match destination.as_ref() {
                                        "discard" => {
                                            player.waitroom.add_card(card_id);
                                        }
                                        "hand" => {
                                            player.hand.add_card(card_id);
                                        }
                                        "deck_bottom" => {
                                            player.main_deck.cards.push(card_id);
                                        }
                                        "deck_top" => {
                                            player.main_deck.cards.insert(0, card_id);
                                        }
                                        "same_area" => {
                                            // Place in the same area as the activating card
                                            if let Some(activating_card_id) = self.activating_card_id {
                                                // Find which position the activating card was in
                                                for (pos_idx, &stage_card_id) in player.stage.stage.iter().enumerate() {
                                                    if stage_card_id == activating_card_id {
                                                        // Place the card in the same position
                                                        player.stage.stage[pos_idx] = card_id;
                                                        eprintln!("Placed card {} in same area as activating card at position {}", card_id, pos_idx);
                                                        break;
                                                    }
                                                }
                                            } else {
                                                // No activating card, default to hand
                                                player.hand.add_card(card_id);
                                            }
                                        }
                                        "live_card_zone" => {
                                            // Validate card type - only live cards can go to live card zone
                                            if card_type_filter.is_none() || matches_card_type(card_id, Some("live_card")) {
                                                player.live_card_zone.cards.push(card_id);
                                            } else {
                                                return Err(format!("Card {} is not a live card, cannot move to live_card_zone", card_id));
                                            }
                                        }
                                        "success_live_zone" => {
                                            // Validate card type - only live cards can go to success live zone
                                            if card_type_filter.is_none() || matches_card_type(card_id, Some("live_card")) {
                                                player.success_live_card_zone.cards.push(card_id);
                                            } else {
                                                return Err(format!("Card {} is not a live card, cannot move to success_live_zone", card_id));
                                            }
                                        }
                                        _ => {
                                            player.hand.add_card(card_id);
                                        }
                                    }
                                    eprintln!("Self-cost: moved activating card {} from stage position {} to {}", activating_card_id, i, destination);
                                    found = true;
                                } else {
                                    return Err(format!("Activating card {} does not match cost requirements", activating_card_id));
                                }
                                break;
                            }
                        }
                        if !found {
                            return Err(format!("Activating card {} not found on stage", activating_card_id));
                        }
                    } else {
                        return Err("Self-cost required but no activating card tracked".to_string());
                    }
                } else {
                    // Non-self-cost: collect valid cards, apply exclude_self filter, then move
                    let mut valid_cards: Vec<(usize, i16)> = Vec::new();
                    let activating_card_id = self.game_state.activating_card;
                    
                    for i in 0..3 {
                        if player.stage.stage[i] != -1 {
                            let card_id = player.stage.stage[i];
                            // Skip if exclude_self and this is the activating card
                            if exclude_self && activating_card_id == Some(card_id) {
                                eprintln!("Excluding activating card {} from selection", card_id);
                                continue;
                            }
                            if matches_card_type(card_id, card_type_filter) && 
                               matches_group(card_id, group_filter) && 
                               matches_cost_limit(card_id, cost_limit) {
                                valid_cards.push((i, card_id));
                            }
                        }
                    }
                    
                    if valid_cards.len() < (count as usize) {
                        return Err(format!(
                            "Not enough valid cards on stage: needed {}, have {} (after exclude_self filter)",
                            count, valid_cards.len()
                        ));
                    }
                    
                    // If more valid cards than needed, prompt user to choose
                    if valid_cards.len() > (count as usize) {
                        let card_type_desc = if let Some(ct) = card_type_filter {
                            format!("{} ", ct)
                        } else {
                            "".to_string()
                        };
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "stage".to_string(),
                            card_type: card_type_filter.map(|s| s.to_string()),
                            count: count as usize,
                            description: format!("Select {} {}card(s) from stage to move to {} ({} available)", count, card_type_desc, destination, valid_cards.len()),
                            allow_skip: false,
                        });
                        self.game_state.pending_choice = self.pending_choice.clone();
                        // Store execution context to resume after user choice
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        // Return early - execution will continue after user provides choice
                        return Ok(());
                    }
                    
                    // If exact number or fewer (already checked), move them automatically
                    let mut cards_to_record: Vec<i16> = Vec::new();
                    for (i, card_id) in valid_cards.iter().take(count as usize) {
                        // Skip if source and destination are both stage (no-op)
                        if destination == "stage" {
                            eprintln!("Skipping move of card {} from stage to stage (no-op)", card_id);
                            continue;
                        }
                        
                        player.stage.stage[*i] = -1;
                        match destination.as_ref() {
                            "discard" => {
                                player.waitroom.add_card(*card_id);
                            }
                            "hand" => {
                                player.hand.add_card(*card_id);
                            }
                            "deck_bottom" => {
                                player.main_deck.cards.push(*card_id);
                            }
                            "deck_top" => {
                                player.main_deck.cards.insert(0, *card_id);
                            }
                            "live_card_zone" => {
                                // Validate card type - only live cards can go to live card zone
                                if card_type_filter.is_none() || matches_card_type(*card_id, Some("live_card")) {
                                    player.live_card_zone.cards.push(*card_id);
                                } else {
                                    eprintln!("Card {} is not a live card, cannot move to live_card_zone", card_id);
                                    player.hand.add_card(*card_id); // Return to hand instead
                                }
                            }
                            "success_live_zone" => {
                                // Validate card type - only live cards can go to success live zone
                                if card_type_filter.is_none() || matches_card_type(*card_id, Some("live_card")) {
                                    player.success_live_card_zone.cards.push(*card_id);
                                } else {
                                    eprintln!("Card {} is not a live card, cannot move to success_live_zone", card_id);
                                    player.hand.add_card(*card_id); // Return to hand instead
                                }
                            }
                            _ => {
                                player.hand.add_card(*card_id);
                            }
                        }
                        cards_to_record.push(*card_id);
                        eprintln!("Moved card {} from stage position {} to {}", card_id, i, destination);
                    }
                    // Record movements after mutable borrow ends
                    for card_id in cards_to_record {
                        self.game_state.record_card_movement(card_id);
                    }
                }
            }
            "deck" | "deck_top" => {
                let mut moved = 0;
                let mut cards_drawn = 0;
                let mut cards_to_record: Vec<i16> = Vec::new();
                // Prevent infinite loop by tracking total cards drawn
                let max_draws = player.main_deck.cards.len() + count as usize;
                
                while moved < count && cards_drawn < max_draws {
                    if let Some(card) = player.main_deck.draw() {
                        cards_drawn += 1;
                        if matches_card_type(card, card_type_filter) && matches_group(card, group_filter) && matches_cost_limit(card, cost_limit) {
                            match destination.as_ref() {
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
                                    cards_to_record.push(card);
                                }
                                "live_card_zone" => {
                                    // Validate card type - only live cards can go to live card zone
                                    if card_type_filter.is_none() || matches_card_type(card, Some("live_card")) {
                                        player.live_card_zone.cards.push(card);
                                    } else {
                                        eprintln!("Card {} is not a live card, cannot move to live_card_zone", card);
                                        player.main_deck.cards.push(card); // Put back on bottom
                                    }
                                }
                                "success_live_zone" => {
                                    // Validate card type - only live cards can go to success live zone
                                    if card_type_filter.is_none() || matches_card_type(card, Some("live_card")) {
                                        player.success_live_card_zone.cards.push(card);
                                    } else {
                                        eprintln!("Card {} is not a live card, cannot move to success_live_zone", card);
                                        player.main_deck.cards.push(card); // Put back on bottom
                                    }
                                }
                                "deck_top" => {
                                    player.main_deck.cards.insert(0, card);
                                }
                                _ => {
                                    eprintln!("Move to destination '{}' not yet implemented", destination);
                                }
                            }
                            moved += 1;
                        } else {
                            // Card doesn't match filter, put it back on bottom of deck
                            player.main_deck.cards.push(card);
                            // Continue drawing to find matching cards
                        }
                    } else {
                        break; // Deck empty
                    }
                }
                
                // Record movements after mutable borrow ends
                for card_id in cards_to_record {
                    self.game_state.record_card_movement(card_id);
                }
                
                // If we couldn't draw enough matching cards, that's okay per rules 1.3.2
                // "possibleな限りその行動を行います" - do as much as possible
            }
            "hand" => {
                match destination.as_ref() {
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
                        self.game_state.pending_choice = self.pending_choice.clone();
                        // Store execution context to resume after user choice
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.push(card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.insert(0, card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            cards_to_record.push(card);
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
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_record {
                            self.game_state.clear_modifiers_for_card(card_id);
                        }
                    }
                    "live_card_zone" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.hand.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            // Validate card type - only live cards can go to live card zone
                            if matches_card_type(*card, Some("live_card")) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.hand.cards.remove(i);
                            cards_to_clear.push(card);
                            player.live_card_zone.cards.push(card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
                        }
                    }
                    _ => {}
                }
            }
            "discard" => {
                match destination.as_ref() {
                    "hand" => {

                        // First, count how many matching cards are in discard
                        let matching_indices: Vec<usize> = player.waitroom.cards.iter().enumerate()
                            .filter(|(_, card)| {
                                matches_card_type(**card, card_type_filter) && 
                                matches_group(**card, group_filter) && 
                                matches_cost_limit(**card, cost_limit)
                            })
                            .map(|(i, _)| i)
                            .collect();

                        if matching_indices.len() < (count as usize) {
                            return Err(format!(
                                "Not enough cards in discard: needed {}, have {}",
                                count, matching_indices.len()
                            ));
                        }

                        // If there are more matching cards than needed, prompt user to choose
                        if matching_indices.len() > (count as usize) {
                            let card_type_desc = if let Some(ct) = card_type_filter {
                                format!("{} ", ct)
                            } else {
                                "".to_string()
                            };
                            self.pending_choice = Some(Choice::SelectCard {
                                zone: "discard".to_string(),
                                card_type: card_type_filter.map(|s| s.to_string()),
                                count: count as usize,
                                description: format!("Select {} {}card(s) from discard to add to hand ({} available)", count, card_type_desc, matching_indices.len()),
                                allow_skip: false,
                            });
                            self.game_state.pending_choice = self.pending_choice.clone();
                            // Store execution context to resume after user choice
                            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                            // Return early - execution will continue after user provides choice
                            return Ok(());
                        }

                        // If exact number or fewer (already checked), move them automatically
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
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            player.hand.add_card(card);
                            // Modifiers stay when moving to hand (card still in play)
                        }
                        // Record movements after mutable borrow ends
                        for card_id in cards_to_record {
                            self.game_state.record_card_movement(card_id);
                        }
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
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            player.main_deck.cards.push(card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in &cards_to_record {
                            self.game_state.clear_modifiers_for_card(*card_id);
                        }
                        // Record movements after mutable borrow ends
                        for card_id in &cards_to_record {
                            self.game_state.record_card_movement(*card_id);
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
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            player.main_deck.cards.insert(0, card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in &cards_to_record {
                            self.game_state.clear_modifiers_for_card(*card_id);
                        }
                        // Record movements after mutable borrow ends
                        for card_id in &cards_to_record {
                            self.game_state.record_card_movement(*card_id);
                        }
                    }
                    "deck" => {
                        // Handle position-based deck placement (Q226: 一番上から4枚目)
                        let position_info = effect.position.as_ref();
                        let placement_order = effect.placement_order.as_deref();
                        
                        // Check if player needs to choose order (any_order)
                        if placement_order == Some("any_order") && count > 1 {
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
                            // Remove cards and store them for ordering
                            let mut cards_to_place: Vec<i16> = Vec::new();
                            for i in indices_to_remove.into_iter().rev() {
                                let card = player.waitroom.cards.remove(i);
                                cards_to_place.push(card);
                            }
                            // Clear modifiers after mutable borrow ends
                            for card_id in cards_to_place.iter() {
                                self.game_state.clear_modifiers_for_card(*card_id);
                            }
                            // Request player to choose order
                            let card_db = self.game_state.card_database.clone();
                            let card_names: Vec<String> = cards_to_place.iter()
                                .map(|&card_id| {
                                    card_db.get_card(card_id)
                                        .map(|c| c.name.clone())
                                        .unwrap_or(format!("Card {}", card_id))
                                })
                                .collect();
                            
                            self.pending_choice = Some(Choice::SelectTarget {
                                target: cards_to_place.iter().map(|&id| id.to_string()).collect::<Vec<_>>().join("|"),
                                description: format!("Choose order for cards to place on deck:\n{}", 
                                    card_names.iter().enumerate()
                                        .map(|(i, name)| format!("{}. {}", i + 1, name))
                                        .collect::<Vec<_>>()
                                        .join("\n")),
                            });
                            // Store cards temporarily for placement after choice
                            self.looked_at_cards = cards_to_place;
                            // Store execution context
                            self.execution_context = ExecutionContext::LookAndSelect {
                                step: LookAndSelectStep::Finalize { destination: "deck".to_string() },
                            };
                            return Ok(());
                        }
                        
                        // Standard placement (not any_order or single card)
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
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            // Calculate insertion index based on position
                            // PositionInfo has position field as Option<String>
                            // Position is 1-indexed from top (e.g., 4 = 4th from top)
                            // If deck has fewer cards than position, place at bottom
                            if let Some(pos_info) = position_info {
                                let deck_len = player.main_deck.cards.len();
                                let insert_index = if let Some(pos_str) = pos_info.get_position() {
                                    if let Ok(pos_num) = pos_str.parse::<usize>() {
                                        if pos_num > deck_len {
                                            deck_len // Place at bottom if position exceeds deck size
                                        } else {
                                            pos_num.saturating_sub(1) // Convert to 0-indexed
                                        }
                                    } else {
                                        0 // Default to top if parsing fails
                                    }
                                } else {
                                    0 // No position specified, place at top
                                };
                                player.main_deck.cards.insert(insert_index, card);
                            } else {
                                // No position specified, place at top
                                player.main_deck.cards.insert(0, card);
                            }
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in &cards_to_record {
                            self.game_state.clear_modifiers_for_card(*card_id);
                        }
                        // Record movements after mutable borrow ends
                        for card_id in &cards_to_record {
                            self.game_state.record_card_movement(*card_id);
                        }
                    }
                    "stage" => {
                        let mut moved = 0;
                        let mut indices_to_remove = Vec::new();
                        for (i, card) in player.waitroom.cards.iter().enumerate() {
                            if moved >= count {
                                break;
                            }
                            // Validate card type - only member cards can go to stage
                            if matches_card_type(*card, Some("member_card")) && matches_group(*card, group_filter) && matches_cost_limit(*card, cost_limit) {
                                indices_to_remove.push(i);
                                moved += 1;
                            }
                        }
                        // Remove in reverse order to maintain indices
                        let mut cards_to_record: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_record.push(card);
                            // Check which stage areas are available
                            let available_areas: Vec<&str> = vec![
                                if player.stage.stage[1] == -1 { Some("center") } else { None },
                                if player.stage.stage[0] == -1 { Some("left_side") } else { None },
                                if player.stage.stage[2] == -1 { Some("right_side") } else { None },
                            ].into_iter().filter_map(|x| x).collect();

                            if available_areas.len() > 1 {
                                // Multiple areas available - let user choose
                                let areas_str = available_areas.join(", ");
                                self.pending_choice = Some(Choice::SelectPosition {
                                    position: areas_str.clone(),
                                    description: format!("Select stage area to place card (available: {})", areas_str),
                                });
                                self.game_state.pending_choice = Some(Choice::SelectPosition {
                                    position: areas_str.clone(),
                                    description: format!("Select stage area to place card (available: {})", areas_str),
                                });
                                // Store the card temporarily for placement after choice
                                self.looked_at_cards.push(card);
                                // Store execution context
                                self.execution_context = ExecutionContext::LookAndSelect {
                                    step: LookAndSelectStep::Finalize { destination: "stage".to_string() },
                                };
                                return Ok(());
                            } else if available_areas.len() == 1 {
                                // Only one area available - place automatically
                                let area = available_areas[0];
                                match area {
                                    "center" => {
                                        player.stage.stage[1] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::Center);
                                    }
                                    "left_side" => {
                                        player.stage.stage[0] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::LeftSide);
                                    }
                                    "right_side" => {
                                        player.stage.stage[2] = card;
                                        player.areas_locked_this_turn.insert(crate::zones::MemberArea::RightSide);
                                    }
                                    _ => {
                                        player.hand.add_card(card); // Fallback
                                    }
                                }
                                cards_to_record.push(card);
                            } else {
                                // No areas available - return to hand
                                player.hand.add_card(card);
                            }
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in &cards_to_record {
                            self.game_state.clear_modifiers_for_card(*card_id);
                        }
                        // Record movements after mutable borrow ends
                        for card_id in &cards_to_record {
                            self.game_state.record_card_movement(*card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_clear.push(card);
                            player.live_card_zone.cards.push(card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.waitroom.cards.remove(i);
                            cards_to_clear.push(card);
                            player.success_live_card_zone.cards.push(card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
                        }
                    }
                    _ => {}
                }
            }
            "success_live_zone" => {
                match destination.as_ref() {
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
                            // Modifiers stay when moving to hand (card still in play)
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.push(card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.insert(0, card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.success_live_card_zone.cards.remove(i);
                            cards_to_clear.push(card);
                            // Try to place card in first available stage area (center, left, right)
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
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
                        }
                    }
                    _ => {}
                }
            }
            "live_card_zone" => {
                match destination.as_ref() {
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
                            // Modifiers stay when moving to hand (card still in play)
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            cards_to_clear.push(card);
                            player.waitroom.add_card(card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.push(card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for i in indices_to_remove.into_iter().rev() {
                            let card = player.live_card_zone.cards.remove(i);
                            cards_to_clear.push(card);
                            player.main_deck.cards.insert(0, card);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                            // Modifiers stay when moving between live zones (card still in play)
                        }
                    }
                    _ => {}
                }
            }
            "energy_zone" => {
                match destination.as_ref() {
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for (i, card_id) in indices_to_remove.into_iter().rev() {
                            player.energy_zone.cards.remove(i);
                            cards_to_clear.push(card_id);
                            // Decrement active_energy_count if the removed card was active
                            if player.energy_zone.active_energy_count > 0 {
                                player.energy_zone.active_energy_count -= 1;
                            }
                            player.hand.add_card(card_id);
                            // Modifiers stay when moving to hand (card still in play)
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
                        }
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for (i, card_id) in indices_to_remove.into_iter().rev() {
                            player.energy_zone.cards.remove(i);
                            cards_to_clear.push(card_id);
                            // Decrement active_energy_count if the removed card was active
                            if player.energy_zone.active_energy_count > 0 {
                                player.energy_zone.active_energy_count -= 1;
                            }
                            player.waitroom.cards.push(card_id);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for (i, card_id) in indices_to_remove.into_iter().rev() {
                            player.energy_zone.cards.remove(i);
                            cards_to_clear.push(card_id);
                            // Decrement active_energy_count if the removed card was active
                            if player.energy_zone.active_energy_count > 0 {
                                player.energy_zone.active_energy_count -= 1;
                            }
                            player.main_deck.cards.push(card_id);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
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
                        let mut cards_to_clear: Vec<i16> = Vec::new();
                        for (i, card_id) in indices_to_remove.into_iter().rev() {
                            player.energy_zone.cards.remove(i);
                            cards_to_clear.push(card_id);
                            // Decrement active_energy_count if the removed card was active
                            if player.energy_zone.active_energy_count > 0 {
                                player.energy_zone.active_energy_count -= 1;
                            }
                            player.main_deck.cards.insert(0, card_id);
                        }
                        // Clear modifiers after mutable borrow ends
                        for card_id in cards_to_clear {
                            self.game_state.clear_modifiers_for_card(card_id);
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                eprintln!("Move from source '{}' not yet implemented", source);
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

        let player_id = match target {
            "self" => self.game_state.player1.id.clone(),
            "opponent" => self.game_state.player2.id.clone(),
            _ => self.game_state.player1.id.clone(),
        };

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
        let mut effect_data: Option<serde_json::Value> = None;

        // Collect card_ids and modifier data before applying
        let activating_card_id = self.game_state.activating_card;

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

                // When per_unit is true, we counted cards from a different location (e.g., success_live_card_zone)
                // and want to add the total (final_count) to the activating card on stage
                // When per_unit is false, add final_count to each matching card
                let blades_to_add = if per_unit == Some(true) { final_count as i32 } else { final_count as i32 };

                if card_ids.is_empty() {
                    // If no cards match on stage but we have an activating card, add to it
                    if let Some(card_id) = activating_card_id {
                        self.game_state.add_blade_modifier(card_id, blades_to_add);
                        // Track which card got blades for reversion
                        if is_temporary {
                            let mut data = serde_json::Map::new();
                            data.insert("card_id".to_string(), serde_json::Value::Number(card_id.into()));
                            data.insert("amount".to_string(), serde_json::Value::Number(blades_to_add.into()));
                            effect_data = Some(serde_json::Value::Object(data));
                        }
                    }
                } else {
                    // For per_unit abilities, add total to activating card if present in card_ids
                    // Otherwise, distribute evenly
                    if per_unit == Some(true) {
                        if let Some(card_id) = activating_card_id {
                            if card_ids.contains(&card_id) {
                                self.game_state.add_blade_modifier(card_id, blades_to_add);
                                if is_temporary {
                                    let mut data = serde_json::Map::new();
                                    data.insert("card_id".to_string(), serde_json::Value::Number(card_id.into()));
                                    data.insert("amount".to_string(), serde_json::Value::Number(blades_to_add.into()));
                                    effect_data = Some(serde_json::Value::Object(data));
                                }
                            } else {
                                // Activating card not in card_ids, add to first matching card
                                if let Some(&first_card_id) = card_ids.first() {
                                    self.game_state.add_blade_modifier(first_card_id, blades_to_add);
                                    if is_temporary {
                                        let mut data = serde_json::Map::new();
                                        data.insert("card_id".to_string(), serde_json::Value::Number(first_card_id.into()));
                                        data.insert("amount".to_string(), serde_json::Value::Number(blades_to_add.into()));
                                        effect_data = Some(serde_json::Value::Object(data));
                                    }
                                }
                            }
                        } else {
                            // No activating card, add to first matching card
                            if let Some(&first_card_id) = card_ids.first() {
                                self.game_state.add_blade_modifier(first_card_id, blades_to_add);
                                if is_temporary {
                                    let mut data = serde_json::Map::new();
                                    data.insert("card_id".to_string(), serde_json::Value::Number(first_card_id.into()));
                                    data.insert("amount".to_string(), serde_json::Value::Number(blades_to_add.into()));
                                    effect_data = Some(serde_json::Value::Object(data));
                                }
                            }
                        }
                    } else {
                        // Non-per-unit: add final_count to each matching card
                        let mut cards_data = Vec::new();
                        for card_id in card_ids {
                            self.game_state.add_blade_modifier(card_id, blades_to_add);
                            if is_temporary {
                                let mut data = serde_json::Map::new();
                                data.insert("card_id".to_string(), serde_json::Value::Number(card_id.into()));
                                data.insert("amount".to_string(), serde_json::Value::Number(blades_to_add.into()));
                                cards_data.push(serde_json::Value::Object(data));
                            }
                        }
                        if is_temporary && !cards_data.is_empty() {
                            effect_data = Some(serde_json::Value::Array(cards_data));
                        }
                    }
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
                let mut cards_data = Vec::new();
                for card_id in card_ids {
                    self.game_state.add_heart_modifier(card_id, color, final_count as i32);
                    if is_temporary {
                        let mut data = serde_json::Map::new();
                        data.insert("card_id".to_string(), serde_json::Value::Number(card_id.into()));
                        data.insert("amount".to_string(), serde_json::Value::Number(final_count.into()));
                        cards_data.push(serde_json::Value::Object(data));
                    }
                }
                if is_temporary && !cards_data.is_empty() {
                    effect_data = Some(serde_json::Value::Array(cards_data));
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

        // Create temporary effect after applying the resource gain
        if is_temporary {
            let duration_enum = match duration {
                Some("live_end") => crate::game_state::Duration::LiveEnd,
                Some("this_turn") => crate::game_state::Duration::ThisTurn,
                Some("this_live") => crate::game_state::Duration::ThisLive,
                Some("as_long_as") => crate::game_state::Duration::ThisLive, // Treat as this_live for now
                _ => crate::game_state::Duration::ThisLive,
            };

            let order = self.game_state.effect_creation_counter;
            self.game_state.effect_creation_counter += 1;

            let temp_effect = crate::game_state::TemporaryEffect {
                effect_type: format!("gain_resource_{}", resource),
                duration: duration_enum,
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: player_id.clone(),
                description: format!("Gain {} {} for {}", final_count, resource, target),
                creation_order: order,
                effect_data,
            };
            self.game_state.temporary_effects.push(temp_effect);
        }

        Ok(())
    }

    fn execute_change_state(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let state_change = effect.state_change.as_deref().unwrap_or("");
        let count = effect.count.unwrap_or(1);
        let _max = effect.max.unwrap_or(false);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();
        let group_filter = effect.group.as_ref().and_then(|g| Some(&g.name));
        let cost_limit = effect.cost_limit;
        let optional = effect.optional.unwrap_or(false);
        let _destination = effect.destination.as_deref();
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
                            activating_card: None,
                            ability_index: 0,
                            cost: None,
                            cost_choice: None,
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
                            activating_card: None,
                            ability_index: 0,
                            cost: None,
                            cost_choice: None,
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
                        activating_card: None,
                        ability_index: 0,
                        cost: None,
                        cost_choice: None,
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
                        activating_card: None,
                        ability_index: 0,
                        cost: None,
                        cost_choice: None,
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
                    let current_active = player.energy_zone.active_energy_count;
                    let total_cards = player.energy_zone.cards.len();
                    
                    // Calculate new active count, but don't exceed total cards
                    let new_active = (current_active + count_usize).min(total_cards);
                    player.energy_zone.active_energy_count = new_active;
                }
            }
            "wait" => {
                let count_usize = count as usize;
                
                // Check if targeting member cards or energy cards
                let is_member = card_type_filter == Some("member_card");
                
                if is_member {
                    // Handle member cards on stage
                    let valid_targets: Vec<i16> = player.stage.stage.iter()
                        .filter(|&&id| id != -1)
                        .filter(|&card_id| matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) && matches_cost_limit(*card_id, cost_limit))
                        .copied()
                        .collect();
                    
                    eprintln!("change_state (wait): Found {} valid targets (count limit: {}, cost_limit: {:?}, card_type: {:?}, group: {:?})",
                        valid_targets.len(), count_usize, cost_limit, card_type_filter, group_filter);
                    
                    // For "up to" (まで) effects, having 0 valid targets is valid - effect simply does nothing
                    if valid_targets.is_empty() {
                        eprintln!("change_state (wait): No valid targets found. Effect does nothing (expected for 'up to' effects).");
                        return Ok(());
                    }
                    
                    // If multiple valid targets and count < valid_targets.len(), need user choice
                    if valid_targets.len() > count_usize && count_usize > 0 {
                        let description = format!("Select {} member(s) to set to wait state from {} valid targets", count_usize, valid_targets.len());
                        
                        // Store valid targets temporarily for selection
                        self.looked_at_cards = valid_targets;
                        
                        self.pending_choice = Some(Choice::SelectCard {
                            zone: "stage".to_string(),
                            card_type: card_type_filter.map(|s| s.to_string()),
                            count: count_usize,
                            description,
                            allow_skip: false,
                        });
                        self.game_state.pending_choice = self.pending_choice.clone();
                        // Store execution context to resume after user choice
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        // Return early - execution will continue after user provides choice
                        return Ok(());
                    }
                    
                    // Otherwise, set the first count_usize cards to wait state using orientation modifiers
                    for card_id in valid_targets.iter().take(count_usize) {
                        self.game_state.add_orientation_modifier(*card_id, "wait");
                    }
                } else {
                    // Handle energy cards
                    let valid_targets: Vec<i16> = player.energy_zone.cards.iter()
                        .filter(|&card_id| matches_card_type(*card_id, card_type_filter) && matches_group(*card_id, group_filter) && matches_cost_limit(*card_id, cost_limit))
                        .copied()
                        .collect();
                    
                    eprintln!("change_state (wait) for energy: Found {} valid targets (count limit: {}, cost_limit: {:?}, card_type: {:?}, group: {:?})",
                        valid_targets.len(), count_usize, cost_limit, card_type_filter, group_filter);
                    
                    // For "up to" (まで) effects, having 0 valid targets is valid - effect simply does nothing
                    if valid_targets.is_empty() {
                        eprintln!("change_state (wait) for energy: No valid targets found. Effect does nothing (expected for 'up to' effects).");
                        return Ok(());
                    }
                    
                    // If multiple valid targets and count < valid_targets.len(), need user choice
                    if valid_targets.len() > count_usize && count_usize > 0 {
                        let description = format!("Select {} energy card(s) to set to wait state from {} valid targets", count_usize, valid_targets.len());
                        
                        // Store valid targets temporarily for selection
                        self.looked_at_cards = valid_targets;
                        
                        self.game_state.pending_choice = Some(Choice::SelectCard {
                            zone: "energy_zone".to_string(),
                            card_type: card_type_filter.map(|s| s.to_string()),
                            count: count_usize,
                            description,
                            allow_skip: false,
                        });
                        self.pending_choice = self.game_state.pending_choice.clone();
                        // Store execution context to resume after user choice
                        self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                        // Return early - execution will continue after user provides choice
                        return Ok(());
                    }
                    
                    // Otherwise, deactivate the first count_usize cards
                    let max_to_wait = player.energy_zone.active_energy_count.min(count_usize);
                    player.energy_zone.active_energy_count -= max_to_wait;
                }
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
                creation_order: 0,
                effect_data: None,
            };
            self.game_state.temporary_effects.push(temp_effect);
        }

        // Handle effect_constraint (e.g., minimum_value:0)
        let (min_value, max_value) = if let Some(ref constraint) = effect.effect_constraint {
            // Parse constraint format: "constraint_type:value"
            let mut min_val: Option<i32> = None;
            let mut max_val: Option<i32> = None;
            for part in constraint.split(',') {
                let parts: Vec<&str> = part.split(':').collect();
                if parts.len() == 2 {
                    let constraint_type = parts[0].trim();
                    let constraint_value = parts[1].trim();
                    if let Ok(val) = constraint_value.parse::<i32>() {
                        match constraint_type {
                            "minimum_value" => min_val = Some(val),
                            "maximum_value" => max_val = Some(val),
                            _ => {}
                        }
                    }
                }
            }
            (min_val, max_val)
        } else {
            (None, None)
        };

        // Apply modifiers after releasing borrow
        match operation {
            "add" => {
                for card_id in card_ids {
                    let current_modifier = self.game_state.score_modifiers.get(&card_id).copied().unwrap_or(0);
                    let new_value = current_modifier + (value as i32);
                    // Apply max constraint if present
                    let final_value = if let Some(max) = max_value {
                        new_value.min(max)
                    } else {
                        new_value
                    };
                    self.game_state.score_modifiers.insert(card_id, final_value);
                }
            }
            "remove" | "subtract" => {
                for card_id in card_ids {
                    let current_modifier = self.game_state.score_modifiers.get(&card_id).copied().unwrap_or(0);
                    let new_value = current_modifier - (value as i32);
                    // Apply min constraint if present
                    let final_value = if let Some(min) = min_value {
                        new_value.max(min)
                    } else {
                        new_value
                    };
                    self.game_state.score_modifiers.insert(card_id, final_value);
                }
            }
            "set" => {
                for card_id in card_ids {
                    let new_value = value as i32;
                    // Apply min/max constraints if present
                    let final_value = match (min_value, max_value) {
                        (Some(min), Some(max)) => new_value.clamp(min, max),
                        (Some(min), None) => new_value.max(min),
                        (None, Some(max)) => new_value.min(max),
                        (None, None) => new_value,
                    };
                    self.game_state.score_modifiers.insert(card_id, final_value);
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
                "live_end" => crate::game_state::Duration::LiveEnd,
                "as_long_as" => crate::game_state::Duration::ThisLive,
                "this_turn" => crate::game_state::Duration::ThisTurn,
                _ => crate::game_state::Duration::LiveEnd,
            };

            let temp_effect = crate::game_state::TemporaryEffect {
                effect_type: format!("set_blade_type_{}", blade_type_str),
                duration: duration_enum,
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: target.to_string(),
                description: format!("Set blade type to {}", blade_type_str),
                creation_order: 0,
                effect_data: None,
            };
            self.game_state.temporary_effects.push(temp_effect);
        }

        Ok(())
    }

    fn execute_set_heart_type(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Use heart_type field if available, otherwise fall back to heart_color
        let heart_color = effect.heart_type.as_deref().or(effect.heart_color.as_deref()).unwrap_or("heart00");
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
        let ability_text = effect.ability_gain.as_ref().map(|a| a.clone())
            .or_else(|| Some(effect.text.clone()));

        if let Some(ability_text) = ability_text {
            // Parse the duration
            let parsed_duration = match duration {
                Some("live_end") | Some("ライブ終了時まで") => crate::game_state::Duration::LiveEnd,
                Some("this_turn") | Some("このターン") => crate::game_state::Duration::ThisTurn,
                Some("this_live") | Some("このライブ") => crate::game_state::Duration::ThisLive,
                None | Some("permanent") | Some("永続") => crate::game_state::Duration::Permanent,
                _ => crate::game_state::Duration::Permanent,
            };

            // Track the granted ability as a temporary effect
            let temp_effect = crate::game_state::TemporaryEffect {
                effect_type: "gain_ability".to_string(),
                duration: parsed_duration.clone(),
                created_turn: self.game_state.turn_number,
                created_phase: self.game_state.current_phase.clone(),
                target_player_id: if target == "self" {
                    self.game_state.player1.id.clone()
                } else {
                    self.game_state.player2.id.clone()
                },
                description: format!("Gain ability: {}", ability_text),
                creation_order: self.game_state.temporary_effects.len() as u32,
                effect_data: Some(serde_json::json!({
                    "ability_text": ability_text,
                    "target": target
                })),
            };
            self.game_state.temporary_effects.push(temp_effect);

            // Track as a prohibition effect (existing behavior)
            let granted_ability = format!("gain_ability:{}:{}:{}", target, ability_text, duration.unwrap_or("permanent"));
            self.game_state.prohibition_effects.push(granted_ability);

            // If the ability is a simple score modification, track it as a score modifier
            if ability_text.contains("ライブの合計スコアを＋") {
                // Extract the score value
                if let Some(score_match) = ability_text.find("＋") {
                    let score_part = &ability_text[score_match + 3..]; // Skip "＋"
                    if let Some(end) = score_part.find("する") {
                        let score_str = &score_part[..end];
                        if let Ok(score_value) = score_str.parse::<i32>() {
                            // Track as a score modifier for the live
                            // Use a generic key for live-wide score modifiers
                            self.game_state.score_modifiers.insert(-1, score_value);
                        }
                    }
                }
            }

            println!("Gained ability '{}' with duration '{:?}' to {}", ability_text, parsed_duration, target);
        }

        Ok(())
    }

    fn execute_play_baton_touch(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let position = effect.position.as_ref().and_then(|p| p.get_position());

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

        // Parse heart colors from effect text (e.g., "heart02かheart04かheart05を持つ")
        let required_heart_colors: Vec<crate::card::HeartColor> = {
            let text = &effect.text;
            let mut colors = Vec::new();
            if text.contains("heart02") {
                colors.push(crate::card::HeartColor::Heart02);
            }
            if text.contains("heart04") {
                colors.push(crate::card::HeartColor::Heart04);
            }
            if text.contains("heart05") {
                colors.push(crate::card::HeartColor::Heart05);
            }
            if text.contains("heart01") {
                colors.push(crate::card::HeartColor::Heart01);
            }
            if text.contains("heart03") {
                colors.push(crate::card::HeartColor::Heart03);
            }
            if text.contains("heart06") {
                colors.push(crate::card::HeartColor::Heart06);
            }
            colors
        };

        // Helper function to check if card matches heart color filter
        let matches_heart_colors = |card: &crate::card::Card, required_colors: &[crate::card::HeartColor]| -> bool {
            if required_colors.is_empty() {
                return true; // No heart color filter specified
            }
            // Check if card has at least one of the required heart colors in base_heart
            if let Some(ref base_heart) = card.base_heart {
                for required_color in required_colors {
                    if base_heart.hearts.get(required_color).map_or(false, |&count| count > 0) {
                        return true;
                    }
                }
            }
            false
        };

        match source.as_ref() {
            "hand" => {
                // Mark cards as revealed in hand
                let mut revealed_count = 0;
                let mut cards_to_reveal: Vec<i16> = Vec::new();
                for card_id in &player.hand.cards {
                    if revealed_count >= count {
                        break;
                    }
                    if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                        if matches_card_type(card, card_type_filter) && matches_heart_colors(card, &required_heart_colors) {
                            cards_to_reveal.push(*card_id);
                            println!("Revealed card: {} from hand (matches heart filter)", card.name);
                            revealed_count += 1;
                        }
                    }
                }
                // Add revealed cards after the loop to avoid borrow conflict
                for card_id in cards_to_reveal {
                    self.game_state.add_revealed_card(card_id);
                }
                if revealed_count < count {
                    eprintln!("Warning: Only {} cards revealed, requested {}", revealed_count, count);
                }
            }
            "looked_at" => {
                // Reveal cards from looked_at buffer (after look_at action)
                let mut revealed_count = 0;
                let mut cards_to_reveal: Vec<i16> = Vec::new();
                for card_id in &self.looked_at_cards {
                    if revealed_count >= count {
                        break;
                    }
                    if let Some(card) = self.game_state.card_database.get_card(*card_id as i16) {
                        if matches_card_type(card, card_type_filter) && matches_heart_colors(card, &required_heart_colors) {
                            cards_to_reveal.push(*card_id as i16);
                            println!("Revealed card: {} from looked_at (matches heart filter)", card.name);
                            revealed_count += 1;
                        }
                    }
                }
                // Add revealed cards after the loop to avoid borrow conflict
                for card_id in cards_to_reveal {
                    self.game_state.add_revealed_card(card_id);
                }
                if revealed_count < count {
                    eprintln!("Warning: Only {} cards revealed from looked_at, requested {}", revealed_count, count);
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
        let distinct = effect.distinct.as_deref();

        // Parse heart colors from effect text (e.g., "heart02かheart04かheart05を持つ")
        let required_heart_colors: Vec<crate::card::HeartColor> = {
            let text = &effect.text;
            let mut colors = Vec::new();
            if text.contains("heart02") {
                colors.push(crate::card::HeartColor::Heart02);
            }
            if text.contains("heart04") {
                colors.push(crate::card::HeartColor::Heart04);
            }
            if text.contains("heart05") {
                colors.push(crate::card::HeartColor::Heart05);
            }
            if text.contains("heart01") {
                colors.push(crate::card::HeartColor::Heart01);
            }
            if text.contains("heart03") {
                colors.push(crate::card::HeartColor::Heart03);
            }
            if text.contains("heart06") {
                colors.push(crate::card::HeartColor::Heart06);
            }
            colors
        };

        // Filter looked_at cards by heart colors if specified
        if source == "looked_at" && !required_heart_colors.is_empty() {
            let mut filtered_cards: Vec<i16> = Vec::new();
            for card_id in &self.looked_at_cards {
                if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                    // Check card type filter
                    let matches_type = match card_type_filter {
                        Some("member_card") => card.is_member(),
                        Some("live_card") => card.is_live(),
                        Some("energy_card") => card.is_energy(),
                        None => true,
                        _ => true,
                    };
                    
                    // Check heart color filter
                    let matches_heart = if let Some(ref base_heart) = card.base_heart {
                        required_heart_colors.iter().any(|color| {
                            base_heart.hearts.get(color).map_or(false, |&c| c > 0)
                        })
                    } else {
                        false
                    };
                    
                    if matches_type && matches_heart {
                        filtered_cards.push(*card_id);
                    }
                }
            }
            
            // Update looked_at_cards to only include filtered cards
            self.looked_at_cards = filtered_cards;
            eprintln!("Filtered looked_at cards by heart colors: {} cards remain", self.looked_at_cards.len());
        }

        // Build more descriptive message
        let card_type_desc = if let Some(ct) = card_type_filter {
            format!("{} ", ct)
        } else {
            "".to_string()
        };
        
        let target_desc = if target == "self" { String::new() } else { format!("for {} ", target) };
        let distinct_desc = if distinct == Some("card_name") {
            "with distinct names "
        } else {
            ""
        };
        let description = if optional {
            format!("Select {} {}{}card(s) from {} {}(or skip)", count, distinct_desc, card_type_desc, source, target_desc)
        } else {
            format!("Select {} {}{}card(s) from {} {}", count, distinct_desc, card_type_desc, source, target_desc)
        };

        // If distinct selection is required, we need to filter available cards
        // For now, we'll add a note to the description and let the UI handle the filtering
        // A full implementation would pre-filter the available cards based on distinct names
        if distinct == Some("card_name") {
            eprintln!("Distinct card name selection required - UI should enforce this");
        }

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
        match source.as_ref() {
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
    pub fn resolve_ability(&mut self, ability: &Ability, activating_card: Option<i16>, ability_index: usize) -> Result<(), String> {
        eprintln!("DEBUG: RESOLVING ABILITY - triggers: {:?}, full_text: {}, activating_card: {:?}", 
            ability.triggers, ability.full_text, activating_card);
        
        // Check use_limit before executing ability
        if let Some(use_limit) = ability.use_limit {
            if let Some(card_id) = activating_card {
                let ability_key = format!("{}_{}_{}", card_id, ability_index, self.game_state.turn_number);
                if self.game_state.turn_limited_abilities_used.contains(&ability_key) {
                    eprintln!("DEBUG: Ability already used this turn (use_limit: {})", use_limit);
                    return Err(format!("Ability has already been used this turn (use_limit: {})", use_limit));
                }
                // Mark ability as used
                self.game_state.turn_limited_abilities_used.insert(ability_key);
                eprintln!("DEBUG: Marked ability as used this turn (use_limit: {})", use_limit);
            }
        }
        
        // Set current ability for optional cost handling
        self.current_ability = Some(ability.clone());
        
        // Set activating card in game state for self-cost handling
        self.game_state.activating_card = activating_card;
        
        eprintln!("DEBUG: About to pay cost - cost: {:?}", ability.cost);
        // First, pay the cost if there is one
        if let Some(ref cost) = ability.cost {
            self.pay_cost(cost)?;
            eprintln!("DEBUG: Cost payment completed");
        } else {
            eprintln!("DEBUG: No cost to pay");
        }

        // Check if there's a pending choice from cost payment (e.g., optional cost)
        // If so, don't execute the effect yet - wait for user choice
        if self.pending_choice.is_some() {
            eprintln!("DEBUG: Pending choice from cost payment, pausing ability execution");
            // Store current_ability in game_state for resumption after user pays cost
            self.game_state.pending_current_ability = self.current_ability.clone();
            // Don't clear activating_card yet - we need it when resuming
            // Copy pending_choice to game_state so it's accessible to the caller
            self.game_state.pending_choice = self.pending_choice.clone();
            return Ok(());
        }

        eprintln!("DEBUG: About to execute effect - effect: {:?}", ability.effect);
        // Then execute the effect
        if let Some(ref effect) = ability.effect {
            self.execute_effect(effect)?;
            eprintln!("DEBUG: Effect execution completed successfully");
        } else {
            eprintln!("DEBUG: No effect to execute");
        }

        // Clear activating card after successful execution
        self.game_state.activating_card = None;
        self.current_ability = None;
        eprintln!("DEBUG: Ability resolution completed successfully");
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

        // Mark that a position change occurred this turn (for keyword validation)
        self.game_state.position_change_occurred_this_turn = true;

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
                activating_card: None,
                ability_index: 0,
                cost: None,
                cost_choice: None,
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

    fn execute_position_change_with_destination(&mut self, effect: &AbilityEffect, destination: &str) -> Result<(), String> {
        // Execute position change with a specific destination (user-selected)
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

        // Map destination string to stage index
        let dest_index = match destination {
            "left_side" | "左" => 0,
            "center" | "中央" => 1,
            "right_side" | "右" => 2,
            _ => {
                eprintln!("Unknown destination: {}", destination);
                return Err(format!("Unknown destination: {}", destination));
            }
        };

        // Find a card to move (prioritize center, then left, then right)
        let source_indices = [1, 0, 2]; // center, left, right
        let mut moved = 0;

        for &source_idx in source_indices.iter() {
            if moved >= count {
                break;
            }

            let card_id = player.stage.stage[source_idx];
            if card_id != -1 && matches_card_type(card_id, card_type_filter) && matches_group(card_id, group_filter) {
                // Check if destination is empty
                if player.stage.stage[dest_index] == -1 {
                    // Move card to destination
                    player.stage.stage[dest_index] = card_id;
                    player.stage.stage[source_idx] = -1;
                    moved += 1;
                    eprintln!("Moved card {} from position {} to position {}", card_id, source_idx, dest_index);
                } else {
                    // Destination is occupied, swap
                    let dest_card = player.stage.stage[dest_index];
                    player.stage.stage[dest_index] = card_id;
                    player.stage.stage[source_idx] = dest_card;
                    moved += 1;
                    eprintln!("Swapped card {} (position {}) with card {} (position {})", card_id, source_idx, dest_card, dest_index);
                }
            }
        }

        eprintln!("Position change with destination executed: moved {} cards to {}", moved, destination);
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

        match source.as_ref() {
            "hand" => {
                match destination.as_ref() {
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
        let choice_type = effect.choice_type.as_deref();

        // Check if options are AbilityEffect objects (choice effects) or strings (heart options)
        if let Some(ref options) = effect.options {
            // Try to deserialize as Vec<AbilityEffect> first
            if let Ok(effect_options) = serde_json::from_value::<Vec<AbilityEffect>>(serde_json::json!(options)) {
                // These are full effect objects - handle as choice effect
                let description = if effect.text.is_empty() {
                    match choice_type {
                        Some("emma_punch") => "Choose your response to Emma Punch".to_string(),
                        _ => "Make a choice".to_string()
                    }
                } else {
                    effect.text.clone()
                };

                let options_display = effect_options.iter()
                    .enumerate()
                    .map(|(i, opt)| format!("{}. {}", i + 1, opt.text))
                    .collect::<Vec<_>>()
                    .join("\n");

                let full_description = format!("{}\n{}", description, options_display);
                let options_json = serde_json::to_string(&effect_options).unwrap_or_default();

                self.pending_choice = Some(Choice::SelectTarget {
                    target: options_json.clone(),
                    description: full_description,
                });

                self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                    card_no: "choice".to_string(),
                    player_id: "self".to_string(),
                    action_index: 0,
                    effect: effect.clone(),
                    conditional_choice: Some(options_json),
                    activating_card: None,
                    ability_index: 0,
                    cost: None,
                    cost_choice: None,
                });
                return Ok(());
            } else if let Ok(string_options) = serde_json::from_value::<Vec<String>>(serde_json::json!(options)) {
                // These are string options (e.g., heart choices) - use choice_options
                let description = if effect.text.is_empty() {
                    match choice_type {
                        Some("emma_punch") => "Choose your response to Emma Punch".to_string(),
                        _ => "Make a choice".to_string()
                    }
                } else {
                    effect.text.clone()
                };

                let options_display = string_options.iter()
                    .enumerate()
                    .map(|(i, opt)| format!("{}. {}", i + 1, opt))
                    .collect::<Vec<_>>()
                    .join("\n");

                let full_description = format!("{}\n{}", description, options_display);
                let options_json = serde_json::to_string(&string_options).unwrap_or_default();

                self.pending_choice = Some(Choice::SelectTarget {
                    target: options_json.clone(),
                    description: full_description,
                });

                self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                    card_no: "choice_string".to_string(),
                    player_id: "self".to_string(),
                    action_index: 0,
                    effect: effect.clone(),
                    conditional_choice: Some(options_json),
                    activating_card: None,
                    ability_index: 0,
                    cost: None,
                    cost_choice: None,
                });
                return Ok(());
            }
        }

        // Fallback to choice_options for backward compatibility
        if let Some(ref options) = effect.choice_options {
            let description = if effect.text.is_empty() {
                match choice_type {
                    Some("emma_punch") => "Choose your response to Emma Punch".to_string(),
                    _ => "Make a choice".to_string()
                }
            } else {
                effect.text.clone()
            };

            let options_display = options.iter()
                .enumerate()
                .map(|(i, opt)| format!("{}. {}", i + 1, opt))
                .collect::<Vec<_>>()
                .join("\n");

            let full_description = format!("{}\n{}", description, options_display);
            let options_json = serde_json::to_string(options).unwrap_or_default();

            self.pending_choice = Some(Choice::SelectTarget {
                target: options_json.clone(),
                description: full_description,
            });

            self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                card_no: "choice_string".to_string(),
                player_id: "self".to_string(),
                action_index: 0,
                effect: effect.clone(),
                conditional_choice: Some(options_json),
                activating_card: None,
                ability_index: 0,
                cost: None,
                cost_choice: None,
            });
            return Ok(());
        }

        Ok(())
    }

    fn execute_pay_energy(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let amount = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");

        let active_id = self.game_state.active_player().id.clone();
        let player = match target {
            "self" => self.game_state.active_player_mut(),
            "opponent" => {
                if active_id == self.game_state.player1.id {
                    &mut self.game_state.player2
                } else {
                    &mut self.game_state.player1
                }
            }
            _ => self.game_state.active_player_mut(),
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

    /// Helper method for discard - requires user choice when hand has cards
    fn execute_discard(&mut self, player_id: &str, target_count: usize) -> Result<(), String> {
        let player = if player_id == self.game_state.player1.id {
            &mut self.game_state.player1
        } else if player_id == self.game_state.player2.id {
            &mut self.game_state.player2
        } else {
            return Err(format!("Unknown player: {}", player_id));
        };

        let current_count = player.hand.len();
        eprintln!("DEBUG: Discard check - current hand size: {}, target: {}", current_count, target_count);
        
        if current_count > target_count {
            let cards_to_discard = current_count - target_count;
            eprintln!("DEBUG: Need to discard {} cards from hand", cards_to_discard);
            
            // If hand has cards, require user choice instead of auto-discard
            if current_count > 0 {
                eprintln!("DEBUG: Creating discard choice prompt for {} cards", cards_to_discard);
                
                // Create pending choice for user to select cards to discard
                self.pending_choice = Some(crate::ability_resolver::Choice::SelectCard {
                    zone: "hand".to_string(),
                    card_type: None,
                    count: cards_to_discard,
                    description: format!("Select {} card(s) to discard from hand ({} available)", cards_to_discard, current_count),
                    allow_skip: false,
                });
                self.game_state.pending_choice = self.pending_choice.clone();
                
                return Err("Pending choice required: select cards to discard from hand".to_string());
            } else {
                eprintln!("DEBUG: Hand is empty, nothing to discard");
            }
        } else {
            eprintln!("DEBUG: Hand size {} is already at or below target {}", current_count, target_count);
        }
        Ok(())
    }

    fn execute_discard_until_count(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let target_count = effect.target_count.unwrap_or(effect.count.unwrap_or(0)) as usize;
        let target = effect.target.as_deref().unwrap_or("self");

        // Handle "both" target (Q229)
        if target == "both" {
            // Apply discard to both players
            self.execute_discard(&self.game_state.player1.id.clone(), target_count)?;
            self.execute_discard(&self.game_state.player2.id.clone(), target_count)?;
            return Ok(());
        }

        let active_id = self.game_state.active_player().id.clone();
        let is_opponent = target == "opponent";
        let player_id = if is_opponent {
            if active_id == self.game_state.player1.id {
                self.game_state.player2.id.clone()
            } else {
                self.game_state.player1.id.clone()
            }
        } else {
            active_id
        };

        self.execute_discard(&player_id, target_count)
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
        let lose_blade_hearts = effect.lose_blade_hearts.unwrap_or(false);

        let _player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Track re-yell as a temporary effect
        for _ in 0..count {
            self.game_state.prohibition_effects.push(format!("re_yell:{}", target));
        }

        // If lose_blade_hearts is true, remove blade hearts from the player
        if lose_blade_hearts {
            eprintln!("re_yell: losing blade hearts for {}", target);
            // Implement blade heart removal logic
            // Get the target player's ID
            let target_player_id = match target {
                "self" => self.game_state.player1.id.clone(),
                "opponent" => self.game_state.player2.id.clone(),
                _ => self.game_state.player1.id.clone(),
            };
            
            // Remove all blade modifiers from the target player's cards on stage
            // This effectively removes the blade hearts they would have gained
            let player = if target_player_id == self.game_state.player1.id {
                &self.game_state.player1
            } else {
                &self.game_state.player2
            };
            
            // Collect card IDs that need blade modifiers cleared
            let mut cards_to_clear: Vec<i16> = Vec::new();
            for &card_id in &player.stage.stage {
                if card_id != crate::constants::EMPTY_SLOT {
                    cards_to_clear.push(card_id);
                }
            }
            
            // Clear blade modifiers for those cards
            for card_id in cards_to_clear {
                self.game_state.blade_modifiers.remove(&card_id);
                eprintln!("Cleared blade modifier for card {}", card_id);
            }
            
            // Also reset the cheer blade heart count for this player
            if target_player_id == self.game_state.player1.id {
                self.game_state.player1_cheer_blade_heart_count = 0;
            } else {
                self.game_state.player2_cheer_blade_heart_count = 0;
            }
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
                activating_card: None,
                ability_index: 0,
                cost: None,
                cost_choice: None,
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

        let _player = match target {
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

    fn execute_specify_heart_color(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Allow player to choose a heart color
        let choice = effect.choice.unwrap_or(false);
        let target = effect.target.as_deref().unwrap_or("self");

        eprintln!("specify_heart_color: choice={}, target={}", choice, target);

        if choice {
            // Present choice to player to select heart color
            self.pending_choice = Some(Choice::SelectTarget {
                target: "heart_color".to_string(),
                description: "Choose a heart color".to_string(),
            });
            eprintln!("Pending choice: heart color selection");
        }

        Ok(())
    }

    fn execute_set_card_identity_all_regions(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Set card identity in all regions (e.g., treat card as belonging to multiple groups)
        let target = effect.target.as_deref().unwrap_or("self");
        let identities = effect.identities.as_ref();

        eprintln!("set_card_identity_all_regions: target={}, identities={:?}", target, identities);

        // Get the activating card if available
        let card_id = self.activating_card_id.or_else(|| {
            // If no activating card, try to get from game state
            self.game_state.activating_card
        });

        if let Some(card_id) = card_id {
            if let Some(identities) = identities {
                // Add group identities to the card
                // For now, just log - full implementation would modify card identity in game state
                eprintln!("Would add identities {:?} to card {}", identities, card_id);
            }
        }

        Ok(())
    }

    fn execute_modify_required_hearts_success(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Modify required hearts for success (e.g., for live cards)
        let operation = effect.operation.as_deref().unwrap_or("increase");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type_filter = effect.card_type.as_deref();

        eprintln!("modify_required_hearts_success: operation={}, value={}, target={}, card_type={:?}",
            operation, value, target, card_type_filter);

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // Target specific cards based on card_type_filter
        let card_ids: Vec<i16> = if let Some("live_card") = card_type_filter {
            player.live_card_zone.cards.iter().copied().collect()
        } else {
            vec![]
        };

        let delta = match operation {
            "increase" => value as i32,
            "decrease" => -(value as i32),
            _ => return Err(format!("Unknown operation: {}", operation)),
        };

        for card_id in card_ids {
            // Use existing add_heart_modifier method
            // For now, just log - full implementation would modify required hearts
            eprintln!("Would modify required hearts for card {} by {}", card_id, delta);
        }

        Ok(())
    }

    fn execute_set_cost_to_use(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Set the cost to use a card (e.g., modify heart cost for activation)
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        eprintln!("set_cost_to_use: value={}, target={}", value, target);

        let card_id = self.activating_card_id.or_else(|| {
            self.game_state.activating_card
        });

        if let Some(card_id) = card_id {
            self.game_state.set_cost_modifier(card_id, value as i32);
            eprintln!("Set cost to use for card {} to {}", card_id, value);
        }

        Ok(())
    }

    fn execute_all_blade_timing(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Handle ALL blade timing effect (e.g., treat ALL blade as any heart color during required heart check)
        let timing = effect.timing.as_deref().unwrap_or("check_required_hearts");
        let treat_as = effect.treat_as.as_deref().unwrap_or("any_heart_color");

        eprintln!("all_blade_timing: timing={}, treat_as={}", timing, treat_as);

        // For now, just log - full implementation would register timing-specific effect
        eprintln!("Would register timing effect: {} -> {}", timing, treat_as);

        Ok(())
    }

    fn execute_shuffle(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Shuffle a zone (deck or energy deck)
        let target = effect.target.as_deref().unwrap_or("deck");
        let count = effect.count.unwrap_or(0);

        eprintln!("shuffle: target={}, count={}", target, count);

        let player = &mut self.game_state.player1;

        match target {
            "deck" => {
                // Shuffle the main deck
                use rand::seq::SliceRandom;
                player.main_deck.cards.shuffle(&mut rand::thread_rng());
                eprintln!("Shuffled main deck");
            }
            "energy_deck" => {
                // Shuffle the energy deck
                use rand::seq::SliceRandom;
                player.energy_deck.cards.shuffle(&mut rand::thread_rng());
                eprintln!("Shuffled energy deck");
            }
            _ => {
                eprintln!("Unknown shuffle target: {}", target);
            }
        }

        Ok(())
    }

    fn execute_reveal_per_group(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Reveal cards per group (e.g., 1 card per group name)
        let source = effect.source.as_deref().unwrap_or("hand");
        let count = effect.count.unwrap_or(1);
        let per_unit = effect.per_unit.unwrap_or(false);

        eprintln!("reveal_per_group: source={}, count={}, per_unit={}", source, count, per_unit);

        let player = &mut self.game_state.player1;

        // Get cards from source
        let card_ids: Vec<i16> = match source {
            "hand" => player.hand.cards.iter().copied().collect(),
            "deck" => player.main_deck.cards.iter().copied().collect(),
            "discard" => player.waitroom.cards.iter().copied().collect(),
            _ => vec![],
        };

        // For now, reveal all cards from source
        // A full implementation would:
        // 1. Identify unique groups among the cards
        // 2. Reveal count cards per group
        eprintln!("Revealing {} cards from {}", card_ids.len(), source);

        Ok(())
    }

    fn execute_conditional_on_result(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Execute action based on result of previous action
        let primary_action = effect.primary_effect.as_ref();
        let result_condition = effect.result_condition.as_ref();
        let followup_action = effect.followup_action.as_ref();

        eprintln!("conditional_on_result: primary_action={:?}, result_condition={:?}, followup_action={:?}",
            primary_action, result_condition, followup_action);

        // Execute primary action first
        if let Some(ref primary) = primary_action {
            self.execute_effect(primary)?;
        }

        // Check result condition
        // For now, always execute followup action
        if let Some(ref followup) = followup_action {
            self.execute_effect(followup)?;
        }

        Ok(())
    }

    fn execute_conditional_on_optional(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // Execute action based on whether optional action was taken
        let optional_action = effect.optional_action.as_ref();
        let conditional_action = effect.conditional_action.as_ref();

        eprintln!("conditional_on_optional: optional_action={:?}, conditional_action={:?}",
            optional_action, conditional_action);

        // For now, execute optional action then conditional action
        // A full implementation would:
        // 1. Present optional action to player
        // 2. If player chooses to do it, execute conditional action
        if let Some(ref optional) = optional_action {
            self.execute_effect(optional)?;
        }

        if let Some(ref conditional) = conditional_action {
            self.execute_effect(conditional)?;
        }

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

    pub fn validate_cost(&self, cost: &AbilityCost) -> Result<(), String> {
        // Validate that a cost can be paid without actually paying it
        // This is used for sequential cost validation to prevent partial payment
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs {
                        self.validate_cost(sub_cost)?;
                    }
                    Ok(())
                } else {
                    Err("Sequential cost has no sub-costs".to_string())
                }
            }
            Some("choice_condition") => {
                // Choice condition costs are valid if at least one option can be paid
                if let Some(ref options) = cost.options {
                    for option in options {
                        if self.validate_cost(option).is_ok() {
                            return Ok(());
                        }
                    }
                    Err("No valid cost option available".to_string())
                } else {
                    Err("Choice condition cost has no options".to_string())
                }
            }
            Some("move_cards") => {
                // Validate that the required cards are available in the source zone
                let source = cost.source.as_deref().unwrap_or("");
                let count = cost.count.unwrap_or(1) as usize;
                let player = self.game_state.active_player();
                
                let available = match source {
                    "hand" => player.hand.cards.len(),
                    "stage" => player.stage.stage.iter().filter(|&&id| id != -1).count(),
                    "waitroom" => player.waitroom.cards.len(),
                    "energy_zone" => player.energy_zone.cards.len(),
                    _ => return Ok(()), // Other sources not validated
                };
                
                if available < count {
                    return Err(format!("Not enough cards in {}: need {}, have {}", source, count, available));
                }
                Ok(())
            }
            _ => Ok(()), // Other cost types not validated for now
        }
    }

    pub fn pay_cost(&mut self, cost: &AbilityCost) -> Result<(), String> {
        eprintln!("PAY_COST: cost_type={:?}, source={:?}, destination={:?}, card_type={:?}", cost.cost_type, cost.source, cost.destination, cost.card_type);
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                // Sequential costs require paying multiple costs in sequence (～し、～)
                if let Some(ref costs) = cost.costs {
                    eprintln!("Sequential cost with {} steps", costs.len());
                    // First, validate that all sub-costs can be paid
                    // This prevents partial payment if a later cost fails
                    for sub_cost in costs {
                        if let Err(e) = self.validate_cost(sub_cost) {
                            return Err(format!("Cannot pay sequential cost: {}", e));
                        }
                    }
                    // All costs can be paid, now actually pay them
                    for sub_cost in costs {
                        self.pay_cost(sub_cost)?;
                    }
                    Ok(())
                } else {
                    Err("Sequential cost has no sub-costs".to_string())
                }
            }
            Some("choice_condition") => {
                // Choice condition costs require the player to choose between multiple cost options
                // This is a mandatory choice - the player must pick one option, but gets to choose which
                if let Some(ref options) = cost.options {
                    let option_texts: Vec<String> = options.iter()
                        .map(|o| o.text.clone())
                        .collect();

                    // Present choice to the player
                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "choice_condition".to_string(),
                        description: format!("Choose cost option: {}", option_texts.join(" OR ")),
                    });

                    // Store the cost options for resuming after choice
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "choice_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: AbilityEffect {
                            text: cost.text.clone(),
                            action: "choice_condition".to_string(),
                            ..Default::default()
                        },
                        conditional_choice: None,
                        activating_card: None,
                        ability_index: 0,
                        cost: Some(cost.clone()),
                        cost_choice: None,
                    });

                    return Ok(());
                } else {
                    return Err("Choice condition cost has no options".to_string());
                }
            }
            Some("move_cards") => {
                // Check if this is an activation ability - if so, cost is mandatory regardless of optional flag
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == "起動");

                // Check if this is an optional cost (only for auto abilities, not activation)
                if cost.optional == Some(true) && !is_activation {
                    // Present choice to pay or skip optional cost
                    let source = cost.source.as_deref().unwrap_or("");
                    let count = cost.count.unwrap_or(1);

                    self.pending_choice = Some(Choice::SelectCard {
                        zone: source.to_string(),
                        card_type: cost.card_type.clone(),
                        count: count as usize,
                        description: format!("Select card(s) to pay optional cost (or skip): {}", cost.text),
                        allow_skip: true,
                    });

                    // Store effect for resuming after choice
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: AbilityEffect {
                            text: cost.text.clone(),
                            action: cost.cost_type.clone().unwrap_or_default(),
                            source: cost.source.clone(),
                            destination: cost.destination.clone(),
                            count: cost.count,
                            card_type: cost.card_type.clone(),
                            target: cost.target.clone(),
                            effect_type: None,
                            ..Default::default()
                        },
                        conditional_choice: None,
                        activating_card: None,
                        ability_index: 0,
                        cost: Some(cost.clone()),
                        cost_choice: None,
                    });

                    return Ok(());
                }

                // Validate that source zone has enough cards for cost payment (Q234)
                if let Some(ref source) = cost.source {
                    let count = cost.count.unwrap_or(1);
                    let target = cost.target.as_deref().unwrap_or("self");
                    let cost_limit = cost.cost_limit;
                    let card_type_filter = cost.card_type.as_deref();

                    let player = match target {
                        "self" => &self.game_state.player1,
                        "opponent" => &self.game_state.player2,
                        _ => &self.game_state.player1,
                    };

                    // Helper function to check if card matches cost limit
                    let matches_cost_limit = |card_id: i16, limit: Option<u32>| -> bool {
                        if let Some(limit_val) = limit {
                            if let Some(card) = self.game_state.card_database.get_card(card_id) {
                                card.cost.map_or(false, |c| c <= limit_val)
                            } else {
                                false
                            }
                        } else {
                            true
                        }
                    };

                    // Helper function to check if card matches type filter
                    let matches_card_type = |card_id: i16, filter: Option<&str>| -> bool {
                        match filter {
                            Some("live_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_live()).unwrap_or(false),
                            Some("member_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_member()).unwrap_or(false),
                            Some("energy_card") => self.game_state.card_database.get_card(card_id).map(|c| c.is_energy()).unwrap_or(false),
                            None => true,
                            _ => true,
                        }
                    };

                    // Helper function to check if card matches character names (for combined name cost payment)
                    let matches_character_names = |card_id: i16, names: Option<&Vec<String>>| -> bool {
                        if let Some(ref required_names) = names {
                            if let Some(card) = self.game_state.card_database.get_card(card_id) {
                                // Check if card name matches any of the required names
                                // For combined name cards (e.g., "上原歩夢&澁谷かのん&日野下花帆"), the card has all names
                                // but for cost payment, each card counts as 1 card, not as multiple names
                                required_names.iter().any(|name| card.name.contains(name) || card.name == *name)
                            } else {
                                false
                            }
                        } else {
                            true // No name filter, all cards match
                        }
                    };

                    // Count cards that match type filter, cost limit, and character names
                    let character_filter = cost.characters.as_ref();
                    let matching_count = match source.as_str() {
                        "deck" | "deck_top" => player.main_deck.cards.iter()
                            .filter(|&&card_id| matches_card_type(card_id, card_type_filter) && matches_cost_limit(card_id, cost_limit) && matches_character_names(card_id, character_filter))
                            .count(),
                        "hand" => player.hand.cards.iter()
                            .filter(|&&card_id| matches_card_type(card_id, card_type_filter) && matches_cost_limit(card_id, cost_limit) && matches_character_names(card_id, character_filter))
                            .count(),
                        "discard" => player.waitroom.cards.iter()
                            .filter(|&&card_id| matches_card_type(card_id, card_type_filter) && matches_cost_limit(card_id, cost_limit) && matches_character_names(card_id, character_filter))
                            .count(),
                        "energy_zone" => player.energy_zone.cards.iter()
                            .filter(|&&card_id| matches_card_type(card_id, card_type_filter) && matches_cost_limit(card_id, cost_limit) && matches_character_names(card_id, character_filter))
                            .count(),
                        _ => usize::MAX, // Unknown source, don't validate
                    };

                    if matching_count < count as usize {
                        return Err(format!("Cannot pay cost: {} has only {} cards matching cost limit {}, need {}", 
                            source, matching_count, 
                            cost_limit.map(|l| l.to_string()).unwrap_or("none".to_string()), 
                            count));
                    }
                }

                // Execute the move action as a cost
                let effect = AbilityEffect {
                    text: cost.text.clone(),
                    action: cost.cost_type.clone().unwrap_or_default(),
                    source: cost.source.clone(),
                    destination: cost.destination.clone(),
                    count: cost.count,
                    card_type: cost.card_type.clone(),
                    target: cost.target.clone(),
                    effect_type: None,
                    ..Default::default()
                };
                self.execute_move_cards(&effect)
            }
            Some("change_state") => {
                // Execute state change as a cost (e.g., put member to wait state)
                let state_change = cost.state_change.as_deref().unwrap_or("");
                eprintln!("PAY_COST: Executing state change: {}", state_change);

                // Check if this is an activation ability - if so, cost is mandatory regardless of optional flag
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == "起動");

                // Check if this is an optional cost (only for auto abilities, not activation)
                if cost.optional == Some(true) && !is_activation {
                    // Present choice to pay or skip optional cost
                    let cost_description = if state_change == "wait" {
                        "Put this member to wait state"
                    } else {
                        "Pay cost"
                    };

                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay optional cost: {}? (pay or skip)", cost_description),
                    });

                    // Store the actual ability effect (from current_ability) for resuming after choice
                    let actual_effect = if let Some(ref ability) = self.current_ability {
                        eprintln!("Retrieving effect from current_ability: {:?}", ability.effect);
                        ability.effect.clone()
                    } else {
                        eprintln!("No current_ability found!");
                        None
                    };

                    // Store effect for resuming after choice
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: actual_effect.unwrap_or_default(),
                        conditional_choice: None,
                        activating_card: None,
                        ability_index: 0,
                        cost: Some(cost.clone()),
                        cost_choice: None,
                    });

                    eprintln!("Stored pending_ability with effect: {:?}", self.game_state.pending_ability);

                    return Ok(());
                }
                
                if state_change == "wait" {
                    // Set member to wait state (inactive but still on stage)
                    let target = cost.target.as_deref().unwrap_or("self");
                    let player = if target == "self" {
                        &self.game_state.player1
                    } else {
                        &self.game_state.player2
                    };
                    
                    // Collect card IDs first to avoid borrow issues
                    let card_ids: Vec<i16> = player.stage.stage.iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect();
                    
                    // Set orientation to wait for each card
                    for card_id in card_ids {
                        self.game_state.add_orientation_modifier(card_id, "wait");
                        eprintln!("PAY_COST: Set card {} to wait state (still on stage)", card_id);
                    }
                }
                Ok(())
            }
            Some("pay_energy") => {
                // Check if this is an activation ability - if so, cost is mandatory regardless of optional flag
                let is_activation = self.current_ability.as_ref()
                    .and_then(|a| a.triggers.as_ref())
                    .map_or(false, |t| t == "起動");

                // Check if this is an optional cost (only for auto abilities, not activation)
                if cost.optional == Some(true) && !is_activation {
                    // Present choice to pay or skip optional energy cost
                    let energy = cost.energy.unwrap_or(0);

                    self.pending_choice = Some(Choice::SelectTarget {
                        target: "pay_optional_cost:skip_optional_cost".to_string(),
                        description: format!("Pay {} energy (or skip)?", energy),
                    });

                    // Store the actual ability effect (from current_ability) for resuming after choice
                    let actual_effect = if let Some(ref ability) = self.current_ability {
                        eprintln!("Retrieving effect from current_ability for pay_energy: {:?}", ability.effect);
                        ability.effect.clone()
                    } else {
                        eprintln!("No current_ability found for pay_energy!");
                        None
                    };

                    // Store effect for resuming after choice
                    self.game_state.pending_ability = Some(crate::game_state::PendingAbilityExecution {
                        card_no: "optional_cost".to_string(),
                        player_id: "self".to_string(),
                        action_index: 0,
                        effect: actual_effect.unwrap_or_default(),
                        conditional_choice: None,
                        activating_card: None,
                        ability_index: 0,
                        cost: Some(cost.clone()),
                        cost_choice: None,
                    });

                    eprintln!("Stored pending_ability with effect: {:?}", self.game_state.pending_ability);

                    return Ok(());
                }

                // Pay energy cost
                let energy = cost.energy.unwrap_or(0);
                let target = cost.target.as_deref().unwrap_or("self");

                // Q25: Skip energy cost if baton touch resulted in 0 cost (equal or lower cost)
                if self.game_state.baton_touch_zero_cost && energy > 0 {
                    eprintln!("Skipping pay_energy cost of {} due to baton touch zero cost", energy);
                    return Ok(());
                }

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
            target_count: None,
            card_type: None,
            target: None,
            duration: None,
            parenthetical: None,
            look_action: None,
            select_action: None,
            name_constraint: None,
            name_constraint_source: None,
            actions: None,
            resource: None,
            resource_icon_count: None,
            position: None,
            choice: None,
            timing: None,
            treat_as: None,
            identities: None,
            state_change: None,
            optional: None,
            destination_choice: None,
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
            result_condition: None,
            followup_action: None,
            optional_action: None,
            conditional_action: None,
            operation: None,
            value: None,
            choice_condition: None,
            choice_modifier: None,
            aggregate: None,
            comparison_type: None,
            choice_options: None,
            options: None,
            group: None,
            per_unit_count: None,
            per_unit_type: None,
            per_unit_reference: None,
            group_matching: None,
            repeat_limit: None,
            repeat_optional: None,
            is_further: None,
            restriction_type: None,
            restricted_destination: None,
            cost_result_reference: None,
            dynamic_count: None,
            placement_order: None,
            cost_limit: None,
            any_number: None,
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
            self_cost: None,
            exclude_self: None,
            effect_type: None,
            action_by: None,
            opponent_action: None,
            lose_blade_hearts: None,
            conditional: None,
            choice_type: None,
            heart_type: None,
            values: None,
        }
    }
}
