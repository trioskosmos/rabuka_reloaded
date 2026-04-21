use crate::card::{Ability, AbilityCost, AbilityEffect, Card, Condition};
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
        _game_state: &GameState,
    ) -> CostCalculation {
        // Check if source zone has the required card
        let source = cost.source.as_deref().unwrap_or("");
        let card_type = cost.card_type.as_deref().unwrap_or("");

        let has_card = match source {
            "stage" | "ステージ" => {
                // Check if player has a member on stage
                if card_type == "member_card" || card_type == "メンバー" {
                    player.stage.left_side.is_some()
                        || player.stage.center.is_some()
                        || player.stage.right_side.is_some()
                } else {
                    false
                }
            }
            "hand" | "手札" => {
                if card_type == "member_card" || card_type == "メンバー" {
                    player.hand.cards.iter().any(|c| c.is_member())
                } else if card_type == "live_card" || card_type == "ライブ" {
                    player.hand.cards.iter().any(|c| c.is_live())
                } else {
                    !player.hand.is_empty()
                }
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
                    "No {} card in {}",
                    card_type,
                    source
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
                let has_active = player.stage.left_side.as_ref()
                    .map_or(false, |c| c.orientation == Some(crate::zones::Orientation::Active))
                    || player.stage.center.as_ref()
                        .map_or(false, |c| c.orientation == Some(crate::zones::Orientation::Active))
                    || player.stage.right_side.as_ref()
                        .map_or(false, |c| c.orientation == Some(crate::zones::Orientation::Active));

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
            Some("character_presence_condition") => self.evaluate_character_presence(condition, player),
            Some("group_presence_condition") => self.evaluate_group_presence(condition, player),
            Some("energy_state_condition") => self.evaluate_energy_state(condition, player),
            _ => true, // Unknown condition types default to true for now
        }
    }

    fn evaluate_location_condition(
        &self,
        condition: &Condition,
        player: &Player,
        _game_state: &GameState,
    ) -> bool {
        let location = condition.location.as_deref().unwrap_or("");
        let card_type = condition.card_type.as_deref().unwrap_or("");

        match location {
            "stage" | "ステージ" => {
                if card_type == "member_card" || card_type == "メンバー" {
                    player.stage.left_side.is_some()
                        || player.stage.center.is_some()
                        || player.stage.right_side.is_some()
                } else {
                    false
                }
            }
            "hand" | "手札" => {
                if card_type == "member_card" || card_type == "メンバー" {
                    player.hand.cards.iter().any(|c| c.is_member())
                } else if card_type == "live_card" || card_type == "ライブ" {
                    player.hand.cards.iter().any(|c| c.is_live())
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

    fn evaluate_character_presence(&self, condition: &Condition, player: &Player) -> bool {
        if let Some(ref characters) = condition.characters {
            if characters.is_empty() {
                return true;
            }
            // Check if ANY of the characters are present (OR logic)
            characters.iter().any(|name| player.has_character_on_stage(name))
        } else {
            true
        }
    }

    fn evaluate_group_presence(&self, condition: &Condition, player: &Player) -> bool {
        if let Some(ref group) = condition.group {
            // Convert serde_json::Value to string
            let group_str = group.as_str().unwrap_or("");
            player.has_group_on_stage(group_str)
        } else if let Some(ref group_names) = condition.group_names {
            if group_names.is_empty() {
                return true;
            }
            // Check if ANY of the groups are present (OR logic)
            group_names.iter().any(|name| player.has_group_on_stage(name))
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

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Execute move based on source and destination
            let count_usize: usize = count.try_into().unwrap_or(usize::MAX);
            match (source, destination) {
                ("discard" | "控え室", "hand" | "手札") => {
                    self.move_from_discard_to_hand(player, count_usize, card_type)?;
                }
                ("stage" | "ステージ", "discard" | "控え室") => {
                    self.move_from_stage_to_discard(player)?;
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
        &self,
        player: &mut Player,
        count: usize,
        card_type: &str,
    ) -> Result<(), String> {
        let mut moved = 0;
        let mut indices_to_remove = Vec::new();

        for (i, card) in player.waitroom.cards.iter().enumerate() {
            if moved >= count {
                break;
            }

            let matches_type = match card_type {
                "member_card" | "メンバー" => card.is_member(),
                "live_card" | "ライブ" => card.is_live(),
                _ => true,
            };

            if matches_type {
                indices_to_remove.push(i);
                player.hand.add_card(card.clone());
                moved += 1;
            }
        }

        // Remove cards from waitroom (in reverse order to maintain indices)
        for i in indices_to_remove.into_iter().rev() {
            player.waitroom.cards.remove(i);
        }

        if moved < count {
            return Err(format!(
                "Not enough cards in discard: needed {}, moved {}",
                count, moved
            ));
        }

        Ok(())
    }

    fn move_from_stage_to_discard(&self, player: &mut Player) -> Result<(), String> {
        // This is a cost - move the activating member to discard
        // For now, just remove all members (simplified)
        if let Some(card) = player.stage.left_side.take() {
            player.waitroom.add_card(card.card);
        }
        if let Some(card) = player.stage.center.take() {
            player.waitroom.add_card(card.card);
        }
        if let Some(card) = player.stage.right_side.take() {
            player.waitroom.add_card(card.card);
        }
        Ok(())
    }

    fn move_from_hand_to_discard(&self, player: &mut Player, count: usize) -> Result<(), String> {
        // This requires user choice - for now, discard first count cards
        let cards_to_remove: Vec<_> = player.hand.cards.iter().take(count).cloned().collect();
        for card in cards_to_remove {
            player.waitroom.add_card(card);
        }
        for _ in 0..count.min(player.hand.cards.len()) {
            player.hand.cards.remove(0);
        }
        Ok(())
    }

    fn draw_cards(&self, player: &mut Player, count: usize) -> Result<(), String> {
        for _ in 0..count {
            if let Some(card) = player.main_deck.draw() {
                player.hand.add_card(card);
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

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Add resource to all stage members based on type
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(ref mut card) = target_player.stage.get_area_mut(area) {
                    match resource {
                        "blade" | "ブレード" => {
                            card.card.add_blades(count);
                        }
                        "heart" | "ハート" => {
                            // If heart_color is specified, add that specific color
                            if let Some(ref heart_color) = effect.heart_color {
                                card.card.add_heart(heart_color, count);
                            } else {
                                // Default to heart00 (wildcard) if no color specified
                                card.card.add_heart("heart00", count);
                            }
                        }
                        _ => {}
                    }
                }
            }
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

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Modify score for live cards in live card zone
            for card in &mut target_player.live_card_zone.cards {
                match operation {
                    "add" => card.add_score(value),
                    "remove" => card.remove_score(value),
                    "set" => card.set_score(value),
                    _ => return Err(format!("Unknown operation: {}", operation)),
                }
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

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Modify required hearts for live cards
            for card in &mut target_player.live_card_zone.cards {
                if let Some(ref mut need_heart) = card.need_heart {
                    match operation {
                        "decrease" => {
                            let current = need_heart.hearts.get(heart_color).copied().unwrap_or(0);
                            if current <= value {
                                need_heart.hearts.remove(heart_color);
                            } else {
                                need_heart.hearts.insert(heart_color.to_string(), current - value);
                            }
                        }
                        "increase" => {
                            *need_heart.hearts.entry(heart_color.to_string()).or_insert(0) += value;
                        }
                        "set" => {
                            need_heart.hearts.insert(heart_color.to_string(), value);
                        }
                        _ => return Err(format!("Unknown operation: {}", operation)),
                    }
                }
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

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Set required hearts for live cards
            for card in &mut target_player.live_card_zone.cards {
                if card.need_heart.is_none() {
                    card.need_heart = Some(crate::card::BaseHeart {
                        hearts: std::collections::HashMap::new(),
                    });
                }
                if let Some(ref mut need_heart) = card.need_heart {
                    need_heart.hearts.insert(heart_color.to_string(), count);
                }
            }
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

        for target_player in target_players {
            // Modify required hearts for all live cards
            for card in &mut target_player.live_card_zone.cards {
                if let Some(ref mut need_heart) = card.need_heart {
                    match operation {
                        "increase" => {
                            *need_heart.hearts.entry(heart_color.to_string()).or_insert(0) += value;
                        }
                        "decrease" => {
                            let current = need_heart.hearts.get(heart_color).copied().unwrap_or(0);
                            if current <= value {
                                need_heart.hearts.remove(heart_color);
                            } else {
                                need_heart.hearts.insert(heart_color.to_string(), current - value);
                            }
                        }
                        _ => return Err(format!("Unknown operation: {}", operation)),
                    }
                }
            }
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

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);
        
        // Collect temporary effects first to avoid borrow conflicts
        let mut temp_effects = Vec::new();

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Set blade type for stage members
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(ref mut card) = target_player.stage.get_area_mut(area) {
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
                        description: format!("Set blade type to {} for {}", blade_type, card.card.name),
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

        let target_players = game_state.resolve_target_mut(target, perspective_player_id);

        for target_player in target_players {
            if target_player.id != player.id {
                continue; // Skip for now, only implement self target
            }

            // Set heart type for stage members
            let areas = [crate::zones::MemberArea::LeftSide, crate::zones::MemberArea::Center, crate::zones::MemberArea::RightSide];
            for area in areas {
                if let Some(ref mut card) = target_player.stage.get_area_mut(area) {
                    // Set heart type - replace base_heart with specified type
                    let count = effect.count.unwrap_or(1);
                    card.card.set_heart(heart_type, count);
                }
            }
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
                            target_player.energy_zone.cards.push(crate::zones::CardInZone {
                                card: energy_card,
                                orientation: Some(crate::zones::Orientation::Active),
                                energy_underneath: Vec::new(),
                                face_state: crate::zones::FaceState::FaceUp,
                                played_via_ability: false,
                                turn_played: 0,
                            });
                        }
                        _ => {
                            // Place under specified member
                            target_player.energy_zone.cards.push(crate::zones::CardInZone {
                                card: energy_card,
                                orientation: Some(crate::zones::Orientation::Active),
                                energy_underneath: Vec::new(),
                                face_state: crate::zones::FaceState::FaceUp,
                                played_via_ability: false,
                                turn_played: 0,
                            });
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
    pub fn execute_look_at<'a>(
        &mut self,
        effect: &AbilityEffect,
        player: &'a Player,
    ) -> Result<Vec<&'a Card>, String> {
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
        _game_state: &mut GameState,
        _perspective_player_id: &str,
    ) -> Result<(), String> {
        match cost.cost_type.as_deref() {
            Some("move_cards") => {
                let source = cost.source.as_deref().unwrap_or("");
                let destination = cost.destination.as_deref().unwrap_or("");

                match (source, destination) {
                    ("stage" | "ステージ", "discard" | "控え室") => {
                        self.move_from_stage_to_discard(player)?;
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
                let mut deactivated = 0;
                for card in &mut player.energy_zone.cards {
                    if deactivated >= energy_needed {
                        break;
                    }
                    if card.orientation == Some(crate::zones::Orientation::Active) {
                        card.orientation = Some(crate::zones::Orientation::Wait);
                        deactivated += 1;
                    }
                }

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
                    let area = match pos {
                        "center" | "センターエリア" => MemberArea::Center,
                        "left_side" | "左サイドエリア" => MemberArea::LeftSide,
                        "right_side" | "右サイドエリア" => MemberArea::RightSide,
                        _ => return Err(format!("Unknown position: {}", pos)),
                    };

                    let orientation = match state {
                        "wait" | "ウェイト" => crate::zones::Orientation::Wait,
                        "active" | "アクティブ" => crate::zones::Orientation::Active,
                        _ => return Err(format!("Unknown state: {}", state)),
                    };

                    player.stage.set_card_orientation(area, orientation)?;
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
    ) -> Result<(), String> {
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

        Ok(())
    }
}
