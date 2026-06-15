use super::debug::AbDebug;
use super::util;
use super::util::compare_counts;
use crate::card::Condition;
use crate::game_state::Phase;

fn comparison_default_count(condition: &Condition) -> u32 {
    if condition.location.is_some() || condition.card_type.is_some() {
        1
    } else {
        0
    }
}

fn stage_has_any_member(player: &crate::player::Player) -> bool {
    player.stage.stage.iter().any(|&id| id != -1)
}

/// Read-only context for evaluating ability conditions.
/// Extracted from AbilityResolver to reduce the god-struct surface.
pub struct ConditionContext<'a> {
    pub game_state: &'a crate::game_state::GameState,
    pub activating_card_id: Option<i16>,
    /// Cards moved by the most recent `move_cards` effect within the same sequential chain.
    /// Used for `source: "preceding_moved"` conditions.
    pub moved_cards: &'a [i16],
    /// Cards selected by the most recent `select` effect within the same sequential chain.
    /// Used for conditions checking properties of the selected card.
    pub selected_card_ids: &'a [i16],
}

impl<'a> ConditionContext<'a> {
    pub fn new(game_state: &'a crate::game_state::GameState) -> Self {
        let activating_card_id = game_state.activating_card;
        ConditionContext {
            game_state,
            activating_card_id,
            moved_cards: &[],
            selected_card_ids: &[],
        }
    }

    pub fn with_moved_cards(
        game_state: &'a crate::game_state::GameState,
        moved_cards: &'a [i16],
    ) -> Self {
        let activating_card_id = game_state.activating_card;
        ConditionContext {
            game_state,
            activating_card_id,
            moved_cards,
            selected_card_ids: &[],
        }
    }

    pub fn with_moved_and_selected(
        game_state: &'a crate::game_state::GameState,
        moved_cards: &'a [i16],
        selected_card_ids: &'a [i16],
    ) -> Self {
        let activating_card_id = game_state.activating_card;
        ConditionContext {
            game_state,
            activating_card_id,
            moved_cards,
            selected_card_ids,
        }
    }
}

impl<'a> ConditionContext<'a> {
    fn resolve_condition_player(&self, target: &str) -> &crate::player::Player {
        if target == "self" {
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

    fn evaluate_condition_list(&self, conditions: &[Condition], operator: &str) -> (usize, bool) {
        let mut passed = 0usize;
        let mut all_pass = true;
        let mut any_pass = false;
        for condition in conditions {
            let result = self.evaluate_condition(condition);
            if result {
                passed += 1;
            } else {
                all_pass = false;
            }
            any_pass |= result;
        }
        let final_result = match operator {
            "and" => all_pass,
            "or" => any_pass,
            _ => true,
        };
        (passed, final_result)
    }

    fn no_excess_heart_flag(&self, target: &str) -> bool {
        match target {
            "opponent" => self.game_state.opponent_live_no_excess_heart_this_turn,
            _ => self.game_state.self_no_excess_heart_this_turn,
        }
    }

    fn any_of_matches(&self, condition_type: &str) -> bool {
        let master = &*self.game_state.resolve_target_player("self");
        match condition_type {
            "has_member" => stage_has_any_member(master),
            "has_energy" => !master.energy_zone.cards.is_empty(),
            "has_hand" => !master.hand.cards.is_empty(),
            "has_blade_heart" => master.stage.stage.iter().any(|&id| {
                id != crate::constants::EMPTY_SLOT
                    && self
                        .game_state
                        .card_database
                        .get_card(id)
                        .map(|c| c.has_blade_heart())
                        .unwrap_or(false)
            }),
            "has_live_card" => !master.live_card_zone.cards.is_empty(),
            "is_active_phase" => matches!(
                self.game_state.current_phase,
                crate::game_state::Phase::Active
            ),
            "is_main_phase" => matches!(
                self.game_state.current_phase,
                crate::game_state::Phase::Main
            ),
            _ => {
                eprintln!("Unknown any_of condition type: {}", condition_type);
                false
            }
        }
    }

    /// Resolve the effective target for a condition, respecting scope.
    /// When scope is "both" (parser emits this for "自分と相手" / self and opponent),
    /// override target from "self" to "both" so both players' data is combined.
    fn resolve_target_for_scope<'b>(&self, condition: &'b Condition) -> &'b str {
        let target = condition.target.as_deref().unwrap_or("self");
        if target == "self" && condition.scope.as_deref() == Some("both") {
            "both"
        } else {
            target
        }
    }

    pub fn evaluate_condition(&self, condition: &Condition) -> bool {
        let mut dbg = AbDebug::new();
        let ct = condition.condition_type.as_deref().unwrap_or("?");
        let result = match ct {
            "compound" => self.evaluate_compound_condition(condition),
            "comparison_condition" => self.evaluate_comparison_condition(condition),
            "location_condition" => self.evaluate_location_condition(condition),
            "card_count_condition" => self.evaluate_card_count_condition(condition),
            "card_blade_condition" => self.evaluate_card_blade_condition(condition),
            "group_condition" => self.evaluate_group_condition(condition),
            "position_condition" => self.evaluate_position_condition(condition),
            "appearance_condition" => self.evaluate_appearance_condition(condition),
            "temporal_condition" => self.evaluate_temporal_condition(condition),
            "state_condition" => self.evaluate_state_condition(condition),
            "energy_state_condition" => self.evaluate_energy_state_condition(condition),
            "movement_condition" => self.evaluate_movement_condition(condition),
            "ability_filter_condition" => self.evaluate_ability_filter_condition(condition),
            "or_condition" => self.evaluate_or_condition(condition),
            "any_of_condition" => self.evaluate_any_of_condition(condition),
            "score_threshold_condition" => self.evaluate_score_threshold_condition(condition),
            "choice_condition" => self.evaluate_choice_condition(condition),
            "position_change_condition" => self.evaluate_position_change_condition(condition),
            "state_change_condition" => self.evaluate_state_change_condition(condition),
            "opponent_choice_condition" => self.evaluate_opponent_choice_condition(condition),
            "opponent_live_success" => self.evaluate_opponent_live_success_condition(condition),
            "complex_condition" => self.evaluate_complex_condition(condition),
            "no_excess_heart" => self.evaluate_no_excess_heart_condition(condition),
            "otherwise_condition" => true, // "otherwise" = catch-all, always true when reached
            _ => false,
        };

        let final_result = if condition.negation.unwrap_or(false) {
            !result
        } else {
            result
        };
        let thresh = if ct == "comparison_condition" {
            condition.count.unwrap_or(0)
        } else {
            1
        };
        let dbg_actual = if result {
            condition.count.unwrap_or(1)
        } else {
            0
        };
        dbg.condition(condition, dbg_actual, thresh, final_result);

        // Check ability_filter field on any condition type
        if let Some(ref filter) = condition.ability_filter {
            let filtered =
                self.evaluate_ability_filter_condition_with_card_check(condition, filter);
            if !filtered {
                return false;
            }
        }

        final_result
    }

    fn evaluate_compound_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            let mut dbg = AbDebug::new();
            dbg.p(
                "COMPOUND",
                format_args!(
                    "{} sub-conditions, operator={}",
                    conditions.len(),
                    condition.operator.as_deref().unwrap_or("and")
                ),
            );
            let op = condition.operator.as_deref().unwrap_or("and");
            let (passed_count, all_pass) = self.evaluate_condition_list(conditions, op);
            dbg.p(
                "COMPOUND",
                format_args!(
                    "→ {}/{} passed = {}",
                    passed_count,
                    conditions.len(),
                    if all_pass { "PASS" } else { "FAIL" }
                ),
            );
            all_pass
        } else {
            eprintln!("[COMPOUND] no conditions array!");
            true
        }
    }

    fn evaluate_comparison_condition(&self, condition: &Condition) -> bool {
        // If position is "front", check the opponent member at the front area
        // of the activating card, and compare its cost to this card's cost.
        if let Some(ref pos) = condition.position {
            if pos.get_position() == Some("front") {
                return self.evaluate_front_comparison(condition);
            }
        }

        let count = self.get_count_for_condition(condition);

        if let Some(ref values) = condition.values {
            return values.contains(&(count as u32));
        }

        // For score conditions, the count field is the score threshold, not a card count.
        // Always use condition.count if present; fall back to comparison_default_count for
        // non-score conditions (e.g. card_count_condition type).
        let target_count = if let Some(ref comparison_target) = condition.comparison_target {
            if comparison_target == "opponent" {
                self.get_count_for_target(condition, "opponent")
            } else if condition.comparison_type.as_deref() == Some("score")
                || condition.comparison_type.as_deref() == Some("cost")
            {
                condition.count.unwrap_or(0)
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

        // Handle cost_total: sum costs of cards in the target location
        if condition.cost_total.is_some() {
            let total = condition.cost_total.unwrap_or(0);
            let operator = condition
                .cost_total_operator
                .as_deref()
                .or(condition.operator.as_deref())
                .unwrap_or("=");
            let target = condition.target.as_deref().unwrap_or("self");
            let player = self.game_state.resolve_target_player(target);
            let location = condition.location.as_deref().unwrap_or("stage");
            let card_ids: Vec<i16> = match location {
                "stage" => player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .copied()
                    .collect(),
                "hand" => player.hand.cards.to_vec(),
                "discard" | "waitroom" => player.waitroom.cards.to_vec(),
                _ => vec![],
            };
            let sum_cost: u32 = card_ids
                .iter()
                .filter_map(|&id| {
                    self.game_state
                        .card_database
                        .get_card(id)
                        .and_then(|c| c.cost)
                })
                .sum();
            return compare_counts(Some(operator), sum_cost, total);
        }

        compare_counts(condition.operator.as_deref(), count, target_count)
    }

    fn evaluate_front_comparison(&self, condition: &Condition) -> bool {
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
            .unwrap_or(0) as u32;
        let front_cost = gs
            .card_database
            .get_card(front_card_id)
            .and_then(|c| c.cost)
            .unwrap_or(0) as u32;

        compare_counts(condition.operator.as_deref(), front_cost, master_cost)
    }

    fn check_heart_type_all(&self, condition: &Condition, player: &crate::player::Player) -> bool {
        if condition.heart_type.as_deref() != Some("all") {
            return true;
        }
        let card_db = &self.game_state.card_database;
        player.stage.stage.iter().any(|&id| {
            id != -1
                && card_db.get_card(id).map_or(false, |c| {
                    c.base_heart.as_ref().map_or(false, |bh| {
                        bh.hearts.contains_key(&crate::card::HeartColor::Heart00)
                    })
                })
        })
    }

    fn check_heart_colors(&self, condition: &Condition, player: &crate::player::Player) -> bool {
        let cols = match &condition.heart_colors {
            Some(c) if !c.is_empty() => c,
            _ => return true,
        };
        let card_db = &self.game_state.card_database;
        cols.iter().all(|cs| {
            player.stage.stage.iter().any(|&id| {
                id != -1
                    && card_db.get_card(id).map_or(false, |c| {
                        c.base_heart.as_ref().map_or(false, |bh| {
                            bh.hearts.contains_key(&crate::zones::parse_heart_color(cs))
                        })
                    })
            })
        })
    }

    fn check_has_blade_heart(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        if condition.card_property.as_deref() != Some("has_blade_heart") {
            return true;
        }
        let card_db = &self.game_state.card_database;
        let check_cards: Vec<i16> = match location {
            "revealed_cards" => self.game_state.revealed_cards.iter().copied().collect(),
            "stage" => player
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
            .any(|&id| card_db.get_card(id).map_or(false, |c| c.has_blade_heart()))
    }

    fn check_baton_touch(&self, condition: &Condition) -> bool {
        if !condition.baton_touch_trigger.unwrap_or(false) {
            return true;
        }
        if self.game_state.baton_touch_count == 0 {
            return false;
        }
        // Check minimum baton touch count (e.g. 2 for double baton touch)
        if let Some(min_count) = condition.min_baton_touch_count {
            if self.game_state.baton_touch_count < min_count {
                return false;
            }
        }
        let card_db = &self.game_state.card_database;
        // Check group_names: the card that was replaced (baton-touched from) must belong to the specified group
        if let Some(ref groups) = condition.group_names {
            if !groups.is_empty() {
                if let Some(replaced_id) = self.game_state.baton_touch_replaced_member_id {
                    let group_ok = groups.iter().any(|g| {
                        crate::ability::util::card_matches_group_str(&card_db, replaced_id, Some(g))
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
                .any(|&id| card_db.get_card(id).map_or(false, |c| c.name.contains(src)))
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

    fn check_aggregate_total(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> Option<bool> {
        if condition.aggregate.as_deref() == Some("total") && location == "stage" {
            Some(compare_counts(
                condition.operator.as_deref(),
                player.stage.total_blades(
                    &self.game_state.card_database,
                    &self.game_state.mods.blade_modifiers,
                    &self.game_state.mods.orientation_modifiers,
                ),
                condition.count.unwrap_or(0),
            ))
        } else {
            None
        }
    }

    fn check_distinct_names(&self, condition: &Condition, player: &crate::player::Player) -> bool {
        if !condition.distinct.unwrap_or(false) {
            return true;
        }
        let card_db = &self.game_state.card_database;
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
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
        true
    }

    fn check_no_excess_heart(
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
                                .map_or(false, |bh| !bh.hearts.is_empty())
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

    fn evaluate_location_condition(&self, condition: &Condition) -> bool {
        if let Some(ref locs) = condition.locations {
            if locs.len() >= 2 {
                // Both-target equality comparison (e.g. P1 success == P2 success)
                if condition.target.as_deref() == Some("both")
                    && condition.comparison_type.as_deref() == Some("equality")
                {
                    return compare_counts(
                        condition.operator.as_deref(),
                        self.get_count_for_target(condition, "self"),
                        self.get_count_for_target(condition, "opponent"),
                    );
                }
                // Either-target: true if ANY matching card in either location
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
                // Otherwise delegate to multi-location handler (counts/distinct across zones)
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

        // Handle "both" target with equality comparison for single-location conditions
        if is_both && condition.comparison_type.as_deref() == Some("equality") {
            let self_count = self.get_count_for_target(condition, "self");
            let opp_count = self.get_count_for_target(condition, "opponent");
            return compare_counts(condition.operator.as_deref(), self_count, opp_count);
        }

        // Main count path (inlined from calculate_location_value)
        let group = condition
            .group_names
            .as_ref()
            .and_then(|g| g.first().map(|s| s.as_str()));
        let op = condition.operator.as_deref();

        // When scope is "both", combine cards from both players
        let cards: Vec<i16> = if is_both {
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
                &|cid| self.check_original_blade_filter(condition, cid),
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

    fn check_original_blade_filter(&self, condition: &Condition, card_id: i16) -> bool {
        if !condition.original_value.unwrap_or(false) {
            return true;
        }
        if let Some(op) = condition.operator.as_deref() {
            let threshold = condition.count.unwrap_or(0) as u32;
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

    fn evaluate_multi_location_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        // When target is "both" and comparison_type is "equality", compare P1 vs P2 counts
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
        eprintln!("[MULTI] combined {} cards", combined.len());

        if condition.distinct.unwrap_or(false) {
            let mut distinct_names: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for &cid in &combined {
                if cid == -1 {
                    eprintln!("[MULTI]   skipping -1");
                    continue;
                }
                let passes_type = card_type_filter
                    .map_or(true, |f| util::card_matches_type(card_db, cid, Some(f)));
                let passes_group = group_names.map_or(true, |gn| {
                    card_db
                        .get_card(cid)
                        .map(|c| {
                            gn.iter()
                                .any(|g| c.group == *g || c.unit.as_deref() == Some(g.as_str()))
                        })
                        .unwrap_or(false)
                });
                eprintln!(
                    "[MULTI]   card={} type_pass={} group_pass={}",
                    cid, passes_type, passes_group
                );
                if !passes_type || !passes_group {
                    continue;
                }
                let names = card_db.get_card_names(cid);
                for name in &names {
                    eprintln!("[MULTI]   name='{}'", name);
                    distinct_names.insert(name.clone());
                }
            }
            let count = distinct_names.len() as u32;
            eprintln!(
                "[MULTI] distinct_names={} threshold={}",
                count, count_threshold
            );
            compare_counts(operator, count, count_threshold)
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

    fn evaluate_position_condition(&self, condition: &Condition) -> bool {
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

    fn evaluate_group_condition(&self, condition: &Condition) -> bool {
        // When all_members is true, every card on stage must match the group ("のみ")
        if condition.all_members.unwrap_or(false) {
            let target = condition.target.as_deref().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            let group_name = condition
                .group_names
                .as_ref()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            let card_db = &self.game_state.card_database;
            // Every non-empty stage slot must match the group
            return player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .all(|&id| crate::ability::util::card_matches_group_str(card_db, id, group_name));
        }
        let mut count = self.get_group_card_count(condition);
        // Exclude self (for "ほかのメンバー" / "other members" patterns)
        if condition.exclude_self.unwrap_or(false) {
            if let Some(aid) = self.activating_card_id {
                let target = condition.target.as_deref().unwrap_or("self");
                let player = self.resolve_condition_player(target);
                if player.stage.stage.contains(&aid) {
                    count = count.saturating_sub(1);
                }
            }
        }
        // Default to 1 when count is not set (checking "at least one matching")
        let target_count = condition.count.unwrap_or(1);
        let operator = condition.operator.as_deref().or(Some(">="));
        compare_counts(operator, count, target_count)
    }

    fn evaluate_card_count_condition(&self, condition: &Condition) -> bool {
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
            .heart_colors
            .as_ref()
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Helper: count cards in a zone filtered by type + heart + group
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

        // Explicit source: preceding_moved — check against the resolver's moved_cards
        // (set by a prior move_cards action in the same sequential chain)
        if condition.source.as_deref() == Some("preceding_moved") {
            let actual = count_filtered(&self.moved_cards, card_type) as u32;
            return compare_counts(condition.operator.as_deref(), actual, count);
        }

        // Resolve location to an actual card list
        let actual = match location {
            "revealed_cards" => count_filtered(&self.game_state.revealed_cards, card_type),
            "stage" => {
                let stage_cards: Vec<i16> = if is_both {
                    let opp = self.resolve_condition_player("opponent");
                    let mut combined = player.stage.stage.to_vec();
                    combined.extend_from_slice(&opp.stage.stage);
                    combined
                } else {
                    player.stage.stage.to_vec()
                };
                if condition.unit.as_deref() == Some("types") {
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
                                for (color, _) in &bh.hearts {
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
            "hand"
            | "deck"
            | "live_card_zone"
            | "energy_zone"
            | "success_live_zone"
            | "success_live_card_zone" => {
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
            "discard" | "waitroom" => {
                // Check recently moved cards first (chained from a preceding move_cards action)
                if let Some(ref moved) = self.game_state.recently_moved_cards {
                    count_filtered(moved, card_type)
                } else {
                    count_filtered(&player.waitroom.cards, card_type)
                }
            }
            // No explicit location: infer from card_type (don't use recently_moved_cards
            // as fallback — that breaks sequential conditions where an earlier action moved
            // cards but the later condition still refers to the stage)
            "" => match card_type {
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
                        let self_blade = player.stage.total_blades(
                            card_db,
                            &self.game_state.mods.blade_modifiers,
                            &self.game_state.mods.orientation_modifiers,
                        );
                        if is_both {
                            let opp = self.resolve_condition_player("opponent");
                            (self_blade
                                + opp.stage.total_blades(
                                    card_db,
                                    &self.game_state.mods.blade_modifiers,
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
                                if player.stage.stage.iter().any(|&id| id == cid) {
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
            _ => 0,
        } as u32;
        let passed = compare_counts(condition.operator.as_deref(), actual, count);
        let mut dbg = AbDebug::new();
        dbg.condition(condition, actual, count, passed);
        passed
    }

    fn evaluate_card_blade_condition(&self, condition: &Condition) -> bool {
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
        eprintln!(
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

    fn evaluate_appearance_condition(&self, condition: &Condition) -> bool {
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
            match location {
                "stage" => {
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
                    // Check activation_position from effect (e.g. "left_side,right_side")
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

                    // Check position constraint (e.g. "左サイドに登場した場合")
                    if let Some(ref pos) = condition.position {
                        let pos_str = pos.get_position();
                        let pos_idx = match pos_str {
                            Some("left") | Some("leftside") | Some("left_side") => 0,
                            Some("center") | Some("centre") => 1,
                            Some("right") | Some("rightside") | Some("right_side") => 2,
                            _ => {
                                eprintln!("[APPEARANCE] unknown position: {:?}", pos_str);
                                return false;
                            }
                        };

                        // Check that the ACTIVATING CARD is at this position, not just any card
                        let expected = self.activating_card_id;
                        if pos_idx >= player.stage.stage.len()
                            || expected.is_none()
                            || player.stage.stage[pos_idx] != expected.unwrap()
                        {
                            return false;
                        }
                    }
                    if let Some(ref chars) = condition.characters {
                        eprintln!(
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
                        eprintln!("[APPEARANCE] stage card names: {:?}", stage_card_names);
                        let result = chars.iter().all(|name| {
                            stage_card_names
                                .iter()
                                .any(|cname| cname.contains(name.as_str()))
                        });
                        eprintln!("[APPEARANCE] result={}", result);
                        if !result {
                            return false;
                        }
                        // Check character cost comparison (「A」よりコストの大きい「B」)
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
                            eprintln!("[APPEARANCE] cost_compare: subject={} cost={:?} ref={} cost={:?} op={} ok={}",
                                subject, subject_cost, ref_char, ref_cost, op, ok);
                            ok
                        } else {
                            true
                        }
                    } else {
                        eprintln!(
                            "[APPEARANCE] no characters filter, stage_ids={:?}",
                            stage_ids
                        );
                        !stage_ids.is_empty()
                    }
                }
                "hand" => !player.hand.cards.is_empty(),
                "discard" => !player.waitroom.cards.is_empty(),
                _ => true,
            }
        } else {
            match location {
                "stage" => {
                    player.stage.stage[0] == -1
                        && player.stage.stage[1] == -1
                        && player.stage.stage[2] == -1
                }
                "hand" => player.hand.cards.is_empty(),
                "discard" => player.waitroom.cards.is_empty(),
                _ => true,
            }
        }
    }

    fn evaluate_temporal_condition(&self, condition: &Condition) -> bool {
        let temporal = condition.temporal.as_deref().unwrap_or("");
        let phase = condition.phase.as_deref();

        match temporal {
            "this_turn" => {
                if let Some(count) = condition.count {
                    if condition.location.as_deref() == Some("stage")
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
                        match nested_condition.condition_type.as_deref() {
                            Some("not_moved") => {
                                if let Some(activating_card_id) = self.activating_card_id {
                                    !self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else {
                                    true
                                }
                            }
                            Some("has_moved") => {
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
                // Check turn_number constraint (e.g. "このゲームの1ターン目のライブフェイズの場合")
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

    fn evaluate_state_condition(&self, condition: &Condition) -> bool {
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

    fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        let energy_state = condition.energy_state.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        match energy_state {
            "active" => player.energy_zone.active_count() > 0,
            _ => true,
        }
    }

    fn evaluate_movement_condition(&self, condition: &Condition) -> bool {
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
                            // "has_moved" = position change (area move), not deploy/discard.
                            // The activating card must have been involved in a movement.
                            if !card_moved {
                                return false;
                            }
                            let is_position_change =
                                self.game_state.position_change_occurred_this_turn;
                            // If text mentions 登場 (e.g. "登場か、エリアを移動"), also allow
                            // deploy/appearance — but only if the card is on stage (not discarded).
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
                    // Bare "moved" — area move without specific state.
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
                // Check cost_limit on moved cards
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
                // Verify replaced card's location (e.g. "waitroom" for sent-to-discard
                // trigger). "stage" is always satisfied for baton touch deploy (the replaced
                // card was on stage before the swap), so skip that check.
                if let Some(ref loc) = condition.location {
                    if loc.as_str() == "discard" || loc.as_str() == "waitroom" {
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
                // Check exclude_self: if true, the activating card must NOT be the replaced card
                if condition.exclude_self.unwrap_or(false) {
                    if self.game_state.activating_card == Some(replaced_id) {
                        return false;
                    }
                }
                // Check group_names: replaced card must belong to specified group
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
                // Check baton_touch_source: replaced card's name must match
                if let Some(source_name) = condition.baton_touch_source.as_deref() {
                    if let Some(card) = self.game_state.card_database.get_card(replaced_id) {
                        if !card.name.contains(source_name) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                // Check cost_limit on the replaced card (absolute threshold, e.g. cost >= 10)
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
                // Check cost comparison (e.g. "lower cost than this member")
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
                // OR logic: at least one trigger condition must be met.
                // The ability fires on EITHER area move OR energy placement (「か」).
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
                // Determine which checks are active based on whether the condition's
                // self_effect_only / energy_placed fields are actually specified.
                // None = unspecified (check is not active), Some(true) = check is active.
                let has_area_check = condition.self_effect_only.is_some();
                let has_energy_check = condition.energy_placed.is_some();
                if !has_area_check && !has_energy_check {
                    // Neither check configured → any movement counts (bare "moves")
                    true
                } else if has_area_check && has_energy_check {
                    area_ok || energy_ok
                } else if has_area_check {
                    area_ok
                } else {
                    energy_ok
                }
            }
            _ => match location {
                "stage" => {
                    player.stage.stage[0] != -1
                        || player.stage.stage[1] != -1
                        || player.stage.stage[2] != -1
                }
                "hand" => !player.hand.cards.is_empty(),
                "discard" => !player.waitroom.cards.is_empty(),
                _ => true,
            },
        }
    }

    fn evaluate_ability_filter_condition(&self, condition: &Condition) -> bool {
        // Check if target card(s) have ability text or not
        let target = condition.target.as_deref().unwrap_or("self");
        let card_db = &self.game_state.card_database;
        let player = self.game_state.resolve_target_player(target);
        let filter = condition.ability_filter.as_deref().unwrap_or("no_ability");

        // Check the activating card's abilities
        let has_ability = if let Some(card_id) = self.game_state.activating_card {
            if let Some(card) = card_db.get_card(card_id) {
                !card.abilities.is_empty()
            } else {
                false
            }
        } else {
            // Check all cards on the player's stage for collective ability presence
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
            "no_ability_type" => {
                if has_ability {
                    !self.card_has_matching_ability_type(condition, filter)
                } else {
                    true
                }
            }
            _ => true,
        }
    }

    fn card_has_matching_ability_type(&self, condition: &Condition, _filter: &str) -> bool {
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

    fn evaluate_ability_filter_condition_with_card_check(
        &self,
        condition: &Condition,
        filter: &str,
    ) -> bool {
        // For conditions that target a specific location, check cards in that location
        let card_db = &self.game_state.card_database;
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);

        let location = condition.location.as_deref().unwrap_or("stage");
        let card_ids: Vec<i16> = match location {
            "stage" => player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect(),
            "hand" => player.hand.cards.to_vec(),
            "discard" | "waitroom" => player.waitroom.cards.to_vec(),
            "energy_zone" => player.energy_zone.cards.to_vec(),
            "live_card_zone" => player.live_card_zone.cards.to_vec(),
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
                            "no_ability_type" => {
                                if has_ability {
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
                                } else {
                                    true
                                }
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

    fn evaluate_or_condition(&self, condition: &Condition) -> bool {
        if let Some(ref conditions) = condition.conditions {
            self.evaluate_condition_list(conditions, "or").1
        } else {
            true
        }
    }

    fn evaluate_any_of_condition(&self, condition: &Condition) -> bool {
        if let Some(ref any_of) = condition.any_of {
            any_of
                .iter()
                .any(|condition_type| self.any_of_matches(condition_type))
        } else {
            true
        }
    }

    fn evaluate_score_threshold_condition(&self, condition: &Condition) -> bool {
        // Default to 1 when count is not set (checking "score exists")
        let count = condition.count.unwrap_or(1);
        let operator = condition.operator.as_deref();
        let target = condition.target.as_deref().unwrap_or("self");
        let cheer_count = if target == "self" {
            self.game_state.player1_cheer_blade_heart_count
        } else if target == "opponent" {
            self.game_state.player2_cheer_blade_heart_count
        } else {
            self.game_state.player1_cheer_blade_heart_count
        };
        util::compare_counts(operator, cheer_count, count)
    }

    fn evaluate_choice_condition(&self, condition: &Condition) -> bool {
        if let Some(ref options) = condition.options {
            !options.is_empty()
        } else {
            true
        }
    }

    fn evaluate_position_change_condition(&self, condition: &Condition) -> bool {
        let optional = condition.options.as_ref().map(|_| true).unwrap_or(false);
        if optional {
            if self.game_state.position_change_occurred_this_turn {
                return true;
            }
            return false;
        }
        self.game_state.position_change_occurred_this_turn
    }

    fn evaluate_state_change_condition(&self, condition: &Condition) -> bool {
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
            // Check if any member on the target player's stage has the target orientation
            let _orientation_check = |card_id: i16| -> bool {
                let o = self.game_state.mods.get_orientation_modifier(card_id);
                match (from, to) {
                    ("active", "wait") => o.map_or(false, |s| s == "wait"),
                    ("wait", "active") => o.map_or(true, |s| s == "active"),
                    _ => false,
                }
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

    fn evaluate_opponent_choice_condition(&self, condition: &Condition) -> bool {
        let _target = condition.target.as_deref().unwrap_or("opponent");
        let negation = condition.negation.unwrap_or(false);
        let opponent_declined = self.game_state.opponent_choice_declined;
        if negation {
            opponent_declined
        } else {
            !opponent_declined
        }
    }

    fn evaluate_opponent_live_success_condition(&self, condition: &Condition) -> bool {
        if !self.game_state.opponent_live_success_this_turn {
            return false;
        }
        // If no_excess_heart is set, also check the stored flag
        if condition.no_excess_heart.unwrap_or(false) {
            return self.no_excess_heart_flag("opponent");
        }
        true
    }

    fn evaluate_no_excess_heart_condition(&self, condition: &Condition) -> bool {
        let target = condition.target.as_deref().unwrap_or("self");
        self.no_excess_heart_flag(target)
    }

    fn evaluate_complex_condition(&self, condition: &Condition) -> bool {
        if let Some(ref cause) = condition.cause {
            self.evaluate_condition(cause)
        } else {
            true
        }
    }

    fn zone_len(&self, player: &crate::player::Player, location: &str) -> u32 {
        match location {
            "stage" => player.stage.total_blades(
                &self.game_state.card_database,
                &self.game_state.mods.blade_modifiers,
                &self.game_state.mods.orientation_modifiers,
            ),
            "hand" => player.hand.len() as u32,
            "deck" => player.main_deck.len() as u32,
            "discard" => player.waitroom.len() as u32,
            "energy_zone" => player.energy_zone.cards.len() as u32,
            "live_card_zone" => player.live_card_zone.len() as u32,
            "success_live_zone" => player.success_live_card_zone.len() as u32,
            _ => 0,
        }
    }

    fn card_matches_count_filters(
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
        if card_id == -1 {
            return false;
        }
        let card_db = &self.game_state.card_database;
        let type_ok = match card_type_filter {
            Some("live_card") => card_db
                .get_card(card_id)
                .map(|c| c.is_live())
                .unwrap_or(false),
            Some("member_card") => card_db
                .get_card(card_id)
                .map(|c| c.is_member())
                .unwrap_or(false),
            Some("energy_card") => card_db
                .get_card(card_id)
                .map(|c| c.is_energy())
                .unwrap_or(false),
            _ => true,
        };
        if !type_ok {
            return false;
        }
        if !heart_colors.is_empty()
            && !crate::ability::util::card_matches_heart_colors(card_db, card_id, heart_colors)
        {
            return false;
        }
        if let Some(groups) = group_names {
            if groups.is_empty() {
                return false;
            }
            let first_group = groups.first().map(|s| s.as_str());
            if !crate::ability::util::card_matches_group_str(card_db, card_id, first_group) {
                return false;
            }
        }
        if !crate::ability::util::card_matches_cost_limit_op(
            card_db,
            card_id,
            cost_limit,
            cost_limit_operator,
        ) {
            return false;
        }
        if respect_original_value
            && condition.original_value.unwrap_or(false)
            && !self.check_original_blade_filter(condition, card_id)
        {
            return false;
        }
        true
    }

    fn count_cards_with_filters(
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
        cards
            .iter()
            .filter(|&&card_id| {
                exclude_self.map_or(true, |ex| card_id != ex)
                    && self.card_matches_count_filters(
                        card_id,
                        card_type_filter,
                        group_names,
                        heart_colors,
                        cost_limit,
                        cost_limit_operator,
                        respect_original_value,
                        condition,
                    )
            })
            .count() as u32
    }

    fn sum_group_hearts_in_stage(
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

    fn count_group_cards_in_cards(
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
                if let Some(ref exc) = exclude_characters {
                    if let Some(card) = card_db.get_card(card_id) {
                        if exc.iter().any(|e| {
                            card.name.contains(e.as_str()) || card.card_no.contains(e.as_str())
                        }) {
                            eprintln!(
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
        eprintln!(
            "[COUNT_GROUP] zone_count={} group={:?} ct={:?} exc={:?} total={}",
            cards.len(),
            group_name,
            card_type,
            exclude_characters,
            count
        );
        count
    }

    fn count_for_player_target(
        &self,
        player: &crate::player::Player,
        location: &str,
        comparison_type: Option<&str>,
    ) -> u32 {
        match comparison_type {
            Some("score") => {
                let mut total_score = 0;
                // Only score from the specified location; if no location, score both.
                let cards: Vec<i16> = match location {
                    "success_live_zone" | "success_live_card_zone" => {
                        player.success_live_card_zone.cards.to_vec()
                    }
                    "live_card_zone" => player.live_card_zone.cards.to_vec(),
                    "" => player
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
                let mut total_cost = 0;
                for card_id in &player.stage.stage {
                    if *card_id != -1 {
                        if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                            total_cost += card.cost.unwrap_or(0);
                        }
                    }
                }
                total_cost
            }
            Some("energy") => player.energy_zone.cards.len() as u32,
            _ => self.zone_len(player, location),
        }
    }

    fn get_count_for_condition(&self, condition: &Condition) -> u32 {
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let comparison_type = condition.comparison_type.as_deref();
        let resource_type = condition.resource_type.as_deref();
        if comparison_type == Some("score") {
            return self.get_count_for_target(condition, target);
        }
        if comparison_type == Some("cost") && condition.location.is_none() {
            return self
                .game_state
                .revealed_cost_cards
                .iter()
                .filter_map(|&id| self.game_state.card_database.get_card(id))
                .filter_map(|c| c.cost)
                .sum();
        }
        if resource_type == Some("hand_count") {
            let player = self.resolve_condition_player(target);
            return player.hand.len() as u32;
        }
        if let Some(rt) = resource_type {
            if rt.starts_with("heart") && rt.len() == 7 {
                // Count specific heart icons on stage members (e.g. heart_02)
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
            let player = self.resolve_condition_player(target);
            let card_db = &self.game_state.card_database;
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
            eprintln!(
                "[GET_COUNT] resource_type=energy target={} → {}",
                target, count
            );
            return count;
        }
        let player = self.resolve_condition_player(target);
        self.zone_len(player, location)
    }

    fn get_count_for_target(&self, condition: &Condition, target: &str) -> u32 {
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
                    let cards = super::util::zone_cards(player, loc.as_str());
                    combined.extend_from_slice(cards);
                }
                let card_db = &self.game_state.card_database;
                let card_type_filter = condition.card_type.as_deref();
                let filter = super::util::filter_from_parts(
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
                count = super::util::count_matching(&combined, card_db, &filter, false);
            }
        }
        count
    }

    fn get_group_card_count(&self, condition: &Condition) -> u32 {
        let group_filter = condition.group_names.as_ref();
        let location = condition.location.as_deref().unwrap_or("");
        let target = condition.target.as_deref().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let group_name = group_filter.and_then(|g| g.first().map(|s| s.as_str()));

        let is_aggregate = condition.aggregate.as_deref() == Some("total");

        // When aggregate="total", sum heart values instead of counting cards
        if is_aggregate {
            return self.sum_group_hearts_in_stage(player, group_name);
        }

        let ct = condition.card_type.as_deref();
        let exc = condition.exclude_characters.as_ref().map(|v| v.as_slice());
        match location {
            "stage" => self.count_group_cards_in_cards(
                &player.stage.stage,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            "hand" => self.count_group_cards_in_cards(
                &player.hand.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            "discard" | "waitroom" => self.count_group_cards_in_cards(
                &player.waitroom.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            "live_card_zone" => self.count_group_cards_in_cards(
                &player.live_card_zone.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            "success_live_card_zone" | "success_live_zone" => self.count_group_cards_in_cards(
                &player.success_live_card_zone.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            "energy_zone" => self.count_group_cards_in_cards(
                &player.energy_zone.cards,
                group_filter.map(|v| &**v),
                ct,
                exc,
            ),
            _ => 0,
        }
    }
}
