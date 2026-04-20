use crate::card::{Ability, AbilityCost, AbilityEffect, Condition};
use crate::game_state::GameState;

pub struct AbilityResolver {
    game_state: GameState,
}

impl AbilityResolver {
    pub fn new(game_state: GameState) -> Self {
        AbilityResolver { game_state }
    }

    pub fn new_mut(game_state: &mut GameState) -> Self {
        // Clone the game state for now - this is a limitation
        // In a proper implementation, the resolver would hold a mutable reference
        // For now, we'll need to update the original game_state after execution
        AbilityResolver { game_state: game_state.clone() }
    }

    /// Evaluate a condition against the current game state
    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        match condition.condition_type.as_deref() {
            Some("compound") => self.evaluate_compound_condition(condition),
            Some("comparison_condition") => self.evaluate_comparison_condition(condition),
            Some("location_condition") => self.evaluate_location_condition(condition),
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
        // Check if cards exist in specified location
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        
        let player = match target {
            "self" => &self.game_state.player1,
            "opponent" => &self.game_state.player2,
            _ => &self.game_state.player1,
        };

        match location {
            "stage" => player.stage.total_blades() > 0,
            "hand" => !player.hand.is_empty(),
            "deck" => !player.main_deck.is_empty(),
            "discard" => !player.waitroom.cards.is_empty(),
            "energy_zone" => !player.energy_zone.cards.is_empty(),
            "live_card_zone" => !player.live_card_zone.cards.is_empty(),
            "success_live_zone" => !player.success_live_card_zone.cards.is_empty(),
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
            "member_card" => player.stage.total_blades() as usize, // Approximate
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
        // Check if a card has appeared (this would need to track appearance events)
        // For now, return true as a placeholder
        condition.appearance.unwrap_or(false)
    }

    fn evaluate_temporal_condition(&self, condition: &Condition) -> bool {
        // Check temporal conditions (this turn, live_end, etc.)
        // This would need to track turn state
        match condition.temporal_scope.as_deref() {
            Some("this_turn") => true, // Placeholder
            _ => true,
        }
    }

    fn evaluate_state_condition(&self, condition: &Condition) -> bool {
        // Check card state (active, wait, face_up, face_down)
        let state = condition.state.as_deref().unwrap_or("");
        // This would need to check specific card states
        match state {
            "active" => true,
            "wait" => true,
            _ => true,
        }
    }

    fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        // Check energy card states
        let energy_state = condition.energy_state.as_deref().unwrap_or("");
        match energy_state {
            "active" => self.game_state.player1.energy_zone.active_count() > 0,
            _ => true,
        }
    }

    fn evaluate_movement_condition(&self, condition: &Condition) -> bool {
        // Check movement-related conditions
        condition.movement_condition.is_some()
    }

    fn evaluate_ability_negation_condition(&self, condition: &Condition) -> bool {
        // Check if abilities are negated
        !condition.negation.unwrap_or(false)
    }

    fn evaluate_or_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            conditions.iter().any(|c| self.evaluate_condition(c))
        } else {
            true
        }
    }

    fn evaluate_any_of_condition(&self, condition: &Condition) -> bool {
        if let Some(ref any_of) = condition.any_of {
            any_of.iter().any(|_| true) // Placeholder
        } else {
            true
        }
    }

    fn evaluate_score_threshold_condition(&self, _condition: &Condition) -> bool {
        // Check score thresholds
        true // Placeholder
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
            "stage" => player.stage.total_blades(),
            "hand" => player.hand.len() as u32,
            "deck" => player.main_deck.len() as u32,
            "discard" => player.waitroom.len() as u32,
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => player.live_card_zone.len() as u32,
            "success_live_zone" => player.success_live_card_zone.len() as u32,
            _ => 0,
        }
    }

    fn get_group_card_count(&self, _condition: &Condition) -> u32 {
        // Count cards of a specific group
        // This would need to iterate through cards and check their groups
        0 // Placeholder
    }

    /// Execute an ability effect
    pub fn execute_effect(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // First, check if there's a condition
        if let Some(ref condition) = effect.condition {
            if !self.evaluate_condition(condition) {
                return Ok(()); // Condition not met, skip effect
            }
        }

        match effect.action.as_str() {
            "sequential" => self.execute_sequential_effect(effect),
            "conditional_alternative" => self.execute_conditional_alternative(effect),
            "look_and_select" => self.execute_look_and_select(effect),
            "draw" => self.execute_draw(effect),
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

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        for _ in 0..count {
            if let Some(card) = player.main_deck.draw() {
                player.hand.add_card(card);
            }
        }

        Ok(())
    }

    fn execute_move_cards(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let count = effect.count.unwrap_or(1);
        let source = effect.source.as_deref().unwrap_or("");
        let destination = effect.destination.as_deref().unwrap_or("");
        let _card_type = effect.card_type.as_deref();
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        // This is a simplified implementation
        // A full implementation would need to handle card selection, filtering by type, etc.
        match source {
            "deck" => {
                for _ in 0..count {
                    if let Some(card) = player.main_deck.draw() {
                        match destination {
                            "hand" => player.hand.add_card(card),
                            "discard" => player.waitroom.add_card(card),
                            _ => {}
                        }
                    }
                }
            }
            "hand" => {
                match destination {
                    "discard" => {
                        for _ in 0..count {
                            player.hand.remove_card(0);
                        }
                    }
                    _ => {}
                }
            }
            "discard" => {
                match destination {
                    "hand" => {
                        // Simplified: just take from waitroom
                        let cards = player.waitroom.take_all();
                        for (i, card) in cards.into_iter().enumerate() {
                            if i < count as usize {
                                player.hand.add_card(card);
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn execute_gain_resource(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let resource = effect.resource.as_deref().unwrap_or("");
        let target = effect.target.as_deref().unwrap_or("self");

        let player = match target {
            "self" => &mut self.game_state.player1,
            "opponent" => &mut self.game_state.player2,
            _ => &mut self.game_state.player1,
        };

        match resource {
            "blade" => {
                // Add blades to stage cards
                player.stage.activate_all_cards();
            }
            "heart" => {
                // Add hearts to stage cards
                // This would need to target specific cards
            }
            "energy" => {
                // Add energy
            }
            _ => {}
        }

        Ok(())
    }

    fn execute_change_state(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let state_change = effect.state_change.as_deref().unwrap_or("");
        let count = effect.count.unwrap_or(1);

        let player = &mut self.game_state.player1;

        match state_change {
            "active" => {
                if count == 2 {
                    player.energy_zone.activate_all();
                } else {
                    player.stage.activate_all_cards();
                }
            }
            "wait" => {
                player.stage.set_all_orientation(crate::zones::Orientation::Wait);
            }
            _ => {}
        }

        Ok(())
    }

    fn execute_modify_score(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let _value = effect.value.unwrap_or(0);

        // This would need to target specific cards
        match operation {
            "add" => {
                // Add score to target card
            }
            "remove" => {
                // Remove score from target card
            }
            "set" => {
                // Set score to value
            }
            _ => {}
        }

        Ok(())
    }

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

    fn pay_cost(&mut self, cost: &AbilityCost) -> Result<(), String> {
        match cost.action.as_deref() {
            Some("move_cards") => {
                // Execute the move action as a cost
                let effect = AbilityEffect {
                    text: cost.text.clone(),
                    action: cost.action.clone().unwrap_or_default(),
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
                if energy > 0 {
                    // Deactivate energy cards
                    self.game_state.player1.energy_zone.cards.iter_mut().take(energy as usize).for_each(|card| {
                        card.orientation = Some(crate::zones::Orientation::Wait);
                    });
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
        }
    }
}
