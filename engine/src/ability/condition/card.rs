use super::comparison_default_count;
use super::ConditionContext;
use crate::ability::debug::AbDebug;
use crate::ability::enums::Zone;
use crate::ability::util;
use crate::ability::util::compare_counts;
use crate::card::Condition;

impl<'a> ConditionContext<'a> {
    pub(crate) fn resolve_condition_player(&self, target: &str) -> &crate::player::Player {
        if target == "self" {
            // Use cached player if available
            if let Some(p) = self.self_player {
                return p;
            }
            self.activating_card_id
                .and_then(|cid| {
                    let p1 = &self.game_state.player1;
                    let p2 = &self.game_state.player2;
                    if p1.stage.stage.contains(&cid)
                        || p1.hand.cards.contains(&cid)
                        || p1.live_card_zone.cards.contains(&cid)
                        || p1.energy_zone.cards.contains(&cid)
                    {
                        Some(p1)
                    } else if p2.stage.stage.contains(&cid)
                        || p2.hand.cards.contains(&cid)
                        || p2.live_card_zone.cards.contains(&cid)
                        || p2.energy_zone.cards.contains(&cid)
                    {
                        Some(p2)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| self.game_state.resolve_target_player(target))
        } else {
            self.game_state.resolve_target_player(target)
        }
    }

    pub(crate) fn resolve_target_for_scope<'b>(&self, condition: &'b Condition) -> &'b str {
        let target = condition.target.as_deref().unwrap_or("self");
        if target == "self" && condition.scope.as_deref() == Some("both") {
            "both"
        } else {
            target
        }
    }

    pub(crate) fn evaluate_comparison_condition(&self, condition: &Condition) -> bool {
        if let Some(ref pos) = condition.position {
            if pos.get_position() == Some("front") {
                return self.evaluate_front_comparison(condition);
            }
        }

        let count = self.get_count_for_condition(condition);

        if let Some(ref values) = condition.values {
            return values.contains(&{ count });
        }

        let target_count = if let Some(ref comparison_target) = condition.comparison_target {
            if comparison_target == "opponent" {
                self.get_count_for_target(condition, "opponent")
            } else if comparison_target == "self"
                && (condition.comparison_type.as_deref() == Some("cost")
                    || condition.comparison_type.as_deref() == Some("score"))
            {
                let target = condition.target.as_deref().unwrap_or("self");
                let player = self.game_state.resolve_target_player(target);
                if let Some(act_id) = self.activating_card_id {
                    let ctype = condition.comparison_type.as_deref().unwrap_or("cost");
                    player
                        .stage
                        .stage
                        .iter()
                        .find(|&&id| id == act_id)
                        .map(|_| {
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
                                base.saturating_sub(
                                    self.game_state.mods.get_cost_modifier(act_id).max(0) as u32,
                                )
                            } else {
                                base
                            }
                        })
                        .unwrap_or_else(|| self.get_count_for_target(condition, target))
                } else {
                    self.get_count_for_target(condition, target)
                }
            } else {
                condition
                    .count
                    .unwrap_or(comparison_default_count(condition))
            }
        } else if condition.comparison_type.as_deref() == Some("score")
            || condition.comparison_type.as_deref() == Some("cost")
        {
            condition.count.unwrap_or(0)
        } else {
            condition
                .count
                .unwrap_or(comparison_default_count(condition))
        };

        if condition.cost_total.is_some() {
            let total = condition.cost_total.unwrap_or(0);
            let operator = condition
                .cost_total_operator
                .as_deref()
                .or(condition.operator.as_deref())
                .unwrap_or("=");
            let card_db = &self.game_state.card_database;
            let target = condition.target.as_deref().unwrap_or("self");
            let sum_cost: i32 = if condition.source.as_deref() == Some("preceding_moved")
                || condition.source.as_deref() == Some("previous_moved_cards")
                || condition.location.is_none()
            {
                // When no explicit location, or source is preceding_moved,
                // sum costs from moved_cards (cards moved by preceding cost/effect).
                self.moved_cards
                    .iter()
                    .filter(|&&id| {
                        if let Some(ref groups) = condition.group_names {
                            if !groups.is_empty()
                                && !groups
                                    .iter()
                                    .any(|g| util::card_matches_group_str(card_db, id, Some(g)))
                            {
                                return false;
                            }
                        }
                        if let Some(ref ct) = condition.card_type {
                            if !util::card_matches_type(card_db, id, Some(ct)) {
                                return false;
                            }
                        }
                        true
                    })
                    .map(|&id| {
                        let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
                        base + self.game_state.mods.get_cost_modifier(id)
                    })
                    .sum()
            } else {
                let player = self.game_state.resolve_target_player(target);
                let location = condition
                    .location
                    .as_deref()
                    .unwrap_or(Zone::Stage.to_str());
                let card_ids: Vec<i16> = match Zone::from_str(location) {
                    Some(Zone::Stage) => player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect(),
                    Some(Zone::Hand) => player.hand.cards.to_vec(),
                    Some(Zone::Discard) | Some(Zone::Waitroom) => player.waitroom.cards.to_vec(),
                    _ => vec![],
                };
                card_ids
                    .iter()
                    .filter(|&&id| {
                        if let Some(ref groups) = condition.group_names {
                            if !groups.is_empty()
                                && !groups
                                    .iter()
                                    .any(|g| util::card_matches_group_str(card_db, id, Some(g)))
                            {
                                return false;
                            }
                        }
                        if let Some(ref ct) = condition.card_type {
                            if !util::card_matches_type(card_db, id, Some(ct)) {
                                return false;
                            }
                        }
                        true
                    })
                    .map(|&id| {
                        let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
                        base + self.game_state.mods.get_cost_modifier(id)
                    })
                    .sum()
            };
            return compare_counts(Some(operator), sum_cost.max(0) as u32, total);
        }

        let result = compare_counts(condition.operator.as_deref(), count, target_count);
        if condition.negation.unwrap_or(false) {
            !result
        } else {
            result
        }
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
        let master_area = match master_idx {
            0 => crate::zones::MemberArea::LeftSide,
            1 => crate::zones::MemberArea::Center,
            _ => crate::zones::MemberArea::RightSide,
        };
        let front_area = master_area.front_area();
        let front_idx = match front_area {
            crate::zones::MemberArea::LeftSide => 0,
            crate::zones::MemberArea::Center => 1,
            crate::zones::MemberArea::RightSide => 2,
        };
        let opponent = gs.resolve_target_player("opponent");
        let front_card_id = opponent.stage.stage[front_idx];
        if front_card_id == -1 {
            return false;
        }

        let master_cost = gs
            .card_database
            .get_card(master_id)
            .and_then(|c| c.cost)
            .unwrap_or(0);
        let front_cost = gs
            .card_database
            .get_card(front_card_id)
            .and_then(|c| c.cost)
            .unwrap_or(0);

        compare_counts(condition.operator.as_deref(), front_cost, master_cost)
    }

    pub(crate) fn evaluate_all_cost_comparison_condition(&self, condition: &Condition) -> bool {
        let gs = self.game_state;
        let self_player = gs.resolve_target_player("self");
        let opp_player = gs.resolve_target_player("opponent");
        let card_db = &gs.card_database;

        let get_stage_costs = |player: &crate::player::Player| -> Vec<u32> {
            player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .map(|&id| {
                    let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
                    (base + gs.mods.get_cost_modifier(id)).max(0) as u32
                })
                .collect()
        };

        let mut opp_costs = get_stage_costs(opp_player);
        let self_costs = get_stage_costs(self_player);

        opp_costs.sort_unstable();
        let max_opp = opp_costs.last().copied().unwrap_or(0);

        let operator = condition.operator.as_deref().unwrap_or(">");

        self_costs
            .iter()
            .any(|&sc| compare_counts(Some(operator), sc, max_opp))
    }

    pub(crate) fn check_heart_type_all(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
    ) -> bool {
        if condition.heart_type.as_deref() != Some("all") {
            return true;
        }
        let card_db = &self.game_state.card_database;
        player.stage.stage.iter().any(|&id| {
            id != -1
                && card_db.get_card(id).is_some_and(|c| {
                    c.base_heart.as_ref().is_some_and(|bh| {
                        bh.hearts.contains_key(&crate::card::HeartColor::Heart00)
                    })
                })
        })
    }

    pub(crate) fn check_heart_colors(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
    ) -> bool {
        let cols = match &condition.heart_colors {
            Some(c) if !c.is_empty() => c,
            _ => return true,
        };
        let card_db = &self.game_state.card_database;
        cols.iter().all(|cs| {
            player.stage.stage.iter().any(|&id| {
                id != -1
                    && card_db.get_card(id).is_some_and(|c| {
                        c.base_heart.as_ref().is_some_and(|bh| {
                            bh.hearts.contains_key(&crate::zones::parse_heart_color(cs))
                        })
                    })
            })
        })
    }

    pub(crate) fn check_has_blade_heart(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        if condition.card_property.as_deref() != Some("has_blade_heart") {
            return true;
        }
        let card_db = &self.game_state.card_database;
        let check_cards: Vec<i16> = match Zone::from_str(location) {
            Some(Zone::RevealedCards) => self.game_state.revealed_cards.to_vec(),
            Some(Zone::Stage) => player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect(),
            _ => vec![],
        };
        if check_cards.is_empty() {
            return true;
        }
        check_cards
            .iter()
            .any(|&id| card_db.get_card(id).is_some_and(|c| c.has_blade_heart()))
    }

    pub(crate) fn check_baton_touch(&self, condition: &Condition) -> bool {
        if !condition.baton_touch_trigger.unwrap_or(false) {
            return true;
        }
        if self.game_state.baton_touch_count == 0 {
            return false;
        }
        if let Some(min_count) = condition.min_baton_touch_count {
            if self.game_state.baton_touch_count < min_count {
                return false;
            }
        }
        let card_db = &self.game_state.card_database;
        if let Some(ref groups) = condition.group_names {
            if !groups.is_empty() {
                if let Some(replaced_id) = self.game_state.baton_touch_replaced_member_id {
                    let group_ok = groups.iter().any(|g| {
                        crate::ability::util::card_matches_group_str(card_db, replaced_id, Some(g))
                    });
                    if !group_ok {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        if let Some(src) = condition.baton_touch_source.as_deref() {
            if !self
                .game_state
                .player1
                .waitroom
                .cards
                .iter()
                .chain(self.game_state.player2.waitroom.cards.iter())
                .any(|&id| card_db.get_card(id).is_some_and(|c| c.name.contains(src)))
            {
                return false;
            }
        }
        if condition.comparison_type.as_deref() == Some("cost") {
            if let Some(replaced) = self.game_state.baton_touch_replaced_member_cost {
                if let Some(act) = self.game_state.activating_card {
                    if let Some(c) = card_db.get_card(act) {
                        if let Some(cc) = c.cost {
                            if !compare_counts(condition.operator.as_deref(), replaced, cc) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    pub(crate) fn check_aggregate_total(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> Option<bool> {
        if condition.aggregate.as_deref() == Some("total")
            && Zone::from_str(location) == Some(Zone::Stage)
        {
            let bm_flat: std::collections::HashMap<i16, i32> = self
                .game_state
                .mods
                .blade_modifiers
                .iter()
                .map(|(&k, e)| (k, e.total()))
                .collect();
            Some(compare_counts(
                condition.operator.as_deref(),
                player.stage.total_blades(
                    &self.game_state.card_database,
                    &bm_flat,
                    &self.game_state.mods.orientation_modifiers,
                ),
                condition.count.unwrap_or(0),
            ))
        } else {
            None
        }
    }

    pub(crate) fn check_distinct_names(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
    ) -> bool {
        let is_distinct = condition
            .distinct
            .as_ref()
            .is_some_and(|d| d.is_distinct());
        if !is_distinct {
            return true;
        }
        let card_db = &self.game_state.card_database;

        let distinct_type = match condition.distinct.as_ref() {
            Some(crate::core::card::DistinctInfo::String(s)) => s.as_str(),
            _ => "card_name",
        };

        if distinct_type == "cost" {
            let mut seen_costs: std::collections::HashSet<u32> = std::collections::HashSet::new();
            for &cid in player.stage.stage.iter().filter(|&&id| id != -1) {
                if let Some(card) = card_db.get_card(cid) {
                    let cost = card.cost.unwrap_or(0);
                    let modified_cost =
                        (cost as i32 + self.game_state.mods.get_cost_modifier(cid)).max(0) as u32;
                    if !seen_costs.insert(modified_cost) {
                        return false;
                    }
                }
            }
        } else {
            let mut seen_names: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for &cid in player.stage.stage.iter().filter(|&&id| id != -1) {
                let names = card_db.get_card_names(cid);
                if names.is_empty() {
                    continue;
                }
                for name in names {
                    if !seen_names.insert(name) {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub(crate) fn check_no_excess_heart(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        target: &str,
    ) -> bool {
        if !condition.no_excess_heart.unwrap_or(false) && !self.no_excess_heart_flag(target) {
            return true;
        }
        let card_db = &self.game_state.card_database;
        let h: u32 = player
            .stage
            .stage
            .iter()
            .filter(|&&id| id != -1)
            .map(|&id| {
                card_db
                    .get_card(id)
                    .map(|c| {
                        if c.blade > 0
                            || c.base_heart
                                .as_ref()
                                .is_some_and(|bh| !bh.hearts.is_empty())
                        {
                            c.total_hearts()
                        } else {
                            0
                        }
                    })
                    .unwrap_or(0)
            })
            .sum();
        let need: u32 = player
            .live_card_zone
            .cards
            .iter()
            .chain(player.success_live_card_zone.cards.iter())
            .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
            .sum();
        h <= need
    }

    pub(crate) fn evaluate_location_condition(&self, condition: &Condition) -> bool {
        if let Some(ref locs) = condition.locations {
            if locs.len() >= 2 {
                if condition.target.as_deref() == Some("both")
                    && condition.comparison_type.as_deref() == Some("equality")
                {
                    return compare_counts(
                        condition.operator.as_deref(),
                        self.get_count_for_target(condition, "self"),
                        self.get_count_for_target(condition, "opponent"),
                    );
                }
                if condition.target.as_deref() == Some("either") {
                    return compare_counts(
                        condition.operator.as_deref(),
                        if self.get_count_for_target(condition, "self") > 0
                            || self.get_count_for_target(condition, "opponent") > 0
                        {
                            1
                        } else {
                            0
                        },
                        condition.count.unwrap_or(1),
                    );
                }
                return self.evaluate_multi_location_condition(condition);
            }
        }

        let _target = condition.target.as_deref().unwrap_or("self");
        let location = condition.location.as_deref().unwrap_or("");
        let card_db = &self.game_state.card_database;
        let target = self.resolve_target_for_scope(condition);
        let is_both = target == "both";
        let player = self.resolve_condition_player(target);

        if !self.check_heart_type_all(condition, player) {
            return false;
        }
        if !self.check_heart_colors(condition, player) {
            return false;
        }
        if !self.check_has_blade_heart(condition, player, location) {
            return false;
        }
        if !self.check_baton_touch(condition) {
            return false;
        }
        if let Some(res) = self.check_aggregate_total(condition, player, location) {
            return res;
        }
        if !self.check_distinct_names(condition, player) {
            return false;
        }
        if !self.check_no_excess_heart(condition, player, target) {
            return false;
        }

        if condition.comparison_type.as_deref() == Some("equality") {
            // Cross-position equality: compare cost/count at position vs position_compare
            if let (Some(ref pos_a), Some(ref pos_b)) =
                (&condition.position, &condition.position_compare)
            {
                let pos_a_str = pos_a.get_position().unwrap_or("");
                let card_a = util::card_at_position(player, pos_a_str);
                let card_b = util::card_at_position(player, pos_b);
                let cost_a = card_a
                    .and_then(|id| card_db.get_card(id))
                    .and_then(|c| c.cost)
                    .unwrap_or(0);
                let cost_b = card_b
                    .and_then(|id| card_db.get_card(id))
                    .and_then(|c| c.cost)
                    .unwrap_or(0);
                return compare_counts(condition.operator.as_deref(), cost_a, cost_b);
            }
            // Self vs opponent equality
            if is_both {
                let self_count = self.get_count_for_target(condition, "self");
                let opp_count = self.get_count_for_target(condition, "opponent");
                return compare_counts(condition.operator.as_deref(), self_count, opp_count);
            }
        }

        let group = condition
            .group_names
            .as_ref()
            .and_then(|g| g.first().map(|s| s.as_str()));
        let op = condition.operator.as_deref();

        let cards: Vec<i16> = if Zone::from_str(location) == Some(Zone::RevealedCards) {
            self.game_state.revealed_cards.clone()
        } else if is_both {
            let self_p = self.resolve_condition_player("self");
            let opp_p = self.resolve_condition_player("opponent");
            let mut combined: Vec<i16> = util::zone_cards(self_p, location).to_vec();
            combined.extend_from_slice(util::zone_cards(opp_p, location));
            combined
        } else {
            util::zone_cards(player, location).to_vec()
        };

        let count = if condition.exclude_self.unwrap_or(false) {
            let ex_id = self.activating_card_id;
            cards
                .iter()
                .filter(|&&id| {
                    id != -1
                        && Some(id) != ex_id
                        && util::card_matches_type(card_db, id, condition.card_type.as_deref())
                        && util::card_matches_group_str(card_db, id, group)
                        && util::card_matches_cost_limit_op(card_db, id, condition.cost_limit, op)
                        && self.check_original_blade_filter(condition, id)
                        && self.check_original_heart_filter(condition, id)
                })
                .count() as u32
        } else {
            util::count_matching_with_blade(
                &cards,
                card_db,
                condition.card_type.as_deref(),
                group,
                condition.cost_limit,
                op,
                |cid| {
                    self.check_original_blade_filter(condition, cid)
                        && self.check_original_heart_filter(condition, cid)
                },
            )
        };
        if condition.all_areas.unwrap_or(false) {
            let areas_ok = if is_both {
                let self_p = self.resolve_condition_player("self");
                let opp_p = self.resolve_condition_player("opponent");
                let self_full = self_p.stage.stage.iter().filter(|&&c| c != -1).count() == 3;
                let opp_full = opp_p.stage.stage.iter().filter(|&&c| c != -1).count() == 3;
                self_full && opp_full
            } else {
                player.stage.stage.iter().filter(|&&c| c != -1).count() == 3
            };
            if !areas_ok {
                return false;
            }
        }
        let thresh = condition.count.unwrap_or(1);
        let effective_op = match op {
            None if thresh > 0 => Some(">="),
            _ => op,
        };
        compare_counts(effective_op, count, thresh)
    }

    pub(crate) fn check_original_blade_filter(&self, condition: &Condition, card_id: i16) -> bool {
        if !condition.original_value.unwrap_or(false) {
            return true;
        }
        // For member cards, check_original_heart_filter handles the comparison
        if let Some(card) = self.game_state.card_database.get_card(card_id) {
            if card.is_member() {
                return true;
            }
        }
        if let Some(op) = condition.operator.as_deref() {
            let threshold = condition.count.unwrap_or(0);
            let card_blade = self
                .game_state
                .card_database
                .get_card(card_id)
                .map(|c| c.blade)
                .unwrap_or(0);
            compare_counts(Some(op), card_blade, threshold)
        } else {
            true
        }
    }

    /// "元々持つハートの数より多い/少ない" — compare a member's current total hearts
    /// (base + heart modifiers from constant abilities) against its original base.
    /// Uses the condition's operator (">", "<", ">=", etc.) and count (threshold).
    pub(crate) fn check_original_heart_filter(&self, condition: &Condition, card_id: i16) -> bool {
        if !condition.original_value.unwrap_or(false) {
            return true;
        }
        let op = match condition.operator.as_deref() {
            Some(o) => o,
            None => return true,
        };
        let card = match self.game_state.card_database.get_card(card_id) {
            Some(c) => c,
            None => return true,
        };
        if !card.is_member() {
            return true;
        }
        let base_hearts = card.total_hearts();
        let mut current_hearts = base_hearts;
        use crate::card::HeartColor;
        for color in [
            HeartColor::Heart01,
            HeartColor::Heart02,
            HeartColor::Heart03,
            HeartColor::Heart04,
            HeartColor::Heart05,
            HeartColor::Heart06,
            HeartColor::Heart00,
        ] {
            let modifier = self.game_state.mods.get_heart_modifier(card_id, color);
            if modifier > 0 {
                current_hearts += modifier as u32;
            }
        }
        compare_counts(Some(op), current_hearts, base_hearts)
    }

    pub(crate) fn evaluate_multi_location_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        if target == "both" && condition.comparison_type.as_deref() == Some("equality") {
            let p1_count = self.get_count_for_target(condition, "self");
            let p2_count = self.get_count_for_target(condition, "opponent");
            return compare_counts(condition.operator.as_deref(), p1_count, p2_count);
        }

        let player = self.resolve_condition_player(target);
        let card_db = &self.game_state.card_database;
        let card_type_filter = condition.card_type.as_deref();
        let group_names = condition.group_names.as_ref();
        let operator = condition.operator.as_deref();
        let count_threshold = condition.count.unwrap_or(1);
        let locs = condition.locations.as_ref().unwrap();

        let mut combined: Vec<i16> = Vec::new();
        for loc in locs {
            let cards = util::zone_cards(player, loc.as_str());
            combined.extend_from_slice(cards);
        }
        log::debug!("[MULTI] combined {} cards", combined.len());

        let is_distinct = condition
            .distinct
            .as_ref()
            .is_some_and(|d| d.is_distinct());
        if is_distinct {
            let distinct_type = match condition.distinct.as_ref() {
                Some(crate::core::card::DistinctInfo::String(s)) => s.as_str(),
                _ => "card_name",
            };
            if distinct_type == "cost" {
                let mut distinct_costs: std::collections::HashSet<u32> =
                    std::collections::HashSet::new();
                for &cid in &combined {
                    if cid == -1 {
                        continue;
                    }
                    let passes_type = card_type_filter
                        .is_none_or(|f| util::card_matches_type(card_db, cid, Some(f)));
                    let passes_group = group_names.is_none_or(|gn| {
                        gn.iter()
                            .any(|g| util::card_matches_group_str(card_db, cid, Some(g.as_str())))
                    });
                    if !passes_type || !passes_group {
                        continue;
                    }
                    if let Some(card) = card_db.get_card(cid) {
                        let cost = card.cost.unwrap_or(0);
                        let modified_cost = (cost as i32
                            + self.game_state.mods.get_cost_modifier(cid))
                        .max(0) as u32;
                        distinct_costs.insert(modified_cost);
                    }
                }
                let count = distinct_costs.len() as u32;
                compare_counts(operator, count, count_threshold)
            } else {
                let mut distinct_names: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for &cid in &combined {
                    if cid == -1 {
                        log::debug!("[MULTI]   skipping -1");
                        continue;
                    }
                    let passes_type = card_type_filter
                        .is_none_or(|f| util::card_matches_type(card_db, cid, Some(f)));
                    let passes_group = group_names.is_none_or(|gn| {
                        gn.iter()
                            .any(|g| util::card_matches_group_str(card_db, cid, Some(g.as_str())))
                    });
                    log::debug!(
                        "[MULTI]   card={} type_pass={} group_pass={}",
                        cid, passes_type, passes_group
                    );
                    if !passes_type || !passes_group {
                        continue;
                    }
                    let names = card_db.get_card_names(cid);
                    for name in &names {
                        log::debug!("[MULTI]   name='{}'", name);
                        distinct_names.insert(name.clone());
                    }
                }
                let count = distinct_names.len() as u32;
                log::debug!(
                    "[MULTI] distinct_names={} threshold={}",
                    count, count_threshold
                );
                compare_counts(operator, count, count_threshold)
            }
        } else {
            let filter = util::filter_from_parts(
                card_type_filter,
                group_names.and_then(|g| g.first().map(|s| s.as_str())),
                condition.cost_limit,
                operator,
                None,
                None,
                None,
            );
            let matching_count = util::count_matching(&combined, card_db, &filter, false);
            compare_counts(operator, matching_count, count_threshold)
        }
    }

    pub(crate) fn evaluate_position_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        let position = condition
            .position
            .as_ref()
            .and_then(|p| p.get_position())
            .unwrap_or("");
        let player = self.resolve_condition_player(target);
        match position {
            "center" => player.stage.stage[1] != -1,
            "left_side" => player.stage.stage[0] != -1,
            "right_side" => player.stage.stage[2] != -1,
            "any" => {
                player.stage.stage[0] != -1
                    || player.stage.stage[1] != -1
                    || player.stage.stage[2] != -1
            }
            _ => true,
        }
    }

    pub(crate) fn evaluate_group_condition(&self, condition: &Condition) -> bool {
        if condition.all_members.unwrap_or(false) {
            let target = condition.target.as_deref().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            let group_name = condition
                .group_names
                .as_ref()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            let card_db = &self.game_state.card_database;
            return player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .all(|&id| crate::ability::util::card_matches_group_str(card_db, id, group_name));
        }
        // When heart_colors are specified, check that the collective cards in the
        // target zone cover ALL required heart colors (e.g. yell-revealed cards).
        let hc: &[String] = condition
            .heart_colors.as_deref()
            .unwrap_or(&[]);
        if !hc.is_empty() {
            let target = condition.target.as_deref().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            let card_db = &self.game_state.card_database;
            let group_name = condition
                .group_names
                .as_ref()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            let location = condition.location.as_deref().unwrap_or("");
            let source_cards: Vec<i16> = match Zone::from_str(location) {
                Some(Zone::RevealedCards) => self.game_state.revealed_cards.clone(),
                Some(Zone::Stage) => player
                    .stage
                    .stage
                    .iter()
                    .copied()
                    .filter(|&id| id != -1)
                    .collect(),
                _ => return false,
            };
            let required_colors: Vec<crate::card::HeartColor> = hc
                .iter()
                .map(|s| crate::zones::parse_heart_color(s))
                .collect();
            let mut present = std::collections::HashSet::new();
            for &cid in &source_cards {
                if cid == -1 {
                    continue;
                }
                if !crate::ability::util::card_matches_group_str(card_db, cid, group_name) {
                    continue;
                }
                if let Some(ref ct) = condition.card_type {
                    if !crate::ability::util::card_matches_type(card_db, cid, Some(ct)) {
                        continue;
                    }
                }
                if let Some(card) = card_db.get_card(cid) {
                    if let Some(ref bh) = card.base_heart {
                        for color in bh.hearts.keys() {
                            if required_colors.contains(color) {
                                present.insert(*color);
                            }
                        }
                    }
                    if let Some(ref bld) = card.blade_heart {
                        for color in bld.hearts.keys() {
                            if required_colors.contains(color) {
                                present.insert(*color);
                            }
                        }
                    }
                }
            }
            return present.len() >= required_colors.len();
        }
        let mut count = self.get_group_card_count(condition);
        if condition.exclude_self.unwrap_or(false) {
            if let Some(aid) = self.activating_card_id {
                let target = condition.target.as_deref().unwrap_or("self");
                let player = self.resolve_condition_player(target);
                if player.stage.stage.contains(&aid) {
                    count = count.saturating_sub(1);
                }
            }
        }
        let target_count = condition.count.unwrap_or(1);
        let operator = condition.operator.as_deref().or(Some(">="));
        compare_counts(operator, count, target_count)
    }

    pub(crate) fn evaluate_card_count_condition(&self, condition: &Condition) -> bool {
        let card_type = condition.card_type.as_deref().unwrap_or("");
        let target = self.resolve_target_for_scope(condition);
        let is_both = target == "both";
        let count = condition.count.unwrap_or(1);
        let player = self.resolve_condition_player(target);
        let exclude_self = condition.exclude_self.unwrap_or(false);
        let activating_id = self.activating_card_id;
        let card_db = &self.game_state.card_database;

        let location = condition.location.as_deref().unwrap_or("");
        let group_names = condition.group_names.as_ref();
        let hc: &[String] = condition
            .heart_colors.as_deref()
            .unwrap_or(&[]);

        let count_filtered = |zone_source: &[i16], ct: &str| -> usize {
            self.count_cards_with_filters(
                zone_source,
                Some(ct),
                group_names.map(|gn| gn.as_slice()),
                hc,
                condition.cost_limit,
                condition.cost_limit_operator.as_deref(),
                None,
                false,
                condition,
            ) as usize
        };

        if condition.source.as_deref() == Some("preceding_moved")
            || condition.source.as_deref() == Some("previous_moved_cards")
        {
            let card_db = &self.game_state.card_database;
            let negate = condition.negation.unwrap_or(false);
            let wants_blade_heart_prop =
                condition.card_property.as_deref() == Some("has_blade_heart");
            log::debug!(
                "[MOVED_DEBUG] moved_cards={:?} iter_count={}",
                self.moved_cards,
                self.moved_cards.len()
            );
            let actual = self
                .moved_cards
                .iter()
                .filter(|&&cid| {
                    if cid == -1 {
                        return false;
                    }
                    let type_ok = card_type.is_empty()
                        || util::card_matches_type(card_db, cid, Some(card_type));
                    let has_bh = wants_blade_heart_prop
                        && card_db.get_card(cid).is_some_and(|c| c.has_blade_heart());
                    let bh_reject =
                        wants_blade_heart_prop && ((negate && has_bh) || (!negate && !has_bh));
                    log::debug!(
                        "[MFLT] cid={} type={} has_bh={} bh_reject={}",
                        cid, type_ok, has_bh, bh_reject
                    );
                    if !type_ok || bh_reject {
                        return false;
                    }
                    if !hc.is_empty() && !util::card_matches_heart_colors(card_db, cid, hc) {
            
                        return false;
                    }
        
                    true
                })
                .count() as u32;
            // negation for the count comparison only applies when card_property is not
            // driving the per-card filter (handled above). For pure count negation
            // ("ない場合" style), flip the compare result.
            let passed = compare_counts(condition.operator.as_deref(), actual, count);
            let result = if wants_blade_heart_prop {
                // Per-card negation already applied in the filter above
                passed
            } else if negate {
                !passed
            } else {
                passed
            };
            log::debug!(
                "[PRECEDING_MOVED] card_type={:?} prop={:?} hc={:?} negate={} actual={} op={:?} count={} -> {}",
                card_type,
                condition.card_property,
                hc,
                negate,
                actual,
                condition.operator,
                count,
                result
            );
            return result;
        }

        let actual = match Zone::from_str(location) {
            Some(Zone::RevealedCards) => count_filtered(&self.game_state.revealed_cards, card_type),
            Some(Zone::Stage) => {
                let stage_cards: Vec<i16> = if is_both {
                    let opp = self.resolve_condition_player("opponent");
                    let mut combined = player.stage.stage.to_vec();
                    combined.extend_from_slice(&opp.stage.stage);
                    combined
                } else {
                    player.stage.stage.to_vec()
                };
                let is_distinct_cost = condition.distinct.as_ref().is_some_and(
                    |d| matches!(d, crate::core::card::DistinctInfo::String(s) if s == "cost"),
                );
                if is_distinct_cost {
                    // Count the number of distinct modified costs among matching stage members.
                    // The condition "コストがそれぞれ異なるメンバーが3人以上" means:
                    // there are >= 3 members all with unique costs from each other.
                    // Equivalent: # of distinct cost values >= count AND == total matching members.
                    let mut distinct_costs: std::collections::HashSet<u32> =
                        std::collections::HashSet::new();
                    let mut total_matching = 0u32;
                    for &cid in &stage_cards {
                        if cid == -1 {
                            continue;
                        }
                        if !card_type.is_empty()
                            && !util::card_matches_type(card_db, cid, Some(card_type))
                        {
                            continue;
                        }
                        let base = card_db.get_card(cid).and_then(|c| c.cost).unwrap_or(0);
                        let modified = (base as i32 + self.game_state.mods.get_cost_modifier(cid))
                            .max(0) as u32;
                        distinct_costs.insert(modified);
                        total_matching += 1;
                    }
                    // Costs are "all different" only if every card has a unique cost
                    // (i.e., distinct count == total count). We then compare that
                    // distinct count to the threshold.
                    if distinct_costs.len() as u32 == total_matching {
                        distinct_costs.len()
                    } else {
                        // Duplicate costs exist — the group is not "all distinct"
                        0
                    }
                } else if condition.unit.as_deref() == Some("types") {
                    let required_colors: Vec<crate::card::HeartColor> = hc
                        .iter()
                        .map(|s| crate::zones::parse_heart_color(s))
                        .collect();
                    let mut present = std::collections::HashSet::new();
                    for &cid in &stage_cards {
                        if cid == -1 {
                            continue;
                        }
                        if let Some(card) = card_db.get_card(cid) {
                            if let Some(ref bh) = card.base_heart {
                                for color in bh.hearts.keys() {
                                    if required_colors.contains(color) {
                                        present.insert(*color);
                                    }
                                }
                            }
                        }
                    }
                    present.len()
                } else {
                    count_filtered(&stage_cards, card_type)
                }
            }
            Some(Zone::Hand)
            | Some(Zone::Deck)
            | Some(Zone::LiveCardZone)
            | Some(Zone::Energy)
            | Some(Zone::SuccessLiveZone) => {
                if target == "either" || target == "both" {
                    let p1 = count_filtered(
                        util::zone_cards(&self.game_state.player1, location),
                        card_type,
                    );
                    let p2 = count_filtered(
                        util::zone_cards(&self.game_state.player2, location),
                        card_type,
                    );
                    if target == "either" {
                        p1.max(p2)
                    } else {
                        p1 + p2
                    }
                } else {
                    count_filtered(util::zone_cards(player, location), card_type)
                }
            }
            Some(Zone::Discard) | Some(Zone::Waitroom) => {
                if condition.text.contains("手札から") {
                    // Event-based: only count recently-moved cards from hand
                    if let Some(ref moved) = self.game_state.recently_moved_cards {
                        let from_hand =
                            self.game_state.recently_moved_from_zone.as_deref() == Some("hand");
                        if !from_hand {
                            return false;
                        }
                        count_filtered(moved, card_type)
                    } else {
                        0
                    }
                } else {
                    // State-based: count all cards in the zone
                    count_filtered(&player.waitroom.cards, card_type)
                }
            }
            None | Some(_) => match card_type {
                "live_card" => {
                    if is_both {
                        let opp = self.resolve_condition_player("opponent");
                        count_filtered(&player.waitroom.cards, card_type)
                            + count_filtered(&opp.waitroom.cards, card_type)
                    } else {
                        count_filtered(&player.waitroom.cards, card_type)
                    }
                }
                "member_card" => {
                    if condition.aggregate.as_deref() == Some("total") {
                        let bm_flat: std::collections::HashMap<i16, i32> = self
                            .game_state
                            .mods
                            .blade_modifiers
                            .iter()
                            .map(|(&k, e)| (k, e.total()))
                            .collect();
                        let self_blade = player.stage.total_blades(
                            card_db,
                            &bm_flat,
                            &self.game_state.mods.orientation_modifiers,
                        );
                        if is_both {
                            let opp = self.resolve_condition_player("opponent");
                            (self_blade
                                + opp.stage.total_blades(
                                    card_db,
                                    &bm_flat,
                                    &self.game_state.mods.orientation_modifiers,
                                )) as usize
                        } else {
                            self_blade as usize
                        }
                    } else {
                        let stage_slots: Vec<i16> = if is_both {
                            let opp = self.resolve_condition_player("opponent");
                            let mut combined = player.stage.stage.to_vec();
                            combined.extend_from_slice(&opp.stage.stage);
                            combined
                        } else {
                            player.stage.stage.to_vec()
                        };
                        let mut stage_count = stage_slots
                            .iter()
                            .filter(|&&id| {
                                id != -1 && {
                                    if hc.is_empty() {
                                        true
                                    } else {
                                        crate::ability::util::card_matches_heart_colors(
                                            card_db, id, hc,
                                        )
                                    }
                                }
                            })
                            .count();
                        if exclude_self {
                            if let Some(cid) = activating_id {
                                if player.stage.stage.contains(&cid) {
                                    stage_count = stage_count.saturating_sub(1);
                                }
                            }
                        }
                        stage_count
                    }
                }
                "energy_card" => {
                    if is_both {
                        let opp = self.resolve_condition_player("opponent");
                        player.energy_zone.cards.len() + opp.energy_zone.cards.len()
                    } else {
                        player.energy_zone.cards.len()
                    }
                }
                _ => 0,
            },
        } as u32;
        let passed = compare_counts(condition.operator.as_deref(), actual, count);
        let mut dbg = AbDebug::new();
        dbg.condition(condition, actual, count, passed);
        passed
    }

    pub(crate) fn evaluate_card_blade_condition(&self, condition: &Condition) -> bool {
        let count = condition.count.unwrap_or(1);
        let operator = condition.operator.as_deref();
        let source = condition.source.as_deref().unwrap_or("selected_cards");
        let cards: &[i16] = match source {
            "selected_cards" => self.selected_card_ids,
            "preceding_moved" => self.moved_cards,
            _ => self.selected_card_ids,
        };
        if cards.is_empty() {
            return false;
        }
        let card_db = &self.game_state.card_database;
        let mut total_blades = 0i32;
        for &cid in cards {
            let base = card_db.get_card(cid).map(|c| c.blade as i32).unwrap_or(0);
            let modifier = self.game_state.mods.get_blade_modifier(cid);
            total_blades += base + modifier;
        }
        let names: String = cards
            .iter()
            .map(|&cid| {
                card_db
                    .get_card(cid)
                    .map(|c| c.name.as_str())
                    .unwrap_or("?")
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join(", ");
        let op_display = operator.unwrap_or(">=");
        log::debug!(
            "▶ Condition blade {} {} on [{}] → {} {} {} → {}",
            op_display,
            count,
            names,
            total_blades,
            op_display,
            count,
            if util::compare_counts(operator, total_blades.max(0) as u32, count) {
                "PASS"
            } else {
                "FAIL"
            }
        );
        util::compare_counts(operator, total_blades.max(0) as u32, count)
    }

    pub(crate) fn evaluate_appearance_condition(&self, condition: &Condition) -> bool {
        let appearance = condition.appearance.unwrap_or(false);
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let baton_touch_trigger = condition.baton_touch_trigger.unwrap_or(false);
        let player = self.resolve_condition_player(target);

        if baton_touch_trigger {
            if let Some(ref _activating_card) = self.game_state.activating_card {
                if self.game_state.baton_touch_count == 0 {
                    return false;
                }
                if let Some(min_count) = condition.min_baton_touch_count {
                    if self.game_state.baton_touch_count < min_count {
                        return false;
                    }
                }
            } else {
                return false;
            }
        }

        if !baton_touch_trigger && !appearance {
            return false;
        }

        if appearance {
            match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    let stage_ids: Vec<i16> = player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect();
                    if stage_ids.is_empty() {
                        return false;
                    }
                    if condition.all_areas.unwrap_or(false)
                        && player.stage.stage.iter().filter(|&&id| id != -1).count() != 3 {
                            return false;
                        }
                    if let Some(ref groups) = condition.group_names {
                        if !groups.is_empty() {
                            let card_db = &self.game_state.card_database;
                            let all_match = stage_ids.iter().all(|&cid| {
                                groups.iter().any(|g| {
                                    crate::ability::util::card_matches_group_str(
                                        card_db,
                                        cid,
                                        Some(g),
                                    )
                                })
                            });
                            if !all_match {
                                return false;
                            }
                        }
                    }
                    if condition.position.is_none() {
                        if let Some(ref act_pos) = condition.activation_position {
                            let card_id = self.activating_card_id;
                            let passes = act_pos.split(',').any(|p| {
                                let trimmed = p.trim();
                                let idx = match trimmed {
                                    "left" | "left_side" => 0,
                                    "center" => 1,
                                    "right" | "right_side" => 2,
                                    _ => return false,
                                };
                                idx < player.stage.stage.len()
                                    && card_id.is_some()
                                    && player.stage.stage[idx] == card_id.unwrap()
                            });
                            if !passes {
                                return false;
                            }
                        }
                    }

                    if let Some(ref pos) = condition.position {
                        let pos_str = pos.get_position();
                        let pos_idx = match pos_str {
                            Some("left") | Some("leftside") | Some("left_side") => 0,
                            Some("center") | Some("centre") => 1,
                            Some("right") | Some("rightside") | Some("right_side") => 2,
                            _ => {
                                log::debug!("[APPEARANCE] unknown position: {:?}", pos_str);
                                return false;
                            }
                        };

                        let expected = self.activating_card_id;
                        if pos_idx >= player.stage.stage.len()
                            || expected.is_none()
                            || player.stage.stage[pos_idx] != expected.unwrap()
                        {
                            return false;
                        }
                    }
                    if let Some(ref chars) = condition.characters {
                        log::debug!(
                            "[APPEARANCE] checking characters: {:?} against stage_ids={:?}",
                            chars, stage_ids
                        );
                        if chars.is_empty() {
                            return !stage_ids.is_empty();
                        }
                        let stage_card_names: Vec<String> = stage_ids
                            .iter()
                            .filter_map(|&cid| {
                                self.game_state
                                    .card_database
                                    .get_card(cid)
                                    .map(|c| c.name.clone())
                            })
                            .collect();
                        log::debug!("[APPEARANCE] stage card names: {:?}", stage_card_names);
                        let result = chars.iter().all(|name| {
                            stage_card_names
                                .iter()
                                .any(|cname| cname.contains(name.as_str()))
                        });
                        log::debug!("[APPEARANCE] result={}", result);
                        if !result {
                            return false;
                        }
                        if let Some(ref ref_char) = condition.cost_reference_character {
                            let subject = chars[0].as_str();
                            let subject_cost = stage_ids
                                .iter()
                                .filter_map(|&cid| {
                                    let card = self.game_state.card_database.get_card(cid)?;
                                    if card.name.contains(subject) {
                                        card.cost
                                    } else {
                                        None
                                    }
                                })
                                .next();
                            let ref_cost = stage_ids
                                .iter()
                                .filter_map(|&cid| {
                                    let card = self.game_state.card_database.get_card(cid)?;
                                    if card.name.contains(ref_char.as_str()) {
                                        card.cost
                                    } else {
                                        None
                                    }
                                })
                                .next();
                            let op = condition.cost_reference_operator.as_deref().unwrap_or(">");
                            let ok = match (subject_cost, ref_cost) {
                                (Some(sc), Some(rc)) if op == ">" => sc > rc,
                                (Some(sc), Some(rc)) if op == ">=" => sc >= rc,
                                (Some(sc), Some(rc)) if op == "<" => sc < rc,
                                (Some(sc), Some(rc)) if op == "<=" => sc <= rc,
                                _ => false,
                            };
                            log::debug!("[APPEARANCE] cost_compare: subject={} cost={:?} ref={} cost={:?} op={} ok={}",
                                subject, subject_cost, ref_char, ref_cost, op, ok);
                            ok
                        } else {
                            true
                        }
                    } else {
                        log::debug!(
                            "[APPEARANCE] no characters filter, stage_ids={:?}",
                            stage_ids
                        );
                        !stage_ids.is_empty()
                    }
                }
                Some(Zone::Hand) => !player.hand.cards.is_empty(),
                Some(Zone::Discard) => !player.waitroom.cards.is_empty(),
                _ => true,
            }
        } else {
            match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    player.stage.stage[0] == -1
                        && player.stage.stage[1] == -1
                        && player.stage.stage[2] == -1
                }
                Some(Zone::Hand) => player.hand.cards.is_empty(),
                Some(Zone::Discard) => player.waitroom.cards.is_empty(),
                _ => true,
            }
        }
    }

    pub(crate) fn evaluate_ability_filter_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        let card_db = &self.game_state.card_database;
        let player = self.game_state.resolve_target_player(target);
        let filter = condition.ability_filter.as_deref().unwrap_or("no_ability");

        let has_ability = if let Some(card_id) = self.game_state.activating_card {
            if let Some(card) = card_db.get_card(card_id) {
                !card.abilities.is_empty()
            } else {
                false
            }
        } else {
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
            stage_cards.iter().any(|&id| {
                card_db
                    .get_card(id)
                    .map(|c| !c.abilities.is_empty())
                    .unwrap_or(false)
            })
        };

        match filter {
            "no_ability" => !has_ability,
            "has_ability" => has_ability,
            "no_ability_type"
                if has_ability => {
                    !self.card_has_matching_ability_type(condition, filter)
                }
            _ => true,
        }
    }

    pub(crate) fn card_has_matching_ability_type(
        &self,
        condition: &Condition,
        _filter: &str,
    ) -> bool {
        let excluded_triggers: Vec<&str> = condition
            .ability_filter_triggers
            .as_ref()
            .map(|t| t.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        if excluded_triggers.is_empty() {
            return false;
        }
        if let Some(card_id) = self.game_state.activating_card {
            let card_db = &self.game_state.card_database;
            if let Some(card) = card_db.get_card(card_id) {
                card.abilities.iter().any(|a| {
                    a.triggers
                        .as_ref()
                        .map(|t| excluded_triggers.iter().any(|et| t.starts_with(et)))
                        .unwrap_or(false)
                })
            } else {
                false
            }
        } else {
            false
        }
    }

    pub(crate) fn evaluate_ability_filter_condition_with_card_check(
        &self,
        condition: &Condition,
        filter: &str,
    ) -> bool {
        let card_db = &self.game_state.card_database;
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);

        let location = condition
            .location
            .as_deref()
            .unwrap_or(Zone::Stage.to_str());
        let card_ids: Vec<i16> = match Zone::from_str(location) {
            Some(Zone::Stage) => player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect(),
            Some(Zone::Hand) => player.hand.cards.to_vec(),
            Some(Zone::Discard) | Some(Zone::Waitroom) => player.waitroom.cards.to_vec(),
            Some(Zone::Energy) => player.energy_zone.cards.to_vec(),
            Some(Zone::LiveCardZone) => player.live_card_zone.cards.to_vec(),
            _ => {
                if let Some(card_id) = self.game_state.activating_card {
                    vec![card_id]
                } else {
                    return true;
                }
            }
        };

        let operator = condition.operator.as_deref().unwrap_or("any");
        let count_needed = condition.count.unwrap_or(1);

        let match_count = card_ids
            .iter()
            .filter(|&&id| {
                card_db
                    .get_card(id)
                    .map(|c| {
                        let has_ability = !c.abilities.is_empty();
                        match filter {
                            "no_ability" => !has_ability,
                            "has_ability" => has_ability,
                            "no_ability_type"
                                if has_ability => {
                                    let excluded_triggers: Vec<&str> = condition
                                        .ability_filter_triggers
                                        .as_ref()
                                        .map(|t| t.iter().map(|s| s.as_str()).collect())
                                        .unwrap_or_default();
                                    !c.abilities.iter().any(|a| {
                                        a.triggers
                                            .as_ref()
                                            .map(|t| {
                                                excluded_triggers.iter().any(|et| t.starts_with(et))
                                            })
                                            .unwrap_or(false)
                                    })
                                }
                            _ => true,
                        }
                    })
                    .unwrap_or(false)
            })
            .count() as u32;

        match operator {
            "=" => match_count == count_needed,
            ">=" => match_count >= count_needed,
            "<=" => match_count <= count_needed,
            ">" => match_count > count_needed,
            "<" => match_count < count_needed,
            _ => match_count >= count_needed,
        }
    }

    pub(crate) fn zone_len(&self, player: &crate::player::Player, location: &str) -> u32 {
        match Zone::from_str(location) {
            Some(Zone::Stage) => {
                let bm_flat: std::collections::HashMap<i16, i32> = self
                    .game_state
                    .mods
                    .blade_modifiers
                    .iter()
                    .map(|(&k, e)| (k, e.total()))
                    .collect();
                player.stage.total_blades(
                    &self.game_state.card_database,
                    &bm_flat,
                    &self.game_state.mods.orientation_modifiers,
                )
            }
            Some(Zone::Hand) => player.hand.len() as u32,
            Some(Zone::Deck) => player.main_deck.len() as u32,
            Some(Zone::Discard) => player.waitroom.len() as u32,
            Some(Zone::Energy) => player.energy_zone.cards.len() as u32,
            Some(Zone::LiveCardZone) => player.live_card_zone.len() as u32,
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.len() as u32,
            _ => 0,
        }
    }

    pub(crate) fn card_matches_count_filters(
        &self,
        card_id: i16,
        card_type_filter: Option<&str>,
        group_names: Option<&[String]>,
        heart_colors: &[String],
        cost_limit: Option<u32>,
        cost_limit_operator: Option<&str>,
        respect_original_value: bool,
        condition: &Condition,
    ) -> bool {
        // Delegate to the canonical CardFilter from util.rs to avoid
        // maintaining two parallel filtering implementations.
        let mut filter = crate::ability::util::CardFilter::default();
        filter.card_type = card_type_filter;
        filter.group = group_names.and_then(|g| g.first().map(|s| s.as_str()));
        filter.heart_colors = heart_colors;
        filter.cost_limit = cost_limit;
        filter.cost_operator = cost_limit_operator;

        let card_db = &self.game_state.card_database;
        if !filter.matches(card_db, card_id, true) {
            return false;
        }
        if respect_original_value
            && condition.original_value.unwrap_or(false)
            && !self.check_original_blade_filter(condition, card_id)
        {
            return false;
        }
        if respect_original_value
            && condition.original_value.unwrap_or(false)
            && !self.check_original_heart_filter(condition, card_id)
        {
            return false;
        }
        true
    }

    pub(crate) fn count_cards_with_filters(
        &self,
        cards: &[i16],
        card_type_filter: Option<&str>,
        group_names: Option<&[String]>,
        heart_colors: &[String],
        cost_limit: Option<u32>,
        cost_limit_operator: Option<&str>,
        exclude_self: Option<i16>,
        respect_original_value: bool,
        condition: &Condition,
    ) -> u32 {
        // Build a single CardFilter and use its .count() — avoids re-parsing
        // the filter fields for every card in the slice.
        let mut filter = crate::ability::util::CardFilter::default();
        filter.card_type = card_type_filter;
        filter.group = group_names.and_then(|g| g.first().map(|s| s.as_str()));
        filter.heart_colors = heart_colors;
        filter.cost_limit = cost_limit;
        filter.cost_operator = cost_limit_operator;
        if let Some(ex) = exclude_self {
            filter.exclude_self = Some(ex);
        }
        let card_db = &self.game_state.card_database;
        let mut count = 0u32;
        for &card_id in cards {
            if !filter.matches(card_db, card_id, true) {
                continue;
            }
            if respect_original_value
                && condition.original_value.unwrap_or(false)
                && !self.check_original_blade_filter(condition, card_id)
            {
                continue;
            }
            if respect_original_value
                && condition.original_value.unwrap_or(false)
                && !self.check_original_heart_filter(condition, card_id)
            {
                continue;
            }
            count += 1;
        }
        count
    }

    pub(crate) fn sum_group_hearts_in_stage(
        &self,
        player: &crate::player::Player,
        group_name: Option<&str>,
    ) -> u32 {
        let card_db = &self.game_state.card_database;
        player
            .stage
            .stage
            .iter()
            .filter(|&&id| {
                id != -1 && crate::ability::util::card_matches_group_str(card_db, id, group_name)
            })
            .filter_map(|&id| card_db.get_card(id))
            .map(|card| {
                card.base_heart
                    .as_ref()
                    .map(|bh| bh.hearts.values().copied().sum::<u32>())
                    .unwrap_or(0)
            })
            .sum()
    }

    pub(crate) fn count_group_cards_in_cards(
        &self,
        cards: &[i16],
        group_names: Option<&[String]>,
        card_type: Option<&str>,
        exclude_characters: Option<&[String]>,
    ) -> u32 {
        let card_db = &self.game_state.card_database;
        let group_name = group_names.and_then(|g| g.first().map(|s| s.as_str()));
        let count = cards
            .iter()
            .filter(|&&card_id| {
                if card_id == -1 {
                    return false;
                }
                if !crate::ability::util::card_matches_group_str(card_db, card_id, group_name) {
                    return false;
                }
                if let Some(ct) = card_type {
                    if !crate::ability::util::card_matches_type(card_db, card_id, Some(ct)) {
                        return false;
                    }
                }
                if let Some(exc) = exclude_characters {
                    if let Some(card) = card_db.get_card(card_id) {
                        if exc.iter().any(|e| {
                            card.name.contains(e.as_str()) || card.card_no.contains(e.as_str())
                        }) {
                            log::debug!(
                                "[COUNT_EXCLUDE] excluding card {} (name={})",
                                card_id, card.name
                            );
                            return false;
                        }
                    }
                }
                true
            })
            .count() as u32;
        log::debug!(
            "[COUNT_GROUP] zone_count={} group={:?} ct={:?} exc={:?} total={}",
            cards.len(),
            group_name,
            card_type,
            exclude_characters,
            count
        );
        count
    }

    pub(crate) fn count_for_player_target(
        &self,
        player: &crate::player::Player,
        location: &str,
        comparison_type: Option<&str>,
    ) -> u32 {
        match comparison_type {
            Some("score") => {
                let mut total_score = 0;
                let cards: Vec<i16> = match Zone::from_str(location) {
                    Some(Zone::SuccessLiveZone) => player.success_live_card_zone.cards.to_vec(),
                    Some(Zone::LiveCardZone) => player.live_card_zone.cards.to_vec(),
                    None => player
                        .success_live_card_zone
                        .cards
                        .iter()
                        .chain(player.live_card_zone.cards.iter())
                        .copied()
                        .collect::<Vec<i16>>(),
                    _ => Vec::new(),
                };
                for card_id in &cards {
                    if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                        total_score += card.score.unwrap_or(0);
                    }
                }
                total_score
            }
            Some("cost") => {
                let mut total_cost: i32 = 0;
                for card_id in &player.stage.stage {
                    if *card_id != -1 {
                        if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                            total_cost += card.cost.unwrap_or(0) as i32
                                + self.game_state.mods.get_cost_modifier(*card_id);
                        }
                    }
                }
                total_cost.max(0) as u32
            }
            Some("energy") => player.energy_zone.cards.len() as u32,
            _ => self.zone_len(player, location),
        }
    }

    pub(crate) fn get_count_for_condition(&self, condition: &Condition) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let comparison_type = condition.comparison_type.as_deref();
        let resource_type = condition.resource_type.as_deref();
        if comparison_type == Some("score") {
            return self.get_count_for_target(condition, target);
        }
        if comparison_type == Some("cost") {
            if condition.location.is_none() {
                return self
                    .game_state
                    .revealed_cost_cards
                    .iter()
                    .map(|&id| {
                        let base = self
                            .game_state
                            .card_database
                            .get_card(id)
                            .and_then(|c| c.cost)
                            .unwrap_or(0) as i32;
                        (base + self.game_state.mods.get_cost_modifier(id)).max(0) as u32
                    })
                    .sum();
            }
            // comparison_type=cost + comparison_target=self means individual
            // cost comparison ("このメンバーよりコストの大きいメンバー").
            // Return the MAXIMUM cost among matching cards so the caller
            // can compare it against the self card's cost via compare_counts.
            // When comparison_target != "self", the condition uses total sum
            // (handled by count_for_player_target instead).
            if condition.comparison_target.as_deref() == Some("self") {
                let player = self.resolve_condition_player(target);
                let card_db = &self.game_state.card_database;
                let location = condition.location.as_deref().unwrap_or("stage");
                let cards: Vec<i16> = match Zone::from_str(location) {
                    Some(Zone::Stage) => player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect(),
                    Some(Zone::Hand) => player.hand.cards.to_vec(),
                    Some(Zone::Discard) | Some(Zone::Waitroom) => player.waitroom.cards.to_vec(),
                    _ => vec![],
                };
                if cards.is_empty() {
                    return 0;
                }
                let exclude_id = if condition.exclude_self.unwrap_or(false) {
                    self.activating_card_id
                } else {
                    None
                };
                let filter = util::CardFilter {
                    card_type: condition.card_type.as_deref(),
                    group: condition
                        .group_names
                        .as_ref()
                        .and_then(|v| v.first())
                        .map(|s| s.as_str()),
                    groups: condition.group_names.as_ref().map(|v| v),
                    exclude_self: exclude_id,
                    characters: condition.characters.as_ref().map(|v| v),
                    ..util::CardFilter::default()
                };
                let max_cost = cards
                    .iter()
                    .filter(|&&id| filter.matches(card_db, id, false))
                    .filter_map(|&id| {
                        let base = card_db.get_card(id).and_then(|c| c.cost)?;
                        Some(
                            (base as i32 + self.game_state.mods.get_cost_modifier(id)).max(0)
                                as u32,
                        )
                    })
                    .max()
                    .unwrap_or(0);
                return max_cost;
            }
        }
        if resource_type == Some("hand_count") {
            let player = self.resolve_condition_player(target);
            return player.hand.len() as u32;
        }
        if let Some(rt) = resource_type {
            if rt.starts_with("heart") && rt.len() == 7 {
                let color = crate::zones::parse_heart_color(rt);
                let player = self.resolve_condition_player(target);
                let card_db = &self.game_state.card_database;
                let count: u32 = player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .map(|&id| {
                        card_db
                            .get_card(id)
                            .and_then(|c| c.base_heart.as_ref())
                            .map(|bh| bh.hearts.get(&color).copied().unwrap_or(0))
                            .unwrap_or(0)
                    })
                    .sum();
                return count;
            }
        }
        if resource_type == Some("surplus_heart") {
            // After live clearance the computed surplus count is stored on GameState.
            // Prefer the stored snapshot value over a runtime recalculation
            // (includes yell ALL hearts that can fill any color gap).
            if self.game_state.live_surplus_ready_this_turn {
                return match target {
                    "opponent" => self.game_state.opponent_live_surplus_count,
                    _ => self.game_state.self_live_surplus_count,
                };
            }
            // Fallback: compute from current state (before live clearance or if snapshot
            // unavailable). Supports color-specific surplus via condition.heart_colors.
            let player = self.resolve_condition_player(target);
            let card_db = &self.game_state.card_database;

            if let Some(ref colors) = condition.heart_colors {
                let mut total = 0u32;
                for hc_str in colors {
                    let color = crate::zones::parse_heart_color(hc_str);
                    let member_of_color: u32 = player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .map(|&id| {
                            card_db
                                .get_card(id)
                                .and_then(|c| c.base_heart.as_ref())
                                .and_then(|bh| bh.hearts.get(&color))
                                .copied()
                                .unwrap_or(0)
                        })
                        .sum();
                    let needed_of_color: u32 = player
                        .live_card_zone
                        .cards
                        .iter()
                        .chain(player.success_live_card_zone.cards.iter())
                        .map(|&id| {
                            card_db
                                .get_card(id)
                                .and_then(|c| c.need_heart.as_ref())
                                .and_then(|nh| nh.hearts.get(&color))
                                .copied()
                                .unwrap_or(0)
                        })
                        .sum();
                    total += member_of_color.saturating_sub(needed_of_color);
                }
                return total;
            }

            let member_hearts: u32 = player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            let needed: u32 = player
                .live_card_zone
                .cards
                .iter()
                .chain(player.success_live_card_zone.cards.iter())
                .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            return member_hearts.saturating_sub(needed);
        }
        if resource_type == Some("energy") {
            let player = self.resolve_condition_player(target);
            let count = player.energy_zone.cards.len() as u32;
            log::debug!(
                "[GET_COUNT] resource_type=energy target={} → {}",
                target, count
            );
            return count;
        }
        let player = self.resolve_condition_player(target);
        self.zone_len(player, location)
    }

    pub(crate) fn get_count_for_target(&self, condition: &Condition, target: &str) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let resource_type = condition.resource_type.as_deref();
        let comparison_type = condition.comparison_type.as_deref();
        if resource_type == Some("energy") {
            let player = self.resolve_condition_player(target);
            return player.energy_zone.cards.len() as u32;
        }
        let player = self.resolve_condition_player(target);
        let mut count = self.count_for_player_target(player, location, comparison_type);
        if count == 0 && location.is_empty() {
            if let Some(ref locs) = condition.locations {
                let mut combined: Vec<i16> = Vec::new();
                for loc in locs {
                    let cards = crate::ability::util::zone_cards(player, loc.as_str());
                    combined.extend_from_slice(cards);
                }
                let card_db = &self.game_state.card_database;
                let card_type_filter = condition.card_type.as_deref();
                let filter = crate::ability::util::filter_from_parts(
                    card_type_filter,
                    condition
                        .group_names
                        .as_ref()
                        .and_then(|g| g.first().map(|s| s.as_str())),
                    condition.cost_limit,
                    condition.operator.as_deref(),
                    None,
                    None,
                    None,
                );
                count = crate::ability::util::count_matching(&combined, card_db, &filter, false);
            }
        }
        count
    }

    pub(crate) fn get_group_card_count(&self, condition: &Condition) -> u32 {
        let group_filter = condition.group_names.as_ref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let group_name = group_filter.and_then(|g| g.first().map(|s| s.as_str()));

        let is_aggregate = condition.aggregate.as_deref() == Some("total");

        if is_aggregate {
            return self.sum_group_hearts_in_stage(player, group_name);
        }

        let ct = condition.card_type.as_deref();
        let exc = condition.exclude_characters.as_deref();
        match Zone::from_str(location) {
            Some(Zone::Stage) => self.count_group_cards_in_cards(
                &player.stage.stage,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            Some(Zone::Hand) => self.count_group_cards_in_cards(
                &player.hand.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            Some(Zone::Discard) | Some(Zone::Waitroom) => self.count_group_cards_in_cards(
                &player.waitroom.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            Some(Zone::LiveCardZone) => self.count_group_cards_in_cards(
                &player.live_card_zone.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            Some(Zone::SuccessLiveZone) => self.count_group_cards_in_cards(
                &player.success_live_card_zone.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            Some(Zone::Energy) => self.count_group_cards_in_cards(
                &player.energy_zone.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            _ => 0,
        }
    }

    /// Evaluate resource_condition: check resource count (e.g. total blades)
    /// against the operator and count threshold.
    /// Used by parser issue #10 where "blade total >= 10" should be a resource condition.
    pub(crate) fn evaluate_resource_condition(&self, condition: &Condition) -> bool {
        let resource = condition.resource_type.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let card_db = &self.game_state.card_database;
        let location = condition.location.as_deref().unwrap_or("stage");
        let op = condition.operator.as_deref();
        let threshold = condition.count.unwrap_or(1);

        let count = match resource {
            "blade" => {
                let card_ids: Vec<i16> = match Zone::from_str(location) {
                    Some(Zone::Stage) => player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect(),
                    _ => vec![],
                };
                let bm_flat: std::collections::HashMap<i16, i32> = self
                    .game_state
                    .mods
                    .blade_modifiers
                    .iter()
                    .map(|(&k, e)| (k, e.total()))
                    .collect();
                card_ids
                    .iter()
                    .map(|&cid| {
                        let base = card_db.get_card(cid).map(|c| c.blade as i32).unwrap_or(0);
                        let modifier = bm_flat.get(&cid).copied().unwrap_or(0);
                        (base + modifier).max(0) as u32
                    })
                    .sum()
            }
            "surplus_heart" => {
                // Count surplus hearts for the target player
                let heart_total: u32 = player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
                    .sum();
                let need: u32 = player
                    .live_card_zone
                    .cards
                    .iter()
                    .chain(player.success_live_card_zone.cards.iter())
                    .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
                    .sum();
                heart_total.saturating_sub(need)
            }
            _ => 0,
        };
        util::compare_counts(op, count, threshold)
    }
}
