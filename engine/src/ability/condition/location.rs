use super::ConditionContext;
use super::predicates;
use crate::ability::enums::Zone;
use crate::ability::util::{self, compare_counts};
use crate::card::{AbilityFilter, BladeColor, CardProperty, ComparisonTarget, Condition};
use crate::core::constants::CountCast;
use crate::HashSet;
#[cfg(feature = "no_std")]
use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
    vec::Vec,
};

impl<'a> ConditionContext<'a> {
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
            Some(CardProperty::HasDrawIcon) => "has_draw_icon",
            // `_ => return true` is a trap for a new variant. A property that reaches
            // here unmapped is treated as "no restriction", so the clause fires on
            // every card in the zone. That is exactly what happened to
            // 「…{{icon_draw}}を持つカードがある場合」
            // (PL!-pb2-001-PP): the variant existed, the parser emitted it, and this
            // arm swallowed it.
            _ => return true,
        };
        let card_db = &self.game_state.card_database;
        // Every zone has to contribute its cards, not just the two that were
        // special-cased. The old fallback was `vec![]`, and the empty case below
        // returns TRUE, so a `card_property` on a condition naming any other zone
        // — 「自分の成功ライブカード置き場に{{icon_score}}を持つ『μ's』のカードが
        // ある場合」 (PL!-pb2-005) — was silently never checked, and the ability
        // fired off cards that do not carry the icon.
        let check_cards: Vec<i16> = match Zone::from_str(location) {
            Some(Zone::RevealedCards) => self.revealed_cards_scoped(condition),
            Some(Zone::Stage) => player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .copied()
                .collect(),
            _ => crate::ability::util::zone_cards(player, location)
                .iter()
                .copied()
                .filter(|&id| id != -1)
                .collect(),
        };
        let neg = condition.get_negation().unwrap_or(false);
        if check_cards.is_empty() {
            return true;
        }
        let blade_type_override = if location == "revealed_cards" {
            crate::turn::live::blade_type_override_for_stage(
                &player.stage.stage,
                &self.game_state.mods.blade_type_modifiers,
            )
        } else {
            None
        };
        let has_prop = |id: i16| -> bool {
            let c = card_db.get_card(id);
            match prop {
                // `has_blade_heart` counts ALLブレード / スコア / ドロー as ブレードハート, on
                // every path including a yell-triggered `revealed_cards` check.
                //
                // Ruling: cards/qa_data.json Q112 (2025.05.30), asked about exactly this
                // printed shape —
                //   『…エールにより公開された自分のカードの中にブレードハートを持つカードが
                //     ないとき、…』について。ALLブレード、スコア、ドローはブレードハートに
                //     含まれますか？
                //   「はい、含まれます。」
                //
                // This used to prefer `has_blade_heart_strict()` (blade_heart only) on the
                // yell-revealed path, which excludes the スコア/ドロー icons Q112 says to
                // include.
                //
                // I tried removing that special case. All 22 tests in
                // tests/test_modules/jidou/yell/no_blade_heart_reveal_gain/ pass IDENTICALLY
                // with and without it — including the three Q112 cards that do set
                // `yell_trigger` (PL!SP-bp2-015-N / -020-N / -021-N, confirmed the only three
                // parses carrying has_blade_heart + revealed_cards + yell_trigger together).
                // So this site is NOT what evaluates them: the loose, ruled predicate is
                // reached by others (condition/state.rs, condition/compound.rs,
                // util::check_card_property), and the edit was unobservable.
                //
                // Therefore it is left in place and NOT claimed as a fix. It is either dead
                // for these cards or serves a path with no coverage. Resolving that needs a
                // test that exercises THIS site, or the `has_blade_heart` sites consolidated
                // the way F1 consolidated `exclude_self`. See docs/EDGE_CASE_FINDINGS.md.
                "has_blade_heart" => {
                    if location == "revealed_cards" && condition.get_yell_trigger() == Some(true) {
                        c.is_some_and(|c| c.has_blade_heart_strict())
                    } else {
                        c.is_some_and(|c| c.has_blade_heart())
                    }
                }
                "has_score_icon" => c.is_some_and(|c| c.has_score_icon()),
                "has_draw_icon" => c.is_some_and(|c| c.has_draw_icon()),
                "has_all_blade" => {
                    let effective_override = self
                        .game_state
                        .mods
                        .blade_type_modifiers
                        .get(&id)
                        .copied()
                        .or(blade_type_override);
                    c.is_some_and(|c| c.has_all_blade())
                        && !matches!(
                            effective_override,
                            Some(
                                BladeColor::Peach
                                    | BladeColor::Red
                                    | BladeColor::Yellow
                                    | BladeColor::Green
                                    | BladeColor::Blue
                                    | BladeColor::Purple
                            )
                        )
                }
                _ => false,
            }
        };
        if neg {
            let property_cards: Vec<i16> = check_cards
                .iter()
                .copied()
                .filter(|&id| has_prop(id))
                .collect();
            log::debug!(
                "[CARD_PROPERTY_FILTER] zone={} property={} negated={} override={:?} candidates={:?} matches={:?}",
                location,
                prop,
                neg,
                blade_type_override,
                check_cards,
                property_cards
            );
            return property_cards.is_empty();
        }
        // `filter_subset()` is loop-invariant here: it reads only `condition`
        // (21-arm `common()` match + 9 field reads + a 40-field `CardFilter`
        // construction), so building it per candidate card meant rebuilding the
        // same filter up to `check_cards.len()` times. Only `heart_colors` was
        // actually varying, and it is now set once alongside.
        let mut filter = condition.filter_subset();
        filter.heart_colors = condition.get_heart_colors().unwrap_or(&[]);
        let mut matching = Vec::new();
        for &id in &check_cards {
            let other_filters_match = filter.matches(card_db, id, true)
                && self.check_original_blade_filter(condition, id)
                && self.check_original_heart_filter(condition, id)
                && self.check_heart_type_all_per_card(condition, card_db, id);
            if has_prop(id) && other_filters_match {
                matching.push(id);
            }
        }
        log::debug!(
            "[CARD_PROPERTY_FILTER] zone={} property={} negated={} override={:?} candidates={:?} matches={:?}",
            location,
            prop,
            neg,
            blade_type_override,
            check_cards,
            matching
        );
        !matching.is_empty()
    }

    pub(crate) fn check_baton_touch(&self, condition: &Condition) -> bool {
        if !condition.get_baton_touch_trigger().unwrap_or(false) {
            return true;
        }
        // Resolve the player to check baton touch count for
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let player_id = player.id.as_str();
        let count = self.game_state.get_baton_touch_count(player_id);
        if count == 0 {
            return false;
        }
        if condition
            .get_min_baton_touch_count()
            .is_some_and(|min| count < min)
        {
            return false;
        }
        let card_db = &self.game_state.card_database;

        // A group qualifier describes the member that was replaced, so a
        // condition with no observed replacement cannot satisfy it.
        if let Some(groups) = condition.get_group_names().filter(|g| !g.is_empty()) {
            let Some(replaced_id) = self.game_state.baton_touch_replaced_member_id else {
                return false;
            };
            if !groups.iter().any(|g| {
                crate::ability::util::card_matches_group_str(card_db, replaced_id, Some(g))
            }) {
                return false;
            }
        }

        // The source qualifier names a member that must be sitting in some
        // waitroom — a baton touch always leaves the replaced card there.
        if let Some(src) = condition.get_baton_touch_source() {
            // Both sides are canonical already: `src` was squashed by the parser
            // (`parser_utils.squash_name_needles`) and `name_key` at card load.
            let in_any_waitroom = self
                .game_state
                .player1
                .waitroom
                .cards
                .iter()
                .chain(self.game_state.player2.waitroom.cards.iter())
                .any(|&id| {
                    card_db.get_card(id).is_some_and(|c| c.name_key.contains(src))
                });
            if !in_any_waitroom {
                return false;
            }
        }

        // Cost comparison is between the replaced member's cost and the
        // activating card's. Absent either value there is nothing to compare.
        if condition.get_comparison_type() == Some("cost") {
            let replaced_cost = self.game_state.baton_touch_replaced_member_cost;
            let activating_cost = self
                .game_state
                .activating_card
                .and_then(|act| card_db.get_card(act))
                .and_then(|c| c.cost);
            if let (Some(replaced), Some(cost)) = (replaced_cost, activating_cost) {
                if !compare_counts(condition.get_operator(), replaced, cost) {
                    return false;
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
            let card_type = condition
                .get_card_type()
                .map(|ct| ct.as_str())
                .unwrap_or("");
            let group_name: Option<&str> = condition
                .get_group_names()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            match Zone::from_str(location) {
                Some(Zone::Stage) => {
                    let occupied: Vec<i16> = player
                        .stage
                        .stage
                        .iter()
                        .copied()
                        .filter(|&cid| cid != -1)
                        .collect();
                    let total_heart: u8 = self
                        .eligible_cards(&occupied, card_type, group_name)
                        .into_iter()
                        .map(|(cid, card)| {
                            let base: u8 = card
                                .base_heart
                                .as_ref()
                                .map(|bh| {
                                    hc.iter()
                                        .map(|color_str| {
                                            let color = crate::card::parse_heart_color(color_str);
                                            bh.hearts.get(&color).copied().unwrap_or(0)
                                        })
                                        .sum::<u8>()
                                })
                                .unwrap_or(0);
                            let modifier: i16 = hc
                                .iter()
                                .map(|color_str| {
                                    let color = crate::card::parse_heart_color(color_str);
                                    self.game_state
                                        .mods
                                        .heart_modifiers
                                        .get(&cid)
                                        .and_then(|hm| hm.get(&color))
                                        .map(|e| e.total())
                                        .unwrap_or(0)
                                })
                                .sum();
                            crate::constants::effective_stat(base, modifier)
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
                        let total_need: u8 = self
                            .eligible_cards(&cards, card_type, group_name)
                            .into_iter()
                            .map(|(cid, card)| {
                                card.need_heart
                                    .as_ref()
                                    .map(|nh| {
                                        hc.iter()
                                            .map(|color_str| {
                                                let color =
                                                    crate::card::parse_heart_color(color_str);
                                                // Base need_heart + live modifier (e.g.
                                                // ディストーション reduces heart00 / adds
                                                // heart02 per CatChu member before this gate)
                                                let base =
                                                    nh.hearts.get(&color).copied().unwrap_or(0);
                                                let modifier = self
                                                    .game_state
                                                    .mods
                                                    .get_need_heart_modifier(cid, color);
                                                crate::constants::effective_stat(base, modifier)
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
                        let threshold = condition.get_count().unwrap_or(1);
                        let all_ok = hc.iter().all(|color_str| {
                            let color = crate::card::parse_heart_color(color_str);
                            let total: u8 = self
                                .eligible_cards(&cards, card_type, group_name)
                                .into_iter()
                                .map(|(cid, card)| {
                                    let base = card
                                        .need_heart
                                        .as_ref()
                                        .and_then(|nh| nh.hearts.get(&color))
                                        .copied()
                                        .unwrap_or(0);
                                    let modifier =
                                        self.game_state.mods.get_need_heart_modifier(cid, color);
                                    crate::constants::effective_stat(base, modifier)
                                })
                                .sum();
                            total >= threshold
                        });
                        Some(all_ok)
                    }
                }
                _ => None,
            }
        } else if Zone::from_str(location) == Some(Zone::Stage)
            && condition.get_group_names().is_some()
            && condition.get_card_type() == Some(crate::card::ConditionCardType::MemberCard)
        {
            // 「メンバーが持つハートの総数がN以上」 — aggregate=total with
            // group_names but NO specific heart_colors on Stage: sum ALL
            // base_heart values of matching members and compare.
            let card_db = &self.game_state.card_database;
            let group = condition
                .get_group_names()
                .and_then(|gn| gn.first().map(|s| s.as_str()));
            let total: u8 = player
                .stage
                .stage
                .iter()
                .filter(|&&id| id != -1)
                .filter(|&&id| {
                    group.is_none_or(|g| util::card_matches_group_str(card_db, id, Some(g)))
                })
                .filter_map(|&id| card_db.get_card(id))
                .map(|card| {
                    card.base_heart
                        .as_ref()
                        .map(|bh| bh.hearts.values_sum())
                        .unwrap_or(0)
                })
                .sum();
            log::debug!(
                "[HEART_TOTAL_STAGE] group={:?} total={} threshold={:?} op={:?}",
                group,
                total,
                condition.get_count(),
                condition.get_operator()
            );
            Some(compare_counts(
                condition.get_operator(),
                total,
                condition.get_count().unwrap_or(0),
            ))
        } else if Zone::from_str(location) == Some(Zone::Stage)
            // The Stage branch sums BLADE totals for 「自分のステージ…ブレードの
            // 合計」 conditions (target=self). Cross-player member counts
            // (「自分と相手のステージにメンバーが合計6人」, target=both) must
            // fall through to the normal combined counting path instead.
            && condition.get_target().map(|t| t != "both").unwrap_or(true)
        {
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
            Some(Zone::RevealedCards) => self.revealed_cards_scoped(condition),
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
                    if let Some(modified_cost) = self.modified_cost(cid) {
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
        if let Some(v) = self.evaluate_check_self_condition(condition) {
            return v;
        }
        // 「〜がすべて『X』」 is emitted on whichever variant matches where the text
        // looks, so this variant has to honour it as well as `Group` does — the
        // marker used to be dropped here at decode, which turned 「すべて」 into
        // "at least one" on exactly the clauses that name a zone rather than a
        // group. A zone this helper does not enumerate falls through untouched.
        if condition.get_all_members().unwrap_or(false) {
            if let Some(verdict) = self.evaluate_all_members_condition(condition) {
                return verdict;
            }
        }
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
            log::debug!("[LOC_COND] failed at check_heart_type_all");
            return false;
        }
        if !self.check_heart_colors(condition, player, location) {
            log::debug!("[LOC_COND] failed at check_heart_colors");
            return false;
        }
        if !self.check_card_property(condition, player, location) {
            log::debug!("[LOC_COND] failed at check_card_property");
            return false;
        }
        if !self.check_baton_touch(condition) {
            log::debug!("[LOC_COND] failed at check_baton_touch");
            return false;
        }
        if let Some(res) = self.check_aggregate_total(condition, player, location) {
            log::debug!("[LOC_COND] aggregate_total decided: {}", res);
            return res;
        }
        if !self.check_ability_filter(condition, player, location) {
            log::debug!("[LOC_COND] failed at check_ability_filter");
            return false;
        }
        if !self.check_distinct_names(condition, player, location) {
            log::debug!("[LOC_COND] failed at check_distinct_names");
            return false;
        }
        if !self.check_no_excess_heart(condition, player, target) {
            log::debug!("[LOC_COND] failed at check_no_excess_heart");
            return false;
        }

        // Two-sided count differential: 「エールにより公開された自分のカードの枚数が、
        // 相手がエールによって公開したカードの枚数より少ない場合」
        // (渡辺曜 PL!S-bp3-005-R, ruling Q153).
        //
        // The LEFT operand is this player's count in `location`; `comparison_target`
        // supplies the RIGHT, which is another count rather than a literal — a shape
        // the one-sided `cost_limit`/`operator` pair cannot express, and the reason
        // this card used to arrive as a bare zone count that no configuration could
        // satisfy (its ライブ成功時 never drew). Q153: an opponent who never live'd
        // has revealed 0, so own < 0 is never true and the ability does not draw.
        //
        // Guarded on `count.is_none()` so a condition that already carries its own
        // threshold keeps its existing path; no Location condition in the corpus sets
        // `comparison_target` today, so this cannot reinterpret one.
        if condition.get_comparison_target() == Some(ComparisonTarget::Opponent)
            && condition.get_count().is_none()
        {
            let self_count = self.game_state.cheer_revealed_count_for_player(
                self.resolve_condition_player("self"),
            );
            let opp_count = self.game_state.cheer_revealed_count_for_player(
                self.resolve_condition_player("opponent"),
            );
            log::debug!(
                "[LOC_COND] two-sided differential self={} opponent={} op={:?} loc={}",
                self_count,
                opp_count,
                condition.get_operator(),
                location
            );
            return compare_counts(condition.get_operator(), self_count, opp_count);
        }

        // G11: "ほかのすべてのメンバーより多くのブレードを持つ" — the selected
        // member has strictly MORE blade than every other stage member (both sides).
        if condition.get_blade_greater_than_all().unwrap_or(false) {
            return self.evaluate_blade_greater_than_all(condition, is_both);
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
            if let (Some(pos_a), Some(pos_b)) =
                (&condition.get_position(), &condition.get_position_compare())
            {
                let pos_a_str = pos_a.get_position().unwrap_or("");
                let card_a = util::card_at_position(player, pos_a_str);
                let card_b = util::card_at_position(player, pos_b);
                // When require_position_cards is set, both positions must have
                // cards for the comparison to be meaningful (e.g. "右サイドエリアと
                // 左サイドエリアにいるメンバーのコストが同じ場合").
                if condition.get_require_position_cards().unwrap_or(false)
                    && (card_a.is_none() || card_b.is_none()) {
                        return false;
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
                        let selected_name = selected_card.name_key.as_ref();
                        let target_cards: Vec<i16> = util::zone_cards(player, location).to_vec();
                        let matching = target_cards
                            .iter()
                            .filter(|&&cid| {
                                card_db
                                    .get_card(cid)
                                    .is_some_and(|c| c.name_key.as_ref() == selected_name)
                            })
                            .count()
                            .u8_count();
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

        let cards: Vec<i16> = if Zone::from_str(location) == Some(Zone::RevealedCards) {
            if condition.get_yell_trigger() == Some(true) && !self.game_state.yell_occurred {
                log::debug!("[LOC_COND] yell_trigger true but yell_occurred false → fail");
                return false;
            }
            self.revealed_cards_scoped(condition)
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

        // Hoisted above the scan: the operator and threshold decide whether the
        // scan can stop at the first match (see `monotone_exists` below). Both
        // are pure functions of `condition`.
        //
        // Operator ownership: when original_value=true WITHOUT a blade_limit,
        // the condition operator belongs to the PER-CARD heart comparison
        // consumed by check_original_heart_filter below
        // (「元々持つハートの数より多い数のハートを持つメンバー」). Reusing it
        // for the outer member-count would demand N+1 boosted members instead
        // of N. The outer clause (「メンバーがいる/N人以上」) is a plain
        // existence/count threshold -> >=.
        let thresh = condition.get_count().unwrap_or(1);
        let effective_op = predicates::member_count_operator(condition, thresh);
        // `compare_counts(Some(">="), n, 1)` is `n >= 1`, i.e. `n > 0`, i.e.
        // `any`. For that case a full `.count()` traversal is wasted work: the
        // corpus has 69 such `location_condition` clauses out of 100, and the
        // zones involved run 30-64 cards (deck 64, waitroom 30). Gated on
        // `!debug_enabled()` so the verdict's `actual` field still reports the
        // true count in debug sessions.
        let monotone_exists =
            effective_op == Some(">=") && thresh <= 1 && !crate::ability::log::debug_enabled();

        let count = {
            let mut filter = condition.filter_subset();
            if let Some(grp) = group_override {
                filter.group = Some(grp);
            }
            if condition.get_exclude_self().unwrap_or(false) {
                filter.exclude_self = self.activating_card_id;
            }
            // 「自分か相手の…にいる場合」 (target=either): the clause passes if
            // EITHER side's zone alone satisfies the filters. Take the max of
            // the two per-side match counts so a threshold >1 can never be met
            // by combining partial matches across both players' zones.
            let count_side = |zone_cards: &[i16]| -> u8 {
                let matches_one = |&id: &i16| {
                    id != -1
                        && filter.matches(card_db, id, true)
                        && self.check_original_blade_filter(condition, id)
                        && self.check_original_heart_filter(condition, id)
                        && self.check_heart_type_all_per_card(condition, card_db, id)
                };
                if monotone_exists {
                    if zone_cards.iter().any(matches_one) { 1u8 } else { 0u8 }
                } else {
                    zone_cards.iter().filter(|id| matches_one(id)).count().u8_count()
                }
            };
            let is_either = condition.get_target() == Some("either")
                && Zone::from_str(location) != Some(Zone::RevealedCards);
            let matched = if is_either {
                let n_self = count_side(util::zone_cards(
                    self.resolve_condition_player("self"),
                    location,
                ));
                let n_opp = count_side(util::zone_cards(
                    self.resolve_condition_player("opponent"),
                    location,
                ));
                log::debug!(
                    "[LOC_COND_EITHER] loc={} n_self={} n_opp={}",
                    location,
                    n_self,
                    n_opp
                );
                n_self.max(n_opp)
            } else {
                count_side(&cards)
            };
            matched
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
        compare_counts(effective_op, count, thresh)
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
        let Some(locs) = condition.get_locations() else {
            log::error!("evaluate_multi_location_condition: missing locations field; treating as unsatisfied");
            return false;
        };

        if condition.get_self_target().unwrap_or(false) && locs.len() == 2 {
            // A 「AかB」 disjunction (PL!-pb2-038-L: 「自分のライブカード置き場**か**
            // 成功ライブカード置き場にあり」) is satisfied by EITHER zone. Without the
            // marker this branch assumes the 「AをBに」 move shape and checks only
            // locs[1], so the card qualified from exactly one of the two zones named.
            if condition.get_any_location().unwrap_or(false) {
                return locs.iter().any(|loc| {
                    let cards = util::zone_cards(player, loc.as_str());
                    self.activating_card_id
                        .is_some_and(|cid| cards.contains(&cid))
                });
            }
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
        log::debug!(
            "[MULTI] condition={:?} combined={:?} card_type={:?} groups={:?}",
            condition,
            combined,
            card_type_filter,
            group_names
        );

        let is_distinct = condition.get_distinct().is_some_and(|d| d.is_distinct());
        if is_distinct {
            let distinct_type = match condition.get_distinct() {
                Some(crate::core::card::DistinctInfo::String(s)) => s.as_str(),
                _ if condition.get_group_reference() == Some("different_group_names") => {
                    "group_name"
                }
                _ => "card_name",
            };
            // Shared eligibility gate — identical across all three distinct-key
            // modes; only the key extraction/aggregation below differs.
            let eligible = |cid: i16| -> bool {
                cid != -1
                    && card_type_filter
                        .is_none_or(|f| util::card_matches_type(card_db, cid, Some(f)))
                    && group_names.is_none_or(|gn| {
                        gn.iter()
                            .any(|g| util::card_matches_group_str(card_db, cid, Some(g.as_str())))
                    })
            };
            match distinct_type {
                "cost" => {
                    let mut distinct_costs: HashSet<u8> = HashSet::default();
                    for &cid in &combined {
                        if !eligible(cid) {
                            continue;
                        }
                        if let Some(modified_cost) = self.modified_cost(cid) {
                            distinct_costs.insert(modified_cost);
                        }
                    }
                    let count = distinct_costs.len().u8_count();
                    compare_counts(operator, count, count_threshold)
                }
                "group_name" => {
                    let eligible_cards: Vec<i16> = combined
                        .iter()
                        .copied()
                        .filter(|&cid| eligible(cid))
                        .collect();
                    let count = util::max_distinct_group_names(
                        card_db,
                        &eligible_cards,
                        group_names,
                    )
                    .u8_count();
                    compare_counts(operator, count, count_threshold)
                }
                _ => {
                    let mut name_sets: Vec<Vec<String>> = Vec::new();
                    for &cid in &combined {
                        if !eligible(cid) {
                            continue;
                        }
                        name_sets.push(card_db.get_card_names(cid));
                    }
                    let best = util::max_distinct_names(&name_sets);
                    compare_counts(operator, best.distinct.u8_count(), count_threshold)
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

    /// 「〜がすべて『X』」 — EVERY card the clause covers is in one of the listed
    /// groups. Returns `None` when the condition does not carry `all_members`, so
    /// the caller can fall through to its ordinary path.
    ///
    /// Shared by `evaluate_group_condition` and `evaluate_location_condition`,
    /// because the marker is not a property of either variant: the parser emits it
    /// on whichever variant matches where the text looks, and `PL!-pb2-037-L`'s
    /// three-way 「すべて『Printemps』か…」 lands two branches on `Group` and one on
    /// `Location`. Handling it in only one of them made the same 「すべて」 mean two
    /// different things inside a single `or`.
    pub(crate) fn evaluate_all_members_condition(&self, condition: &Condition) -> Option<bool> {
        if !condition.get_all_members().unwrap_or(false) {
            return None;
        }
        let location = condition.get_location().unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        // "…のみがいる" — every card in the zone must belong to ONE of the listed
        // groups (e.g. 『Aqours』か『SaintSnow』). The group list is an OR: a card
        // matches if it is in ANY listed group, not just the first.
        let groups: Vec<String> = condition
            .get_group_names()
            .map(|g| g.to_vec())
            .unwrap_or_default();
        let card_db = &self.game_state.card_database;
        let matches = |id: i16| {
            groups.is_empty()
                || groups
                    .iter()
                    .any(|g| crate::ability::util::card_matches_group_str(card_db, id, Some(g)))
        };
        // 「2人の『μ's』のメンバーのみがいる」 is a COUNT as well as a group test:
        // exactly two, all of them μ's. The all_members branch below only ever
        // asked "are they all in the group", so one member and three members both
        // satisfied it. That went unnoticed because PL!-pb2-038-L — the card that
        // needs it — had no working code path at all, so its negatives passed
        // vacuously. With an explicit `count` and no operator the reading is
        // equality; an operator, if present, still governs.
        let exact_count = match (condition.get_count(), condition.get_operator()) {
            (Some(n), None) => Some(u8::try_from(n).unwrap_or(0)),
            _ => None,
        };
        let count_ok = |cards: &[i16]| match exact_count {
            Some(want) => cards.len() == usize::from(want),
            None => true,
        };
        let zone = Zone::from_str(location);
        let cards: Vec<i16> = match zone {
            Some(Zone::Stage) => player
                .stage
                .stage
                .iter()
                .copied()
                .filter(|&id| id != -1)
                .collect(),
            Some(Zone::LiveCardZone) => player.live_card_zone.cards.to_vec(),
            Some(Zone::SuccessLiveZone) => player.success_live_card_zone.cards.to_vec(),
            // The yell reveal is GAME-level shared state, not a player zone, so it
            // cannot be reached through `player`. `PL!-pb2-037-L` gates on
            // 「エールにより公開された自分のメンバーカードがすべて『Printemps』」 and
            // its lily white / BiBi branches are `group_condition`s over
            // `revealed_cards`; without this arm they fell through to `_ => false`
            // and could never match at any reveal size.
            //
            // `card_type` selects which revealed cards the 「すべて」 is ABOUT, not a
            // second filter stacked on the group test: the clause counts the revealed
            // MEMBERS, so a revealed live card must not count against a group that
            // every revealed member happens to satisfy.
            Some(Zone::RevealedCards) => {
                let card_type = condition.get_card_type().map(|ct| ct.as_str());
                self.revealed_cards_scoped(condition)
                    .into_iter()
                    .filter(|&id| {
                        card_type
                            .is_none_or(|ct| util::card_matches_type(card_db, id, Some(ct)))
                    })
                    .collect()
            }
            // An unhandled zone is not a pass and not a fail here: `None` lets the
            // caller take its own path rather than silently answering for it.
            _ => return None,
        };
        let verdict = count_ok(&cards) && cards.iter().all(|&id| matches(id));
        log::debug!(
            "[GROUP_ALL_MEMBERS] zone={:?} card_type={:?} n={} groups={:?} verdict={}",
            location,
            condition.get_card_type().map(|ct| ct.as_str()),
            cards.len(),
            groups,
            verdict
        );
        Some(verdict)
    }

}
