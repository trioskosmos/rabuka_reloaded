use crate::card::{Ability, AbilityCost, AbilityEffect, Condition};
use crate::game_state::GameState;
use crate::player::Player;
use crate::zones::MemberArea;

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
}

#[derive(Debug, Clone)]
pub enum ChoiceResult {
    CardSelected { indices: Vec<usize> },
    TargetSelected { target: String },
    PositionSelected { position: String },
}

#[derive(Debug, Clone)]
pub struct AbilityExecutor {
    pending_choice: Option<Choice>,
}

impl AbilityExecutor {
    pub fn new() -> Self {
        Self {
            pending_choice: None,
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
            "stage" | "ステージ" => {
                // Check if player has a member on stage
                if card_type == "member_card" || card_type == "メンバー" {
                    let count = (player.stage.stage[0] != -1) as usize +
                                   (player.stage.stage[1] != -1) as usize +
                                   (player.stage.stage[2] != -1) as usize;
                    count >= count_needed
                } else {
                    false
                }
            }
            "hand" | "手札" => {
                let card_db = &game_state.card_database;
                if card_type == "member_card" || card_type == "メンバー" {
                    player.hand.cards.iter().filter(|&id| {
                        card_db.get_card(*id).map_or(false, |c| c.is_member())
                    }).count() >= count_needed
                } else if card_type == "live_card" || card_type == "ライブ" {
                    player.hand.cards.iter().filter(|&id| {
                        card_db.get_card(*id).map_or(false, |c| c.is_live())
                    }).count() >= count_needed
                } else {
                    player.hand.cards.len() >= count_needed
                }
            }
            "discard" | "控え室" => {
                let card_db = &game_state.card_database;
                if card_type == "member_card" || card_type == "メンバー" {
                    player.waitroom.cards.iter().filter(|&id| {
                        card_db.get_card(*id).map_or(false, |c| c.is_member())
                    }).count() >= count_needed
                } else if card_type == "live_card" || card_type == "ライブ" {
                    player.waitroom.cards.iter().filter(|&id| {
                        card_db.get_card(*id).map_or(false, |c| c.is_live())
                    }).count() >= count_needed
                } else {
                    player.waitroom.cards.len() >= count_needed
                }
            }
            "deck" | "デッキ" => {
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
            "stage" | "ステージ" => {
                if card_type == "member_card" || card_type == "メンバー" {
                    player.stage.stage[0] != -1
                        || player.stage.stage[1] != -1
                        || player.stage.stage[2] != -1
                } else {
                    false
                }
            }
            "hand" | "手札" => {
                let card_db = &game_state.card_database;
                if card_type == "member_card" || card_type == "メンバー" {
                    player.hand.cards.iter().map(|&id| {
                        card_db.get_card(id).map_or(false, |c| c.is_member())
                    }).any(|x| x)
                } else if card_type == "live_card" || card_type == "ライブ" {
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
            "active" | "アクティブ" => player.count_active_energy() > 0,
            "wait" | "ウェイト" => player.count_wait_energy() > 0,
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
            let cost_text = ability.cost.as_ref().map(|c| c.text.clone()).unwrap_or_default();
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
                ("discard" | "控え室", "hand" | "手札") => {
                    self.move_from_discard_to_hand(player, count_usize, card_type, game_state)?;
                }
                ("stage" | "ステージ", "discard" | "控え室") => {
                    self.move_from_stage_to_discard(player, false, false, game_state)?;
                }
                ("hand" | "手札", "discard" | "控え室") => {
                    self.move_from_hand_to_discard(player, count_usize)?;
                }
                ("deck" | "デッキ", "hand" | "手札") => {
                    self.draw_cards(player, count_usize)?;
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
                        "member_card" | "メンバー" => card.is_member(),
                        "live_card" | "ライブ" => card.is_live(),
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
                    "member_card" | "メンバー" => card.is_member(),
                    "live_card" | "ライブ" => card.is_live(),
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
                            _ => crate::game_state::Duration::ThisLive,
                        }).unwrap_or(crate::game_state::Duration::ThisLive),
                        created_turn: current_turn,
                        created_phase: current_phase.clone(),
                        target_player_id: target_player.id.clone(),
                        description: format!("Set blade type to {} for {}", blade_type, card_db.get_card(card_id).map(|c| c.name.as_str()).unwrap_or("unknown")),
                        creation_order: 0,
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
        let heart_type = effect.heart_color.as_deref().unwrap_or("heart00");
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
        let position = effect.position.as_ref().and_then(|p| p.position.as_deref()).unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Move member to specified position
            let _target_area = match position {
                "center" | "センターエリア" => crate::zones::MemberArea::Center,
                "left_side" | "左サイドエリア" => crate::zones::MemberArea::LeftSide,
                "right_side" | "右サイドエリア" => crate::zones::MemberArea::RightSide,
                _ => return Err(format!("Unknown position: {}", position)),
            };

            // Find and move the member (simplified - assumes moving from current position to target)
            // This is a complex operation that requires user choice in real gameplay
            // For now, we'll just log the intent
            println!("Position change requested: move member to {}", position);
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
                            // This requires tracking which member activated - simplified for now
                            // Just add to energy zone for now
                            target_player.energy_zone.cards.push(energy_card);
                        }
                        _ => {
                            // Place under specified member
                            target_player.energy_zone.cards.push(energy_card);
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
            (Some(Choice::SelectCard { .. }), ChoiceResult::CardSelected { .. }) => {
                self.pending_choice = None;
                Ok(())
            }
            (Some(Choice::SelectTarget { .. }), ChoiceResult::TargetSelected { .. }) => {
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

        for sub_effect in actions {
            match sub_effect.action.as_str() {
                "draw" => {
                    self.execute_draw(sub_effect, player)?;
                }
                "move_cards" => {
                    self.execute_move_cards(sub_effect, player, game_state, perspective_player_id)?;
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
            "move_cards" => {
                self.execute_move_cards(effect, player, game_state, perspective_player_id)
            }
            "draw" => self.execute_draw(effect, player),
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
            _ => Err(format!("Unknown effect action: {}", effect.action)),
        }
    }

    /// Execute ability cost
    pub fn execute_cost(
        &mut self,
        cost: &AbilityCost,
        player: &mut Player,
        game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        match cost.cost_type.as_deref() {
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
                let position = cost.position.as_ref().and_then(|p| p.position.as_deref());

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
                // For now, skip optional costs
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
}
