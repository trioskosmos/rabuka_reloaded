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
                                if let Some(activating_card_id) = self.activating_card_id {
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
                matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::LiveCardSetFirstAttacker
                ) || matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::LiveCardSetSecondAttacker
                ) || matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::FirstAttackerPerformance
                ) || matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::SecondAttackerPerformance
                ) || matches!(
                    self.game_state.current_phase,
                    crate::game_state::Phase::LiveVictoryDetermination
                )
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
                        player.energy_zone.active_energy_count == player.energy_zone.cards.len()
                    } else {
                        player.energy_zone.active_energy_count > 0
                    }
                }
                "wait" => {
                    if all_cards {
                        player.energy_zone.active_energy_count == 0
                    } else {
                        player.energy_zone.active_energy_count < player.energy_zone.cards.len()
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
                        || condition
                            .characters
                            .as_ref()
                            .map_or(false, |c| !c.is_empty());
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
                                    condition.group_names.as_ref().map(|v| v.as_slice()),
                                    &[],
                                    condition.cost_limit,
                                    condition.cost_limit_operator.as_deref(),
                                    false,
                                    condition,
                                )
                        })
                    } else {
                        self.activating_card_id.map_or(false, |cid| {
                            stage_cards.contains(&cid) && check_orientation(cid)
                        })
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

    pub(crate) fn evaluate_movement_condition(&self, condition: &Condition) -> bool {
        let movement = condition.movement.as_deref().unwrap_or("");
        let movement_state = condition.movement_state.as_deref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);

        match movement {
            "moved" => {
                let card_moved = self
                    .activating_card_id
                    .map_or(false, |cid| self.game_state.has_card_moved_this_turn(cid));
                let base_check = if let Some(state) = movement_state {
                    match state {
                        "to_stage" => {
                            player.stage.stage[0] != -1
                                || player.stage.stage[1] != -1
                                || player.stage.stage[2] != -1
                        }
                        "from_stage" => !player.waitroom.cards.is_empty(),
                        "to_discard" => !player.waitroom.cards.is_empty(),
                        "has_moved" => {
                            if !card_moved {
                                return false;
                            }
                            let is_position_change =
                                self.game_state.position_change_occurred_this_turn;
                            if condition.text.contains("登場") {
                                let on_stage = self.activating_card_id.map_or(false, |cid| {
                                    let p = self.resolve_condition_player(
                                        condition.target.as_deref().unwrap_or("self"),
                                    );
                                    p.stage.stage.contains(&cid)
                                });
                                on_stage && card_moved
                            } else {
                                is_position_change && card_moved
                            }
                        }
                        _ => true,
                    }
                } else {
                    if !card_moved {
                        return false;
                    }
                    let is_position_change = self.game_state.position_change_occurred_this_turn;
                    if condition.text.contains("登場") {
                        let on_stage = self.activating_card_id.map_or(false, |cid| {
                            let p = self.resolve_condition_player(
                                condition.target.as_deref().unwrap_or("self"),
                            );
                            p.stage.stage.contains(&cid)
                        });
                        on_stage && card_moved
                    } else {
                        is_position_change && card_moved
                    }
                };
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
                                .map_or(false, |c| {
                                    c.cost.map_or(false, |cost| {
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
                if self.game_state.baton_touch_count == 0 {
                    return false;
                }
                if let Some(min_count) = condition.min_baton_touch_count {
                    if self.game_state.baton_touch_count < min_count {
                        return false;
                    }
                }
                let replaced_id = match self.game_state.baton_touch_replaced_member_id {
                    Some(id) => id,
                    None => return false,
                };
                if let Some(ref loc) = condition.location {
                    if Zone::from_str(loc.as_str()) == Some(Zone::Discard)
                        || Zone::from_str(loc.as_str()) == Some(Zone::Waitroom)
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
                if condition.exclude_self.unwrap_or(false) {
                    if self.game_state.activating_card == Some(replaced_id) {
                        return false;
                    }
                }
                if let Some(ref groups) = condition.group_names {
                    if !groups.is_empty() {
                        let group_ok = groups.iter().any(|g| {
                            crate::ability::util::card_matches_group_str(
                                &self.game_state.card_database,
                                replaced_id,
                                Some(g),
                            )
                        });
                        if !group_ok {
                            return false;
                        }
                    }
                }
                if let Some(source_name) = condition.baton_touch_source.as_deref() {
                    if let Some(card) = self.game_state.card_database.get_card(replaced_id) {
                        if !card.name.contains(source_name) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if let Some(cost_limit) = condition.cost_limit {
                    if let Some(card) = self.game_state.card_database.get_card(replaced_id) {
                        let op = condition.cost_limit_operator.as_deref().unwrap_or(">=");
                        if !card
                            .cost
                            .map_or(false, |cost| compare_counts(Some(op), cost, cost_limit))
                        {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if condition.comparison_type.as_deref() == Some("cost") {
                    if let Some(replaced_cost) = self.game_state.baton_touch_replaced_member_cost {
                        if let Some(activating_id) = self.game_state.activating_card {
                            if let Some(card) =
                                self.game_state.card_database.get_card(activating_id)
                            {
                                if let Some(current_cost) = card.cost {
                                    if !compare_counts(
                                        condition.operator.as_deref(),
                                        replaced_cost,
                                        current_cost,
                                    ) {
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
                let area_ok = condition.self_effect_only.map_or(true, |_| {
                    self.game_state.last_area_move_card_id.is_some()
                        && self
                            .game_state
                            .last_area_move_by_player
                            .as_ref()
                            .map_or(false, |mover| mover == &player.id)
                });
                let energy_ok = condition
                    .energy_placed
                    .map_or(true, |_| self.game_state.last_energy_placed_by_effect);
                let has_area_check = condition.self_effect_only.is_some();
                let has_energy_check = condition.energy_placed.is_some();
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
        if let (Some(from), Some(to)) = (
            condition.from_state.as_deref(),
            condition.to_state.as_deref(),
        ) {
            if let Some(target_count) = condition.count {
                if condition.operator.as_deref() == Some(">=") {
                    let actual = self.game_state.last_state_change_wait_to_active_count;
                    return actual >= target_count;
                }
            }
            let target = condition.target.as_deref().unwrap_or("self");
            let is_opponent = target == "opponent" || condition.text.contains("相手");
            let player = if is_opponent {
                self.resolve_condition_player("opponent")
            } else {
                self.resolve_condition_player("self")
            };
            for &card_id in &player.stage.stage {
                if card_id == -1 {
                    continue;
                }
                let ori = self.game_state.mods.get_orientation_modifier(card_id);
                let orientation_ok = match (from, to) {
                    ("active", "wait") => ori.map_or(false, |s| s == "wait"),
                    ("wait", "active") => ori.map_or(true, |s| s == "active"),
                    _ => false,
                };
                eprintln!(
                    "[STATE_CHANGE_COND] card_id={} from={:?} to={:?} ori={:?} orientation_ok={}",
                    card_id, from, to, ori, orientation_ok
                );
                if !orientation_ok {
                    continue;
                }
                if let Some(cl) = condition.cost_limit {
                    let card_db = &self.game_state.card_database;
                    let cost_ok = card_db.get_card(card_id).map_or(false, |c| {
                        let card_cost = c.cost.unwrap_or(0);
                        let op = condition.cost_limit_operator.as_deref().unwrap_or("<=");
                        eprintln!(
                            "[STATE_CHANGE_COND] cost_limit={} card_cost={} op={:?}",
                            cl, card_cost, op
                        );
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
                        eprintln!("[STATE_CHANGE_COND] cost check failed");
                        continue;
                    }
                }
                eprintln!("[STATE_CHANGE_COND] ALL CHECKS PASSED, returning true");
                return true;
            }
            eprintln!("[STATE_CHANGE_COND] no matching card found, returning false");
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
            self.evaluate_condition(cause)
        } else {
            true
        }
    }
}
