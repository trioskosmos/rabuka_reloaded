use super::*;

impl super::resolver::AbilityResolver {
    pub(in crate::ability::choice) fn handle_position_change_choice(
        &mut self,
        gs: &mut GameState,
        choice_card_no: Option<ChoiceRoute>,
        selected: &str,
    ) -> Result<(), String> {
        log::debug!(
            "[HPCC] entry: choice_card_no={:?} selected={} entry_effect={:?} activating={:?}",
            choice_card_no,
            selected,
            gs.entry_effect().map(|e| e.action),
            gs.activating_card
        );
        if selected == "skip" {
            self.carried.formation_plan.clear();
            // Clear pending actions so conditional-sequential sub-effects
            // (e.g. gain_resource gated by this position change) don't fire
            // when the player skips.
            gs.ability_queue.set_pending_actions(vec![]);
            self.clear_choice_state_and_resume(gs)?;
            self.in_flight.execution_context = ExecutionContext::None;
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
                        if let Some(f) = modified.kind.as_mut().map(crate::Arc::make_mut).and_then(|k| k.filter_mut()) {
                            f.source_position = Some(selected.into());
                        }
                        let pc_ok =
                            self.execute_position_change_with_destination(gs, &modified, "front");
                        if let Err(ref e) = pc_ok {
                            log::debug!("Failed to execute position change: {}", e);
                        }
                        if pc_ok.is_ok() {
                            gs.trigger_auto_abilities_for_movement_current();
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
                    .unwrap_or_else(|| "self".to_string());
                self.clear_choice_meta(gs);
                self.awaiting.choice = None;
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
                        Self::apply_selected_position_change(gs, &modified, src_pos, &fixed_dest);
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
                self.awaiting.choice = Some(Choice::SelectTarget {
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
            if !self.carried.formation_plan.is_empty() {
                let target_card_id = choice_card_no.as_ref().and_then(|ccn| match ccn {
                    ChoiceRoute::Raw(s) => s
                        .strip_prefix("position_change:self:")
                        .and_then(|id_str| id_str.parse::<i16>().ok()),
                    _ => None,
                });
                let entry_idx = target_card_id
                    .and_then(|cid| self.carried.formation_plan.iter().position(|(id, _)| *id == cid));
                if let Some(idx) = entry_idx {
                    self.carried.formation_plan[idx].1 = dest.to_string();
                    let next = self.carried.formation_plan.iter().position(|(_, d)| d.is_empty());
                    if let Some(next_idx) = next {
                        let next_cid = self.carried.formation_plan[next_idx].0;
                        let next_cname = self
                            .session.card_database
                            .get_card(next_cid)
                            .map(|c| c.name.to_string())
                            .unwrap_or_else(|| "member".to_string());
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
                            gs.trigger_auto_abilities_for_movement_current();
                            self.clear_choice_state_and_resume(gs)?;
                            return Ok(());
                        }
                        if let Some(entry) = gs.ability_queue.current_entry_mut() {
                            entry.choice_card_no = Some(ChoiceRoute::Raw(format!(
                                "position_change:self:{}",
                                next_cid
                            )));
                        }
                        self.awaiting.choice = Some(Choice::SelectTarget {
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
                        gs.trigger_auto_abilities_for_movement_current();
                        self.clear_choice_state_and_resume(gs)?;
                        return Ok(());
                    }
                }
            }

            modified.destination = Some(Zone::from_source_str(dest));
            // Use explicit_source_pos if available (handles Choice/compound
            // effects where source_position_any() returns None).
            if let Some(ref src_pos) = explicit_source_pos {
                Self::apply_selected_position_change(gs, &modified, src_pos, dest);
            } else {
                if let Err(e) = self.execute_position_change_with_destination(gs, &modified, dest) {
                    log::debug!("Failed to execute position change: {}", e);
                } else {
                    gs.trigger_auto_abilities_for_movement_current();
                }
            }
            self.selection.area = None;
        }
        self.clear_choice_state_and_resume(gs)?;
        Ok(())
    }

    pub(in crate::ability::choice) fn apply_selected_position_change(
        gs: &mut GameState,
        effect: &AbilityEffect,
        source: &str,
        destination: &str,
    ) {
        let target = effect.target.as_deref().unwrap_or("self");
        let player = gs.resolve_target_player_mut(target);
        let src_idx = util::stage_position_index(source).unwrap_or(999);
        let dst_idx = util::stage_position_index(destination).unwrap_or(999);
        if src_idx != dst_idx && src_idx < 3 && dst_idx < 3 && player.stage.stage[src_idx] != -1 {
            let from = util::pos_to_area(src_idx);
            let to = util::pos_to_area(dst_idx);
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
        gs.trigger_auto_abilities_for_movement_current();
    }

    pub(in crate::ability::choice) fn apply_effect_modification<F>(
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

    pub(in crate::ability::choice) fn handle_primary_alternative(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        self.apply_effect_modification(gs, |effect| {
            let chosen: Option<&AbilityEffect> = if selected == "1"
                || selected == "alternative"
                || selected == "secondary"
            {
                effect
                    .alternative_effect_any()
                    .or(effect.compound_branch().and_then(|c| c.primary_effect.as_deref()))
            } else {
                effect.compound_branch().and_then(|c| c.primary_effect.as_deref())
            };
            if let Some(sub_effect) = chosen {
                *effect = sub_effect.clone();
            }
        })
    }

    pub(in crate::ability::choice) fn handle_position_destination(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        if self.selection.cards.is_empty() {
            if let Some(&card_id) = gs.revealed_cost_cards.last() {
                self.selection.cards.push(card_id);
            }
        }
        // Check if we have a saved MoveCardsPosition context (card was already taken from source zone).
        // If so, place the card directly instead of re-running the entire effect which would fail
        // because the card is no longer in the source zone.
        let ctx = core::mem::replace(&mut self.in_flight.execution_context, ExecutionContext::None);
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
                    destination,
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

    pub(in crate::ability::choice) fn handle_double_baton_touch(
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
        // NOTE: record_baton_touch is called TWICE deliberately — this matches the
        // canonical double-baton path in phases.rs ("Record 2 baton touches"), which
        // also records 2. Do NOT "fix" this into a single call.
        // Known limitations of this standalone path (execute_play_baton_touch
        // count>1): it mutates player1 unconditionally, and `arriving` is derived
        // as the first non-empty stage slot, which misidentifies the arriving card
        // when an untouched member occupies slot 0. The web UI uses the canonical
        // play_member_to_stage path instead; pinned by
        // sumire_double_baton_choice_path_records_two_touches.
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

    pub(in crate::ability::choice) fn handle_conditional_optional(
        &mut self,
        gs: &mut GameState,
        selected: &str,
    ) -> Result<(), String> {
        // Safety timeout: a runaway optional-cost re-trigger loop (see the
        // each_time watcher fix) must abort rather than hang forever.
        // Counted PER QUEUE ENTRY (card + ability index): the counter resets
        // whenever resolution moves to a different ability, so long batch or
        // arena runs can no longer accumulate toward the cap across games and
        // silently clear an unrelated game's queue.
        use crate::compat::atomic::AtomicU32;
        use core::sync::atomic::Ordering;
        static CHOICE_CALLS: AtomicU32 = AtomicU32::new(0);
        static LAST_CARD: AtomicU32 = AtomicU32::new(u32::MAX);
        static LAST_ABILITY: AtomicU32 = AtomicU32::new(u32::MAX);
        let entry = gs.ability_queue.current_entry();
        let key_card = u32::try_from(
            entry
                .and_then(|e| e.card_id)
                .map_or(-1, i32::from),
        )
        .unwrap_or(u32::MAX);
        let key_ability = entry
            .map(|e| e.ability_index.u32_count())
            .unwrap_or(u32::MAX);
        if LAST_CARD.load(Ordering::Relaxed) != key_card
            || LAST_ABILITY.load(Ordering::Relaxed) != key_ability
        {
            CHOICE_CALLS.store(0, Ordering::Relaxed);
            LAST_CARD.store(key_card, Ordering::Relaxed);
            LAST_ABILITY.store(key_ability, Ordering::Relaxed);
        }
        if CHOICE_CALLS.fetch_add(1, Ordering::Relaxed) > 200_000 {
            log::error!(
                "[CHOICE_TIMEOUT] exceeded 200k conditional-optional resolutions for one ability; aborting"
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
            .or(entry_eff);
        if let Some(effect) = effect {
            let is_negation = effect
                .compound_branch()
                .and_then(|c| c.conditional_negation)
                .unwrap_or(false);
            if chose_yes && !is_negation {
                if let Some(optional) = effect
                    .compound_branch()
                    .and_then(|c| c.optional_action.as_ref())
                {
                    if optional.action == ActionType::PayEnergy {
                        let mut payment = optional.clone();
                        payment.set_optional(Some(false));
                        log::debug!(
                            "[CONDITIONAL_OPTIONAL] source={:?} action=pay_energy energy={:?} target={:?}",
                            self.session.activating_card_id,
                            payment.energy_count_any(),
                            payment.target
                        );
                        self.execute_effect(gs, &payment)?;
                    }
                }
            }
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
            let cmd =
                super::compound::conditional::route_conditional_branch(&effect, chose_yes, is_negation);
            log::debug!(
                "[CONDITION] source={:?} action={} answer={:?} accepted={} negation={} branch={} next_action={:?}",
                self.session.activating_card_id, effect.action, selected, chose_yes, is_negation,
                if cmd.is_none() { "none" } else if chose_yes && is_negation { "optional" } else { "conditional" },
                cmd.as_ref().map(|a| a.action)
            );
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
                gs.ability_queue.set_pending_actions(vec![*cmd]);
            }
        }
        self.resume_pending_actions(gs)?;
        Ok(())
    }

}
