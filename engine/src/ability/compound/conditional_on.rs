//! Conditional-on-optional and conditional-on-result effect execution

use crate::ability::enums::ActionType;
use crate::ability::condition::ConditionContext;
use crate::ability::resolver::AbilityResolver;
use crate::ability::types::Choice;
use crate::ability_queue::ConditionalChoice;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

/// Execute conditional-on-result effect.
pub(crate) fn execute_conditional_on_result(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> Result<(), String> {
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
        if let Err(e) = resolver.execute_effect(gs, primary) {
            log::debug!("Primary action failed in conditional_on_result: {}", e);
            return Err(e);
        }
        if resolver.pending_choice.is_some() {
            let mut finish = effect.clone();
            finish.compound.primary_effect = None;
            finish.condition = None;
            gs.ability_queue.save_pending_actions(vec![finish]);
            return Ok(());
        }
    }

    let condition_met = result_condition
        .map(|c| {
            if let Some(reference) = c.get_action_reference() {
                resolver
                    .last_action_result
                    .is_some_and(|(action, succeeded)| action.to_str() == reference && succeeded)
            } else {
                let ctx = ConditionContext::with_moved_cards(gs, &resolver.moved_cards);
                ctx.evaluate_condition(c)
            }
        })
        .unwrap_or(true);

    if condition_met {
        if let Some(followup) = followup_action {
            resolver.selected_cards.clear();
            resolver.execute_effect(gs, followup)?;
        }
    } else {
        log::debug!("Result condition not met, skipping followup action");
    }
    if let Some(entry_mut) = gs.ability_queue.current_entry_mut() {
        entry_mut.conditional_choice = None;
    }
    Ok(())
}

/// Q92 / Q217: Conditional-on-optional effect
pub(crate) fn execute_conditional_on_optional(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> Result<(), String> {
    let optional_action = effect.compound.optional_action.as_ref();
    let conditional_action = effect.compound.conditional_action.as_ref();
    let is_negation = effect.compound.conditional_negation.unwrap_or(false);

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
                    log::debug!("[CONDITION] source={:?} action={} branch=conditional next_action={} reason=insufficient_energy active={} need={} negation={}", resolver.activating_card_id, effect.action, cond.action, active, need, is_negation);
                    gs.push_rule_log_fmt(format_args!(
                        "{}: [[log_cost_skip:reason=compound_insufficient_energy,need={},active={}]]",
                        pp, need, active
                    ));
                    let cmd = *cond.clone();
                    gs.ability_queue.set_pending_actions(vec![cmd]);
                    return resolver.resume_pending_actions(gs);
                }
            }
        }
    }

    if optional_action.is_some() && conditional_action.is_some() {
        let result = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.optional_cost_result);
        if let Some(cost_was_paid) = result {
            let chose_yes = cost_was_paid;
            let cmd = crate::ability::compound::conditional::route_conditional_branch(effect, chose_yes, is_negation);
            log::debug!("[CONDITION] source={:?} action={} answer=stored accepted={} negation={} branch={} next_action={:?}", resolver.activating_card_id, effect.action, chose_yes, is_negation, if cmd.is_none() { "none" } else if chose_yes && is_negation { "optional" } else { "conditional" }, cmd.as_ref().map(|a| a.action));
            if let Some(cmd) = cmd {
                gs.ability_queue.set_pending_actions(vec![*cmd]);
            }
            return resolver.resume_pending_actions(gs);
        }
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.conditional_choice = Some(ConditionalChoice::Effect(effect.clone()));
        }
        resolver.pending_choice = Some(Choice::SelectTarget {
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
        resolver.execute_effect(gs, optional)?;
    }
    if let Some(conditional) = conditional_action {
        if !is_negation {
            resolver.execute_effect(gs, conditional)?;
        }
    }
    Ok(())
}

/// Handle choice string selection for prohibition effects.
pub(crate) fn handle_choice_string_selection(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    selected: &str,
    conditional_choice: Option<ConditionalChoice>,
) -> Result<(), String> {
    log::debug!("[HANDLE_CHOICE_STRING_SELECTION] selected={} conditional_choice={:?} parent_effect_or_card_types={:?} current_effect_or_card_types={:?}", 
        selected, conditional_choice, 
        resolver.parent_effect.as_ref().and_then(|e| e.or_card_types_any()),
        resolver.current_effect.as_ref().and_then(|e| e.or_card_types_any()));
    let is_or_card_types = resolver.parent_effect.as_ref().and_then(|e| e.or_card_types_any()).is_some();
    if let Some(ConditionalChoice::Strings(options)) = conditional_choice {
        log::debug!("[HANDLE_CHOICE_STRING_SELECTION] options={:?}", options);
        if let Ok(idx) = selected.parse::<usize>() {
            log::debug!("[HANDLE_CHOICE_STRING_SELECTION] idx={} options.len()={}", idx, options.len());
            if idx < options.len() {
                let val = &options[idx];
                log::debug!("[HANDLE_CHOICE_STRING_SELECTION] val={}", val);
                if val.starts_with("heart")
                    || ["赤", "桃", "緑", "青", "黄", "紫"].contains(&val.as_str())
                {
                    gs.prohibition_effects
                        .push(format!("selected_heart_color:{}", val));
                }
                // Store the selected option for or_card_types handling
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.conditional_choice = Some(ConditionalChoice::Str(val.clone()));
                }
            }
        }
    }
    // Re-queue the parent effect if it's a look_and_select (has or_card_types)
    // so that execute_look_and_select continues after the or_card_types choice.
    if is_or_card_types {
        log::debug!("[HANDLE_CHOICE_STRING_SELECTION] Re-queueing look_and_select effect");
        let mut existing = gs.ability_queue.take_pending_actions();
        existing.push(resolver.parent_effect.clone().unwrap());
        gs.ability_queue.set_pending_actions(existing);
    }
    resolver.pending_choice = None;
    // For or_card_types, preserve conditional_choice for the re-queued effect.
    // Only clear choice_card_no and pending_deferred_costs.
    if is_or_card_types {
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = None;
        }
        resolver.pending_deferred_costs.clear();
    } else {
        resolver.clear_choice_meta(gs);
    }
    resolver.resume_pending_actions(gs)?;
    Ok(())
}

/// Handle choice string store: the number/heart-color pick made by a
/// preceding `select_number` / `select` step. Persists the chosen value as
/// `ConditionalChoice::Str` so later comparison conditions (e.g. Kosuzu's
/// "cost >= chosen number") can read it, then resumes the sequential chain.
pub(crate) fn handle_choice_string_store(
    resolver: &mut AbilityResolver,
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
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.conditional_choice = Some(ConditionalChoice::Str(s.clone()));
        }
        log::debug!("[DBG_CHOICE] stored conditional_choice");
    }
    resolver.pending_choice = None;
    resolver.resume_pending_actions(gs)?;
    Ok(())
}