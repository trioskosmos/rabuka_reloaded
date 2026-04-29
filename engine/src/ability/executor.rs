use crate::card::{Ability, AbilityCost, AbilityEffect, Condition};
use crate::game_state::GameState;
use crate::player::Player;

// Re-export types from types module
pub use super::types::{CostCalculation, AbilityValidation, Choice, ChoiceResult};

use super::cost::calculate_cost;
use super::condition::evaluate_condition;
use super::choice::ChoiceHandler;
use super::effects::EffectExecutor;

#[derive(Debug, Clone)]
pub struct AbilityExecutor {
    choice_handler: ChoiceHandler,
    effect_executor: EffectExecutor,
}

impl AbilityExecutor {
    pub fn new() -> Self {
        Self {
            choice_handler: ChoiceHandler::new(),
            effect_executor: EffectExecutor::new(),
        }
    }

    /// Calculate if a cost can be paid and return detailed information
    pub fn calculate_cost(
        &self,
        cost: &AbilityCost,
        player: &Player,
        game_state: &GameState,
    ) -> CostCalculation {
        calculate_cost(cost, player, game_state)
    }

    /// Evaluate if a condition is met
    pub fn evaluate_condition(
        &self,
        condition: &Condition,
        player: &Player,
        game_state: &GameState,
    ) -> bool {
        evaluate_condition(condition, player, game_state)
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
        self.effect_executor.execute_move_cards(effect, player, game_state, perspective_player_id)
    }

    /// Execute a draw effect
    pub fn execute_draw(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
    ) -> Result<(), String> {
        self.effect_executor.execute_draw(effect, player)
    }

    /// Execute a gain_resource effect
    pub fn execute_gain_resource(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_gain_resource(effect, player, game_state, perspective_player_id)
    }

    /// Execute a modify_score effect
    pub fn execute_modify_score(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_modify_score(effect, player, game_state, perspective_player_id)
    }

    /// Execute a modify_required_hearts effect
    pub fn execute_modify_required_hearts(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_modify_required_hearts(effect, player, game_state, perspective_player_id)
    }

    /// Execute a set_required_hearts effect
    pub fn execute_set_required_hearts(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_set_required_hearts(effect, player, game_state, perspective_player_id)
    }

    /// Execute a modify_required_hearts_global effect
    pub fn execute_modify_required_hearts_global(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_modify_required_hearts_global(effect, player, game_state, perspective_player_id)
    }

    /// Execute a set_blade_type effect
    pub fn execute_set_blade_type(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_set_blade_type(effect, player, game_state, perspective_player_id)
    }

    /// Execute a set_heart_type effect
    pub fn execute_set_heart_type(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_set_heart_type(effect, player, game_state, perspective_player_id)
    }

    /// Execute a position_change effect
    pub fn execute_position_change(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_position_change(effect, player, game_state, perspective_player_id)
    }

    /// Execute a place_energy_under_member effect
    pub fn execute_place_energy_under_member(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_place_energy_under_member(effect, player, game_state, perspective_player_id)
    }

    /// Execute a modify_yell_count effect
    pub fn execute_modify_yell_count(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_modify_yell_count(effect, player, game_state, perspective_player_id)
    }

    /// Request a choice from the user
    pub fn request_choice(&mut self, choice: Choice) -> Result<(), String> {
        self.choice_handler.request_choice(choice)
    }

    /// Get pending choice (if any)
    pub fn get_pending_choice(&self) -> Option<&Choice> {
        self.choice_handler.get_pending_choice()
    }

    /// Provide choice result
    pub fn provide_choice_result(&mut self, result: ChoiceResult) -> Result<(), String> {
        self.choice_handler.provide_choice_result(result)
    }

    /// Execute the pending optional cost
    pub fn execute_pending_optional_cost(
        &mut self,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.choice_handler.execute_pending_optional_cost(
            player, game_state, perspective_player_id,
            |cost, p, gs, pid| self.effect_executor.execute_cost(cost, p, gs, pid)
        )
    }

    /// Skip the pending optional cost
    pub fn skip_pending_optional_cost(&mut self) {
        self.choice_handler.skip_pending_optional_cost()
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
            self.execute_pending_optional_cost(player, game_state, perspective_player_id)?;
        } else {
            self.skip_pending_optional_cost();
        }

        if let Some(ref effect) = ability.effect {
            self.execute_effect(effect, player, game_state, perspective_player_id)?;
        }

        game_state.activating_card = None;
        Ok(())
    }

    /// Execute a look_at effect
    pub fn execute_look_at(
        &mut self,
        effect: &AbilityEffect,
        player: &Player,
    ) -> Result<Vec<i16>, String> {
        self.effect_executor.execute_look_at(effect, player)
    }

    /// Execute sequential actions
    pub fn execute_sequential(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_sequential(effect, player, game_state, perspective_player_id)
    }

    /// Execute an ability effect
    pub fn execute_effect(
        &mut self,
        effect: &AbilityEffect,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_effect(effect, player, game_state, perspective_player_id)
    }

    /// Execute ability cost
    pub fn execute_cost(
        &mut self,
        cost: &AbilityCost,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
    ) -> Result<(), String> {
        self.effect_executor.execute_cost(cost, player, game_state, perspective_player_id)
    }

    /// Execute full ability
    pub fn execute_ability(
        &mut self,
        ability: &Ability,
        player: &mut Player,
        game_state: &mut GameState,
        perspective_player_id: &str,
        activating_card: Option<i16>,
    ) -> Result<(), String> {
        game_state.activating_card = activating_card;
        
        if let Some(ref cost) = ability.cost {
            if cost.optional.unwrap_or(false) {
                let description = if cost.text.is_empty() {
                    format!("Optional cost: Discard cards. Do you want to pay this cost?")
                } else {
                    format!("Optional cost: {}. Do you want to pay this cost?", cost.text)
                };
                
                self.choice_handler.pending_optional_cost = Some(cost.clone());
                
                self.choice_handler.request_choice(Choice::SelectTarget {
                    target: "pay_optional_cost".to_string(),
                    description,
                })?;
                
                return Err("Pending choice required: choose whether to pay optional cost".to_string());
            } else {
                self.execute_cost(cost, player, game_state, perspective_player_id)?;
            }
        }

        if let Some(ref effect) = ability.effect {
            self.execute_effect(effect, player, game_state, perspective_player_id)?;
        }

        game_state.activating_card = None;
        Ok(())
    }
}
