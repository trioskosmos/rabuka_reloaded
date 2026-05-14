use super::resolver::AbilityResolver;
use super::types::{Choice, ExecutionContext};
use crate::card::AbilityEffect;

impl<'a> AbilityResolver<'a> {
    pub fn execute_sequential_effect(
        &mut self,
        effect: &AbilityEffect,
        conditional: bool,
        is_further: bool,
    ) -> Result<(), String> {
        let cond_met = if conditional {
            effect
                .condition
                .as_ref()
                .map_or(true, |c| self.evaluate_condition(c))
        } else {
            true
        };
        if !cond_met {
            return Ok(());
        }

        if is_further {
            eprintln!("Further conditional effect (さらに) - executing additional actions");
        }

        if let Some(ref actions) = effect.compound.actions {
            let has_repeat = actions
                .last()
                .map_or(false, |a| a.action == "repeat_procedure");
            let repeat_max = if has_repeat {
                actions.last().and_then(|a| a.repeat_limit).unwrap_or(1)
            } else {
                1
            };
            let repeat_actions: &[AbilityEffect] = if has_repeat {
                &actions[..actions.len() - 1]
            } else {
                actions.as_slice()
            };

            eprintln!(
                "[ABILITY] sequential: {} actions, repeat_max={} card_id={:?}",
                repeat_actions.len(),
                repeat_max,
                self.activating_card_id
            );
            for _repeat in 0..repeat_max {
                for (i, action) in repeat_actions.iter().enumerate() {
                    eprintln!(
                        "[ABILITY]  >> sub-action[{}]: action={} has_condition={} card_id={:?}",
                        i,
                        action.action,
                        action.condition.is_some(),
                        self.activating_card_id
                    );
                    let mut action_to_execute = action.clone();
                    // Only inherit per_unit properties for actions that support them
                    // Discard/move_cards actions should not inherit per_unit multipliers
                    let supports_per_unit = matches!(
                        action.action.as_str(),
                        "draw"
                            | "draw_card"
                            | "gain_resource"
                            | "modify_score"
                            | "modify_required_hearts"
                            | "gain_ability"
                            | "set_blade_count"
                    );
                    if supports_per_unit {
                        if action_to_execute.per_unit.is_none() && effect.per_unit.is_some() {
                            action_to_execute.per_unit = effect.per_unit;
                        }
                        if action_to_execute.per_unit_count.is_none()
                            && effect.per_unit_count.is_some()
                        {
                            action_to_execute.per_unit_count = effect.per_unit_count;
                        }
                        if action_to_execute.per_unit_type.is_none()
                            && effect.per_unit_type.is_some()
                        {
                            action_to_execute.per_unit_type = effect.per_unit_type.clone();
                        }
                    }

                    match self.execute_effect(&action_to_execute) {
                        Ok(_) => {
                            if self.pending_choice.is_some() {
                                // A nested sequential may have already stored its remaining
                                // actions in pending_sequential_actions. Don't overwrite
                                // if the current action had no remaining.
                                let current_was_optional = action.optional.unwrap_or(false);
                                let remaining = if current_was_optional {
                                    let mut actions: Vec<AbilityEffect> =
                                        repeat_actions[i..].to_vec();
                                    if !actions.is_empty() {
                                        actions[0].optional = None;
                                    }
                                    actions
                                } else {
                                    repeat_actions[i + 1..].to_vec()
                                };
                                if !remaining.is_empty() {
                                    self.game_state.pending_sequential_actions = Some(remaining);
                                }
                                return Ok(());
                            }
                        }
                        Err(e) if e.contains("Pending choice required") => {
                            let current_was_optional = action.optional.unwrap_or(false);
                            let remaining = if current_was_optional {
                                let mut actions: Vec<AbilityEffect> = repeat_actions[i..].to_vec();
                                if !actions.is_empty() {
                                    actions[0].optional = None;
                                }
                                actions
                            } else {
                                repeat_actions[i + 1..].to_vec()
                            };
                            if !remaining.is_empty() {
                                self.game_state.pending_sequential_actions = Some(remaining);
                            }
                            return Ok(());
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        Ok(())
    }

    pub fn execute_conditional_alternative(
        &mut self,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let has_primary = effect.compound.primary_effect.is_some();
        let has_alternative = effect.compound.alternative_effect.is_some();

        if has_primary && has_alternative {
            let primary_text = effect
                .compound
                .primary_effect
                .as_ref()
                .map(|e| e.text.as_str())
                .unwrap_or("Primary effect");
            let alternative_text = effect
                .compound
                .alternative_effect
                .as_ref()
                .map(|e| e.text.as_str())
                .unwrap_or("Alternative effect");
            let description = format!(
                "Choose effect:\nPrimary: {}\nAlternative: {}",
                primary_text, alternative_text
            );
            self.pending_choice = Some(Choice::SelectTarget {
                target: "primary|alternative".to_string(),
                description,
                allow_skip: false,
            });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }

        if let Some(ref alt_condition) = effect.compound.alternative_condition {
            if self.evaluate_condition(alt_condition) {
                if let Some(ref alt_effect) = effect.compound.alternative_effect {
                    return self.execute_effect(alt_effect);
                }
            }
        }

        if let Some(ref primary_effect) = effect.compound.primary_effect {
            self.execute_effect(primary_effect)
        } else {
            Ok(())
        }
    }

    pub fn execute_repeat_procedure(
        &mut self,
        effect: &AbilityEffect,
        repeat_limit: u32,
    ) -> Result<(), String> {
        let repeat_limit = repeat_limit as usize;
        if let Some(ref actions) = effect.compound.actions {
            for _ in 0..repeat_limit {
                for action in actions {
                    self.execute_effect(action)?;
                }
            }
        }
        Ok(())
    }

    pub fn execute_conditional_on_result(&mut self, effect: &AbilityEffect) -> Result<(), String> {
        // If the ability has an optional cost and it was NOT paid (skipped),
        // the primary effect should not run. This fixes "may pay" patterns
        // where the effect is gated behind the optional cost.
        let cost_was_paid = self
            .game_state
            .ability_queue
            .current_entry()
            .map_or(true, |e| {
                e.optional_cost_was_paid || !e.cost_paid || e.ability.cost.is_none()
            });
        if !cost_was_paid {
            return Ok(());
        }

        let primary_action = effect.compound.primary_effect.as_ref();
        let result_condition = effect.compound.result_condition.as_ref();
        let followup_action = effect.compound.followup_action.as_ref();

        if let Some(ref primary) = primary_action {
            if let Err(e) = self.execute_effect(primary) {
                eprintln!("Primary action failed in conditional_on_result: {}", e);
                return Err(e);
            }
        }

        let condition_met = result_condition
            .map(|c| self.evaluate_condition(c))
            .unwrap_or(true);

        if condition_met {
            if let Some(ref followup) = followup_action {
                self.execute_effect(followup)?;
            }
        } else {
            eprintln!("Result condition not met, skipping followup action");
        }
        Ok(())
    }

    pub fn execute_conditional_on_optional(
        &mut self,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let optional_action = effect.compound.optional_action.as_ref();
        let conditional_action = effect.compound.conditional_action.as_ref();
        let is_negation = effect.compound.conditional_negation.unwrap_or(false);

        if optional_action.is_some() && conditional_action.is_some() {
            let desc = optional_action
                .as_ref()
                .map(|a| a.text.as_str())
                .unwrap_or("Perform optional action");
            self.pending_choice = Some(Choice::SelectTarget {
                target: "conditional_optional".to_string(),
                description: format!("{}?", desc),
                allow_skip: true,
            });
            return Ok(());
        }

        if let Some(ref optional) = optional_action {
            self.execute_effect(optional)?;
        }
        if let Some(ref conditional) = conditional_action {
            if !is_negation {
                self.execute_effect(conditional)?;
            }
        }
        Ok(())
    }
}
