//! Additional methods for AbilityResolver: compound effect execution

use crate::ability::resolver::AbilityResolver;
use crate::ability::queue::ConditionalChoice;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
// `crate::Box` is the compat alias: `alloc::boxed::Box` under `no_std`,
// `std::boxed::Box` on host, so the signature below compiles on both.
use crate::Box;
#[cfg(feature = "no_std")]
use alloc::string::String;

impl AbilityResolver {
    /// Execute sequential effect. The step list is supplied by the caller so the
    /// routing path can pass a borrowed slice instead of deep-cloning the effect
    /// tree to hang the steps off a synthetic parent.
    pub fn execute_sequential_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let steps: &[Box<AbilityEffect>] = match effect
            .compound_branch()
            .and_then(|c| c.actions.as_deref())
        {
            Some(actions) => actions,
            None => &[],
        };
        crate::ability::compound::sequential::execute_sequential_effect(
            self,
            gs,
            effect,
            steps,
            effect.action,
        )
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