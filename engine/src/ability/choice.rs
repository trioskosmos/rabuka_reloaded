use super::condition::ConditionContext;
use super::enums::{ActionType, Zone};
use super::types::{
    Choice, ChoiceBuilder, ChoiceResult, ChoiceRoute, ExecutionContext, LookAndSelectStep,
    TriggerEvent,
};
use super::util;
use crate::ability::debug::ABILITY_DEBUG;
use crate::ability_queue::ConditionalChoice;
use crate::card::AbilityEffect;
use crate::game_state::GameState;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::Ordering;
use smallvec::SmallVec;

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
            && self.pending_choice.is_none()
        {
            self.execution_context = ExecutionContext::None;
        }
        Ok(())
    }

    /// Resumes executing sequential commands parked on the current queue entry.
    /// If another choice interrupts execution, the remaining actions are safely parked back.
    pub fn resume_pending_actions(&mut self, gs: &mut GameState) -> Result<(), String> {
        let pending = gs.ability_queue.take_pending_actions();
        for (idx, effect) in pending.iter().enumerate() {
            self.spawn_context.target = effect.target.clone().map(|s| s.to_string());
            self.execute_effect(gs, effect)?;
            if self.pending_choice.is_some() {
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.effect_started = true;
                }
                // Merge remaining actions from the original batch so they
                // aren't lost when a sub-action creates its own pending_choice.
                if idx + 1 < pending.len() {
                    let mut existing = gs.ability_queue.take_pending_actions();
                    let mut remaining: Vec<AbilityEffect> = pending[idx + 1..].to_vec();
                    // If the current effect had a condition type and it passed
                    // (the action ran to completion and created a choice), strip
                    // the same condition type from remaining actions. This prevents
                    // re-evaluation against stale game state (e.g. revealed_cards
                    // mutated by select_cards filtering).
                    if let Some(ref cond) = effect.condition {
                        let disc = core::mem::discriminant(cond.as_ref());
                        for a in &mut remaining {
                            if let Some(ref a_cond) = a.condition {
                                if core::mem::discriminant(a_cond.as_ref()) == disc {
                                    a.condition = None;
                                }
                            }
                        }
                    }
                    existing.extend(remaining);
                    gs.ability_queue.set_pending_actions(existing);
                }
                return Ok(());
            }
            if self.cancel_remaining_commands {
                self.cancel_remaining_commands = false;
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
            self.pending_repeat_actions.clear();
            // Mark effect as started so RWC goes to cost_was_paid
            // (not effect_ready which restarts from scratch).
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.effect_started = true;
            }
        }
        // Feed the next repeat action + "Repeat?" prompt, one at a time.
        log::debug!(
            "[RPA_REPEAT] pending_repeat={} pending_choice={:?}",
            self.pending_repeat_actions.len(),
            self.pending_choice.is_some()
        );
        if !self.pending_repeat_actions.is_empty() && self.pending_choice.is_none() {
            let next = self.pending_repeat_actions.remove(0);
            gs.ability_queue.set_pending_actions(vec![*next]);
            self.pending_choice = Some(Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: "Repeat effect?".to_string(),
                description_en: Some("Repeat effect?".to_string()),
                description_ja: Some("効果を繰り返しますか？".to_string()),
                allow_skip: true,
                options: Some(vec!["Stop".to_string(), "Continue".to_string()]),
            });
        }
        // Set re-prompt choice if one is pending and no other choice was created.
        if let Some(reprompt) = self.pending_reprompt_choice.take() {
            if self.pending_choice.is_none() {
                self.pending_choice = Some(reprompt);
            }
        }
        Ok(())
    }

    /// Shared epilogue: clear pending_choice, resume execution, process pending sequential actions.
    fn finalize_choice(
        &mut self,
        gs: &mut GameState,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        let is_actual_looked_at_choice = self
            .pending_choice
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
        self.pay_deferred_costs(gs)?;

        let sub_choice = self.sub_choice_created;
        self.sub_choice_created = false;
        if !should_preserve && !sub_choice {
            self.pending_choice = None;
        }
        self.resume_execution(gs, context.clone())?;
        let has_pending = gs.ability_queue.has_pending_actions();
        let was_select_card = matches!(self.pending_choice, Some(Choice::SelectCard { .. }));
        let cont = match (sub_choice, has_pending, was_select_card) {
            (true, _, _) => Continuation::Immediate, // sub-choice will handle resume
            (false, true, true) => Continuation::DeferredSelectCard,
            (false, true, false) => Continuation::DeferredOther,
            (false, false, _) => Continuation::Immediate,
        };
        #[cfg(not(feature = "no_std"))]
        eprintln!(
            "[FIN_DBG] looked={:?} sub_choice={} has_pending={} was_select_card={} cont={:?}",
            is_actual_looked_at_choice, sub_choice, has_pending, was_select_card, cont
        );
        match cont {
            Continuation::DeferredSelectCard => {
                self.pending_choice = None;
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
        // If a deferred cost (is_select_action=true stage selection) just had
        // its state change applied during handle_stage_selection, mark the cost
        // as paid so the ability queue can transition to the effect phase.
        if !sub_choice && self.pending_choice.is_none() {
            if let Some(e) = gs.ability_queue.current_entry_mut() {
                if !e.cost_paid && !e.effect_started {
                    e.cost_paid = true;
                }
            }
        }
        log::debug!(
            "[FINALIZE_CHOICE] pending={} selected={:?} context={:?}",
            has_pending_sequential,
            self.selected_cards,
            context
        );
        Ok(())
    }

    fn reveal_selected_looked_at(&mut self, gs: &mut GameState, indices: &[usize]) {
        let mut revealed_ids = Vec::new();
        let source = gs.current_ability_source_card_id();
        let looked_owner = gs
            .ability_master_id()
            .as_deref()
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
                gs.push_rule_log(format!(
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
        let choice = self.pending_choice.clone();
        let context = self.execution_context.clone();
        match (&choice, result) {
            (
                Some(Choice::SelectCard {
                    zone,
                    card_type,
                    count,
                    description: _,
                    allow_skip,
                    cost_limit,
                    cost_limit_operator,
                    cost_total,
                    cost_total_operator,
                    group,
                    characters,
                    filtered_indices,
                    is_select_action,
                    ref target_player_id,
                    blind,
                    destination,
                    discard_remaining,
                    ..
                }),
                ChoiceResult::CardSelected { indices },
            ) => self.handle_select_card(
                gs,
                choice.as_ref().unwrap(),
                zone,
                context,
                SelectionContext {
                    card_type: card_type.clone(),
                    count: *count,
                    allow_skip: *allow_skip,
                    indices: indices.to_vec(),
                    cost_limit: *cost_limit,
                    cost_limit_operator: cost_limit_operator.clone(),
                    cost_total: *cost_total,
                    cost_total_operator: cost_total_operator.clone(),
                    group: group.clone(),
                    characters: characters.clone(),
                    filtered_indices: filtered_indices.clone(),
                    is_select_action: *is_select_action,
                    target_player_id: target_player_id.clone(),
                    destination: destination.clone(),
                    discard_remaining: *discard_remaining,
                    blind: *blind,
                    is_reveal: choice.as_ref().is_some_and(|c| {
                        matches!(
                            c,
                            Choice::SelectCard {
                                is_reveal: true,
                                ..
                            }
                        )
                    }),
                },
            ),
            (
                Some(Choice::SelectCard {
                    count: 0,
                    allow_skip: true,
                    ..
                }),
                ChoiceResult::Skip,
            ) => {
                // any_number re-prompt skip: cards were already moved in a
                // previous sub-selection. Resume pending actions so
                // downstream actions (e.g. gain_resource) still execute.
                self.clear_choice_state_and_resume(gs)
            }
            (Some(Choice::SelectCard { .. }), ChoiceResult::Skip) => {
                // Clear pending commands saved by sequential conditional handlers
                // so that skipped optional sub-actions don't re-execute as mandatory.
                gs.ability_queue.take_pending_actions();
                self.clear_choice_state(gs);
                self.resume_execution(gs, context)
            }
            (Some(Choice::SelectTarget { .. }), ChoiceResult::Skip) => {
                // Skip the choice entirely — no option is executed.
                gs.ability_queue.take_pending_actions();
                self.clear_choice_state(gs);
                self.resume_execution(gs, context)
            }
            (
                Some(Choice::SelectTarget { target, .. }),
                ChoiceResult::TargetSelected { target: selected },
            ) if target == "area_select" => {
                // area_select: selected is either a numeric index (for backward compat
                // with direct select_option calls) or the actual area name (when the
                // choice was rendered as position|destination buttons).
                if let Some(Choice::SelectTarget {
                    options: Some(ref opts),
                    ..
                }) = choice
                {
                    if let Ok(idx) = selected.parse::<usize>() {
                        let opt = opts.get(idx).map(|s| s.as_str()).unwrap_or("left");
                        self.selected_area = Some(opt.to_string());
                    } else if opts.contains(&selected) {
                        self.selected_area = Some(selected.clone());
                    } else {
                        self.selected_area = Some(opts[0].clone());
                    }
                    self.clear_choice_state(gs);
                    return self.resume_pending_actions(gs);
                }
                self.selected_area = Some(selected.clone());
                self.clear_choice_state(gs);
                self.resume_pending_actions(gs)
            }
            (
                Some(Choice::SelectTarget { target, .. }),
                ChoiceResult::TargetSelected { target: selected },
            ) => self.handle_select_target(gs, target, &selected),
            (Some(Choice::SelectPosition { .. }), ChoiceResult::PositionSelected { position }) => {
                self.handle_select_position(gs, &position, context)
            }
            (
                Some(Choice::SelectHeartColor { count, .. }),
                ChoiceResult::HeartColorSelected { colors },
            )
            | (
                Some(Choice::SelectHeartType { count, .. }),
                ChoiceResult::HeartTypeSelected { types: colors },
            ) => self.handle_heart_selection(gs, *count as u8, &colors),
            _ => Err("Choice result does not match pending choice".to_string()),
        }
    }

    fn handle_select_card(
        &mut self,
        gs: &mut GameState,
        choice: &Choice,
        zone: &str,
        context: ExecutionContext,
        ctx: SelectionContext,
    ) -> Result<(), String> {
        if ABILITY_DEBUG.load(Ordering::Relaxed) {
            log::debug!(
                "[SEL_CARD] zone='{}' indices={:?} count={} allow_skip={} context={:?} is_reveal={}",
                zone, ctx.indices, ctx.count, ctx.allow_skip, context, ctx.is_reveal
            );
        }

        // Distinguish cost vs effect: cost handler only fires when effect NOT yet started.
        // effect_started is false during cost payment, true during effect execution.
        let effect_started = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.effect_started);

        // Handle reveal action: push selected cards to revealed_cards, don't discard.
        if ctx.is_reveal && Zone::from_str(zone) == Some(Zone::Hand) {
            return self.handle_reveal_selection(gs, &ctx);
        }

        if !effect_started
            && gs
                .entry_cost()
                .is_some_and(|c| c.action == ActionType::Reveal)
        {
            return self.handle_entry_cost_reveal(gs, &ctx, &context);
        }

        let card_type: &Option<String> = &ctx.card_type;
        let count: usize = ctx.count;
        let allow_skip: bool = ctx.allow_skip;
        let indices: &[usize] = &ctx.indices;
        let cost_limit: Option<u8> = ctx.cost_limit;
        let cost_limit_operator: &Option<String> = &ctx.cost_limit_operator;
        let cost_total: Option<u8> = ctx.cost_total;
        let cost_total_operator: &Option<String> = &ctx.cost_total_operator;
        let group: &Option<String> = &ctx.group;
        let characters: &Option<Vec<String>> = &ctx.characters;
        let filtered_indices: &Option<Vec<usize>> = &ctx.filtered_indices;
        let is_select_action: bool = ctx.is_select_action;
        let target_player_id: &Option<String> = &ctx.target_player_id;

        let card_db = gs.card_database.clone();
        let validate_filter = choice.as_filter();
        let mut validate_card =
            |cid: i16| -> bool { validate_filter.matches(&card_db, cid, false) };

        let mfi = |indices: &[usize]| -> Vec<usize> {
            match &filtered_indices {
                Some(fi) => indices.iter().filter_map(|&i| fi.get(i).copied()).collect(),
                None => indices.to_vec(),
            }
        };

        let common_re = |zone: &str,
                         count: usize,
                         desc: String,
                         desc_ja: String,
                         skip: bool,
                         fi: Option<Vec<usize>>,
                         tpid: Option<String>,
                         ct: Option<u8>,
                         cto: Option<String>|
         -> ChoiceBuilder {
            Choice::select_cards(zone, count, desc, skip)
                .description_ja(Some(desc_ja))
                .card_type(card_type.clone())
                .cost_limit(cost_limit, cost_limit_operator.clone())
                .cost_total(ct, cto)
                .group(group.clone())
                .characters(characters.clone())
                .filtered_indices(fi)
                .target_player_id(tpid)
        };

        log::debug!(
            "[KANAN_DEBUG] check hand-cost block: zone={} is_hand={} has_cost={} effect_started={}",
            zone,
            Zone::from_str(zone) == Some(Zone::Hand),
            gs.entry_cost().is_some(),
            effect_started
        );
        if Zone::from_str(zone) == Some(Zone::Hand) && gs.entry_cost().is_some() && !effect_started
        {
            if !indices.is_empty() {
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.optional_cost_result = Some(true);
                }
            }
            let target = target_player_id
                .clone()
                .unwrap_or_else(|| "self".to_string().into());
            let hand_cards: Vec<i16> = {
                let p = gs.resolve_target_player_mut(&target);
                p.hand.cards.to_vec()
            };
            let cost_hand_indices = mfi(indices);
            let new_card_ids: Vec<i16> = cost_hand_indices
                .iter()
                .filter_map(|&i| {
                    if i < hand_cards.len() {
                        Some(hand_cards[i])
                    } else {
                        None
                    }
                })
                .filter(|&cid| validate_card(cid))
                .collect();
            if !new_card_ids.is_empty() {
                log::debug!(
                    "[KANAN_DEBUG] moving cards: count={} new_card_ids={:?}",
                    count,
                    new_card_ids
                );
                let player = gs.resolve_target_player_mut(&target);
                let _ = util::move_cards(
                    player,
                    &new_card_ids,
                    Zone::Hand.to_str(),
                    Zone::Discard.to_str(),
                    None,
                    &card_db,
                );
                gs.mods.last_cost_discard_count += new_card_ids.len() as u8;
                gs.mods
                    .last_cost_moved_card_ids
                    .extend(new_card_ids.iter().copied());
                for &cid in &new_card_ids {
                    self.moved_cards.push(cid);
                    if !self.selected_cards.contains(&cid) {
                        self.selected_cards.push(cid);
                    }
                }
                log::debug!(
                    "[DBG_HSC] moved {} cards, moved_cards.len={}",
                    new_card_ids.len(),
                    self.moved_cards.len()
                );
            }
            if new_card_ids.is_empty() {
                if !self.moved_cards.is_empty() {
                    log::debug!("[KANAN_DEBUG] cost finalize: moved_cards={:?}, setting optional_cost_result=true", self.moved_cards);
                    gs.mods.last_cost_discard_count = self.moved_cards.len() as u8;
                    gs.mods.last_cost_moved_card_ids = self.moved_cards.clone();
                    gs.recently_moved_cards = Some(self.moved_cards.clone());
                    gs.recently_moved_from_zone = Some("hand".to_string());
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.cost_paid = true;
                        entry.optional_cost_result = Some(true);
                    }
                } else if allow_skip {
                    log::debug!("[KANAN_DEBUG] cost finalize: NO moved cards, setting optional_cost_result=false");
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.cost_paid = true;
                        entry.optional_cost_result = Some(false);
                    }
                }
                self.pending_choice = None;
                return Ok(());
            }
            // Rule 9.4.2.3: cost must be paid in full. Once a player selects the
            // first card for a fixed-count cost, they are committed — no bail-out.
            if count > 0 && new_card_ids.len() < count {
                let remaining = count - new_card_ids.len();
                let hand_now: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.to_vec()
                };
                let same_unit_name = gs
                    .entry_cost()
                    .and_then(|c| c.same_unit_name_any())
                    .unwrap_or(false);
                let same_unit_filter = if same_unit_name {
                    self.moved_cards
                        .first()
                        .and_then(|&cid| card_db.get_card(cid))
                        .and_then(|c| c.unit.clone())
                } else {
                    None
                };
                let available_idxs: Vec<usize> = (0..hand_now.len())
                    .filter(|i| {
                        let cid = hand_now[*i];
                        validate_card(cid)
                            && same_unit_filter.as_ref().map_or(true, |unit| {
                                card_db.get_card(cid).and_then(|c| c.unit.as_ref()) == Some(unit)
                            })
                    })
                    .collect();
                let fi = if available_idxs.is_empty() {
                    None
                } else {
                    Some(available_idxs)
                };
                self.pending_choice = Some(
                    common_re(
                        Zone::Hand.to_str(),
                        remaining,
                        format!("Select {} more card(s) from hand for cost", remaining),
                        format!("コストとして手札からさらに{}枚選択", remaining),
                        false, // Rule 9.4.2.3: no bail-out once committed
                        fi,
                        Some(target.clone()),
                        cost_total,
                        cost_total_operator.clone(),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            // Same-unit cost: after selecting the first card, re-prompt
            // for remaining cards filtered to only the chosen unit.
            if count > 0 && !new_card_ids.is_empty() && new_card_ids.len() == count {
                if let Some(cost) = gs.entry_cost() {
                    if cost.same_unit_name_any().unwrap_or(false) {
                        let total_needed = cost.count.unwrap_or(1) as usize;
                        let total_moved = self.moved_cards.len();
                        if total_moved < total_needed {
                            let remaining = total_needed - total_moved;
                            let hand_now: Vec<i16> = {
                                let p = gs.resolve_target_player_mut(&target);
                                p.hand.cards.to_vec()
                            };
                            let unit_name = self
                                .moved_cards
                                .first()
                                .and_then(|&cid| card_db.get_card(cid))
                                .and_then(|c| c.unit.clone());
                            let same_unit_idxs: Vec<usize> = (0..hand_now.len())
                                .filter(|i| {
                                    let cid = hand_now[*i];
                                    validate_card(cid)
                                        && unit_name.as_ref().map_or(false, |un| {
                                            card_db.get_card(cid).and_then(|c| c.unit.as_ref())
                                                == Some(un)
                                        })
                                })
                                .collect();
                            if same_unit_idxs.is_empty() {
                                return Err(format!(
                                    "Cannot pay cost: not enough cards with unit '{}' in hand",
                                    unit_name.unwrap_or_default()
                                ));
                            }
                            self.pending_choice = Some(
                                common_re(
                                    Zone::Hand.to_str(),
                                    remaining,
                                    format!(
                                        "Select {} more card(s) with the same unit name",
                                        remaining
                                    ),
                                    format!("同じユニット名のカードをさらに{}枚選択", remaining),
                                    false,
                                    Some(same_unit_idxs),
                                    Some(target.clone()),
                                    cost_total,
                                    cost_total_operator.clone(),
                                )
                                .build(),
                            );
                            self.store_pending_choice(gs);
                            return Ok(());
                        }
                    }
                }
            }
            // Re-prompt for any_number costs: after each selection, ask
            // if the player wants to select more cards or skip to finish.
            let cost_max_cap = gs
                .entry_cost()
                .and_then(|c| c.count)
                .map(|c| c as usize)
                .unwrap_or(usize::MAX);
            if count == 0 && allow_skip && self.moved_cards.len() < cost_max_cap {
                log::debug!("[KANAN_DEBUG] any_number re-prompt: count={} allow_skip={} new_card_ids.len={} moved_cards={:?}", count, allow_skip, new_card_ids.len(), self.moved_cards);
                let hand_now: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.to_vec()
                };
                let available_idxs: Vec<usize> = (0..hand_now.len())
                    .filter(|i| {
                        let cid = hand_now[*i];
                        validate_card(cid)
                    })
                    .collect();
                let fi = if available_idxs.is_empty() {
                    None
                } else {
                    Some(available_idxs)
                };
                let desc = if fi.as_ref().map_or(0, |v| v.len()) > 0 {
                    "Select more card(s) from hand for cost (or skip to finish)".to_string()
                } else {
                    "No more matching cards in hand (skip to finish)".to_string()
                };
                let desc_ja = if fi.as_ref().map_or(0, |v| v.len()) > 0 {
                    "コストとして手札からさらに選択（スキップで終了）".to_string()
                } else {
                    "手札に一致するカードがありません（スキップで終了）".to_string()
                };
                self.pending_choice = Some(
                    common_re(
                        Zone::Hand.to_str(),
                        0,
                        desc,
                        desc_ja,
                        true,
                        fi,
                        Some(target.clone()),
                        cost_total,
                        cost_total_operator.clone(),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            log::debug!(
                "[DBG_HSC] finalizing with {} moved cards",
                self.moved_cards.len()
            );
            self.pay_deferred_costs(gs)?;
            let final_count = self.moved_cards.len() as u8;
            gs.mods.last_cost_discard_count = final_count;
            gs.mods.last_cost_moved_card_ids = self.moved_cards.clone();
            gs.recently_moved_cards = Some(self.moved_cards.clone());
            gs.recently_moved_from_zone = Some("hand".to_string());
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.cost_paid = true;
            }
            self.pending_choice = None;
            return Ok(());
        }

        if Zone::from_str(zone) == Some(Zone::Energy)
            && !effect_started
            && ctx.destination.as_deref() != Some("under_member")
        {
            let count_paid = indices.len();
            if count_paid > 0 {
                let player =
                    gs.resolve_target_player_mut(target_player_id.as_deref().unwrap_or("self"));
                player.energy_zone.pay_energy(count_paid as u8)?;
                gs.mods.last_cost_energy_count += count_paid as u8;
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.cost_paid = true;
                    entry.optional_cost_result = Some(true);
                }
                // Energy-count constants (「エネルギーがちょうどN枚あるかぎり」)
                // are live state — re-evaluate right after the payment.
                gs.mark_constants_dirty();
                gs.recalculate_constants();
            }
            let energy_left = {
                let player =
                    gs.resolve_target_player(target_player_id.as_deref().unwrap_or("self"));
                player.energy_zone.active_energy_count
            };
            if count_paid > 0 && energy_left > 0 {
                let efi: Vec<usize> = (0..energy_left as usize).collect();
                let target = target_player_id
                    .clone()
                    .unwrap_or_else(|| "self".to_string().into());
                self.pending_choice = Some(
                    common_re(
                        Zone::Energy.to_str(),
                        0,
                        format!(
                            "Select energy card to pay (active: {}). Skip when done",
                            energy_left
                        ),
                        format!(
                            "エネルギーカードを選択（アクティブ: {}）完了でスキップ",
                            energy_left
                        ),
                        true,
                        Some(efi),
                        Some(target),
                        cost_total,
                        cost_total_operator.clone(),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            self.clear_choice_state(gs);
            return self.resume_pending_actions(gs);
        }

        if crate::ability::debug::ABILITY_DEBUG.load(core::sync::atomic::Ordering::Relaxed) {
            log::debug!(
                "[OUTER_MATCH] zone={} effect_started={} pending_commands={}",
                zone,
                effect_started,
                gs.ability_queue.has_pending_actions()
            );
        }
        // "Place a card under a member" resume: the stage-member SelectCard choice
        // from `place_card_with_stage_choice` (destination=under_member) has been
        // answered. Put the pending card (carried in the MoveCardsPosition
        // execution context) under the chosen stage member and finish the move.
        if Zone::from_str(zone) == Some(Zone::Stage)
            && ctx.destination.as_deref() == Some(Zone::UnderMember.to_str())
        {
            if let ExecutionContext::MoveCardsPosition {
                card_id,
                state_change,
                target,
                source_zone,
            } = &context
            {
                let (card_id, target, source_zone, state_change) =
                    (*card_id, target.clone(), source_zone.clone(), state_change.clone());
                self.pending_choice = None;
                let player = gs.resolve_target_player_mut(&target);
                let chosen_idx = ctx
                    .mfi(&ctx.indices)
                    .first()
                    .copied()
                    .filter(|&i| i < 3)
                    .unwrap_or(1);
                if chosen_idx < 3 && player.stage.stage[chosen_idx] != -1 {
                    let area = match chosen_idx {
                        0 => crate::zones::MemberArea::LeftSide,
                        1 => crate::zones::MemberArea::Center,
                        _ => crate::zones::MemberArea::RightSide,
                    };
                    player.stage.place_under_card(area, card_id);
                    gs.mods.clear_all_for_card(card_id);
                    gs.record_card_movement(card_id);
                    if !self.moved_cards.contains(&card_id) {
                        self.moved_cards.push(card_id);
                    }
                    if state_change.as_deref() == Some("wait") {
                        gs.mods.add_orientation_modifier(card_id, "wait");
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
                    gs.mark_constants_dirty();
                    gs.recalculate_constants();
                }
                return self.resume_pending_actions(gs);
            }
        }
        match Zone::from_str(zone) {
            Some(Zone::Hand) => {
                return self.handle_hand_selection(gs, &ctx, &context, &mut validate_card);
            }
            Some(Zone::Deck) => self.execute_selected_cards_from_zone(
                gs,
                Zone::Deck.to_str(),
                indices,
                card_type.as_deref(),
                cost_limit,
                cost_limit_operator.as_deref(),
                cost_total,
                cost_total_operator.as_deref(),
                group.as_deref(),
                characters.as_ref(),
                target_player_id.as_deref(),
            )?,
            Some(Zone::Discard) => {
                return self.handle_discard_selection(gs, &ctx, &context, &mut validate_card);
            }
            Some(Zone::LookedAt) => {
                return self.handle_looked_at_selection(gs, &ctx, &context);
            }
            Some(Zone::RevealedCards) => {
                let edst = gs.entry_destination().map(|s| s.to_string());
                let dst_str = ctx
                    .destination
                    .clone()
                    .or(edst)
                    .unwrap_or_else(|| Zone::Discard.to_string().into());
                self.handle_revealed_cards_selection(gs, &ctx, &mut validate_card, &dst_str)?;
            }
            Some(Zone::Energy) => {
                let dst = if let Choice::SelectCard { destination, .. } = choice {
                    destination.clone()
                } else {
                    None
                };
                self.handle_energy_zone_selection(gs, indices, count, dst, &mut validate_card)?;
            }
            Some(Zone::SelectedCards) => {
                log::debug!(
                    "[SELECTED_CARDS_BEFORE] self.selected_cards={:?} indices={:?}",
                    self.selected_cards,
                    indices
                );
                let mut cards = SmallVec::new();
                for &i in indices.iter() {
                    if i < self.selected_cards.len() {
                        cards.push(self.selected_cards[i]);
                    }
                }
                self.selected_cards = cards;
                log::debug!(
                    "[SELECTED_CARDS_AFTER] self.selected_cards={:?}",
                    self.selected_cards
                );
            }
            Some(Zone::LiveCardZone) => {
                if is_select_action {
                    let target = target_player_id
                        .clone()
                        .unwrap_or_else(|| "self".to_string().into());
                    let player = gs.resolve_target_player_mut(&target);
                    let mapped_indices = mfi(indices);
                    let mut cards: Vec<i16> = Vec::new();
                    for &i in mapped_indices.iter() {
                        if i < player.live_card_zone.cards.len() {
                            let cid = player.live_card_zone.cards[i];
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                                cards.push(cid);
                            }
                        }
                    }
                    player.live_card_zone.cards.retain(|c| !cards.contains(c));
                    log::debug!(
                        "[LIVE_CARD_SELECTION] selected_cards={:?} removed from live_card_zone",
                        self.selected_cards
                    );
                } else {
                    let edst = gs.entry_destination().map(|s| s.to_string());
                    let dst_str = ctx
                        .destination
                        .clone()
                        .or(edst)
                        .unwrap_or_else(|| Zone::Discard.to_string().into());
                    let tgt = target_player_id
                        .clone()
                        .unwrap_or_else(|| "self".to_string().into());
                    let player = gs.resolve_target_player_mut(&tgt);
                    let card_ids: Vec<i16> = mfi(indices)
                        .iter()
                        .filter_map(|&i| player.live_card_zone.cards.get(i).copied())
                        .filter(|&cid| validate_card(cid))
                        .collect();
                    let moved = util::move_cards(
                        player,
                        &card_ids,
                        Zone::LiveCardZone.to_str(),
                        &dst_str,
                        None,
                        &card_db,
                    );
                    if moved > 0 {
                        for &cid in &card_ids {
                            if !self.selected_cards.contains(&cid) {
                                self.selected_cards.push(cid);
                            }
                        }
                        gs.recently_moved_cards = Some(card_ids.into());
                        gs.recently_moved_from_zone = Some(Zone::LiveCardZone.to_string());
                    }
                }
            }
            Some(Zone::Stage) => {
                self.handle_stage_selection(gs, &ctx, &mut validate_card)?;
            }
            Some(Zone::UnderMember) => {
                let edst = gs.entry_destination().map(|s| s.to_string());
                let dst_str = ctx
                    .destination
                    .clone()
                    .or(edst)
                    .unwrap_or_else(|| Zone::EnergyDeck.to_str().to_string().into());
                let tgt = target_player_id
                    .clone()
                    .unwrap_or_else(|| "self".to_string().into());
                let moved = self.move_from_under_member(
                    gs,
                    &ctx.indices,
                    &mut validate_card,
                    &dst_str,
                    &tgt,
                )?;
                if !moved.is_empty() {
                    self.moved_cards.extend(moved.iter().copied());
                    // Accumulate across any_number re-prompts unconditionally
                    // (same card ID can appear multiple times under a member).
                    let combined = gs.recently_moved_cards.get_or_insert_with(SmallVec::new);
                    combined.extend(moved.iter().copied());
                }
                // any_number re-prompt: after each selection, show remaining cards
                // Empty indices = player chose to stop selecting (skip).
                if ctx.indices.is_empty() {
                    // Done selecting — fall through to handle_selection_epilogue
                } else if ctx.count == 0 && ctx.allow_skip {
                    let player = gs.resolve_target_player_mut(&tgt);
                    let mut remaining_idxs: Vec<usize> = Vec::new();
                    let mut global_idx = 0;
                    for si in 0..3 {
                        for &cid in &player.stage.under_cards[si] {
                            if validate_card(cid) {
                                remaining_idxs.push(global_idx);
                            }
                            global_idx += 1;
                        }
                    }
                    if !remaining_idxs.is_empty() {
                        self.pending_choice = Some(
                            Choice::select_cards(
                                Zone::UnderMember.to_str(),
                                0,
                                "Select more energy cards (or skip to finish)",
                                true,
                            )
                            .description_ja(Some(
                                "エネルギーカードをさらに選択（スキップで終了）".to_string(),
                            ))
                            .card_type(Some("energy_card".to_string()))
                            .target_player_id(Some(tgt.clone()))
                            .filtered_indices(Some(remaining_idxs))
                            .build(),
                        );
                        self.store_pending_choice(gs);
                        return Ok(());
                    }
                }
            }
            Some(Zone::SuccessLiveZone) => {
                return self.handle_success_live_zone_selection(gs, &ctx, &mut validate_card);
            }
            _ => {
                return Err(format!(
                    "Card selection from zone '{}' not yet implemented or unsupported",
                    zone
                ))
            }
        }
        log::debug!(
            "▶ Select: {} card(s) selected from zone={:?} → [{}]",
            self.selected_cards.len(),
            zone,
            self.fmt_ids(&self.selected_cards)
        );
        return self.handle_selection_epilogue(gs, &context);
    }

    fn build_reprompt(
        &self,
        ctx: &SelectionContext,
        zone: &str,
        count: usize,
        desc: String,
        desc_ja: String,
        skip: bool,
        fi: Option<Vec<usize>>,
        tpid: Option<String>,
        ct: Option<u8>,
        cto: Option<String>,
    ) -> ChoiceBuilder {
        Choice::select_cards(zone, count, desc, skip)
            .description_ja(Some(desc_ja))
            .card_type(ctx.card_type.clone())
            .cost_limit(ctx.cost_limit, ctx.cost_limit_operator.clone())
            .cost_total(ct, cto)
            .group(ctx.group.clone())
            .characters(ctx.characters.clone())
            .filtered_indices(fi)
            .target_player_id(tpid)
    }

    fn handle_hand_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        let mapped_indices = ctx.mfi(&ctx.indices);
        let hand_idx = if mapped_indices.is_empty() && !ctx.allow_skip && ctx.count > 0 {
            return Err("No cards selected from hand for required selection".to_string());
        } else {
            &mapped_indices
        };
        // C6 keep-N-shuffle-rest: the selected hand cards are KEPT in hand (not
        // moved); the handler later shuffles the non-selected under the deck.
        // Record the chosen card IDs into selected_cards and return — do NOT
        // execute the selection (move) or clear selected_cards.
        if self.keep_shuffle_under_phase > 0 {
            let target = ctx
                .target_player_id
                .as_deref()
                .unwrap_or("self")
                .to_string();
            let hand_cards: Vec<i16> = {
                let p = gs.resolve_target_player_mut(&target);
                p.hand.cards.to_vec()
            };
            // Record the chosen hand POSITIONS to keep (hand is unchanged during
            // selection, so these absolute positions map onto the snapshot).
            for &idx in hand_idx.iter() {
                let idx = idx as u8;
                if !self.keep_shuffle_selected.contains(&idx) {
                    self.keep_shuffle_selected.push(idx);
                }
            }
            let count = self.keep_shuffle_under_count as usize;
            let available_idxs: Vec<usize> = (0..hand_cards.len())
                .filter(|i| !self.keep_shuffle_selected.contains(&(*i as u8)))
                .collect();
            if hand_idx.len() < count && !hand_idx.is_empty() && !available_idxs.is_empty() {
                let remaining = count.saturating_sub(self.keep_shuffle_selected.len().min(count));
                let fi = Some(available_idxs);
                let desc = format!(
                    "Select up to {} more card(s) from hand to keep",
                    remaining
                );
                self.pending_choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Hand.to_str(),
                        remaining,
                        desc,
                        "手札からさらに選ぶ（スキップで終了）".to_string(),
                        true,
                        fi,
                        Some(target.clone()),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            if self.keep_shuffle_under_phase == 1 {
                // Self's selection completed (including empty "keep 0" which
                // correctly shuffles the entire hand under). Move self's
                // non-selected cards, then prompt opponent.
                let snapshot = self.keep_shuffle_under_snapshots[0].clone();
                self.move_non_selected_hand_to_deck_bottom(gs, "self", &snapshot);
                self.keep_shuffle_selected.clear();
                // Snapshot opponent hand before prompting
                let opp_hand = gs.resolve_target_player_mut("opponent").hand.cards.to_vec();
                self.keep_shuffle_under_snapshots.push(opp_hand);
                let opp_hand_len = self.keep_shuffle_under_snapshots[1].len();
                let pick = (count).min(opp_hand_len);
                let c = crate::ability::types::Choice::select_cards(
                    Zone::Hand.to_str(),
                    pick,
                    format!("Select up to {} card(s) to keep", count),
                    true,
                )
                .target_player_id(Some("opponent".to_string()))
                .build();
                self.keep_shuffle_under_phase = 2;
                self.spawn_context.target = Some("opponent".to_string());
                self.pending_choice = Some(c);
                self.store_pending_choice(gs);
                return Ok(());
            }
            // Opponent's selection is recorded (phase 2). The effect does not
            // re-enter at phase 2 (sub-choice resolution), so move opponent's
            // non-selected hand cards under opponent's deck right here.
            if self.keep_shuffle_under_phase == 2 {
                let snapshot = self.keep_shuffle_under_snapshots[1].clone();
                self.move_non_selected_hand_to_deck_bottom(gs, "opponent", &snapshot);
                self.keep_shuffle_under_phase = 0;
                self.keep_shuffle_under_snapshots.clear();
                self.keep_shuffle_selected.clear();
                self.spawn_context.target = None;
                // The sequential's second action is "draw 3 for both". The
                // pending queue may be corrupted (observed n=2 [select,draw]),
                // so perform the draw directly here and clear any pending draw
                // to avoid double-draw.
                let card_db = gs.card_database.clone();
                for player in [&mut gs.player1, &mut gs.player2] {
                    let _ = crate::ability::effects::draw::draw_cards_for_player(
                        player,
                        3,
                        "deck",
                        "hand",
                        None,
                        false,
                        None,
                        &card_db,
                        None,
                    );
                }
                // Clear any pending sequential actions (the draw is now
                // performed directly). This prevents the corrupted
                // [select,draw] pending from restarting a second keep_shuffle.
                gs.ability_queue.take_pending_actions();
            }
            return self.handle_selection_epilogue(gs, context);
        }
        if !hand_idx.is_empty() || ctx.allow_skip {
            if !hand_idx.is_empty() && ctx.count > 0 && hand_idx.len() < ctx.count {
                let target = ctx
                    .target_player_id
                    .as_deref()
                    .unwrap_or("self")
                    .to_string();
                let hand_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.to_vec()
                };
                let mut all_hand_idxs = hand_idx.to_vec();
                if !self.selected_cards.is_empty() {
                    for (hidx, cid) in hand_cards.iter().enumerate() {
                        if self.selected_cards.contains(cid) && !all_hand_idxs.contains(&hidx) {
                            all_hand_idxs.push(hidx);
                        }
                    }
                }
                let new_card_ids: Vec<i16> = all_hand_idxs
                    .iter()
                    .filter_map(|&idx| {
                        if idx < hand_cards.len() {
                            Some(hand_cards[idx])
                        } else {
                            None
                        }
                    })
                    .collect();
                for cid in new_card_ids {
                    if !self.selected_cards.contains(&cid) {
                        self.selected_cards.push(cid);
                    }
                }
                let remaining = ctx.count - hand_idx.len();
                let available_idxs: Vec<usize> = (0..hand_cards.len())
                    .filter(|i| !all_hand_idxs.contains(i))
                    .collect();
                let fi = if available_idxs.is_empty() {
                    None
                } else {
                    Some(available_idxs)
                };
                let desc = format!(
                    "Select {} more card(s) from hand{}",
                    remaining,
                    if ctx.blind { " (blind)" } else { "" }
                );
                let desc_ja = format!(
                    "手札からさらに{}枚選択{}",
                    remaining,
                    if ctx.blind {
                        "（控えに選択）"
                    } else {
                        ""
                    }
                );
                self.pending_choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Hand.to_str(),
                        remaining,
                        desc,
                        desc_ja,
                        false,
                        fi,
                        Some(target.clone()),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .blind(ctx.blind)
                    .is_reveal(ctx.is_reveal)
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            if !hand_idx.is_empty() && ctx.count == 0 && ctx.allow_skip {
                let target = ctx
                    .target_player_id
                    .as_deref()
                    .unwrap_or("self")
                    .to_string();
                let old_hand_cards: Vec<i16> = {
                    let p = gs.resolve_target_player(&target);
                    p.hand.cards.to_vec()
                };
                let moved_ids: Vec<i16> = hand_idx
                    .iter()
                    .filter_map(|&idx| old_hand_cards.get(idx).copied())
                    .collect();
                self.execute_selected_cards_from_zone(
                    gs,
                    Zone::Hand.to_str(),
                    hand_idx,
                    ctx.card_type.as_deref(),
                    ctx.cost_limit,
                    ctx.cost_limit_operator.as_deref(),
                    ctx.cost_total,
                    ctx.cost_total_operator.as_deref(),
                    ctx.group.as_deref(),
                    ctx.characters.as_ref(),
                    ctx.target_player_id.as_deref(),
                )?;
                for &cid in &moved_ids {
                    if !self.selected_cards.contains(&cid) {
                        self.selected_cards.push(cid);
                    }
                }
                let hand_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.to_vec()
                };
                let include_idxs: Vec<usize> = (0..hand_cards.len())
                    .filter(|i| validate_card(hand_cards[*i]))
                    .collect();
                let fi = if include_idxs.is_empty() {
                    None
                } else {
                    Some(include_idxs)
                };
                self.pending_choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Hand.to_str(),
                        0,
                        "Select more card(s) from hand (or skip to finish)".to_string(),
                        "手札からさらに選択（スキップで終了）".to_string(),
                        true,
                        fi,
                        Some(target),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            let mut all_idxs = hand_idx.to_vec();
            if !self.selected_cards.is_empty() {
                let target = ctx
                    .target_player_id
                    .as_deref()
                    .unwrap_or("self")
                    .to_string();
                let hand_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.hand.cards.to_vec()
                };
                for (hidx, cid) in hand_cards.iter().enumerate() {
                    if self.selected_cards.contains(cid) && !all_idxs.contains(&hidx) {
                        all_idxs.push(hidx);
                    }
                }
            }
            self.execute_selected_cards_from_zone(
                gs,
                Zone::Hand.to_str(),
                &all_idxs,
                ctx.card_type.as_deref(),
                ctx.cost_limit,
                ctx.cost_limit_operator.as_deref(),
                ctx.cost_total,
                ctx.cost_total_operator.as_deref(),
                ctx.group.as_deref(),
                ctx.characters.as_ref(),
                ctx.target_player_id.as_deref(),
            )?;
            self.selected_cards.clear();
        }
        if ctx.allow_skip {
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.optional_cost_result =
                    Some(!ctx.indices.is_empty() || !self.moved_cards.is_empty());
            }
            let is_opponent_action = ctx.target_player_id.as_deref() == Some("opponent");
            if ctx.indices.is_empty()
                && ctx.count > 0
                && !is_opponent_action
                && self.moved_cards.is_empty()
            {
                gs.ability_queue.take_pending_actions();
            }
        }
        self.handle_selection_epilogue(gs, context)
    }

    fn handle_reveal_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
    ) -> Result<(), String> {
        let effect_started = gs
            .ability_queue
            .current_entry()
            .is_some_and(|e| e.effect_started);
        let target = ctx
            .target_player_id
            .clone()
            .unwrap_or_else(|| "self".to_string().into());

        let hand_positions: Vec<usize> = if let Some(ref fi) = ctx.filtered_indices {
            if ctx.count == 0 {
                ctx.indices
                    .iter()
                    .filter_map(|&i| fi.get(i).copied())
                    .collect()
            } else {
                ctx.indices.to_vec()
            }
        } else {
            ctx.indices.to_vec()
        };

        if !hand_positions.is_empty() && ctx.count > 0 && hand_positions.len() < ctx.count {
            for &hp in &hand_positions {
                if !self.selected_cards.contains(&(hp as i16)) {
                    self.selected_cards.push(hp as i16);
                }
            }
            let remaining = ctx.count - hand_positions.len();
            self.pending_choice = Some(
                Choice::select_cards(
                    Zone::Hand.to_str(),
                    remaining,
                    format!(
                        "Select {} more card(s) from hand{}",
                        remaining,
                        if ctx.blind { " (blind)" } else { "" }
                    ),
                    false,
                )
                .description_ja(Some(format!(
                    "手札からさらに{}枚選択{}",
                    remaining,
                    if ctx.blind {
                        "（控えに選択）"
                    } else {
                        ""
                    }
                )))
                .target_player_id(Some(target.clone()))
                .blind(ctx.blind)
                .is_reveal(true)
                .filtered_indices(Some(
                    self.selected_cards.iter().map(|&i| i as usize).collect(),
                ))
                .build(),
            );
            self.store_pending_choice(gs);
            return Ok(());
        }

        let mut all_indices: Vec<usize> = self.selected_cards.iter().map(|&i| i as usize).collect();
        self.selected_cards.clear();
        for &hp in &hand_positions {
            if !all_indices.contains(&hp) {
                all_indices.push(hp);
            }
        }

        let ids_to_reveal = if ctx.count == 0 {
            &hand_positions
        } else {
            &all_indices
        };
        let revealed_card_ids = {
            let player = gs.resolve_target_player_mut(&target);
            util::resolve_indices_to_ids(player, Zone::Hand.to_str(), ids_to_reveal)
        };

        let source = gs.current_ability_source_card_id();
        let owner = util::target_player_index(&target, gs.ability_master_id().as_deref());
        for &cid in &revealed_card_ids {
            gs.push_revealed_card(cid, source, false, owner, "ability");
        }
        if !revealed_card_ids.is_empty() {
            let player_label =
                super::util::target_player_label(&target, gs.ability_master_id().as_deref());
            let turn = gs.turn_number;
            gs.push_rule_log(format!(
                "[Turn {}] {} [[log_reveal_hand:n={}]]",
                turn,
                player_label,
                revealed_card_ids.len()
            ));
        }
        if !effect_started {
            let cost_source = gs.current_ability_source_card_id();
            let cost_owner = util::target_player_index(&target, gs.ability_master_id().as_deref());
            for &cid in &revealed_card_ids {
                gs.push_revealed_cost_card(cid, cost_source, false, cost_owner, "cost");
            }
        }

        if ctx.count == 0 && ctx.allow_skip && !effect_started && !all_indices.is_empty() {
            self.selected_cards = all_indices.iter().map(|&i| i as i16).collect();
            let hand_len = {
                let p = gs.resolve_target_player_mut(&target);
                p.hand.cards.len()
            };
            let remaining_indices: Vec<usize> = (0..hand_len)
                .filter(|&i| !all_indices.contains(&i))
                .collect();
            if !remaining_indices.is_empty() {
                self.pending_choice = Some(
                    Choice::select_cards(
                        Zone::Hand.to_str(),
                        0,
                        "Select more cards to reveal from hand (or skip to finish)",
                        true,
                    )
                    .description_ja(Some(
                        "手札からさらにカードを選択して公開（スキップで終了）".to_string(),
                    ))
                    .filtered_indices(Some(remaining_indices))
                    .card_type(ctx.card_type.clone())
                    .is_reveal(true)
                    .target_player_id(Some(target.clone()))
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
        }

        self.clear_choice_state(gs);
        self.resume_pending_actions(gs)
    }

    fn handle_revealed_cards_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        validate_card: &mut impl FnMut(i16) -> bool,
        dst_str: &str,
    ) -> Result<(), String> {
        let mapped = ctx.mfi(&ctx.indices);
        let moved = self.move_from_revealed(gs, &mapped, validate_card, dst_str);
        // If this is a select action, store selected cards for downstream
        // sequential actions (e.g. gain_resource with heart_colors_from_selected_card).
        if ctx.is_select_action {
            for &cid in &moved {
                if !self.selected_cards.contains(&cid) {
                    self.selected_cards.push(cid);
                }
            }
        }
        // Track moved cards so preceding_moved conditions on the same
        // ability (e.g. conditional_on_result) can see them.
        self.moved_cards.extend(moved.iter().copied());
        // Apply resource_on_select if present — grants resource (e.g. blade)
        // automatically when a card is selected from revealed_cards.
        let res = self
            .current_effect
            .as_ref()
            .and_then(|e| e.resource_on_select_any().cloned());
        if let Some(ref res) = res {
            self.execute_effect(gs, res)?;
        }
        if self
            .current_effect
            .as_ref()
            .is_some_and(|e| e.discard_remaining_any().unwrap_or(false))
        {
            let remaining: Vec<i16> = gs
                .revealed_cards
                .iter()
                .filter(|&&cid| !moved.contains(&cid))
                .copied()
                .collect();
            let player = gs.resolve_target_player_mut("self");
            for &cid in &remaining {
                player.waitroom.add_card(cid);
            }
            gs.revealed_cards.clear();
        }
        // When user skips an optional revealed_cards selection, clear any
        // pending sequential actions (e.g. re_yell should not fire after skip).
        let is_opponent_action = ctx.target_player_id.as_deref() == Some("opponent");
        if ctx.indices.is_empty()
            && ctx.allow_skip
            && ctx.count > 0
            && !is_opponent_action
            && moved.is_empty()
        {
            gs.ability_queue.take_pending_actions();
        }
        Ok(())
    }

    fn handle_success_live_zone_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        let mapped = ctx.mfi(&ctx.indices);
        let edst = gs.entry_destination().map(|s| s.to_string());
        let dst_str = ctx
            .destination
            .clone()
            .or(edst)
            .unwrap_or_else(|| Zone::Discard.to_string().into());
        let target = ctx.target_player_id.as_deref().unwrap_or("self");
        let card_db = gs.card_database.clone();
        let player = gs.resolve_target_player_mut(target);
        let card_ids =
            util::resolve_indices_to_ids(player, Zone::SuccessLiveZone.to_str(), &mapped);
        if !card_ids.is_empty() {
            let valid_ids: Vec<i16> = card_ids
                .into_iter()
                .filter(|&cid| validate_card(cid))
                .collect();
            if !valid_ids.is_empty() {
                let mc = util::move_cards(
                    player,
                    &valid_ids,
                    Zone::SuccessLiveZone.to_str(),
                    &dst_str,
                    None,
                    &card_db,
                );
                if mc > 0 {
                    self.moved_cards = valid_ids.clone().into();
                    gs.recently_moved_cards = Some(valid_ids.into());
                    gs.recently_moved_from_zone = Some(Zone::SuccessLiveZone.to_str().to_string());
                }
            }
        }
        self.clear_choice_state(gs);
        let pending = gs.ability_queue.take_pending_actions();
        let filtered: Vec<AbilityEffect> = pending
            .into_iter()
            .filter(|cmd| cmd.source != Some(Zone::SuccessLiveZone))
            .collect();
        gs.ability_queue.set_pending_actions(filtered);
        self.resume_pending_actions(gs)
    }

    fn handle_entry_cost_reveal(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        let cost = gs.entry_cost().cloned().unwrap();
        let card_db = gs.card_database.clone();
        let player = gs.resolve_target_player("self");
        let card_ids: Vec<i16> = ctx
            .indices
            .iter()
            .filter_map(|&idx| {
                if idx < player.hand.cards.len() {
                    let cid = player.hand.cards[idx];
                    let passes =
                        util::card_matches_type(
                            &card_db,
                            cid,
                            cost.card_type_any().map(|ct| ct.as_card_str()),
                        ) && util::card_matches_characters(
                            &card_db,
                            cid,
                            cost.characters_any().map(|v| &**v),
                        ) && match cost.group_names_any().as_ref() {
                            Some(groups) => groups.iter().any(|g| {
                                util::card_matches_group_str(&card_db, cid, Some(g.as_str()))
                            }),
                            None => true,
                        } && util::card_matches_cost_limit(&card_db, cid, cost.cost_limit_any());
                    if passes {
                        Some(cid)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        let count = cost.count.unwrap_or(1) as usize;
        if card_ids.len() < count {
            return Err("Not enough valid cards to reveal for cost".to_string());
        }
        if !card_ids.is_empty() {
            let card_db = &gs.card_database;
            let names: Vec<String> = card_ids
                .iter()
                .filter_map(|id| card_db.get_card(*id))
                .map(|c| c.name.to_string())
                .collect();
            let turn = gs.turn_number;
            let player_num = if core::ptr::eq(gs.resolve_target_player("self"), &gs.player1) {
                1
            } else {
                2
            };
            if !names.is_empty() {
                gs.push_rule_log(format!(
                    "[Turn {}] P{} [[log_reveal_cost]]: {}",
                    turn,
                    player_num,
                    names.join(", ")
                ));
            }
        }
        let cost_source = gs.current_ability_source_card_id();
        let cost_owner: Option<u8> = if core::ptr::eq(gs.resolve_target_player("self"), &gs.player1)
        {
            Some(0)
        } else {
            Some(1)
        };
        for card_id in card_ids {
            gs.push_revealed_card(card_id, cost_source, false, cost_owner, "cost");
            gs.push_revealed_cost_card(card_id, cost_source, false, cost_owner, "cost");
        }
        self.finalize_choice(gs, context)
    }

    // Q86 / Q122: Handle user's selection from looked_at_cards
    //
    // After the user selects which cards to take, this method:
    //   1. Reveals selected cards if the effect requires it (reveal flag)
    //   2. Delegates to handle_select_cards_looked_at (move_cards.rs)
    //      which moves selected to destination, remainder to discard
    //   3. If max/optional/any_number and more cards can be selected,
    //      re-prompts the user with remaining looked_at_cards
    //
    // Q86: If the selection empties the looked_at set AND the source deck
    //   becomes empty, refresh (Rule 10.2.2.1) fires at next check timing.
    //
    // Q122: If the effect is a rearrangement (look + rearrange on deck),
    //   no refresh happens until cards actually leave the deck zone.
    fn handle_looked_at_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        let valid = ctx.mfi(&ctx.indices);

        let select_action_entry = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.ability.effect.as_ref())
            .and_then(|ef| ef.compound.select_action.clone())
            .or_else(|| {
                self.current_effect.as_ref().and_then(|ef| {
                    if ef.action == ActionType::SelectCards {
                        Some(Box::new(ef.clone()))
                    } else {
                        None
                    }
                })
            });
        let is_select_cards = select_action_entry
            .as_ref()
            .map(|sa| sa.action == ActionType::SelectCards)
            .unwrap_or(false);

        if select_action_entry
            .as_ref()
            .and_then(|sa| sa.reveal_any())
            .unwrap_or(false)
        {
            self.reveal_selected_looked_at(gs, &valid);
        }

        if is_select_cards {
            self.handle_select_cards_looked_at(gs, &valid, None, None)?;

            if matches!(self.pending_choice, Some(Choice::SelectTarget { ref target, .. }) if super::enums::SelectTargetKind::from_str(target) == Some(super::enums::SelectTargetKind::Order))
            {
                return Ok(());
            }

            if is_select_cards && !valid.is_empty() {
                let sa = select_action_entry.as_ref();
                let any_number = sa.and_then(|s| s.any_number_any()).unwrap_or(false);
                let is_max = sa.and_then(|s| s.max).unwrap_or(false);
                let is_optional = sa.and_then(|s| s.optional).unwrap_or(false);
                let json_count = sa.and_then(|s| s.count).unwrap_or(1) as usize;
                let max_count = ctx.count;
                let selected_count = valid.len();
                let remaining = gs.looked_at_cards.len();

                let can_reprompt =
                    is_max || is_optional || any_number || json_count > selected_count;
                if can_reprompt && max_count > selected_count && remaining > 0 {
                    let remaining_max = max_count - selected_count;
                    let ct = ctx.card_type.clone();
                    // Compute filtered_indices for remaining looked_at cards
                    let remaining_indices: Vec<usize> = {
                        let card_db = &gs.card_database;
                        let filter = sa.map(|s| super::util::CardFilter::from_effect(s));
                        match filter {
                            Some(ref f) if f.has_filter() => gs
                                .looked_at_cards
                                .iter()
                                .enumerate()
                                .filter(|&(_, &cid)| f.matches(card_db, cid, false))
                                .map(|(i, _)| i)
                                .collect(),
                            _ => (0..gs.looked_at_cards.len()).collect(),
                        }
                    };
                    self.pending_choice = Some(
                        Choice::select_cards(
                            Zone::LookedAt.to_str(),
                            remaining_max,
                            format!("Select up to {} more card(s) from the {} remaining looked-at cards", remaining_max, remaining),
                            is_optional || any_number,
                        )
                        .description_ja(Some(format!(
                            "残り{}枚の確認済みカードから最大{}枚選択",
                            remaining, remaining_max
                        )))
                        .card_type(ct)
                        .cost_limit(
                            sa.and_then(|s| s.cost_limit_any()),
                            sa.and_then(|s| s.cost_limit_operator_any()).map(|s| s.to_string()),
                        )
                        .group(sa.as_ref().and_then(|s| s.group_names_any()).and_then(|v| v.first().cloned()))
                        .characters(sa.and_then(|s| s.characters_any().cloned()))
                        .filtered_indices(Some(remaining_indices))
                        .build(),
                    );
                    self.execution_context = context.clone();
                    return Ok(());
                }
            }
        } else {
            self.handle_select_cards_looked_at(
                gs,
                &valid,
                ctx.destination.clone(),
                ctx.discard_remaining,
            )?;
        }
        self.finalize_choice(gs, context)
    }

    fn handle_stage_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        if ctx.is_select_action {
            if ctx.indices.is_empty() {
                gs.ability_queue.take_pending_actions();
                self.selected_cards = SmallVec::new();
                // For under_member optional skip, mark no move
                if gs.ability_queue.current_entry().is_some_and(|e| {
                    e.ability.effect.as_ref().is_some_and(|eff| {
                        eff.source
                            .as_ref()
                            .is_some_and(|z| z.to_str() == "under_member")
                            || eff
                                .compound
                                .primary_effect
                                .as_ref()
                                .is_some_and(|pe| {
                                    pe.source
                                        .as_ref()
                                        .is_some_and(|z| z.to_str() == "under_member")
                                })
                    })
                }) {
                    self.last_move_moved_any = Some(false);
                    self.moved_cards.clear();
                    gs.recently_moved_cards = Some(SmallVec::new());
                }
                log::debug!("[SELECT_STAGE] no selection: cleared pending commands");
            }
            let stage_indices = ctx.mfi(&ctx.indices);
            log::debug!(
                "[SELECT_STAGE_DBG] ctx.indices={:?} mfi={:?} filtered={:?} stage={:?}",
                ctx.indices,
                stage_indices,
                ctx.filtered_indices,
                gs.resolve_target_player(ctx.target_player_id.as_deref().unwrap_or("self"))
                    .stage
                    .stage
            );
            let player =
                gs.resolve_target_player_mut(ctx.target_player_id.as_deref().unwrap_or("self"));
            let mut cards: Vec<i16> = Vec::new();
            for &idx in stage_indices.iter() {
                if idx < 3 && player.stage.stage[idx] != -1 {
                    let cid = player.stage.stage[idx];
                    if validate_card(cid) {
                        cards.push(cid);
                    } else {
                        log::debug!("[SELECT_STAGE] validate failed cid={} stage idx {}", cid, idx);
                    }
                } else {
                    log::debug!("[SELECT_STAGE] empty slot idx {}", idx);
                }
            }
            log::debug!(
                "[SELECT_STAGE] is_select_action=true cards={:?} filtered_idx={:?}",
                cards,
                ctx.filtered_indices
            );
            for &cid in &cards {
                if !self.selected_cards.contains(&cid) {
                    self.selected_cards.push(cid);
                }
            }
            // During cost phase (effect_started=false) the only is_select_action
            // stage selections come from change_state: "wait" costs. Apply the
            // wait directly so the cost is truly paid — the cost handler won't
            // re-enter because abilities.rs skips setting cost_paid for deferred
            // choices.
            if !cards.is_empty()
                && gs
                    .ability_queue
                    .current_entry()
                    .is_some_and(|e| !e.effect_started)
            {
                for &cid in &cards {
                    gs.mods.add_orientation_modifier(cid, "wait");
                    gs.last_cost_wait_member = Some(cid);
                }
            }
            // Burn!! under_member move: Stage selection with is_select_action true
            // is used to choose which member's under energies to move. The actual
            // energy movement must happen here, not in a second resolve call which
            // is never re-entered for conditional_on_result.
            if !cards.is_empty()
                && (gs.ability_queue.current_entry().is_some_and(|e| {
                    e.ability
                        .effect
                        .as_ref()
                        .is_some_and(|eff| {
                            eff.source
                                .as_ref()
                                .is_some_and(|z| z.to_str() == "under_member")
                                || eff
                                    .compound
                                    .primary_effect
                                    .as_ref()
                                    .is_some_and(|pe| {
                                        pe.source
                                            .as_ref()
                                            .is_some_and(|z| z.to_str() == "under_member")
                                    })
                        })
                }) || self
                    .current_effect
                    .as_ref()
                    .is_some_and(|eff| {
                        eff.source
                            .as_ref()
                            .is_some_and(|z| z.to_str() == "under_member")
                            || eff
                                .compound
                                .primary_effect
                                .as_ref()
                                .is_some_and(|pe| {
                                    pe.source
                                        .as_ref()
                                        .is_some_and(|z| z.to_str() == "under_member")
                                })
                    }) || {
                        // Fallback: any Stage is_select_action where selected member has under cards → treat as under_member move
                        cards.iter().any(|&mid| {
                            gs.resolve_target_player("self")
                                .stage
                                .stage
                                .iter()
                                .position(|&id| id == mid)
                                .map(|idx| {
                                    !gs.player1.stage.under_cards[idx].is_empty()
                                        || !gs.player2.stage.under_cards[idx].is_empty()
                                })
                                .unwrap_or(false)
                        })
                    })
            {
                if let Some(entry) = gs.ability_queue.current_entry().cloned() {
                    let target = entry
                        .ability
                        .effect
                        .as_ref()
                        .and_then(|eff| eff.target.as_deref())
                        .unwrap_or("self");
                    let mut moved: Vec<i16> = Vec::new();
                    for &mid in &cards {
                        if let Some(idx) = gs
                            .resolve_target_player(target)
                            .stage
                            .stage
                            .iter()
                            .position(|&id| id == mid)
                        {
                            let mut m =
                                crate::ability::move_cards::drain_under_cards_to_energy_zone(
                                    gs, target, idx,
                                );
                            moved.append(&mut m);
                        }
                    }
                    if !moved.is_empty() {
                        self.moved_cards.extend(moved.iter().copied());
                        gs.recently_moved_cards = Some(moved.clone().into());
                        gs.recently_moved_from_zone = Some("under_member".to_string());
                        self.last_move_moved_any = Some(true);
                    } else {
                        self.last_move_moved_any = Some(false);
                    }
                    log::debug!(
                        "[UNDER_MEMBER_VIA_STAGE] moved {} energies for members {:?} -> moved_cards={:?}",
                        moved.len(),
                        cards,
                        self.moved_cards
                    );
                }
            }
        } else {
            let edst = gs.entry_destination().map(|s| s.to_string());
            let dst_str = ctx
                .destination
                .clone()
                .or(edst)
                .unwrap_or_else(|| Zone::Discard.to_string().into());
            let card_db = gs.card_database.clone();
            let player =
                gs.resolve_target_player_mut(ctx.target_player_id.as_deref().unwrap_or("self"));
            let card_ids = util::resolve_indices_to_ids(player, Zone::Stage.to_str(), &ctx.indices);
            let valid_ids: Vec<i16> = card_ids
                .into_iter()
                .filter(|&cid| validate_card(cid))
                .collect();
            let mut last_vacated = None;
            for &cid in &valid_ids {
                if let Some(pos) = player.stage.stage.iter().position(|&x| x == cid) {
                    last_vacated = Some(pos);
                }
            }
            let moved_count = util::move_cards(
                player,
                &valid_ids,
                Zone::Stage.to_str(),
                &dst_str,
                None,
                &card_db,
            );
            if moved_count > 0 {
                if let Some(pos) = last_vacated {
                    gs.last_vacated_stage_area = Some(pos as u8);
                }
                self.selected_cards = valid_ids.clone().into();
                self.moved_cards = valid_ids.clone().into();
                gs.recently_moved_cards = Some(valid_ids.into());
                gs.recently_moved_from_zone = Some("stage".to_string());
            }
        }
        Ok(())
    }

    fn filter_discard_by_budget(
        &self,
        gs: &GameState,
        ctx: &SelectionContext,
        waitroom_cards: &[i16],
    ) -> (Option<u8>, Vec<usize>) {
        let spent: u8 = self
            .selected_cards
            .iter()
            .filter_map(|&cid| gs.card_database.get_card(cid).and_then(|c| c.cost))
            .sum();
        let remaining_budget = ctx.cost_total.map(|tb| tb.saturating_sub(spent));
        let use_budget_filter = match ctx.cost_total_operator.as_deref() {
            Some(op) => op == "<=",
            None => ctx.cost_total.is_some(),
        };
        let all_idxs: Vec<usize> = if use_budget_filter {
            let rb = remaining_budget.unwrap_or(0);
            waitroom_cards
                .iter()
                .enumerate()
                .filter(|(_, &cid)| {
                    gs.card_database
                        .get_card(cid)
                        .and_then(|c| c.cost)
                        .unwrap_or(99)
                        <= rb
                })
                .map(|(idx, _)| idx)
                .collect()
        } else {
            (0..waitroom_cards.len()).collect()
        };
        (remaining_budget, all_idxs)
    }

    fn handle_discard_selection(
        &mut self,
        gs: &mut GameState,
        ctx: &SelectionContext,
        context: &ExecutionContext,
        _validate_card: &mut impl FnMut(i16) -> bool,
    ) -> Result<(), String> {
        let mapped_indices = ctx.mfi(&ctx.indices);
        if ctx.is_select_action {
            let target = ctx
                .target_player_id
                .as_deref()
                .unwrap_or("self")
                .to_string();
            let player = gs.resolve_target_player_mut(&target);
            let mut cards: Vec<i16> = Vec::new();
            for &i in mapped_indices.iter() {
                if i < player.waitroom.cards.len() {
                    let cid = player.waitroom.cards[i];
                    if !self.selected_cards.contains(&cid) {
                        self.selected_cards.push(cid);
                        cards.push(cid);
                    }
                }
            }
            let selected_so_far = self.selected_cards.len();
            if ctx.count > 0 && !mapped_indices.is_empty() && selected_so_far < ctx.count {
                let remaining = ctx.count - selected_so_far;
                let waitroom_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.waitroom.cards.to_vec()
                };
                let filtered_idxs: Vec<usize> = (0..waitroom_cards.len())
                    .filter(|&i| !self.selected_cards.contains(&waitroom_cards[i]))
                    .collect();
                let remaining_count = filtered_idxs.len();
                let fi = if filtered_idxs.is_empty() {
                    None
                } else {
                    Some(filtered_idxs)
                };
                let desc = format!(
                    "Select {} more card(s) from discard from {} remaining",
                    remaining, remaining_count
                );
                let desc_ja = format!(
                    "控え室から残り{}枚中さらに{}枚選択",
                    remaining_count, remaining
                );
                self.pending_choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Discard.to_str(),
                        remaining,
                        desc,
                        desc_ja,
                        false,
                        fi,
                        Some(target.clone()),
                        ctx.cost_total,
                        ctx.cost_total_operator.clone(),
                    )
                    .is_select_action(true)
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            return self.finalize_choice(gs, &context);
        } else {
            if !mapped_indices.is_empty() && ctx.count > 0 && mapped_indices.len() < ctx.count {
                let target = ctx
                    .target_player_id
                    .as_deref()
                    .unwrap_or("self")
                    .to_string();
                let waitroom_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.waitroom.cards.to_vec()
                };
                let mut filtered_idxs: Vec<usize> = Vec::new();
                if !self.selected_cards.is_empty() {
                    for (idx, cid) in waitroom_cards.iter().enumerate() {
                        if self.selected_cards.contains(cid) && !filtered_idxs.contains(&idx) {
                            filtered_idxs.push(idx);
                        }
                    }
                }
                let current_card_ids: Vec<i16> = mapped_indices
                    .iter()
                    .filter_map(|&idx| {
                        if idx < waitroom_cards.len() {
                            Some(waitroom_cards[idx])
                        } else {
                            None
                        }
                    })
                    .collect();
                for &cid in &current_card_ids {
                    if !self.selected_cards.contains(&cid) {
                        self.selected_cards.push(cid);
                    }
                }
                self.execute_selected_cards_from_zone(
                    gs,
                    Zone::Discard.to_str(),
                    &mapped_indices,
                    ctx.card_type.as_deref(),
                    ctx.cost_limit,
                    ctx.cost_limit_operator.as_deref(),
                    ctx.cost_total,
                    ctx.cost_total_operator.as_deref(),
                    ctx.group.as_deref(),
                    ctx.characters.as_ref(),
                    ctx.target_player_id.as_deref(),
                )?;
                if self.sub_choice_created {
                    self.sub_choice_created = false;
                    let remaining = ctx.count - mapped_indices.len();
                    if remaining > 0 {
                        let waitroom_cards: Vec<i16> = {
                            let p = gs.resolve_target_player_mut(&target);
                            p.waitroom.cards.to_vec()
                        };
                        let (remaining_budget, all_idxs) =
                            self.filter_discard_by_budget(gs, ctx, &waitroom_cards);
                        if !all_idxs.is_empty() || ctx.allow_skip {
                            let desc = format!(
                                "Select {} more card(s) from discard{}",
                                remaining,
                                if ctx.allow_skip {
                                    " (or skip to finish)"
                                } else {
                                    ""
                                }
                            );
                            let desc_ja = format!(
                                "控え室からさらに{}枚選択{}",
                                remaining,
                                if ctx.allow_skip {
                                    "（スキップで終了）"
                                } else {
                                    ""
                                }
                            );
                            let fi = if all_idxs.is_empty() {
                                Some(vec![])
                            } else {
                                Some(all_idxs)
                            };
                            let reprompt = self
                                .build_reprompt(
                                    ctx,
                                    Zone::Discard.to_str(),
                                    remaining,
                                    desc,
                                    desc_ja,
                                    ctx.allow_skip,
                                    fi,
                                    Some(target),
                                    remaining_budget,
                                    Some("<=".to_string()),
                                )
                                .build();
                            let pending = gs.ability_queue.take_pending_actions();
                            self.pending_reprompt_choice = Some(reprompt);
                            gs.ability_queue.set_pending_actions(pending);
                        }
                    }
                    return Ok(());
                }
                let waitroom_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.waitroom.cards.to_vec()
                };
                let (remaining_budget, all_idxs) =
                    self.filter_discard_by_budget(gs, ctx, &waitroom_cards);
                let remaining = ctx.count - mapped_indices.len();
                let desc = format!(
                    "Select {} more card(s) from discard{}",
                    remaining,
                    if ctx.allow_skip {
                        " (or skip to finish)"
                    } else {
                        ""
                    }
                );
                let desc_ja = format!(
                    "控え室からさらに{}枚選択{}",
                    remaining,
                    if ctx.allow_skip {
                        "（スキップで終了）"
                    } else {
                        ""
                    }
                );
                let fi = if all_idxs.is_empty() {
                    Some(vec![])
                } else {
                    Some(all_idxs)
                };
                self.pending_choice = Some(
                    self.build_reprompt(
                        ctx,
                        Zone::Discard.to_str(),
                        remaining,
                        desc,
                        desc_ja,
                        ctx.allow_skip,
                        fi,
                        Some(target),
                        remaining_budget,
                        Some("<=".to_string()),
                    )
                    .build(),
                );
                self.store_pending_choice(gs);
                return Ok(());
            }
            let target = ctx
                .target_player_id
                .as_deref()
                .unwrap_or("self")
                .to_string();
            let mut all_idxs: Vec<usize> = mapped_indices.to_vec();
            let prev_ids = self.selected_cards.clone();
            if !prev_ids.is_empty() {
                let waitroom_cards: Vec<i16> = {
                    let p = gs.resolve_target_player_mut(&target);
                    p.waitroom.cards.to_vec()
                };
                for (idx, cid) in waitroom_cards.iter().enumerate() {
                    if prev_ids.contains(cid) && !all_idxs.contains(&idx) {
                        all_idxs.push(idx);
                    }
                }
                self.selected_cards.clear();
            }
            self.execute_selected_cards_from_zone(
                gs,
                Zone::Discard.to_str(),
                &all_idxs,
                ctx.card_type.as_deref(),
                ctx.cost_limit,
                ctx.cost_limit_operator.as_deref(),
                ctx.cost_total,
                ctx.cost_total_operator.as_deref(),
                ctx.group.as_deref(),
                ctx.characters.as_ref(),
                ctx.target_player_id.as_deref(),
            )?;
            if self.sub_choice_created {
                self.store_pending_choice(gs);
            }
        }
        self.handle_selection_epilogue(gs, context)
    }

    fn handle_selection_epilogue(
        &mut self,
        gs: &mut GameState,
        context: &ExecutionContext,
    ) -> Result<(), String> {
        if gs.ability_queue.has_pending_actions() && self.pending_choice.is_none() {
            self.clear_choice_state(gs);
            return self.resume_pending_actions(gs);
        }
        self.finalize_choice(gs, context)
    }

    fn handle_select_target(
        &mut self,
        gs: &mut GameState,
        target: &str,
        selected: &str,
    ) -> Result<(), String> {
        let choice_card_no = gs.entry_choice_card_no();
        let conditional_choice = gs.entry_conditional_choice();
        log::debug!(
            "[HST] target={} selected={} choice_card_no={:?} activating={:?}",
            target,
            selected,
            choice_card_no,
            gs.activating_card
        );

        // choice_card_no-based routing
        match choice_card_no.as_ref() {
            Some(ChoiceRoute::Choice) => {
                if let Some(ConditionalChoice::Effects(all_options)) = conditional_choice {
                    let idx: usize = selected.parse().unwrap_or(0);
                    if idx < all_options.len() {
                        let selected_effect = (*all_options[idx]).clone();
                        let mut remaining = all_options.clone();
                        remaining.remove(idx);
                        // Schedule the selected effect and optional re-prompt
                        // as pending commands. resume_pending_commands runs them
                        // sequentially — after the selected effect completes (and
                        // any sub-choices), the re-prompt command re-enters
                        // execute_choice which sees the updated conditional_choice
                        // enum and creates a new SelectTarget for the
                        // remaining options.
                        let commands = vec![selected_effect];
                        let wants_re_prompt = !remaining.is_empty()
                            && gs.entry_effect().map_or(false, |eff| {
                                if eff.any_number_any().unwrap_or(false) {
                                    return true;
                                }
                                if let Some(ref alt_cond) = eff.compound.alternative_condition {
                                    let ctx =
                                        ConditionContext::with_moved_cards(gs, &self.moved_cards);
                                    ctx.evaluate_condition(alt_cond)
                                        && eff.alternative_count_type_any().as_deref()
                                            == Some("any_number")
                                } else {
                                    false
                                }
                            });
                        if wants_re_prompt {
                            let desc: Vec<String> = remaining
                                .iter()
                                .map(|o| {
                                    o.answers_any()
                                        .as_ref()
                                        .map(|a| a.join(", "))
                                        .unwrap_or_else(|| o.text.to_string())
                                })
                                .collect();
                            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                                entry.conditional_choice =
                                    Some(ConditionalChoice::Effects(remaining));
                            }
                            self.pending_reprompt_choice = Some(Choice::SelectTarget {
                                target: "choice".to_string(),
                                description: desc.join(" / "),
                                description_en: Some(desc.join(" / ")),
                                description_ja: Some(desc.join(" / ")),
                                allow_skip: true,
                                options: None,
                            });
                        }
                        gs.ability_queue.set_pending_actions(commands);
                        self.pending_choice = None; // clear stale
                        return self.resume_pending_actions(gs);
                    }
                } else if self.pending_choice.is_some() {
                    self.pending_choice = None;
                }
                return self.resume_pending_actions(gs);
            }
            Some(ChoiceRoute::ChoiceString) => {
                return self.handle_choice_string_selection(gs, selected, conditional_choice);
            }
            Some(ChoiceRoute::Raw(s)) if s.starts_with("position_change") => {
                log::debug!(
                    "[HPCC_MATCH] routing to handle_position_change_choice: s={} selected={}",
                    s,
                    selected
                );
                return self.handle_position_change_choice(gs, choice_card_no, selected);
            }
            _ => {}
        }

        // target-based routing via typed enum
        // Note: Choice and ChoiceString are handled by choice_card_no above
        match super::enums::SelectTargetKind::from_str(target) {
            Some(super::enums::SelectTargetKind::Choice) => {
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::ChoiceString) => {
                return self.handle_choice_string_store(gs, selected, conditional_choice);
            }
            Some(super::enums::SelectTargetKind::PayOptionalCostSkipOptionalCost) => {
                return self.handle_optional_cost_payment(gs, selected);
            }
            Some(super::enums::SelectTargetKind::PayCostAllDiscard) => {
                return self.handle_pay_cost_all_discard(gs, selected);
            }
            Some(super::enums::SelectTargetKind::DoubleBatonTouch) => {
                return self.handle_double_baton_touch(gs, selected);
            }
            Some(super::enums::SelectTargetKind::PrimaryAlternative) => {
                return self.handle_primary_alternative(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ApplyReplacement) => {
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::ChooseRequiredHearts) => {
                gs.prohibition_effects
                    .push(format!("chosen_required_hearts:{}", selected));
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::PositionDestination) => {
                return self.handle_position_destination(gs, selected);
            }
            Some(super::enums::SelectTargetKind::HeartColor) => {
                return self.handle_heart_color_selection(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ChoiceType) => {
                self.clear_choice_state(gs);
                return Ok(());
            }
            Some(super::enums::SelectTargetKind::ChoiceCondition) => {
                return self.handle_choice_condition(gs, selected);
            }
            Some(super::enums::SelectTargetKind::ConditionalOptional) => {
                return self.handle_conditional_optional(gs, selected);
            }
            Some(super::enums::SelectTargetKind::DrawAnyNumber) => {
                return self.handle_draw_any_number(gs, selected);
            }
            Some(super::enums::SelectTargetKind::Order) => {
                return self.handle_order_selection(gs, selected);
            }
            Some(super::enums::SelectTargetKind::SelfOrOpponent) => {
                let chosen = match selected {
                    "自分" => "self",
                    "相手" => "opponent",
                    _ => return Err("Invalid choice for SelfOrOpponent".to_string()),
                };
                log::debug!("[SELFOR] chosen={}", chosen);
                self.spawn_context.target = Some(chosen.to_string());
                if let Some(ref current) = self.current_effect {
                    log::debug!(
                        "[SELFOR] current.action={} steps={:?}",
                        current.action,
                        current.effect_steps.as_ref().map(|s| s.len())
                    );
                    if let Some(ref steps) = current.effect_steps {
                        if let Some(inner) = steps.first() {
                            let mut modified = inner.clone();
                            log::debug!(
                                "[SELFOR] inner.action={} steps_before={}",
                                modified.action,
                                modified.effect_steps.as_ref().map(|s| s.len()).unwrap_or(0)
                            );
                            Self::set_chosen_target(&mut modified, chosen);
                            log::debug!(
                                "[SELFOR] after_set: target={:?} has_la={} has_sa={}",
                                modified.target,
                                modified.compound.look_action.is_some(),
                                modified.compound.select_action.is_some()
                            );
                            self.pending_choice = None;
                            gs.ability_queue.set_pending_actions(vec![*modified]);
                            let res = self.resume_pending_actions(gs);
                            log::debug!(
                                "[SELFOR] after resume: pending={:?} res={:?}",
                                self.pending_choice.is_some(),
                                res
                            );
                            match res {
                                Ok(()) => return Ok(()),
                                Err(e) => {
                                    log::debug!("[SELFOR] inner effect failed: {}", e);
                                    self.clear_choice_state(gs);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                log::debug!("[SELFOR] no inner effect found");
                self.clear_choice_state(gs);
                return Ok(());
            }
            None => {}
        }

        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_draw_any_number(&mut self, gs: &mut GameState, selected: &str) -> Result<(), String> {
        let count: usize = selected.parse().unwrap_or(0);
        if let Some(effect) = gs.entry_effect().cloned() {
            let source = effect.source_any().unwrap_or(Zone::Deck.to_str());
            let destination = effect
                .destination
                .map(|d| d.to_str())
                .unwrap_or(Zone::Hand.to_str());
            let ct_binding = effect.card_type_any();
            let card_type = ct_binding.map(|ct| ct.as_card_str());
            let card_db = gs.card_database.clone();
            let target = effect.target.as_deref().unwrap_or("self");
            let player = gs.resolve_target_player_mut(target);
            if count > 0 {
                crate::ability::effects::draw_cards_for_player(
                    player,
                    count as u8,
                    source,
                    destination,
                    card_type,
                    false,
                    None,
                    &card_db,
                    None,
                )?;
            }
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_order_selection(&mut self, gs: &mut GameState, selected: &str) -> Result<(), String> {
        let ctx = self.execution_context.clone();
        if let ExecutionContext::LookAndSelect { step } = ctx {
            if let LookAndSelectStep::Finalize { destination, .. } = step {
                if Zone::from_str(&destination) == Some(Zone::Deck) {
                    if let Ok(idx) = selected.parse::<usize>() {
                        if idx < gs.looked_at_cards.len() {
                            let card = gs.looked_at_cards.remove(idx);
                            gs.looked_at_cards.insert(0, card);
                        }
                    }
                    let card_ids: Vec<i16> = gs.looked_at_cards.iter().rev().copied().collect();
                    let target = self
                        .spawn_context
                        .target
                        .clone()
                        .or_else(|| {
                            gs.entry_effect()
                                .and_then(|e| e.target.clone().map(|s| s.to_string()))
                        })
                        .unwrap_or_else(|| "self".to_string().into());
                    let player = gs.resolve_target_player_mut(&target);
                    for card_id in card_ids {
                        player.main_deck.cards.insert(0, card_id);
                    }
                    gs.looked_at_cards.clear();
                }
            }
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_position_change_choice(
        &mut self,
        gs: &mut GameState,
        choice_card_no: Option<ChoiceRoute>,
        selected: &str,
    ) -> Result<(), String> {
        log::debug!(
            "[HPCC] entry: choice_card_no={:?} selected={} entry_effect={:?} activating={:?}",
            choice_card_no,
            selected,
            gs.entry_effect().map(|e| e.action.clone()),
            gs.activating_card
        );
        if selected == "skip" {
            self.formation_plan.clear();
            // Clear pending actions so conditional-sequential sub-effects
            // (e.g. gain_resource gated by this position change) don't fire
            // when the player skips.
            gs.ability_queue.set_pending_actions(vec![]);
            self.clear_choice_state_and_resume(gs)?;
            self.execution_context = ExecutionContext::None;
            return Ok(());
        }
        if let Some(effect) = gs.entry_effect().cloned() {
            let mut modified = effect.clone();
            let dest = match selected {
                "0" | "left" | "left_side" => "left",
                "1" | "center" => "center",
                "2" | "right" | "right_side" => "right",
                _ => selected
                    .split_once(':')
                    .map(|(_, pos)| pos)
                    .unwrap_or(selected),
            };
            let mut explicit_source_pos: Option<String> = None;
            if let Some(ChoiceRoute::Raw(ref raw)) = choice_card_no {
                if let Some(tgt) = raw.strip_prefix("position_change:") {
                    if tgt == "opponent:front" {
                        if let Some(f) = modified.kind.as_deref_mut().and_then(|k| k.filter_mut()) {
                            f.source_position = Some(selected.into());
                        }
                        let pc_ok =
                            self.execute_position_change_with_destination(gs, &modified, "front");
                        if let Err(ref e) = pc_ok {
                            log::debug!("Failed to execute position change: {}", e);
                        }
                        if pc_ok.is_ok() {
                            let pid = gs
                                .ability_queue
                                .current_entry()
                                .map(|e| e.player_id.clone())
                                .unwrap_or_default();
                            gs.trigger_auto_abilities_for_player_with_event(
                                &pid,
                                &TriggerEvent {
                                    moved_cards: gs
                                        .recently_moved_cards
                                        .clone()
                                        .unwrap_or_default()
                                        .into(),
                                    position_change_occurred: gs.position_change_occurred_this_turn,
                                    ..Default::default()
                                },
                            );
                        }
                        self.clear_choice_state_and_resume(gs)?;
                        return Ok(());
                    } else if tgt.contains(':') {
                        let parts: Vec<&str> = tgt.splitn(2, ':').collect();
                        modified.target = Some(parts[0].into());
                        if parts[1] == "select" {
                            // Check if the selected option encodes player info (e.g. "self:left",
                            // "opponent:center") — used when effect.target was null (any member).
                            if let Some((player_prefix, position)) = selected.split_once(':') {
                                modified.target = Some(player_prefix.into());
                                explicit_source_pos = Some(position.to_string());
                            } else {
                                explicit_source_pos = Some(dest.to_string());
                            }
                        } else if super::util::stage_position_index(parts[1]).is_some() {
                            explicit_source_pos = Some(parts[1].to_string());
                        } else {
                            modified.set_target_member(Some(parts[1].into()));
                        }
                    } else {
                        modified.target = Some(tgt.into());
                    }
                }
            }
            let was_select = choice_card_no.as_ref().is_some_and(|ccn| match ccn {
                ChoiceRoute::Raw(s) => s.contains(":select"),
                _ => false,
            });
            if was_select {
                // The user just chose WHICH member to move (the source position).
                let target_str = modified
                    .target
                    .clone()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "self".to_string().into());
                self.clear_choice_meta(gs);
                self.pending_choice = None;
                // If the card fixes the destination (e.g. "…をセンターエリアにポジション
                // チェンジ" → position=center), use it directly — no destination choice.
                // Otherwise ask the player where to move the chosen member.
                let fixed_dest = modified
                    .position_any()
                    .and_then(|p| p.get_position())
                    .map(|s| s.to_string());
                if let Some(fixed_dest) = fixed_dest {
                    // The selected source is `dest` (from the source-selection choice);
                    // move that member to the fixed destination.
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                            "position_change:{}:{}",
                            target_str, dest
                        )));
                    }
modified.destination = Some(Zone::from_source_str(dest));
                    if let Some(ref src_pos) = explicit_source_pos {
                        let target = modified.target.as_deref().unwrap_or("self");
                        let player = gs.resolve_target_player_mut(target);
                        let src_idx = crate::ability::util::stage_position_index(src_pos)
                            .unwrap_or(999);
                        let dst_idx =
                            crate::ability::util::stage_position_index(&fixed_dest).unwrap_or(999);
                        if src_idx != dst_idx
                            && src_idx < 3
                            && dst_idx < 3
                            && player.stage.stage[src_idx] != -1
                        {
                            use crate::zones::MemberArea;
                            let from = match src_idx {
                                0 => MemberArea::LeftSide,
                                1 => MemberArea::Center,
                                _ => MemberArea::RightSide,
                            };
                            let to = match dst_idx {
                                0 => MemberArea::LeftSide,
                                1 => MemberArea::Center,
                                _ => MemberArea::RightSide,
                            };
                            let tgt_id = player.stage.stage[dst_idx];
                            let src_id = player.stage.stage[src_idx];
                            if let Err(e) = player.stage.position_change(from, to) {
                                log::debug!("Direct position change failed: {}", e);
                            } else {
                                gs.position_change_occurred_this_turn = true;
                                if src_id != -1 {
                                    gs.record_card_movement(src_id);
                                }
                                if tgt_id != -1 {
                                    gs.record_card_movement(tgt_id);
                                }
                            }
                        }
                        let pid = gs
                            .ability_queue
                            .current_entry()
                            .map(|e| e.player_id.clone())
                            .unwrap_or_default();
                        gs.trigger_auto_abilities_for_player_with_event(
                            &pid,
                            &TriggerEvent {
                                moved_cards: gs.recently_moved_cards.clone().unwrap_or_default().into(),
                                position_change_occurred: gs.position_change_occurred_this_turn,
                                ..Default::default()
                            },
                        );
                    }
                    self.clear_choice_state_and_resume(gs)?;
                    return Ok(());
                }
                // Compute destinations directly — all positions except source
                // are valid (no group_names/exclude_self filtering for dest).
                let all_positions = ["left", "center", "right"];
                let valid_destinations: Vec<String> = all_positions
                    .iter()
                    .filter(|pos| **pos != dest)
                    .map(|pos| pos.to_string())
                    .collect();
                if valid_destinations.is_empty() {
                    self.clear_choice_state_and_resume(gs)?;
                    return Ok(());
                }
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                        "position_change:{}:{}",
                        target_str, dest
                    )));
                }
                let from_label = match dest {
                    "left" | "left_side" => "Left",
                    "center" => "Center",
                    "right" | "right_side" => "Right",
                    _ => "?",
                };
                self.pending_choice = Some(Choice::SelectTarget {
                    target: "position|destination".to_string(),
                    description: format!(
                        "Choose destination for position change (currently at {})",
                        from_label
                    ),
                    description_en: Some(format!(
                        "Choose destination for position change (currently at {})",
                        from_label
                    )),
                    description_ja: Some(format!("移動先を選択（現在: {}）", from_label)),
                    allow_skip: false,
                    options: Some(valid_destinations),
                });
                self.store_pending_choice(gs);
                return Ok(());
            }

            // Formation change: store the assignment and either present the next
            // destination choice or finalize the batch once all members are set.
            if !self.formation_plan.is_empty() {
                let target_card_id = choice_card_no.as_ref().and_then(|ccn| match ccn {
                    ChoiceRoute::Raw(s) => s
                        .strip_prefix("position_change:self:")
                        .and_then(|id_str| id_str.parse::<i16>().ok()),
                    _ => None,
                });
                let entry_idx = target_card_id
                    .and_then(|cid| self.formation_plan.iter().position(|(id, _)| *id == cid));
                if let Some(idx) = entry_idx {
                    self.formation_plan[idx].1 = dest.to_string();
                    let next = self.formation_plan.iter().position(|(_, d)| d.is_empty());
                    if let Some(next_idx) = next {
                        let next_cid = self.formation_plan[next_idx].0;
                        let next_cname = self
                            .card_database
                            .get_card(next_cid)
                            .map(|c| c.name.to_string())
                            .unwrap_or_else(|| "member".to_string().into());
                        let current_pos = {
                            let player = gs.resolve_target_player_mut(
                                effect.target.as_deref().unwrap_or("self"),
                            );
                            player.stage.stage.iter().position(|&id| id == next_cid)
                        };
                        let pos_name = match current_pos {
                            Some(0) => "Left",
                            Some(1) => "Center",
                            Some(2) => "Right",
                            _ => "?",
                        };
                        let valid_destinations =
                            self.compute_valid_position_destinations(gs, &effect, "self");
                        if valid_destinations.is_empty() {
                            self.finalize_formation_change(gs)?;
                            let pid = gs
                                .ability_queue
                                .current_entry()
                                .map(|e| e.player_id.clone())
                                .unwrap_or_default();
                            gs.trigger_auto_abilities_for_player_with_event(
                                &pid,
                                &TriggerEvent {
                                    moved_cards: gs
                                        .recently_moved_cards
                                        .clone()
                                        .unwrap_or_default()
                                        .into(),
                                    position_change_occurred: gs.position_change_occurred_this_turn,
                                    ..Default::default()
                                },
                            );
                            self.clear_choice_state_and_resume(gs)?;
                            return Ok(());
                        }
                        if let Some(entry) = gs.ability_queue.current_entry_mut() {
                            entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                                "position_change:self:{}",
                                next_cid
                            )));
                        }
                        self.pending_choice = Some(Choice::SelectTarget {
                            target: "position|destination".to_string(),
                            description: format!(
                                "Choose destination for {} (currently at {})",
                                next_cname, pos_name
                            ),
                            description_en: Some(format!(
                                "Choose destination for {} (currently at {})",
                                next_cname, pos_name
                            )),
                            description_ja: Some(format!(
                                "{}の移動先を選択（現在: {}）",
                                next_cname, pos_name
                            )),
                            allow_skip: effect.optional.unwrap_or(false),
                            options: Some(valid_destinations),
                        });
                        self.store_pending_choice(gs);
                        return Ok(());
                    } else {
                        // All members assigned — execute batch swap.
                        self.finalize_formation_change(gs)?;
                        let pid = gs
                            .ability_queue
                            .current_entry()
                            .map(|e| e.player_id.clone())
                            .unwrap_or_default();
                        gs.trigger_auto_abilities_for_player_with_event(
                            &pid,
                            &TriggerEvent {
                                moved_cards: gs
                                    .recently_moved_cards
                                    .clone()
                                    .unwrap_or_default()
                                    .into(),
                                position_change_occurred: gs.position_change_occurred_this_turn,
                                ..Default::default()
                            },
                        );
                        self.clear_choice_state_and_resume(gs)?;
                        return Ok(());
                    }
                }
            }

            modified.destination = Some(Zone::from_source_str(dest));
            // Use explicit_source_pos if available (handles Choice/compound
            // effects where source_position_any() returns None).
            if let Some(ref src_pos) = explicit_source_pos {
                let target = modified.target.as_deref().unwrap_or("self");
                let player = gs.resolve_target_player_mut(target);
                let src_idx = crate::ability::util::stage_position_index(src_pos).unwrap_or(999);
                let dst_idx = crate::ability::util::stage_position_index(dest).unwrap_or(999);
                if src_idx != dst_idx
                    && src_idx < 3
                    && dst_idx < 3
                    && player.stage.stage[src_idx] != -1
                {
                    use crate::zones::MemberArea;
                    let from = match src_idx {
                        0 => MemberArea::LeftSide,
                        1 => MemberArea::Center,
                        _ => MemberArea::RightSide,
                    };
                    let to = match dst_idx {
                        0 => MemberArea::LeftSide,
                        1 => MemberArea::Center,
                        _ => MemberArea::RightSide,
                    };
                    let tgt_id = player.stage.stage[dst_idx];
                    let src_id = player.stage.stage[src_idx];
                    if let Err(e) = player.stage.position_change(from, to) {
                        log::debug!("Direct position change failed: {}", e);
                    } else {
                        gs.position_change_occurred_this_turn = true;
                        if src_id != -1 {
                            gs.record_card_movement(src_id);
                        }
                        if tgt_id != -1 {
                            gs.record_card_movement(tgt_id);
                        }
                    }
                }
                let pid = gs
                    .ability_queue
                    .current_entry()
                    .map(|e| e.player_id.clone())
                    .unwrap_or_default();
                gs.trigger_auto_abilities_for_player_with_event(
                    &pid,
                    &TriggerEvent {
                        moved_cards: gs.recently_moved_cards.clone().unwrap_or_default().into(),
                        position_change_occurred: gs.position_change_occurred_this_turn,
                        ..Default::default()
                    },
                );
            } else {
                if let Err(e) = self.execute_position_change_with_destination(gs, &modified, dest) {
                    log::debug!("Failed to execute position change: {}", e);
                } else {
                    let pid = gs
                        .ability_queue
                        .current_entry()
                        .map(|e| e.player_id.clone())
                        .unwrap_or_default();
                    gs.trigger_auto_abilities_for_player_with_event(
                        &pid,
                        &TriggerEvent {
                            moved_cards: gs.recently_moved_cards.clone().unwrap_or_default().into(),
                            position_change_occurred: gs.position_change_occurred_this_turn,
                            ..Default::default()
                        },
                    );
                }
            }
            self.selected_area = None;
        }
        self.clear_choice_state_and_resume(gs)?;
        Ok(())
    }

    fn apply_effect_modification<F>(
        &mut self,
        gs: &mut GameState,
        modifier: F,
    ) -> Result<(), String>
    where
        F: Fn(&mut AbilityEffect),
    {
        self.clear_choice_state(gs);
        if let Some(mut effect) = gs.entry_effect().cloned() {
            modifier(&mut effect);
            gs.ability_queue.set_pending_actions(vec![effect]);
        }
        self.resume_pending_actions(gs)?;
        Ok(())
    }

    fn handle_primary_alternative(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        self.apply_effect_modification(gs, |effect| {
            let chosen = if selected == "1" || selected == "alternative" || selected == "secondary"
            {
                effect
                    .alternative_effect_any()
                    .clone()
                    .or(effect.compound.primary_effect.as_ref())
            } else {
                effect.compound.primary_effect.as_ref()
            };
            if let Some(sub_effect) = chosen {
                *effect = *(*sub_effect).clone();
            }
        })
    }

    fn handle_position_destination(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        // Check if we have a saved MoveCardsPosition context (card was already taken from source zone).
        // If so, place the card directly instead of re-running the entire effect which would fail
        // because the card is no longer in the source zone.
        let ctx = core::mem::replace(&mut self.execution_context, ExecutionContext::None);
        match ctx {
            ExecutionContext::MoveCardsPosition {
                card_id,
                target,
                source_zone,
                ..
            } => {
                // Consume use_limit for optional effects BEFORE the mutable
                // player borrow, to avoid borrow conflicts with gs.
                let mut use_limit_key: Option<(i16, usize, u8)> = None;
                if selected != "skip" {
                    if let Some(entry) = gs.ability_queue.current_entry() {
                        let is_optional = entry
                            .ability
                            .effect
                            .as_ref()
                            .is_some_and(|e| e.optional.unwrap_or(false));
                        if is_optional && entry.ability.use_limit.is_some() {
                            if let Some(cid) = entry.card_id.or(gs.activating_card) {
                                use_limit_key = Some((cid, entry.ability_index, gs.turn_number));
                            }
                        }
                    }
                }
                let player = gs.resolve_target_player_mut(&target);
                let destination = selected;
                // Player chose to skip — card stays in waitroom, no-op.
                if destination == "skip" {
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!("[DECK_DIAG] skip — card stays in waitroom");
                    }
                    // Mark optional as skipped so the RWC handler doesn't record use_limit.
                    if let Some(entry) = gs.ability_queue.current_entry_mut() {
                        entry.optional_cost_result = Some(false);
                    }
                    self.clear_choice_state(gs);
                    return self.resume_pending_actions(gs);
                }
                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    log::debug!("[DECK_DIAG] handle_position_destination ctx=MoveCardsPosition card_id={} dest={} src={}", card_id, destination, source_zone);
                }
                // Remove card from source zone first, then place in destination.
                // The card was left in place when the deck_top_or_bottom choice was created.
                let src_is_hand = source_zone == Zone::Hand.to_str();
                if source_zone == Zone::Discard.to_str()
                    || source_zone == Zone::Waitroom.to_str()
                    || source_zone == "those_cards"
                {
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!(
                            "[DECK_DIAG] waitroom before retain={:?} removing card_id={}",
                            player.waitroom.cards,
                            card_id
                        );
                    }
                    let before = player.waitroom.cards.len();
                    player.waitroom.cards.retain(|c| *c != card_id);
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!(
                            "[DECK_DIAG] waitroom after retain={:?} ({} -> {})",
                            player.waitroom.cards,
                            before,
                            player.waitroom.cards.len()
                        );
                    }
                } else if src_is_hand {
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!(
                            "[DECK_DIAG] hand before retain={:?} removing card_id={}",
                            player.hand.cards,
                            card_id
                        );
                    }
                    player.hand.cards.retain(|c| *c != card_id);
                }
                crate::ability::util::place_card_in_zone(
                    player,
                    card_id,
                    &destination,
                    None,
                    false,
                    1,
                );
                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    log::debug!(
                        "[DECK_DIAG] deck len={} first={:?} last={:?}",
                        player.main_deck.cards.len(),
                        player.main_deck.cards.first(),
                        player.main_deck.cards.last()
                    );
                }
                // The player chose to place the card — insert use_limit key
                // after the player borrow is done (avoid conflicts with gs).
                if let Some(key) = use_limit_key {
                    gs.record_ability_use(key);
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!("[DECK_DIAG] recorded use_limit for optional effect");
                    }
                }
                self.clear_choice_state(gs);
                self.resume_pending_actions(gs)
            }
            _ => {
                // Fall back to effect modification for non-card-specific position choices
                // (e.g. stage position selection).
                self.apply_effect_modification(gs, |effect| {
                    effect.destination = Some(Zone::from_source_str(selected));
                })
            }
        }
    }

    fn handle_double_baton_touch(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        if selected == "skip" {
            self.clear_choice_state(gs);
            return Ok(());
        }
        // selected is "left,center" or "left,right" or "center,right"
        let areas: Vec<&str> = selected.split(',').collect();
        if areas.len() != 2 {
            self.clear_choice_state(gs);
            return Err(format!("Invalid double baton selection: {}", selected));
        }
        let area_from_name = |name: &str| -> usize {
            match name {
                "left" => 0,
                "center" => 1,
                "right" => 2,
                _ => 999,
            }
        };
        let idx1 = area_from_name(areas[0]);
        let idx2 = area_from_name(areas[1]);
        if idx1 > 2 || idx2 > 2 {
            self.clear_choice_state(gs);
            return Err(format!(
                "Invalid area in double baton selection: {}",
                selected
            ));
        }
        // Replace both members (move to waitroom)
        let p1 = &mut gs.player1;
        if idx1 < 3 && p1.stage.stage[idx1] != -1 {
            let old_id = p1.stage.stage[idx1];
            p1.waitroom.add_card(old_id);
            p1.stage.stage[idx1] = -1;
            // Rule 9.6.2.1.2.1: Card left stage, clean up tracking (member-based, not area-based).
            p1.deployed_this_turn.retain(|id| *id != old_id);
        }
        if idx2 < 3 && p1.stage.stage[idx2] != -1 {
            let old_id = p1.stage.stage[idx2];
            p1.waitroom.add_card(old_id);
            p1.stage.stage[idx2] = -1;
            p1.deployed_this_turn.retain(|id| *id != old_id);
        }
        let p1_id = gs.player1.id.clone();
        // For double baton via choice, the arriving card has already been placed on stage
        // before this handler runs. Get it from the stage placement.
        let arriving = gs
            .player1
            .stage
            .stage
            .iter()
            .find(|&&cid| cid != -1)
            .copied();
        gs.record_baton_touch(&p1_id, arriving);
        gs.record_baton_touch(&p1_id, arriving);
        // Place the activating card (Sumire) in the player's chosen placement area.
        // The play_baton_touch constant ability fires after PlayMemberToStage has already
        // placed the card, so at this point the card is already on stage. If this is triggered
        // as a standalone ability (non-PlayMemberToStage path), we need to move the card.
        // For now, the baton touch replacements and lock are recorded so debut abilities
        // see baton_touch_count > 0 and can trigger correctly.
        self.clear_choice_state(gs);
        self.resume_pending_actions(gs)
    }

    fn handle_conditional_optional(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        // Safety timeout: a runaway optional-cost re-trigger loop (see the
        // each_time watcher fix) must abort rather than hang forever.
        use crate::compat::atomic::AtomicU32;
        use core::sync::atomic::Ordering;
        static CHOICE_CALLS: AtomicU32 = AtomicU32::new(0);
        if CHOICE_CALLS.fetch_add(1, Ordering::Relaxed) > 200_000 {
            log::error!(
                "[CHOICE_TIMEOUT] exceeded 200k conditional-optional resolutions; aborting"
            );
            gs.ability_queue.clear();
            return Ok(());
        }
        // Read before clear_choice_state since it nullifies conditional_choice.
        let entry_eff = gs.entry_effect().cloned();
        let cond_choice = gs.entry_conditional_choice();
        let chose_yes = selected == "1" || selected == "yes";
        self.clear_choice_state(gs);

        // Set choice_card_no and optional_cost_result AFTER clear_choice_state
        // which would otherwise erase them.
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = Some(crate::ability::types::ChoiceRoute::OptionalCost);
            entry.optional_cost_result = Some(chose_yes);
        }

        // Prefer cond_choice (the sub-effect with optional_action/conditional_action)
        // over entry_eff (which may return a parent sequential effect lacking these fields).
        let effect = cond_choice
            .and_then(|cc| match cc {
                ConditionalChoice::Effect(e) => Some(e),
                _ => None,
            })
            .or_else(|| entry_eff);
        if let Some(effect) = effect {
            let is_negation = effect.compound.conditional_negation.unwrap_or(false);
            let chose_yes = selected == "1" || selected == "yes";
            // Record use_limit when the player chose to pay (but NOT when declined)
            if chose_yes {
                if let Some(entry) = gs.ability_queue.current_entry() {
                    if let Some(cid) = entry.card_id {
                        let turn = gs.turn_number;
                        // Record the use against the ability ACTUALLY being resolved
                        // (entry.ability_index), not the first ability on the card with a
                        // use_limit. Recording the wrong ability (e.g. the parent's ab#0
                        // instead of this each_time ab#1) leaves the real ability unmarked
                        // as used, so a later movement re-trigger re-queues it forever.
                        let key = (cid, entry.ability_index, turn);
                        gs.record_ability_use(key);
                    }
                }
            }
            let cmd = match (chose_yes, is_negation) {
                // yes + negation → optional_action fires, conditional skipped
                (true, true) => effect.compound.optional_action.map(|a| *a),
                // yes + no negation → conditional_action fires (the follow-up)
                (true, false) => effect.compound.conditional_action.map(|a| *a),
                // no + negation → conditional_action fires (the penalty)
                (false, true) => effect.compound.conditional_action.map(|a| *a),
                // no + no negation → nothing fires
                (false, false) => None,
            };
            // The player accepted the optional placement. Arm the gate so the
            // trailing "そうしたとき" consequence only fires if the placement
            // actually moved a card. Any optional move that auto-skips (e.g. a
            // group with no card in discard) clears it, suppressing the draw —
            // Q118 "それぞれ1枚ずつ…置いてもよい。そうしたとき" is all-or-nothing.
            if chose_yes && !is_negation {
                if let Some(entry) = gs.ability_queue.current_entry_mut() {
                    entry.optional_moves_all_moved = Some(true);
                }
            }
            if let Some(cmd) = cmd {
                gs.ability_queue.set_pending_actions(vec![cmd]);
            }
        }
        self.resume_pending_actions(gs)?;
        Ok(())
    }

    fn handle_heart_color_selection(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        const HEART_VALS: [&str; 7] = [
            "heart00", "heart01", "heart02", "heart03", "heart04", "heart05", "heart06",
        ];
        let idx: usize = selected.parse().unwrap_or(0);
        if idx < HEART_VALS.len() {
            let color = HEART_VALS[idx];
            gs.prohibition_effects
                .push(format!("selected_heart_color:{}", color));
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(ConditionalChoice::Str(color.to_string()));
            }
        }
        self.clear_choice_state(gs);
        self.resume_pending_actions(gs)
    }

    fn handle_choice_condition(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        let idx: usize = selected.parse().unwrap_or(0);
        if let Some(options) = gs.entry_cost().and_then(|c| c.compound.actions.clone()) {
            if idx < options.len() {
                let label = options[idx]
                    .text
                    .split("}}")
                    .last()
                    .unwrap_or(&options[idx].text)
                    .trim()
                    .to_string();
                let pp = gs.player_prefix();
                let act_name = gs
                    .activating_card
                    .and_then(|id| gs.card_database.get_card(id))
                    .map(|c| c.name.to_string());
                gs.log_entry(
                    format!(
                        "{pp} {}: [choice] {} ✓",
                        act_name.as_deref().unwrap_or(""),
                        label
                    ),
                    &pp,
                    gs.activating_card,
                    act_name,
                    "choice",
                );
                // Take the pending SelectTarget so we can detect if pay_cost
                // creates a new sub-choice (e.g., SelectCard for discard from hand).
                let old_choice = self.pending_choice.take();
                self.pay_cost(gs, &options[idx])?;
                if self.pending_choice.is_some() {
                    // pay_cost created a sub-choice — signal to preserve it
                    self.sub_choice_created = true;
                } else {
                    // pay_cost resolved immediately — restore original choice
                    // so clear_choice_state can clean it up properly.
                    self.pending_choice = old_choice;
                }
            }
        }
        self.clear_choice_state(gs);
        Ok(())
    }

    fn handle_heart_selection(
        &mut self,
        gs: &mut GameState,
        count: u8,
        colors: &[String],
    ) -> Result<(), String> {
        if let Some(chosen) = colors.first() {
            let color = crate::card::parse_heart_color(chosen);
            if let Some(card_id) = gs.activating_card {
                gs.set_heart_override(card_id, color, count.max(1), "live_end");
            }
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.conditional_choice = Some(ConditionalChoice::Str(chosen.clone()));
            }
        }
        self.pending_choice = None;
        self.finalize_choice(gs, &self.execution_context.clone())
    }

    pub fn clear_choice_meta(&mut self, gs: &mut GameState) {
        if let Some(entry) = gs.ability_queue.current_entry_mut() {
            entry.choice_card_no = None;
            entry.conditional_choice = None;
        }
        // On skip/clear, drop deferred costs so they aren't paid.
        self.pending_deferred_costs.clear();
    }

    fn clear_choice_state(&mut self, gs: &mut GameState) {
        if self.sub_choice_created {
            self.sub_choice_created = false;
        } else {
            self.pending_choice = None;
        }
        self.clear_choice_meta(gs);
    }

    fn clear_choice_state_and_resume(&mut self, gs: &mut GameState) -> Result<(), String> {
        self.clear_choice_state(gs);
        self.resume_pending_actions(gs)
    }

    /// Recursively set target on all sub-effects that don't have an explicit target.
    /// Excludes draw/draw_card actions (always target self) and select_cards (handled
    /// via spawn_context.target fallback in handle_select_card_internal).
    fn set_chosen_target(effect: &mut AbilityEffect, target: &str) {
        if matches!(
            effect.action,
            ActionType::DrawCard | ActionType::SelectCards
        ) {
            return;
        }
        if effect.target.is_none() || effect.target.as_deref() == Some("self") {
            effect.target = Some(target.into());
        }
        if let Some(ref mut la) = effect.compound.look_action {
            Self::set_chosen_target(la, target);
        }
        if let Some(ref mut sa) = effect.compound.select_action {
            Self::set_chosen_target(sa, target);
        }
        if let Some(ref mut actions) = effect.compound.actions {
            for a in actions.iter_mut() {
                Self::set_chosen_target(a, target);
            }
        }
        if let Some(ref mut steps) = effect.effect_steps {
            for s in steps.iter_mut() {
                Self::set_chosen_target(s, target);
            }
        }
        if let Some(ref mut oa) = effect.compound.optional_action.as_mut() {
            Self::set_chosen_target(oa, target);
        }
        if let Some(ref mut pri) = effect.compound.primary_effect {
            Self::set_chosen_target(pri, target);
        }
        if let Some(ref mut ca) = effect.compound.conditional_action {
            Self::set_chosen_target(ca, target);
        }
        if let Some(ref mut fu) = effect.compound.followup_action {
            Self::set_chosen_target(fu, target);
        }
    }
}
