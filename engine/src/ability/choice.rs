use crate::card::AbilityCost;
use crate::game_state::GameState;
use crate::player::Player;
use super::types::{Choice, ChoiceResult};

#[derive(Debug, Clone)]
pub struct ChoiceHandler {
    pub pending_choice: Option<Choice>,
    pub pending_optional_cost: Option<AbilityCost>,
}

impl ChoiceHandler {
    pub fn new() -> Self {
        Self {
            pending_choice: None,
            pending_optional_cost: None,
        }
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
            (Some(Choice::SelectCard { zone: _, card_type: _, count: _, .. }), ChoiceResult::CardSelected { indices: _ }) => {
                self.pending_choice = None;
                Ok(())
            }
            (Some(Choice::SelectTarget { target, .. }), ChoiceResult::TargetSelected { .. }) => {
                if target == "pay_optional_cost" {
                    self.pending_choice = None;
                    return Ok(());
                } else {
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
        mut execute_cost_fn: impl FnMut(&AbilityCost, &mut Player, &mut GameState, &str) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ref cost) = self.pending_optional_cost {
            let cost_to_execute = cost.clone();
            self.pending_optional_cost = None;
            execute_cost_fn(&cost_to_execute, player, game_state, perspective_player_id)
        } else {
            Ok(())
        }
    }

    /// Skip the pending optional cost if the player chose not to pay it
    pub fn skip_pending_optional_cost(&mut self) {
        self.pending_optional_cost = None;
    }
}
