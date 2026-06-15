use super::resolver::AbilityResolver;
use super::types::{AbilityTraceNode, Choice, ExecutionContext, ZoneSnapshot};
use crate::ability::types::Command;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    pub fn execute_sequential_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        conditional: bool,
        is_further: bool,
    ) -> Result<(), String> {
        // Trace sequential compound effect
        let seq_label = if conditional {
            "sequential_conditional".to_string()
        } else if is_further {
            "sequential_further".to_string()
        } else {
            "sequential".to_string()
        };

        let card_name = self
            .activating_card_id
            .and_then(|cid| gs.card_database.get_card(cid))
            .map(|c| c.name.clone());

        let before = ZoneSnapshot::from_game_state(gs);
        let mut seq_node = AbilityTraceNode::new(seq_label)
            .with_card(card_name)
            .with_before(before);

        let cond_met = if conditional {
            let ctx = super::condition::ConditionContext::new(gs);
            effect
                .condition
                .as_ref()
                .map_or(true, |c| ctx.evaluate_condition(c))
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
                        crate::ability::enums::ActionType::from_str(&action.action),
                        Some(crate::ability::enums::ActionType::Draw)
                            | Some(crate::ability::enums::ActionType::DrawCard)
                            | Some(crate::ability::enums::ActionType::GainResource)
                            | Some(crate::ability::enums::ActionType::ModifyScore)
                            | Some(crate::ability::enums::ActionType::ModifyRequiredHearts)
                            | Some(crate::ability::enums::ActionType::GainAbility)
                            | Some(crate::ability::enums::ActionType::SetBladeCount)
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

                    eprintln!(
                        "[SEQ_LOOP] executing action[{}] action={} pending_before={:?}",
                        i,
                        action.action,
                        self.pending_choice.is_some()
                    );
                    fn save_remaining(
                        gs: &mut crate::game_state::GameState,
                        remaining: Vec<AbilityEffect>,
                    ) {
                        if !remaining.is_empty() {
                            let mut existing = gs.ability_queue.take_pending_commands();
                            existing.extend(remaining.into_iter().map(Command::Effect));
                            gs.ability_queue.set_pending_commands(existing);
                        }
                    }

                    match self.execute_effect(gs, &action_to_execute) {
                        Ok(_) => {
                            eprintln!(
                                "[SEQ_LOOP] after execute: pending={:?}",
                                self.pending_choice.is_some()
                            );
                            if self.pending_choice.is_some() {
                                let current_was_optional = action.optional.unwrap_or(false);
                                let remaining = if current_was_optional && i + 1 < repeat_actions.len() {
                                    let mut actions: Vec<AbilityEffect> =
                                        repeat_actions[i..].to_vec();
                                    if !actions.is_empty() {
                                        actions[0].optional = None;
                                    }
                                    actions
                                } else {
                                    repeat_actions[i + 1..].to_vec()
                                };
                                save_remaining(gs, remaining);
                                return Ok(());
                            } else if action.optional.unwrap_or(false)
                                && action.action == "change_state"
                            {
                                // Optional change_state completed without creating a choice
                                // (no valid targets). Skip remaining actions (そうした場合).
                                return Ok(());
                            }
                        }
                        Err(e) if e.contains("Pending choice required") => {
                            let current_was_optional = action.optional.unwrap_or(false);
                            let remaining = if current_was_optional && i + 1 < repeat_actions.len() {
                                let mut actions: Vec<AbilityEffect> = repeat_actions[i..].to_vec();
                                if !actions.is_empty() {
                                    actions[0].optional = None;
                                }
                                actions
                            } else {
                                repeat_actions[i + 1..].to_vec()
                            };
                            save_remaining(gs, remaining);
                            return Ok(());
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        // Clear context so resume_with_choice doesn't re-process the ability.
        self.execution_context = ExecutionContext::None;

        // Finalize sequential trace node
        seq_node.after = Some(ZoneSnapshot::from_game_state(gs));
        self.pipeline.trace.children.push(seq_node);

        Ok(())
    }

    pub fn execute_conditional_alternative(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let has_primary = effect.compound.primary_effect.is_some();
        let has_alternative = effect.compound.alternative_effect.is_some();

        if has_primary && has_alternative {
            // Check if the condition can decide which path to take automatically,
            // using either the compound's alternative_condition or the top-level effect.condition.
            let condition = effect
                .compound
                .alternative_condition
                .as_ref()
                .or(effect.condition.as_ref());
            if let Some(cond) = condition {
                let ctx =
                    super::condition::ConditionContext::with_moved_cards(gs, &self.moved_cards);
                if ctx.evaluate_condition(cond) {
                    if let Some(ref alt_effect) = effect.compound.alternative_effect {
                        return self.execute_effect(gs, alt_effect);
                    }
                } else if let Some(ref primary_effect) = effect.compound.primary_effect {
                    return self.execute_effect(gs, primary_effect);
                }
                return Ok(());
            }

            // No condition — ask the player to choose
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
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }

        if let Some(ref alt_condition) = effect.compound.alternative_condition {
            let ctx = super::condition::ConditionContext::with_moved_cards(gs, &self.moved_cards);
            if ctx.evaluate_condition(alt_condition) {
                if let Some(ref alt_effect) = effect.compound.alternative_effect {
                    return self.execute_effect(gs, alt_effect);
                }
            }
        }

        if let Some(ref primary_effect) = effect.compound.primary_effect {
            self.execute_effect(gs, primary_effect)
        } else {
            Ok(())
        }
    }

    pub fn execute_repeat_procedure(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        repeat_limit: u32,
    ) -> Result<(), String> {
        let repeat_limit = repeat_limit as usize;
        if let Some(ref actions) = effect.compound.actions {
            for _ in 0..repeat_limit {
                for action in actions {
                    self.execute_effect(gs, action)?;
                }
            }
        }
        Ok(())
    }

    pub fn execute_conditional_on_result(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        // If the ability has an optional cost and it was NOT paid (skipped),
        // the primary effect should not run. This fixes "may pay" patterns
        // where the effect is gated behind the optional cost.
        let cost_was_paid = gs.ability_queue.current_entry().map_or(true, |e| {
            e.optional_cost_was_paid || !e.cost_paid || e.ability.cost.is_none()
        });
        if !cost_was_paid {
            return Ok(());
        }

        let primary_action = effect.compound.primary_effect.as_ref();
        let result_condition = effect.compound.result_condition.as_ref();
        let followup_action = effect.compound.followup_action.as_ref();

        if let Some(ref primary) = primary_action {
            if let Err(e) = self.execute_effect(gs, primary) {
                eprintln!("Primary action failed in conditional_on_result: {}", e);
                return Err(e);
            }
            // If primary created a choice (e.g. "select 3 cards to reveal"),
            // save the condition check + followup as a pending sequential action
            // so it resumes after the choice is resolved.
            if self.pending_choice.is_some() {
                let mut finish = effect.clone();
                finish.compound.primary_effect = None;
                gs.ability_queue
                    .save_pending_sequential_actions(vec![Command::Effect(finish)]);
                return Ok(());
            }
        }

        let condition_met = result_condition
            .map(|c| {
                let ctx =
                    super::condition::ConditionContext::with_moved_cards(gs, &self.moved_cards);
                ctx.evaluate_condition(c)
            })
            .unwrap_or(true);

        if condition_met {
            if let Some(ref followup) = followup_action {
                self.execute_effect(gs, followup)?;
            }
        } else {
            eprintln!("Result condition not met, skipping followup action");
        }
        Ok(())
    }

    pub fn execute_conditional_on_optional(
        &mut self,
        gs: &mut GameState,
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
                options: Some(vec!["Skip".to_string(), "Pay".to_string()]),
            });
            return Ok(());
        }

        if let Some(ref optional) = optional_action {
            self.execute_effect(gs, optional)?;
        }
        if let Some(ref conditional) = conditional_action {
            if !is_negation {
                self.execute_effect(gs, conditional)?;
            }
        }
        Ok(())
    }

    pub fn handle_choice_string_selection(
        &mut self,
        gs: &mut GameState,
        selected: &str,
        conditional_choice: Option<String>,
    ) -> Result<(), String> {
        if let Some(ref options_json) = conditional_choice {
            if let Ok(options) = serde_json::from_str::<Vec<String>>(options_json) {
                if let Ok(idx) = selected.parse::<usize>() {
                    if idx > 0 && idx <= options.len() {
                        let val = &options[idx - 1];
                        if val.starts_with("heart")
                            || ["赤", "桃", "緑", "青", "黄", "紫"].contains(&val.as_str())
                        {
                            gs.prohibition_effects
                                .push(format!("selected_heart_color:{}", val));
                        }
                    }
                }
            }
        }
        self.pending_choice = None;
        self.clear_choice_meta(gs);
        self.resume_pending_commands(gs)?;
        Ok(())
    }

    pub fn handle_choice_string_store(
        &mut self,
        gs: &mut GameState,
        selected: &str,
        conditional_choice: Option<String>,
    ) -> Result<(), String> {
        let chosen = conditional_choice.and_then(|json| {
            serde_json::from_str::<Vec<String>>(&json)
                .ok()
                .and_then(|opts| {
                    selected
                        .parse::<usize>()
                        .ok()
                        .and_then(|idx| opts.get(idx).cloned())
                })
        });
        if let Some(s) = chosen {
            gs.ability_queue
                .current_entry_mut()
                .map(|e| e.conditional_choice = Some(s));
        }
        self.pending_choice = None;
        self.resume_pending_commands(gs)?;
        Ok(())
    }
}
