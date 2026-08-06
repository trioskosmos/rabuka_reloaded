use super::comparison_default_count;
use super::ConditionContext;
use crate::ability::debug::AbDebug;
use crate::ability::enums::Zone;
use crate::ability::util;
use crate::ability::util::compare_counts;
use crate::ability_queue::ConditionalChoice;
use crate::card::{
    AbilityFilter, CardProperty, CardState, ComparisonTarget, Condition, HeartColor,
};
use crate::{HashMap, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeartTotal {
    None,
    All,
    Value(u8),
}

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
                        || p1.success_live_card_zone.cards.contains(&cid)
                    {
                        Some(p1)
                    } else if p2.stage.stage.contains(&cid)
                        || p2.hand.cards.contains(&cid)
                        || p2.live_card_zone.cards.contains(&cid)
                        || p2.energy_zone.cards.contains(&cid)
                        || p2.success_live_card_zone.cards.contains(&cid)
                    {
                        Some(p2)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| self.game_state.resolve_target_player(target))
        } else if target == "both" {
            // For "both" scope in comparison conditions (comparing self vs opponent),
            // resolve to the activating card's owner (self). The comparison_target
            // field handles the opponent side of the comparison.
            self.self_player
                .unwrap_or_else(|| self.game_state.resolve_target_player("self"))
        } else {
            self.game_state.resolve_target_player(target)
        }
    }

    pub(crate) fn resolve_target_for_scope<'b>(&self, condition: &'b Condition) -> &'b str {
        let target = condition.get_target().unwrap_or("self");
        if target == "self" && condition.get_scope() == Some("both") {
            "both"
        } else {
            target
        }
    }

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
        for &val in values {
            let found = cards.iter().any(|&cid| {
                self.game_state
                    .card_database
                    .get_card(cid)
                    .and_then(|c| c.score)
                    .map_or(false, |s| s == val)
            });
            if !found {
                return false;
            }
        }
        true
    }

    pub(crate) fn evaluate_comparison_condition(&self, condition: &Condition) -> bool {
        if let Some(ref pos) = condition.get_position() {
            if pos.get_position() == Some("front") {
                return self.evaluate_front_comparison(condition);
            }
            // Position-based cost check: "センターエリアにコスト9以上のメンバー"
            if condition.get_comparison_type() == Some("cost") {
                if let Some(position) = pos.get_position() {
                    let target = condition.get_target().unwrap_or("self");
                    let player = self.resolve_condition_player(target);
                    let card_db = &self.game_state.card_database;
                    if let Some(card_id) = util::card_at_position(player, position) {
                        // Apply group filter if specified (e.g. Aqours)
                        if let Some(ref groups) = condition.get_group_names() {
                            if !util::card_matches_any_group(card_db, card_id, groups) {
                                return false;
                            }
                        }
                        // Apply card type filter if specified (e.g. member_card)
                        if let Some(ref ct) = condition.get_card_type() {
                            if !util::card_matches_type(card_db, card_id, Some(ct)) {
                                return false;
                            }
                        }
                        let card_cost = self
                            .game_state
                            .card_database
                            .get_card(card_id)
                            .and_then(|c| c.cost)
                            .unwrap_or(0);
                        // Handle comparison_target: compare self vs opponent at same position
                        if condition.get_comparison_target() == Some(ComparisonTarget::Opponent) {
                            let opponent = self.game_state.resolve_target_player("opponent");
                            let opp_card_id = util::card_at_position(opponent, position);
                            let opp_cost = opp_card_id
                                .and_then(|id| self.game_state.card_database.get_card(id))
                                .and_then(|c| c.cost)
                                .unwrap_or(0);
                            let operator = condition.get_operator().unwrap_or(">=");
                            return util::compare_counts(Some(operator), card_cost, opp_cost);
                        }
                        let threshold = condition.get_count().unwrap_or(0);
                        let operator = condition.get_operator().unwrap_or(">=");
                        return util::compare_counts(Some(operator), card_cost, threshold);
                    }
                    // No card at the specified position → condition fails
                    return false;
                }
            }
        }

        let count = self.get_count_for_condition(condition);

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
            return values.contains(&{ count });
        }

        let target_count =
            if let Some(ref comparison_target) = condition.get_comparison_target() {
                if *comparison_target == ComparisonTarget::Opponent {
                    self.get_count_for_target(condition, "opponent")
                } else if *comparison_target == ComparisonTarget::Self_
                    && (condition.get_comparison_type() == Some("cost")
                        || condition.get_comparison_type() == Some("score"))
                {
                    let target = condition.get_target().unwrap_or("self");
                    let player = self.game_state.resolve_target_player(target);
                    if let Some(act_id) = self.activating_card_id {
                        let ctype = condition.get_comparison_type().unwrap_or("cost");
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
                                // Per Q129 (qa_data.json:2199-2200): cost conditions use
                                // the CURRENT/MODIFIED cost (NOT base/printed).  Hand-based
                                // cost reductions ("手札にあるこのメンバーカードのコストは
                                // ...少なくなる") lower the cost used for condition checks.
                                // This is distinct from "元々のコスト" which checks the
                                // printed cost directly via cost_threshold_met().
                                if ctype == "cost" {
                                    base.saturating_sub(
                                        self.game_state.mods.get_cost_modifier(act_id).max(0) as u8,
                                    )
                                } else {
                                    base
                                }
                            })
                            .unwrap_or_else(|| self.get_count_for_target(condition, target))
                    } else {
                        self.get_count_for_target(condition, target)
                    }
                } else if condition.get_resource_type().is_some() {
                    self.get_count_for_target(condition, comparison_target.as_str())
                } else {
                    condition
                        .get_count()
                        .unwrap_or(comparison_default_count(condition))
                }
            } else if condition.get_comparison_type() == Some("cost") {
                let entry_choice = self.game_state.ability_queue.current_entry().and_then(|e| {
                    match &e.conditional_choice {
                        Some(ConditionalChoice::Str(s)) => s.parse::<u8>().ok(),
                        _ => None,
                    }
                });
                condition
                    .get_cost_limit()
                    .or(condition.get_count())
                    .or(entry_choice)
                    .unwrap_or(0)
            } else if condition.get_comparison_type() == Some("score") {
                condition.get_count().unwrap_or(0)
            } else {
                let entry_choice = self.game_state.ability_queue.current_entry().and_then(|e| {
                    match &e.conditional_choice {
                        Some(ConditionalChoice::Str(s)) => s.parse::<u8>().ok(),
                        _ => None,
                    }
                });
                entry_choice
                    .or(condition.get_count())
                    .unwrap_or(comparison_default_count(condition))
            };

        if condition.get_cost_total().is_some() {
            let total = condition.get_cost_total().unwrap_or(0);
            let operator = condition
                .get_cost_total_operator()
                .map(|o| o.as_str())
                .or(condition.get_operator())
                .unwrap_or("=");
            let card_db = &self.game_state.card_database;
            let target = condition.get_target().unwrap_or("self");
            let sum_cost: i32 = if condition.get_source() == Some("preceding_moved")
                || condition.get_source() == Some("previous_moved_cards")
                || condition.get_location().is_none()
            {
                // When no explicit location, or source is preceding_moved,
                // sum costs from moved_cards (cards moved by preceding cost/effect).
                self.moved_cards
                    .iter()
                    .filter(|&&id| {
                        if let Some(ref groups) = condition.get_group_names() {
                            if !groups.is_empty()
                                && !groups
                                    .iter()
                                    .any(|g| util::card_matches_group_str(card_db, id, Some(g)))
                            {
                                return false;
                            }
                        }
                        if let Some(ref ct) = condition.get_card_type() {
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
                card_ids
                    .iter()
                    .filter(|&&id| {
                        if let Some(ref groups) = condition.get_group_names() {
                            if !groups.is_empty()
                                && !groups
                                    .iter()
                                    .any(|g| util::card_matches_group_str(card_db, id, Some(g)))
                            {
                                return false;
                            }
                        }
                        if let Some(ref ct) = condition.get_card_type() {
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
            return compare_counts(Some(operator), sum_cost.max(0) as u8, total);
        }

        let result = compare_counts(condition.get_operator(), count, target_count);
        // When card_type filters on revealed_cards yield zero matching cards,
        // the condition fails — no revealed card matched the required type.
        // Skip this when revealed_cards is empty (re-evaluation after state
        // change, where compound.rs already handled the original check).
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
        super::push_cond_verdict(
            condition,
            &format!("実際={}, 期待={}{}", count, op_str, target_count),
            final_result,
            vec![],
        );
        final_result
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

        compare_counts(condition.get_operator(), front_cost, master_cost)
    }

    pub(crate) fn evaluate_all_cost_comparison_condition(&self, condition: &Condition) -> bool {
        let gs = self.game_state;
        let self_player = gs.resolve_target_player("self");
        let opp_player = gs.resolve_target_player("opponent");
        let card_db = &gs.card_database;

        let get_stage_costs = |player: &crate::player::Player| -> Vec<u8> {
            player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .map(|&id| {
                    let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
                    (base + gs.mods.get_cost_modifier(id)).max(0) as u8
                })
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

    /// "センターエリアにいるメンバーが最も大きいコストを持つ"
    /// Checks if the card at `position` has strictly the highest cost among all
    /// stage members (excluding empty slots and the card itself).
    pub(crate) fn evaluate_highest_cost_on_stage_condition(&self, condition: &Condition) -> bool {
        let position = match condition.get_position().and_then(|p| p.get_position()) {
            Some(p) => p,
            None => return false,
        };
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let card_db = &self.game_state.card_database;

        let card_at_pos = match util::card_at_position(player, position) {
            Some(id) => id,
            None => return false,
        };

        let pos_cost = {
            let base = card_db
                .get_card(card_at_pos)
                .and_then(|c| c.cost)
                .unwrap_or(0) as i32;
            (base + self.game_state.mods.get_cost_modifier(card_at_pos)).max(0) as u8
        };

        let operator = condition.get_operator().unwrap_or(">");

        for &other_id in &player.stage.stage {
            if other_id == -1 || other_id == card_at_pos {
                continue;
            }
            let other_cost = {
                let base = card_db.get_card(other_id).and_then(|c| c.cost).unwrap_or(0) as i32;
                (base + self.game_state.mods.get_cost_modifier(other_id)).max(0) as u8
            };
            if !compare_counts(Some(operator), pos_cost, other_cost) {
                return false;
            }
        }
        true
    }

    pub(crate) fn check_heart_type_all(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        if condition.get_heart_type() != Some("all") {
            return true;
        }
        if Zone::from_str(location) != Some(Zone::Stage) {
            return true;
        }
        let card_db = &self.game_state.card_database;
        let mods = &self.game_state.mods;
        if condition.get_negation().unwrap_or(false) {
            // Negated: check whether the specific triggering member lacks all-heart.
            // When triggered via each_time, use the triggering_member_id from the
            // queue entry to check only "そのメンバー" (that specific member).
            // Falls back to ALL stage cards if no specific member is targeted.
            let target_ids: Vec<i16> = self
                .game_state
                .ability_queue
                .current_entry()
                .and_then(|e| e.triggering_member_id)
                .map(|id| vec![id])
                .unwrap_or_else(|| {
                    player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .copied()
                        .collect()
                });
            return target_ids.iter().any(|&id| {
                if id == -1 {
                    return false;
                }
                let has_all = card_db.get_card(id).is_some_and(|c| {
                    c.base_heart
                        .as_ref()
                        .is_some_and(|bh| bh.hearts.contains_key(&crate::card::HeartColor::Heart00))
                }) || mods
                    .heart_modifiers
                    .get(&id)
                    .and_then(|m| m.get(&crate::card::HeartColor::All))
                    .map_or(false, |e| e.total() > 0)
                    || mods
                        .constant_heart_bonuses
                        .get(&id)
                        .and_then(|cols| cols.get("all"))
                        .copied()
                        .unwrap_or(0)
                        > 0;
                !has_all
            });
        }
        player.stage.stage.iter().any(|&id| {
            if id == -1 {
                return false;
            }
            // Check base_heart for Heart00 (wildcard base — rare but valid)
            if card_db.get_card(id).is_some_and(|c| {
                c.base_heart
                    .as_ref()
                    .is_some_and(|bh| bh.hearts.contains_key(&crate::card::HeartColor::Heart00))
            }) {
                return true;
            }
            // Check heart_modifiers for HeartColor::All
            if mods
                .heart_modifiers
                .get(&id)
                .and_then(|m| m.get(&crate::card::HeartColor::All))
                .map_or(false, |e| e.total() > 0)
            {
                return true;
            }
            // Check constant_heart_bonuses for "all" key
            if mods
                .constant_heart_bonuses
                .get(&id)
                .and_then(|cols| cols.get("all"))
                .copied()
                .unwrap_or(0)
                > 0
            {
                return true;
            }
            false
        })
    }

    /// Per-card check: does this card match the `heart_type: "all"` filter?
    /// Returns true if heart_type is not "all" (skip filter).
    /// When negated, returns true for cards WITHOUT all-heart.
    pub(crate) fn check_heart_type_all_per_card(
        &self,
        condition: &Condition,
        card_db: &crate::card::CardDatabase,
        card_id: i16,
    ) -> bool {
        if condition.get_heart_type() != Some("all") {
            return true;
        }
        let negate = condition.get_negation().unwrap_or(false);
        let mods = &self.game_state.mods;
        let has_all = card_db.get_card(card_id).is_some_and(|c| {
            c.base_heart
                .as_ref()
                .is_some_and(|bh| bh.hearts.contains_key(&crate::card::HeartColor::Heart00))
        }) || mods
            .heart_modifiers
            .get(&card_id)
            .and_then(|m| m.get(&crate::card::HeartColor::All))
            .map_or(false, |e| e.total() > 0)
            || mods
                .constant_heart_bonuses
                .get(&card_id)
                .and_then(|cols| cols.get("all"))
                .copied()
                .unwrap_or(0)
                > 0;
        if negate {
            !has_all
        } else {
            has_all
        }
    }

    pub(crate) fn check_heart_colors(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        let colors_binding = condition.get_heart_colors();
        let cols = match &colors_binding {
            Some(c) if !c.is_empty() => *c,
            _ => return true,
        };
        // heart00 is a wildcard — skip check since no card has it in base_heart
        if cols
            .iter()
            .any(|cs| crate::card::parse_heart_color(cs) == crate::card::HeartColor::Heart00)
        {
            return true;
        }
        // This check (heart colors present in base_heart) only applies to the stage.
        // Non-stage zones like live_card_zone use need_heart; check_aggregate_total
        // handles those separately with proper zone awareness.
        if Zone::from_str(location) != Some(Zone::Stage) {
            return true;
        }
        let card_db = &self.game_state.card_database;
        cols.iter().all(|cs| {
            player.stage.stage.iter().any(|&id| {
                id != -1
                    && card_db.get_card(id).is_some_and(|c| {
                        c.base_heart.as_ref().is_some_and(|bh| {
                            bh.hearts.contains_key(&crate::card::parse_heart_color(cs))
                        })
                    })
            })
        })
    }

    pub(crate) fn check_card_property(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        let prop = match condition.get_card_property() {
            Some(CardProperty::HasBladeHeart) => "has_blade_heart",
            Some(CardProperty::HasScoreIcon) => "has_score_icon",
            Some(CardProperty::HasAllBlade) => "has_all_blade",
            _ => return true,
        };
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
        let has_prop = |id: i16| -> bool {
            let c = card_db.get_card(id);
            match prop {
                "has_blade_heart" => c.is_some_and(|c| c.has_blade_heart()),
                "has_score_icon" => c.is_some_and(|c| c.has_score_icon()),
                "has_all_blade" => c.is_some_and(|c| c.has_all_blade()),
                _ => false,
            }
        };
        check_cards.iter().any(|&id| has_prop(id)) != condition.get_negation().unwrap_or(false)
    }

    pub(crate) fn check_baton_touch(&self, condition: &Condition) -> bool {
        if !condition.get_baton_touch_trigger().unwrap_or(false) {
            return true;
        }
        // Resolve the player to check baton touch count for
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let player_id = player.id.as_str();
        if self.game_state.get_baton_touch_count(player_id) == 0 {
            return false;
        }
        if let Some(min_count) = condition.get_min_baton_touch_count() {
            if self.game_state.get_baton_touch_count(player_id) < min_count {
                return false;
            }
        }
        let card_db = &self.game_state.card_database;
        if let Some(ref groups) = condition.get_group_names() {
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
        if let Some(src) = condition.get_baton_touch_source() {
            if !self
                .game_state
                .player1
                .waitroom
                .cards
                .iter()
                .chain(self.game_state.player2.waitroom.cards.iter())
                .any(|&id| {
                    card_db.get_card(id).is_some_and(|c| {
                        crate::card::CardDatabase::normalize_name(&c.name)
                            .contains(&crate::card::CardDatabase::normalize_name(src))
                    })
                })
            {
                return false;
            }
        }
        if condition.get_comparison_type() == Some("cost") {
            if let Some(replaced) = self.game_state.baton_touch_replaced_member_cost {
                if let Some(act) = self.game_state.activating_card {
                    if let Some(c) = card_db.get_card(act) {
                        if let Some(cc) = c.cost {
                            if !compare_counts(condition.get_operator(), replaced, cc) {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        true
    }

    pub(crate) fn check_ability_filter(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        let filter = match condition.get_ability_filter() {
            Some(f) => f,
            None => return true,
        };
        let card_db = &self.game_state.card_database;
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
            _ => return true,
        };
        let triggers: Vec<&str> = condition
            .get_ability_filter_triggers()
            .as_ref()
            .map(|t| t.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        match filter {
            AbilityFilter::HasAbility => {
                if triggers.is_empty() {
                    card_ids.iter().any(|&id| {
                        card_db
                            .get_card(id)
                            .is_some_and(|c| !c.abilities.is_empty())
                    })
                } else {
                    card_ids.iter().any(|&id| {
                        card_db.get_card(id).is_some_and(|c| {
                            c.abilities.iter().any(|ar| {
                                ar.resolve()
                                    .triggers
                                    .as_ref()
                                    .is_some_and(|t| triggers.iter().any(|et| t.contains(et)))
                            })
                        })
                    })
                }
            }
            AbilityFilter::NoAbility => card_ids
                .iter()
                .any(|&id| card_db.get_card(id).is_some_and(|c| c.abilities.is_empty())),
            _ => true,
        }
    }

    pub(crate) fn check_aggregate_total(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> Option<bool> {
        if condition.get_aggregate() != Some("total") {
            return None;
        }
        let hc: &[String] = condition.get_heart_colors().unwrap_or(&[]);
        if !hc.is_empty() {
            let card_db = &self.game_state.card_database;
            let card_type = condition
                .get_card_type()
                .map(|ct| ct.as_str())
                .unwrap_or("");
            let group_name: Option<&str> = condition
                .get_group_names()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    let total_heart: u8 = player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&cid| cid != -1)
                        .filter(|&&cid| {
                            card_type.is_empty()
                                || util::card_matches_type(card_db, cid, Some(card_type))
                        })
                        .filter(|&&cid| {
                            group_name.is_none()
                                || util::card_matches_group_str(card_db, cid, group_name)
                        })
                        .map(|&cid| (cid, card_db.get_card(cid)))
                        .filter_map(|(cid, card)| card.map(|c| (cid, c)))
                        .map(|(cid, card)| {
                            let base: u8 = card
                                .base_heart
                                .as_ref()
                                .map(|bh| {
                                    hc.iter()
                                        .map(|color_str| {
                                            let color = crate::card::parse_heart_color(color_str);
                                            bh.hearts.get(&color).copied().unwrap_or(0) as u8
                                        })
                                        .sum::<u8>()
                                })
                                .unwrap_or(0);
                            let modifier: i32 = hc
                                .iter()
                                .map(|color_str| {
                                    let color = crate::card::parse_heart_color(color_str);
                                    self.game_state
                                        .mods
                                        .heart_modifiers
                                        .get(&cid)
                                        .and_then(|hm| hm.get(&color))
                                        .map(|e| e.set as i32 + e.additive as i32)
                                        .unwrap_or(0)
                                })
                                .sum();
                            (base as i32 + modifier).max(0) as u8
                        })
                        .sum();
                    Some(compare_counts(
                        condition.get_operator(),
                        total_heart,
                        condition.get_count().unwrap_or(0),
                    ))
                }
                Some(Zone::LiveCardZone) | Some(Zone::SuccessLiveZone) => {
                    let mut cards: Vec<i16> = player.live_card_zone.cards.to_vec();
                    cards.extend(player.success_live_card_zone.cards.iter().copied());
                    // If an operator is set (e.g. >=), sum all heart colors and compare
                    if let Some(op) = condition.get_operator() {
                        let total_need: u8 = cards
                            .iter()
                            .filter(|&&cid| {
                                card_type.is_empty()
                                    || util::card_matches_type(card_db, cid, Some(card_type))
                            })
                            .filter(|&&cid| {
                                group_name.is_none()
                                    || util::card_matches_group_str(card_db, cid, group_name)
                            })
                            .filter_map(|&cid| card_db.get_card(cid))
                            .map(|card| {
                                card.need_heart
                                    .as_ref()
                                    .map(|nh| {
                                        hc.iter()
                                            .map(|color_str| {
                                                let color =
                                                    crate::card::parse_heart_color(color_str);
                                                nh.hearts.get(&color).copied().unwrap_or(0) as u8
                                            })
                                            .sum::<u8>()
                                    })
                                    .unwrap_or(0)
                            })
                            .sum();
                        Some(compare_counts(
                            Some(op),
                            total_need,
                            condition.get_count().unwrap_or(0),
                        ))
                    } else {
                        // No operator: check each color individually across all cards
                        let threshold = condition.get_count().unwrap_or(1) as u8;
                        let all_ok = hc.iter().all(|color_str| {
                            let color = crate::card::parse_heart_color(color_str);
                            let total: u8 = cards
                                .iter()
                                .filter(|&&cid| {
                                    card_type.is_empty()
                                        || util::card_matches_type(card_db, cid, Some(card_type))
                                })
                                .filter(|&&cid| {
                                    group_name.is_none()
                                        || util::card_matches_group_str(card_db, cid, group_name)
                                })
                                .filter_map(|&cid| card_db.get_card(cid))
                                .map(|card| {
                                    card.need_heart
                                        .as_ref()
                                        .map(|nh| nh.hearts.get(&color).copied().unwrap_or(0) as u8)
                                        .unwrap_or(0)
                                })
                                .sum();
                            total >= threshold
                        });
                        Some(all_ok)
                    }
                }
                _ => None,
            }
        } else if Zone::from_str(location) == Some(Zone::Stage) {
            Some(compare_counts(
                condition.get_operator(),
                player.stage.total_blades(
                    &self.game_state.card_database,
                    &self.game_state.mods.blade_modifiers,
                    &self.game_state.mods.orientation_modifiers,
                    true,
                ),
                condition.get_count().unwrap_or(0),
            ))
        } else {
            None
        }
    }

    pub(crate) fn check_distinct_names(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        location: &str,
    ) -> bool {
        let is_distinct = condition.get_distinct().is_some_and(|d| d.is_distinct());
        if !is_distinct {
            return true;
        }
        let card_db = &self.game_state.card_database;

        let group = condition
            .get_group_names()
            .and_then(|g| g.first().map(|s| s.as_str()));
        let mut cards: Vec<i16> = match Zone::from_str(location) {
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
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.cards.to_vec(),
            Some(Zone::RevealedCards) => self.game_state.revealed_cards.to_vec(),
            _ => return true,
        };
        // If a group filter is specified, only check distinctness within that group
        if let Some(g) = group {
            cards.retain(|&cid| util::card_matches_group_str(card_db, cid, Some(g)));
        }

        let distinct_type = match condition.get_distinct() {
            Some(crate::core::card::DistinctInfo::String(s)) => s.as_str(),
            _ => "card_name",
        };
        let distinct_only = condition.get_count().map(|c| c as usize);

        let pass = match distinct_type {
            "cost" => {
                let mut seen_costs: HashSet<u8> = HashSet::default();
                for &cid in cards.iter() {
                    if let Some(card) = card_db.get_card(cid) {
                        let cost = card.cost.unwrap_or(0);
                        let modified_cost = (cost as i32
                            + self.game_state.mods.get_cost_modifier(cid))
                        .max(0) as u8;
                        seen_costs.insert(modified_cost);
                    }
                }
                if let Some(n) = distinct_only {
                    seen_costs.len() >= n
                } else {
                    seen_costs.len() == cards.len()
                }
            }
            "group_name" => {
                let mut seen: HashSet<String> = HashSet::default();
                for &cid in cards.iter() {
                    if let Some(card) = card_db.get_card(cid) {
                        if !card.group.is_empty() {
                            seen.insert(card.group.to_string());
                        }
                    }
                }
                if let Some(n) = distinct_only {
                    seen.len() >= n
                } else {
                    seen.len() == cards.len()
                }
            }
            _ => {
                // Collect each card's possible names (multi-name cards contribute
                // all constituent names from get_card_names).
                let name_sets: Vec<Vec<String>> = cards
                    .iter()
                    .map(|&cid| card_db.get_card_names(cid))
                    .filter(|ns| !ns.is_empty())
                    .collect();

                // Brute-force: try every combination of picking one name per card
                // to find the maximum distinct count.  At most 3 stage positions ×
                // 3 names each = 27 combos — easily fits in a small stack.
                let best = util::max_distinct_names(&name_sets);
                if let Some(n) = distinct_only {
                    best.distinct >= n
                } else {
                    !best.collision
                }
            }
        };
        pass
    }

    pub(crate) fn check_no_excess_heart(
        &self,
        condition: &Condition,
        player: &crate::player::Player,
        target: &str,
    ) -> bool {
        if !condition.get_no_excess_heart().unwrap_or(false) && !self.no_excess_heart_flag(target) {
            return true;
        }
        let card_db = &self.game_state.card_database;
        let h: u8 = player
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
        let need: u8 = player
            .live_card_zone
            .cards
            .iter()
            .chain(player.success_live_card_zone.cards.iter())
            .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
            .sum();
        h <= need
    }

    pub(crate) fn evaluate_location_condition(&self, condition: &Condition) -> bool {
        // NOTE: negation is NOT handled here — the parser sets `negation: True` on
        // conditions derived from natural-language negatives (e.g. "ない"), and the
        // conditional_alternative normalized_steps() also emits a negated condition.
        // Both cases rely on the caller (evaluate_comparison_condition or the
        // sequential pipeline's can_activate_effect) having already checked the
        // negation flag. Adding negation inside this function would DOUBLE-apply it.
        // When the sequential pipeline needs to negate a location condition for
        // conditional_alternative collapse, it should use a non-location
        // condition type (e.g. a simple comparison on a step_result flag) instead.
        if let Some(locs) = condition.get_locations() {
            if locs.len() >= 2 {
                if condition.get_target() == Some("both")
                    && condition.get_comparison_type() == Some("equality")
                {
                    return compare_counts(
                        condition.get_operator(),
                        self.get_count_for_target(condition, "self"),
                        self.get_count_for_target(condition, "opponent"),
                    );
                }
                if condition.get_target() == Some("either") {
                    return compare_counts(
                        condition.get_operator(),
                        if self.get_count_for_target(condition, "self") > 0
                            || self.get_count_for_target(condition, "opponent") > 0
                        {
                            1
                        } else {
                            0
                        },
                        condition.get_count().unwrap_or(1),
                    );
                }
                return self.evaluate_multi_location_condition(condition);
            }
        }

        let _target = condition.get_target().unwrap_or("self");
        let location = condition.get_location().unwrap_or("");
        let card_db = &self.game_state.card_database;
        let target = self.resolve_target_for_scope(condition);
        let is_both = target == "both";
        let player = self.resolve_condition_player(target);

        // temporal deck condition (e.g. "このターン、自分のデッキがリフレッシュしていた場合")
        if condition.get_temporal() == Some("this_turn") && location == "deck" {
            return player.deck_refreshed_this_turn;
        }

        if !self.check_heart_type_all(condition, player, location) {
            return false;
        }
        if !self.check_heart_colors(condition, player, location) {
            return false;
        }
        if !self.check_card_property(condition, player, location) {
            return false;
        }
        if !self.check_baton_touch(condition) {
            return false;
        }
        if let Some(res) = self.check_aggregate_total(condition, player, location) {
            return res;
        }
        if !self.check_ability_filter(condition, player, location) {
            return false;
        }
        if !self.check_distinct_names(condition, player, location) {
            return false;
        }
        if !self.check_no_excess_heart(condition, player, target) {
            return false;
        }

        // all=true with no operator/count → "more hearts than all others" comparison
        if condition.get_all().unwrap_or(false)
            && condition.get_operator().is_none()
            && condition.get_count().is_none()
            && location == "stage"
        {
            return self.evaluate_heart_greater_than_all(condition, is_both);
        }

        // NOTE: Movement gating for "was placed" (置かれた) triggers
        // is handled at the TAS enqueue level — see the pre-filter
        // in trigger_auto_abilities_for_player_with_event. At this
        // condition-evaluation layer, moved_cards may be empty (e.g.
        // during resolver::can_activate_effect where only the resolver's
        // own movement tracker is available, not the trigger event data).
        // The TAS-level gate prevents stale enqueue; this layer is purely
        // state-based (is the card in the zone?).

        if condition.get_comparison_type() == Some("equality") {
            // Cross-position equality: compare cost/count at position vs position_compare
            if let (Some(ref pos_a), Some(ref pos_b)) =
                (&condition.get_position(), &condition.get_position_compare())
            {
                let pos_a_str = pos_a.get_position().unwrap_or("");
                let card_a = util::card_at_position(player, pos_a_str);
                let card_b = util::card_at_position(player, pos_b);
                // When require_position_cards is set, both positions must have
                // cards for the comparison to be meaningful (e.g. "右サイドエリアと
                // 左サイドエリアにいるメンバーのコストが同じ場合").
                if condition.get_require_position_cards().unwrap_or(false) {
                    if card_a.is_none() || card_b.is_none() {
                        return false;
                    }
                }
                let cost_a = card_a
                    .and_then(|id| card_db.get_card(id))
                    .and_then(|c| c.cost)
                    .unwrap_or(0);
                let cost_b = card_b
                    .and_then(|id| card_db.get_card(id))
                    .and_then(|c| c.cost)
                    .unwrap_or(0);
                return compare_counts(condition.get_operator(), cost_a, cost_b);
            }
            // Self vs opponent equality
            if is_both {
                let self_count = self.get_count_for_target(condition, "self");
                let opp_count = self.get_count_for_target(condition, "opponent");
                return compare_counts(condition.get_operator(), self_count, opp_count);
            }
            // reference_card equality: compare card names against previously selected card
            if condition.get_reference_card() == Some("previous_selected") {
                let selected: &[i16] = self.selected_card_ids;
                if let Some(&selected_id) = selected.last() {
                    if let Some(selected_card) = card_db.get_card(selected_id) {
                        let selected_name =
                            crate::card::CardDatabase::normalize_name(&selected_card.name);
                        let target_cards: Vec<i16> = util::zone_cards(player, location).to_vec();
                        let matching = target_cards
                            .iter()
                            .filter(|&&cid| {
                                card_db.get_card(cid).is_some_and(|c| {
                                    crate::card::CardDatabase::normalize_name(&c.name)
                                        == selected_name
                                })
                            })
                            .count() as u8;
                        let required = condition.get_count().unwrap_or(1);
                        return matching >= required;
                    }
                }
                return false;
            }
        }

        let group_override = condition.get_group_reference().and_then(|gr| {
            if gr == "same_group_name" {
                self.activating_card_id
                    .and_then(|cid| self.game_state.card_database.get_card(cid))
                    .map(|c| c.group.as_ref())
            } else {
                None
            }
        });
        let op = condition.get_operator();

        let cards: Vec<i16> = if Zone::from_str(location) == Some(Zone::RevealedCards) {
            // When yell_trigger is true, the ability triggers on yell alone
            // regardless of how many (or which) cards were revealed.
            // The revealed pool is an action detail, not a trigger gate.
            if condition.get_yell_trigger() == Some(true) {
                return self.game_state.yell_occurred;
            }
            self.game_state.revealed_cards.to_vec()
        } else if is_both {
            let self_p = self.resolve_condition_player("self");
            let opp_p = self.resolve_condition_player("opponent");
            let mut combined: Vec<i16> = util::zone_cards(self_p, location).to_vec();
            combined.extend_from_slice(util::zone_cards(opp_p, location));
            combined
        } else {
            util::zone_cards(player, location).to_vec()
        };

        let effective_group = group_override.or_else(|| {
            condition
                .get_group_names()
                .and_then(|g| g.first().map(|s| s.as_str()))
        });

        let count = {
            let mut filter = condition.filter_subset();
            if let Some(grp) = group_override {
                filter.group = Some(grp);
            }
            if condition.get_exclude_self().unwrap_or(false) {
                filter.exclude_self = self.activating_card_id;
            }
            cards
                .iter()
                .filter(|&&id| {
                    id != -1
                        && filter.matches(card_db, id, true)
                        && self.check_original_blade_filter(condition, id)
                        && self.check_original_heart_filter(condition, id)
                        && self.check_heart_type_all_per_card(condition, card_db, id)
                })
                .count() as u8
        };
        // For revealed_cards: check that filtered cards collectively have all required
        // heart colors in their base_heart (printed hearts).
        if Zone::from_str(location) == Some(Zone::RevealedCards) {
            if let Some(hc) = condition.get_heart_colors() {
                if !hc.is_empty()
                    && !hc.iter().any(|cs| {
                        crate::card::parse_heart_color(cs) == crate::card::HeartColor::Heart00
                    })
                {
                    let filtered: Vec<i16> = cards
                        .iter()
                        .filter(|&&id| {
                            id != -1
                                && util::card_matches_type(
                                    card_db,
                                    id,
                                    condition.get_card_type().map(|ct| ct.as_str()),
                                )
                                && util::card_matches_group_str(card_db, id, effective_group)
                                && util::card_matches_characters(
                                    card_db,
                                    id,
                                    condition.get_characters(),
                                )
                        })
                        .copied()
                        .collect();
                    for color_str in hc {
                        let color = crate::card::parse_heart_color(color_str);
                        let found = filtered.iter().any(|&cid| {
                            card_db.get_card(cid).is_some_and(|c| {
                                c.base_heart
                                    .as_ref()
                                    .is_some_and(|bh| bh.hearts.contains_key(&color))
                            })
                        });
                        if !found {
                            return false;
                        }
                    }
                }
            }
        }
        if condition.get_all_areas().unwrap_or(false) {
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
        let thresh = condition.get_count().unwrap_or(1);
        let effective_op = op.or_else(|| if thresh == 0 { Some("==") } else { Some(">=") });
        compare_counts(effective_op, count, thresh)
    }

    fn get_card_total_hearts(&self, card_id: i16) -> HeartTotal {
        let card = match self.game_state.card_database.get_card(card_id) {
            Some(c) => c,
            None => return HeartTotal::None,
        };
        let base = match card.base_heart.as_ref() {
            Some(b) => b,
            None => return HeartTotal::None,
        };

        if base.hearts.contains_key(&HeartColor::Heart00)
            || base.hearts.contains_key(&HeartColor::All)
        {
            return HeartTotal::All;
        }

        let base_sum: u8 = base.hearts.values_sum();

        let modifier_total: i32 = self
            .game_state
            .mods
            .heart_modifiers
            .get(&card_id)
            .map(|hm| {
                hm.values()
                    .map(|e| e.set as i32 + e.additive as i32)
                    .sum::<i32>()
            })
            .unwrap_or(0);

        HeartTotal::Value((base_sum as i32 + modifier_total).max(0) as u8)
    }

    fn evaluate_heart_greater_than_all(&self, condition: &Condition, is_both: bool) -> bool {
        let activating_id = match self.activating_card_id {
            Some(id) => id,
            None => return false,
        };

        let self_hearts = self.get_card_total_hearts(activating_id);
        if self_hearts == HeartTotal::None {
            return false;
        }

        let card_db = &self.game_state.card_database;
        let card_type = condition.get_card_type().map(|ct| ct.as_str());
        let excl_self = condition.get_exclude_self().unwrap_or(false);

        let self_player = self.resolve_condition_player("self");
        let mut other_ids: Vec<i16> = self_player
            .stage
            .stage
            .iter()
            .filter(|&&cid| {
                if cid == -1 {
                    return false;
                }
                if excl_self && Some(cid) == self.activating_card_id {
                    return false;
                }
                if !util::card_matches_type(card_db, cid, card_type) {
                    return false;
                }
                true
            })
            .copied()
            .collect();

        if is_both {
            let opp_player = self.resolve_condition_player("opponent");
            let opp_ids: Vec<i16> = opp_player
                .stage
                .stage
                .iter()
                .filter(|&&cid| {
                    if cid == -1 {
                        return false;
                    }
                    if excl_self && Some(cid) == self.activating_card_id {
                        return false;
                    }
                    if !util::card_matches_type(card_db, cid, card_type) {
                        return false;
                    }
                    true
                })
                .copied()
                .collect();
            other_ids.extend(opp_ids);
        }

        if other_ids.is_empty() {
            return true;
        }

        for &other_id in &other_ids {
            let other_hearts = self.get_card_total_hearts(other_id);
            let self_wins = match (&self_hearts, &other_hearts) {
                (HeartTotal::All, HeartTotal::All) => false,
                (HeartTotal::All, _) => true,
                (_, HeartTotal::All) => false,
                (HeartTotal::Value(sh), HeartTotal::Value(oh)) => *sh > *oh,
                _ => false,
            };
            if !self_wins {
                return false;
            }
        }

        true
    }

    /// "元々持つブレード" — checks base/printed blade (card.blade from DB).
    ///
    /// Per Q195 (qa_data.json:1071-1074): "元々持つブレードの数を変更した後、
    /// ブレードを得る効果が適用される" — setting the original blade changes
    /// the base (Rules 9.9.1.4), then +blade effects stack on top (9.9.1.5).
    /// For current blade totals (e.g. "ブレードの合計"), Q116 (lines 2487-2488)
    /// confirms modified/current values are used instead.
    pub(crate) fn check_original_blade_filter(&self, condition: &Condition, card_id: i16) -> bool {
        if !condition.get_original_value().unwrap_or(false) {
            return true;
        }
        // For member cards, check_original_heart_filter handles the comparison
        if let Some(card) = self.game_state.card_database.get_card(card_id) {
            if card.is_member() {
                return true;
            }
        }
        if let Some(op) = condition.get_operator() {
            let threshold = condition.get_count().unwrap_or(0);
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
    ///
    /// Per Q172 (qa_data.json:1405-1406): "能力によって得たハートも含みます。
    /// ただし、エールによって得たブレードハートは含みません。" — ability-granted
    /// hearts ARE counted, but yell blade hearts are NOT.  The base comparison is
    /// against the card's printed hearts (total_hearts() = base_heart for members).
    /// Per Q149 (lines 1957-1958): "ハートの総数" = 基本ハート (basic hearts).
    /// This function sums heart_modifiers (ability-granted hearts) on top of
    /// base_hearts, then compares against base_hearts using the condition's operator.
    /// Rules 9.9.1.4→9.9.1.5 (rules.txt:1196-1212) defines the application order:
    /// printed base → set-to-value → add/subtract.
    pub(crate) fn check_original_heart_filter(&self, condition: &Condition, card_id: i16) -> bool {
        if !condition.get_original_value().unwrap_or(false) {
            return true;
        }
        let op = match condition.get_operator() {
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
            HeartColor::All,
        ] {
            let modifier = self.game_state.mods.get_heart_modifier(card_id, color);
            if modifier > 0 {
                current_hearts += modifier as u8;
            }
        }
        compare_counts(Some(op), current_hearts, base_hearts)
    }

    pub(crate) fn evaluate_multi_location_condition(&self, condition: &Condition) -> bool {
        let target = condition.get_target().unwrap_or("self");
        if target == "both" && condition.get_comparison_type() == Some("equality") {
            let p1_count = self.get_count_for_target(condition, "self");
            let p2_count = self.get_count_for_target(condition, "opponent");
            return compare_counts(condition.get_operator(), p1_count, p2_count);
        }

        let player = self.resolve_condition_player(target);
        let card_db = &self.game_state.card_database;
        let card_type_filter = condition.get_card_type().map(|ct| ct.as_str());
        let group_names = condition.get_group_names();
        let operator = condition.get_operator();
        let count_threshold = condition.get_count().unwrap_or(1);
        let locs = condition.get_locations().unwrap();

        if condition.get_self_target().unwrap_or(false) && locs.len() == 2 {
            let dest_zone = &locs[1];
            let dest_cards = util::zone_cards(player, dest_zone);
            return self
                .activating_card_id
                .is_some_and(|cid| dest_cards.contains(&cid));
        }

        let mut combined: Vec<i16> = Vec::new();
        for loc in locs {
            let cards = util::zone_cards(player, loc.as_str());
            combined.extend_from_slice(cards);
        }
        log::debug!("[MULTI] combined {} cards", combined.len());

        let is_distinct = condition.get_distinct().is_some_and(|d| d.is_distinct());
        if is_distinct {
            let distinct_type = match condition.get_distinct() {
                Some(crate::core::card::DistinctInfo::String(s)) => s.as_str(),
                _ if condition.get_group_reference() == Some("different_group_names") => {
                    "group_name"
                }
                _ => "card_name",
            };
            match distinct_type {
                "cost" => {
                    let mut distinct_costs: HashSet<u8> = HashSet::default();
                    for &cid in &combined {
                        if cid == -1 {
                            continue;
                        }
                        let passes_type = card_type_filter
                            .is_none_or(|f| util::card_matches_type(card_db, cid, Some(f)));
                        let passes_group = group_names.is_none_or(|gn| {
                            gn.iter().any(|g| {
                                util::card_matches_group_str(card_db, cid, Some(g.as_str()))
                            })
                        });
                        if !passes_type || !passes_group {
                            continue;
                        }
                        if let Some(card) = card_db.get_card(cid) {
                            let cost = card.cost.unwrap_or(0);
                            let modified_cost = (cost as i32
                                + self.game_state.mods.get_cost_modifier(cid))
                            .max(0) as u8;
                            distinct_costs.insert(modified_cost);
                        }
                    }
                    let count = distinct_costs.len() as u8;
                    compare_counts(operator, count, count_threshold)
                }
                "group_name" => {
                    let mut distinct_groups: HashSet<String> = HashSet::default();
                    for &cid in &combined {
                        if cid == -1 {
                            continue;
                        }
                        let passes_type = card_type_filter
                            .is_none_or(|f| util::card_matches_type(card_db, cid, Some(f)));
                        let passes_group = group_names.is_none_or(|gn| {
                            gn.iter().any(|g| {
                                util::card_matches_group_str(card_db, cid, Some(g.as_str()))
                            })
                        });
                        if !passes_type || !passes_group {
                            continue;
                        }
                        if let Some(card) = card_db.get_card(cid) {
                            if !card.group.is_empty() {
                                distinct_groups.insert(card.group.to_string());
                            }
                        }
                    }
                    let count = distinct_groups.len() as u8;
                    compare_counts(operator, count, count_threshold)
                }
                _ => {
                    let mut name_sets: Vec<Vec<String>> = Vec::new();
                    for &cid in &combined {
                        if cid == -1 {
                            continue;
                        }
                        let passes_type = card_type_filter
                            .is_none_or(|f| util::card_matches_type(card_db, cid, Some(f)));
                        let passes_group = group_names.is_none_or(|gn| {
                            gn.iter().any(|g| {
                                util::card_matches_group_str(card_db, cid, Some(g.as_str()))
                            })
                        });
                        if !passes_type || !passes_group {
                            continue;
                        }
                        name_sets.push(card_db.get_card_names(cid));
                    }
                    let best = util::max_distinct_names(&name_sets);
                    compare_counts(operator, best.distinct as u8, count_threshold)
                }
            }
        } else {
            let mut filter = condition.filter_subset();
            filter.card_type = card_type_filter;
            filter.group = group_names.and_then(|g| g.first().map(|s| s.as_str()));
            filter.cost_operator = operator;
            let matching_count = util::count_matching(&combined, card_db, &filter, false);
            compare_counts(operator, matching_count, count_threshold)
        }
    }

    pub(crate) fn evaluate_position_condition(&self, condition: &Condition) -> bool {
        let target = condition.get_target().unwrap_or("self");
        let position = condition
            .get_position()
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
        if condition.get_all_members().unwrap_or(false) {
            let location = condition.get_location().unwrap_or("");
            let target = condition.get_target().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            let group_name = condition
                .get_group_names()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            let card_db = &self.game_state.card_database;
            match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    return player
                        .stage
                        .stage
                        .iter()
                        .filter(|&&id| id != -1)
                        .all(|&id| {
                            crate::ability::util::card_matches_group_str(card_db, id, group_name)
                        });
                }
                Some(Zone::LiveCardZone) => {
                    return player.live_card_zone.cards.iter().all(|&id| {
                        crate::ability::util::card_matches_group_str(card_db, id, group_name)
                    });
                }
                Some(Zone::SuccessLiveZone) => {
                    return player.success_live_card_zone.cards.iter().all(|&id| {
                        crate::ability::util::card_matches_group_str(card_db, id, group_name)
                    });
                }
                _ => return false,
            }
        }
        let g_target = condition.get_target().unwrap_or("self");
        let g_player = self.resolve_condition_player(g_target);
        let g_location = condition.get_location().unwrap_or("");

        // Aggregate total check (sum of heart values, e.g. heart02 >= 6)
        if let Some(res) = self.check_aggregate_total(condition, &g_player, g_location) {
            return res;
        }

        // When heart_colors are specified, check that the collective cards in the
        // target zone cover ALL required heart colors (e.g. yell-revealed cards).
        let hc: &[String] = condition.get_heart_colors().unwrap_or(&[]);
        if !hc.is_empty() {
            let target = condition.get_target().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            let card_db = &self.game_state.card_database;
            let group_name = condition
                .get_group_names()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            let location = condition.get_location().unwrap_or("");
            let source_cards: Vec<i16> = match Zone::from_str(location) {
                Some(Zone::RevealedCards) => self.game_state.revealed_cards.to_vec(),
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
                .map(|s| crate::card::parse_heart_color(s))
                .collect();
            let mut present = HashSet::<HeartColor>::default();
            for &cid in &source_cards {
                if cid == -1 {
                    continue;
                }
                if !crate::ability::util::card_matches_group_str(card_db, cid, group_name) {
                    continue;
                }
                if let Some(ref ct) = condition.get_card_type() {
                    if !crate::ability::util::card_matches_type(card_db, cid, Some(ct)) {
                        continue;
                    }
                }
                if let Some(card) = card_db.get_card(cid) {
                    let is_blade = condition.get_heart_source() == Some("blade");
                    if !is_blade {
                        if let Some(ref bh) = card.base_heart {
                            for color in bh.hearts.keys() {
                                if required_colors.contains(color) {
                                    present.insert(*color);
                                }
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
        // When multiple group_names are specified, check EACH group has at least `count` members
        if let Some(groups) = condition.get_group_names() {
            if groups.len() > 1 && condition.get_aggregate() != Some("total") {
                let target_count = condition.get_count().unwrap_or(1);
                let ct = condition.get_card_type().map(|ct| ct.as_str());
                let exc = condition.get_exclude_characters();
                let location = condition.get_location().unwrap_or("");
                let cards: &[i16] = match Zone::from_str(location) {
                    Some(Zone::Stage) => &g_player.stage.stage,
                    Some(Zone::Hand) => &g_player.hand.cards,
                    Some(Zone::Discard) | Some(Zone::Waitroom) => &g_player.waitroom.cards,
                    Some(Zone::LiveCardZone) => &g_player.live_card_zone.cards,
                    Some(Zone::SuccessLiveZone) => &g_player.success_live_card_zone.cards,
                    Some(Zone::Energy) => &g_player.energy_zone.cards,
                    _ => return false,
                };
                for group_name in groups {
                    let group_slice = [group_name.clone()];
                    let count =
                        self.count_group_cards_in_cards(cards, Some(&group_slice[..]), ct, exc);
                    if count < target_count {
                        return false;
                    }
                }
                return true;
            }
        }
        if condition.get_temporal() == Some("this_turn")
            && condition.get_self_target().unwrap_or(false)
            && condition.get_group_names().is_some_and(|g| !g.is_empty())
            && condition.get_card_type().map(|ct| ct.as_str()) == Some("member_card")
        {
            if let Some(activating_card_id) = self.activating_card_id {
                let target = condition.get_target().unwrap_or("self");
                let player = self.resolve_condition_player(target);
                let card_db = &self.game_state.card_database;
                let groups = condition.get_group_names().unwrap();
                return self.game_state.turn_area_movements.iter().any(|m| {
                    m.moved_card_id == activating_card_id
                        && m.effect_only
                        && m.cause_player_id == player.id
                        && m.cause_card_id.is_some_and(|cause_cid| {
                            groups.iter().any(|g| {
                                crate::ability::util::card_matches_group_str(
                                    card_db,
                                    cause_cid,
                                    Some(g),
                                )
                            })
                        })
                });
            }
            return false;
        }

        let mut count = self.get_group_card_count(condition);
        if condition.get_exclude_self().unwrap_or(false) {
            if let Some(aid) = self.activating_card_id {
                let target = condition.get_target().unwrap_or("self");
                let player = self.resolve_condition_player(target);
                if player.stage.stage.contains(&aid) {
                    count = count.saturating_sub(1);
                }
            }
        }
        let target_count = condition.get_count().unwrap_or(1);
        let operator = condition.get_operator().or(Some(">="));
        compare_counts(operator, count, target_count)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_moved_cards_source(
        &self,
        condition: &Condition,
        is_new_movement: bool,
        card_type: &str,
        hc: &[String],
        count: u8,
    ) -> bool {
        let card_db = &self.game_state.card_database;
        let negate = condition.get_negation().unwrap_or(false);
        let wants_blade_heart_prop =
            condition.get_card_property() == Some(CardProperty::HasBladeHeart);
        // Source of moved card IDs: for the new format (source=zone+dst),
        // query turn_movements for all matching zone-transition events
        // this turn. For the old format (preceding_moved), use the
        // enqueue-time snapshot or recently_moved_cards.
        let source_zone = condition.get_source().unwrap_or("");
        // Build moved_source: for new format (source=zone+dst), prefer
        // turn_movements (has zone/player metadata for filtering).  When
        // turn_movements has any data, trust its filtered result (empty
        // means no matching events for this player/zone — still correct).
        // Only fall back to the trigger-context card IDs when
        // turn_movements is completely empty (e.g. manual test setup).
        let moved_source: SmallVec<[i16; 8]> = if is_new_movement {
            let dest_zone = condition.get_destination().unwrap_or("");
            let target = condition.get_target().unwrap_or("self");
            let self_pl = self.resolve_condition_player(target);
            let target_id = self_pl.id.as_str();
            let event_cards: SmallVec<[i16; 8]> = if !self.moved_cards.is_empty() {
                self.moved_cards.iter().copied().collect()
            } else if let Some(enq) = self.game_state.entry_trigger_moved_cards() {
                enq.iter().copied().collect()
            } else if let Some(global) = self.game_state.recently_moved_cards.clone() {
                global.iter().copied().collect()
            } else {
                SmallVec::new()
            };
            if !event_cards.is_empty() {
                let require_self_effect = condition.get_self_effect_only().unwrap_or(false);
                let from_tm: SmallVec<[i16; 8]> = self
                    .game_state
                    .turn_movements
                    .iter()
                    .filter(|m| {
                        let src_ok = source_zone.is_empty()
                            || m.source_zone == source_zone
                            || (source_zone == "discard" && m.source_zone == "waitroom")
                            || (source_zone == "waitroom" && m.source_zone == "discard");
                        let cause_ok = (condition.get_self_target().unwrap_or(false)
                            && !require_self_effect)
                            || m.cause_player_id == target_id;
                        src_ok
                            && cause_ok
                            && (m.dest_zone == dest_zone
                                || (dest_zone == "discard" && m.dest_zone == "waitroom")
                                || (dest_zone == "waitroom" && m.dest_zone == "discard"))
                            && event_cards.contains(&m.moved_card_id)
                    })
                    .map(|m| m.moved_card_id)
                    .collect();
                let result: SmallVec<[i16; 8]> = event_cards
                    .iter()
                    .filter(|&&cid| {
                        let tm: SmallVec<[&crate::types::MovementEvent; 2]> = self
                            .game_state
                            .turn_movements
                            .iter()
                            .filter(|m| m.moved_card_id == cid)
                            .collect();
                        if tm.is_empty() {
                            return true;
                        }
                        tm.iter().any(|m| {
                            let src_ok = m.source_zone == source_zone
                                || (source_zone == "discard" && m.source_zone == "waitroom")
                                || (source_zone == "waitroom" && m.source_zone == "discard");
                            let cause_ok = (condition.get_self_target().unwrap_or(false)
                                && !require_self_effect)
                                || m.cause_player_id == target_id;
                            src_ok
                                && cause_ok
                                && (m.dest_zone == dest_zone
                                    || (dest_zone == "discard" && m.dest_zone == "waitroom")
                                    || (dest_zone == "waitroom" && m.dest_zone == "discard"))
                        })
                    })
                    .copied()
                    .collect();
                if !from_tm.is_empty() {
                    from_tm
                } else {
                    result
                }
            } else if !self.game_state.turn_movements.is_empty() {
                let require_self_effect = condition.get_self_effect_only().unwrap_or(false);
                self.game_state
                    .turn_movements
                    .iter()
                    .filter(|m| {
                        let src_ok = source_zone.is_empty()
                            || m.source_zone == source_zone
                            || (source_zone == "discard" && m.source_zone == "waitroom")
                            || (source_zone == "waitroom" && m.source_zone == "discard");
                        let cause_ok = (condition.get_self_target().unwrap_or(false)
                            && !require_self_effect)
                            || m.cause_player_id == target_id;
                        src_ok
                            && cause_ok
                            && (m.dest_zone == dest_zone
                                || (dest_zone == "discard" && m.dest_zone == "waitroom")
                                || (dest_zone == "waitroom" && m.dest_zone == "discard"))
                    })
                    .map(|m| m.moved_card_id)
                    .collect()
            } else {
                SmallVec::new()
            }
        } else if self.moved_cards.is_empty() {
            let enqueued = self.game_state.entry_trigger_moved_cards();
            let global = self.game_state.recently_moved_cards.clone();
            if let Some(ev) = &enqueued {
                if !ev.is_empty() {
                    ev.iter().copied().collect()
                } else {
                    global.map_or_else(SmallVec::new, |g| g.iter().copied().collect())
                }
            } else {
                global.map_or_else(SmallVec::new, |g| g.iter().copied().collect())
            }
        } else {
            self.moved_cards.iter().copied().collect()
        };
        // When self_target, restrict to the activating card only —
        // don't check ALL moved cards. Applies to all source formats.
        let moved_source = if condition.get_self_target().unwrap_or(false) {
            self.activating_card_id
                .filter(|id| moved_source.contains(id))
                .map_or(smallvec::SmallVec::new(), |id| smallvec::smallvec![id])
        } else {
            moved_source
        };
        // Determine destination zone for zone-transition filtering.
        // Prefer the explicit `destination` field (new pattern), then fall back
        // to inferring from locations (old pattern: location=source, locations contains dest).
        let dest_zone: Option<String> =
            condition
                .get_destination()
                .map(|s| s.to_string())
                .or_else(|| {
                    condition.get_location().and_then(|src| {
                        condition
                            .get_locations()
                            .and_then(|locs| locs.iter().find(|l| l.as_str() != src).cloned())
                    })
                });
        let moved_group_name = condition
            .get_group_names()
            .and_then(|g| g.first().map(|s| s.as_str()));
        let moved_ids = moved_source
            .iter()
            .filter(|&&cid| {
                // Energy is tracked as an anonymous `-1` resource for
                // energy_zone↔energy_deck moves. Count it only when the
                // condition is a zone-change on energy (source+dest) and skip
                // all card-id-dependent filters (there is no real card).
                if cid == -1 {
                    return is_new_movement
                        && condition.get_resource_type() == Some("energy");
                }
                if let Some(gn) = moved_group_name {
                    if !util::card_matches_group_str(card_db, cid, Some(gn)) {
                        return false;
                    }
                }
                let type_ok =
                    card_type.is_empty() || util::card_matches_type(card_db, cid, Some(card_type));
                let has_bh = wants_blade_heart_prop
                    && card_db.get_card(cid).is_some_and(|c| c.has_blade_heart());
                let bh_reject =
                    wants_blade_heart_prop && ((negate && has_bh) || (!negate && !has_bh));
                if !type_ok || bh_reject {
                    return false;
                }
                if !hc.is_empty() && !util::card_matches_heart_colors(card_db, cid, hc) {
                    return false;
                }
                if !util::card_matches_characters(card_db, cid, condition.get_characters()) {
                    return false;
                }
                // Zone-transition filter: if destination zone is specified,
                // only count cards that are currently in that zone.
                if let Some(ref dest) = dest_zone {
                    let zone_match = match Zone::from_str(dest) {
                        Some(Zone::Discard) | Some(Zone::Waitroom) => {
                            self.game_state.player1.waitroom.cards.contains(&cid)
                                || self.game_state.player2.waitroom.cards.contains(&cid)
                        }
                        Some(Zone::Stage) => {
                            self.game_state.player1.stage.stage.contains(&cid)
                                || self.game_state.player2.stage.stage.contains(&cid)
                        }
                        Some(Zone::Hand) => {
                            self.game_state.player1.hand.cards.contains(&cid)
                                || self.game_state.player2.hand.cards.contains(&cid)
                        }
                        Some(Zone::EnergyZone) | Some(Zone::Energy) => {
                            self.game_state.player1.energy_zone.cards.contains(&cid)
                                || self.game_state.player2.energy_zone.cards.contains(&cid)
                        }
                        Some(Zone::Deck) | Some(Zone::DeckTop) | Some(Zone::DeckBottom) => {
                            self.game_state.player1.main_deck.cards.contains(&cid)
                                || self.game_state.player2.main_deck.cards.contains(&cid)
                        }
                        Some(Zone::LiveCardZone) => {
                            self.game_state.player1.live_card_zone.cards.contains(&cid)
                                || self.game_state.player2.live_card_zone.cards.contains(&cid)
                        }
                        Some(Zone::SuccessLiveZone) => {
                            self.game_state
                                .player1
                                .success_live_card_zone
                                .cards
                                .contains(&cid)
                                || self
                                    .game_state
                                    .player2
                                    .success_live_card_zone
                                    .cards
                                    .contains(&cid)
                        }
                        _ => true, // Unknown zone — don't filter
                    };
                    if !zone_match {
                        return false;
                    }
                    // Source-zone verification: also check that the card
                    // moved from the expected source zone, not just that
                    // it's currently in the destination zone. Catches
                    // cases like deck→waitroom when trigger expects
                    // stage→waitroom.
                    let expected_source = condition.get_location().or_else(|| {
                        condition
                            .get_source()
                            .filter(|&s| s != "preceding_moved" && s != "previous_moved_cards")
                    });
                    if let Some(src_zone) = expected_source {
                        let card_movements: Vec<_> = self
                            .game_state
                            .turn_movements
                            .iter()
                            .filter(|m| m.moved_card_id == cid)
                            .collect();
                        if !card_movements.is_empty() {
                            let src_match = card_movements.iter().any(|m| {
                                let src_ok = m.source_zone == src_zone
                                    || (src_zone == "discard" && m.source_zone == "waitroom")
                                    || (src_zone == "waitroom" && m.source_zone == "discard");
                                src_ok
                                    && (m.dest_zone == *dest
                                        || (*dest == "discard" && m.dest_zone == "waitroom")
                                        || (*dest == "waitroom" && m.dest_zone == "discard"))
                            });
                            if !src_match {
                                return false;
                            }
                        }
                    }
                }

                true
            })
            .copied()
            .collect::<Vec<i16>>();
        // unit:"types" → count DISTINCT blade-heart colors among the moved member
        // cards (G18: "…の中に2種類以上のブレードハートの色がある場合"), not card count.
        let unit_is_types = condition.get_unit().as_deref() == Some("types");
        let actual: u8 = if unit_is_types {
            let mut colors: HashSet<HeartColor> = HashSet::default();
            for &cid in &moved_ids {
                if let Some(card) = card_db.get_card(cid) {
                    if let Some(ref bh) = card.blade_heart {
                        for color in bh.hearts.keys() {
                            colors.insert(*color);
                        }
                    }
                }
            }
            colors.len() as u8
        } else {
            moved_ids.len() as u8
        };
        // negation for the count comparison only applies when card_property is not
        // driving the per-card filter (handled above). For pure count negation
        // ("ない場合" style), flip the compare result.
        let op = condition.get_operator().unwrap_or(">=");
        let passed = compare_counts(Some(op), actual, count);
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
                condition.get_card_property(),
                hc,
                negate,
                actual,
                condition.get_operator(),
                count,
                result
            );
        result
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resolve_zone_card_count(
        &self,
        condition: &Condition,
        location: &str,
        hc: &[String],
        is_both: bool,
        player: &crate::player::Player,
        card_type: &str,
        group_names: Option<&[String]>,
    ) -> u8 {
        let target = self.resolve_target_for_scope(condition);
        let exclude_self = condition.get_exclude_self().unwrap_or(false);
        let activating_id = self.activating_card_id;
        let card_db = &self.game_state.card_database;
        let count_filtered = |zone_source: &[i16], ct: &str| -> usize {
            let is_distinct = condition.get_distinct().is_some_and(|d| d.is_distinct())
                || condition.get_group_reference() == Some("different_group_names");
            if is_distinct {
                self.count_distinct_in_cards(
                    zone_source,
                    condition,
                    Some(ct).filter(|c| !c.is_empty()),
                    group_names,
                ) as usize
            } else {
                self.count_cards_with_filters(
                    zone_source,
                    Some(ct),
                    group_names,
                    hc,
                    condition.get_cost_limit(),
                    condition.get_cost_limit_operator().map(|o| o.as_str()),
                    None,
                    false,
                    condition,
                ) as usize
            }
        };

        let actual = match Zone::from_str(location) {
            Some(Zone::RevealedCards) => {
                if condition.get_unit().as_deref() == Some("types") && !hc.is_empty() {
                    let required_colors: Vec<crate::card::HeartColor> = hc
                        .iter()
                        .map(|s| crate::card::parse_heart_color(s))
                        .collect();
                    let mut present = HashSet::<HeartColor>::default();
                    let is_blade = condition.get_heart_source() == Some("blade");
                    for &cid in &self.game_state.revealed_cards {
                        if cid == -1 {
                            continue;
                        }
                        if let Some(card) = card_db.get_card(cid) {
                            if !is_blade {
                                if let Some(ref bh) = card.base_heart {
                                    for color in bh.hearts.keys() {
                                        if required_colors.contains(color) {
                                            present.insert(*color);
                                        }
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
                    let actual = present.len();
                    log::debug!(
                        "[REVEALED_TYPES] heart_source={:?} required={:?} present={:?} count={}",
                        condition.get_heart_source(),
                        required_colors,
                        present,
                        actual,
                    );
                    actual as usize
                } else {
                    count_filtered(&self.game_state.revealed_cards, card_type)
                }
            }
            Some(Zone::Stage) => {
                let stage_cards: SmallVec<[i16; 6]> = if is_both {
                    let opp = self.resolve_condition_player("opponent");
                    let mut combined: SmallVec<[i16; 6]> =
                        player.stage.stage.iter().copied().collect();
                    combined.extend_from_slice(&opp.stage.stage);
                    combined
                } else {
                    player.stage.stage.iter().copied().collect()
                };
                // Filter by state (wait/active) if specified
                let stage_cards: SmallVec<[i16; 6]> = if let Some(ref state) = condition.get_state()
                {
                    stage_cards
                        .into_iter()
                        .filter(|&cid| {
                            if cid == -1 {
                                return false;
                            }
                            self.game_state
                                .mods
                                .get_orientation_modifier(cid)
                                .map_or(state == &CardState::Active, |o| o == state.as_str())
                        })
                        .collect()
                } else {
                    stage_cards
                };
                // Baton touch filter: when baton_touch_trigger is set on a
                // card_count_condition with location=stage, only count cards
                // that arrived via baton touch this turn.
                let stage_cards: SmallVec<[i16; 6]> =
                    if condition.get_baton_touch_trigger().unwrap_or(false) {
                        let player_id = player.id.as_str();
                        let bt_ids = &self.game_state.baton_touch_arriving_card_ids;
                        // Also gate on per-player baton touch count
                        if let Some(min_count) = condition.get_min_baton_touch_count() {
                            if self.game_state.get_baton_touch_count(player_id) < min_count {
                                SmallVec::new()
                            } else {
                                stage_cards
                                    .into_iter()
                                    .filter(|cid| bt_ids.contains(cid))
                                    .collect()
                            }
                        } else {
                            stage_cards
                                .into_iter()
                                .filter(|cid| bt_ids.contains(cid))
                                .collect()
                        }
                    } else {
                        stage_cards
                    };
                let is_distinct_cost = condition.get_distinct().is_some_and(
                    |d| matches!(d, crate::core::card::DistinctInfo::String(s) if s == "cost"),
                );
                if is_distinct_cost {
                    let mut distinct_costs: HashSet<u8> = HashSet::default();
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
                            .max(0) as u8;
                        distinct_costs.insert(modified);
                    }
                    distinct_costs.len()
                } else if condition.get_unit().as_deref() == Some("types") {
                    let required_colors: Vec<crate::card::HeartColor> = hc
                        .iter()
                        .map(|s| crate::card::parse_heart_color(s))
                        .collect();
                    let mut present = HashSet::<HeartColor>::default();
                    let is_blade = condition.get_heart_source() == Some("blade");
                    for &cid in &stage_cards {
                        if cid == -1 {
                            continue;
                        }
                        if let Some(card) = card_db.get_card(cid) {
                            if !is_blade {
                                if let Some(ref bh) = card.base_heart {
                                    for color in bh.hearts.keys() {
                                        if required_colors.contains(color) {
                                            present.insert(*color);
                                        }
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
            Some(Zone::UnderMember) => {
                // Cards under a specific member (このメンバーの下). Find the
                // activating card's stage position and count its under_cards.
                if let Some(cid) = self.activating_card_id {
                    let pos = player.stage.stage.iter().position(|&id| id == cid);
                    if let Some(idx) = pos {
                        count_filtered(&player.stage.under_cards[idx], card_type)
                    } else {
                        0
                    }
                } else {
                    // Fallback: count under_cards across all positions
                    let all_under: Vec<i16> = player
                        .stage
                        .under_cards
                        .iter()
                        .flat_map(|sv| sv.iter())
                        .copied()
                        .collect();
                    count_filtered(&all_under, card_type)
                }
            }
            Some(Zone::Discard) | Some(Zone::Waitroom) => {
                if condition
                    .get_text()
                    .map_or(false, |t| t.contains("手札から"))
                {
                    // Event-based: only count recently-moved cards from hand
                    if let Some(ref moved) = self.game_state.recently_moved_cards {
                        let from_hand =
                            self.game_state.recently_moved_from_zone.as_deref() == Some("hand");
                        if !from_hand {
                            return 0;
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
                    if condition.get_aggregate() == Some("total") {
                        let self_blade = player.stage.total_blades(
                            card_db,
                            &self.game_state.mods.blade_modifiers,
                            &self.game_state.mods.orientation_modifiers,
                            true,
                        );
                        if is_both {
                            let opp = self.resolve_condition_player("opponent");
                            (self_blade
                                + opp.stage.total_blades(
                                    card_db,
                                    &self.game_state.mods.blade_modifiers,
                                    &self.game_state.mods.orientation_modifiers,
                                    true,
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
                                if id == -1 {
                                    return false;
                                }
                                if !hc.is_empty()
                                    && !crate::ability::util::card_matches_heart_colors(
                                        card_db, id, hc,
                                    )
                                {
                                    return false;
                                }
                                if let Some(ref state) = condition.get_state() {
                                    let state_ok = self
                                        .game_state
                                        .mods
                                        .get_orientation_modifier(id)
                                        .map_or(state == &CardState::Active, |o| {
                                            o == state.as_str()
                                        });
                                    if !state_ok {
                                        return false;
                                    }
                                }
                                true
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
        } as u8;
        actual
    }

    pub(crate) fn evaluate_card_count_condition(&self, condition: &Condition) -> bool {
        let card_type = condition
            .get_card_type()
            .map(|ct| ct.as_str())
            .unwrap_or("");
        let target = self.resolve_target_for_scope(condition);
        let is_both = target == "both";
        let count = condition.get_count().unwrap_or(1);
        let player = self.resolve_condition_player(target);
        let card_db = &self.game_state.card_database;

        let location = condition.get_location().unwrap_or("");
        let group_names_owned: Option<Vec<String>> =
            condition.get_group_names().map(|g| g.to_vec()).or_else(|| {
                if condition.get_group_reference() == Some("same_group_name") {
                    self.activating_card_id
                        .and_then(|cid| self.game_state.card_database.get_card(cid))
                        .map(|c| c.group.to_string())
                        .map(|g| vec![g])
                } else {
                    None
                }
            });
        let group_names: Option<&[String]> = group_names_owned.as_deref();
        let hc: &[String] = condition.get_heart_colors().unwrap_or(&[]);

        // Early-out for aggregate total (sum heart colors, not count cards)
        if let Some(res) = self.check_aggregate_total(condition, player, location) {
            return res;
        }

        let is_old_movement = condition.get_source() == Some("preceding_moved")
            || condition.get_source() == Some("previous_moved_cards");
        let is_new_movement = condition.get_source().map_or(true, |s| {
            s != "preceding_moved" && s != "previous_moved_cards"
        }) && condition.get_destination().is_some();

        if is_old_movement || is_new_movement {
            return self.resolve_moved_cards_source(
                condition,
                is_new_movement,
                card_type,
                hc,
                count,
            );
        }

        let actual = self.resolve_zone_card_count(
            condition,
            location,
            hc,
            is_both,
            player,
            card_type,
            group_names,
        );
        let count_op =
            condition
                .get_operator()
                .or_else(|| if count == 0 { Some("==") } else { Some(">=") });
        let mut passed = compare_counts(count_op, actual, count);
        // Same-name constraint: if set, ensure at least 2 counted cards share a name
        if passed && condition.get_same_name().unwrap_or(false) {
            let stage_cards: SmallVec<[i16; 6]> = if is_both {
                let opp = self.resolve_condition_player("opponent");
                let mut combined: SmallVec<[i16; 6]> = player.stage.stage.iter().copied().collect();
                combined.extend_from_slice(&opp.stage.stage);
                combined
            } else {
                player.stage.stage.iter().copied().collect()
            };
            // Apply state filter for same-name check if specified
            let stage_cards: SmallVec<[i16; 6]> = if let Some(ref state) = condition.get_state() {
                stage_cards
                    .into_iter()
                    .filter(|&cid| {
                        if cid == -1 {
                            return false;
                        }
                        self.game_state
                            .mods
                            .get_orientation_modifier(cid)
                            .map_or(state == &CardState::Active, |o| o == state.as_str())
                    })
                    .collect()
            } else {
                stage_cards
            };
            let mut name_counts: HashMap<String, u8> = HashMap::default();
            for &cid in &stage_cards {
                if cid == -1 {
                    continue;
                }
                // Multi-name cards (e.g. "A&B&C") contribute each constituent
                // name — two cards share a name if any constituent overlaps.
                for name in card_db.get_card_names(cid) {
                    *name_counts.entry(name).or_insert(0) += 1;
                }
            }
            passed = name_counts.values().any(|&c| c >= 2);
        }
        let mut dbg = AbDebug::new();
        dbg.condition(condition, actual, count, passed);
        #[cfg(not(feature = "no_std"))]
        super::push_cond_verdict(condition, &format!("{}枚", actual), passed, vec![]);
        passed
    }

    pub(crate) fn evaluate_card_blade_condition(&self, condition: &Condition) -> bool {
        let count = condition.get_count().unwrap_or(1);
        let operator = condition.get_operator();
        let source = condition.get_source().unwrap_or("selected_cards");
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
            let set_blade = self.game_state.mods.get_blade_set_modifier(cid);
            let additive = self.game_state.mods.get_blade_modifier(cid) - set_blade;
            let effective_base = if set_blade != 0 { set_blade } else { base };
            total_blades += effective_base + additive;
        }
        let names: String = cards
            .iter()
            .map(|&cid| {
                card_db
                    .get_card(cid)
                    .map(|c| c.name.as_ref())
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
            if util::compare_counts(operator, total_blades.max(0) as u8, count) {
                "PASS"
            } else {
                "FAIL"
            }
        );
        util::compare_counts(operator, total_blades.max(0) as u8, count)
    }

    pub(crate) fn evaluate_appearance_condition(&self, condition: &Condition) -> bool {
        let appearance = condition.get_appearance().unwrap_or(false);
        let location = condition.get_location().unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let baton_touch_trigger = condition.get_baton_touch_trigger().unwrap_or(false);
        let player = self.resolve_condition_player(target);

        // Helper to push enriched verdict with character/cost data
        #[cfg(not(feature = "no_std"))]
        let push_rich = |actual: &str, ok: bool| {
            super::push_cond_verdict(condition, actual, ok, vec![]);
        };
        #[cfg(feature = "no_std")]
        let push_rich = |_actual: &str, _ok: bool| {};

        if baton_touch_trigger {
            if let Some(ref _activating_card) = self.game_state.activating_card {
                let player_id = player.id.as_str();
                let bt_count = self.game_state.get_baton_touch_count(player_id);
                if bt_count == 0 {
                    push_rich("バトンタッチ未実行", false);
                    return false;
                }
                if let Some(min_count) = condition.get_min_baton_touch_count() {
                    if bt_count < min_count {
                        push_rich(&format!("バトンタッチ回数不足({})", bt_count), false);
                        return false;
                    }
                }
            } else {
                push_rich("起動カードなし", false);
                return false;
            }
        }

        if !baton_touch_trigger && !appearance {
            push_rich("不在条件", false);
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
                        push_rich("ステージ空", false);
                        return false;
                    }
                    // Self-trigger guard: when the condition has NO card-
                    // targeting filters (group_names, cost_limit, card_type,
                    // characters), it can only be "このメンバーが登場" (this
                    // member appears). For those conditions, the scanned card
                    // must have actually appeared this turn.
                    let has_card_filters =
                        condition.get_group_names().map_or(false, |g| !g.is_empty())
                            || condition.get_cost_limit().is_some()
                            || condition.get_card_type().is_some()
                            || condition.get_characters().map_or(false, |c| !c.is_empty());
                    if !baton_touch_trigger && !has_card_filters {
                        if self.game_state.cards_appeared_this_turn.is_empty() {
                            push_rich("今ターン未登場", false);
                            return false;
                        }
                        let self_appeared = self.activating_card_id.is_some_and(|cid| {
                            // Batch-scoped guard: when moved_cards is non-empty,
                            // the card must have appeared in the current batch,
                            // not the entire turn. Prevents stale turn-level data
                            // from triggering re-scans on unrelated events.
                            // Also accept cards in recently_appeared_cards — during
                            // baton touch the arriving card's movement event is not
                            // pushed (only the replaced member's is), but the card
                            // DID appear in this batch via record_card_appearance.
                            let batch_ok = self.moved_cards.is_empty()
                                || self.moved_cards.contains(&cid)
                                || self
                                    .game_state
                                    .recently_moved_cards
                                    .as_ref()
                                    .map_or(false, |v| v.contains(&cid))
                                || self.game_state.recently_appeared_cards.contains(&cid);
                            batch_ok
                                && self.game_state.has_card_appeared_this_turn(cid)
                                && stage_ids.contains(&cid)
                        });
                        if !self_appeared {
                            push_rich("自カード未登場", false);
                            return false;
                        }
                    }
                    // Generalized self-trigger guard: when cost_limit or card_type filters
                    // are set AND exclude_self is true, prevent the activating card from
                    // triggering on its own appearance (the same logic as the group_names
                    // guard below).
                    if !baton_touch_trigger
                        && !self.game_state.cards_appeared_this_turn.is_empty()
                        && condition.get_exclude_self().unwrap_or(false)
                        && (condition.get_cost_limit().is_some()
                            || condition.get_card_type().is_some())
                    {
                        let has_other_appeared = stage_ids.iter().any(|&cid| {
                            self.activating_card_id.map_or(true, |act_id| cid != act_id)
                                && self.game_state.has_card_appeared_this_turn(cid)
                        });
                        if !has_other_appeared {
                            push_rich("自カードのみ登場", false);
                            return false;
                        }
                    }
                    let filled_count = player.stage.stage.iter().filter(|&&id| id != -1).count();
                    if condition.get_all_areas().unwrap_or(false) && filled_count != 3 {
                        push_rich(&format!("全エリア未充足({}/3)", filled_count), false);
                        return false;
                    }
                    // Check cost_limit if specified (e.g. コスト10のメンバー)
                    if let Some(cost_limit) = condition.get_cost_limit() {
                        let operator = condition.get_operator().unwrap_or("=");
                        let cost_match = stage_ids.iter().any(|&cid| {
                            self.game_state
                                .card_database
                                .get_card(cid)
                                .and_then(|c| c.cost)
                                .map_or(false, |cost| match operator {
                                    ">=" => cost >= cost_limit,
                                    "<=" => cost <= cost_limit,
                                    ">" => cost > cost_limit,
                                    "<" => cost < cost_limit,
                                    "!=" => cost != cost_limit,
                                    _ => cost == cost_limit,
                                })
                        });
                        if !cost_match {
                            push_rich(&format!("コスト不一致(limit={})", cost_limit), false);
                            return false;
                        }
                    }
                    // Check card_type if specified (e.g. メンバー → member_card)
                    if let Some(ref ct) = condition.get_card_type() {
                        if !stage_ids.iter().any(|&cid| {
                            util::card_matches_type(
                                &self.game_state.card_database,
                                cid,
                                Some(ct.as_str()),
                            )
                        }) {
                            push_rich(&format!("カード種別不一致(type={})", ct.as_str()), false);
                            return false;
                        }
                    }
                    if let Some(ref groups) = condition.get_group_names() {
                        if !groups.is_empty() {
                            let card_db = &self.game_state.card_database;
                            let all_areas = condition.get_all_areas().unwrap_or(false);
                            let match_fn = |cid: i16| -> bool {
                                groups.iter().any(|g| {
                                    crate::ability::util::card_matches_group_str(
                                        card_db,
                                        cid,
                                        Some(g),
                                    )
                                })
                            };
                            let ok = if all_areas {
                                stage_ids.iter().all(|&cid| match_fn(cid))
                            } else {
                                stage_ids.iter().any(|&cid| match_fn(cid))
                            };
                            if !ok {
                                push_rich(&format!("グループ不一致: {:?}", groups), false);
                                return false;
                            }
                            // Verify that an appeared card matches the group filter.
                            // The group check above ensures SOME card on stage belongs
                            // to the group, but the appearance trigger should only fire
                            // when a card THAT actually appeared this turn matches the group.
                            // If appearance tracking is cleared (resolution time), accept
                            // — the ability was already queued by the trigger event.
                            if !baton_touch_trigger {
                                if self.game_state.cards_appeared_this_turn.is_empty() {
                                    // Resolution: appearance happened (ability was queued)
                                } else {
                                    let has_appeared_matching = stage_ids.iter().any(|&cid| {
                                        match_fn(cid)
                                            && self.game_state.has_card_appeared_this_turn(cid)
                                    });
                                    if !has_appeared_matching {
                                        push_rich("該当グループ未登場", false);
                                        return false;
                                    }
                                }
                            }
                            // Verify that an appeared card matches the group filter.
                            // The group check above ensures SOME card on stage belongs
                            // to the group, but the appearance trigger should only fire
                            // when a card THAT actually appeared this turn matches the group.
                            // Skip during constant evaluation (no cards "appeared").
                            if !baton_touch_trigger
                                && !self.game_state.cards_appeared_this_turn.is_empty()
                            {
                                let has_appeared_matching = stage_ids.iter().any(|&cid| {
                                    match_fn(cid)
                                        && self.game_state.has_card_appeared_this_turn(cid)
                                });
                                if !has_appeared_matching {
                                    push_rich("該当グループ未登場", false);
                                    return false;
                                }
                            }
                            // Prevent self-trigger on own appearance when the condition
                            // explicitly excludes self (e.g. "ほかのメンバー" / exclude_self).
                            // baton_touch appearances legitimately trigger on the activating card.
                            if !baton_touch_trigger
                                && !self.game_state.cards_appeared_this_turn.is_empty()
                                && condition.get_exclude_self().unwrap_or(false)
                            {
                                let has_other_matching = stage_ids.iter().any(|&cid| {
                                    match_fn(cid)
                                        && self
                                            .activating_card_id
                                            .map_or(true, |act_id| cid != act_id)
                                        && self.game_state.has_card_appeared_this_turn(cid)
                                });
                                if !has_other_matching {
                                    push_rich("自カードのみ登場", false);
                                    return false;
                                }
                            }
                        }
                    }
                    // activation_position: independent position requirement for the
                    // activating card itself (e.g. "center" means ability only works
                    // when the card is at center).  This is distinct from the `position`
                    // field, which may be a cross-comparison reference (with position_compare).
                    if let Some(ref act_pos) = condition.get_activation_position() {
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
                            push_rich(&format!("位置不一致: {}", act_pos), false);
                            return false;
                        }
                    }

                    if let Some(ref pos) = condition.get_position() {
                        // When position_compare is set, `position` is a cross-comparison
                        // reference (e.g. compare cost at left_side vs right_side), not
                        // a requirement that the activating card be at that position.
                        // Skip the card-own-position check in that case.
                        if condition.get_position_compare().is_some() {
                            // position is for cross-comparison, not card positioning
                        } else {
                            log::debug!(
                                "[POS_CHECK] pos={:?} get_position={:?}",
                                pos,
                                pos.get_position()
                            );
                            let pos_str = pos.get_position();
                            let pos_idx = match pos_str {
                                Some("left") | Some("leftside") | Some("left_side") => 0,
                                Some("center") | Some("centre") => 1,
                                Some("right") | Some("rightside") | Some("right_side") => 2,
                                _ => {
                                    log::debug!("[APPEARANCE] unknown position: {:?}", pos_str);
                                    push_rich(&format!("不明な位置: {:?}", pos_str), false);
                                    return false;
                                }
                            };

                            let expected = self.activating_card_id;
                            if pos_idx >= player.stage.stage.len()
                                || expected.is_none()
                                || player.stage.stage[pos_idx] != expected.unwrap()
                            {
                                push_rich(&format!("位置不一致(idx={})", pos_idx), false);
                                return false;
                            }
                        } // end else (position without position_compare)
                    }
                    log::debug!(
                        "[PCOND] position={:?} pc_some={} chars={:?} type={:?}",
                        condition.get_position(),
                        condition.get_positions_characters().is_some(),
                        condition.get_characters().map(|v| v.len()),
                        condition.condition_type()
                    );
                    if let Some(pos_chars) = condition.get_positions_characters() {
                        log::debug!(
                            "[POSCHARS] checking {} entries: {:?}",
                            pos_chars.len(),
                            pos_chars
                                .iter()
                                .map(|p| format!("{}@{}", p.character, p.position))
                                .collect::<Vec<_>>()
                        );
                        for pc in pos_chars {
                            let pos_idx = match pc.position.as_str() {
                                "left_side" => 0,
                                "center" => 1,
                                "right_side" => 2,
                                _ => {
                                    push_rich(&format!("不明な位置: {}", pc.position), false);
                                    return false;
                                }
                            };
                            let card_id = player.stage.stage[pos_idx];
                            if card_id == -1 {
                                push_rich(&format!("{}にカードなし", pc.position), false);
                                return false;
                            }
                            let card_name = self
                                .game_state
                                .card_database
                                .get_card(card_id)
                                .map(|c| crate::card::CardDatabase::normalize_name(&c.name));
                            let norm_char =
                                crate::card::CardDatabase::normalize_name(&pc.character);
                            match card_name {
                                Some(ref name) if name.contains(&norm_char) => {}
                                _ => {
                                    push_rich(
                                        &format!(
                                            "{}に{}不在(実際={:?})",
                                            pc.position, pc.character, card_name
                                        ),
                                        false,
                                    );
                                    return false;
                                }
                            }
                        }
                    }
                    if let Some(ref chars) = condition.get_characters() {
                        log::debug!(
                            "[APPEARANCE] checking characters: {:?} against stage_ids={:?}",
                            chars,
                            stage_ids
                        );
                        if chars.is_empty() {
                            let r = !stage_ids.is_empty();
                            push_rich(&format!("ステージ在籍={}", stage_ids.len()), r);
                            return r;
                        }
                        let stage_card_names: Vec<String> = stage_ids
                            .iter()
                            .filter_map(|&cid| {
                                self.game_state
                                    .card_database
                                    .get_card(cid)
                                    .map(|c| crate::card::CardDatabase::normalize_name(&c.name))
                            })
                            .collect();
                        log::debug!("[APPEARANCE] stage card names: {:?}", stage_card_names);
                        let result = chars.iter().all(|name| {
                            let norm = crate::card::CardDatabase::normalize_name(name);
                            stage_card_names.iter().any(|cname| cname.contains(&norm))
                        });
                        log::debug!("[APPEARANCE] result={}", result);
                        if !result {
                            let names = stage_card_names.join(", ");
                            push_rich(
                                &format!("キャラ不在: 期待={:?}, 在籍=[{}]", chars, names),
                                false,
                            );
                            return false;
                        }
                        if let Some(ref ref_char) = condition.get_cost_reference_character() {
                            let subject = chars[0].as_str();
                            let norm_subject = crate::card::CardDatabase::normalize_name(subject);
                            let subject_cost = stage_ids
                                .iter()
                                .filter_map(|&cid| {
                                    let card = self.game_state.card_database.get_card(cid)?;
                                    let norm_name =
                                        crate::card::CardDatabase::normalize_name(&card.name);
                                    if norm_name.contains(&norm_subject) {
                                        card.cost
                                    } else {
                                        None
                                    }
                                })
                                .next();
                            let norm_ref = crate::card::CardDatabase::normalize_name(ref_char);
                            let ref_cost = stage_ids
                                .iter()
                                .filter_map(|&cid| {
                                    let card = self.game_state.card_database.get_card(cid)?;
                                    let norm_name =
                                        crate::card::CardDatabase::normalize_name(&card.name);
                                    if norm_name.contains(&norm_ref) {
                                        card.cost
                                    } else {
                                        None
                                    }
                                })
                                .next();
                            let op = condition
                                .get_cost_reference_operator()
                                .map(|o| o.as_str())
                                .unwrap_or(">");
                            let ok = match (subject_cost, ref_cost) {
                                (Some(sc), Some(rc)) if op == ">" => sc > rc,
                                (Some(sc), Some(rc)) if op == ">=" => sc >= rc,
                                (Some(sc), Some(rc)) if op == "<" => sc < rc,
                                (Some(sc), Some(rc)) if op == "<=" => sc <= rc,
                                _ => false,
                            };
                            log::debug!("[APPEARANCE] cost_compare: subject={} cost={:?} ref={} cost={:?} op={} ok={}",
                                subject, subject_cost, ref_char, ref_cost, op, ok);
                            let _cost_actual = match (subject_cost, ref_cost) {
                                (Some(sc), Some(rc)) => format!("{} {} {}", sc, op, rc),
                                (Some(sc), None) => format!("{} {} ?", sc, op),
                                (None, Some(rc)) => format!("? {} {}", op, rc),
                                (None, None) => format!("コスト未取得"),
                            };
                            push_rich(
                                &format!(
                                    "{}({}) {} {}({}) → {}",
                                    subject,
                                    subject_cost.unwrap_or(0),
                                    if ok { "成立" } else { "不成立" },
                                    ref_char,
                                    ref_cost.unwrap_or(0),
                                    if ok { "成立" } else { "不成立" }
                                ),
                                ok,
                            );
                            ok
                        } else {
                            push_rich(&format!("全キャラ在籍: {:?}", chars), true);
                            true
                        }
                    } else {
                        if let Some(expected_source) = condition.get_appearance_source() {
                            let card_to_check = self.activating_card_id;
                            let ok = card_to_check.map_or(false, |cid| {
                                self.game_state.get_card_appearance_source(cid)
                                    == Some(expected_source)
                            });
                            if !ok {
                                push_rich(
                                    &format!("登場元不一致: 期待={}", expected_source),
                                    false,
                                );
                                return false;
                            }
                        }
                        let names = stage_ids
                            .iter()
                            .filter_map(|&cid| {
                                self.game_state
                                    .card_database
                                    .get_card(cid)
                                    .map(|c| c.name.to_string())
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        push_rich(&format!("在籍キャラ: [{}]", names), true);
                        let stage_occupied = !stage_ids.is_empty();
                        if stage_occupied {
                            if let Some(ref prop) = condition.get_card_property() {
                                if !self.moved_cards.is_empty() {
                                    let has_prop = self.moved_cards.iter().any(|&cid| {
                                        self.game_state.card_database.get_card(cid).is_some_and(
                                            |c| match *prop {
                                                CardProperty::HasBladeHeart => c.has_blade_heart(),
                                                CardProperty::HasScoreIcon => c.has_score_icon(),
                                                CardProperty::HasAllBlade => c.has_all_blade(),
                                            },
                                        )
                                    });
                                    if condition.get_negation().unwrap_or(false) == has_prop {
                                        push_rich(
                                            &format!("card_property={} unmet", prop.as_str()),
                                            false,
                                        );
                                        return false;
                                    }
                                }
                            }
                        }
                        stage_occupied
                    }
                }
                Some(Zone::Hand) => {
                    let r = !player.hand.cards.is_empty();
                    push_rich(&format!("手札枚数={}", player.hand.cards.len()), r);
                    r
                }
                Some(Zone::Discard) => {
                    let r = !player.waitroom.cards.is_empty();
                    push_rich(&format!("控え室枚数={}", player.waitroom.cards.len()), r);
                    r
                }
                _ => {
                    push_rich("他ゾーン", true);
                    true
                }
            }
        } else {
            match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    let r = player.stage.stage[0] == -1
                        && player.stage.stage[1] == -1
                        && player.stage.stage[2] == -1;
                    push_rich(
                        if r {
                            "不在=全エリア空"
                        } else {
                            "不在≠全エリア空"
                        },
                        r,
                    );
                    r
                }
                Some(Zone::Hand) => {
                    let r = player.hand.cards.is_empty();
                    push_rich(&format!("手札={}", player.hand.cards.len()), r);
                    r
                }
                Some(Zone::Discard) => {
                    let r = player.waitroom.cards.is_empty();
                    push_rich(&format!("控え室={}", player.waitroom.cards.len()), r);
                    r
                }
                _ => {
                    push_rich("他ゾーン不在", true);
                    true
                }
            }
        }
    }

    pub(crate) fn evaluate_ability_filter_condition(&self, condition: &Condition) -> bool {
        let target = condition.get_target().unwrap_or("self");
        let card_db = &self.game_state.card_database;
        let player = self.game_state.resolve_target_player(target);
        let filter = condition
            .get_ability_filter()
            .unwrap_or(&AbilityFilter::NoAbility);

        let location = condition.get_location().unwrap_or(Zone::Stage.to_str());
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

        let has_ability = if let Some(card_id) = self.game_state.activating_card {
            if let Some(card) = card_db.get_card(card_id) {
                !card.abilities.is_empty()
            } else {
                false
            }
        } else {
            if card_ids.is_empty() {
                return false;
            }
            card_ids.iter().any(|&id| {
                card_db
                    .get_card(id)
                    .map(|c| !c.abilities.is_empty())
                    .unwrap_or(false)
            })
        };

        match filter {
            AbilityFilter::NoAbility => !has_ability,
            AbilityFilter::HasAbility => has_ability,
            AbilityFilter::NoAbilityType if has_ability => {
                !self.card_has_matching_ability_type(condition, filter)
            }
            _ => true,
        }
    }

    pub(crate) fn card_has_matching_ability_type(
        &self,
        condition: &Condition,
        _filter: &AbilityFilter,
    ) -> bool {
        let excluded_triggers: Vec<&str> = condition
            .get_ability_filter_triggers()
            .as_ref()
            .map(|t| t.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default();
        if excluded_triggers.is_empty() {
            return false;
        }
        if let Some(card_id) = self.game_state.activating_card {
            let card_db = &self.game_state.card_database;
            if let Some(card) = card_db.get_card(card_id) {
                card.abilities.iter().any(|ar| {
                    ar.resolve()
                        .triggers
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
        let target = condition.get_target().unwrap_or("self");
        let player = self.game_state.resolve_target_player(target);

        let location = condition.get_location().unwrap_or(Zone::Stage.to_str());
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

        let operator = condition.get_operator().unwrap_or("any");
        let count_needed = condition.get_count().unwrap_or(1);

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
                            "no_ability_type" if has_ability => {
                                let excluded_triggers: Vec<&str> = condition
                                    .get_ability_filter_triggers()
                                    .as_ref()
                                    .map(|t| t.iter().map(|s| s.as_str()).collect())
                                    .unwrap_or_default();
                                !c.abilities.iter().any(|ar| {
                                    ar.resolve()
                                        .triggers
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
            .count() as u8;

        match operator {
            "=" => match_count == count_needed,
            ">=" => match_count >= count_needed,
            "<=" => match_count <= count_needed,
            ">" => match_count > count_needed,
            "<" => match_count < count_needed,
            _ => match_count >= count_needed,
        }
    }

    pub(crate) fn zone_len(&self, player: &crate::player::Player, location: &str) -> u8 {
        match Zone::from_str(location) {
            Some(Zone::Stage) => player.stage.total_blades(
                &self.game_state.card_database,
                &self.game_state.mods.blade_modifiers,
                &self.game_state.mods.orientation_modifiers,
                true,
            ),
            Some(Zone::Hand) => player.hand.len() as u8,
            Some(Zone::Deck) => player.main_deck.len() as u8,
            Some(Zone::Discard) => player.waitroom.len() as u8,
            Some(Zone::Energy) => player.energy_zone.cards.len() as u8,
            Some(Zone::LiveCardZone) => player.live_card_zone.len() as u8,
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.len() as u8,
            Some(Zone::RevealedCards) => self.game_state.revealed_cards.len() as u8,
            _ => 0,
        }
    }

    pub(crate) fn card_matches_count_filters(
        &self,
        card_id: i16,
        card_type_filter: Option<&str>,
        group_names: Option<&[String]>,
        heart_colors: &[String],
        cost_limit: Option<u8>,
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
            && condition.get_original_value().unwrap_or(false)
            && !self.check_original_blade_filter(condition, card_id)
        {
            return false;
        }
        if respect_original_value
            && condition.get_original_value().unwrap_or(false)
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
        cost_limit: Option<u8>,
        cost_limit_operator: Option<&str>,
        exclude_self: Option<i16>,
        respect_original_value: bool,
        condition: &Condition,
    ) -> u8 {
        // Build a single CardFilter and use its .count() — avoids re-parsing
        // the filter fields for every card in the slice. Base it on the
        // condition's own filter (filter_subset → the single CardFilter builder)
        // so characters/heart_colors/etc. are never silently dropped.
        let mut filter = condition.filter_subset();
        if let Some(ct) = card_type_filter {
            filter.card_type = Some(ct);
        }
        if let Some(gn) = group_names {
            filter.group = gn.first().map(|s| s.as_str());
        }
        if !heart_colors.is_empty() {
            filter.heart_colors = heart_colors;
        }
        if let Some(cl) = cost_limit {
            filter.cost_limit = Some(cl);
        }
        if let Some(co) = cost_limit_operator {
            filter.cost_operator = Some(co);
        }
        if let Some(ex) = exclude_self {
            filter.exclude_self = Some(ex);
        }
        // card_property + negation live on the Condition variant (not in the
        // flat EffectFilter that filter_subset reads), so re-apply them here.
        if let Some(cp) = &condition.get_card_property() {
            filter.card_property = Some(cp.as_str());
            filter.negation = condition.get_negation().unwrap_or(false);
        }
        let card_db = &self.game_state.card_database;
        let mut count = 0u8;
        for &card_id in cards {
            if !filter.matches(card_db, card_id, true) {
                continue;
            }
            if respect_original_value
                && condition.get_original_value().unwrap_or(false)
                && !self.check_original_blade_filter(condition, card_id)
            {
                continue;
            }
            if respect_original_value
                && condition.get_original_value().unwrap_or(false)
                && !self.check_original_heart_filter(condition, card_id)
            {
                continue;
            }
            count += 1;
        }
        count
    }

    /// Count distinct names/costs/groups among cards matching the condition's filters.
    pub(crate) fn count_distinct_in_cards(
        &self,
        cards: &[i16],
        condition: &Condition,
        card_type: Option<&str>,
        group_names: Option<&[String]>,
    ) -> u8 {
        let card_db = &self.game_state.card_database;
        // Collect matching cards first
        let matching: Vec<i16> = cards
            .iter()
            .filter(|&&cid| {
                cid != -1
                    && card_type.is_none_or(|ct| util::card_matches_type(card_db, cid, Some(ct)))
                    && group_names.is_none_or(|gn| {
                        gn.iter()
                            .any(|g| util::card_matches_group_str(card_db, cid, Some(g.as_str())))
                    })
            })
            .copied()
            .collect();
        let distinct_type = match condition.get_distinct() {
            Some(crate::core::card::DistinctInfo::String(s)) => s.as_str(),
            _ if condition.get_group_reference() == Some("different_group_names") => "group_name",
            _ => "card_name",
        };
        match distinct_type {
            "cost" => {
                let mut seen: HashSet<u8> = HashSet::default();
                for &cid in &matching {
                    if let Some(card) = card_db.get_card(cid) {
                        let cost = card.cost.unwrap_or(0);
                        let modified = (cost as i32 + self.game_state.mods.get_cost_modifier(cid))
                            .max(0) as u8;
                        seen.insert(modified);
                    }
                }
                seen.len() as u8
            }
            "group_name" => {
                let mut seen: HashSet<String> = HashSet::default();
                for &cid in &matching {
                    if let Some(card) = card_db.get_card(cid) {
                        seen.insert(card.group.to_string());
                    }
                }
                seen.len() as u8
            }
            _ => {
                let name_sets: Vec<Vec<String>> = matching
                    .iter()
                    .map(|&cid| card_db.get_card_names(cid))
                    .collect();
                util::max_distinct_names(&name_sets).distinct as u8
            }
        }
    }

    pub(crate) fn sum_group_hearts_in_stage(
        &self,
        player: &crate::player::Player,
        group_name: Option<&str>,
    ) -> u8 {
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
                    .map(|bh| bh.hearts.values_sum())
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
    ) -> u8 {
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
                            crate::card::CardDatabase::normalize_name(&card.name)
                                .contains(&crate::card::CardDatabase::normalize_name(e.as_str()))
                                || card.card_no.contains(e.as_str())
                        }) {
                            log::debug!(
                                "[COUNT_EXCLUDE] excluding card {} (name={})",
                                card_id,
                                card.name
                            );
                            return false;
                        }
                    }
                }
                true
            })
            .count() as u8;
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
    ) -> u8 {
        let zone = Zone::from_str(location);
        match comparison_type {
            Some("score") => {
                let is_live_zone = zone.map_or(true, |z| {
                    matches!(z, Zone::LiveCardZone | Zone::SuccessLiveZone)
                });

                if is_live_zone {
                    let is_p1 = player.id == self.game_state.player1.id;
                    let cheer_blade = if is_p1 {
                        self.game_state.player1_cheer_blade_heart_count
                    } else {
                        self.game_state.player2_cheer_blade_heart_count
                    };
                    let constant_bonus = if is_p1 {
                        self.game_state.mods.p1_constant_total_score_bonus
                    } else {
                        self.game_state.mods.p2_constant_total_score_bonus
                    };

                    let score_flat: HashMap<i16, i32> = self
                        .game_state
                        .mods
                        .score_modifiers
                        .iter()
                        .map(|(&k, e)| (k, e.total()))
                        .collect();

                    let live_score = player.live_card_zone.calculate_live_score(
                        &self.game_state.card_database,
                        0,
                        player.stage_hearts.as_ref(),
                        Some(&self.game_state.mods.need_heart_modifiers),
                        Some(&score_flat),
                        0,
                    );

                    let success_score = {
                        let mut total = 0u8;
                        for &card_id in player.success_live_card_zone.cards.iter() {
                            if let Some(card) = self.game_state.card_database.get_card(card_id) {
                                let base = card.get_score() as i32;
                                let modifier = score_flat.get(&card_id).copied().unwrap_or(0);
                                total += (base + modifier).max(0) as u8;
                            }
                        }
                        total
                    };

                    return match zone {
                        Some(Zone::LiveCardZone) => player.live_card_zone.calculate_live_score(
                            &self.game_state.card_database,
                            cheer_blade,
                            player.stage_hearts.as_ref(),
                            Some(&self.game_state.mods.need_heart_modifiers),
                            Some(&score_flat),
                            constant_bonus,
                        ),
                        Some(Zone::SuccessLiveZone) => success_score,
                        None => {
                            live_score + success_score + cheer_blade + constant_bonus.max(0) as u8
                        }
                        _ => 0,
                    };
                }

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
                total_cost.max(0) as u8
            }
            Some("energy") => player.energy_zone.cards.len() as u8,
            _ => self.zone_len(player, location),
        }
    }

    pub(crate) fn get_count_for_condition(&self, condition: &Condition) -> u8 {
        let location = condition.get_location().unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let comparison_type = condition.get_comparison_type();
        let resource_type = condition.get_resource_type();
        if comparison_type == Some("score") {
            return self.get_count_for_target(condition, target);
        }
        if comparison_type == Some("cost") {
            if condition.get_location().is_none()
                || condition.get_location() == Some("revealed_cards")
            {
                let card_db = &self.game_state.card_database;
                let ct = condition.get_card_type().map(|ct| ct.as_str());
                return self
                    .game_state
                    .revealed_cards
                    .iter()
                    .filter(|&&id| ct.is_none() || util::card_matches_type(card_db, id, ct))
                    .map(|&id| {
                        let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
                        (base + self.game_state.mods.get_cost_modifier(id)).max(0) as u8
                    })
                    .sum();
            }
            // comparison_type=cost + comparison_target=self means individual
            // cost comparison ("このメンバーよりコストの大きいメンバー").
            // Return the MAXIMUM cost among matching cards so the caller
            // can compare it against the self card's cost via compare_counts.
            // When comparison_target != "self", the condition uses total sum
            // (handled by count_for_player_target instead).
            if condition.get_comparison_target() == Some(ComparisonTarget::Self_) {
                let player = self.resolve_condition_player(target);
                let card_db = &self.game_state.card_database;
                let location = condition.get_location().unwrap_or("stage");
                let mut cards = util::zone_card_ids(&player, location);
                if Zone::from_str(location) == Some(Zone::Stage) {
                    cards.retain(|&id| id != -1);
                }
                if cards.is_empty() {
                    return 0;
                }
                let exclude_id = if condition.get_exclude_self().unwrap_or(false) {
                    self.activating_card_id
                } else {
                    None
                };
                let mut filter = condition.filter_subset();
                filter.exclude_self = exclude_id;
                let groups_vec = condition.get_group_names().map(|v| v.to_vec());
                filter.groups = groups_vec.as_ref();
                let max_cost = cards
                    .iter()
                    .filter(|&&id| filter.matches(card_db, id, false))
                    .filter_map(|&id| {
                        let base = card_db.get_card(id).and_then(|c| c.cost)?;
                        Some(
                            (base as i32 + self.game_state.mods.get_cost_modifier(id)).max(0) as u8,
                        )
                    })
                    .max()
                    .unwrap_or(0);
                return max_cost;
            }
            // Fallback for cost comparison with specific location and non-self
            // target (e.g. "自分のステージにいる『蓮ノ空』のメンバーのコストの
            // 合計が、相手のステージにいるメンバーのコストの合計より高い").
            // Sum modified costs with group/type filtering.
            if let Some(loc) = condition.get_location() {
                if !loc.is_empty() {
                    let player = self.resolve_condition_player(target);
                    let card_db = &self.game_state.card_database;
                    let mut cards = util::zone_card_ids(&player, loc);
                    if Zone::from_str(loc) == Some(Zone::Stage) {
                        cards.retain(|&id| id != -1);
                    }
                    let total: u8 = cards
                        .iter()
                        .filter(|&&id| {
                            if let Some(ref groups) = condition.get_group_names() {
                                if !groups.is_empty()
                                    && !groups
                                        .iter()
                                        .any(|g| util::card_matches_group_str(card_db, id, Some(g)))
                                {
                                    return false;
                                }
                            }
                            if let Some(ref ct) = condition.get_card_type() {
                                if !util::card_matches_type(card_db, id, Some(ct)) {
                                    return false;
                                }
                            }
                            true
                        })
                        .map(|&id| {
                            let base =
                                card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
                            (base + self.game_state.mods.get_cost_modifier(id)).max(0) as u8
                        })
                        .sum();
                    return total;
                }
            }
        }
        if resource_type == Some("hand_count") {
            let player = self.resolve_condition_player(target);
            return player.hand.len() as u8;
        }
        if let Some(rt) = resource_type {
            if rt.starts_with("heart") {
                let clean: String = rt.chars().filter(|&c| c != '_').collect();
                let color = crate::card::parse_heart_color(&clean);
                let player = self.resolve_condition_player(target);
                let card_db = &self.game_state.card_database;
                let sum_all = || -> u8 {
                    player
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
                        .sum()
                };
                let count = if let Some(ref pos) = condition.get_position() {
                    if let Some(p) = pos.get_position() {
                        if let Some(idx) = util::stage_position_index(p) {
                            let cid = player.stage.stage[idx];
                            if cid != -1 {
                                card_db
                                    .get_card(cid)
                                    .and_then(|c| c.base_heart.as_ref())
                                    .map(|bh| bh.hearts.get(&color).copied().unwrap_or(0))
                                    .unwrap_or(0)
                            } else {
                                0
                            }
                        } else {
                            sum_all()
                        }
                    } else {
                        sum_all()
                    }
                } else {
                    sum_all()
                };
                return count;
            }
        }
        if resource_type == Some("surplus_heart") {
            // delta=true means the condition should check the count that was LOST
            // by the preceding action, not the current absolute surplus.
            if condition.get_delta() == Some(true) {
                return self.game_state.mods.last_surplus_loss_count;
            }
            // After live clearance the computed surplus count is stored on GameState.
            // Prefer the stored snapshot value over a runtime recalculation
            // (includes yell ALL hearts that can fill any color gap).
            if self.game_state.live_surplus_ready_this_turn {
                // Q174: When a heart_colors filter is present (e.g. heart04 for La Bella
                // Patria), read the per-color surplus from the performance snapshot rather
                // than returning the total surplus. ALL hearts (icon_all) are consumed
                // during Phase 4 of allocation to fill color deficits and do NOT contribute
                // to colored surplus; see rules 8.3.15.1.1-8.3.15.1.2. The "treated as any
                // color" in 8.3.15.1.1 is a check-time fiction — surplus is computed from
                // the actual pool indices (colored = 1..6, ALL = 7), not from the
                // functional assignment used during the check.
                // snap.surplus_hearts is populated at live.rs:~148 before LiveSuccess
                // abilities fire, so it is safe to read here.
                if let Some(colors) = condition.get_heart_colors() {
                    let player = self.resolve_condition_player(target);
                    let player_id = &player.id;
                    for snap in &self.game_state.performance_snapshots {
                        if &snap.player_id == player_id {
                            let mut total = 0u8;
                            for hc_str in colors {
                                let color = crate::card::parse_heart_color(hc_str);
                                total += snap.surplus_hearts[color.index()];
                            }
                            return total;
                        }
                    }
                }
                return match target {
                    "opponent" => self.game_state.opponent_live_surplus_count,
                    _ => {
                        let player = self.resolve_condition_player(target);
                        if player.id == self.game_state.player1.id {
                            self.game_state.self_live_surplus_count
                        } else {
                            self.game_state.opponent_live_surplus_count
                        }
                    }
                };
            }
            // Fallback: compute from current state (before live clearance or if snapshot
            // unavailable). Supports color-specific surplus via condition.get_heart_colors().
            let player = self.resolve_condition_player(target);
            let card_db = &self.game_state.card_database;

            if let Some(colors) = condition.get_heart_colors() {
                let mut total = 0u8;
                for hc_str in colors {
                    let color = crate::card::parse_heart_color(hc_str);
                    let member_of_color: u8 = player
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
                    let needed_of_color: u8 = player
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

            let member_hearts: u8 = player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
                .sum();
            let needed: u8 = player
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
            let count = player.energy_zone.cards.len() as u8;
            log::debug!(
                "[GET_COUNT] resource_type=energy target={} → {}",
                target,
                count
            );
            return count;
        }
        // For preceding_moved conditions, count matching cards in moved_cards
        // rather than summing costs (fixes wrong log display for card_count_condition).
        if location.is_empty()
            && (condition.get_source() == Some("preceding_moved")
                || condition.get_source() == Some("previous_moved_cards"))
            && !self.moved_cards.is_empty()
        {
            let ct = condition.get_card_type().map(|ct| ct.as_str());
            let hc: &[String] = condition.get_heart_colors().unwrap_or(&[]);
            let card_db = &self.game_state.card_database;
            let count: u8 = self
                .moved_cards
                .iter()
                .filter(|&&cid| {
                    if cid == -1 {
                        return false;
                    }
                    if let Some(card_type) = ct {
                        if !util::card_matches_type(card_db, cid, Some(card_type)) {
                            return false;
                        }
                    }
                    if !hc.is_empty() && !util::card_matches_heart_colors(card_db, cid, hc) {
                        return false;
                    }
                    if !util::card_matches_characters(card_db, cid, condition.get_characters()) {
                        return false;
                    }
                    true
                })
                .count() as u8;
            return count;
        }

        // Fallback: when revealed_cards was consumed by a preceding move_cards
        // (Kosuzu pattern: step 2 moves the card from revealed_cards to hand,
        // then step 3 needs to check the SAME card's cost against the chosen
        // number). Try moved_cards as a fallback source.
        if location.is_empty()
            && self.game_state.revealed_cards.is_empty()
            && !self.moved_cards.is_empty()
        {
            let card_db = &self.game_state.card_database;
            let total: u8 = self
                .moved_cards
                .iter()
                .filter_map(|&id| {
                    let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
                    Some((base + self.game_state.mods.get_cost_modifier(id)).max(0) as u8)
                })
                .sum();
            if total > 0 {
                return total;
            }
        }
        // Fallback for conditions referencing revealed cards without
        // explicit comparison_type (e.g. "選んだ数以下の場合" in Kosuzu).
        // Use the cost of revealed cards when available.
        if location.is_empty() && !self.game_state.revealed_cards.is_empty() {
            let card_db = &self.game_state.card_database;
            let total: u8 = self
                .game_state
                .revealed_cards
                .iter()
                .filter_map(|&id| {
                    let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32;
                    Some((base + self.game_state.mods.get_cost_modifier(id)).max(0) as u8)
                })
                .sum();
            return total;
        }
        let player = self.resolve_condition_player(target);
        self.zone_len(player, location)
    }

    pub(crate) fn get_count_for_target(&self, condition: &Condition, target: &str) -> u8 {
        let location = condition.get_location().unwrap_or("");
        let resource_type = condition.get_resource_type();
        let comparison_type = condition.get_comparison_type();
        if resource_type == Some("energy") {
            let player = self.resolve_condition_player(target);
            return player.energy_zone.cards.len() as u8;
        }
        let player = self.resolve_condition_player(target);
        let mut count = self.count_for_player_target(player, location, comparison_type);
        if count == 0 && location.is_empty() {
            if let Some(locs) = condition.get_locations() {
                let mut combined: Vec<i16> = Vec::new();
                for loc in locs {
                    let cards = crate::ability::util::zone_cards(player, loc.as_str());
                    combined.extend_from_slice(cards);
                }
                let card_db = &self.game_state.card_database;
                let card_type_filter = condition.get_card_type().map(|ct| ct.as_str());
                let mut filter = condition.filter_subset();
                filter.card_type = card_type_filter;
                filter.cost_operator = condition.get_operator();
                count = crate::ability::util::count_matching(&combined, card_db, &filter, false);
            }
        }
        count
    }

    pub(crate) fn get_group_card_count(&self, condition: &Condition) -> u8 {
        let group_filter = condition.get_group_names();
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let group_name = group_filter.and_then(|g| g.first().map(|s| s.as_str()));

        let is_aggregate = condition.get_aggregate() == Some("total");

        if is_aggregate {
            let location = condition.get_location().unwrap_or("");
            let ct = condition.get_card_type().map(|ct| ct.as_str());
            return match Zone::from_str(location) {
                Some(Zone::Stage) => self.sum_group_hearts_in_stage(player, group_name),
                Some(Zone::LiveCardZone) => {
                    let card_db = &self.game_state.card_database;
                    player
                        .live_card_zone
                        .cards
                        .iter()
                        .filter(|&&cid| cid != -1)
                        .filter(|&&cid| ct.is_none() || util::card_matches_type(card_db, cid, ct))
                        .filter(|&&cid| {
                            group_name.is_none()
                                || util::card_matches_group_str(card_db, cid, group_name)
                        })
                        .filter_map(|&cid| card_db.get_card(cid))
                        .map(|card| {
                            card.need_heart
                                .as_ref()
                                .map(|nh| nh.hearts.values_sum())
                                .unwrap_or(0)
                        })
                        .sum()
                }
                Some(Zone::SuccessLiveZone) => {
                    let card_db = &self.game_state.card_database;
                    player
                        .success_live_card_zone
                        .cards
                        .iter()
                        .filter(|&&cid| cid != -1)
                        .filter(|&&cid| ct.is_none() || util::card_matches_type(card_db, cid, ct))
                        .filter(|&&cid| {
                            group_name.is_none()
                                || util::card_matches_group_str(card_db, cid, group_name)
                        })
                        .filter_map(|&cid| card_db.get_card(cid))
                        .map(|card| card.score.unwrap_or(0) as u8)
                        .sum()
                }
                _ => 0,
            };
        }

        let ct = condition.get_card_type().map(|ct| ct.as_str());
        let exc = condition.get_exclude_characters();

        // Preceding-moved path: check recently moved cards instead of a zone.
        if condition.get_source() == Some("preceding_moved")
            || condition.get_source() == Some("previous_moved_cards")
        {
            let moved_source: SmallVec<[i16; 8]> = if self.moved_cards.is_empty() {
                let enqueued = self.game_state.entry_trigger_moved_cards();
                let global = self.game_state.recently_moved_cards.clone();
                match (&enqueued, &global) {
                    (Some(ev), None) if !ev.is_empty() => ev.iter().copied().collect(),
                    _ => global.map_or_else(SmallVec::new, |g| g.iter().copied().collect()),
                }
            } else {
                self.moved_cards.iter().copied().collect()
            };
            return self.count_group_cards_in_cards(&moved_source, group_filter, ct, exc);
        }

        // When `locations` has multiple entries, count across all listed zones.
        // This handles zone-transition conditions (e.g. "card in live_card_zone OR discard").
        if let Some(locs) = condition.get_locations() {
            if locs.len() >= 2 {
                let mut total = 0u8;
                for loc in locs {
                    let cards = crate::ability::util::zone_cards(player, loc.as_str());
                    total += self.count_group_cards_in_cards(cards, group_filter, ct, exc);
                }
                return total;
            }
        }

        let location = condition.get_location().unwrap_or("");
        // Default to stage when location is empty but position is set
        // (e.g. "センターエリアにμ'sのメンバーがいる場合" → position=center, location is inferred as stage).
        let location = if location.is_empty() && condition.get_position().is_some() {
            "stage"
        } else {
            location
        };
        match Zone::from_str(location) {
            Some(Zone::Stage) => {
                self.count_group_cards_in_cards(&player.stage.stage, group_filter, ct, exc)
            }
            Some(Zone::Hand) => {
                self.count_group_cards_in_cards(&player.hand.cards, group_filter, ct, exc)
            }
            Some(Zone::Discard) | Some(Zone::Waitroom) => {
                self.count_group_cards_in_cards(&player.waitroom.cards, group_filter, ct, exc)
            }
            Some(Zone::LiveCardZone) => {
                self.count_group_cards_in_cards(&player.live_card_zone.cards, group_filter, ct, exc)
            }
            Some(Zone::SuccessLiveZone) => self.count_group_cards_in_cards(
                &player.success_live_card_zone.cards,
                group_filter,
                ct,
                exc,
            ),
            Some(Zone::Energy) => {
                self.count_group_cards_in_cards(&player.energy_zone.cards, group_filter, ct, exc)
            }
            _ => 0,
        }
    }

    /// Evaluate resource_condition: check resource count (e.g. total blades)
    /// against the operator and count threshold.
    /// Used by parser issue #10 where "blade total >= 10" should be a resource condition.
    pub(crate) fn evaluate_resource_condition(&self, condition: &Condition) -> bool {
        let resource = condition.get_resource_type().unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let card_db = &self.game_state.card_database;
        let location = condition.get_location().unwrap_or("stage");
        let op = condition.get_operator();
        let threshold = condition.get_count().unwrap_or(1);

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
                let bm_flat: HashMap<i16, i32> = self
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
                        (base + modifier).max(0) as u8
                    })
                    .sum()
            }
            "surplus_heart" => {
                // When delta=true, read the count saved before the preceding step
                // zeroed the surplus, instead of computing from current state.
                if condition.get_delta() == Some(true) {
                    let count = self.game_state.mods.last_surplus_loss_count;
                    return compare_counts(
                        condition.get_operator(),
                        count,
                        condition.get_count().unwrap_or(1),
                    );
                }
                // Count surplus hearts for the target player
                let heart_total: u8 = player
                    .stage
                    .stage
                    .iter()
                    .filter(|&&id| id != -1)
                    .map(|&id| card_db.get_card(id).map(|c| c.total_hearts()).unwrap_or(0))
                    .sum();
                let need: u8 = player
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
