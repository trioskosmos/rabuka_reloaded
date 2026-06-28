use super::super::enums::Zone;
use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ChoiceRoute, ExecutionContext};
use super::super::util;
use crate::card::AbilityEffect;
use crate::game_state::GameState;

impl AbilityResolver {
    pub(crate) fn execute_change_state(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        state_change: &str,
        target: &str,
        count: u32,
        max: bool,
        card_type: Option<&str>,
        cost_limit: Option<u32>,
        optional: bool,
        group_name: Option<&str>,
        self_cost: bool,
        source: Option<&str>,
        destination: Option<&str>,
        cost_limit_operator: Option<String>,
        characters: Option<&Vec<String>>,
        blade_limit: Option<u32>,
        blade_limit_operator: Option<&str>,
    ) -> Result<(), String> {
        let state_change = state_change.to_string();
        let target = target.to_string();
        let card_type_filter = card_type.map(|s| s.to_string());
        // When targeting opponent, group_names is trigger-level metadata
        // (from the wrapper's condition), not an effect filter.
        let group_filter = if target == "opponent" {
            None
        } else {
            group_name.map(|s| s.to_string())
        };

        if optional {
            // Only offer the optional choice if there's at least one valid target.
            // For state_change="active", the member must be in "wait" state.
            // For state_change="wait", any member works.
            // If no valid targets exist, return early without creating the choice.
            let can_target = if state_change == "active" {
                let p = gs.resolve_target_player(&target);
                let ct = card_type_filter.as_deref();
                let gf = group_filter.as_deref();
                p.stage.stage.iter().any(|&cid| {
                    if cid == -1 {
                        return false;
                    }
                    let is_wait = gs
                        .mods
                        .get_orientation_modifier(cid)
                        .is_some_and(|o| o == "wait");
                    if !is_wait {
                        return false;
                    }
                    if let Some(t) = ct {
                        if !util::card_matches_type(&gs.card_database, cid, Some(t)) {
                            return false;
                        }
                    }
                    if let Some(g) = gf {
                        if !util::card_matches_group_str(&gs.card_database, cid, Some(g)) {
                            return false;
                        }
                    }
                    true
                })
            } else if state_change == "wait" && effect.state.as_deref() == Some("active") {
                // wait effect targeting only active members:
                // check if there is at least one active (non-wait) member
                let p = gs.resolve_target_player(&target);
                let ct = card_type_filter.as_deref();
                let gf = group_filter.as_deref();
                p.stage.stage.iter().any(|&cid| {
                    if cid == -1 {
                        return false;
                    }
                    // member is active when there is no "wait" orientation modifier
                    let is_active = gs
                        .mods
                        .get_orientation_modifier(cid)
                        .is_none_or(|o| o != "wait");
                    if !is_active {
                        return false;
                    }
                    if let Some(t) = ct {
                        if !util::card_matches_type(&gs.card_database, cid, Some(t)) {
                            return false;
                        }
                    }
                    if let Some(g) = gf {
                        if !util::card_matches_group_str(&gs.card_database, cid, Some(g)) {
                            return false;
                        }
                    }
                    true
                })
            } else {
                true // no state filter: any member is a valid target
            };
            if !can_target {
                log::debug!(
                    "[EXEC_CHANGE_STATE] optional {} but no valid targets — skipping",
                    state_change
                );
                return Ok(());
            }
            self.pending_choice = Some(Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: format!("Change state to {} (pay optional cost)?", state_change),
                description_en: None,
                description_ja: None,
                allow_skip: optional,
                options: None,
            });
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some(ChoiceRoute::ChangeState);
            }
            return Ok(());
        }

        // Draw from energy deck and place in energy zone with state (e.g. wait)
        if Zone::from_str(source.unwrap_or("")) == Some(Zone::Deck)
            && Zone::from_str(destination.unwrap_or("")) == Some(Zone::Energy)
        {
            self.execute_energy_placement(gs, &state_change, &target, count);
            return Ok(());
        }

        // Member card state change — operate on stage
        let is_member_op = card_type_filter.as_deref() == Some("member_card") || self_cost;

        if is_member_op {
            eprintln!(
                "[EXEC_CHANGE_STATE] member_op: target={} count={} max={} state_change={}",
                target, count, max, state_change
            );

            // Check cannot_activate_by_effect restriction before mutable borrow.
            let is_cannot_activate_by_effect = if state_change == "active" {
                let target_player = gs.resolve_target_player(&target);
                gs.cannot_activate_members.contains(&target_player.id)
            } else {
                false
            };

            let exclude_self_id = if effect.exclude_self.unwrap_or(false) {
                gs.activating_card
            } else {
                None
            };

            let card_db = self.card_db();
            let player = gs.resolve_target_player_mut(&target);

            let mut filter = crate::ability::util::CardFilter::default();
            filter.card_type = card_type_filter.as_deref();
            filter.group = group_filter.as_deref();
            filter.cost_limit = cost_limit;
            filter.cost_operator = cost_limit_operator.as_deref();
            filter.characters = characters;
            filter.exclude_self = exclude_self_id;
            let filter = filter.original_blade_limit(blade_limit, blade_limit_operator);
            let mut candidates: Vec<(usize, i16)> = Vec::new();

            // If we have selected cards from a previous choice, use them
            if !self.selected_cards.is_empty() {
                for &card_id in &self.selected_cards {
                    if let Some(pos) = player.stage.stage.iter().position(|&id| id == card_id) {
                        candidates.push((pos, card_id));
                    }
                }
            } else {
                // Collect all potential candidates (filter by card_type, group, etc.)
                // in a first pass, then filter by orientation in a second pass
                // to avoid borrow conflicts with gs.mods.
                let stage_snapshot: Vec<(usize, i16)> = player
                    .stage
                    .stage
                    .iter()
                    .copied()
                    .enumerate()
                    .filter(|(_, id)| *id != -1 && filter.matches(&card_db, *id, false))
                    .collect();
                let _ = card_db;
                let _ = player;
                for (i, card_id) in &stage_snapshot {
                    // State-based candidate filtering:
                    //   wait→active (state_change=="active"): only "wait" members.
                    //   active→wait with state=="active" filter: only active members.
                    //   Everything else: accept any member.
                    let matches_state = if state_change == "active" {
                        let ori = gs.mods.get_orientation_modifier(*card_id);
                        ori.is_some_and(|o| o == "wait")
                    } else if effect.state.as_deref() == Some("active") {
                        // e.g. "アクティブ状態のメンバーをウェイトにする"
                        // Only members currently in active state (no wait modifier).
                        let ori = gs.mods.get_orientation_modifier(*card_id);
                        ori.is_none_or(|o| o != "wait")
                    } else if state_change == "wait" {
                        // For "wait" state change, exclude cards already in wait state
                        // so previously waited targets don't remain selectable.
                        let ori = gs.mods.get_orientation_modifier(*card_id);
                        ori.is_none_or(|o| o != "wait")
                    } else {
                        true
                    };
                    if matches_state {
                        candidates.push((*i, *card_id));
                    }
                }
            }

            if candidates.is_empty() {
                let has_energy_in_text =
                    effect.text.contains("エネルギー") || effect.text.contains("energy");
                if has_energy_in_text {
                    if let Err(e) = self.execute_energy_state_change(
                        gs,
                        effect,
                        &state_change,
                        &target,
                        count,
                        max,
                        Some("energy_card"),
                        None,
                    ) {
                        log::debug!("Failed to change energy state: {}", e);
                    }
                }
                return Ok(());
            }

            // count=0 means "change all matching" (no limit)
            let is_change_all = count == 0;

            // Prompt when: there are candidates to choose from AND we haven't already selected,
            // AND either max allows subset selection or more candidates than count need narrowing.
            // Exception: if the effect targets "this member" (activating card is among candidates)
            // and it's a single-target self-member effect, auto-select instead of prompting.
            let is_self_target = count == 1
                && target.as_str() != "opponent"
                && card_type_filter.as_deref() == Some("member_card")
                && gs
                    .activating_card
                    .is_some_and(|act_id| candidates.iter().any(|(_, cid)| *cid == act_id));
            let needs_prompt = !is_self_target
                && self.selected_cards.is_empty()
                && ((max && !candidates.is_empty())
                    || (!is_change_all && candidates.len() > count as usize));

            if needs_prompt {
                let allow_skip = max;
                let pick_count = if max { count as usize } else { count as usize };
                let desc = if max {
                    format!("Select up to {} member(s) to change state", count)
                } else {
                    format!("Select {} member(s) to change state", count)
                };
                // Map candidate positions to stage indices for filtered_indices
                let candidate_positions: Vec<usize> =
                    candidates.iter().map(|(pos, _)| *pos).collect();
                self.pending_choice = Some(
                    Choice::select_cards(Zone::Stage.to_str(), pick_count, desc, allow_skip)
                        .card_type(card_type_filter.clone())
                        .cost_limit(cost_limit, cost_limit_operator.clone())
                        .group(group_filter.clone())
                        .characters(characters.cloned())
                        .filtered_indices(Some(candidate_positions))
                        .is_select_action(true)
                        .target_player_id(Some(target.clone()))
                        .build(),
                );
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                // Store a re-apply effect so finalize_choice applies the state
                // change to the selected target after the choice is resolved.
                gs.ability_queue
                    .set_pending_actions(vec![crate::card::AbilityEffect {
                        action: "change_state".to_string(),
                        state_change: Some(state_change.clone()),
                        // Preserve the state filter (e.g. "active" for
                        // "アクティブ状態のメンバーをウェイトにする") so the
                        // re-run after the player's choice keeps the same
                        // candidate filter and doesn't accept already-waited members.
                        state: effect.state.clone(),
                        target: Some(target.clone()),
                        count: Some(count),
                        card_type: card_type_filter.clone(),
                        cost_limit,
                        group_names: group_filter.clone().map(|g| vec![g]),
                        self_cost: Some(self_cost),
                        cost_limit_operator,
                        ..Default::default()
                    }]);
                return Ok(());
            }

            let change_count = if is_change_all {
                candidates.len()
            } else {
                count.min(candidates.len() as u32) as usize
            };

            let actual_targets: Vec<_> = if is_self_target {
                if let Some(act_id) = gs.activating_card {
                    candidates
                        .iter()
                        .filter(|(_, cid)| *cid == act_id)
                        .take(change_count)
                        .collect()
                } else {
                    candidates.iter().take(change_count).collect()
                }
            } else {
                candidates.iter().take(change_count).collect()
            };

            log::debug!(
                "[EXEC_CHANGE_STATE] targets={:?} state_change={}",
                actual_targets
                    .iter()
                    .map(|(_, cid)| cid)
                    .collect::<Vec<_>>(),
                state_change
            );

            // Snapshot orientations BEFORE applying any changes (for active→wait
            // and wait→active transition detection).
            let snapshots: std::collections::HashMap<i16, Option<String>> = actual_targets
                .iter()
                .map(|(_, card_id)| {
                    (
                        *card_id,
                        gs.mods.get_orientation_modifier(*card_id).cloned(),
                    )
                })
                .collect();
            gs.state_snapshot_before_change = Some(snapshots);

            // Count how many are in wait state before changing (for wait→active tracking)
            let wait_before_count = actual_targets
                .iter()
                .filter(|(_, card_id)| {
                    let o = gs.mods.get_orientation_modifier(*card_id);
                    // None = active (no modifier), Some("wait") = wait
                    o.is_some_and(|o| o == "wait")
                })
                .count();

            for (_, card_id) in &actual_targets {
                if is_cannot_activate_by_effect {
                    log::debug!(
                        "[EXEC_CHANGE_STATE] blocked by cannot_activate_by_effect: card_id={}",
                        card_id
                    );
                    continue;
                }
                log::debug!(
                    "[EXEC_CHANGE_STATE] applying: card_id={} state={} before_ori={:?}",
                    card_id,
                    state_change,
                    gs.mods.get_orientation_modifier(*card_id)
                );
                gs.mods.add_orientation_modifier(*card_id, &state_change);
                log::debug!(
                    "[EXEC_CHANGE_STATE] after: card_id={} ori={:?}",
                    card_id,
                    gs.mods.get_orientation_modifier(*card_id)
                );
            }

            // Push changed cards to selected_cards so subsequent sequential
            // actions (e.g. gain_resource with target_from_selection: true)
            // can target the affected member(s).
            for (_, card_id) in &actual_targets {
                eprintln!(
                    "[EXEC_CHANGE_STATE] pushing card_id={} to selected_cards (len={})",
                    card_id,
                    self.selected_cards.len()
                );
                if !self.selected_cards.contains(card_id) {
                    self.selected_cards.push(*card_id);
                }
            }

            // Track how many members were actually changed from wait→active
            // (activations blocked by cannot_activate_by_effect don't count)
            if state_change == "active" {
                let actual_count = if is_cannot_activate_by_effect {
                    0
                } else {
                    wait_before_count as u32
                };
                gs.last_state_change_wait_to_active_count = actual_count;
            }

            // Compare snapshot with current state to detect actual transitions.
            if let Some(before) = gs.state_snapshot_before_change.take() {
                for (card_id, before_ori) in &before {
                    let after_ori = gs.mods.get_orientation_modifier(*card_id).cloned();
                    if before_ori != &after_ori {
                        let from_str = before_ori.as_deref().unwrap_or("active").to_string();
                        let to_str = after_ori.as_deref().unwrap_or("active").to_string();
                        gs.recently_state_changed.push((
                            *card_id,
                            from_str.clone(),
                            to_str.clone(),
                        ));
                        log::debug!(
                            "[STATE_CHANGE] detected: card={} {}→{}",
                            card_id,
                            from_str,
                            to_str
                        );
                    }
                }
            }

            // Re-trigger auto abilities for both players — a member's state
            // change may satisfy state_change_condition on an auto ability
            // (e.g. "when opponent cost ≤4 member is waited → draw").
            log::debug!(
                "[STATE_CHANGE] modifier applied, re-triggering auto abilities (state={})",
                state_change
            );
            let p1 = gs.player1.id.clone();
            let p2 = gs.player2.id.clone();
            gs.trigger_auto_abilities_for_player(&p1);
            gs.trigger_auto_abilities_for_player(&p2);

            let has_energy_in_text =
                effect.text.contains("エネルギー") || effect.text.contains("energy");
            if has_energy_in_text {
                if let Err(e) = self.execute_energy_state_change(
                    gs,
                    effect,
                    &state_change,
                    &target,
                    count,
                    max,
                    Some("energy_card"),
                    None,
                ) {
                    log::debug!("Failed to change energy state: {}", e);
                }
            }

            return Ok(());
        }

        // Energy card state change (original behavior) — delegated
        self.execute_energy_state_change(
            gs,
            effect,
            &state_change,
            &target,
            count,
            max,
            card_type_filter.as_deref(),
            group_filter.as_deref(),
        )
    }

    /// Place energy from deck to energy zone with specific state (wait/active).
    pub(crate) fn execute_energy_placement(
        &mut self,
        gs: &mut GameState,
        state_change: &str,
        target: &str,
        count: u32,
    ) {
        let cause_cid = gs.activating_card;
        let mut placed_energy: Vec<i16> = Vec::new();
        let player_id = {
            let player = gs.resolve_target_player_mut(target);
            for _ in 0..count {
                if let Some(energy_id) = player.energy_deck.draw() {
                    player.energy_zone.cards.push(energy_id);
                    if state_change == "active" {
                        player.energy_zone.add_active(1);
                    }
                    placed_energy.push(energy_id);
                }
            }
            player.id.clone()
        };
        for &eid in &placed_energy {
            gs.push_movement_event(eid, "energy_deck", "energy", cause_cid, &player_id, true);
        }
    }

    /// Change the state of energy zone cards (wait/active).
    pub(crate) fn execute_energy_state_change(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        state_change: &str,
        target: &str,
        count: u32,
        max: bool,
        card_type_filter: Option<&str>,
        group_filter: Option<&str>,
    ) -> Result<(), String> {
        let card_db = self.card_db();
        let exclude_self_id = if effect.exclude_self.unwrap_or(false) {
            gs.activating_card
        } else {
            None
        };
        let (wait_cards, deactivate_count) = {
            let player = gs.resolve_target_player_mut(target);

            let mut filter = effect.filter_subset();
            if card_type_filter == Some("energy_card") {
                filter.group = None;
                filter.characters = None;
                filter.cost_limit = None;
                filter.cost_operator = None;
            } else {
                filter.group = group_filter;
            }
            filter.card_type = card_type_filter;
            filter.exclude_self = exclude_self_id;
            let valid_indices =
                util::matching_indices(&player.energy_zone.cards, &card_db, &filter, false);

            let effective_count = if max {
                let available = match state_change {
                    "active" | "アクティブ" => player
                        .energy_zone
                        .cards
                        .len()
                        .saturating_sub(player.energy_zone.active_count()),
                    _ => player.energy_zone.active_count(),
                };
                let capped = (count as usize).min(available) as u32;
                log::debug!(
                    "[ENERGY] max=true: count={} available={} effective={}",
                    count,
                    available,
                    capped
                );
                capped
            } else if count == 0 {
                let val = match state_change {
                    "active" | "アクティブ" => player
                        .energy_zone
                        .cards
                        .len()
                        .saturating_sub(player.energy_zone.active_count()),
                    _ => player.energy_zone.active_count(),
                };
                log::debug!("[ENERGY] count=0 (all): effective={}", val);
                val as u32
            } else {
                log::debug!("[ENERGY] max=false: count={} effectve={}", count, count);
                count
            };

            if valid_indices.len() < effective_count as usize {
                return Err(format!(
                    "Not enough energy cards to deactivate: need {}, have {}",
                    effective_count,
                    valid_indices.len()
                ));
            }

            if !max
                && valid_indices.len() > effective_count as usize
                && state_change != "active"
                && state_change != "アクティブ"
            {
                self.pending_choice = Some(
                    Choice::select_cards(
                        Zone::Energy.to_str(),
                        effective_count as usize,
                        format!(
                            "Select {} energy card(s) to deactivate (set to wait)",
                            effective_count
                        ),
                        false,
                    )
                    .card_type(card_type_filter.map(|s| s.to_string()))
                    .group(group_filter.map(|s| s.to_string()))
                    .target_player_id(Some(target.to_string()))
                    .build(),
                );
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                return Ok(());
            }

            let wait_cards: Vec<i16> = valid_indices
                .iter()
                .take(effective_count as usize)
                .filter_map(|i| {
                    if *i < player.energy_zone.cards.len() {
                        Some(player.energy_zone.cards[*i])
                    } else {
                        None
                    }
                })
                .collect();

            (wait_cards, effective_count)
        };

        let active_cards: Vec<i16> = if state_change == "active" || state_change == "アクティブ"
        {
            let player = gs.resolve_target_player(target);
            let mut result = Vec::new();
            let mut active_count = 0u32;
            let group_filter_for_active = if card_type_filter == Some("energy_card") {
                None
            } else {
                group_filter
            };
            for i in 0..player.energy_zone.cards.len() {
                if active_count >= deactivate_count {
                    break;
                }
                if let Some(&card_id) = player.energy_zone.cards.get(i) {
                    let matches_type = card_type_filter
                        .is_none_or(|ct| util::card_matches_type(&card_db, card_id, Some(ct)));
                    let matches_grp = group_filter_for_active
                        .is_none_or(|gf| util::card_matches_group_str(&card_db, card_id, Some(gf)));
                    if matches_type && matches_grp {
                        result.push(card_id);
                        active_count += 1;
                    }
                }
            }
            result
        } else {
            vec![]
        };

        match state_change {
            "wait" | "ウェイト" => {
                for card_id in &wait_cards {
                    gs.mods.add_orientation_modifier(*card_id, "wait");
                }
                for _ in 0..deactivate_count {
                    let player = gs.resolve_target_player_mut(target);
                    player.energy_zone.sub_active(1);
                }
            }
            "active" | "アクティブ" => {
                for card_id in &active_cards {
                    gs.mods.add_orientation_modifier(*card_id, "active");
                }
                let player = gs.resolve_target_player_mut(target);
                player.energy_zone.add_active(active_cards.len());
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn execute_set_cost(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        value: u32,
    ) {
        let target = effect.target_name();
        let card_type = effect.card_type.as_deref();
        let player = gs.resolve_target_player_mut(target);
        let mut card_ids: Vec<i16> = if let Some("live_card") = card_type {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type {
            player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect()
        } else {
            player.hand.cards.iter().copied().collect()
        };
        if effect.group_names.is_some()
            || effect.exclude_group_names.is_some()
            || effect.characters.is_some()
            || effect.exclude_characters.is_some()
        {
            let filter = effect.filter_subset();
            card_ids = util::matching_ids_filtered(
                &card_ids,
                &gs.card_database,
                &filter,
                true,
                None,
                None,
                None,
            );
        }
        for card_id in card_ids {
            gs.mods.set_cost_modifier(card_id, value as i32);
        }
    }

    pub(crate) fn execute_set_blade_type(&mut self, gs: &mut GameState, effect: &AbilityEffect) {
        let blade_type = effect.blade_type.as_deref();
        let target = effect.target_name();
        let duration = effect.duration.as_deref();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let bt_str = blade_type.unwrap_or("none");
        gs.rule_log.push(format!(
            "{} {}: ブレード種類を{}に設定",
            pp, act_name, bt_str
        ));
        let card_db = self.card_db();
        let blade_color = blade_type.and_then(|bt| match bt {
            "red" | "赤ブレード" => Some(crate::card::BladeColor::Red),
            "blue" | "青ブレード" => Some(crate::card::BladeColor::Blue),
            "green" | "緑ブレード" => Some(crate::card::BladeColor::Green),
            "yellow" | "黄ブレード" => Some(crate::card::BladeColor::Yellow),
            "purple" | "紫ブレード" => Some(crate::card::BladeColor::Purple),
            _ => {
                log::debug!("[set_blade_type] Unknown blade type: {:?}", bt);
                None
            }
        });
        let mut stage_card_ids: Vec<(i16, String)> = {
            let player = gs.resolve_target_player(target);
            (0..3)
                .filter_map(|i| {
                    let id = player.stage.stage[i];
                    if id == -1 {
                        None
                    } else {
                        Some((id, player.id.clone()))
                    }
                })
                .collect()
        };
        if effect.group_names.is_some()
            || effect.exclude_group_names.is_some()
            || effect.characters.is_some()
            || effect.exclude_characters.is_some()
        {
            let filter = effect.filter_subset();
            let ids: Vec<i16> = stage_card_ids.iter().map(|(id, _)| *id).collect();
            let filtered = util::matching_ids_filtered(
                &ids,
                &gs.card_database,
                &filter,
                true,
                None,
                None,
                None,
            );
            stage_card_ids.retain(|(id, _)| filtered.contains(id));
        }
        for (card_id, pid) in stage_card_ids {
            if let Some(color) = blade_color {
                gs.mods.set_blade_type_modifier(card_id, color);
            }
            let ed = serde_json::json!({"card_id": card_id});
            util::push_temporary_effect(
                gs,
                &format!("set_blade_type:{}", blade_type.unwrap_or("")),
                duration,
                &pid,
                &format!(
                    "Set blade type to {} for {}",
                    blade_type.unwrap_or(""),
                    card_db
                        .get_card(card_id)
                        .map(|c| c.name.as_str())
                        .unwrap_or("unknown")
                ),
                Some(ed),
            );
        }
    }

    pub(crate) fn execute_set_heart_type(
        &mut self,
        gs: &mut GameState,
        heart_type: Option<&str>,
        _target: &str,
        _count: i32,
        duration: Option<&str>,
    ) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        // "selected" means the heart type was chosen by a preceding select action
        // in a Sequential effect; look up the choice from the queue entry.
        let resolved_heart_type = match heart_type {
            Some("selected") => gs
                .ability_queue
                .current_entry()
                .and_then(|e| e.conditional_choice.as_deref()),
            other => other,
        };
        let ht = resolved_heart_type.unwrap_or("heart00").to_string();
        gs.rule_log
            .push(format!("{} {}: ハート種類を{}に設定", pp, act_name, ht));
        // Use selected_target from self.selected_cards if available (member-targeting
        // abilities like PL!HS-bp5-021-L), otherwise fall back to activating_card
        // (self-targeting abilities like Kanan PL!S-pb1-003-R).
        let card_id = self
            .selected_cards
            .first()
            .copied()
            .or(gs.activating_card)
            .unwrap_or(-1);
        if card_id == -1 {
            return;
        }
        let color = crate::zones::parse_heart_color(&ht);
        gs.mods.heart_color_multiplier.insert(card_id, color);
        gs.record_ability_application(
            card_id,
            format!("Transform hearts to {}", ht),
            "transform",
            card_id,
            Some(color.index()),
            0,
        );
        let ed = serde_json::json!({"card_id": card_id});
        util::push_temporary_effect(
            gs,
            "set_heart_type",
            duration,
            "self",
            &format!("Set heart type to {} for card {}", ht, card_id),
            Some(ed),
        );
    }

    pub(crate) fn execute_activation_cost(
        &mut self,
        gs: &mut GameState,
        operation: &str,
        value: u32,
        target: &str,
        duration: Option<&str>,
    ) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log.push(format!(
            "{} {}: 起動コスト{} {} (target={})",
            pp, act_name, operation, value, target
        ));
        let prohibition_text = format!("activation_cost_{}_{}", operation, value);
        match target {
            "self" | "opponent" => {
                gs.prohibition_effects.push(prohibition_text);
            }
            _ => {}
        }
        util::push_temporary_effect(
            gs,
            &format!("activation_cost_{}_{}", operation, value),
            duration,
            target,
            &format!("Modify activation cost by {} {}", operation, value),
            None,
        );
    }

    pub(crate) fn execute_set_card_identity(&mut self, gs: &mut GameState, identities: &[String]) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log.push(format!(
            "{} {}: カード同一性変更 identities={:?}",
            pp, act_name, identities
        ));
        if !identities.is_empty() {
            gs.prohibition_effects
                .push(format!("card_identity:{}", identities.join(",")));
        }
    }

    pub(crate) fn execute_reduce_live_card_set_limit(&mut self, gs: &mut GameState, count: u32) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log.push(format!(
            "{} {}: ライブカードセット上限を{}減らす",
            pp, act_name, count
        ));
        let player = gs.resolve_target_player_mut("self");
        player.live_card_set_limit_reduction += count;
    }

    pub(crate) fn execute_set_blade_count(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        value: u32,
    ) {
        let target = effect.target_name();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: ブレード数を{}に設定", pp, act_name, value));
        let mut stage_cards: Vec<i16> = {
            let player = gs.resolve_target_player_mut(target);
            player.stage.stage.to_vec()
        };
        stage_cards.retain(|&id| id != -1);
        if effect.group_names.is_some()
            || effect.exclude_group_names.is_some()
            || effect.characters.is_some()
            || effect.exclude_characters.is_some()
        {
            let filter = effect.filter_subset();
            stage_cards = util::matching_ids_filtered(
                &stage_cards,
                &gs.card_database,
                &filter,
                true,
                None,
                None,
                None,
            );
        }
        if let Some(ref pos) = effect.position {
            if let Some(p) = pos.get_position() {
                if let Some(stage_idx) = util::stage_position_index(p) {
                    let player = gs.resolve_target_player(target);
                    let expected = player.stage.stage[stage_idx];
                    stage_cards.retain(|&cid| expected == -1 || cid == expected);
                }
            }
        }
        let card_db = gs.card_database.clone();
        for &card_id in &stage_cards {
            let original_blade = card_db
                .get_card(card_id)
                .map(|c| c.blade as i32)
                .unwrap_or(0);
            let set_value = (value as i32) - original_blade;
            gs.mods.set_blade_modifier(card_id, set_value);
            // Register for cleanup at live end / duration expiry
            if effect.duration.is_some() {
                let mut data = serde_json::Map::new();
                data.insert(
                    "card_id".to_string(),
                    serde_json::Value::Number(card_id.into()),
                );
                util::push_temporary_effect(
                    gs,
                    "set_blade_count",
                    effect.duration.as_deref(),
                    target,
                    &format!("set blade count to {} for card {}", value, card_id),
                    Some(serde_json::Value::Object(data)),
                );
            }
        }
    }

    pub(crate) fn execute_specify_heart_color(
        &mut self,
        _gs: &mut GameState,
        choice: bool,
        _target: &str,
    ) {
        log::debug!("specify_heart_color: choice={}", choice);
        if choice {
            // Q190 (2025.11.17): ALL heart (heart00) cannot be selected.
            // Present the 6 individual heart colors for the player to choose.
            self.pending_choice = Some(Choice::SelectHeartColor {
                count: 1,
                options: vec![
                    "heart01".into(),
                    "heart02".into(),
                    "heart03".into(),
                    "heart04".into(),
                    "heart05".into(),
                    "heart06".into(),
                ],
                description: "Choose a heart color".to_string(),
                description_en: None,
                description_ja: None,
            });
        }
    }

    pub(crate) fn execute_set_card_identity_all_regions(
        &mut self,
        gs: &mut GameState,
        identities: Option<&Vec<String>>,
        target: &str,
    ) {
        let _target = target;
        let card_id = self.activating_card_id.or(gs.activating_card);
        if let Some(card_id) = card_id {
            if let Some(identities) = identities {
                for identity in identities {
                    gs.prohibition_effects
                        .push(format!("card_identity:{}:{}", card_id, identity));
                }
            }
        }
    }

    pub(crate) fn execute_set_cost_to_use(
        &mut self,
        gs: &mut GameState,
        value: u32,
    ) -> Result<(), String> {
        let card_id = self.activating_card_id.or(gs.activating_card);
        if let Some(card_id) = card_id {
            gs.mods.set_cost_modifier(card_id, value as i32);
        }
        Ok(())
    }

    pub(crate) fn execute_all_blade_timing(
        &mut self,
        gs: &mut GameState,
        timing: &str,
        treat_as: &str,
    ) {
        let card_id = self.activating_card_id.or(gs.activating_card);
        if let Some(card_id) = card_id {
            gs.prohibition_effects.push(format!(
                "all_blade_timing:{}:{}:{}",
                card_id, timing, treat_as
            ));
        }
    }

    pub(crate) fn execute_modify_cost(
        &mut self,
        gs: &mut GameState,
        effect: &AbilityEffect,
        value: u32,
    ) {
        let operation = effect.operation.as_deref().unwrap_or("add");
        let target = effect.target_name();
        let card_type = effect.card_type.as_deref();
        let duration = effect.duration.as_deref();
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log.push(format!(
            "{} {}: コスト{} {}",
            pp, act_name, operation, value
        ));
        let player = gs.resolve_target_player_mut(target);
        let mut card_ids: Vec<i16> = if let Some("live_card") = card_type {
            player.live_card_zone.cards.iter().copied().collect()
        } else if let Some("member_card") = card_type {
            player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect()
        } else if let Some("energy_card") = card_type {
            player.energy_zone.cards.iter().copied().collect()
        } else {
            player.hand.cards.iter().copied().collect()
        };
        // Filter by group_names etc. using the effect's CardFilter
        if effect.group_names.is_some()
            || effect.exclude_group_names.is_some()
            || effect.characters.is_some()
            || effect.exclude_characters.is_some()
        {
            let filter = effect.filter_subset();
            card_ids = util::matching_ids_filtered(
                &card_ids,
                &gs.card_database,
                &filter,
                true,
                None,
                None,
                None,
            );
        }
        // When self_target is set, only the activating card receives the modifier
        // (e.g. "このメンバーのコストを+Nする" — only this member, not all matching).
        if effect.self_target.unwrap_or(false) {
            if let Some(cid) = gs.activating_card {
                card_ids.retain(|&id| id == cid);
            }
        }
        let delta = match operation {
            "add" => value as i32,
            "subtract" => -(value as i32),
            "set" => value as i32,
            _ => {
                log::debug!("Unknown operation: {}", operation);
                return;
            }
        };
        for card_id in &card_ids {
            if operation == "set" {
                gs.mods.set_cost_modifier(*card_id, delta);
            } else {
                gs.mods.add_cost_modifier(*card_id, delta);
            }
        }
        if let Some(dur) = duration {
            if dur != "permanent" {
                let target_str = target.to_string();
                let data: Vec<serde_json::Value> = card_ids
                    .iter()
                    .map(|&cid| serde_json::json!({"card_id": cid, "amount": delta.abs()}))
                    .collect();
                util::push_temporary_effect(
                    gs,
                    "modify_cost",
                    Some(dur),
                    &target_str,
                    &format!("Cost {} {} ({})", operation, value, dur),
                    Some(serde_json::Value::Array(data)),
                );
            }
        }
    }
}
