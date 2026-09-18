//! Additional methods for AbilityResolver: compound effect execution

use crate::ability::resolver::AbilityResolver;
use crate::ability_queue::ConditionalChoice;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    /// Execute sequential effect.
    pub fn execute_sequential_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        crate::ability::compound::sequential::execute_sequential_effect(self, gs, effect)
    }

    /// Execute conditional alternative effect.
    pub fn execute_conditional_alternative(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        crate::ability::compound::conditional::execute_conditional_alternative(self, gs, effect)
    }

    /// Execute conditional-on-result effect.
    pub fn execute_conditional_on_result(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        crate::ability::compound::conditional_on::execute_conditional_on_result(self, gs, effect)
    }

    /// Execute conditional-on-optional effect.
    pub fn execute_conditional_on_optional(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        crate::ability::compound::conditional_on::execute_conditional_on_optional(self, gs, effect)
    }

    /// Handle choice string selection.
    pub fn handle_choice_string_selection(
        &mut self,
        gs: &mut GameState,
        selected: &str,
        conditional_choice: Option<ConditionalChoice>,
    ) -> Result<(), String> {
        crate::ability::compound::conditional_on::handle_choice_string_selection(self, gs, selected, conditional_choice)
    }

    /// Handle choice string store.
    pub fn handle_choice_string_store(
        &mut self,
        gs: &mut GameState,
        selected: &str,
        conditional_choice: Option<ConditionalChoice>,
    ) -> Result<(), String> {
        crate::ability::compound::conditional_on::handle_choice_string_store(self, gs, selected, conditional_choice)
    }
}