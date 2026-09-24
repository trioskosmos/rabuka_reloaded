use super::{comparison_default_count, ConditionContext};
use crate::ability::enums::Zone;
use crate::ability::util::{self, compare_counts};
use crate::ability_queue::ConditionalChoice;
use crate::card::{ComparisonTarget, Condition};
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
use smallvec::SmallVec;

impl<'a> ConditionContext<'a> {
    pub(crate) fn evaluate_both_condition(&self, condition: &Condition) -> bool {
        let values = match condition.get_values() {
            Some(v) if !v.is_empty() => v,
            _ => return false,
        };
        let location = condition.get_location().unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let cards: Vec<i16> = match Zone::from_str(location) {
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.cards.to_vec(),
            Some(Zone::LiveCardZone) => player.live_card_zone.cards.to_vec(),
            _ => player
                .success_live_card_zone
                .cards
                .iter()
                .chain(player.live_card_zone.cards.iter())
                .copied()
                .collect(),
        };
        values.iter().all(|&val| {
            cards.iter().any(|&cid| {
                self.game_state
                    .card_database
                    .get_card(cid)
                    .and_then(|c| c.score)
                    .map_or(false, |s| s == val)
            })
        })
    }

    pub(crate) fn evaluate_comparison_condition(&self, condition: &Condition) -> bool {
        if let Some(v) = self.evaluate_check_self_condition(condition) {
            return v;
        }
        if condition.get_comparison_type() == Some("energy_relative") {
            return self.evaluate_relative_energy(condition);
        }
        if condition.get_comparison_type() == Some("cost")
            && condition.get_location() == Some("activating_card")
        {
            if let Some(act) = self.game_state.activating_card {
                return self.evaluate_activating_cost(condition, act);
            }
        }
        if condition.get_original_value() == Some(true)
            && condition.get_comparison_type() == Some("score")
        {
            return self.evaluate_original_score(condition);
        }
        if let Some(ref pos) = condition.get_position() {
            if pos.get_position() == Some("front") {
                return self.evaluate_front_comparison(condition);
            }
            if condition.get_comparison_type() == Some("cost") {
                if let Some(position) = pos.get_position() {
                    return self.evaluate_position_cost(condition, position);
                }
            }
        }

        let count = self.get_count_for_condition(condition);
        if condition.get_comparison_type() == Some("score") {
            let opp = self.get_count_for_target(condition, "opponent");
            log::debug!(target: "rabuka_engine::ability::condition::card",
                "[SCORE_CMP] location={:?} aggregate={:?} scope={:?} self_total={} opp_total={} op={:?}",
                condition.get_location(),
                condition.get_aggregate(),
                condition.get_scope(),
                count,
                opp,
                condition.get_operator()
            );
        }
        if let Some(ref values) = condition.get_values() {
            if condition.get_comparison_type() == Some("score") {
                let location = condition.get_location().unwrap_or("");
                let target = condition.get_target().unwrap_or("self");
                let player = self.resolve_condition_player(target);
                let cards: SmallVec<[i16; 6]> = match Zone::from_str(location) {
                    Some(Zone::SuccessLiveZone) => player
                        .success_live_card_zone
                        .cards
                        .iter()
                        .copied()
                        .collect(),
                    Some(Zone::LiveCardZone) => {
                        player.live_card_zone.cards.iter().copied().collect()
                    }
                    _ => SmallVec::new(),
                };
                return cards.iter().any(|&cid| {
                    self.game_state
                        .card_database
                        .get_card(cid)
                        .and_then(|c| c.score)
                        .map_or(false, |s| values.contains(&s))
                });
            }
            return values.contains(&count);
        }

        let target_count = self.comparison_target_count(condition);
        if condition.get_cost_total().is_some() {
            return self.evaluate_cost_total(condition);
        }
        let result = compare_counts(condition.get_operator(), count, target_count);
        let result = if result
            && condition.get_card_type().is_some()
            && condition.get_location() == Some("revealed_cards")
            && !self.game_state.revealed_cards.is_empty()
            && count == 0
        {
            false
        } else {
            result
        };
        let final_result = if condition.get_negation().unwrap_or(false) {
            !result
        } else {
            result
        };
        #[cfg(not(feature = "no_std"))]
        let op_str = condition.get_operator().unwrap_or(">=");
        #[cfg(not(feature = "no_std"))]
        super::super::push_cond_verdict(
            condition,
            &format!("実際={}, 期待={}{}", count, op_str, target_count),
            final_result,
            vec![],
        );
        final_result
    }

    fn evaluate_relative_energy(&self, condition: &Condition) -> bool {
        let me = self.resolve_condition_player(condition.get_target().unwrap_or("self"));
        let opp = self.game_state.resolve_target_player("opponent");
        let mine = me.energy_zone.active_count() as i32;
        let theirs = opp.energy_zone.active_count() as i32;
        let ahead = (theirs - mine).max(0);
        let n = condition.get_count().unwrap_or(0);
        let op = condition.get_operator().unwrap_or(">=");
        log::debug!(target: "rabuka_engine::ability::condition::card",
            "[ENERGY_REL] mine={} theirs={} ahead={} n={} op={}",
            mine,
            theirs,
            ahead,
            n,
            op
        );
        compare_counts(Some(op), u8::try_from(ahead).unwrap_or(u8::MAX), n)
    }

    fn evaluate_activating_cost(&self, condition: &Condition, act: i16) -> bool {
        let printed = self.printed_cost(act);
        let effective = crate::constants::saturate_u8(crate::constants::effective_stat(
            printed,
            self.game_state.mods.get_cost_modifier(act),
        ));
        let threshold = condition.get_count().unwrap_or(0);
        let op = condition.get_operator().unwrap_or(">=");
        log::debug!(target: "rabuka_engine::ability::condition::card",
            "[COST_SELF] card={} printed={} effective={} threshold={} op={}",
            act,
            printed,
            effective,
            threshold,
            op
        );
        compare_counts(
            Some(op),
            effective,
            threshold,
        )
    }

    fn evaluate_original_score(&self, condition: &Condition) -> bool {
        let player = self.resolve_condition_player(condition.get_target().unwrap_or("self"));
        let location = condition.get_location().unwrap_or("");
        let ids: Vec<i16> = match Zone::from_str(location) {
            Some(Zone::LiveCardZone) => player.live_card_zone.cards.to_vec(),
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.cards.to_vec(),
            _ => Vec::new(),
        };
        let card_db = &self.game_state.card_database;
        let op = condition.get_operator();
        ids.iter().any(|&id| {
            let original = card_db.get_card(id).and_then(|c| c.score).unwrap_or(0);
            let current = crate::constants::saturate_u8(crate::constants::effective_stat(
                original,
                self.game_state.mods.get_score_modifier(id),
            ));
            compare_counts(op, current, original)
        })
    }

    fn evaluate_position_cost(&self, condition: &Condition, position: &str) -> bool {
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let card_db = &self.game_state.card_database;
        let Some(card_id) = util::card_at_position(player, position) else {
            return false;
        };
        if let Some(ref groups) = condition.get_group_names() {
            if !util::card_matches_any_group(card_db, card_id, groups) {
                return false;
            }
        }
        if let Some(ref ct) = condition.get_card_type() {
            if !util::card_matches_type(card_db, card_id, Some(ct)) {
                return false;
            }
        }
        let card_cost = self.printed_cost(card_id);
        let threshold = if condition.get_comparison_target() == Some(ComparisonTarget::Opponent) {
            let opponent = self.game_state.resolve_target_player("opponent");
            util::card_at_position(opponent, position)
                .map(|id| self.printed_cost(id))
                .unwrap_or(0)
        } else {
            condition.get_count().unwrap_or(0)
        };
        let operator = condition.get_operator().unwrap_or(">=");
        compare_counts(Some(operator), card_cost, threshold)
    }

    fn comparison_target_count(&self, condition: &Condition) -> u8 {
        if let Some(ref comparison_target) = condition.get_comparison_target() {
            if *comparison_target == ComparisonTarget::Opponent {
                return self.get_count_for_target(condition, "opponent");
            }
            if *comparison_target == ComparisonTarget::Self_
                && matches!(condition.get_comparison_type(), Some("cost" | "score"))
            {
                return self.comparison_self_count(condition);
            }
            if condition.get_resource_type().is_some() {
                return self.get_count_for_target(condition, comparison_target.as_str());
            }
            return condition
                .get_count()
                .unwrap_or(comparison_default_count(condition));
        }
        match condition.get_comparison_type() {
            Some("cost") => {
                let entry_choice = self.comparison_entry_choice();
                condition
                    .get_cost_limit()
                    .or(condition.get_count())
                    .or(entry_choice)
                    .unwrap_or(0)
            }
            Some("score") => condition.get_count().unwrap_or(0),
            _ => self
                .comparison_entry_choice()
                .or(condition.get_count())
                .unwrap_or(comparison_default_count(condition)),
        }
    }

    fn comparison_entry_choice(&self) -> Option<u8> {
        self.game_state
            .ability_queue
            .current_entry()
            .and_then(|e| match &e.conditional_choice {
                Some(ConditionalChoice::Str(s)) => s.parse::<u8>().ok(),
                _ => None,
            })
    }

    fn comparison_self_count(&self, condition: &Condition) -> u8 {
        let target = condition.get_target().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);
        let Some(act_id) = self.activating_card_id else {
            return self.get_count_for_target(condition, target);
        };
        let ctype = condition.get_comparison_type().unwrap_or("cost");
        if !player.stage.stage.iter().any(|&id| id == act_id) {
            return self.get_count_for_target(condition, target);
        }
        let base = self
            .game_state
            .card_database
            .get_card(act_id)
            .and_then(|c| c.cost)
            .or_else(|| {
                self.game_state
                    .card_database
                    .get_card(act_id)
                    .and_then(|c| c.score)
            })
            .unwrap_or(0);
        if ctype == "cost" {
            base.saturating_sub(crate::constants::saturate_u8(
                self.game_state.mods.get_cost_modifier(act_id),
            ))
        } else {
            base
        }
    }

    fn evaluate_cost_total(&self, condition: &Condition) -> bool {
        let total = condition.get_cost_total().unwrap_or(0);
        let operator = condition
            .get_cost_total_operator()
            .map(|o| o.as_str())
            .or(condition.get_operator())
            .unwrap_or("=");
        let target = condition.get_target().unwrap_or("self");
        let sum_cost: i32 = if condition.get_source() == Some("preceding_moved")
            || condition.get_source() == Some("previous_moved_cards")
            || condition.get_location().is_none()
        {
            self.sum_condition_costs(condition, self.moved_cards)
        } else {
            let player = self.game_state.resolve_target_player(target);
            let location = condition.get_location().unwrap_or(Zone::Stage.to_str());
            let card_ids: SmallVec<[i16; 8]> = match Zone::from_str(location) {
                Some(Zone::Stage) => player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .copied()
                    .collect(),
                Some(Zone::Hand) => player.hand.cards.iter().copied().collect(),
                Some(Zone::Discard) | Some(Zone::Waitroom) => {
                    player.waitroom.cards.iter().copied().collect()
                }
                _ => SmallVec::new(),
            };
            self.sum_condition_costs(condition, &card_ids)
        };
        let result = compare_counts(
            Some(operator),
            crate::constants::saturate_u8(sum_cost),
            total,
        );
        log::debug!(
            "[COST_TOTAL] sum={} threshold={} operator={} result={} location={:?} target={}",
            sum_cost,
            total,
            operator,
            result,
            condition.get_location(),
            condition.get_target().unwrap_or("self"),
        );
        result
    }

    fn sum_condition_costs(&self, condition: &Condition, cards: &[i16]) -> i32 {
        cards
            .iter()
            .filter(|&&id| self.matches_condition_groups_and_type(condition, id))
            .map(|&id| {
                crate::constants::effective_stat(
                    self.printed_cost(id),
                    self.game_state.mods.get_cost_modifier(id),
                )
            })
            .sum()
    }

    pub(crate) fn evaluate_front_comparison(&self, condition: &Condition) -> bool {
        let master_id = match self.activating_card_id {
            Some(id) => id,
            None => return false,
        };
        let gs = self.game_state;
        let master_player = gs.resolve_target_player("self");
        let master_idx = match master_player
            .stage
            .stage
            .iter()
            .position(|&id| id == master_id)
        {
            Some(idx) => idx,
            None => return false,
        };
        let master_area = util::pos_to_area(master_idx);
        let front_area = master_area.front_area();
        let front_idx = front_area.to_index();
        let opponent = gs.resolve_target_player("opponent");
        let front_card_id = opponent.stage.stage[front_idx];
        if front_card_id == -1 {
            return false;
        }
        let master_cost = self.printed_cost(master_id);
        let front_cost = self.printed_cost(front_card_id);
        compare_counts(condition.get_operator(), front_cost, master_cost)
    }

    pub(crate) fn evaluate_all_cost_comparison_condition(&self, condition: &Condition) -> bool {
        let gs = self.game_state;
        let self_player = gs.resolve_target_player("self");
        let opp_player = gs.resolve_target_player("opponent");
        let get_stage_costs = |player: &crate::player::Player| -> Vec<u8> {
            player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .map(|&id| self.effective_condition_cost(id))
                .collect()
        };
        let mut opp_costs = get_stage_costs(opp_player);
        let self_costs = get_stage_costs(self_player);
        opp_costs.sort_unstable();
        let max_opp = opp_costs.last().copied().unwrap_or(0);
        let operator = condition.get_operator().unwrap_or(">");
        self_costs
            .iter()
            .any(|&sc| compare_counts(Some(operator), sc, max_opp))
    }

    pub(crate) fn evaluate_highest_cost_on_stage_condition(&self, condition: &Condition) -> bool {
        let position = match condition.get_position().and_then(|p| p.get_position()) {
            Some(p) => p,
            None => return false,
        };
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let card_at_pos = match util::card_at_position(player, position) {
            Some(id) => id,
            None => return false,
        };
        let pos_cost = self.effective_condition_cost(card_at_pos);
        let operator = condition.get_operator().unwrap_or(">");
        player
            .stage
            .stage
            .iter()
            .filter(|&&id| id != -1 && id != card_at_pos)
            .all(|&id| compare_counts(Some(operator), pos_cost, self.effective_condition_cost(id)))
    }
}
