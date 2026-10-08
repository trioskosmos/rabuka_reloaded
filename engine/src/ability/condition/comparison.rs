use super::{comparison_default_count, ConditionContext};
use crate::ability::enums::Zone;
use crate::ability::util::{self, compare_counts};
use crate::ability::queue::ConditionalChoice;
use crate::card::{
    ComparisonTarget, Condition, HeartColor,
};
use crate::core::constants::CountCast;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
use smallvec::SmallVec;

/// Every colour `get_heart_modifier` can be keyed by, in one place.
///
/// `check_original_heart_filter` sums ability-granted hearts over all of them on every
/// call; the list lived inline as an array literal, which meant the set of colours this
/// reads was invisible at the point where anyone would want to check it.
const ORIGINAL_HEART_COLORS: [HeartColor; 8] = [
    HeartColor::Heart01,
    HeartColor::Heart02,
    HeartColor::Heart03,
    HeartColor::Heart04,
    HeartColor::Heart05,
    HeartColor::Heart06,
    HeartColor::Heart00,
    HeartColor::All,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeartTotal {
    None,
    All,
    Value(u8),
}

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
                    .and_then(|c| c.score) == Some(val)
            })
        })
    }

    pub(crate) fn evaluate_comparison_condition(&self, condition: &Condition) -> bool {
        if let Some(v) = self.evaluate_check_self_condition(condition) {
            return v;
        }
        // Per-turn cost-payment history: 「このメンバーが、自分のカードの能力のコストを
        // 支払うため」. PL!-pb2-021-N 南ことり.
        //
        // This lives HERE, in the comparison evaluator, and that placement is the whole point.
        // The clause is a per-turn history of who PAID a cost, so the parser emits a
        // `comparison_condition` carrying only the `paid_cost_this_turn` flag — no
        // `comparison_type`, no operator, no bound. Without this branch it fell straight
        // through to the numeric path below, where `compare_counts` reads a missing operator
        // as `>=` and there was nothing to compare against: `cost >= 0`. True for every
        // card, every turn, so the enclosing `or_condition` was permanently satisfied and her
        // 自動 fired unconditionally, granting a blade for free.
        //
        // The clause is about the PAYER, so membership is tested on the activating card — the
        // member whose 自動/起動 is resolving — and not on anything scanned off the board. The
        // tracker is a set, so a second payment does not read as a different member.
        if condition.get_paid_cost_this_turn() == Some(true) {
            let payer = self
                .activating_card_id
                .or_else(|| self.game_state.ability_queue.current_entry().and_then(|e| e.card_id));
            let paid = match payer {
                Some(cid) => self.game_state.cards_paid_cost_this_turn.contains(&cid),
                None => false,
            };
            let need = condition.get_count().unwrap_or(1).max(1) as usize;
            return usize::from(paid) >= need;
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
        if let Some(pos) = condition.get_position() {
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
        if let Some(values) = condition.get_values() {
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
                        .is_some_and(|s| values.contains(&s))
                });
            }
            // Generic 「A、Bのいずれかの場合」 membership — the ladder test 千砂都
            // `PL!SP-bp1-003-P` needs (「コストの合計が、10、20、30、40、50のいずれか」), and the
            // only shape `compare_counts` cannot express.
            //
            // NOTE this returns WITHOUT applying `negation`, unlike the ordinary path below
            // (which inverts at :153). Nothing in the corpus carries `values` and
            // `negation` together today, so this is latent rather than live — but a 「〜の
            // いずれでもない場合」 clause would parse into exactly this pair and silently lose
            // its negation. If you add one: apply `negation` here too, or route it through
            // the shared path below.
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
        super::super::log::push_cond_verdict(
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
        let effective = crate::constants::effective_stat(
            printed,
            self.game_state.mods.get_cost_modifier(act),
        );
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
        compare_counts(Some(op), effective, threshold)
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
            let current = crate::constants::effective_stat(
                original,
                self.game_state.mods.get_score_modifier(id),
            );
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
        if let Some(groups) = condition.get_group_names() {
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
        if !player.stage.stage.contains(&act_id) {
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
            // A cost reduction is a signed modifier; `saturating_sub` on the
            // u8 printed value already floors at 0, and `u8_count` keeps a
            // negative delta (a cost INCREASE) as "no reduction" rather than
            // subtracting it a second time.
            base.saturating_sub(
                self.game_state
                    .mods
                    .get_cost_modifier(act_id)
                    .max(0)
                    .u8_count(),
            )
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
        // `i32` because the result is compared against a parsed threshold and
        // a wide card list must not itself be the thing that overflows. Each
        // per-card value is already a saturated `u8`.
        cards
            .iter()
            .filter(|&&id| self.matches_condition_groups_and_type(condition, id))
            .map(|&id| {
                i32::from(crate::constants::effective_stat(
                    self.printed_cost(id),
                    self.game_state.mods.get_cost_modifier(id),
                ))
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
        let master_cost = self.effective_condition_cost(master_id);
        let front_cost = self.effective_condition_cost(front_card_id);
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


impl<'a> ConditionContext<'a> {
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

        // `i16` accumulates the per-colour entries of ONE card. Each entry is
        // an `i16` already; the map holds a handful of colours, so this cannot
        // leave the modifier's own range in play.
        let modifier_total: i16 = self
            .game_state
            .mods
            .heart_modifiers
            .get(&card_id)
            .map(|hm| hm.values().map(|e| e.total()).sum())
            .unwrap_or(0);

        HeartTotal::Value(crate::constants::effective_stat(base_sum, modifier_total))
    }

    /// The total effective cost of a set of cards, saturating at 255.
    ///
    /// "Effective" means after the constant cost modifiers: a comparison the
    /// card text writes is about what the card actually costs in this state.
    pub(crate) fn total_effective_cost(&self, ids: &[i16]) -> u8 {
        let card_db = &self.game_state.card_database;
        ids.iter()
            .map(|&id| {
                let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0);
                crate::constants::effective_stat(
                    base,
                    self.game_state.mods.get_cost_modifier(id),
                )
            })
            .sum()
    }

    /// Stage members eligible as "others" for greater-than-all comparisons:
    /// non-empty slots matching `card_type`, minus `exclude_id`, optionally
    /// including the opponent's stage. Shared by the heart and blade variants.
    fn collect_other_stage_ids(
        &self,
        exclude_id: Option<i16>,
        card_type: Option<&str>,
        include_opponent: bool,
    ) -> Vec<i16> {
        let card_db = &self.game_state.card_database;
        let eligible = |player: &crate::player::Player| {
            player
                .stage
                .stage
                .iter()
                .filter(|&&cid| {
                    cid != -1
                        && Some(cid) != exclude_id
                        && util::card_matches_type(card_db, cid, card_type)
                })
                .copied()
                .collect::<Vec<i16>>()
        };
        let mut ids = eligible(self.resolve_condition_player("self"));
        if include_opponent {
            ids.extend(eligible(self.resolve_condition_player("opponent")));
        }
        ids
    }

    pub(crate) fn evaluate_heart_greater_than_all(&self, condition: &Condition, is_both: bool) -> bool {
        let activating_id = match self.activating_card_id {
            Some(id) => id,
            None => return false,
        };

        let self_hearts = self.get_card_total_hearts(activating_id);
        if self_hearts == HeartTotal::None {
            return false;
        }

        // Note: the heart variant only drops the activator itself when the
        // condition explicitly says exclude_self — the blade variant always
        // excludes its reference member.
        let excl_self = condition.get_exclude_self().unwrap_or(false);
        let exclude_id = if excl_self {
            self.activating_card_id
        } else {
            None
        };
        let other_ids = self.collect_other_stage_ids(
            exclude_id,
            condition.get_card_type().map(|ct| ct.as_str()),
            is_both,
        );

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

    /// G11: "そのメンバーが、自分と相手のステージにいるほかのすべてのメンバー
    /// より多くのブレードを持つ場合" — the SELECTED member (from a preceding
    /// select step) has strictly MORE blade than every other stage member on
    /// both sides. Reference is the last selected card, not the activating card
    /// (which for a live-success live card is the live card itself).
    pub(crate) fn evaluate_blade_greater_than_all(&self, condition: &Condition, is_both: bool) -> bool {
        let reference_id = match self.selected_card_ids.last() {
            Some(&id) => id,
            None => return false,
        };
        let reference_blade = self
            .game_state
            .card_database
            .get_card(reference_id)
            .map(|c| c.blade)
            .unwrap_or(0);

        // The reference member itself is always excluded from the comparison.
        let other_ids = self.collect_other_stage_ids(
            Some(reference_id),
            condition.get_card_type().map(|ct| ct.as_str()),
            is_both,
        );

        // No other members → the predicate is vacuously true.
        if other_ids.is_empty() {
            return true;
        }

        other_ids.iter().all(|&other_id| {
            self.game_state
                .card_database
                .get_card(other_id)
                .map(|c| c.blade)
                .unwrap_or(0)
                < reference_blade
        })
    }

    /// "元々持つブレード" — checks base/printed blade (card.blade from DB).
    /// Per Q195 (qa_data.json:1071-1074): "元々持つブレードの数を変更した後、
    /// ブレードを得る効果が適用される" — setting the original blade changes
    /// the base (Rules 9.9.1.4), then +blade effects stack on top (9.9.1.5).
    /// For current blade totals (e.g. "ブレードの合計"), Q116 (lines 2487-2488)
    /// confirms modified/current values are used instead.
    pub(crate) fn check_original_blade_filter(&self, condition: &Condition, card_id: i16) -> bool {
        if !condition.get_original_value().unwrap_or(false) {
            return true;
        }
        // Explicit per-card blade limit (「元々のブレードの数がNつ」): compare
        // the card's printed blade against blade_limit. This must run for
        // members too — the early return below only covers conditions whose
        // threshold rides on count/operator (heart-style comparisons).
        if let Some(limit) = condition.get_blade_limit() {
            let op = condition.get_blade_limit_operator().map(|o| o.as_str());
            let card_blade = self
                .game_state
                .card_database
                .get_card(card_id)
                .map(|c| c.blade)
                .unwrap_or(0);
            return compare_counts(op, card_blade, limit);
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
        for color in ORIGINAL_HEART_COLORS {
            let modifier = self.game_state.mods.get_heart_modifier(card_id, color);
            if modifier > 0 {
                current_hearts += modifier.u8_count();
            }
        }
        let verdict = compare_counts(Some(op), current_hearts, base_hearts);
        log::debug!(
            "[ORIG_HEART] card={} base={} current={} op={:?} -> {}",
            card_id,
            base_hearts,
            current_hearts,
            op,
            verdict
        );
        verdict
    }
}
