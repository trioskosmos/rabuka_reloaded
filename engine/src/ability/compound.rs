use super::condition::ConditionContext;
use super::resolver::AbilityResolver;
use super::types::{AbilityTraceNode, Choice, ExecutionContext, StepOutput, ZoneSnapshot};
use crate::ability::debug::ABILITY_DEBUG;
use crate::ability::enums::{ActionType, Zone};
use crate::ability_queue::ConditionalChoice;
use crate::card::{AbilityEffect, Condition};
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::Ordering;

impl AbilityResolver {
    // Rule 9.2.1.1 / Q94 / Q107 / Q217: Sequential effect execution
    //
    // Executes a list of sub-actions in order (Rule 9.2.1.1: single effects
    // resolve completely before the next begins). Supports:
    //
    //   • Conditions on individual steps (if-then-else routing)
    //   • "otherwise_condition" steps that fire only when previous cond failed
    //   • "repeat_procedure" for looping sub-actions
    //   • Optional steps that gate subsequent actions ("そうした場合")
    //   • per_unit inheritance from parent to sub-actions
    //
    // Q94 / Q255 / Q263: Area-move + sequential
    //   Each area move is a separate trigger. Sequential effects run once
    //   per trigger invocation, not once per area move within one trigger.
    //
    // Q107: Re-yell after sequential discard
    //   The sequential pipeline's step_output tracking allows downstream
    //   conditions to reference upstream results (e.g. "if revealed cards
    //   had no live → discard → re-yell").
    //
    // Q217: "any_number" cost paid as 0 → still counts as "paid" for
    //   sequential gating (was_moved = 0 but optional_cost_result = true
    //   because the player affirmatively chose to pay 0).
    //
    // Rule 9.6.2.4.2: Even if the card bearing the ability leaves its
    //   original zone mid-resolution, the remaining steps still resolve.
    pub fn execute_sequential_effect(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let conditional = effect.conditional.unwrap_or(false);
        let is_further = effect.is_further.unwrap_or(false);
        #[cfg(not(feature = "no_std"))]
        let actions_str = effect
            .compound
            .actions
            .as_ref()
            .map(|a| {
                a.iter()
                    .map(|s| s.action.to_str())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();
        #[cfg(not(feature = "no_std"))]
        if ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) { log::debug!("[DEBUG_SEQ] execute_sequential_effect called! action={} n_actions={} actions=[{}] effect_ptr={:p}",
            effect.action,
            effect.compound.actions.as_ref().map(|a| a.len()).unwrap_or(0),
            actions_str,
            effect
        ); }
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
            .map(|c| c.name.to_string());

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
                .is_some_and(|a| a.action == ActionType::RepeatProcedure);
            let repeat_max = if has_repeat {
                // repeat_limit = max additional iterations (e.g. 4 = 4 more times)
                // Total iterations = initial + max_repeats
                actions
                    .last()
                    .and_then(|a| a.repeat_limit_any())
                    .unwrap_or(1)
                    + 1
            } else {
                1
            };
            let repeat_actions = if has_repeat {
                &actions[..actions.len() - 1]
            } else {
                actions.as_slice()
            };

            if has_repeat {
                self.pending_repeat_actions.clear();
            }
            log::debug!(
                "[ABILITY] sequential: {} actions, repeat_max={} card_id={:?}",
                repeat_actions.len(),
                repeat_max,
                self.activating_card_id
            );
            // Track if a preceding conditional step was satisfied, for
            // if-then-else (otherwise_condition) support.
            let mut condition_failed: Option<bool>;
            for repeat_idx in 0..repeat_max {
                let repeats_remaining = repeat_max.saturating_sub(repeat_idx + 1);
                // Reset the conditional-failed flag at the start of each repeat
                // iteration so a failed result_condition in one iteration doesn't
                // cause the next iteration's actions to be skipped.
                condition_failed = None;
                'action_loop: for (i, action) in repeat_actions.iter().enumerate() {
                    if ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) { log::debug!("[DEBUG_RA] i={} action={} len={}",
                        i,
                        action.action,
                        repeat_actions.len()
                    ); }
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!(
                            "[SEQ_TRACE] repeat_idx={} i={} action={}",
                            repeat_idx,
                            i,
                            action.action
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
                    let is_otherwise = action
                        .condition
                        .as_ref()
                        .is_some_and(|c| matches!(c.as_ref(), Condition::AlwaysTrue { .. }));
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
                        // Don't reset condition_failed — keep it so ALL subsequent
                        // conditionless actions in this conditional sequential are
                        // skipped, not just the very next one.
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
                            && repeat_actions[i - 1].condition.as_ref()
                                == action.condition.as_ref();
                        if same_as_prev {
                            if condition_failed == Some(true) {
                                continue 'action_loop;
                            }
                        } else {
                            let cond = action.condition.as_ref().unwrap();
                            // Check cache first — avoids re-evaluation against stale
                            // game state after a choice round-trip.
                            let passed = if cond.get_cache().unwrap_or(false) {
                                if let Some(entry) = gs.ability_queue.current_entry() {
                                    if let Some(&(_, cached)) = entry
                                        .condition_cache
                                        .iter()
                                        .find(|(k, _)| {
                                            let cur_key = format!("{:?}", cond);
                                            k == &cur_key
                                        })
                                    {
                                        cached
                                    } else {
                                        false // not cached yet — evaluate below
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                            let passed = if passed {
                                true
                            } else {
                                let ctx = ConditionContext::with_moved_cards(gs, &self.moved_cards);
                                let p = ctx.evaluate_condition(cond);
                                // Cache the result if condition asks for it
                                if cond.get_cache().unwrap_or(false) {
                                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                        let key = format!("{:?}", cond);
                                        entry.condition_cache.retain(|(k, _)| k != &key);
                                        entry.condition_cache.push((key, p));
                                    }
                                }
                                p
                            };
                            if !action.optional.unwrap_or(false) {
                                condition_failed = Some(!passed);
                            }
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
                        action.action,
                        ActionType::DrawCard
                            | ActionType::GainResource
                            | ActionType::ModifyScore
                            | ActionType::ModifyRequiredHearts
                            | ActionType::GainAbility
                            | ActionType::SetBladeCount
                            | ActionType::LookAt
                    );
                    if supports_per_unit {
                        if action_to_execute.per_unit_any().is_none()
                            && effect.per_unit_any().is_some()
                        {
                            action_to_execute.set_per_unit(effect.per_unit_any());
                        }
                        if action_to_execute.per_unit_count_any().is_none()
                            && effect.per_unit_count_any().is_some()
                        {
                            action_to_execute.set_per_unit_count(effect.per_unit_count_any());
                        }
                        if action_to_execute.per_unit_type_any().is_none()
                            && effect.per_unit_type_any().is_some()
                        {
                            action_to_execute
                                .set_per_unit_type(effect.per_unit_type_any().map(|s| s.into()));
                        }
                        // distinct rides on the per-unit wrapper for
                        // ModifyRequiredHearts (e.g. ディストーション:
                        // "名前の異なる『CatChu!』のメンバー1人につき" applies to BOTH the
                        // heart00-decrease and heart02-increase sub-actions). Without
                        // inheriting it, duplicate-named members count twice.
                        // Deliberately NOT inherited for gain_resource — there,
                        // distinct filters TARGET SELECTION (self_and_other patterns
                        // carry it explicitly per-action) and blind inheritance
                        // breaks "メンバー1人" picking.
                        if action.action == ActionType::ModifyRequiredHearts
                            && action_to_execute.distinct_any().is_none()
                        {
                            if let Some(d) = effect.distinct_any() {
                                if let Some(f) = action_to_execute
                                    .kind
                                    .as_deref_mut()
                                    .and_then(|k| k.filter_mut())
                                {
                                    f.distinct = Some(Box::new(d));
                                }
                            }
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
                    if action_to_execute.self_target_any().is_none() {
                        let inheritable = if i > 0 {
                            let first_ct_binding = repeat_actions[0].card_type_any();
                            let first_ct = first_ct_binding;
                            let cur_ct_binding = action.card_type_any();
                            let cur_ct = cur_ct_binding;
                            // If the first action targets a live card (no card_type
                            // or live_card) and the current targets a member, don't
                            // inherit — they're different cards entirely.
                            !(first_ct != Some(&crate::card::CardType::Member)
                                && cur_ct == Some(&crate::card::CardType::Member))
                        } else {
                            true
                        };
                        if inheritable {
                            // Only inherit self_target to action types that actually
                            // support it. "このカード" (this card) in a compound sentence
                            // applies to score/heart modifiers, not to generic draw or
                            // move actions. Prevent cascading into nested sequentials
                            // (draw+move inside would inherit from the nested container).
                            let supports_self = matches!(
                                action.action,
                                ActionType::ModifyScore
                                    | ActionType::ModifyRequiredHearts
                                    | ActionType::GainResource
                                    | ActionType::ChangeState
                            );
                            if effect.self_target_any().is_some() && supports_self {
                                action_to_execute.set_self_target(effect.self_target_any());
                            } else if i > 0 && supports_self {
                                action_to_execute
                                    .set_self_target(repeat_actions[0].self_target_any());
                            }
                        }
                    }
                    // Inherit card_names from the parent (set at the sequential
                    // level, e.g. EMOTION's "card_names": ["EMOTION"]) so that
                    // per-unit counting in sub-actions only counts matching cards.
                    if action_to_execute
                        .card_names_any()
                        .map_or(true, |v| v.is_empty())
                        && !effect.card_names_any().map_or(true, |v| v.is_empty())
                    {
                        if let Some(names) = effect.card_names_any() {
                            action_to_execute.set_card_names(names.clone());
                        }
                    }

                    log::debug!(
                        "[SEQ_LOOP] executing action[{}] action={} pending_before={:?}",
                        i,
                        action.action,
                        self.pending_choice.is_some()
                    );
                    fn save_remaining(
                        gs: &mut crate::game_state::GameState,
                        remaining: Vec<Box<AbilityEffect>>,
                    ) {
                        log::debug!(
                            "[SAVE_REMAINING] count={} actions={:?}",
                            remaining.len(),
                            remaining.iter().map(|a| a.action).collect::<Vec<_>>()
                        );
                        if !remaining.is_empty() {
                            let mut existing = gs.ability_queue.take_pending_actions();
                            existing.extend(remaining.into_iter().map(|b| *b));
                            gs.ability_queue.set_pending_actions(existing);
                        }
                    }

                    // G3: before executing an opponent-action sub-action, tag the spawn
                    // context so that any choice created inside is routed to the opponent.
                    if action.action == ActionType::OpponentAction
                        || action.action_by().as_deref() == Some("opponent")
                    {
                        self.spawn_context.target = Some("opponent".to_string());
                    }

                    let moved_before = self.moved_cards.len();
                    let selected_before = self.selected_cards.len();
                    // "…したとき" (when you do so) gate: a consequence step that
                    // immediately follows a move is only applied when that move
                    // actually moved a card. Consequence shapes: a modify_score
                    // (G13 "そうしたとき +1") and a recover-self move
                    // (G7 "そうしたとき 控え室からこのカードを手札に加える").
                    // The flag is set by execute_move_cards and cleared after every
                    // sub-action so it never gates an unrelated later step.
                    let is_gated_consequence = action.action == ActionType::ModifyScore
                        || (action.action == ActionType::MoveCards
                            && action.destination == Some(Zone::Hand)
                            && action.is_self_target()
                            && action.source_any().is_some_and(|s| {
                                s == "discard" || s == "waitroom"
                            }));
                    if is_gated_consequence && self.last_move_moved_any == Some(false) {
                        self.last_move_moved_any = None;
                        continue 'action_loop;
                    }
                    self.last_move_moved_any = None;
                    match self.execute_effect(gs, &action_to_execute) {
                        Ok(_) => {
                            if ABILITY_DEBUG.load(Ordering::Relaxed) {
                                log::debug!(
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
                            if let Some(ref step_id) = action.id_any() {
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
                                    .entry(step_id.to_string())
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
                                let is_opponent_action = action.action
                                    == ActionType::OpponentAction
                                    || action.action_by().as_deref() == Some("opponent");
                                // Parent-conditional (そうした場合) gating: when
                                // the gate could not be evaluated yet (the move
                                // deferred to a selection), arm the deferred
                                // gate so the answer handler attributes the
                                // outcome (empty/skip => drop remaining).
                                if conditional
                                    && action.condition.is_none()
                                    && condition_failed.is_none()
                                    && !is_opponent_action
                                {
                                    self.deferred_conditional_gate = true;
                                }
                                // Some handlers fully execute the effect during choice
                                // resolution (SelectCard moves cards, PositionChange swaps
                                // members). Re-executing with optional=None would duplicate
                                // the effect. Detect by Choice variant — the variant
                                // encodes whether the handler completed the work.
                                let completes_in_handler =
                                    self.pending_choice.as_ref().is_some_and(|c| {
                                        matches!(
                                            c,
                                            crate::ability::types::Choice::SelectCard { .. }
                                        )
                                    }) || matches!(
                                        self.pending_choice.as_ref(),
                                        Some(crate::ability::types::Choice::SelectTarget {
                                            target,
                                            ..
                                        }) if target == "position|destination"
                                    );
                                let mut remaining = if current_was_optional
                                    && i + 1 < repeat_actions.len()
                                    && !is_opponent_action
                                    && !completes_in_handler
                                {
                                    let mut actions: Vec<Box<AbilityEffect>> =
                                        repeat_actions[i..].to_vec();
                                    if !actions.is_empty() {
                                        actions[0].set_optional(None);
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
                                        !a.condition.as_ref().is_some_and(|c| {
                                            matches!(c.as_ref(), Condition::AlwaysTrue { .. })
                                        })
                                    });
                                }
                                // Strip AllRevealedMatchHeartColor conditions from
                                // saved pending actions only when the sequential loop
                                // already evaluated a condition and it passed
                                // (condition_failed == Some(false)). When condition_failed
                                // is None, no conditions have been evaluated yet — the
                                // gating must be preserved for proper re-evaluation
                                // during resume_pending_actions.
                                if condition_failed == Some(false) {
                                    for a in &mut remaining {
                                        if a.condition.as_ref().is_some_and(|c| {
                                            matches!(
                                                c.as_ref(),
                                                Condition::AllRevealedMatchHeartColor { .. }
                                            )
                                        }) {
                                            a.condition = None;
                                        }
                                    }
                                }
                                // Store remaining repeats on the resolver for
                                // one-at-a-time feeding after each iteration completes.
                                // We DON'T pre-load them into pending_commands — that
                                // causes duplication with RPC's merge logic.
                                if repeats_remaining > 0 && has_repeat {
                                    if let Some(ref repeat_action) = actions.last() {
                                        if repeat_action.action == ActionType::RepeatProcedure
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
                                    log::debug!("[SEQ_TRACE] cancel_remaining_commands set — aborting sequential loop");
                                }
                                return Ok(());
                            } else if action.optional.unwrap_or(false) {
                                if action.action == ActionType::ChangeState {
                                    // Optional change_state completed without creating a choice
                                    // (no valid targets). Skip remaining actions (そうした場合).
                                    return Ok(());
                                }
                                let was_moved = self.moved_cards.len() - moved_before;
                                if was_moved == 0 {
                                    // Optional action auto-skipped (e.g., empty source).
                                    // Record as "skipped" so conditional_on_optional
                                    // can route correctly without prompting.
                                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                        entry.optional_cost_result = Some(false);
                                    }
                                }
                                // Parent-conditional sequential: track implicit
                                // condition via was_moved (old behavior) or
                                // was_selected (state changes that don't move cards
                                // but do select a card to modify, e.g. change_state).
                                // Only set when condition_failed is still None (first
                                // gating action only) — subsequent actions should NOT
                                // overwrite the gate result.
                                if conditional
                                    && action.condition.is_none()
                                    && condition_failed.is_none()
                                {
                                    let was_moved = self.moved_cards.len() - moved_before;
                                    let was_selected = self.selected_cards.len() - selected_before;
                                    condition_failed = Some(was_moved == 0 && was_selected == 0);
                                }
                            } else if condition_failed.is_none()
                                && !self.pending_choice.is_some()
                                && conditional
                                && action.condition.is_none()
                            {
                                // Parent-conditional sequential: track implicit
                                // condition via was_moved (old behavior).
                                let was_moved = self.moved_cards.len() - moved_before;
                                let was_selected = self.selected_cards.len() - selected_before;
                                condition_failed = Some(was_moved == 0 && was_selected == 0);
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
                // After all actions in this iteration, check if repeat_procedure is optional
                // and we have remaining repeats — ask player whether to continue.
                if repeats_remaining > 0 {
                    if let Some(ref repeat_action) = actions.last() {
                        if repeat_action.action == ActionType::RepeatProcedure
                            && repeat_action.optional.unwrap_or(false)
                        {
                            for _ in 0..repeats_remaining {
                                self.pending_repeat_actions
                                    .extend(repeat_actions.iter().cloned());
                            }
                            self.pending_choice = Some(Choice::SelectTarget {
                                target: crate::ability::types::PAY_SKIP_TARGET.to_string(),
                                description: "Repeat effect?".to_string(),
                                description_en: Some("Repeat effect?".to_string()),
                                description_ja: Some("効果を繰り返しますか？".to_string()),
                                allow_skip: true,
                                options: Some(vec!["Stop".to_string(), "Continue".to_string()]),
                            });
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.choice_card_no =
                                    Some(crate::ability::types::ChoiceRoute::Raw(
                                        "pay_optional_cost".to_string(),
                                    ));
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

    // Rule 1.3.2 / Q235 / Q236 / Q237: Conditional alternative effect
    //
    // Implements "if X → effect A, otherwise → effect B" pattern from card text.
    // The condition refers to `preceding_moved` — the card moved by the cost's
    // `move_cards` sub-action (e.g. discarding from hand to waitroom).
    //
    // Tiered evaluation (when both alternative_condition and condition exist):
    //   1. Check alternative_condition first (stricter)
    //   2. If met → execute alternative_effect, skip primary
    //   3. If not met → check base condition
    //   4. If base met → execute primary_effect
    //   5. If neither → do nothing (Rule 1.3.2: impossible actions aren't done)
    //
    // Legacy (single condition): condition=true → alternative_effect,
    //   condition=false → primary_effect. Note: the condition's `negation` field
    //   determines which branch fires — if negation=false, condition=true fires
    //   alternative_effect; if negation=true, condition=true fires primary_effect.
    //
    // Example (the μ's ability from the user's question):
    //   Cost: sequential_cost[pay_energy(2), move_cards(hand→discard, 1)]
    //   Effect: conditional_alternative
    //     condition: group_condition(μ's, source=preceding_moved, negation=false)
    //     primary_effect: look_and_select(4→2, hand, rest→discard)
    //     alternative_effect: move_cards(discard→hand, 1, live_card)
    //   Flow: discard card → check if it has μ's group →
    //     Yes → look 4, select 2 to hand
    //     No  → recover 1 live card from waitroom
    pub fn execute_conditional_alternative(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
    ) -> Result<(), String> {
        let has_primary = effect.compound.primary_effect.is_some();
        let has_alternative = effect.alternative_effect_any().is_some();

        if has_primary && has_alternative {
            let ctx = super::condition::ConditionContext::with_moved_cards(gs, &self.moved_cards);

            // Tiered conditions: alternative_condition (stricter) checked first.
            // If met → execute alternative_effect (replaces primary).
            // Otherwise check the base condition — if met → execute primary_effect.
            // If neither is met → do nothing (Rule 1.3.2).
            if effect.compound.alternative_condition.is_some() && effect.condition.is_some() {
                if let Some(ref alt_cond) = effect.compound.alternative_condition {
                    if ctx.evaluate_condition(alt_cond) {
                        if let Some(ref alt_effect) = effect.alternative_effect_any() {
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
                .as_deref()
                .or(effect.condition.as_deref());
            if let Some(cond) = single_cond {
                if ctx.evaluate_condition(cond) {
                    if let Some(ref alt_effect) = effect.alternative_effect_any() {
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
                .map(|e| e.text.as_ref())
                .unwrap_or("Primary effect");
            let alternative_text = effect
                .alternative_effect_any()
                .as_ref()
                .map(|e| e.text.as_ref())
                .unwrap_or("Alternative effect");
            let description = format!(
                "Choose effect:\nPrimary: {}\nAlternative: {}",
                primary_text, alternative_text
            );
            self.pending_choice = Some(Choice::SelectTarget {
                target: "primary|alternative".to_string(),
                description,
                description_en: Some(format!(
                    "Choose effect:\nPrimary: {}\nAlternative: {}",
                    primary_text, alternative_text
                )),
                description_ja: Some(format!(
                    "効果を選択:\nPrimary: {}\nAlternative: {}",
                    primary_text, alternative_text
                )),
                allow_skip: effect.optional.unwrap_or(false),
                options: None,
            });
            self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
            return Ok(());
        }

        if let Some(ref alt_condition) = effect.compound.alternative_condition {
            let ctx = super::condition::ConditionContext::with_moved_cards(gs, &self.moved_cards);
            if ctx.evaluate_condition(alt_condition) {
                if let Some(ref alt_effect) = effect.alternative_effect_any() {
                    return self.execute_effect(gs, alt_effect);
                }
            }
        }

        // Handle the case where we have alternative_effect + top-level condition
        // but no primary_effect and no alternative_condition (e.g. replacement effects
        // like 錯覚CROSSROADS: "when this card would be placed in success zone, instead...")
        if effect.alternative_effect_any().is_some()
            && effect.compound.primary_effect.is_none()
            && effect.compound.alternative_condition.is_none()
        {
            if let Some(ref cond) = effect.condition {
                let ctx =
                    super::condition::ConditionContext::with_moved_cards(gs, &self.moved_cards);
                if ctx.evaluate_condition(cond) {
                    if let Some(ref alt_effect) = effect.alternative_effect_any() {
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
    ) -> Result<(), String> {
        let repeat_limit = effect.repeat_limit_any().unwrap_or(1) as usize;
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
                finish.condition = None;
                gs.ability_queue.save_pending_actions(vec![finish]);
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
                // Clear selected_cards so the followup doesn't inherit
                // the primary effect's targets (e.g. change_state targeting
                // the deployed card instead of "this member").
                self.selected_cards.clear();
                self.execute_effect(gs, followup)?;
            }
        } else {
            log::debug!("Result condition not met, skipping followup action");
        }
        // Clear conditional_choice so the next repeat iteration creates a
        // fresh optional prompt instead of silently reusing the previous answer.
        if let Some(entry_mut) = gs.ability_queue.current_entry_mut() {
            entry_mut.conditional_choice = None;
        }
        Ok(())
    }

    // Q92 / Q217: Conditional-on-optional effect
    //
    // Implements "may [pay cost]: if you do → A, if you don't → B" patterns.
    //
    // Q92: If the optional action requires energy and the player can't afford
    //   it, skip the choice entirely and execute the conditional action directly.
    //   This prevents pointless "Pay 5 energy?" prompts when player has 0 active.
    //
    // Q217: "好きな枚数～" (any number) cost: choosing 0 still counts as
    //   "cost was paid" (the player chose to pay 0). This triggers the
    //   conditional action branch.
    //
    // Negation mode: when `is_negation` = true, paying the optional action
    //   routes to the optional_action instead of the conditional_action.
    //   This implements "unless you pay → do B" patterns.
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
            if opt.action == ActionType::PayEnergy {
                let need = opt.energy_count_any().unwrap_or(0) as usize;
                if need > 0 {
                    let pp = gs.player_prefix();
                    let active = gs
                        .resolve_target_player(opt.target.as_deref().unwrap_or("self"))
                        .energy_zone
                        .active_count() as usize;
                    if active < need {
                        gs.push_rule_log(format!(
                            "{}: [[log_cost_skip:reason=compound_insufficient_energy,need={},active={}]]",
                            pp, need, active
                        ));
                        let cmd = *cond.clone();
                        gs.ability_queue.set_pending_actions(vec![cmd]);
                        return self.resume_pending_actions(gs);
                    }
                }
            }
        }

        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
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
                    (true, true) => effect.compound.optional_action.map(|a| *a),
                    (true, false) => effect.compound.conditional_action.map(|a| *a),
                    (false, true) => effect.compound.conditional_action.map(|a| *a),
                    (false, false) => None,
                };
                if let Some(cmd) = cmd {
                    gs.ability_queue.set_pending_actions(vec![cmd]);
                }
                return self.resume_pending_actions(gs);
            }
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(ConditionalChoice::Effect(effect.clone()));
            }
            self.pending_choice = Some(Choice::SelectTarget {
                target: "conditional_optional".to_string(),
                description: "Pay optional cost or skip".to_string(),
                description_en: Some("Pay optional cost or skip".to_string()),
                description_ja: Some("オプションコストを支払うかスキップ".to_string()),
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
        conditional_choice: Option<ConditionalChoice>,
    ) -> Result<(), String> {
        if let Some(ConditionalChoice::Strings(options)) = conditional_choice {
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
        self.pending_choice = None;
        self.clear_choice_meta(gs);
        self.resume_pending_actions(gs)?;
        Ok(())
    }

    pub fn handle_choice_string_store(
        &mut self,
        gs: &mut GameState,
        selected: &str,
        conditional_choice: Option<ConditionalChoice>,
    ) -> Result<(), String> {
        let chosen = conditional_choice.and_then(|cc| match cc {
            ConditionalChoice::Strings(opts) => selected
                .parse::<usize>()
                .ok()
                .and_then(|idx| opts.get(idx).cloned()),
            _ => None,
        });
        log::debug!(
            "[DBG_CHOICE] handle_choice_string_store: selected={} chosen_is_some={}",
            selected,
            chosen.is_some()
        );
        if let Some(ref s) = chosen {
            gs.ability_queue
                .current_entry_mut()
                .map(|e| e.conditional_choice = Some(ConditionalChoice::Str(s.clone())));
            log::debug!("[DBG_CHOICE] stored conditional_choice");
        }
        self.pending_choice = None;
        self.resume_pending_actions(gs)?;
        Ok(())
    }
}
