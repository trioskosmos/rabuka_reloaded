use super::condition::ConditionContext;
use super::enums::ConditionType;
use super::resolver::AbilityResolver;
use super::types::{AbilityTraceNode, Choice, ExecutionContext, StepOutput, ZoneSnapshot};
use crate::ability::debug::ABILITY_DEBUG;
use crate::ability::types::Command;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
use std::sync::atomic::Ordering;

impl AbilityResolver {
    pub fn execute_sequential_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        conditional: bool,
        is_further: bool,
    ) -> Result<(), String> {
        if ABILITY_DEBUG.load(Ordering::Relaxed) {
            eprintln!("[SEQ_ENTRY] execute_sequential_effect: action={} conditional={} is_further={} actions={}",
                effect.action, conditional, is_further, 
                effect.compound.actions.as_ref().map(|a| a.len()).unwrap_or(0));
        }
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

        let seq_node = self.debug_trace.then(|| {
            AbilityTraceNode::new(seq_label)
                .with_card(card_name.clone())
                .with_before(ZoneSnapshot::from_game_state(gs))
        });

        // Clear step_results at the top of every sequential so previous
        // ability's outputs never leak in.
        self.step_state.clear();

        let cond_met = {
            let ctx = super::condition::ConditionContext::new(gs);
            effect
                .condition
                .as_ref()
                .is_none_or(|c| ctx.evaluate_condition(c))
        };
        if !cond_met {
            return Ok(());
        }

        if is_further {
            log::debug!("Further conditional effect (さらに) - executing additional actions");
        }

        if let Some(ref actions) = effect.compound.actions {
            let has_repeat = actions
                .last()
                .is_some_and(|a| a.action == "repeat_procedure");
            let repeat_max = if has_repeat {
                // repeat_limit = max additional iterations (e.g. 4 = 4 more times)
                // Total iterations = initial + max_repeats
                actions.last().and_then(|a| a.repeat_limit).unwrap_or(1) + 1
            } else {
                1
            };
            let repeat_actions: &[AbilityEffect] = if has_repeat {
                &actions[..actions.len() - 1]
            } else {
                actions.as_slice()
            };

            log::debug!(
                "[ABILITY] sequential: {} actions, repeat_max={} card_id={:?}",
                repeat_actions.len(),
                repeat_max,
                self.activating_card_id
            );
            // Track if a preceding conditional step was satisfied, for
            // if-then-else (otherwise_condition) support.
            let mut condition_failed: Option<bool> = None;
            for _repeat in 0..repeat_max {
                let repeats_remaining = repeat_max.saturating_sub(_repeat + 1);
                'action_loop: for (i, action) in repeat_actions.iter().enumerate() {
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        eprintln!(
                            "[SEQ_TRACE] _repeat={} i={} action={}",
                            _repeat, i, action.action
                        );
                    }
                    log::debug!(
                        "[ABILITY]  >> sub-action[{}]: action={} has_condition={} card_id={:?}",
                        i,
                        action.action,
                        action.condition.is_some(),
                        self.activating_card_id
                    );
                    // Conditional routing: determine how this step should be
                    // handled based on preceding condition results.
                    //
                    //  • is_otherwise  → skip if condition met, execute if failed
                    //  • has condition  → evaluate directly (for otherwise routing)
                    //  • no condition + parent-conditional → skip if preceding failed
                    let is_otherwise = action.condition.as_ref().and_then(|c| c.condition_type)
                        == Some(ConditionType::OtherwiseCondition);
                    if is_otherwise {
                        match condition_failed {
                            Some(false) => {
                                condition_failed = None;
                                continue 'action_loop;
                            }
                            Some(true) => {
                                condition_failed = None;
                            }
                            None => {}
                        }
                    } else if condition_failed == Some(true) && action.condition.is_none() {
                        condition_failed = None;
                        continue 'action_loop;
                    } else if action.condition.is_some() && !is_otherwise {
                        // Evaluate explicit condition — when it fails, skip the
                        // action. Subsequent otherwise-condition steps check the
                        // cached result.
                        // When the same condition text appears on consecutive
                        // actions (parser splits compound predicates like
                        // "if X: do A AND do B" into separate steps), reuse
                        // the first evaluation's result instead of re-evaluating
                        // against stale state (e.g. revealed_cards emptied by
                        // the first action's execution).
                        let same_as_prev = i > 0
                            && repeat_actions[i - 1]
                                .condition
                                .as_ref()
                                .map(|c| c.text.as_str())
                                == action.condition.as_ref().map(|c| c.text.as_str());
                        if same_as_prev {
                            if condition_failed == Some(true) {
                                continue 'action_loop;
                            }
                        } else {
                            let ctx = ConditionContext::with_moved_cards(gs, &self.moved_cards);
                            let passed = ctx.evaluate_condition(action.condition.as_ref().unwrap());
                            condition_failed = Some(!passed);
                            if !passed {
                                continue 'action_loop;
                            }
                        }
                    }

                    let mut action_to_execute = action.clone();
                    // Clear the condition on the clone — the sequential loop already
                    // checked/gated on it above. Without this, execute_effect →
                    // can_activate_effect re-evaluates the condition against potentially
                    // stale game state (e.g. revealed_cards emptied by a prior step).
                    action_to_execute.condition = None;
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
                    // Inherit self_target from the parent effect or the first
                    // sub-action when this action doesn't have it set.
                    // Japanese grammar attaches "このカード" (this card) to the
                    // first verb only, but the intent applies to all verbs in
                    // a compound sentence (e.g. EMOTION: "card's score+2 AND
                    // required hearts+3" — both target the card itself).
                    // Exception: don't inherit when the current action has an
                    // explicit card_type that differs from the inherited target
                    // (e.g. modify_score self_target bleeds into gain_resource
                    // member_card — the live card is not a member on stage).
                    if action_to_execute.self_target.is_none() {
                        let inheritable = if i > 0 {
                            let first_ct = repeat_actions[0].card_type.as_deref();
                            let cur_ct = action.card_type.as_deref();
                            // If the first action targets a live card (no card_type
                            // or live_card) and the current targets a member, don't
                            // inherit — they're different cards entirely.
                            !(first_ct != Some("member_card") && cur_ct == Some("member_card"))
                        } else {
                            true
                        };
                        if inheritable {
                            // Only inherit self_target to action types that actually
                            // support it. "このカード" (this card) in a compound sentence
                            // applies to score/heart modifiers, not to generic draw or
                            // move actions. Prevent cascading into nested sequentials
                            // (draw+move inside would inherit from the nested container).
                            let at = crate::ability::enums::ActionType::from_str(&action.action);
                            let supports_self = matches!(
                                at,
                                Some(crate::ability::enums::ActionType::ModifyScore)
                                    | Some(crate::ability::enums::ActionType::ModifyRequiredHearts)
                                    | Some(crate::ability::enums::ActionType::GainResource)
                                    | Some(crate::ability::enums::ActionType::ChangeState)
                            );
                            if effect.self_target.is_some() && supports_self {
                                action_to_execute.self_target = effect.self_target;
                            } else if i > 0 && supports_self {
                                action_to_execute.self_target = repeat_actions[0].self_target;
                            }
                        }
                    }
                    // Inherit card_names from the parent (set at the sequential
                    // level, e.g. EMOTION's "card_names": ["EMOTION"]) so that
                    // per-unit counting in sub-actions only counts matching cards.
                    if action_to_execute.card_names.is_empty() && !effect.card_names.is_empty() {
                        action_to_execute.card_names.clone_from(&effect.card_names);
                    }

                    log::debug!(
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

                    // G3: before executing an opponent-action sub-action, tag the spawn
                    // context so that any choice created inside is routed to the opponent.
                    if action.action == "opponent_action"
                        || action.action_by.as_deref() == Some("opponent")
                    {
                        self.spawn_context.target = Some("opponent".to_string());
                    }

                    let moved_before = self.moved_cards.len();
                    match self.execute_effect(gs, &action_to_execute) {
                        Ok(_) => {
                            if ABILITY_DEBUG.load(Ordering::Relaxed) {
                                eprintln!(
                                    "[SEQ_TRACE] after execute: pending={:?}",
                                    self.pending_choice.is_some()
                                );
                            }
                            log::debug!(
                                "[SEQ_LOOP] after execute: pending={:?}",
                                self.pending_choice.is_some()
                            );
                            // Record this step's output under its id (if any)
                            // so downstream steps in the same sequential can
                            // reference it via `ref: "<id>"`.
                            if let Some(ref step_id) = action.id {
                                let mut out = StepOutput::default();
                                // Capture cards the step "produced": selected
                                // cards, moved cards, or looked-at cards.
                                // We take whichever is most relevant for the
                                // action type. Heuristic ordering matches
                                // what the cost handlers do: selected > moved
                                // > looked_at.
                                if !self.selected_cards.is_empty() {
                                    out.cards.extend_from_slice(&self.selected_cards);
                                } else if !self.moved_cards.is_empty() {
                                    out.cards.extend_from_slice(&self.moved_cards);
                                } else if !gs.looked_at_cards.is_empty() {
                                    out.cards.extend_from_slice(&gs.looked_at_cards);
                                } else if !gs.revealed_cards.is_empty() {
                                    out.cards.extend_from_slice(&gs.revealed_cards);
                                }
                                if self.step_state.last_draw_count > 0 {
                                    out.value = Some(self.step_state.last_draw_count as i32);
                                }
                                self.step_state
                                    .step_results
                                    .entry(step_id.clone())
                                    .or_insert_with(StepOutput::default)
                                    .merge(&out);
                                log::debug!(
                                    "[SEQ_LOOP] recorded step '{}' output: cards={:?} value={:?}",
                                    step_id,
                                    out.cards,
                                    out.value
                                );
                            }
                            if self.pending_choice.is_some() {
                                let current_was_optional = action.optional.unwrap_or(false);
                                let is_opponent_action = action.action == "opponent_action"
                                    || action.action_by.as_deref() == Some("opponent");
                                let mut remaining = if current_was_optional
                                    && i + 1 < repeat_actions.len()
                                    && !is_opponent_action
                                {
                                    let mut actions: Vec<AbilityEffect> =
                                        repeat_actions[i..].to_vec();
                                    if !actions.is_empty() {
                                        actions[0].optional = None;
                                    }
                                    actions
                                } else {
                                    repeat_actions[i + 1..].to_vec()
                                };
                                // When the condition already passed (Some(false)),
                                // strip otherwise-condition actions from remaining
                                // so they aren't re-executed by RPC.
                                if condition_failed == Some(false) {
                                    remaining.retain(|a| {
                                        !(a.condition.as_ref().and_then(|c| c.condition_type)
                                            == Some(crate::ability::enums::ConditionType::OtherwiseCondition))
                                    });
                                }
                                // Store remaining repeats on the resolver for
                                // one-at-a-time feeding after each iteration completes.
                                // We DON'T pre-load them into pending_commands — that
                                // causes duplication with RPC's merge logic.
                                if repeats_remaining > 0 && has_repeat {
                                    if let Some(ref repeat_action) = actions.last() {
                                        if repeat_action.action == "repeat_procedure"
                                            && repeat_action.optional.unwrap_or(false)
                                        {
                                            for _ in 0..repeats_remaining {
                                                self.pending_repeat_actions
                                                    .extend(repeat_actions.iter().cloned());
                                            }
                                        }
                                    }
                                }
                                save_remaining(gs, remaining);
                                return Ok(());
                            } else if self.cancel_remaining_commands {
                                // An optional sub-action (e.g. pay_energy with insufficient
                                // energy) requested cancellation of subsequent actions.
                                self.cancel_remaining_commands = false;
                                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                                    eprintln!("[SEQ_TRACE] cancel_remaining_commands set — aborting sequential loop");
                                }
                                return Ok(());
                            } else if action.optional.unwrap_or(false)
                                && action.action == "change_state"
                            {
                                // Optional change_state completed without creating a choice
                                // (no valid targets). Skip remaining actions (そうした場合).
                                return Ok(());
                            } else if !self.pending_choice.is_some()
                                && conditional
                                && action.condition.is_none()
                            {
                                // Parent-conditional sequential: track implicit
                                // condition via was_moved (old behavior).
                                let was_moved = self.moved_cards.len() - moved_before;
                                condition_failed = Some(was_moved == 0);
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                // After all actions in this iteration, check if repeat_procedure is optional
                // and we have remaining repeats — ask player whether to continue.
                if repeats_remaining > 0 {
                    if let Some(ref repeat_action) = actions.last() {
                        if repeat_action.action == "repeat_procedure"
                            && repeat_action.optional.unwrap_or(false)
                        {
                            self.pending_choice = Some(Choice::SelectTarget {
                                target: "pay_optional_cost:skip_optional_cost".to_string(),
                                description: "Repeat effect?".to_string(),
                                allow_skip: true,
                                options: Some(vec!["Stop".to_string(), "Continue".to_string()]),
                            });
                            let mut remaining: Vec<AbilityEffect> = Vec::new();
                            for _ in 0..repeats_remaining {
                                remaining.extend_from_slice(repeat_actions);
                            }
                            if !remaining.is_empty() {
                                let mut existing = gs.ability_queue.take_pending_commands();
                                existing.extend(remaining.into_iter().map(Command::Effect));
                                gs.ability_queue.set_pending_commands(existing);
                            }
                            return Ok(());
                        }
                    }
                }
            }
        }
        // Clear context so resume_with_choice doesn't re-process the ability.
        self.execution_context = ExecutionContext::None;

        // Finalize sequential trace node (only allocated when debug_trace is enabled)
        if let Some(mut node) = seq_node {
            node.after = Some(ZoneSnapshot::from_game_state(gs));
            self.pipeline.trace.children.push(node);
        }

        Ok(())
    }

    pub fn execute_conditional_alternative(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let has_primary = effect.compound.primary_effect.is_some();
        let has_alternative = effect.alternative_effect.is_some();

        if has_primary && has_alternative {
            let ctx = super::condition::ConditionContext::with_moved_cards(gs, &self.moved_cards);

            // Tiered conditions: alternative_condition (stricter) checked first.
            // If met → execute alternative_effect (replaces primary).
            // Otherwise check the base condition — if met → execute primary_effect.
            // If neither is met → do nothing.
            if effect.compound.alternative_condition.is_some() && effect.condition.is_some() {
                if let Some(ref alt_cond) = effect.compound.alternative_condition {
                    if ctx.evaluate_condition(alt_cond) {
                        if let Some(ref alt_effect) = effect.alternative_effect {
                            return self.execute_effect(gs, alt_effect);
                        }
                    }
                }
                if let Some(ref cond) = effect.condition {
                    if ctx.evaluate_condition(cond) {
                        if let Some(ref primary_effect) = effect.compound.primary_effect {
                            return self.execute_effect(gs, primary_effect);
                        }
                    }
                }
                return Ok(());
            }

            // Legacy: single condition selects alternative (true) or primary (false).
            // The alternative_effect is the "special case" when the condition is met;
            // primary_effect is the normal/default case when condition is not met.
            let single_cond = effect
                .compound
                .alternative_condition
                .as_ref()
                .or(effect.condition.as_ref());
            if let Some(cond) = single_cond {
                if ctx.evaluate_condition(cond) {
                    if let Some(ref alt_effect) = effect.alternative_effect {
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
                if let Some(ref alt_effect) = effect.alternative_effect {
                    return self.execute_effect(gs, alt_effect);
                }
            }
        }

        // Handle the case where we have alternative_effect + top-level condition
        // but no primary_effect and no alternative_condition (e.g. replacement effects
        // like 錯覚CROSSROADS: "when this card would be placed in success zone, instead...")
        if effect.alternative_effect.is_some()
            && effect.compound.primary_effect.is_none()
            && effect.compound.alternative_condition.is_none()
        {
            if let Some(ref cond) = effect.condition {
                let ctx =
                    super::condition::ConditionContext::with_moved_cards(gs, &self.moved_cards);
                if ctx.evaluate_condition(cond) {
                    if let Some(ref alt_effect) = effect.alternative_effect {
                        return self.execute_effect(gs, alt_effect);
                    }
                }
            }
            return Ok(());
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
        let cost_was_paid = gs.ability_queue.current_entry().is_none_or(|e| {
            e.optional_cost_result == Some(true) || e.cost_paid || e.ability.cost.is_none()
        });
        if !cost_was_paid {
            return Ok(());
        }

        let primary_action = effect.compound.primary_effect.as_ref();
        let result_condition = effect.compound.result_condition.as_ref();
        let followup_action = effect.compound.followup_action.as_ref();

        if let Some(primary) = primary_action {
            if let Err(e) = self.execute_effect(gs, primary) {
                log::debug!("Primary action failed in conditional_on_result: {}", e);
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
            if let Some(followup) = followup_action {
                self.execute_effect(gs, followup)?;
            }
        } else {
            log::debug!("Result condition not met, skipping followup action");
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

        // Q92: if the optional action requires energy and player can't afford it,
        // skip the choice and execute the conditional action directly
        if let (Some(opt), Some(cond)) = (optional_action, conditional_action) {
            if opt.action == "pay_energy" {
                let need = opt.energy_count.unwrap_or(0) as usize;
                if need > 0 {
                    let player =
                        gs.resolve_target_player_mut(opt.target.as_deref().unwrap_or("self"));
                    if (player.energy_zone.active_count() as usize) < need {
                        let cmd = if is_negation {
                            Command::Effect(*cond.clone())
                        } else {
                            Command::Effect(effect.clone())
                        };
                        gs.ability_queue.set_pending_commands(vec![cmd]);
                        return self.resume_pending_commands(gs);
                    }
                }
            }
        }

        if crate::ability::debug::ABILITY_DEBUG.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!(
                "[COND_OPT] opt={:?} cond={:?}",
                optional_action.is_some(),
                conditional_action.is_some()
            );
        }
        if optional_action.is_some() && conditional_action.is_some() {
            let result = gs
                .ability_queue
                .current_entry()
                .and_then(|e| e.optional_cost_result);
            if let Some(cost_was_paid) = result {
                let chose_yes = cost_was_paid;
                let effect = effect.clone();
                let cmd = match (chose_yes, is_negation) {
                    (true, true) => effect.compound.optional_action.map(|a| Command::Effect(*a)),
                    (true, false) => effect
                        .compound
                        .conditional_action
                        .map(|a| Command::Effect(*a)),
                    (false, true) => effect
                        .compound
                        .conditional_action
                        .map(|a| Command::Effect(*a)),
                    (false, false) => None,
                };
                if let Some(cmd) = cmd {
                    gs.ability_queue.set_pending_commands(vec![cmd]);
                }
                return self.resume_pending_commands(gs);
            }
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                if let Ok(json) = serde_json::to_string(&effect) {
                    entry.conditional_choice = Some(json);
                }
            }
            self.pending_choice = Some(Choice::SelectTarget {
                target: "conditional_optional".to_string(),
                description: "Pay optional cost or skip".to_string(),
                allow_skip: true,
                options: Some(vec!["Skip".to_string(), "Pay".to_string()]),
            });
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some(crate::ability::types::ChoiceRoute::OptionalCost);
            }
            return Ok(());
        }

        if let Some(optional) = optional_action {
            self.execute_effect(gs, optional)?;
        }
        if let Some(conditional) = conditional_action {
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
