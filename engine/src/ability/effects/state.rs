use super::super::resolver::AbilityResolver;
use super::super::types::{Choice, ExecutionContext};
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
        let group_filter = group_name.map(|s| s.to_string());

        if optional {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "pay_optional_cost:skip_optional_cost".to_string(),
                description: format!("Change state to {} (pay optional cost)?", state_change),
                allow_skip: optional,
                options: None,
            });
            if let Some(entry) = gs.ability_queue.current_entry_mut() {
                entry.choice_card_no = Some("change_state".to_string());
            }
            return Ok(());
        }

        // Draw from energy deck and place in energy zone with state (e.g. wait)
        if source == Some("deck") && destination == Some("energy_zone") {
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
            let card_db = self.card_db();
            let player = gs.resolve_target_player_mut(&target);

            let filter = util::filter_from_parts(
                card_type_filter.as_deref(),
                group_filter.as_deref(),
                cost_limit,
                cost_limit_operator.as_deref(),
                characters,
                None, // exclude_characters
                None, // exclude_self
            )
            .original_blade_limit(blade_limit, blade_limit_operator);
            let mut candidates: Vec<(usize, i16)> = Vec::new();

            // If we have selected cards from a previous choice, use them
            if !self.selected_cards.is_empty() {
                for &card_id in &self.selected_cards {
                    if let Some(pos) = player.stage.stage.iter().position(|&id| id == card_id) {
                        candidates.push((pos, card_id));
                    }
                }
            } else {
                for (i, slot_id) in player.stage.stage.iter().enumerate() {
                    if *slot_id == -1 {
                        continue;
                    }
                    if filter.matches(&card_db, *slot_id, false) {
                        candidates.push((i, *slot_id));
                    }
                }
            }

            if candidates.is_empty() {
                return Err("No matching members on stage to change state".to_string());
            }

            // count=0 means "change all matching" (no limit)
            let is_change_all = count == 0;

            // Prompt when: there are candidates to choose from AND we haven't already selected,
            // AND either max allows subset selection or more candidates than count need narrowing.
            let needs_prompt = self.selected_cards.is_empty()
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
                self.pending_choice = Some(
                    Choice::select_cards("stage", pick_count, desc, allow_skip)
                        .card_type(card_type_filter.clone())
                        .cost_limit(cost_limit, cost_limit_operator.clone())
                        .group(group_filter.clone())
                        .characters(characters.cloned())
                        .is_select_action(true)
                        .target_player_id(Some(target.clone()))
                        .build(),
                );
                self.execution_context = ExecutionContext::SingleEffect { effect_index: 0 };
                // Store a re-apply effect so finalize_choice applies the state
                // change to the selected target after the choice is resolved.
                gs.ability_queue.set_pending_commands(vec![
                    crate::ability::types::Command::Effect(crate::card::AbilityEffect {
                        action: "change_state".to_string(),
                        state_change: Some(state_change.clone()),
                        target: Some(target.clone()),
                        count: Some(count),
                        card_type: card_type_filter.clone(),
                        cost_limit,
                        group_names: group_filter.clone().map(|g| vec![g]),
                        self_cost: Some(self_cost),
                        cost_limit_operator,
                        ..Default::default()
                    }),
                ]);
                return Ok(());
            }

            let change_count = if is_change_all {
                candidates.len()
            } else {
                count.min(candidates.len() as u32) as usize
            };

            let actual_targets = candidates.iter().take(change_count).collect::<Vec<_>>();

            eprintln!(
                "[EXEC_CHANGE_STATE] targets={:?} state_change={}",
                actual_targets
                    .iter()
                    .map(|(_, cid)| cid)
                    .collect::<Vec<_>>(),
                state_change
            );

            // Count how many are in wait state before changing (for wait→active tracking)
            let wait_before_count = actual_targets
                .iter()
                .filter(|(_, card_id)| {
                    let o = gs.mods.get_orientation_modifier(*card_id);
                    // None = active (no modifier), Some("wait") = wait
                    o.map_or(false, |o| o == "wait")
                })
                .count();

            for (_, card_id) in &actual_targets {
                eprintln!(
                    "[EXEC_CHANGE_STATE] applying: card_id={} state={} before_ori={:?}",
                    card_id,
                    state_change,
                    gs.mods.get_orientation_modifier(*card_id)
                );
                gs.mods.add_orientation_modifier(*card_id, &state_change);
                eprintln!(
                    "[EXEC_CHANGE_STATE] after: card_id={} ori={:?}",
                    card_id,
                    gs.mods.get_orientation_modifier(*card_id)
                );
            }

            // Track how many members were changed from wait→active
            if state_change == "active" {
                gs.last_state_change_wait_to_active_count = wait_before_count as u32;
            }

            // Re-trigger auto abilities for both players — a member's state
            // change may satisfy state_change_condition on an auto ability
            // (e.g. "when opponent cost ≤4 member is waited → draw").
            eprintln!(
                "[STATE_CHANGE] modifier applied, re-triggering auto abilities (state={})",
                state_change
            );
            let p1 = gs.player1.id.clone();
            let p2 = gs.player2.id.clone();
            gs.trigger_auto_abilities_for_player(&p1);
            gs.trigger_auto_abilities_for_player(&p2);

            return Ok(());
        }

        // Energy card state change (original behavior) — delegated
        return self.execute_energy_state_change(
            gs,
            effect,
            &state_change,
            &target,
            count,
            max,
            card_type_filter.as_deref(),
            group_filter.as_deref(),
        );
    }

    /// Place energy from deck to energy zone with specific state (wait/active).
    pub(crate) fn execute_energy_placement(
        &mut self,
        gs: &mut GameState,
        state_change: &str,
        target: &str,
        count: u32,
    ) {
        let player = gs.resolve_target_player_mut(target);
        for _ in 0..count {
            if let Some(energy_id) = player.energy_deck.draw() {
                player.energy_zone.cards.push(energy_id);
                if state_change == "active" {
                    player.energy_zone.active_energy_count += 1;
                }
            }
        }
        // Track that energy was placed by a card effect (not energy phase draw)
        if count > 0 {
            gs.last_energy_placed_by_effect = true;
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
        let exclude_self_id = effect.exclude_self.and_then(|_| {
            let gs_ref = &*gs;
            gs_ref.activating_card
        });
        let (wait_cards, deactivate_count) = {
            let player = gs.resolve_target_player_mut(target);

            let filter = util::filter_from_parts(
                card_type_filter,
                group_filter,
                effect.cost_limit,
                effect.cost_limit_operator.as_deref(),
                effect.characters.as_ref(),
                effect.exclude_characters.as_ref(),
                exclude_self_id,
            );
            let valid_indices =
                util::matching_indices(&player.energy_zone.cards, &card_db, &filter, false);

            let effective_count = if max {
                let available = match state_change {
                    "active" | "アクティブ" => player
                        .energy_zone
                        .cards
                        .len()
                        .saturating_sub(player.energy_zone.active_energy_count),
                    _ => player.energy_zone.active_energy_count,
                };
                let capped = (count as usize).min(available) as u32;
                eprintln!(
                    "[ENERGY] max=true: count={} available={} effective={}",
                    count, available, capped
                );
                capped
            } else {
                eprintln!("[ENERGY] max=false: count={} effectve={}", count, count);
                count
            };

            if valid_indices.len() < effective_count as usize {
                return Err(format!(
                    "Not enough energy cards to deactivate: need {}, have {}",
                    effective_count,
                    valid_indices.len()
                ));
            }

            if !max && valid_indices.len() > effective_count as usize {
                if state_change != "active" && state_change != "アクティブ" {
                    self.pending_choice = Some(
                        Choice::select_cards(
                            "energy_zone",
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
            for i in 0..player.energy_zone.cards.len() {
                if active_count >= deactivate_count {
                    break;
                }
                if let Some(&card_id) = player.energy_zone.cards.get(i) {
                    let matches_type = card_type_filter.map_or(true, |ct| {
                        util::card_matches_type(&card_db, card_id, Some(ct))
                    });
                    let matches_grp = group_filter.map_or(true, |gf| {
                        util::card_matches_group_str(&card_db, card_id, Some(gf))
                    });
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
                    player.energy_zone.active_energy_count =
                        player.energy_zone.active_energy_count.saturating_sub(1);
                }
            }
            "active" | "アクティブ" => {
                for card_id in &active_cards {
                    gs.mods.add_orientation_modifier(*card_id, "active");
                }
                let player = gs.resolve_target_player_mut(target);
                player.energy_zone.active_energy_count += active_cards.len();
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn execute_set_cost(
        &mut self,
        gs: &mut GameState,
        value: u32,
        target: &str,
        card_type: Option<&str>,
    ) {
        let player = gs.resolve_target_player_mut(target);
        let card_ids: Vec<i16> = if let Some("live_card") = card_type {
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
        for card_id in card_ids {
            gs.mods.set_cost_modifier(card_id, value as i32);
        }
    }

    pub(crate) fn execute_set_blade_type(
        &mut self,
        gs: &mut GameState,
        blade_type: Option<&str>,
        target: &str,
        duration: Option<&str>,
    ) {
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
                eprintln!("[set_blade_type] Unknown blade type: {:?}", bt);
                None
            }
        });
        let stage_card_ids: Vec<(i16, String)> = {
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
        for (card_id, pid) in stage_card_ids {
            if let Some(color) = blade_color {
                gs.mods.set_blade_type_modifier(card_id, color);
            }
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
                None,
            );
        }
    }

    pub(crate) fn execute_set_heart_type(
        &mut self,
        gs: &mut GameState,
        heart_type: Option<&str>,
        target: &str,
        _count: i32,
    ) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let ht = heart_type.unwrap_or("heart00");
        gs.rule_log
            .push(format!("{} {}: ハート種類を{}に設定", pp, act_name, ht));
        let heart_type = heart_type.unwrap_or("heart00");
        let card_ids: Vec<i16> = {
            let player = gs.resolve_target_player_mut(target);
            player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect()
        };
        let color = crate::zones::parse_heart_color(heart_type);
        for card_id in card_ids {
            gs.mods.heart_color_multiplier.insert(card_id, color);
        }
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

    pub(crate) fn execute_set_blade_count(&mut self, gs: &mut GameState, value: u32, target: &str) {
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        gs.rule_log
            .push(format!("{} {}: ブレード数を{}に設定", pp, act_name, value));
        let stage_cards: Vec<i16> = {
            let player = gs.resolve_target_player_mut(target);
            player.stage.stage.to_vec()
        };
        for &card_id in stage_cards.iter().filter(|&&id| id != -1) {
            let current = gs.mods.get_blade_modifier(card_id);
            let delta = (value as i32) - current;
            gs.mods.add_blade_modifier(card_id, delta);
        }
    }

    pub(crate) fn execute_set_required_hearts(
        &mut self,
        gs: &mut GameState,
        heart_colors: &[String],
        target: &str,
    ) {
        let card_ids: Vec<i16> = {
            let player = gs.resolve_target_player_mut(target);
            player.live_card_zone.cards.to_vec()
        };
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let hc_str = heart_colors.join(",");
        for card_id in &card_ids {
            let cn = self.card_name(*card_id);
            gs.rule_log.push(format!(
                "{} {}: {}の要求ハートを[{}]に設定",
                pp, act_name, cn, hc_str
            ));
        }
        for card_id in card_ids {
            let mut color_counts: std::collections::HashMap<crate::card::HeartColor, u32> =
                std::collections::HashMap::new();
            for color_str in heart_colors {
                let color = crate::zones::parse_heart_color(color_str);
                *color_counts.entry(color).or_insert(0) += 1;
            }
            for (color, count) in &color_counts {
                gs.mods
                    .set_need_heart_modifier(card_id, *color, *count as i32);
            }
        }
    }

    pub(crate) fn execute_set_score(&mut self, gs: &mut GameState, value: u32, target: &str) {
        let activating_id = self.activating_card_id;
        let card_ids: Vec<i16> = {
            let player = gs.resolve_target_player_mut(target);
            player.live_card_zone.cards.iter().copied().collect()
        };
        let pp = self.player_prefix(gs);
        let act_name = gs
            .activating_card
            .map(|c| self.card_name(c))
            .unwrap_or_default();
        let target_names: Vec<String> = card_ids.iter().map(|&cid| self.card_name(cid)).collect();
        gs.rule_log.push(format!(
            "{} {}: {}のスコアを{}に設定 [{}]",
            pp,
            act_name,
            target_names.join(","),
            value,
            target
        ));
        let filter_to_activating = activating_id.is_some() && target == "self";
        for &card_id in &card_ids {
            if filter_to_activating {
                if let Some(aid) = activating_id {
                    if card_id != aid {
                        continue;
                    }
                }
            }
            gs.mods.set_score_modifier(card_id, value as i32);
        }
    }

    pub(crate) fn execute_specify_heart_color(
        &mut self,
        _gs: &mut GameState,
        choice: bool,
        target: &str,
    ) {
        eprintln!("specify_heart_color: choice={}, target={}", choice, target);
        if choice {
            self.pending_choice = Some(Choice::SelectTarget {
                target: "heart_color".to_string(),
                description: "Choose a heart color".to_string(),
                allow_skip: false,
                options: None,
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
        let card_id = self.activating_card_id.or_else(|| gs.activating_card);
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
        let card_id = self.activating_card_id.or_else(|| gs.activating_card);
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
        let card_id = self.activating_card_id.or_else(|| gs.activating_card);
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
        operation: &str,
        value: u32,
        target: &str,
        card_type: Option<&str>,
    ) {
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
        let card_ids: Vec<i16> = if let Some("live_card") = card_type {
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
        let delta = match operation {
            "add" => value as i32,
            "subtract" => -(value as i32),
            "set" => value as i32,
            _ => {
                eprintln!("Unknown operation: {}", operation);
                return;
            }
        };
        for card_id in card_ids {
            if operation == "set" {
                gs.mods.set_cost_modifier(card_id, delta);
            } else {
                gs.mods.add_cost_modifier(card_id, delta);
            }
        }
    }
}
