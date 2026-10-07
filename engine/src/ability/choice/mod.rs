use crate::core::constants::CountCast;
use super::condition::ConditionContext;
use super::enums::{ActionType, Zone};
use super::types::{
    Choice, ChoiceBuilder, ChoiceResult, ChoiceRoute, ExecutionContext, LookAndSelectStep,
};
use super::util;
pub(super) use crate::ability::enums;
pub(super) use crate::ability::compound;
pub(super) use crate::ability::resolver;
use crate::ability::debug::ABILITY_DEBUG;
use crate::ability::queue::ConditionalChoice;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
#[cfg(not(feature = "no_std"))]
use std::borrow::Cow;
#[cfg(feature = "no_std")]
use alloc::{
    borrow::Cow,
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::Ordering;
use smallvec::SmallVec;

mod payment;
mod position;
mod result_handlers;
mod select_card;
mod stage_discard;
mod target;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Continuation {
    Immediate,
    DeferredSelectCard,
    DeferredOther,
}

pub(crate) struct SelectionContext {
    pub card_type: Option<String>,
    pub count: usize,
    pub allow_skip: bool,
    pub indices: Vec<usize>,
    pub cost_limit: Option<u8>,
    pub cost_limit_operator: Option<String>,
    pub cost_total: Option<u8>,
    pub cost_total_operator: Option<String>,
    pub group: Option<String>,
    pub characters: Option<Vec<String>>,
    pub filtered_indices: Option<Vec<usize>>,
    pub is_select_action: bool,
    pub target_player_id: Option<String>,
    pub destination: Option<String>,
    pub discard_remaining: Option<bool>,
    pub blind: bool,
    pub is_reveal: bool,
}

impl SelectionContext {
    pub fn mfi(&self, indices: &[usize]) -> Vec<usize> {
        match &self.filtered_indices {
            Some(fi) => indices.iter().filter_map(|&i| fi.get(i).copied()).collect(),
            None => indices.to_vec(),
        }
    }
}

impl super::resolver::AbilityResolver {
    pub fn resume_execution(
        &mut self,
        _gs: &mut GameState,
        context: ExecutionContext,
    ) -> Result<(), String> {
        // Clear the execution context if finalizing a look_and_select — otherwise the
        // caller's saved_ctx check (actions.rs:369) prevents process_current_ability.
        if matches!(context, ExecutionContext::LookAndSelect { .. })
            && self.awaiting.choice.is_none()
        {
            self.in_flight.execution_context = ExecutionContext::None;
        }
        Ok(())
    }

    /// Resumes executing sequential commands parked on the current queue entry.
    /// If another choice interrupts execution, the remaining actions are safely parked back.
    /// Re-queue the rest of the original batch after a sub-action created its
    /// own choice, or the remaining actions are lost.
    ///
    /// Any remaining action carrying the SAME condition type as the action that
    /// just ran has its condition stripped. Re-evaluating that condition later
    /// would test stale game state — `revealed_cards` mutated by the very
    /// select_cards that just happened — and wrongly reject the action.
    fn requeue_remaining_after_choice(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        remaining: &[AbilityEffect],
    ) {
        let mut queued = gs.ability_queue.take_pending_actions();
        // Append first, then strip the condition from ONLY the newly-appended
        // actions. The original built an intermediate `tail: Vec<AbilityEffect>`
        // via `to_vec()` to mutate it in isolation; slicing `queued[start..]`
        // marks the same span, so the strip still applies to the remaining
        // actions and NOT to whatever was already pending — and the separate
        // `tail` Vec (one allocation plus its buffer) is gone.
        let appended_from = queued.len();
        queued.extend_from_slice(remaining);
        if let Some(cond) = effect.condition.as_ref() {
            let disc = core::mem::discriminant(cond.as_ref());
            for action in &mut queued[appended_from..] {
                if action
                    .condition
                    .as_ref()
                    .is_some_and(|other| core::mem::discriminant(other.as_ref()) == disc)
                {
                    action.condition = None;
                }
            }
        }
        gs.ability_queue.set_pending_actions(queued);
    }

    pub fn resume_pending_actions(&mut self, gs: &mut GameState) -> Result<(), String> {
        let pending = gs.ability_queue.take_pending_actions();
        for (idx, effect) in pending.iter().enumerate() {
            self.in_flight.spawn_context.target = effect.target.clone().map(|s| s.to_string());
            self.execute_effect(gs, effect)?;
            if self.awaiting.choice.is_some() {
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.effect_started = true;
                }
                if idx + 1 < pending.len() {
                    self.requeue_remaining_after_choice(gs, effect, &pending[idx + 1..]);
                }
                return Ok(());
            }
            if self.in_flight.cancel_remaining_commands {
                self.in_flight.cancel_remaining_commands = false;
                return Ok(());
            }
        }
        // All pending commands consumed without creating a new choice.
        // Check if the player just chose "Stop" for a repeat prompt.
        let was_stopped = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.optional_cost_result == Some(false));
        if was_stopped {
            self.awaiting.repeat_actions.clear();
            // Mark effect as started so RWC goes to cost_was_paid
            // (not effect_ready which restarts from scratch).
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.effect_started = true;
            }
        }
        // Feed the next repeat action + "Repeat?" prompt, one at a time.
        if !self.awaiting.repeat_actions.is_empty() && self.awaiting.choice.is_none() {
            log::debug!(
                "[REPEAT] offering another iteration: source={:?} remaining_actions={}",
                self.session.activating_card_id,
                self.awaiting.repeat_actions.len()
            );
            let next = self.awaiting.repeat_actions.remove(0);
            gs.ability_queue.set_pending_actions(vec![*next]);
            self.awaiting.choice = Some(crate::ability::types::repeat_prompt_choice());
        }
        // Set re-prompt choice if one is pending and no other choice was created.
        if let Some(reprompt) = self.awaiting.reprompt.take() {
            if self.awaiting.choice.is_none() {
                self.awaiting.choice = Some(reprompt);
            }
        }
        Ok(())
    }

    /// Shared epilogue: clear pending_choice, resume execution, process pending sequential actions.
    pub(in crate::ability::choice) fn finalize_choice(
        &mut self,
        gs: &mut GameState,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        let is_actual_looked_at_choice = self
            .awaiting.choice
            .as_ref()
            .map(|choice| matches!(choice, Choice::SelectCard { zone, .. } if Zone::from_str(zone) == Some(Zone::LookedAt)))
            .unwrap_or(false);

        let has_pending_sequential = gs.ability_queue.has_pending_actions();

        // Only preserve if this is the initial looked_at selection (not a sub-action choice).
        // Sub-choices created by pending_sequential_actions should NOT be preserved,
        // otherwise the chain never terminates.
        let is_initial_looked_at = is_actual_looked_at_choice
            && matches!(
                context,
                ExecutionContext::LookAndSelect {
                    step: LookAndSelectStep::Select { .. }
                }
            );
        let should_preserve = is_initial_looked_at && has_pending_sequential;

        // Pay any deferred costs (from sequential_cost binary sub-costs) now
        // that the player confirmed the choice.
        //
        // …unless the choice was DECLINED. Rule 9.4.2.2 skips the whole cost in
        // that case, so the binary steps deferred ahead of the choice are dropped
        // rather than paid. Paying them would make 「このメンバーをウェイトにし、手札を
        // 1枚控え室に置いてもよい」 wait the member even when no card was discarded.
        if gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.optional_cost_result == Some(false))
        {
            self.discard_deferred_costs();
        } else {
            self.pay_deferred_costs(gs)?;
        }

        let sub_choice = self.in_flight.sub_choice_created;
        self.in_flight.sub_choice_created = false;
        // The owner snapshot exists only for the span between storing a choice
        // and answering it, and this epilogue is the end of that span. Clearing
        // it here (rather than at each of the many `pending_choice = None`
        // sites) keeps it from going stale into the next choice.
        self.owner.of_pending_choice = None;
        if !should_preserve && !sub_choice {
            self.awaiting.choice = None;
        }
        self.resume_execution(gs, context.clone())?;
        let has_pending = gs.ability_queue.has_pending_actions();
        let was_select_card = matches!(self.awaiting.choice, Some(Choice::SelectCard { .. }));
        let cont = match (sub_choice, has_pending, was_select_card) {
            (true, _, _) => Continuation::Immediate, // sub-choice will handle resume
            (false, true, true) => Continuation::DeferredSelectCard,
            (false, true, false) => Continuation::DeferredOther,
            (false, false, _) => Continuation::Immediate,
        };
        log::debug!(
            "[CHOICE] source={:?} continuation={:?} looked={} preserve={} sub_choice={} queued_actions={} select_card={}",
            self.session.activating_card_id, cont, is_actual_looked_at_choice, should_preserve, sub_choice, has_pending, was_select_card
        );
        match cont {
            Continuation::DeferredSelectCard => {
                self.awaiting.choice = None;
                self.resume_pending_actions(gs)?;
            }
            Continuation::DeferredOther => {
                self.resume_pending_actions(gs)?;
            }
            Continuation::Immediate => {
                if !sub_choice {
                    self.resume_pending_actions(gs)?;
                }
            }
        }
        // A cost choice just resolved (e.g. the is_select_action=true stage
        // selection of a change_state sub-cost, applied by
        // handle_stage_selection). A sequential cost yields MID-cost
        // (cost_paid_index < len), so pay whatever sub-costs are still
        // outstanding BEFORE declaring the cost paid — otherwise the trailing
        // 「手札を1枚控え室に置く」 of 「メンバーをウェイトにし、手札を1枚
        // 控え室に置く」 would be skipped and the effect would run for free.
        if !sub_choice && self.awaiting.choice.is_none() {
            self.resume_sequential_cost(gs)?;
        }
        // No sub-cost left (or the cost is a single one already applied by
        // handle_stage_selection): mark the cost paid so the ability queue
        // can transition to the effect phase.
        if !sub_choice && self.awaiting.choice.is_none() {
            if let Some(e) = gs.ability_queue.current_entry_mut() {
                if !e.cost_paid && !e.effect_started {
                    e.mark_cost_paid();
                }
            }
        }
        log::trace!(
            "[CHOICE] source={:?} finalized: pending={} selected={:?} context={:?}",
            self.session.activating_card_id,
            self.awaiting.choice.is_some(),
            self.selection.cards,
            context
        );
        Ok(())
    }

    pub(in crate::ability::choice) fn reveal_selected_looked_at(&mut self, gs: &mut GameState, indices: &[usize]) {
        let mut revealed_ids = Vec::new();
        let source = gs.current_ability_source_card_id();
        let looked_owner = gs
            .ability_master_id()
            .and_then(|m| crate::ability::util::target_player_index("self", Some(m)));
        for &idx in indices.iter() {
            if idx < gs.looked_at_cards.len() {
                let cid = gs.looked_at_cards[idx];
                gs.push_revealed_card(cid, source, false, looked_owner, "ability");
                revealed_ids.push(cid);
            }
        }
        if !revealed_ids.is_empty() {
            let card_db = &gs.card_database;
            let names: Vec<String> = revealed_ids
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.to_string())
                .collect();
            if !names.is_empty() {
                let turn = gs.turn_number;
                gs.push_rule_log_fmt(format_args!(
                    "[Turn {}] P{} [[log_reveal_looked:n={}]]",
                    turn,
                    if core::ptr::eq(gs.active_player(), &gs.player1) {
                        1
                    } else {
                        2
                    },
                    names.len()
                ));
            }
        }
    }

    pub fn provide_choice_result(
        &mut self,
        gs: &mut GameState,
        result: ChoiceResult,
    ) -> Result<(), String> {
        let choice = self.awaiting.choice.clone();
        let context = self.in_flight.execution_context.clone();
        let Some(choice) = choice else {
            return Err("No pending choice".to_string());
        };
        crate::ability::choice::result_handlers::dispatch_choice_result(
            self,
            gs,
            &choice,
            &result,
            context,
        )
    }

    /// Consume the deferred そうした場合 gate (parent-conditional sequential
    /// whose gating move deferred to THIS selection): an empty/skipped answer
    /// drops the remaining actions; a real answer lets them run.
    pub(in crate::ability::choice) fn consume_deferred_gate(
        &mut self,
        gs: &mut GameState,
        indices: &[usize],
    ) {
        if self.in_flight.deferred_conditional_gate {
            self.in_flight.deferred_conditional_gate = false;
            if indices.is_empty() {
                log::debug!("[CONDITION] source={:?} deferred gate failed: empty selection, dropping remaining actions", self.session.activating_card_id);
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.pending_actions.clear();
                }
            }
        }
    }

    /// "Place a card under a member" resume: the stage-member SelectCard choice
    /// from `place_card_with_stage_choice` (destination=under_member) has been
    /// answered. Put the pending card (carried in the MoveCardsPosition
    /// execution context) under the chosen stage member. Returns true when the
    /// choice was an under_member placement (caller must resume pending actions).
    pub(in crate::ability::choice) fn resume_under_member_placement(
        &mut self,
        gs: &mut GameState,
        zone: &str,
        ctx: &SelectionContext,
        context: &ExecutionContext,
    ) -> bool {
        if Zone::from_str(zone) != Some(Zone::Stage)
            || ctx.destination.as_deref() != Some(Zone::UnderMember.to_str())
        {
            return false;
        }
        if let ExecutionContext::MoveCardsPosition {
            card_id,
            state_change,
            target,
            source_zone,
        } = context
        {
            let (card_id, target, source_zone, state_change) =
                (*card_id, target.clone(), source_zone.clone(), state_change.clone());
            self.awaiting.choice = None;
            let player = gs.resolve_target_player_mut(&target);
            let chosen_idx = ctx
                .mfi(&ctx.indices)
                .first()
                .copied()
                .filter(|&i| i < 3)
                .unwrap_or(1);
            if chosen_idx < 3 && player.stage.stage[chosen_idx] != -1 {
                let area = util::pos_to_area(chosen_idx);
                player.stage.place_under_card(area, card_id);
                gs.mods.clear_all_for_card(card_id);
                gs.record_card_movement(card_id);
                if !self.selection.moved_cards.contains(&card_id) {
                    self.selection.moved_cards.push(card_id);
                }
                if state_change.as_deref() == Some("wait") {
                    gs.set_orientation(card_id, "wait");
                }
                let pid = gs
                    .ability_queue
                    .current_entry()
                    .map(|e| e.player_id.clone())
                    .unwrap_or_default();
                gs.push_movement_event(
                    card_id,
                    &source_zone,
                    "under_member",
                    gs.activating_card,
                    &pid,
                    true,
                );
                gs.recalculate_constants();
            }
            return true;
        }
        false
    }

    /// Narrow the SelectedCards pool in place to the chosen indices.
    pub(in crate::ability::choice) fn narrow_selected_pool(&mut self, indices: &[usize]) {
        log::trace!(
            "[CHOICE] selection pool={:?} indices={:?}",
            self.selection.cards,
            indices
        );
        let mut cards = SmallVec::new();
        for &i in indices.iter() {
            if i < self.selection.cards.len() {
                cards.push(self.selection.cards[i]);
            }
        }
        self.selection.cards = cards;
        log::debug!(
            "[CHOICE] source={:?} selected pool narrowed: indices={:?} cards={:?}",
            self.session.activating_card_id,
            indices,
            self.selection.cards
        );
    }

    pub fn clear_choice_meta(&mut self, gs: &mut GameState) {
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = None;
            entry.conditional_choice = None;
        }
        // On skip/clear, drop deferred costs so they aren't paid.
        self.awaiting.deferred_costs.clear();
    }

    pub(in crate::ability::choice) fn clear_choice_state(&mut self, gs: &mut GameState) {
        if self.in_flight.sub_choice_created {
            self.in_flight.sub_choice_created = false;
        } else {
            self.awaiting.choice = None;
        }
        self.clear_choice_meta(gs);
    }

    pub(in crate::ability::choice) fn clear_choice_state_and_resume(&mut self, gs: &mut GameState) -> Result<(), String> {
        self.clear_choice_state(gs);
        self.resume_pending_actions(gs)
    }

    /// Recursively set target on all sub-effects that don't have an explicit target.
    /// Excludes draw/draw_card actions (always target self) and select_cards (handled
    /// via spawn_context.target fallback in handle_select_card_internal).
    pub(in crate::ability::choice) fn set_chosen_target(effect: &mut AbilityEffect, target: &str) {
        if matches!(
            effect.action,
            ActionType::DrawCard | ActionType::SelectCards
        ) {
            return;
        }
        if effect.target.is_none() || effect.target.as_deref() == Some("self") {
            effect.target = Some(target.into());
        }
        // Also propagate action_by: if the inner effect has action_by set,
        // preserve it; otherwise inherit from parent if parent has it and inner doesn't
        if effect.action_by_any().is_none() {
            // Don't override - keep any explicitly set action_by
        }
        // Walk `effect_steps` BEFORE taking the compound branch borrow:
        // `compound_branch_mut()` holds a mutable borrow on `effect` for the
        // rest of this scope, so the two cannot overlap.
        if let Some(ref mut steps) = effect.effect_steps {
            for s in steps.iter_mut() {
                Self::set_chosen_target(s, target);
            }
        }
        let c = effect.compound_branch_mut();
        if let Some(ref mut la) = c.look_action {
            Self::set_chosen_target(la, target);
        }
        if let Some(ref mut sa) = c.select_action {
            Self::set_chosen_target(sa, target);
        }
        if let Some(ref mut actions) = c.actions {
            for a in actions.iter_mut() {
                Self::set_chosen_target(a, target);
            }
        }
        if let Some(ref mut oa) = c.optional_action.as_mut() {
            Self::set_chosen_target(oa, target);
        }
        if let Some(ref mut pri) = c.primary_effect {
            Self::set_chosen_target(pri, target);
        }
        if let Some(ref mut ca) = c.conditional_action {
            Self::set_chosen_target(ca, target);
        }
        if let Some(ref mut fu) = c.followup_action {
            Self::set_chosen_target(fu, target);
        }
    }
}
