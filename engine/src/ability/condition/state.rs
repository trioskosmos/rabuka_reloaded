use super::ConditionContext;
use crate::ability::enums::{ConditionType, Zone};
use crate::ability::util;
use crate::ability::util::compare_counts;
use crate::card::Condition;
use crate::game_state::Phase;

impl<'a> ConditionContext<'a> {
    pub(crate) fn no_excess_heart_flag(&self, target: &str) -> bool {
        let player = self.resolve_condition_player(target);
        if player.is_first_attacker {
            self.game_state.self_no_excess_heart_this_turn
        } else {
            self.game_state.opponent_live_no_excess_heart_this_turn
        }
    }

    pub(crate) fn evaluate_temporal_condition(&self, condition: &Condition) -> bool {
        let temporal = condition.temporal.as_deref().unwrap_or("");
        let phase = condition.phase.as_deref();

        match temporal {
            "this_turn" => {
                if let Some(count) = condition.count {
                    if Zone::from_str(condition.location.as_deref().unwrap_or(""))
                        == Some(Zone::Stage)
                        && condition.card_type.as_deref() == Some("member_card")
                    {
                        let target = condition.target.as_deref().unwrap_or("self");
                        let player = self.resolve_condition_player(target);
                        if let Some(ref groups) = condition.group_names {
                            if !groups.is_empty() {
                                let card_db = &self.game_state.card_database;
                                let debut_count_matching = player
                                    .stage
                                    .stage
                                    .iter()
                                    .filter(|&&cid| cid != -1)
                                    .filter(|&&cid| {
                                        groups.iter().any(|g| {
                                            crate::ability::util::card_matches_group_str(
                                                card_db,
                                                cid,
                                                Some(g),
                                            )
                                        })
                                    })
                                    .filter(|&&cid| self.game_state.has_card_moved_this_turn(cid))
                                    .count();
                                return debut_count_matching >= (count as usize);
                            }
                        }
                        return player.debut_count_this_turn >= count;
                    }
                }
                if let Some(created_turn) = condition
                    .temporal_scope
                    .as_ref()
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    created_turn == self.game_state.turn_number
                } else {
                    if let Some(nested_condition) = &condition.condition {
                        match nested_condition.condition_type {
                            Some(ConditionType::NotMoved) => {
                                if let Some(activating_card_id) = self.activating_card_id {
                                    !self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else {
                                    true
                                }
                            }
                            Some(ConditionType::HasMoved) => {
                                let check_card = condition.position.as_ref().and_then(|pos| {
                                    pos.get_position().and_then(|pos_str| {
                                        let target = condition.target.as_deref().unwrap_or("self");
                                        let player = self.resolve_condition_player(target);
                                        util::stage_position_index(pos_str).and_then(|idx| {
                                            if idx < 3 && player.stage.stage[idx] != -1 {
                                                Some(player.stage.stage[idx])
                                            } else {
                                                None
                                            }
                                        })
                                    })
                                });
                                if let Some(card_id) = check_card {
                                    self.game_state.has_card_moved_this_turn(card_id)
                                } else if self.game_state.position_change_occurred_this_turn {
                                    let target = condition.target.as_deref().unwrap_or("self");
                                    let player = self.resolve_condition_player(target);
                                    player.stage.stage.iter().any(|&cid| {
                                        cid != -1 && self.game_state.has_card_moved_this_turn(cid)
                                    })
                                } else if let Some(activating_card_id) = self.activating_card_id {
                                    self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else {
                                    false
                                }
                            }
                            _ => self.evaluate_condition(nested_condition),
                        }
                    } else {
                        true
                    }
                }
            }
            "live_end" => matches!(
                self.game_state.current_phase,
                crate::game_state::Phase::LiveVictoryDetermination
            ),
            "during_live" | "this_live" => {
                if !matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::LiveCardSetFirstAttacker
                        | crate::game_state::Phase::LiveCardSetSecondAttacker
                        | crate::game_state::Phase::FirstAttackerPerformance
                        | crate::game_state::Phase::SecondAttackerPerformance
                        | crate::game_state::Phase::LiveVictoryDetermination
                ) {
                    return false;
                }
                // Build list of zones to check from condition.locations or condition.location
                let zones_to_check: Vec<&str> = {
                    let mut zones = Vec::new();
                    if let Some(ref locs) = condition.locations {
                        for z in locs {
                            zones.push(z.as_str());
                        }
                    } else if let Some(ref loc) = condition.location {
                        zones.push(loc.as_str());
                    } else if condition.heart_colors.is_some() {
                        // If checking heart colors but no zone specified, default to live_card_zone
                        zones.push("live_card_zone");
                    }
                    zones
                };
                let has_success_or_live_zone = zones_to_check
                    .iter()
                    .any(|z| *z == "success_live_card_zone" || *z == "live_card_zone");
                if has_success_or_live_zone {
                    let target = condition.target.as_deref().unwrap_or("self");
                    let player = self.resolve_condition_player(target);

                    // Aggregate total with heart_colors: sum need_heart across all live cards
                    if condition.aggregate.as_deref() == Some("total")
                        && condition
                            .heart_colors
                            .as_ref()
                            .is_some_and(|c| !c.is_empty())
                    {
                        let location = condition.location.as_deref().unwrap_or("live_card_zone");
                        return self
                            .check_aggregate_total(condition, player, location)
                            .unwrap_or(false);
                    }

                    let mut found_match = false;
                    'zone_loop: for zone_name in &zones_to_check {
                        let cards: Vec<i16> = match *zone_name {
                            "success_live_card_zone" => player
                                .success_live_card_zone
                                .cards
                                .iter()
                                .copied()
                                .collect(),
                            "live_card_zone" => player.live_card_zone.cards.to_vec(),
                            _ => continue,
                        };
                        if crate::ability::debug::ABILITY_DEBUG
                            .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            eprintln!(
                                "[TEMP_DIAG] checking zone={} {} cards={:?}",
                                zone_name,
                                cards.len(),
                                cards
                            );
                        }
                        for &cid in &cards {
                            if let Some(card) = self.game_state.card_database.get_card(cid) {
                                let group_ok =
                                    condition.group_names.as_ref().map_or(true, |groups| {
                                        groups.iter().any(|g| {
                                            crate::ability::util::card_matches_group_str(
                                                &self.game_state.card_database,
                                                cid,
                                                Some(g),
                                            )
                                        })
                                    });
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(std::sync::atomic::Ordering::Relaxed)
                                {
                                    eprintln!(
                                        "[TEMP_DIAG]   card={} name={} group_ok={} nh={:?}",
                                        cid, card.name, group_ok, card.need_heart
                                    );
                                }
                                if !group_ok {
                                    continue;
                                }
                                if let Some(ref hc_list) = condition.heart_colors {
                                    if let Some(ref nh) = card.need_heart {
                                        // Check if ALL specified heart colors meet the count threshold
                                        let threshold = condition.count.unwrap_or(1) as u32;
                                        let all_hearts_present = hc_list.iter().all(|color_str| {
                                            let color = crate::card::parse_heart_color(color_str);
                                            nh.hearts.get(&color).copied().unwrap_or(0) >= threshold
                                        });
                                        if crate::ability::debug::ABILITY_DEBUG
                                            .load(std::sync::atomic::Ordering::Relaxed)
                                        {
                                            eprintln!(
                                                "[TEMP_DIAG]   all_hearts_present={}",
                                                all_hearts_present
                                            );
                                        }
                                        if all_hearts_present {
                                            found_match = true;
                                            break 'zone_loop;
                                        }
                                    } else {
                                        if crate::ability::debug::ABILITY_DEBUG
                                            .load(std::sync::atomic::Ordering::Relaxed)
                                        {
                                            eprintln!("[TEMP_DIAG]   no need_heart on card");
                                        }
                                    }
                                } else {
                                    found_match = true;
                                    break 'zone_loop;
                                }
                            }
                        }
                    }
                    if !found_match {
                        if crate::ability::debug::ABILITY_DEBUG
                            .load(std::sync::atomic::Ordering::Relaxed)
                        {
                            eprintln!("[TEMP_DIAG] CONDITION FAILED: no matching card in zones");
                        }
                        return false;
                    }
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(std::sync::atomic::Ordering::Relaxed)
                    {
                        eprintln!("[TEMP_DIAG] CONDITION PASSED");
                    }
                }
                true
            }
            "before_live" => {
                !matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::LiveCardSetFirstAttacker
                ) && !matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::LiveCardSetSecondAttacker
                ) && !matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::FirstAttackerPerformance
                ) && !matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::SecondAttackerPerformance
                ) && !matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::LiveVictoryDetermination
                )
            }
            "first_turn" => self.game_state.is_first_turn,
            _ => {
                let turn_ok = match condition.turn_number {
                    Some(tn) => self.game_state.turn_number == tn,
                    None => true,
                };
                if !turn_ok {
                    return false;
                }
                if let Some(phase_str) = phase {
                    match phase_str {
                        "active" => matches!(
                            self.game_state.current_phase,
                            crate::game_state::Phase::Active
                        ),
                        "live_phase" | "live" => matches!(
                            self.game_state.current_phase,
                            crate::game_state::Phase::LiveCardSetFirstAttacker
                                | crate::game_state::Phase::LiveCardSetSecondAttacker
                                | crate::game_state::Phase::FirstAttackerPerformance
                                | crate::game_state::Phase::SecondAttackerPerformance
                                | crate::game_state::Phase::LiveVictoryDetermination
                        ),
                        "live_card_set" => matches!(
                            self.game_state.current_phase,
                            crate::game_state::Phase::LiveCardSetFirstAttacker
                                | crate::game_state::Phase::LiveCardSetSecondAttacker
                        ),
                        "live_performance" => {
                            matches!(
                                self.game_state.current_phase,
                                crate::game_state::Phase::FirstAttackerPerformance
                            ) || matches!(
                                self.game_state.current_phase,
                                crate::game_state::Phase::SecondAttackerPerformance
                            )
                        }
                        "live_victory" => matches!(
                            self.game_state.current_phase,
                            crate::game_state::Phase::LiveVictoryDetermination
                        ),
                        _ => true,
                    }
                } else {
                    true
                }
            }
        }
    }

    pub(crate) fn evaluate_state_condition(&self, condition: &Condition) -> bool {
        let state = condition.state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let resource_type = condition.resource_type.as_deref();
        let all_cards = condition.all.unwrap_or(false);
        let player = self.resolve_condition_player(target);

        if resource_type == Some("energy") {
            match state {
                "active" => {
                    if all_cards {
                        player.energy_zone.active_count() == player.energy_zone.cards.len()
                    } else {
                        player.energy_zone.active_count() > 0
                    }
                }
                "wait" => {
                    if all_cards {
                        player.energy_zone.active_count() == 0
                    } else {
                        player.energy_zone.active_count() < player.energy_zone.cards.len()
                    }
                }
                _ => true,
            }
        } else {
            match state {
                "active" | "wait" => {
                    let stage_cards: Vec<i16> = player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect();
                    if stage_cards.is_empty() {
                        return false;
                    }
                    let has_filter = condition.group_names.is_some()
                        || condition.card_type.is_some()
                        || condition.characters.as_ref().is_some_and(|c| !c.is_empty());
                    let check_orientation = |cid: i16| -> bool {
                        self.game_state
                            .mods
                            .get_orientation_modifier(cid)
                            .map_or(state == "active", |o| o.as_str() == state)
                    };
                    if has_filter {
                        stage_cards.iter().any(|&cid| {
                            check_orientation(cid)
                                && self.card_matches_count_filters(
                                    cid,
                                    condition.card_type.as_deref(),
                                    condition.group_names.as_deref(),
                                    &[],
                                    condition.cost_limit,
                                    condition.cost_limit_operator.as_deref(),
                                    false,
                                    condition,
                                )
                        })
                    } else {
                        self.activating_card_id
                            .is_some_and(|cid| stage_cards.contains(&cid) && check_orientation(cid))
                    }
                }
                _ => true,
            }
        }
    }

    pub(crate) fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        let energy_state = condition.energy_state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        match energy_state {
            "active" => player.energy_zone.active_count() > 0,
            _ => true,
        }
    }

    /// Evaluate movement condition — matches jidou auto trigger types:
    ///   on_area_move     ("エリアを移動したとき/するたび")  → movement: "moved"/"moves"
    ///   on_discard_from_stage ("ステージから控え室に置かれたとき") → preceding_moved path
    ///   on_baton_touch    ("バトンタッチして控え室に置かれた") → movement: "baton_touch"
    ///   on_move_or_energy ("エリアを移動するかエネルギーが置かれた") → movement: "moves" + energy check
    ///   on_appear_or_move ("登場か、エリアを移動するたび") → movement: "moves" + appearance check
    /// Core helper: given a slice of PositionChangeEvents, check if any event
    /// matches the condition's filters (self_target, group_names, position).
    /// Used by "position_change", "moved", and "moves" (via entry snapshot).
    fn position_event_matches_filters(
        &self,
        events: &[crate::types::PositionChangeEvent],
        condition: &Condition,
    ) -> bool {
        if events.is_empty() {
            return false;
        }
        let card_db = &self.game_state.card_database;
        let pos_names = ["left_side", "center", "right_side"];
        let is_from = condition.area_direction.as_deref() == Some("from");
        events.iter().any(|event| {
            if condition.self_target.unwrap_or(false)
                && self.activating_card_id != Some(event.moved_card_id)
            {
                return false;
            }
            if let Some(ref groups) = condition.group_names {
                if !groups.is_empty()
                    && !groups.iter().any(|g| {
                        crate::ability::util::card_matches_group_str(
                            card_db,
                            event.moved_card_id,
                            Some(g),
                        )
                    })
                {
                    return false;
                }
            }
            if let Some(req_pos) = condition.position.as_ref().and_then(|p| p.get_position()) {
                let check_pos = if is_from {
                    event.old_position
                } else {
                    event.new_position
                };
                if check_pos >= pos_names.len() || pos_names[check_pos] != req_pos {
                    return false;
                }
            }
            true
        })
    }

    pub(crate) fn evaluate_movement_condition(&self, condition: &Condition) -> bool {
        let movement = condition.movement.as_deref().unwrap_or("");
        let te = condition.trigger_event.as_ref();
        let location = condition
            .location
            .as_deref()
            .or_else(|| te.and_then(|t| t.location.as_deref()))
            .unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);

        match movement {
            "moved" => {
                let base_check = self.evaluate_has_moved(condition, &player);
                if !base_check {
                    return false;
                }
                if let Some(cost_limit) = condition.cost_limit {
                    let op = condition.cost_limit_operator.as_deref().unwrap_or(">=");
                    if let Some(ref moved) = self.game_state.recently_moved_cards {
                        if !moved.iter().any(|&cid| {
                            self.game_state
                                .card_database
                                .get_card(cid)
                                .is_some_and(|c| {
                                    c.cost.is_some_and(|cost| {
                                        compare_counts(Some(op), cost, cost_limit)
                                    })
                                })
                        }) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            "position_change" => {
                if !self.position_event_matches_filters(
                    &self.game_state.position_change_events,
                    condition,
                ) {
                    return false;
                }
                if let Some(cost_limit) = condition.cost_limit {
                    let op = condition.cost_limit_operator.as_deref().unwrap_or(">=");
                    if let Some(ref moved) = self.game_state.recently_moved_cards {
                        if !moved.iter().any(|&cid| {
                            self.game_state
                                .card_database
                                .get_card(cid)
                                .is_some_and(|c| {
                                    c.cost.is_some_and(|cost| {
                                        compare_counts(Some(op), cost, cost_limit)
                                    })
                                })
                        }) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            }
            "notmoved" => true,
            "baton_touch" => {
                let triggered = condition.baton_touch_trigger.unwrap_or(false);
                if !triggered {
                    return false;
                }
                // Check per-player baton touch count
                let player_id = player.id.as_str();
                let bt_count = self.game_state.get_baton_touch_count(player_id);
                if bt_count == 0 {
                    return false;
                }
                if let Some(min_count) = condition
                    .min_baton_touch_count
                    .or_else(|| te.and_then(|t| t.min_count))
                {
                    if bt_count < min_count {
                        return false;
                    }
                }
                let replaced_id = match self.game_state.baton_touch_replaced_member_id {
                    Some(id) => id,
                    None => return false,
                };
                // Verify the baton touch belongs to this player: the replaced
                // card must be in this player's waitroom (baton touch always
                // moves the replaced member to the owner's waitroom).
                let own_waitroom = if player_id == self.game_state.player1.id {
                    &self.game_state.player1.waitroom.cards
                } else {
                    &self.game_state.player2.waitroom.cards
                };
                if !own_waitroom.contains(&replaced_id) {
                    return false;
                }
                if !location.is_empty() {
                    if Zone::from_str(location) == Some(Zone::Discard)
                        || Zone::from_str(location) == Some(Zone::Waitroom)
                    {
                        let in_discard = self
                            .game_state
                            .player1
                            .waitroom
                            .cards
                            .contains(&replaced_id)
                            || self
                                .game_state
                                .player2
                                .waitroom
                                .cards
                                .contains(&replaced_id);
                        if !in_discard {
                            return false;
                        }
                    }
                }
                if condition.exclude_self.unwrap_or(false)
                    && self.game_state.activating_card == Some(replaced_id)
                {
                    return false;
                }
                // group_names and cost_limit may describe the arriving member
                // ("baton touch WITH a X member") or the replaced member
                // ("baton touch FROM a X member"). The location field determines
                // which: if self is in discard (location=discard), self is the
                // replaced member, so group/cost describe the ARRIVING member.
                // If self is on stage (location=stage), self is the arriving
                // member, so group/cost describe the REPLACED member.
                let arriving_id = self.game_state.baton_touch_arriving_card_id;
                let loc_discard = matches!(
                    Zone::from_str(location),
                    Some(Zone::Discard | Zone::Waitroom)
                );
                let check_id_for_group = if loc_discard {
                    // Self is replaced: group/cost describe the arriving member
                    arriving_id
                } else {
                    // Self is arriving: group/cost describe the replaced member
                    Some(replaced_id)
                };
                if let Some(ref groups) = condition.group_names {
                    if !groups.is_empty() {
                        if let Some(check_card) = check_id_for_group {
                            let group_ok = groups.iter().any(|g| {
                                crate::ability::util::card_matches_group_str(
                                    &self.game_state.card_database,
                                    check_card,
                                    Some(g),
                                )
                            });
                            if !group_ok {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                }
                // ability_filter + card_type: apply to the same check_id as
                // group_names (arriving or replaced member depending on location).
                if let Some(check_card) = check_id_for_group {
                    if let Some(card) = self.game_state.card_database.get_card(check_card) {
                        if let Some(ref af) = condition.ability_filter {
                            match af.as_str() {
                                "no_ability" => {
                                    if !card.abilities.is_empty() {
                                        return false;
                                    }
                                }
                                "has_ability" => {
                                    if card.abilities.is_empty() {
                                        return false;
                                    }
                                }
                                _ => {}
                            }
                        }
                        if let Some(ref ct) = condition.card_type {
                            let card_type_ok = match ct.as_str() {
                                "member_card" | "member" => card.is_member(),
                                "live_card" => {
                                    matches!(card.card_type, crate::card::CardType::Live)
                                }
                                _ => true,
                            };
                            if !card_type_ok {
                                return false;
                            }
                        }
                    } else {
                        return false;
                    }
                }
                let bt_source = condition
                    .baton_touch_source
                    .as_deref()
                    .or_else(|| te.and_then(|t| t.source_character.as_deref()));
                if let Some(source_name) = bt_source {
                    if let Some(card) = self.game_state.card_database.get_card(replaced_id) {
                        let norm_name = crate::card::CardDatabase::normalize_name(&card.name);
                        let norm_source = crate::card::CardDatabase::normalize_name(source_name);
                        if !norm_name.contains(&norm_source) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if let Some(cost_limit) = condition.cost_limit {
                    if let Some(check_card) = check_id_for_group {
                        if let Some(card) = self.game_state.card_database.get_card(check_card) {
                            let op = condition.cost_limit_operator.as_deref().unwrap_or(">=");
                            if !card
                                .cost
                                .is_some_and(|cost| compare_counts(Some(op), cost, cost_limit))
                            {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if let Some(ref prop) = condition.card_property {
                    if let Some(check_card) = check_id_for_group {
                        let has_prop = match prop.as_str() {
                            "has_blade_heart" => self
                                .game_state
                                .card_database
                                .get_card(check_card)
                                .is_some_and(|c| c.has_blade_heart()),
                            _ => false,
                        };
                        if condition.negation.unwrap_or(false) == has_prop {
                            return false;
                        }
                    }
                }
                let has_cost_comparison = condition.comparison_type.as_deref() == Some("cost")
                    || te.is_some_and(|t| t.cost_comparison.is_some());
                if has_cost_comparison {
                    if let Some(replaced_cost) = self.game_state.baton_touch_replaced_member_cost {
                        if let Some(activating_id) = self.game_state.activating_card {
                            if let Some(card) =
                                self.game_state.card_database.get_card(activating_id)
                            {
                                if let Some(current_cost) = card.cost {
                                    let op = condition.operator.as_deref().or_else(|| {
                                        te.and_then(|t| {
                                            t.cost_comparison
                                                .as_ref()
                                                .and_then(|cc| cc.operator.as_deref())
                                        })
                                    });
                                    if !compare_counts(op, replaced_cost, current_cost) {
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
                true
            }
            "moves" => {
                let self_effect_only = condition
                    .self_effect_only
                    .or_else(|| te.and_then(|t| t.self_effect_only));
                let energy_placed = condition
                    .energy_placed
                    .or_else(|| te.and_then(|t| t.energy_placed));

                let snapshot_energy = self
                    .game_state
                    .entry_snapshot_last_energy_placed_by_effect();
                let snapshot_energy_player = self
                    .game_state
                    .entry_snapshot_last_energy_placed_by_player();
                let snapshot_area = self.game_state.entry_snapshot_last_area_move_card_id();
                let snapshot_area_player =
                    self.game_state.entry_snapshot_last_area_move_by_player();

                // Two sources: position_change_events (batch-scoped, from position
                // change executors) + turn_area_movements (turn-scoped, from
                // push_movement_event). Both track stage→stage movement.
                let this_card_moved = self.activating_card_id.is_some_and(|cid| {
                    self.game_state
                        .position_change_events
                        .iter()
                        .any(|e| e.moved_card_id == cid)
                        || self
                            .game_state
                            .turn_area_movements
                            .iter()
                            .any(|m| m.moved_card_id == cid)
                });
                let area_ok = if !this_card_moved {
                    // "このメンバーがエリアを移動する" — THIS card must have moved.
                    // The area_ok shortcut (self_effect_only=None) does NOT apply
                    // because a different card moving does not satisfy "this member".

                    false
                } else {
                    self_effect_only.is_none_or(|_| {
                        let area_id = snapshot_area.or(self.game_state.last_area_move_card_id());
                        let area_by = snapshot_area_player
                            .as_deref()
                            .or_else(|| self.game_state.last_area_move_by_player());
                        area_id.is_some() && area_by == Some(player.id.as_str())
                    })
                };
                let energy_ok = energy_placed.is_none_or(|_| {
                    let energy_val = if snapshot_energy {
                        true
                    } else if !self.game_state.last_energy_placed_by_effect() {
                        false
                    } else {
                        // snapshot is false but global is true — use global
                        true
                    };
                    let energy_player = snapshot_energy_player
                        .as_deref()
                        .or_else(|| self.game_state.last_energy_placed_by_player());
                    energy_val
                        && (!self_effect_only.unwrap_or(false) || energy_player == Some(&player.id))
                });
                let has_area_check =
                    self_effect_only.is_some() || condition.movement.as_deref() == Some("moves");
                let has_energy_check = energy_placed.is_some();
                if !has_area_check && !has_energy_check {
                    true
                } else if has_area_check && has_energy_check {
                    area_ok || energy_ok
                } else if has_area_check {
                    area_ok
                } else {
                    energy_ok
                }
            }
            _ => match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    player.stage.stage[0] != -1
                        || player.stage.stage[1] != -1
                        || player.stage.stage[2] != -1
                }
                Some(Zone::Hand) => !player.hand.cards.is_empty(),
                Some(Zone::Discard) => !player.waitroom.cards.is_empty(),
                _ => true,
            },
        }
    }

    /// Evaluate whether the activating card changed its stage position by
    /// checking the explicit PositionChangeEvent list. Replaces the fragile
    /// snapshot-based detection with direct event lookups.
    ///
    /// Per Q126: ステージに登場しているこの能力を持つメンバーが、
    /// レフトサイドエリア、センターエリア、ライトサイドエリアの
    /// いずれかのエリアに移動したときに発動する自動能力です。
    /// (Triggers when a member on stage moves between left/center/right areas,
    /// NOT on zone changes like hand→stage or stage→discard.)
    ///
    /// Per Q220: 他のメンバーがポジションチェンジしたことにより、
    /// ポジションチェンジ先のこのメンバーが移動した場合、自動能力は発動する。
    /// (If another member's position change causes this member to move,
    /// the auto ability does trigger.)
    ///
    /// Japanese text forms that map to this evaluation:
    ///   移動したとき (past tense) — "has_moved" standalone
    ///   登場か、エリアを移動したとき — "has_moved" with appearance OR
    fn evaluate_has_moved(&self, condition: &Condition, _player: &crate::player::Player) -> bool {
        let card_moved_position = self.activating_card_id.is_some_and(|cid| {
            self.game_state
                .position_change_events
                .iter()
                .any(|e| e.moved_card_id == cid)
        });

        if condition.text.contains("登場") {
            let has_appeared = self
                .activating_card_id
                .is_some_and(|cid| self.game_state.has_card_appeared_this_turn(cid));
            has_appeared || card_moved_position
        } else {
            card_moved_position
        }
    }

    pub(crate) fn evaluate_score_threshold_condition(&self, condition: &Condition) -> bool {
        let count = condition.count.unwrap_or(1);
        let operator = condition.operator.as_deref();
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let cheer_count = if player.is_first_attacker {
            self.game_state.player1_cheer_blade_heart_count
        } else {
            self.game_state.player2_cheer_blade_heart_count
        };
        util::compare_counts(operator, cheer_count, count)
    }

    pub(crate) fn evaluate_choice_condition(&self, condition: &Condition) -> bool {
        if let Some(ref options) = condition.options {
            !options.is_empty()
        } else {
            true
        }
    }

    pub(crate) fn evaluate_position_change_condition(&self, condition: &Condition) -> bool {
        let optional = condition.options.as_ref().map(|_| true).unwrap_or(false);
        if optional {
            if self.game_state.position_change_occurred_this_turn {
                return true;
            }
            return false;
        }
        self.game_state.position_change_occurred_this_turn
    }

    pub(crate) fn evaluate_state_change_condition(&self, condition: &Condition) -> bool {
        let _during_main_phase = condition.text.contains("main_phase");
        if _during_main_phase && self.game_state.current_phase != Phase::Main {
            return false;
        }
        if let (Some(from), Some(to)) = (condition.get_from_state(), condition.get_to_state()) {
            if let Some(target_count) = condition.count {
                if condition.operator.as_deref() == Some(">=") {
                    let actual = self.game_state.last_state_change_wait_to_active_count;
                    return actual >= target_count;
                }
            }
            let is_opponent = condition.target.as_deref().unwrap_or("self") == "opponent"
                || condition.text.contains("相手");
            // First pass: check recently_state_changed for actual transitions.
            // This is the primary source — only cards that actually changed state
            // should satisfy the condition.
            let target_player = if is_opponent {
                self.resolve_condition_player("opponent")
            } else {
                self.resolve_condition_player("self")
            };
            let stage_set: std::collections::HashSet<i16> =
                target_player.stage.stage.iter().copied().collect();
            for (cid, cfrom, cto) in &self.game_state.recently_state_changed {
                if !stage_set.contains(cid) {
                    continue;
                }
                if cfrom != from || cto != to {
                    continue;
                }
                // Apply extra filters (cost_limit, etc.)
                if let Some(cl) = condition.cost_limit {
                    let card_db = &self.game_state.card_database;
                    let cost_ok = card_db.get_card(*cid).is_some_and(|c| {
                        let card_cost = c.cost.unwrap_or(0);
                        let op = condition.cost_limit_operator.as_deref().unwrap_or("<=");
                        match op {
                            "<=" => card_cost <= cl,
                            "<" => card_cost < cl,
                            ">=" => card_cost >= cl,
                            ">" => card_cost > cl,
                            "==" | "=" => card_cost == cl,
                            _ => true,
                        }
                    });
                    if !cost_ok {
                        continue;
                    }
                }
                log::debug!(
                    "[STATE_CHANGE_COND] card={} transition {}→{} matches (recently_state_changed)",
                    cid,
                    cfrom,
                    cto
                );
                return true;
            }
            log::debug!(
                "[STATE_CHANGE_COND] no matching transition found in recently_state_changed"
            );
            return false;
        }
        true
    }

    pub(crate) fn evaluate_opponent_choice_condition(&self, condition: &Condition) -> bool {
        let _target = condition.target.as_deref().unwrap_or("opponent");
        let negation = condition.negation.unwrap_or(false);
        let opponent_declined = self.game_state.opponent_choice_declined;
        if negation {
            opponent_declined
        } else {
            !opponent_declined
        }
    }

    pub(crate) fn evaluate_opponent_live_success_condition(&self, condition: &Condition) -> bool {
        if !self.game_state.opponent_live_success_this_turn {
            return false;
        }
        if condition.no_excess_heart.unwrap_or(false) {
            return self.no_excess_heart_flag("opponent");
        }
        true
    }

    pub(crate) fn evaluate_no_excess_heart_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        self.no_excess_heart_flag(target)
    }

    pub(crate) fn evaluate_complex_condition(&self, condition: &Condition) -> bool {
        if let Some(ref cause) = condition.cause {
            if !self.evaluate_condition(cause) {
                return false;
            }
        }
        // Check the effect sub-condition if present.
        if let Some(ref effect) = condition.effect {
            // The effect stores a condition-like AbilityEffect. Evaluate it
            // via evaluate_effect_condition which checks negated card presence.
            if effect.compound.conditional_negation.unwrap_or(false) {
                if let Some(card_type) = &effect.card_type {
                    let loc = effect.location.as_deref().unwrap_or("hand");
                    let zone = crate::ability::enums::Zone::from_str(loc);
                    let target = effect.target_name();
                    let player = self.resolve_condition_player(&target);
                    let cards = util::zone_cards(player, loc);
                    let matching = match zone {
                        Some(z) if z == crate::ability::enums::Zone::RevealedCards => {
                            self.game_state.revealed_cards.clone()
                        }
                        _ => cards.to_vec(),
                    };
                    let count = matching
                        .iter()
                        .filter(|&&cid| {
                            self.game_state
                                .card_database
                                .get_card(cid)
                                .is_some_and(|c| {
                                    if card_type == "live_card" {
                                        c.is_live()
                                    } else if card_type == "member_card" {
                                        c.is_member()
                                    } else {
                                        true
                                    }
                                })
                        })
                        .count() as u32;
                    // Negation: passes only when NO matching cards exist
                    if count > 0 {
                        return false;
                    }
                    return true;
                }
            }
        }
        true
    }
}
