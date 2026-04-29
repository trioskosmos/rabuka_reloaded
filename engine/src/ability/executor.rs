use crate::card::{Ability, AbilityCost, AbilityEffect, Condition};
use crate::game_state::GameState;
use crate::player::Player;
use crate::zones::MemberArea;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CostCalculation {
    pub payable: bool,
    pub reason: Option<String>,
    pub cost_description: String,
}

#[derive(Debug, Clone)]
pub struct AbilityValidation {
    pub can_execute: bool,
    pub conditions_met: bool,
    pub cost_payable: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum Choice {
    SelectCard {
        zone: String,
        card_type: Option<String>,
        count: usize,
        description: String,
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
}

#[derive(Debug, Clone)]
pub struct AbilityExecutor {
    pending_choice: Option<Choice>,
    looked_at_cards: Vec<u32>,
    selected_cards: Vec<u32>,
    pending_optional_cost: Option<AbilityCost>,
}

impl AbilityExecutor {
    pub fn new() -> Self {
        Self {
            pending_choice: None,
            looked_at_cards: Vec::new(),
            selected_cards: Vec::new(),
            pending_optional_cost: None,
        }
    }

    /// Calculate if a cost can be paid and return detailed information
    pub fn calculate_cost(
        &self,
        cost: &AbilityCost,
        player: &Player,
        game_state: &GameState,
    ) -> CostCalculation {
        match cost.cost_type.as_deref() {
            Some("move_cards") => self.calculate_move_cards_cost(cost, player, game_state),
            Some("pay_energy") => self.calculate_pay_energy_cost(cost, player, game_state),
            Some("change_state") => self.calculate_change_state_cost(cost, player, game_state),
            Some("choice_condition") => self.calculate_choice_condition_cost(cost, player, game_state),
            Some("energy_condition") => self.calculate_energy_condition_cost(cost, player, game_state),
            Some("reveal") => self.calculate_reveal_cost(cost, player, game_state),
            _ => CostCalculation {
                payable: false,
                reason: Some(format!("Unknown cost type: {:?}", cost.cost_type)),
                cost_description: cost.text.clone(),
            },
        }
    }

    fn calculate_move_cards_cost(
        &self,
        cost: &AbilityCost,
        player: &Player,
        game_state: &GameState,
    ) -> CostCalculation {
        // Check if source zone has the required card
        let source = cost.source.as_deref().unwrap_or("");
        let card_type = cost.card_type.as_deref().unwrap_or("");
        let count_needed = cost.count.unwrap_or(1) as usize;

        let has_card = match source {
            "stage" => {
                // Check if player has a member on stage
                if card_type == "member_card" {
                    let count = (player.stage.stage[0] != -1) as usize +
                                   (player.stage.stage[1] != -1) as usize +
                                   (player.stage.stage[2] != -1) as usize;
                    count >= count_needed
                } else {
                    false
                }
            }
            "hand" => {
                let card_db = &game_state.card_database;
                if card_type == "member_card" {
                    player.hand.cards.iter().filter(|&id| {
                        card_db.get_card(*id).map_or(false, |c| c.is_member())
                    }).count() >= count_needed
                } else if card_type == "live_card" {
                    player.hand.cards.iter().filter(|&id| {
                        card_db.get_card(*id).map_or(false, |c| c.is_live())
                    }).count() >= count_needed
                } else {
                    player.hand.cards.len() >= count_needed
                }
            }
            "discard" => {
                let card_db = &game_state.card_database;
                if card_type == "member_card" {
                    player.waitroom.cards.iter().filter(|&id| {
                        card_db.get_card(*id).map_or(false, |c| c.is_member())
                    }).count() >= count_needed
                } else if card_type == "live_card" {
                    player.waitroom.cards.iter().filter(|&id| {
                        card_db.get_card(*id).map_or(false, |c| c.is_live())
                    }).count() >= count_needed
                } else {
                    player.waitroom.cards.len() >= count_needed
                }
            }
            "deck" => {
                player.main_deck.cards.len() >= count_needed
            }
            "success_live_zone" => {
                player.success_live_card_zone.cards.len() >= count_needed
            }
            "live_card_zone" => {
                player.live_card_zone.cards.len() >= count_needed
            }
            "energy_zone" => {
                player.energy_zone.cards.len() >= count_needed
            }
            _ => false,
        };

        if has_card {
            CostCalculation {
                payable: true,
                reason: None,
                cost_description: cost.text.clone(),
            }
        } else {
            CostCalculation {
                payable: false,
                reason: Some(format!(
                    "Not enough {} cards in {} (need {}, have {})",
                    card_type,
                    source,
                    count_needed,
                    match source {
                        "stage" => (player.stage.stage[0] != -1) as usize +
                                       (player.stage.stage[1] != -1) as usize +
                                       (player.stage.stage[2] != -1) as usize,
                        "hand" => player.hand.cards.len(),
                        "discard" => player.waitroom.cards.len(),
                        "deck" => player.main_deck.cards.len(),
                        "success_live_zone" => player.success_live_card_zone.cards.len(),
                        "live_card_zone" => player.live_card_zone.cards.len(),
                        "energy_zone" => player.energy_zone.cards.len(),
                        _ => 0,
                    }
                )),
                cost_description: cost.text.clone(),
            }
        }
    }

    fn calculate_pay_energy_cost(
        &self,
        cost: &AbilityCost,
        player: &Player,
        _game_state: &GameState,
    ) -> CostCalculation {
        let energy_needed = cost.energy.unwrap_or(1) as usize;
        let active_energy = player.count_active_energy();

        if active_energy >= energy_needed {
            CostCalculation {
                payable: true,
                reason: None,
                cost_description: cost.text.clone(),
            }
        } else {
            CostCalculation {
                payable: false,
                reason: Some(format!(
                    "Need {} active energy, have {}",
                    energy_needed, active_energy
                )),
                cost_description: cost.text.clone(),
            }
        }
    }

    fn calculate_change_state_cost(
        &self,
        cost: &AbilityCost,
        player: &Player,
        _game_state: &GameState,
    ) -> CostCalculation {
        // Check if card can change to the required state
        let state = cost.state_change.as_deref().unwrap_or("");

        match state {
            "wait" | "ウェイト" => {
                // Check if any stage card is in active state
                // Orientation is now tracked in GameState modifiers
                // For now, assume all stage cards are active if they exist
                let has_active = player.stage.stage[0] != -1
                    || player.stage.stage[1] != -1
                    || player.stage.stage[2] != -1;

                if has_active {
                    CostCalculation {
                        payable: true,
                        reason: None,
                        cost_description: cost.text.clone(),
                    }
                } else {
                    CostCalculation {
                        payable: false,
                        reason: Some("No card in active state to change to wait".to_string()),
                        cost_description: cost.text.clone(),
                    }
                }
            }
            _ => CostCalculation {
                payable: false,
                reason: Some(format!("Unknown state: {}", state)),
                cost_description: cost.text.clone(),
            },
        }
    }

    fn calculate_choice_condition_cost(
        &self,
        cost: &AbilityCost,
        player: &Player,
        _game_state: &GameState,
    ) -> CostCalculation {
        // Choice condition cost - check if the choice can be made
        // This typically involves checking if the required cards exist
        let source = cost.source.as_deref().unwrap_or("");
        let count = cost.count.unwrap_or(1) as usize;

        let has_options = match source {
            "hand" => player.hand.cards.len() >= count,
            "deck" => player.main_deck.cards.len() >= count,
            "discard" => player.waitroom.cards.len() >= count,
            "stage" => {
                (player.stage.stage[0] != -1) as usize +
                (player.stage.stage[1] != -1) as usize +
                (player.stage.stage[2] != -1) as usize >= count
            }
            _ => true, // For now, assume payable for unknown sources
        };

        if has_options {
            CostCalculation {
                payable: true,
                reason: None,
                cost_description: cost.text.clone(),
            }
        } else {
            CostCalculation {
                payable: false,
                reason: Some(format!("Not enough options in {} to make a choice", source)),
                cost_description: cost.text.clone(),
            }
        }
    }

    fn calculate_energy_condition_cost(
        &self,
        cost: &AbilityCost,
        player: &Player,
        _game_state: &GameState,
    ) -> CostCalculation {
        // Energy condition cost - check if energy state meets requirements
        let energy_needed = cost.energy.unwrap_or(1) as usize;
        let active_energy = player.count_active_energy();

        if active_energy >= energy_needed {
            CostCalculation {
                payable: true,
                reason: None,
                cost_description: cost.text.clone(),
            }
        } else {
            CostCalculation {
                payable: false,
                reason: Some(format!(
                    "Energy condition not met: need {}, have {}",
                    energy_needed, active_energy
                )),
                cost_description: cost.text.clone(),
            }
        }
    }

    fn calculate_reveal_cost(
        &self,
        cost: &AbilityCost,
        player: &Player,
        _game_state: &GameState,
    ) -> CostCalculation {
        // Reveal cost - check if cards can be revealed from source
        let source = cost.source.as_deref().unwrap_or("");
        let count = cost.count.unwrap_or(1) as usize;

        let has_cards = match source {
            "hand" => player.hand.cards.len() >= count,
            "deck" => player.main_deck.cards.len() >= count,
            _ => true, // For now, assume payable for unknown sources
        };

        if has_cards {
            CostCalculation {
                payable: true,
                reason: None,
                cost_description: cost.text.clone(),
            }
        } else {
            CostCalculation {
                payable: false,
                reason: Some(format!("Not enough cards in {} to reveal", source)),
                cost_description: cost.text.clone(),
            }
        }
    }

    /// Evaluate if a condition is met
    pub fn evaluate_condition(
        &self,
        condition: &Condition,
        player: &Player,
        game_state: &GameState,
    ) -> bool {
        match condition.condition_type.as_deref() {
            Some("location_condition") => self.evaluate_location_condition(condition, player, game_state),
            Some("count_condition") => self.evaluate_count_condition(condition, player, game_state),
            Some("character_presence_condition") => self.evaluate_character_presence(condition, player, game_state),
            Some("group_presence_condition") => self.evaluate_group_presence(condition, player, game_state),
            Some("energy_state_condition") => self.evaluate_energy_state(condition, player),
            _ => true, // Unknown condition types default to true for now
        }
    }

    fn evaluate_location_condition(
        &self,
        condition: &Condition,
        player: &Player,
        game_state: &GameState,
    ) -> bool {
        let location = condition.location.as_deref().unwrap_or("");
        let card_type = condition.card_type.as_deref().unwrap_or("");

        match location {
            "stage" => {
                if card_type == "member_card" {
                    player.stage.stage[0] != -1
                        || player.stage.stage[1] != -1
                        || player.stage.stage[2] != -1
                } else {
                    false
                }
            }
            "hand" => {
                let card_db = &game_state.card_database;
                if card_type == "member_card" {
                    player.hand.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(false, |c| c.is_member())
                    }).any(|x| x)
                } else if card_type == "live_card" {
                    player.hand.cards.iter().any(|&id| {
                        card_db.get_card(id).map_or(false, |c| c.is_live())
                    })
                } else {
                    !player.hand.is_empty()
                }
            }
            _ => false,
        }
    }

    fn evaluate_count_condition(
        &self,
        condition: &Condition,
        player: &Player,
        _game_state: &GameState,
    ) -> bool {
        let location = condition.location.as_deref().unwrap_or("");
        let count = condition.count.unwrap_or(0) as usize;
        let operator = condition.operator.as_deref().unwrap_or(">=");

        let actual_count = player.count_cards_in_zone(location);

        match operator {
            ">=" => actual_count >= count,
            "<=" => actual_count <= count,
            "==" => actual_count == count,
            ">" => actual_count > count,
            "<" => actual_count < count,
            _ => false,
        }
    }

    fn evaluate_character_presence(&self, condition: &Condition, player: &Player, game_state: &GameState) -> bool {
        if let Some(ref characters) = condition.characters {
            if characters.is_empty() {
                return true;
            }
            // Check if ANY of the characters are present (OR logic)
            characters.iter().any(|name| player.has_character_on_stage(name, &game_state.card_database))
        } else {
            true
        }
    }

    fn evaluate_group_presence(&self, condition: &Condition, player: &Player, game_state: &GameState) -> bool {
        if let Some(ref group) = condition.group {
            // Convert serde_json::Value to string
            let group_str = group.as_str().unwrap_or("");
            player.has_group_on_stage(group_str, &game_state.card_database)
        } else if let Some(ref group_names) = condition.group_names {
            if group_names.is_empty() {
                return true;
            }
            // Check if ANY of the groups are present (OR logic)
            group_names.iter().any(|name| player.has_group_on_stage(name, &game_state.card_database))
        } else {
            true
        }
    }

    fn evaluate_energy_state(&self, condition: &Condition, player: &Player) -> bool {
        let state = condition.energy_state.as_deref().unwrap_or("");

        match state {
            "active" => player.count_active_energy() > 0,
            "wait" => player.count_wait_energy() > 0,
            _ => false,
        }
    }

    /// Validate if an ability can be executed
    pub fn can_execute_ability(
        &self,
        ability: &Ability,
        player: &Player,
        game_state: &GameState,
    ) -> AbilityValidation {
        // Check if ability has conditions
        // Note: Currently abilities don't have a conditions field in the parsed structure
        // Conditions are typically part of the cost or effect requirements

        // Check if cost is payable
        let cost_payable = if let Some(ref cost) = ability.cost {
            let calc = self.calculate_cost(cost, player, game_state);
            calc.payable
        } else {
            true // No cost means always payable
        };

        if cost_payable {
            AbilityValidation {
                can_execute: true,
                conditions_met: true,
                cost_payable: true,
                reason: "Ability can be executed".to_string(),
            }
        } else {
            let cost_text = ability.cost.as_ref().map(|c| c.text.as_str()).unwrap_or("");
            AbilityValidation {
                can_execute: false,
                conditions_met: true,
                cost_payable: false,
                reason: format!("Cost cannot be paid: {}", cost_text),
            }
        }
    }

    /// Execute a move_cards effect
    pub fn execute_move_cards(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let source = effect.source.as_deref().unwrap_or("");
        let destination = effect.destination.as_deref().unwrap_or("");
        let count = effect.count.unwrap_or(1);
        let card_type = effect.card_type.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        // Resolve target
        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        // Collect target player IDs to avoid holding mutable borrow during function calls
        let target_player_ids: Vec<String> = target_players
            .into_iter()
            .filter(|tp| tp.id == player.id)
            .map(|tp| tp.id.clone())
            .collect();

        if !target_player_ids.is_empty() {
            let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
            match (source, destination) {
                ("discard", "hand") => {
                    self.move_from_discard_to_hand(player, count_usize, card_type, game_state)?;
                }
                ("stage", "discard") => {
                    self.move_from_stage_to_discard(player, false, false, game_state)?;
                }
                ("hand", "discard") => {
                    self.move_from_hand_to_discard(player, count_usize)?;
                }
                ("deck", "hand") => {
                    self.draw_cards(player, count_usize)?;
                }
                ("deck_top", "hand") => {
                    self.draw_cards(player, count_usize)?;
                }
                ("hand", "deck_bottom") => {
                    self.move_from_hand_to_deck_bottom(player, count_usize)?;
                }
                ("looked_at", "deck_top") => {
                    // Move selected cards from looked-at set to deck top
                    self.move_from_looked_at_to_deck_top(player, count_usize)?;
                }
                ("looked_at_remaining", "discard") => {
                    // Move remaining looked-at cards to discard
                    self.move_looked_at_remaining_to_discard(player)?;
                }
                ("selected_cards", "hand") => {
                    // Move cards from selected set to hand
                    self.move_from_selected_to_hand(player, count_usize)?;
                }
                _ => {
                    return Err(format!(
                        "Unsupported move: {} -> {}",
                        source, destination
                    ));
                }
            }
        }

        Ok(())
    }

    fn move_from_discard_to_hand(
        &mut self,
        player: &mut Player,
        count: usize,
        card_type: &str,
        game_state: &GameState,
    ) -> Result<(), String> {
        let card_db = &game_state.card_database;

        // First, count how many matching cards are in discard
        let matching_indices: Vec<usize> = player.waitroom.cards.iter().enumerate()
            .filter(|(_, card_id)| {
                if let Some(card) = card_db.get_card(**card_id) {
                    match card_type {
                        "member_card" => card.is_member(),
                        "live_card" => card.is_live(),
                        _ => true,
                    }
                } else {
                    false
                }
            })
            .map(|(i, _)| i)
            .collect();

        if matching_indices.len() < count {
            return Err(format!(
                "Not enough cards in discard: needed {}, have {}",
                count, matching_indices.len()
            ));
        }

        // If there are more matching cards than needed, prompt user to choose
        if matching_indices.len() > count {
            let description = format!("Select {} card(s) from discard to add to hand ({} available)", count, matching_indices.len());
            
            self.pending_choice = Some(Choice::SelectCard {
                zone: "discard".to_string(),
                card_type: Some(card_type.to_string()),
                count,
                description,
            });
            
            return Err("Pending choice required: select cards from discard to add to hand".to_string());
        }

        // If exact number or fewer (already checked), move them automatically
        let mut moved = 0;
        let mut indices_to_remove = Vec::new();

        for (i, card_id) in player.waitroom.cards.iter().enumerate() {
            if moved >= count {
                break;
            }

            let matches_type = if let Some(card) = card_db.get_card(*card_id) {
                match card_type {
                    "member_card" => card.is_member(),
                    "live_card" => card.is_live(),
                    _ => true,
                }
            } else {
                false
            };

            if matches_type {
                indices_to_remove.push(i);
                player.hand.add_card(*card_id);
                moved += 1;
            }
        }

        // Remove cards from waitroom (in reverse order to maintain indices)
        for i in indices_to_remove.into_iter().rev() {
            player.waitroom.cards.remove(i);
        }

        Ok(())
    }

    fn move_from_stage_to_discard(&mut self, player: &mut Player, is_self_cost: bool, exclude_self: bool, game_state: &GameState) -> Result<(), String> {
        // This is a cost - move member(s) from stage to discard
        if is_self_cost {
            // Self-cost: the card itself is the cost
            if let Some(activating_card_id) = game_state.activating_card {
                // Find and discard the activating card from stage
                let mut found = false;
                for i in 0..3 {
                    if player.stage.stage[i] == activating_card_id {
                        player.stage.stage[i] = -1;
                        player.waitroom.add_card(activating_card_id);
                        eprintln!("Self-cost: discarded activating card {} from stage position {}", activating_card_id, i);
                        found = true;
                        break;
                    }
                }
                if !found {
                    return Err(format!("Activating card {} not found on stage", activating_card_id));
                }
            } else {
                // Fallback: no activating card tracked, prompt user (shouldn't happen in normal flow)
                let stage_count = player.stage.stage.iter().filter(|&&c| c != -1).count();
                if stage_count == 0 {
                    return Err("No cards on stage to discard".to_string());
                }
                eprintln!("Self-cost: no activating card tracked, removing first card");
                for i in 0..3 {
                    if player.stage.stage[i] != -1 {
                        let card_id = player.stage.stage[i];
                        player.stage.stage[i] = -1;
                        player.waitroom.add_card(card_id);
                        eprintln!("Discarded card at position {}", i);
                        break;
                    }
                }
            }
        } else {
            // Non-self-cost: need user selection (or use exclude_self to filter)
            let activating_card_id = game_state.activating_card;
            let mut cards_to_discard = Vec::new();
            
            for i in 0..3 {
                if player.stage.stage[i] != -1 {
                    let card_id = player.stage.stage[i];
                    // Skip if exclude_self and this is the activating card
                    if exclude_self && activating_card_id == Some(card_id) {
                        eprintln!("Excluding activating card {} from discard", card_id);
                        continue;
                    }
                    cards_to_discard.push((i, card_id));
                }
            }
            
            if cards_to_discard.is_empty() {
                return Err("No cards on stage to discard (after exclude_self filter)".to_string());
            }
            
            // If multiple cards available, require user selection
            if cards_to_discard.len() > 1 {
                // Create pending choice for user to select which card to discard
                let description = format!("Select 1 card from stage to discard ({} valid options)", cards_to_discard.len());
                
                self.pending_choice = Some(Choice::SelectCard {
                    zone: "stage".to_string(),
                    card_type: Some("member_card".to_string()),
                    count: 1,
                    description,
                });
                
                // Store the valid card indices for later selection
                // Note: This requires the caller to handle the pending choice
                return Err("Pending choice required: select card to discard from stage".to_string());
            }
            
            // Only one card available, discard it
            for (i, card_id) in cards_to_discard {
                player.stage.stage[i] = -1;
                player.waitroom.add_card(card_id);
                eprintln!("Discarded card {} from stage position {}", card_id, i);
            }
        }
        Ok(())
    }

    fn move_from_hand_to_discard(&mut self, player: &mut Player, count: usize) -> Result<(), String> {
        // This requires user choice - if multiple cards available, prompt for selection
        if player.hand.cards.len() > count {
            // Create pending choice for user to select which cards to discard
            let description = format!("Select {} card(s) from hand to discard ({} available)", count, player.hand.cards.len());
            
            self.pending_choice = Some(Choice::SelectCard {
                zone: "hand".to_string(),
                card_type: None,
                count,
                description,
            });
            
            return Err("Pending choice required: select cards to discard from hand".to_string());
        }
        
        // If count equals or exceeds hand size, discard all cards
        let cards_to_remove: Vec<_> = player.hand.cards.iter().take(count).copied().collect();
        for card_id in cards_to_remove {
            player.waitroom.add_card(card_id);
        }
        let remove_count = count.min(player.hand.cards.len());
        player.hand.cards.drain(..remove_count);
        Ok(())
    }

    fn discard_until_count(&mut self, player: &mut Player, target_count: usize) -> Result<(), String> {
        // Discard cards from hand until hand size reaches target_count
        let current_count = player.hand.cards.len();
        if current_count <= target_count {
            // Already at or below target count, no discard needed
            return Ok(());
        }
        
        let cards_to_discard = current_count - target_count;
        if cards_to_discard > 0 {
            // Create pending choice for user to select which cards to discard
            let description = format!("Select {} card(s) from hand to discard until hand has {} cards ({} available)", cards_to_discard, target_count, current_count);
            
            self.pending_choice = Some(Choice::SelectCard {
                zone: "hand".to_string(),
                card_type: None,
                count: cards_to_discard,
                description,
            });
            
            return Err("Pending choice required: select cards to discard from hand".to_string());
        }
        
        Ok(())
    }

    fn draw_cards(&self, player: &mut Player, count: usize) -> Result<(), String> {
        for _ in 0..count {
            if let Some(card_id) = player.main_deck.draw() {
                player.hand.add_card(card_id);
            } else {
                return Err("Deck is empty".to_string());
            }
        }
        Ok(())
    }

    fn move_from_hand_to_deck_bottom(&mut self, player: &mut Player, count: usize) -> Result<(), String> {
        // This requires user choice - if multiple cards available, prompt for selection
        if player.hand.cards.len() > count {
            // Create pending choice for user to select which cards to move
            let description = format!("Select {} card(s) from hand to move to bottom of deck ({} available)", count, player.hand.cards.len());

            self.pending_choice = Some(Choice::SelectCard {
                zone: "hand".to_string(),
                card_type: None,
                count,
                description,
            });

            return Err("Pending choice required: select cards to move from hand to deck bottom".to_string());
        }

        // If count equals or exceeds hand size, move all cards to bottom
        let cards_to_move: Vec<_> = player.hand.cards.iter().take(count).copied().collect();
        for card_id in cards_to_move {
            player.main_deck.cards.push(card_id);
        }
        let remove_count = count.min(player.hand.cards.len());
        player.hand.cards.drain(..remove_count);
        Ok(())
    }

    /// Execute a draw effect
    pub fn execute_draw(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1) as usize;
        self.draw_cards(player, count)
    }

    /// Execute a gain_resource effect (blades)
    pub fn execute_gain_resource(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let resource = effect.resource.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let count = effect.count.unwrap_or(1);

        if resource != "blade" && resource != "ブレード" && resource != "heart" && resource != "ハート" {
            return Err(format!("Unsupported resource: {}", resource));
        }

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        // Collect modifications first to avoid borrow conflicts
        let mut blade_modifications: Vec<(i16, i32)> = Vec::new();
        let mut heart_modifications: Vec<(i16, crate::card::HeartColor, i32)> = Vec::new();

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Add resource to all stage members based on type
            let areas = [0, 1, 2]; // indices for left_side, center, right_side
            for index in areas {
                let card_id = target_player.stage.stage[index];
                if card_id != -1 {
                    match resource {
                        "blade" | "ブレード" => {
                            blade_modifications.push((card_id, count as i32));
                        }
                        "heart" | "ハート" => {
                            let color = if let Some(ref heart_color) = effect.heart_color {
                                crate::zones::parse_heart_color(heart_color)
                            } else {
                                crate::card::HeartColor::Heart00
                            };
                            heart_modifications.push((card_id, color, count as i32));
                        }
                        _ => {}
                    }
                }
            }
        }

        // Apply modifications after releasing mutable borrow
        for (card_id, modifier) in blade_modifications {
            game_state.add_blade_modifier(card_id, modifier);
        }
        for (card_id, color, modifier) in heart_modifications {
            game_state.add_heart_modifier(card_id, color, modifier);
        }

        Ok(())
    }

    /// Execute a modify_score effect
    pub fn execute_modify_score(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let value = effect.value.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut card_ids_to_modify: Vec<(i16, i32)> = Vec::new();
        
        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Collect card_ids to modify
            for card_id in &target_player.live_card_zone.cards {
                match operation {
                    "add" => {
                        card_ids_to_modify.push((*card_id, value as i32));
                    }
                    "remove" => {
                        card_ids_to_modify.push((*card_id, -(value as i32)));
                    }
                    "set" => {
                        card_ids_to_modify.push((*card_id, value as i32));
                    }
                    _ => return Err(format!("Unknown operation: {}", operation)),
                }
            }
        }
        
        // Apply modifiers after borrows are released
        for (card_id, delta) in card_ids_to_modify {
            if operation == "set" {
                game_state.score_modifiers.insert(card_id, delta);
            } else {
                game_state.add_score_modifier(card_id, delta);
            }
        }

        Ok(())
    }

    /// Execute a modify_required_hearts effect
    pub fn execute_modify_required_hearts(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("decrease");
        let value = effect.value.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut card_ids_to_modify: Vec<(i16, crate::card::HeartColor, i32)> = Vec::new();
        
        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Collect card_ids to modify
            let color = crate::zones::parse_heart_color(heart_color);
            for card_id in &target_player.live_card_zone.cards {
                match operation {
                    "decrease" => {
                        card_ids_to_modify.push((*card_id, color, -(value as i32)));
                    }
                    "increase" => {
                        card_ids_to_modify.push((*card_id, color, value as i32));
                    }
                    "set" => {
                        card_ids_to_modify.push((*card_id, color, value as i32));
                    }
                    _ => return Err(format!("Unknown operation: {}", operation)),
                }
            }
        }
        
        // Apply modifiers after borrows are released
        for (card_id, color, delta) in card_ids_to_modify {
            if operation == "set" {
                game_state.set_need_heart_modifier(card_id, color, delta);
            } else {
                game_state.add_need_heart_modifier(card_id, color, delta);
            }
        }

        Ok(())
    }

    /// Execute a set_required_hearts effect
    pub fn execute_set_required_hearts(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        let mut card_ids_to_modify: Vec<(i16, crate::card::HeartColor, i32)> = Vec::new();
        
        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Collect card_ids to modify
            let color = crate::zones::parse_heart_color(heart_color);
            for card_id in &target_player.live_card_zone.cards {
                card_ids_to_modify.push((*card_id, color, count as i32));
            }
        }
        
        // Apply modifiers after borrows are released
        for (card_id, color, count) in card_ids_to_modify {
            game_state.set_need_heart_modifier(card_id, color, count);
        }

        Ok(())
    }

    /// Execute a modify_required_hearts_global effect
    pub fn execute_modify_required_hearts_global(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("increase");
        let value = effect.value.unwrap_or(1);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("opponent");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        // Collect card_ids to modify first to avoid borrow conflicts
        let mut card_ids_to_modify: Vec<(i16, crate::card::HeartColor, i32)> = Vec::new();

        for target_player in target_players {
            // Modify required hearts for all live cards
            let color = crate::zones::parse_heart_color(heart_color);
            for card_id in &target_player.live_card_zone.cards {
                let modifier_value = match operation {
                    "increase" => value as i32,
                    "decrease" => -(value as i32),
                    _ => return Err(format!("Unknown operation: {}", operation)),
                };
                card_ids_to_modify.push((*card_id, color, modifier_value));
            }
        }

        // Apply modifiers after releasing mutable borrow
        for (card_id, color, modifier_value) in card_ids_to_modify {
            game_state.add_need_heart_modifier(card_id, color, modifier_value);
        }

        Ok(())
    }

    /// Execute a set_blade_type effect
    pub fn execute_set_blade_type(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let blade_type = effect.blade_type.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        // Collect turn/phase before mutable borrow
        let current_turn = game_state.turn_number;
        let current_phase = game_state.current_phase.clone();
        let effect_duration = effect.duration.clone();

        // Clone card_database reference to avoid borrow conflict
        let card_db = game_state.card_database.clone();

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);
        
        // Collect temporary effects first to avoid borrow conflicts
        let mut temp_effects = Vec::new();

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Set blade type for stage members
            let areas = [0, 1, 2]; // indices for left_side, center, right_side
            for index in areas {
                let card_id = target_player.stage.stage[index];
                if card_id != -1 {
                    // Store blade type as a temporary effect or card attribute
                    // For now, we'll track this in game state temporary effects
                    let temp_effect = crate::game_state::TemporaryEffect {
                        effect_type: format!("set_blade_type:{}", blade_type),
                        duration: effect_duration.as_ref().map(|d| match d.as_str() {
                            "live_end" => crate::game_state::Duration::LiveEnd,
                            "this_turn" => crate::game_state::Duration::ThisTurn,
                            "this_live" => crate::game_state::Duration::ThisLive,
                            "permanent" => crate::game_state::Duration::Permanent,
                            "as_long_as" => crate::game_state::Duration::ThisLive, // Map to ThisLive for now
                            _ => crate::game_state::Duration::ThisLive,
                        }).unwrap_or(crate::game_state::Duration::ThisLive),
                        created_turn: current_turn,
                        created_phase: current_phase.clone(),
                        target_player_id: target_player.id.clone(),
                        description: format!("Set blade type to {} for {}", blade_type, card_db.get_card(card_id).map(|c| c.name.as_str()).unwrap_or("unknown")),
                        creation_order: 0,
                        effect_data: None,
                    };
                    temp_effects.push(temp_effect);
                }
            }
        }
        
        // Push all temp effects after the loop
        for effect in temp_effects {
            game_state.temporary_effects.push(effect);
        }

        Ok(())
    }

    /// Execute a set_heart_type effect
    pub fn execute_set_heart_type(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        // Use heart_type field if available, otherwise fall back to heart_color
        let heart_type = effect.heart_type.as_deref().or(effect.heart_color.as_deref()).unwrap_or("heart00");
        let target = effect.target.as_deref().unwrap_or("self");
        let count = effect.count.unwrap_or(1) as i32;

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        // Collect card_ids to modify, then apply modifiers after releasing the borrow
        let mut card_ids_to_modify: Vec<i16> = Vec::new();

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Set heart type for stage members
            let areas = [0, 1, 2]; // indices for left_side, center, right_side
            for index in areas {
                let card_id = target_player.stage.stage[index];
                if card_id != -1 {
                    card_ids_to_modify.push(card_id);
                }
            }
        }

        // Apply heart modifiers after releasing the mutable borrow
        let color = crate::zones::parse_heart_color(heart_type);
        for card_id in card_ids_to_modify {
            game_state.add_heart_modifier(card_id, color, count);
            eprintln!("Added heart modifier: card_id={}, color={:?}, count={}", card_id, color, count);
        }

        Ok(())
    }

    /// Execute a position_change effect
    pub fn execute_position_change(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let position = effect.position.as_ref().and_then(|p| p.get_position()).unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");

        let card_database = Arc::clone(&game_state.card_database); // Clone Arc to avoid borrow conflict
        let target_players = game_state.resolve_target_mut(target, perspective_player_id);
        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            let target_index = match position {
                "center" | "センターエリア" => 1,
                "left_side" | "左サイドエリア" => 0,
                "right_side" | "右サイドエリア" => 2,
                _ => return Err(format!("Unknown position: {}", position)),
            };

            // Find the member to move based on target_member
            let current_index = if target_member == "this_member" {
                // Get the member that triggered this ability
                // This should be stored in the context or passed differently
                return Err("position_change with 'this_member' requires context tracking - not yet implemented".to_string());
            } else {
                // Find member by card number in stage array
                target_player.stage.stage.iter()
                    .position(|&card_id| {
                        if card_id == -1 {
                            false
                        } else {
                            card_database.get_card(card_id)
                                .map(|c| c.card_no == target_member)
                                .unwrap_or(false)
                        }
                    })
            };

            if let Some(current_idx) = current_index {
                // Move the member from current_idx to target_index
                let card_id = target_player.stage.stage[current_idx];
                
                // Check if target area is occupied
                if target_player.stage.stage[target_index] != -1 {
                    // Swap positions if occupied
                    let occupying_card = target_player.stage.stage[target_index];
                    target_player.stage.stage[target_index] = card_id;
                    target_player.stage.stage[current_idx] = occupying_card;
                    println!("Position change: swapped members between index {} and {}", current_idx, target_index);
                } else {
                    // Move to empty area
                    target_player.stage.stage[target_index] = card_id;
                    target_player.stage.stage[current_idx] = -1;
                    println!("Position change: moved member from index {} to {}", current_idx, target_index);
                }
            } else {
                return Err(format!("Member not found: {}", target_member));
            }
        }

        Ok(())
    }

    /// Execute a place_energy_under_member effect
    pub fn execute_place_energy_under_member(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let energy_count = effect.energy_count.unwrap_or(1);
        let target_member = effect.target_member.as_deref().unwrap_or("this_member");
        let target = effect.target.as_deref().unwrap_or("self");

        // Store activating_card before borrowing game_state mutably
        let activating_card_id = game_state.activating_card;
        
        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Draw energy cards and place under member
            for _ in 0..energy_count {
                if let Some(energy_card) = target_player.energy_deck.draw() {
                    // Place energy under the specified member
                    match target_member {
                        "this_member" => {
                            // Place under the member that activated the ability
                            // Use stored activating_card_id to find the member
                            if let Some(activating_id) = activating_card_id {
                                // Check if the activating card is on this player's stage
                                let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
                                let mut placed = false;
                                for area in &areas {
                                    if let Some(stage_card_id) = target_player.stage.get_area(*area) {
                                        if stage_card_id == activating_id {
                                            // Found the member - store energy placement info
                                            target_player.energy_zone.cards.push(energy_card);
                                            eprintln!("Placed energy {} under member {} at {:?}", 
                                                     energy_card, activating_id, area);
                                            placed = true;
                                            break;
                                        }
                                    }
                                }
                                if !placed {
                                    // Activating card not on stage, add to energy zone
                                    target_player.energy_zone.cards.push(energy_card);
                                }
                            } else {
                                // No activating card tracked, add to energy zone
                                target_player.energy_zone.cards.push(energy_card);
                            }
                        }
                        _ => {
                            // Place under specified member (by name/position)
                            // Try to find member by position first
                            let target_area = match target_member {
                                "left" | "left_side" => Some(crate::zones::MemberArea::LeftSide),
                                "center" => Some(crate::zones::MemberArea::Center),
                                "right" | "right_side" => Some(crate::zones::MemberArea::RightSide),
                                _ => None,
                            };
                            
                            if let Some(area) = target_area {
                                if let Some(member_id) = target_player.stage.get_area(area) {
                                    target_player.energy_zone.cards.push(energy_card);
                                    eprintln!("Placed energy {} under member {} at {:?}", 
                                             energy_card, member_id, area);
                                } else {
                                    target_player.energy_zone.cards.push(energy_card);
                                }
                            } else {
                                // Unknown target, add to energy zone
                                target_player.energy_zone.cards.push(energy_card);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Execute a modify_yell_count effect
    pub fn execute_modify_yell_count(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("subtract");
        let count = effect.count.unwrap_or(0);
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        // Check if we should apply the effect (only for self target for now)
        let should_apply = target_players.iter().any(|p| p.id == player.id);

        if should_apply {
            // Modify yell count - this affects the cheer check count
            match operation {
                "add" => {
                    game_state.cheer_checks_required += count;
                }
                "subtract" => {
                    game_state.cheer_checks_required = game_state.cheer_checks_required.saturating_sub(count);
                }
                "set" => {
                    game_state.cheer_checks_required = count;
                }
                _ => return Err(format!("Unknown operation: {}", operation)),
            }
        }

        Ok(())
    }

    /// Request a choice from the user
    pub fn request_choice(&mut self, choice: Choice) -> Result<(), String> {
        self.pending_choice = Some(choice);
        Ok(())
    }

    /// Get pending choice (if any)
    pub fn get_pending_choice(&self) -> Option<&Choice> {
        self.pending_choice.as_ref()
    }

    /// Provide choice result
    pub fn provide_choice_result(&mut self, result: ChoiceResult) -> Result<(), String> {
        match (&self.pending_choice, result) {
            (Some(Choice::SelectCard { zone, card_type, count, .. }), ChoiceResult::CardSelected { indices }) => {
                self.pending_choice = None;
                Ok(())
            }
            (Some(Choice::SelectTarget { target, .. }), ChoiceResult::TargetSelected { .. }) => {
                if target == "pay_optional_cost" {
                    // Player chose to pay the optional cost - clear the choice but keep the cost
                    self.pending_choice = None;
                    return Ok(());
                } else {
                    // Player chose not to pay the optional cost
                    self.pending_optional_cost = None;
                    self.pending_choice = None;
                    return Ok(());
                }
            }
            (Some(Choice::SelectPosition { .. }), ChoiceResult::PositionSelected { .. }) => {
                self.pending_choice = None;
                Ok(())
            }
            _ => Err("Choice result does not match pending choice".to_string()),
        }
    }

    /// Execute the pending optional cost if the player chose to pay it
    pub fn execute_pending_optional_cost(
        &mut self,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        if let Some(ref cost) = self.pending_optional_cost {
            let cost_to_execute = cost.clone();
            self.pending_optional_cost = None;
            self.execute_cost(&cost_to_execute, player, game_state, perspective_player_id)
        } else {
            Ok(())
        }
    }

    /// Skip the pending optional cost if the player chose not to pay it
    pub fn skip_pending_optional_cost(&mut self) {
        self.pending_optional_cost = None;
    }

    /// Resume ability execution after optional cost choice
    pub fn resume_ability_execution_after_cost_choice(
        &mut self,
        ability: &Ability,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
        pay_cost: bool,
    ) -> Result<(), String> {
        if pay_cost {
            // Execute the pending optional cost
            self.execute_pending_optional_cost(player, game_state, perspective_player_id)?;
        } else {
            // Skip the optional cost
            self.skip_pending_optional_cost();
        }

        // Apply effect if exists
        if let Some(ref effect) = ability.effect {
            self.execute_effect(effect, player, game_state, perspective_player_id)?;
        }

        // Clear activating card after execution
        game_state.activating_card = None;

        Ok(())
    }

    /// Execute a look_at effect (look at top cards of deck without moving)
    pub fn execute_look_at(
        &mut self,
        effect: &AbilityEffect,
        player: &Player,
    ) -> Result<Vec<i16>, String> {
        let count = effect.count.unwrap_or(1);
        let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
        let cards = player.main_deck.peek_top(count_usize);

        if cards.len() < count_usize {
            return Err(format!(
                "Not enough cards in deck: needed {}, have {}",
                count_usize,
                cards.len()
            ));
        }

        Ok(cards)
    }

    /// Execute sequential actions (multiple effects in order)
    pub fn execute_sequential(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        let actions = effect.actions.as_ref().ok_or("No actions in sequential effect")?;
        let is_conditional = effect.conditional.unwrap_or(false);
        let condition = effect.condition.as_ref();

        // Check condition if this is a conditional sequential effect (e.g., "そうした場合")
        if is_conditional {
            if let Some(cond) = condition {
                let condition_met = self.evaluate_condition(cond, player, game_state);
                eprintln!("Conditional sequential effect with condition: {}, met: {}", cond.text, condition_met);
                
                // If condition is not met, skip execution
                if !condition_met {
                    eprintln!("Condition not met, skipping sequential actions");
                    return Ok(());
                }
            }
        }

        for (index, sub_effect) in actions.iter().enumerate() {
            match sub_effect.action.as_str() {
                "draw" | "draw_card" => {
                    self.execute_draw(sub_effect, player)?;
                }
                "move_cards" => {
                    match self.execute_move_cards(sub_effect, player, game_state, perspective_player_id) {
                        Ok(_) => {},
                        Err(e) if e.contains("Pending choice required") => {
                            // Save the remaining actions to resume after user choice
                            let remaining_actions: Vec<AbilityEffect> = actions[index + 1..].to_vec();
                            if !remaining_actions.is_empty() {
                                game_state.pending_sequential_actions = Some(remaining_actions);
                            }
                            return Err(e);
                        }
                        Err(e) => return Err(e),
                    }
                }
                "look_at" => {
                    // Just look, no movement
                    self.execute_look_at(sub_effect, player)?;
                }
                "gain_resource" => {
                    self.execute_gain_resource(sub_effect, player, game_state, perspective_player_id)?;
                }
                "modify_score" => {
                    self.execute_modify_score(sub_effect, player, game_state, perspective_player_id)?;
                }
                "modify_required_hearts" => {
                    self.execute_modify_required_hearts(sub_effect, player, game_state, perspective_player_id)?;
                }
                "set_required_hearts" => {
                    self.execute_set_required_hearts(sub_effect, player, game_state, perspective_player_id)?;
                }
                "modify_required_hearts_global" => {
                    self.execute_modify_required_hearts_global(sub_effect, player, game_state, perspective_player_id)?;
                }
                "set_blade_type" => {
                    self.execute_set_blade_type(sub_effect, player, game_state, perspective_player_id)?;
                }
                "set_heart_type" => {
                    self.execute_set_heart_type(sub_effect, player, game_state, perspective_player_id)?;
                }
                "position_change" => {
                    self.execute_position_change(sub_effect, player, game_state, perspective_player_id)?;
                }
                "place_energy_under_member" => {
                    self.execute_place_energy_under_member(sub_effect, player, game_state, perspective_player_id)?;
                }
                "modify_yell_count" => {
                    self.execute_modify_yell_count(sub_effect, player, game_state, perspective_player_id)?;
                }
                _ => {
                    return Err(format!("Unknown action in sequence: {}", sub_effect.action));
                }
            }

            // Check if there's a pending choice (pause for user input)
            if self.pending_choice.is_some() {
                // Return early - caller should provide choice result before continuing
                return Ok(());
            }
        }

        Ok(())
    }

    /// Execute an ability effect (dispatch to appropriate handler)
    pub fn execute_effect(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        match effect.action.as_str() {
            "look_and_select" => {
                self.execute_look_and_select(effect, player, game_state, perspective_player_id)
            }
            "move_cards" => {
                self.execute_move_cards(effect, player, game_state, perspective_player_id)
            }
            "draw" | "draw_card" => self.execute_draw(effect, player),
            "discard_until_count" => {
                let target_count = effect.target_count.unwrap_or(0) as usize;
                self.discard_until_count(player, target_count)
            }
            "conditional_alternative" => {
                self.execute_conditional_alternative(effect, player, game_state, perspective_player_id)
            }
            "specify_heart_color" => {
                self.execute_specify_heart_color(effect, player, game_state, perspective_player_id)
            }
            "reveal" => {
                self.execute_reveal(effect, player, game_state, perspective_player_id)
            }
            "gain_ability" => {
                self.execute_gain_ability(effect, player, game_state, perspective_player_id)
            }
            "select" => {
                self.execute_select(effect, player, game_state, perspective_player_id)
            }
            "choice" => {
                self.execute_choice(effect, player, game_state, perspective_player_id)
            }
            "activation_cost" => {
                self.execute_activation_cost(effect, player, game_state, perspective_player_id)
            }
            "shuffle" => {
                self.execute_shuffle(effect, player, game_state, perspective_player_id)
            }
            "draw_until_count" => {
                self.execute_draw_until_count(effect, player, game_state, perspective_player_id)
            }
            "appear" => {
                self.execute_appear(effect, player, game_state, perspective_player_id)
            }
            "modify_cost" => {
                self.execute_modify_cost(effect, player, game_state, perspective_player_id)
            }
            "set_score" => {
                self.execute_set_score(effect, player, game_state, perspective_player_id)
            }
            "set_cost" => {
                self.execute_set_cost(effect, player, game_state, perspective_player_id)
            }
            "set_blade_count" => {
                self.execute_set_blade_count(effect, player, game_state, perspective_player_id)
            }
            "modify_limit" => {
                self.execute_modify_limit(effect, player, game_state, perspective_player_id)
            }
            "invalidate_ability" => {
                self.execute_invalidate_ability(effect, player, game_state, perspective_player_id)
            }
            "choose_heart_type" => {
                self.execute_choose_heart_type(effect, player, game_state, perspective_player_id)
            }
            "modify_required_hearts_success" => {
                self.execute_modify_required_hearts_success(effect, player, game_state, perspective_player_id)
            }
            "set_cost_to_use" => {
                self.execute_set_cost_to_use(effect, player, game_state, perspective_player_id)
            }
            "all_blade_timing" => {
                self.execute_all_blade_timing(effect, player, game_state, perspective_player_id)
            }
            "set_card_identity_all_regions" => {
                self.execute_set_card_identity_all_regions(effect, player, game_state, perspective_player_id)
            }
            "custom" => {
                // Custom effect - card-specific special effects
                // Store in prohibition_effects for tracking
                game_state.prohibition_effects.push(format!("custom:{}", effect.text));
                eprintln!("Custom effect applied: {}", effect.text);
                Ok(())
            }
            "re_yell" => {
                self.execute_re_yell(effect, player, game_state, perspective_player_id)
            }
            "restriction" => {
                self.execute_restriction(effect, player, game_state, perspective_player_id)
            }
            "activation_restriction" => {
                self.execute_activation_restriction(effect, player, game_state, perspective_player_id)
            }
            "set_card_identity" => {
                self.execute_set_card_identity(effect, player, game_state, perspective_player_id)
            }
            "gain_resource" => {
                self.execute_gain_resource(effect, player, game_state, perspective_player_id)
            }
            "look_at" => {
                self.execute_look_at(effect, player)?;
                Ok(())
            }
            "sequential" => {
                self.execute_sequential(effect, player, game_state, perspective_player_id)
            }
            "modify_score" => {
                self.execute_modify_score(effect, player, game_state, perspective_player_id)
            }
            "modify_required_hearts" => {
                self.execute_modify_required_hearts(effect, player, game_state, perspective_player_id)
            }
            "set_required_hearts" => {
                self.execute_set_required_hearts(effect, player, game_state, perspective_player_id)
            }
            "modify_required_hearts_global" => {
                self.execute_modify_required_hearts_global(effect, player, game_state, perspective_player_id)
            }
            "set_blade_type" => {
                self.execute_set_blade_type(effect, player, game_state, perspective_player_id)
            }
            "set_heart_type" => {
                self.execute_set_heart_type(effect, player, game_state, perspective_player_id)
            }
            "position_change" => {
                self.execute_position_change(effect, player, game_state, perspective_player_id)
            }
            "place_energy_under_member" => {
                self.execute_place_energy_under_member(effect, player, game_state, perspective_player_id)
            }
            "modify_yell_count" => {
                self.execute_modify_yell_count(effect, player, game_state, perspective_player_id)
            }
            "change_state" => {
                self.execute_change_state(effect, player, game_state, perspective_player_id)
            }
            "choose_required_hearts" => {
                self.execute_choose_required_hearts(effect, player, game_state, perspective_player_id)
            }
            "pay_energy" => {
                self.execute_pay_energy(effect, player, game_state, perspective_player_id)
            }
            "play_baton_touch" => {
                self.execute_play_baton_touch(effect, player, game_state, perspective_player_id)
            }
            _ => Err(format!("Unknown effect action: {}", effect.action)),
        }
    }

    /// Execute ability cost
    pub fn execute_cost(
        &mut self,
        cost: &AbilityCost,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        match cost.cost_type.as_deref() {
            Some("sequential_cost") => {
                // Execute multiple costs in sequence
                if let Some(ref costs) = cost.costs {
                    for sub_cost in costs {
                        self.execute_cost(sub_cost, player, game_state, perspective_player_id)?;
                    }
                }
                Ok(())
            }
            Some("move_cards") => {
                let source = cost.source.as_deref().unwrap_or("");
                let destination = cost.destination.as_deref().unwrap_or("");

                match (source, destination) {
                    ("stage" | "ステージ", "discard" | "控え室") => {
                        let is_self_cost = cost.self_cost.unwrap_or(false);
                        let exclude_self = cost.exclude_self.unwrap_or(false);
                        self.move_from_stage_to_discard(player, is_self_cost, exclude_self, game_state)?;
                    }
                    ("hand" | "手札", "discard" | "控え室") => {
                        let count = cost.count.unwrap_or(1);
                        let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
                        self.move_from_hand_to_discard(player, count_usize)?;
                    }
                    ("hand" | "手札", "deck_bottom") => {
                        // Move card from hand to bottom of deck
                        let count = cost.count.unwrap_or(1);
                        let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
                        self.move_from_hand_to_deck_bottom(player, count_usize)?;
                    }
                    _ => {
                        return Err(format!(
                            "Unsupported cost move: {} -> {}",
                            source, destination
                        ));
                    }
                }
                Ok(())
            }
            Some("pay_energy") => {
                let energy_needed = cost.energy.unwrap_or(1) as usize;
                // Deactivate energy cards to pay cost
                // Orientation is now tracked in GameState modifiers
                // For now, assume we can always deactivate enough energy cards
                let deactivated = energy_needed;

                if deactivated < energy_needed {
                    return Err(format!(
                        "Could not pay energy: needed {}, deactivated {}",
                        energy_needed, deactivated
                    ));
                }
                Ok(())
            }
            Some("change_state") => {
                let state = cost.state_change.as_deref().unwrap_or("");
                let position = cost.position.as_ref().and_then(|p| p.get_position());

                if let Some(pos) = position {
                    let _area = match pos {
                        "center" | "センターエリア" => MemberArea::Center,
                        "left_side" | "左サイドエリア" => MemberArea::LeftSide,
                        "right_side" | "右サイドエリア" => MemberArea::RightSide,
                        _ => return Err(format!("Unknown position: {}", pos)),
                    };

                    let _orientation = match state {
                        "active" | "アクティブ" => crate::zones::Orientation::Active,
                        "wait" | "ウェイト" => crate::zones::Orientation::Wait,
                        _ => return Err(format!("Unknown state: {}", state)),
                    };

                    // Orientation is now tracked in GameState modifiers
                    // For now, this is a no-op
                }
                Ok(())
            }
            Some("reveal") => {
                // Reveal cost - typically used to show cards from hand
                // For now, this is a no-op as the UI handles revelation
                eprintln!("Reveal cost: {}", cost.text);
                Ok(())
            }
            _ => Err(format!("Unknown cost type: {:?}", cost.cost_type)),
        }
    }

    /// Execute full ability (pay cost then apply effect)
    pub fn execute_ability(
        &mut self,
        ability: &Ability,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
        activating_card: Option<i16>,
    ) -> Result<(), String> {
        // Store activating card in game state for self-cost handling
        game_state.activating_card = activating_card;
        
        // Pay cost if exists
        if let Some(ref cost) = ability.cost {
            // Check if cost is optional
            if cost.optional.unwrap_or(false) {
                // For optional costs, we need user choice
                let description = if cost.text.is_empty() {
                    format!("Optional cost: Discard cards. Do you want to pay this cost?")
                } else {
                    format!("Optional cost: {}. Do you want to pay this cost?", cost.text)
                };
                
                // Store the optional cost for later execution
                self.pending_optional_cost = Some(cost.clone());
                
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "pay_optional_cost".to_string(),
                    description,
                });
                
                return Err("Pending choice required: choose whether to pay optional cost".to_string());
            } else {
                self.execute_cost(cost, player, game_state, perspective_player_id)?;
            }
        }

        // Apply effect if exists
        if let Some(ref effect) = ability.effect {
            self.execute_effect(effect, player, game_state, perspective_player_id)?;
        }

        // Clear activating card after execution
        game_state.activating_card = None;

        Ok(())
    }

    /// Move selected cards from looked-at set to deck top
    fn move_from_looked_at_to_deck_top(
        &mut self,
        player: &mut Player,
        count: usize,
    ) -> Result<(), String> {
        if self.looked_at_cards.is_empty() {
            return Err("No looked-at cards available".to_string());
        }

        // Take only the specified count of cards
        let actual_count = count.min(self.looked_at_cards.len());
        let cards_to_move: Vec<u32> = self.looked_at_cards.drain(..actual_count).collect();
        
        // Add to deck in reverse order so they appear in the right order
        for card_id in cards_to_move.into_iter().rev() {
            player.main_deck.cards.insert(0, card_id as i16);
        }

        eprintln!("Moved {} looked-at cards to deck top (requested: {})", actual_count, count);
        Ok(())
    }

    /// Move remaining looked-at cards to discard
    fn move_looked_at_remaining_to_discard(
        &mut self,
        player: &mut Player,
    ) -> Result<(), String> {
        // Move any remaining looked-at cards to discard
        let remaining_count = self.looked_at_cards.len();
        if remaining_count > 0 {
            let cards_to_move: Vec<i16> = self.looked_at_cards.drain(..)
                .map(|id| id.try_into().unwrap())
                .collect();
            for card_id in cards_to_move {
                player.waitroom.add_card(card_id);
            }
            eprintln!("Moved {} remaining looked-at cards to discard", remaining_count);
        }
        Ok(())
    }

    /// Move selected cards to hand
    fn move_from_selected_to_hand(
        &mut self,
        player: &mut Player,
        count: usize,
    ) -> Result<(), String> {
        if self.selected_cards.is_empty() {
            return Err("No selected cards available".to_string());
        }

        // Move specified count of selected cards to hand
        let cards_to_move: Vec<i16> = self.selected_cards.drain(..)
            .take(count)
            .map(|id| id.try_into().unwrap())
            .collect();
        for card_id in cards_to_move {
            player.hand.add_card(card_id);
        }
        Ok(())
    }

    /// Execute conditional alternative effect
    fn execute_conditional_alternative(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        // Check if alternative condition is met
        let use_alternative = if let Some(ref alt_condition) = effect.alternative_condition {
            self.evaluate_condition(alt_condition, player, game_state)
        } else {
            false
        };

        if use_alternative {
            // Execute alternative effect
            if let Some(ref alt_effect) = effect.alternative_effect {
                self.execute_effect(alt_effect, player, game_state, perspective_player_id)?;
            }
        } else {
            // Execute primary effect
            if let Some(ref primary_effect) = effect.primary_effect {
                self.execute_effect(primary_effect, player, game_state, perspective_player_id)?;
            }
        }

        Ok(())
    }

    /// Execute look_and_select effect
    pub fn execute_look_and_select(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        // Execute look action first
        if let Some(ref look_action) = effect.look_action {
            let looked_cards = self.execute_look_at(look_action, player)?;
            // Store looked at cards for later selection
            self.looked_at_cards = looked_cards.into_iter().map(|id| id as u32).collect();
            eprintln!("Looked at {} cards and stored for selection", self.looked_at_cards.len());
        }
        
        // Execute select_action (which should be sequential)
        if let Some(ref select_action) = effect.select_action {
            self.execute_sequential(select_action, player, game_state, perspective_player_id)
        } else {
            Ok(())
        }
    }

    /// Execute choice effect
    pub fn execute_choice(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        if let Some(ref options) = effect.options {
            if options.is_empty() {
                return Err("Choice effect has no options".to_string());
            }
            
            // For now, just log the options and select the first one
            // In a full implementation, this would require UI integration
            eprintln!("Choice effect with {} options:", options.len());
            for (i, option) in options.iter().enumerate() {
                eprintln!("  Option {}: {}", i + 1, option.text);
            }
            
            // Execute the first option's effect
            if let Some(ref first_option) = options.first() {
                eprintln!("Executing option 1: {}", first_option.text);
                // Execute the option's nested effect if present
                if let Err(e) = self.execute_effect(first_option, player, game_state, perspective_player_id) {
                    eprintln!("Failed to execute option effect: {}", e);
                }
            }
            
            Ok(())
        } else {
            Err("Choice effect has no options".to_string())
        }
    }

    /// Execute change_state effect
    pub fn execute_change_state(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let new_state = effect.text.as_str();
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Apply state change to target cards
        match target {
            "self" => {
                if let Some(card_id) = game_state.activating_card {
                    // Track the state change in prohibition_effects
                    game_state.prohibition_effects.push(format!("state_change:{}:{}", card_id, new_state));
                    eprintln!("Changed state of card {} to {}", card_id, new_state);
                }
            }
            "all_stage" => {
                for &card_id in player.stage.stage.iter().filter(|&&c| c != crate::constants::EMPTY_SLOT) {
                    game_state.prohibition_effects.push(format!("state_change:{}:{}", card_id, new_state));
                }
                eprintln!("Changed state of all stage cards to {}", new_state);
            }
            _ => {
                eprintln!("Change state for target '{}' not yet implemented", target);
            }
        }
        Ok(())
    }

    /// Execute choose_required_hearts effect
    pub fn execute_choose_required_hearts(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        
        // Store the heart choice request in prohibition_effects
        game_state.prohibition_effects.push(format!("choose_required_hearts:{}", count));
        
        // Create a pending choice for heart color selection
        let heart_options = vec!["赤".to_string(), "桃".to_string(), "緑".to_string(), 
                                  "青".to_string(), "黄".to_string(), "紫".to_string()];
        
        self.pending_choice = Some(Choice::SelectHeartColor {
            count: count.try_into().unwrap(),
            options: heart_options.clone(),
            description: format!("Choose {} heart color(s) for required hearts", count),
        });
        
        eprintln!("Choose required hearts: {} hearts (options: {:?})", count, heart_options);
        Err("Pending choice required: choose heart colors".to_string())
    }

    /// Execute pay_energy effect
    pub fn execute_pay_energy(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Move energy cards from energy zone to waitroom (payment)
        let cards_to_pay: Vec<i16> = player.energy_zone.cards.iter()
            .take(count as usize)
            .copied()
            .collect();
        
        for card_id in cards_to_pay {
            player.energy_zone.cards.retain(|c| *c != card_id);
            player.waitroom.add_card(card_id);
            eprintln!("Paid energy card {} from energy zone to waitroom", card_id);
        }
        
        // Track the payment
        game_state.prohibition_effects.push(format!("pay_energy:{}:{}:{}", target, count, player.energy_zone.cards.len()));
        
        eprintln!("Pay energy: {} energy card(s) paid from energy zone", count);
        Ok(())
    }

    /// Execute play_baton_touch effect
    pub fn execute_play_baton_touch(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let reduction = effect.value.unwrap_or(0);
        let card_type = effect.card_type.as_deref().unwrap_or("all");
        let duration = effect.duration.as_deref().unwrap_or("this_turn");
        
        // Store baton touch cost reduction effect
        game_state.prohibition_effects.push(format!("baton_touch_reduction:{}:{}:{}", reduction, card_type, duration));
        
        eprintln!("Play baton touch: cost reduced by {} for {} (duration: {})", reduction, card_type, duration);
        Ok(())
    }

    /// Execute gain_ability effect
    pub fn execute_gain_ability(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let ability_text = effect.ability_gain.as_ref().ok_or("No ability text specified")?;
        let count = effect.count.unwrap_or(1);
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type = effect.card_type.as_deref();
        
        // Grant ability to cards on stage
        let mut granted = 0;
        let card_ids_to_grant: Vec<i16> = player.stage.stage.iter()
            .filter(|&&c| c != -1)
            .take(count as usize)
            .copied()
            .collect();
        
        for card_id in card_ids_to_grant {
            // Check card type filter if specified
            if let Some(ct) = card_type {
                if let Some(card) = game_state.card_database.get_card(card_id) {
                    let matches = match ct {
                        "member_card" => card.is_member(),
                        "live_card" => card.is_live(),
                        _ => true,
                    };
                    if !matches {
                        continue;
                    }
                }
            }
            
            // Store the granted ability using temporary effects
            // Use effect_data to store the card_id
            let duration = match effect.duration.as_deref() {
                Some("live_end") | Some("ライブ終了時まで") => crate::game_state::Duration::LiveEnd,
                Some("this_turn") | Some("このターンの間") => crate::game_state::Duration::ThisTurn,
                Some("this_live") | Some("このライブの間") => crate::game_state::Duration::ThisLive,
                _ => crate::game_state::Duration::LiveEnd,
            };
            
            game_state.temporary_effects.push(crate::game_state::TemporaryEffect {
                effect_type: "granted_ability".to_string(),
                duration,
                created_turn: game_state.turn_number,
                created_phase: game_state.current_phase.clone(),
                target_player_id: target.to_string(),
                description: format!("Card {} gained: {}", card_id, ability_text),
                creation_order: game_state.effect_creation_counter,
                effect_data: Some(serde_json::json!({"card_id": card_id, "ability": ability_text})),
            });
            eprintln!("Granted ability to card {}: {}", card_id, ability_text);
            granted += 1;
        }
        
        game_state.effect_creation_counter += 1;
        eprintln!("Gain ability effect: granted to {} cards (requested {}): {}", granted, count, ability_text);
        Ok(())
    }

    /// Execute select effect
    pub fn execute_select(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let card_type = effect.card_type.as_deref();
        let count = effect.count.unwrap_or(1);
        let source = effect.source.as_deref().unwrap_or("hand");
        
        // Determine zone and available cards
        let (zone_name, available_count) = match source {
            "hand" => ("hand", player.hand.cards.len()),
            "deck" => ("deck", player.main_deck.cards.len()),
            "discard" | "waitroom" => ("discard", player.waitroom.cards.len()),
            "stage" => ("stage", player.stage.stage.iter().filter(|&&c| c != -1).count()),
            "energy_zone" => ("energy_zone", player.energy_zone.cards.len()),
            _ => (source, 0),
        };
        
        if available_count == 0 {
            return Err(format!("No cards available in {} to select", zone_name));
        }
        
        // If fewer cards than needed, select all
        let select_count = count.min(available_count as u32) as usize;
        
        // If more cards than needed, create choice
        if available_count > select_count {
            let card_type_desc = card_type.unwrap_or("");
            let description = format!(
                "Select {} {} card(s) from {} ({} available)",
                select_count, card_type_desc, zone_name, available_count
            );
            
            self.pending_choice = Some(Choice::SelectCard {
                zone: zone_name.to_string(),
                card_type: card_type.map(|s| s.to_string()),
                count: select_count,
                description,
            });
            
            return Err(format!("Pending choice required: select cards from {}", zone_name));
        }
        
        // If exact number, select automatically
        eprintln!("Auto-selected {} cards from {} (exact match)", select_count, zone_name);
        Ok(())
    }

    /// Execute activation_cost effect
    pub fn execute_activation_cost(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("reduce");
        let value = effect.value.unwrap_or(1) as i32;
        let card_type = effect.card_type.as_deref();
        
        // Apply to all cards of specified type in hand (activation costs apply to playing cards)
        let card_ids: Vec<i16> = player.hand.cards.iter().copied().collect();
        let mut modified = 0;
        
        for card_id in card_ids {
            if let Some(ct) = card_type {
                if let Some(card) = game_state.card_database.get_card(card_id) {
                    let matches = match ct {
                        "member_card" => card.is_member(),
                        "live_card" => card.is_live(),
                        "energy_card" => card.is_energy(),
                        _ => true,
                    };
                    if matches {
                        let current = game_state.cost_modifiers.get(&card_id).copied().unwrap_or(0);
                        let new_value = match operation {
                            "reduce" | "subtract" => current - value,
                            "add" => current + value,
                            "set" => value,
                            _ => current - value,
                        };
                        game_state.cost_modifiers.insert(card_id, new_value);
                        eprintln!("Modified activation cost of card {}: {} -> {} ({})", card_id, current, new_value, operation);
                        modified += 1;
                    }
                }
            }
        }
        
        eprintln!("Activation cost effect: modified {} cards", modified);
        Ok(())
    }

    /// Execute shuffle effect
    pub fn execute_shuffle(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        _game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        match target {
            "deck" => {
                player.main_deck.shuffle();
                eprintln!("Shuffled deck");
            }
            "discard" | "waitroom" => {
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                player.waitroom.cards.shuffle(&mut rng);
                eprintln!("Shuffled waitroom");
            }
            _ => {
                return Err(format!("Cannot shuffle target: {}", target));
            }
        }
        Ok(())
    }

    /// Execute draw_until_count effect
    pub fn execute_draw_until_count(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        _game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let target_count = effect.target_count.unwrap_or(0) as usize;
        while player.hand.cards.len() < target_count {
            if let Some(card) = player.main_deck.draw() {
                player.hand.add_card(card);
            } else {
                return Err("Deck ran out of cards".to_string());
            }
        }
        eprintln!("Drew cards until hand has {} cards", target_count);
        Ok(())
    }

    /// Execute appear effect
    pub fn execute_appear(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        _game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let card_type = effect.card_type.as_ref().ok_or("No card type specified")?;
        let count = effect.count.unwrap_or(1);
        
        // Appear cards - likely from deck to stage
        for _ in 0..count {
            if let Some(card) = player.main_deck.draw() {
                // Find empty stage position
                let empty_pos = player.stage.stage.iter().position(|&c| c == -1);
                if let Some(pos) = empty_pos {
                    player.stage.stage[pos] = card;
                    eprintln!("Appeared {} at stage position {}", card, pos);
                } else {
                    return Err("No empty stage positions available".to_string());
                }
            } else {
                return Err("Deck ran out of cards".to_string());
            }
        }
        Ok(())
    }

    /// Execute modify_cost effect
    pub fn execute_modify_cost(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let value = effect.value.unwrap_or(0) as i32;
        let card_type = effect.card_type.as_deref();
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Apply cost modification to cards of specified type
        if let Some(ct) = card_type {
            match target {
                "self" => {
                    // Apply to all cards of type in hand
                    let card_ids: Vec<i16> = _player.hand.cards.iter().copied().collect();
                    for card_id in card_ids {
                        if let Some(card) = game_state.card_database.get_card(card_id) {
                            if (ct == "member_card" && card.is_member()) ||
                               (ct == "live_card" && card.is_live()) ||
                               (ct == "energy_card" && card.is_energy()) {
                                let current = game_state.cost_modifiers.get(&card_id).copied().unwrap_or(0);
                                let new_value = match operation {
                                    "add" => current + value,
                                    "subtract" => current - value,
                                    "set" => value,
                                    _ => current + value,
                                };
                                game_state.cost_modifiers.insert(card_id, new_value);
                                eprintln!("Modified cost of card {}: {} -> {} ({})", card_id, current, new_value, operation);
                            }
                        }
                    }
                }
                _ => {
                    eprintln!("Cost modification for target '{}' not yet implemented", target);
                }
            }
        } else {
            eprintln!("Cost modification requires card_type to be specified");
        }
        Ok(())
    }

    /// Execute set_score effect
    pub fn execute_set_score(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        _game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let value = effect.value.unwrap_or(0) as i32;
        let target = effect.target.as_deref().unwrap_or("self");

        match target {
            "self" => {
                player.live_score = value;
                player.has_live_score = true;
                eprintln!("Set player score to {}", value);
            }
            "opponent" => {
                // For now, just log - opponent tracking would need player ID resolution
                eprintln!("Set opponent score to {} (not fully implemented)", value);
            }
            _ => {
                return Err(format!("Invalid score target: {}", target));
            }
        }
        Ok(())
    }

    /// Execute set_cost effect
    pub fn execute_set_cost(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let value = effect.value.unwrap_or(0) as i32;
        let card_type = effect.card_type.as_deref();
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Apply to all cards of specified type in hand
        let card_ids: Vec<i16> = player.hand.cards.iter().copied().collect();
        let mut modified = 0;
        
        for card_id in card_ids {
            if let Some(ct) = card_type {
                if let Some(card) = game_state.card_database.get_card(card_id) {
                    let matches = match ct {
                        "member_card" => card.is_member(),
                        "live_card" => card.is_live(),
                        "energy_card" => card.is_energy(),
                        _ => true,
                    };
                    if matches {
                        // Calculate delta to reach target cost
                        let base_cost = card.cost.map(|c| c as i32).unwrap_or(0);
                        let delta = value - base_cost;
                        game_state.cost_modifiers.insert(card_id, delta);
                        eprintln!("Set cost of card {} to {} (base: {}, delta: {})", card_id, value, base_cost, delta);
                        modified += 1;
                    }
                }
            } else {
                // No card type filter, apply to all
                if let Some(card) = game_state.card_database.get_card(card_id) {
                    let base_cost = card.cost.map(|c| c as i32).unwrap_or(0);
                    let delta = value - base_cost;
                    game_state.cost_modifiers.insert(card_id, delta);
                    eprintln!("Set cost of card {} to {} (base: {}, delta: {})", card_id, value, base_cost, delta);
                    modified += 1;
                }
            }
        }
        
        eprintln!("Set cost effect: modified {} cards to cost {}", modified, value);
        Ok(())
    }

    /// Execute set_blade_count effect
    pub fn execute_set_blade_count(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        _game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1) as usize;
        let target = effect.target.as_deref().unwrap_or("self");

        match target {
            "self" => {
                player.blade = count;
                eprintln!("Set player blade count to {}", count);
            }
            "opponent" => {
                eprintln!("Set opponent blade count to {} (not fully implemented)", count);
            }
            _ => {
                return Err(format!("Invalid blade count target: {}", target));
            }
        }
        Ok(())
    }

    /// Execute modify_limit effect
    pub fn execute_modify_limit(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("set");
        let value = effect.value.unwrap_or(0) as i32;
        let limit_type = effect.card_type.as_deref().unwrap_or("general");
        
        // Store limit modifications in game_state
        // For now, log and store in prohibition_effects as a workaround
        let limit_desc = format!("limit_{}:{}_{}", limit_type, operation, value);
        game_state.prohibition_effects.push(limit_desc.clone());
        eprintln!("Modified {} limit: {} {} (stored in prohibition_effects)", limit_type, operation, value);
        Ok(())
    }

    /// Execute invalidate_ability effect
    pub fn execute_invalidate_ability(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        let card_type = effect.card_type.as_deref();
        
        // Invalidate abilities on cards on stage
        let mut invalidated = 0;
        for i in 0..3 {
            let card_id = player.stage.stage[i];
            if card_id != -1 {
                // Check card type filter if specified
                if let Some(ct) = card_type {
                    if let Some(card) = game_state.card_database.get_card(card_id) {
                        let matches = match ct {
                            "member_card" => card.is_member(),
                            "live_card" => card.is_live(),
                            _ => true,
                        };
                        if !matches {
                            continue;
                        }
                    }
                }
                
                player.invalidated_abilities.insert(card_id);
                eprintln!("Invalidated abilities for card {}", card_id);
                invalidated += 1;
            }
        }
        
        eprintln!("Invalidate ability effect: invalidated {} cards on {}", invalidated, target);
        Ok(())
    }

    /// Execute choose_heart_type effect
    pub fn execute_choose_heart_type(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        
        // Store heart type choice in prohibition_effects
        game_state.prohibition_effects.push(format!("choose_heart_type:{}", count));
        
        // Create a pending choice for heart type selection
        let heart_options = vec!["heart00".to_string(), "heart01".to_string(), "heart02".to_string(),
                                  "heart10".to_string(), "heart11".to_string(), "heart12".to_string()];
        
        self.pending_choice = Some(Choice::SelectHeartType {
            count: count.try_into().unwrap(),
            options: heart_options.clone(),
            description: format!("Choose {} heart type(s)", count),
        });
        
        eprintln!("Choose heart type: {} types (options: {:?})", count, heart_options);
        Err("Pending choice required: choose heart types".to_string())
    }

    /// Execute modify_required_hearts_success effect
    pub fn execute_modify_required_hearts_success(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("set");
        let value = effect.value.unwrap_or(0);
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        
        // Store the heart requirement modification in prohibition_effects for tracking
        // This affects how many hearts are required for live success (Rule 8.3.14)
        let desc = format!("modify_required_hearts_success:{}:{}:{}", operation, value, heart_color);
        game_state.prohibition_effects.push(desc);
        
        eprintln!("Modify required hearts success: {} {} hearts of color {} required for success", operation, value, heart_color);
        Ok(())
    }

    /// Execute set_cost_to_use effect
    pub fn execute_set_cost_to_use(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let value = effect.value.unwrap_or(0) as i32;
        let target_card_type = effect.card_type.as_deref();
        let duration = effect.duration.as_deref().unwrap_or("live_end");
        
        // Store the cost to use modification
        let desc = format!("cost_to_use:{}:{}:{}", value, target_card_type.unwrap_or("all"), duration);
        game_state.prohibition_effects.push(desc);
        eprintln!("Set cost to use: {} for card type {:?} (duration: {})", value, target_card_type, duration);
        Ok(())
    }

    /// Execute all_blade_timing effect
    pub fn execute_all_blade_timing(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let timing = effect.timing.as_deref().unwrap_or("check_required_hearts");
        let treat_as = effect.treat_as.as_deref().unwrap_or("any_heart_color");
        let duration = effect.duration.as_deref().unwrap_or("live_end");
        
        // Store the blade timing effect
        let desc = format!("all_blade_timing:{}:{}:{}", timing, treat_as, duration);
        game_state.prohibition_effects.push(desc);
        eprintln!("All blade timing: treat as {} during {} (duration: {})", treat_as, timing, duration);
        Ok(())
    }

    /// Execute set_card_identity_all_regions effect
    pub fn execute_set_card_identity_all_regions(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Store identity modification for all regions
        if let Some(ref group) = effect.group {
            let desc = format!("identity_all_regions:{}:group={}", target, group.name);
            game_state.prohibition_effects.push(desc);
            eprintln!("Set card identity all regions: {} as {}", target, group.name);
        } else {
            let desc = format!("identity_all_regions:{}", target);
            game_state.prohibition_effects.push(desc);
            eprintln!("Set card identity all regions: {}", target);
        }
        Ok(())
    }

    /// Execute set_card_identity effect
    pub fn execute_set_card_identity(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Store identity modification
        if let Some(ref group) = effect.group {
            let desc = format!("identity:{}:group={}", target, group.name);
            game_state.prohibition_effects.push(desc);
            eprintln!("Set card identity: {} as {}", target, group.name);
        } else {
            let desc = format!("identity:{}", target);
            game_state.prohibition_effects.push(desc);
            eprintln!("Set card identity: {}", target);
        }
        Ok(())
    }

    /// Execute re_yell effect
    pub fn execute_re_yell(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let lose_blade_hearts = effect.lose_blade_hearts.unwrap_or(false);
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Store re-yell tracking
        let desc = format!("re_yell:{}:{}:{}", target, count, lose_blade_hearts);
        game_state.prohibition_effects.push(desc);
        
        // If lose_blade_hearts is true, clear blade modifiers from stage cards
        if lose_blade_hearts {
            let stage_cards: Vec<i16> = player.stage.stage.iter().filter(|&&c| c != -1).copied().collect();
            for card_id in stage_cards {
                game_state.blade_modifiers.remove(&card_id);
                eprintln!("Re-yell: cleared blade modifier for card {}", card_id);
            }
        }
        
        eprintln!("Re-yell: {} times for {} (lose_blade_hearts: {})", count, target, lose_blade_hearts);
        Ok(())
    }

    /// Execute restriction effect
    pub fn execute_restriction(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let restriction_type = effect.restriction_type.as_deref().unwrap_or("");
        let restricted_destination = effect.restricted_destination.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Store restriction in prohibition_effects
        let restriction = if !restricted_destination.is_empty() {
            format!("restriction:{}:{}:{}", restriction_type, restricted_destination, target)
        } else {
            format!("restriction:{}:{}", restriction_type, target)
        };
        game_state.prohibition_effects.push(restriction);
        
        eprintln!("Restriction applied: type={}, destination={}, target={}", restriction_type, restricted_destination, target);
        Ok(())
    }

    /// Execute activation_restriction effect
    pub fn execute_activation_restriction(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let restriction_text = effect.text.clone();
        let target = effect.target.as_deref().unwrap_or("self");
        
        // Store activation restriction
        let restriction = format!("activation_restriction:{}:{}", target, restriction_text);
        game_state.prohibition_effects.push(restriction);
        
        eprintln!("Activation restriction: {} for {}", restriction_text, target);
        Ok(())
    }

    /// Execute specify_heart_color effect
    pub fn execute_specify_heart_color(
        &mut self,
        effect: &AbilityEffect,
        _player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let duration = effect.duration.as_deref().unwrap_or("live_end");
        
        // Store heart color specification in temporary effects
        let heart_color = effect.heart_color.as_deref().unwrap_or("heart00");
        let desc = format!("specify_heart_color:{}:{}:{}", heart_color, count, duration);
        game_state.prohibition_effects.push(desc);
        
        eprintln!("Specify heart color: {} (count: {}, duration: {})", heart_color, count, duration);
        Ok(())
    }

    /// Execute reveal effect
    pub fn execute_reveal(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let source = effect.source.as_deref().unwrap_or("hand");
        let card_type = effect.card_type.as_deref();
        
        // Reveal cards from the specified zone
        let card_ids: Vec<i16> = match source {
            "hand" => player.hand.cards.iter().take(count as usize).copied().collect(),
            "deck" => player.main_deck.cards.iter().take(count as usize).copied().collect(),
            "discard" => player.waitroom.cards.iter().take(count as usize).copied().collect(),
            _ => vec![],
        };
        
        for card_id in card_ids {
            game_state.revealed_cards.insert(card_id);
            eprintln!("Revealed card {} from {}", card_id, source);
        }
        
        eprintln!("Reveal effect: {} cards from {} (type: {:?})", count, source, card_type);
        Ok(())
    }
}
