//! Common gate and check functions for ability resolution.
//! Extracts repeated patterns from resolver.rs into reusable, composable units.

use crate::ability::types::Choice;
use crate::card::{Ability, AbilityEffect};
use crate::game_state::GameState;

#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};

/// Result of a gate check — either continue or stop with a reason.
#[derive(Debug, Clone, PartialEq)]
pub enum GateResult {
    Continue,
    Stop { reason: String, result_type: String },
}

impl GateResult {
    #[inline]
    pub fn stop(reason: impl Into<String>, result_type: impl Into<String>) -> Self {
        Self::Stop { reason: reason.into(), result_type: result_type.into() }
    }

    #[inline]
    pub fn is_continue(&self) -> bool { matches!(self, Self::Continue) }
    #[inline]
    pub fn is_stop(&self) -> bool { matches!(self, Self::Stop { .. }) }
    
    #[inline]
    pub fn unwrap_stop(self) -> (String, String) {
        match self {
            Self::Stop { reason, result_type } => (reason, result_type),
            Self::Continue => panic!("Called unwrap_stop on Continue"),
        }
    }
}

/// Unified gate function type for composability.
pub type AbilityGateFn = dyn Fn(&mut crate::ability::resolver::AbilityResolver, &mut GameState, &Ability) -> GateResult + Send + Sync;
pub type EffectGateFn = dyn Fn(&mut crate::ability::resolver::AbilityResolver, &mut GameState, &AbilityEffect) -> GateResult + Send + Sync;

/// Composite gate that runs multiple gates in sequence.
pub struct CompositeGate<T: ?Sized> {
    gates: Vec<Box<T>>,
}

impl<T: ?Sized> CompositeGate<T> {
    pub fn new(gates: Vec<Box<T>>) -> Self {
        Self { gates }
    }
}

impl CompositeGate<AbilityGateFn> {
    #[inline]
    pub fn check_ability(&self, resolver: &mut crate::ability::resolver::AbilityResolver, gs: &mut GameState, ability: &Ability) -> GateResult {
        for gate in &self.gates {
            let result = gate(resolver, gs, ability);
            if result.is_stop() {
                return result;
            }
        }
        GateResult::Continue
    }
}

impl CompositeGate<EffectGateFn> {
    #[inline]
    pub fn check_effect(&self, resolver: &mut crate::ability::resolver::AbilityResolver, gs: &mut GameState, effect: &AbilityEffect) -> GateResult {
        for gate in &self.gates {
            let result = gate(resolver, gs, effect);
            if result.is_stop() {
                return result;
            }
        }
        GateResult::Continue
    }
}

/// Check use limit gate.
#[inline]
pub fn use_limit_gate(
    resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    ability: &Ability,
) -> GateResult {
    let Some(card_id) = resolver.activating_card_id else { return GateResult::Continue };
    let Some(use_limit) = ability.use_limit else { return GateResult::Continue };
    let ability_index = resolver.current_ability_index.unwrap_or(0);
    
    if gs.ability_uses_used(card_id, ability_index) >= use_limit {
        let used = gs.ability_uses_used(card_id, ability_index);
        GateResult::stop(
            format!("Ability already used {} of {} times this turn", used, use_limit),
            "skipped"
        )
    } else {
        GateResult::Continue
    }
}

/// Check activation keywords (Center/LeftSide/RightSide/Turn1/Turn2/Debut/LiveStart/LiveSuccess/PositionChange/FormationChange).
#[inline]
pub fn activation_keywords_gate(
    resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    ability: &Ability,
) -> GateResult {
    let Some(card_id) = resolver.activating_card_id else { return GateResult::Continue };
    let position = gs.find_card_stage_position(card_id);
    
    let binding = vec![];
    let keywords = ability.keywords.as_ref().unwrap_or(&binding);
    if resolver.check_keywords(gs, keywords, position) {
        GateResult::Continue
    } else {
        GateResult::stop("Activation keywords not satisfied", "position_fail")
    }
}

/// Check effect condition gate.
#[inline]
pub fn effect_condition_gate(
    resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    ability: &Ability,
) -> GateResult {
    let Some(ref effect) = ability.effect else { return GateResult::Continue };
    
    if !resolver.can_activate_effect(gs, effect) {
        // For activation abilities, record use limit even on condition failure
        record_use_limit_if_activation(gs, resolver, ability);
        GateResult::stop("effect condition not met", "failure")
    } else {
        GateResult::Continue
    }
}

/// Record use limit for activation abilities when condition fails.
#[inline]
pub fn record_use_limit_if_activation(
    gs: &mut GameState,
    resolver: &crate::ability::resolver::AbilityResolver,
    ability: &Ability,
) {
    if ability.use_limit.is_some() && ability.has_trigger(crate::triggers::TriggerKind::Activation) {
        if let Some(card_id) = resolver.activating_card_id {
            let ability_index = resolver.current_ability_index.unwrap_or(0);
            gs.record_ability_use((card_id, ability_index, gs.turn_number));
        }
    }
}

/// Check post-cost position keywords gate.
#[inline]
pub fn post_cost_position_gate(
    resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    ability: &Ability,
) -> GateResult {
    let Some(card_id) = resolver.activating_card_id else { return GateResult::Continue };
    
    // We need to check position keywords after cost payment
    // Use a simplified check here
    let position = gs.find_card_stage_position(card_id);
    let binding = vec![];
    let keywords = ability.keywords.as_ref().unwrap_or(&binding);
    
    // Only check position-related keywords post-cost
    let position_keywords: Vec<crate::card::Keyword> = keywords.iter()
        .filter(|k| matches!(k, crate::card::Keyword::Center | crate::card::Keyword::LeftSide | crate::card::Keyword::RightSide))
        .cloned()
        .collect();
        
    if !position_keywords.is_empty() && !resolver.check_keywords(gs, &position_keywords, position) {
        GateResult::stop("position requirement not met", "position_fail")
    } else {
        GateResult::Continue
    }
}

/// Check optional cost skip gate.
#[inline]
pub fn optional_cost_skip_gate(
    _resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    _ability: &Ability,
) -> GateResult {
    let cost_was_skipped = gs
        .ability_queue
        .current_entry()
        .is_some_and(|e| e.optional_cost_result == Some(false));
    
    if cost_was_skipped {
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.effect_started = true;
        }
        GateResult::stop("optional cost not paid", "skipped")
    } else {
        GateResult::Continue
    }
}

/// Check non-stackable effect gate.
#[inline]
pub fn non_stackable_gate(
    _resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> GateResult {
    if !effect.non_stackable.unwrap_or(false) {
        return GateResult::Continue;
    }
    let effect_key = format!("{}:{}", effect.action, effect.text);
    if gs.non_stackable_effects.iter().any(|x| x == &effect_key) {
        GateResult::stop("non-stackable effect already active", "skipped")
    } else {
        gs.non_stackable_effects.push(effect_key);
        GateResult::Continue
    }
}

/// Check incomplete placement gate (Q118).
#[inline]
pub fn incomplete_placement_gate(
    _resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    effect: &AbilityEffect,
) -> GateResult {
    let placement_incomplete = gs
        .ability_queue
        .current_entry()
        .and_then(|e| e.optional_moves_all_moved)
        == Some(false);
    
    if placement_incomplete && matches!(effect.action, crate::ability::enums::ActionType::DrawCard | crate::ability::enums::ActionType::DrawUntilCount) {
        GateResult::stop("placement incomplete (Q118)", "skipped")
    } else {
        GateResult::Continue
    }
}

/// Pre-cost gates (run before cost payment).
#[inline]
pub fn pre_cost_gates() -> CompositeGate<AbilityGateFn> {
    CompositeGate::new(vec![
        Box::new(use_limit_gate),
        Box::new(activation_keywords_gate),
    ])
}

/// Post-cost gates (run after cost payment, before effect).
#[inline]
pub fn post_cost_gates() -> CompositeGate<AbilityGateFn> {
    CompositeGate::new(vec![
        Box::new(post_cost_position_gate),
        Box::new(optional_cost_skip_gate),
    ])
}

/// Effect execution gates (run before each effect).
#[inline]
pub fn effect_gates() -> CompositeGate<EffectGateFn> {
    CompositeGate::new(vec![Box::new(incomplete_placement_gate)])
}

/// Unified use limit recording logic.
/// 
/// Parameters:
/// - `phase`: when to record - "early" (before effect), "pending" (on choice), "final" (after effect)
/// - `cost_already_paid`: whether cost was already paid in a previous resolution
/// - `is_conditional_optional`: whether this is a ConditionalOnOptional effect
/// - `is_optional_effect`: whether the effect itself is optional
/// - `choice_target`: the target of pending choice (for "pending" phase)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseLimitPhase {
    Early,
    Pending,
    Final,
}

#[inline]
pub fn record_use_limit(
    resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    ability: &Ability,
    ability_key: Option<(i16, usize, u8)>,
    cost_already_paid: bool,
    phase: UseLimitPhase,
    choice_target: Option<&str>,
) {
    let Some(ref key) = ability_key else { return };
    if ability.use_limit.is_none() { return; }

    let is_conditional_optional = ability
        .effect
        .as_ref()
        .is_some_and(|e| e.action == crate::ability::enums::ActionType::ConditionalOnOptional);
    let is_optional_effect = ability
        .effect
        .as_ref()
        .is_some_and(|e| e.optional.unwrap_or(false));

    match phase {
        UseLimitPhase::Early => {
            // Early recording: only for non-conditional, non-optional, no pending choice, cost not paid
            if cost_already_paid || resolver.pending_choice.is_some() || is_conditional_optional || is_optional_effect {
                return;
            }
            let can_activate = ability.effect.as_ref().is_none_or(|e| resolver.can_activate_effect(gs, e));
            if can_activate {
                gs.record_ability_use(*key);
            }
        }
        UseLimitPhase::Pending => {
            // On pending choice: skip for conditional_optional and optional position choices
            let skip_use_limit = matches!(
                choice_target,
                Some("conditional_optional") | Some("position|destination") if is_optional_effect
            );
            if !skip_use_limit {
                gs.record_ability_use(*key);
            }
        }
        UseLimitPhase::Final => {
            // Final recording: after effect execution
            let can_activate = ability.effect.as_ref().is_none_or(|e| resolver.can_activate_effect(gs, e));
            let should_record = if cost_already_paid && is_conditional_optional {
                // For ConditionalOnOptional where cost was already paid, record regardless of can_activate
                true
            } else {
                can_activate
            };
            if should_record {
                gs.record_ability_use(*key);
            }
        }
    }
}

/// Handle pending choice from cost or effect execution.
/// 
/// `is_cost_choice`: true for cost choices, false for effect choices
#[inline]
pub fn handle_pending_choice(
    resolver: &mut crate::ability::resolver::AbilityResolver,
    gs: &mut GameState,
    ability: &Ability,
    ability_key: Option<(i16, usize, u8)>,
    cost_already_paid: bool,
    is_cost_choice: bool,
) -> bool {
    // Called twice per ability resolution; the early-out below means this is
    // almost always free, so its cost is only interesting when it is not.
    #[cfg(not(feature = "no_std"))]
    let _timer = crate::timer::Timer::start("resolve::handle_pending_choice");
    if resolver.pending_choice.is_none() {
        return false;
    }

    if is_cost_choice && !cost_already_paid {
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.cost_paid = true;
        }
    }

    if !is_cost_choice {
        let is_paid = cost_already_paid
            || gs.ability_queue.current_entry().is_some_and(|e| e.cost_paid);
        if is_paid {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.effect_started = true;
            }
        }

        // Handle use limit recording for pending choices
        if !cost_already_paid {
            let choice_target = resolver.pending_choice.as_ref().and_then(|c| match c {
                Choice::SelectTarget { target, .. } => Some(target.as_str()),
                _ => None,
            }).map(|s| s.to_string());
            if let Some(target) = choice_target {
                record_use_limit(resolver, gs, ability, ability_key, cost_already_paid, UseLimitPhase::Pending, Some(&target));
            } else {
                record_use_limit(resolver, gs, ability, ability_key, cost_already_paid, UseLimitPhase::Pending, None);
            }
        }
    }

    resolver.store_pending_choice(gs);
    true
}