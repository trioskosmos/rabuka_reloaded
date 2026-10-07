use super::ConditionContext;
use super::predicates;
use crate::ability::debug::AbDebug;
use crate::ability::enums::Zone;
use crate::ability::util::{self, compare_counts};
use crate::card::{BladeColor, CardProperty, ComparisonTarget, Condition};
use crate::core::constants::CountCast;
use crate::HashMap;
use smallvec::SmallVec;
#[cfg(feature = "no_std")]
use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
    vec::Vec,
};

impl<'a> ConditionContext<'a> {
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
                let base = self.count_cards_with_filters(
                    zone_source,
                    Some(ct),
                    group_names,
                    hc,
                    condition.get_cost_limit(),
                    condition.get_cost_limit_operator().map(|o| o.as_str()),
                    None,
                    // true: honor original_value so per-card current-vs-
                    // original heart/blade comparisons gate each counted
                    // card (「元々持つハートの数より多い…メンバー」).
                    true,
                    condition,
                ) as u32;
                // A card can be worth more than one when something counts it:
                // 「このカードは2枚として扱う」 (PL!-pb2-041-L). Membership is decided
                // by the SAME filter above, applied one card at a time, so a weighted
                // card can only ever be a card this count would already have included
                // — the weight changes the total, never which cards qualify.
                //
                // The weight is a property of the card, not of a zone, so it applies
                // on every path through this closure rather than only the success live
                // zone. That is safe because the rule's own condition scopes it: a
                // card outside the success live zone has no active weight.
                let extra: u32 = zone_source
                    .iter()
                    .filter(|&&cid| {
                        cid != -1
                            && self.count_cards_with_filters(
                                &[cid],
                                Some(ct),
                                group_names,
                                hc,
                                condition.get_cost_limit(),
                                condition.get_cost_limit_operator().map(|o| o.as_str()),
                                None,
                                true,
                                condition,
                            ) == 1
                    })
                    .map(|&cid| {
                        self.game_state
                            .count_as_multiple_weight(cid, activating_id)
                            .saturating_sub(1)
                    })
                    .sum();
                (base + extra) as usize
            }
        };

        let actual = match Zone::from_str(location) {
            Some(Zone::RevealedCards) => {
                // Same 「自分の」 scope as evaluate_location_condition: the pool
                // holds both performers' yell reveals, tagged by seat.
                let scoped = self.revealed_cards_scoped(condition);
                if condition.get_unit() == Some("types") && !hc.is_empty() {
                    let required_colors: Vec<crate::card::HeartColor> = hc
                        .iter()
                        .map(|s| crate::card::parse_heart_color(s))
                        .collect();
                    let is_blade = condition.get_heart_source() == Some("blade");
                    let actual = self.count_distinct_heart_types(
                        &scoped,
                        &required_colors,
                        is_blade,
                        None,
                        None,
                    );
                    log::debug!(
                        "[REVEALED_TYPES] heart_source={:?} required={:?} count={}",
                        condition.get_heart_source(),
                        required_colors,
                        actual,
                    );
                    actual
                } else {
                    count_filtered(&scoped, card_type)
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
                            util::orientation_matches_state(
                                self.game_state.mods.get_orientation_modifier(cid),
                                state.as_str(),
                            )
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
                // Exclude the activating card itself (このメンバー以外).
                // Without this, a lone Wien counts herself and
                // "このメンバー以外のメンバーが1人以上いる" passes with 0 others.
                let stage_cards: SmallVec<[i16; 6]> = if exclude_self {
                    stage_cards
                        .into_iter()
                        .filter(|&cid| Some(cid) != activating_id)
                        .collect()
                } else {
                    stage_cards
                };
                let is_distinct_cost = condition.get_distinct().is_some_and(
                    |d| matches!(d, crate::core::card::DistinctInfo::String(s) if s == "cost"),
                );
                if is_distinct_cost {
                    self.count_distinct_cost(
                        &stage_cards,
                        if card_type.is_empty() {
                            None
                        } else {
                            Some(card_type)
                        },
                    )
                } else if condition.get_unit() == Some("types") {
                    let required_colors: Vec<crate::card::HeartColor> = hc
                        .iter()
                        .map(|s| crate::card::parse_heart_color(s))
                        .collect();
                    let is_blade = condition.get_heart_source() == Some("blade");
                    self.count_distinct_heart_types(
                        &stage_cards,
                        &required_colors,
                        is_blade,
                        None,
                        None,
                    )
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
                } else if Zone::from_str(location) == Some(Zone::SuccessLiveZone) {
                    // Counting weights are applied inside `count_filtered` above, so
                    // this is the plain path. Kept explicit so the success live zone
                    // is visibly covered: it is the zone 「このカードは2枚として扱う」
                    // applies in.
                    count_filtered(util::zone_cards(player, location), card_type)
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
                // State-based: count all cards in the zone. Movement-scoped
                // counts (手札から…置かれた) use source=preceding_moved and
                // never reach this arm (resolved earlier via
                // resolve_moved_cards_source).
                count_filtered(&player.waitroom.cards, card_type)
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
                                    let state_ok = util::orientation_matches_state(
                                        self.game_state.mods.get_orientation_modifier(id),
                                        state.as_str(),
                                    );
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
        }
        .u8_count();
        actual
    }

    pub(crate) fn evaluate_card_count_condition(&self, condition: &Condition) -> bool {
        if let Some(v) = self.evaluate_check_self_condition(condition) {
            return v;
        }
        let card_type = condition
            .get_card_type()
            .map(|ct| ct.as_str())
            .unwrap_or("");
        let target = self.resolve_target_for_scope(condition);
        let is_both = target == "both";
        let count = condition.get_count().unwrap_or(1);
        let player = self.resolve_condition_player(target);
        let card_db = &self.game_state.card_database;

        // Per-turn activation history: count the members switched OUT of wait this
        // turn, not the members currently in wait.
        // PL!-pb2-040-L. Without this the clause degenerated to "some member is in
        // wait", which is a different set and made the reduction unconditional.
        if condition.get_activated_this_turn() == Some(true) {
            let group_names = condition.get_group_names();
            let n = self
                .game_state
                .cards_activated_this_turn
                .iter()
                .copied()
                .filter(|cid| {
                    card_db.get_card(*cid).is_some_and(|card| {
                        if card_type == "member_card" && !card.is_member() {
                            return false;
                        }
                        group_names.is_none_or(|gs| {
                            gs.is_empty()
                                || gs.iter().any(|g| {
                                    crate::ability::util::card_matches_group_str(
                                        card_db,
                                        *cid,
                                        Some(g),
                                    )
                                })
                        })
                    })
                })
                .count();
            return n >= count as usize;
        }

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

        // Cross-position pair requirement (e.g. bp6-009-R: 「右サイドエリアと
        // 左サイドエリアに、元々のブレードの数が2つのメンバーがいるかぎり」):
        // when position AND position_compare are both set, EACH named area
        // must hold a member matching the per-card filters (blade limit etc.).
        if let (Some(pos), Some(pos_cmp)) = (
            condition.get_position().and_then(|p| p.get_position()),
            condition.get_position_compare(),
        ) {
            return [pos, pos_cmp].iter().all(|slot| {
                util::card_at_position(player, slot)
                    .map(|cid| {
                        card_db.get_card(cid).is_some_and(|c| {
                            c.is_member()
                                && condition
                                    .get_blade_limit()
                                    .map(|lim| c.blade == lim)
                                    .unwrap_or(true)
                        })
                    })
                    .unwrap_or(false)
            });
        }

        // Early-out for aggregate total (sum heart colors, not count cards)
        let stage_types = condition.get_unit() == Some("types")
            && Zone::from_str(location) == Some(Zone::Stage);
        log::debug!(
            "[AGGREGATE_DISPATCH] location={} unit={:?} stage_types={} aggregate={:?}",
            location,
            condition.get_unit(),
            stage_types,
            condition.get_aggregate()
        );
        if !stage_types {
            if let Some(res) = self.check_aggregate_total(condition, player, location) {
                return res;
            }
        }

        let is_old_movement = condition
            .get_source()
            .is_some_and(super::is_moved_source);
        let is_new_movement = !condition
            .get_source()
            .is_some_and(super::is_moved_source)
            && condition.get_destination().is_some();

        if is_old_movement || is_new_movement {
            return self.resolve_moved_cards_source(
                condition,
                is_new_movement,
                card_type,
                hc,
                count,
            );
        }

        // 「自分のステージにエネルギーカードが下にあるメンバーがいる場合」
// (PL!N-bp5-013-N 上原歩夢) — the counted set is MEMBERS, and each one is kept only
// if the named card sits beneath it. Overriding `actual` here rather than adding a
// dispatch arm means the clause reuses the ordinary operator/threshold comparison
// below, so 「〜がいる」 is `>= 1` and a future 「N人以上」 needs no second path.
//
// This cannot be expressed as a `Location`, and the reason is worth keeping: `under_member`
// resolves to the ACTIVATING card's under-cards (「このメンバーの下」 — 中須かすみ
// `PL!N-pb1-002-P＋`), while this clause is existential over EVERY member on the stage.
let actual = if condition.get_member_has_resource_under() == Some(true) {
            let card_db = &self.game_state.card_database;
            // `resource_type` names the resource; the under-cards are real cards, so map
            // it to the card type they are. Only `energy` reaches here today, and an
            // unknown token falls back to energy_card rather than counting any card.
            let want_card_type = match condition.get_resource_type().unwrap_or("energy") {
                "live_card" => "live_card",
                _ => "energy_card",
            };
            let n = player
                .stage
                .stage
                .iter()
                .enumerate()
                .filter(|&(_, &id)| id != -1)
                .filter(|&(idx, _)| {
                    player.stage.under_cards[idx]
                        .iter()
                        .any(|&cid| util::card_matches_type(card_db, cid, Some(want_card_type)))
                })
                .count();
            let n = u8::try_from(n).unwrap_or(u8::MAX);
            log::debug!(
                "[UNDER_MEMBER_EXISTS] location={:?} want={} members_with_under={} threshold={}",
                location,
                want_card_type,
                n,
                count
            );
            n
        } else {
            self.resolve_zone_card_count(condition, location, hc, is_both, player, card_type, group_names)
        };
        // Comparison target (e.g. "自分のエネルギーが相手より2枚以上多い"):
        // self is `count` more than the opponent → self_count >= opp_count + count.
        // When comparison_target is opponent, the effective comparison is the
        // DIFFERENCE between self and opponent, not a fixed threshold.
        if condition.get_comparison_target() == Some(ComparisonTarget::Opponent) {            let opp_count = self.get_count_for_target(condition, "opponent");
            let threshold = opp_count.saturating_add(count);
            let op = condition
                .get_operator()
                .unwrap_or(if count == 0 { "==" } else { ">=" });
            log::debug!(
                "[CARD_COUNT_OPPONENT] self={} opp={} count={} threshold={} op={}",
                actual,
                opp_count,
                count,
                threshold,
                op
            );
            return compare_counts(Some(op), actual, threshold);
        }
        // Operator ownership: when original_value=true WITHOUT a blade_limit,
        // the condition operator belongs to the PER-CARD heart comparison
        // applied inside resolve_zone_card_count's filter chain
        // (「元々持つハートの数より多い数のハートを持つメンバー」). Reusing it
        // for the outer member-count would demand N+1 boosted members instead
        // of N. The outer clause (「メンバーがいる/N人以上」) is a plain
        // existence/count threshold -> >=.
        let count_op = predicates::member_count_operator(condition, count);
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
                        util::orientation_matches_state(
                            self.game_state.mods.get_orientation_modifier(cid),
                            state.as_str(),
                        )
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
        // One line that answers "why did the gate fire / not fire" without a
        // debugger: the zone count, the printed threshold, the operator, the verdict.
        log::debug!(
            "[CARD_COUNT_COND] location={} card_type={} actual={} op={} threshold={} passed={}",
            location,
            card_type,
            actual,
            count_op.unwrap_or("?"),
            count,
            passed
        );
        #[cfg(not(feature = "no_std"))]
        super::push_cond_verdict(condition, &format!("{}枚", actual), passed, vec![]);
        passed
    }

    pub(crate) fn evaluate_card_blade_condition(&self, condition: &Condition) -> bool {
        let count = condition.get_count().unwrap_or(1);
        let operator = condition.get_operator();
        let source = condition.get_source().unwrap_or("selected_cards");
        let mut cards: &[i16] = match source {
            "selected_cards" => self.selected_card_ids,
            "preceding_moved" => self.moved_cards,
            _ => self.selected_card_ids,
        };
        // 「このメンバーが持つブレードの数がN以上」 is a SELF-referential reading:
        // the subject is the member whose ability is resolving, not a card the
        // player picked. An auto-triggered ability (ライブ開始時, 自動, …) never
        // runs a selection prompt, so `selected_cards` is empty and the
        // condition had no subject at all — it returned false unconditionally,
        // making every 「このメンバーが持つ〜」 gate dead in a real game.
        //
        // The parser routes that wording here with `source: selected_cards`, so
        // the empty selection IS the self-reference. Fall back to the activating
        // card. A genuinely selection-scoped condition ("選んだカードの〜") is
        // unaffected: its selection is non-empty by the time it is evaluated.
        let owned;
        if cards.is_empty() {
            match self.activating_card_id {
                Some(cid) => {
                    owned = [cid];
                    cards = &owned;
                }
                None => return false,
            }
        }
        let card_db = &self.game_state.card_database;
        // A2: unified effective_blade
        let mut total_blades = 0i32;
        for &cid in cards {
            let entry = self
                .game_state
                .mods
                .blade_modifiers
                .get(&cid)
                .copied()
                .unwrap_or_default();
            total_blades +=
                crate::core::stats_pipeline::effective_blade(card_db, cid, entry) as i32;
            log::debug!(
                "[BLADE_PIPELINE] evaluate_card_blade card={} entry={:?} effective={}",
                cid,
                entry,
                crate::core::stats_pipeline::effective_blade(card_db, cid, entry)
            );
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
            if util::compare_counts(operator, crate::constants::saturate_u8(total_blades), count) {
                "PASS"
            } else {
                "FAIL"
            }
        );
        util::compare_counts(operator, crate::constants::saturate_u8(total_blades), count)
    }

    #[allow(clippy::too_many_arguments)]
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
        let filter = crate::ability::util::CardFilter {
            card_type: card_type_filter,
            group: group_names.and_then(|g| g.first().map(|s| s.as_str())),
            heart_colors,
            cost_limit,
            cost_operator: cost_limit_operator,
            ..Default::default()
        };

        let card_db = &self.game_state.card_database;
        if !filter.matches(card_db, card_id, true) {
            return false;
        }
        self.matches_original_value_filters(condition, card_id, respect_original_value)
    }

    #[allow(clippy::too_many_arguments)]
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
        let card_property = condition.get_card_property();
        if card_property != Some(CardProperty::HasAllBlade) {
            if let Some(cp) = &card_property {
                filter.card_property = Some(cp.as_str());
                filter.negation = condition.get_negation().unwrap_or(false);
            }
        }
        let card_db = &self.game_state.card_database;
        let blade_type_override = if condition.get_location() == Some("revealed_cards") {
            let target = condition.get_target().unwrap_or("self");
            let player = self.resolve_condition_player(target);
            crate::turn::live::blade_type_override_for_stage(
                &player.stage.stage,
                &self.game_state.mods.blade_type_modifiers,
            )
        } else {
            None
        };
        let mut count = 0u8;
        for &card_id in cards {
            if !filter.matches(card_db, card_id, true) {
                continue;
            }
            if card_property == Some(CardProperty::HasAllBlade) {
                let effective_override = self
                    .game_state
                    .mods
                    .blade_type_modifiers
                    .get(&card_id)
                    .copied()
                    .or(blade_type_override);
                let has_effective_all = card_db
                    .get_card(card_id)
                    .is_some_and(|card| card.has_all_blade())
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
                    );
                let negated = condition.get_negation().unwrap_or(false);
                let property_name = card_property
                    .map(|property| property.as_str().to_owned())
                    .unwrap_or_else(|| "<none>".into());
                log::debug!(
                    "[CARD_COUNT_PROPERTY] card={} property={} override={:?} effective_all={} negated={}",
                    card_id,
                    property_name,
                    blade_type_override,
                    has_effective_all,
                    negated
                );
                if has_effective_all == negated {
                    continue;
                }
            }
            if !self.matches_original_value_filters(condition, card_id, respect_original_value) {
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
            "cost" => self.count_distinct_cost(&matching, None).u8_count(),
            "group_name" => util::max_distinct_group_names(card_db, &matching, group_names).u8_count(),
            _ => {
                let name_sets: Vec<Vec<String>> = matching
                    .iter()
                    .map(|&cid| card_db.get_card_names(cid))
                    .collect();
                util::max_distinct_names(&name_sets).distinct.u8_count()
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
                            card.name_key.contains(e.as_str())
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
            .count()
            .u8_count();
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
                let is_live_zone = zone.is_none_or(|z| {
                    matches!(z, Zone::LiveCardZone | Zone::SuccessLiveZone)
                });

                if is_live_zone {
                    let cheer_blade = self.game_state.cheer_count_by_id(&player.id);
                    let constant_bonus = self.game_state.score_bonus_by_id(&player.id);

                    let score_flat: HashMap<i16, i16> = self
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
                                let modifier = score_flat.get(&card_id).copied().unwrap_or(0);
                                total = total.saturating_add(crate::constants::effective_stat(
                                    card.get_score(),
                                    modifier,
                                ));
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
                            live_score
                                + success_score
                                + cheer_blade
                                + crate::constants::saturate_u8(constant_bonus as i32)
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
                // `effective_stat` already saturates per card, so the running
                // total only has to guard its own accumulation — three stage
                // members can each sit at u8::MAX.
                let mut total_cost: u8 = 0;
                for card_id in &player.stage.stage {
                    if *card_id != -1 {
                        if let Some(card) = self.game_state.card_database.get_card(*card_id) {
                            total_cost = total_cost.saturating_add(crate::constants::effective_stat(
                                card.cost.unwrap_or(0),
                                self.game_state.mods.get_cost_modifier(*card_id),
                            ));
                        }
                    }
                }
                total_cost
            }
            Some("energy") => player.energy_zone.cards.len().u8_count(),
            _ => self.zone_len(player, location),
        }
    }

    /// The count for 「それがスコアN以上の『X』のライブカードの場合」 where 「それ」 is
    /// what the preceding step moved or selected, not a zone.
    ///
    /// EVERY matching card must clear the threshold, so the count is the minimum
    /// over the matches. No match at all yields 0, which fails the comparison —
    /// "no card qualified" is not the same as "the threshold was met".
    fn count_for_preceding_moved_score(&self, condition: &Condition) -> u8 {
        let card_db = &self.game_state.card_database;
        let pool = self.moved_card_pool();
        let mut matched_min: Option<u8> = None;
        for &id in &pool {
            if id == -1 {
                continue;
            }
            if let Some(groups) = condition.get_group_names() {
                if !groups.is_empty()
                    && !groups
                        .iter()
                        .any(|g| util::card_matches_group_str(card_db, id, Some(g)))
                {
                    continue;
                }
            }
            if let Some(ref ct) = condition.get_card_type() {
                if !util::card_matches_type(card_db, id, Some(ct)) {
                    continue;
                }
            }
            let s = card_db.get_card(id).and_then(|c| c.score).unwrap_or(0);
            matched_min = Some(matched_min.map_or(s, |m: u8| m.min(s)));
        }
        log::debug!(
            "[SCORE_PRECEDING_MOVED] pool={:?} min={:?}",
            pool,
            matched_min
        );
        matched_min.unwrap_or(0)
    }

    pub(crate) fn get_count_for_condition(&self, condition: &Condition) -> u8 {
        let location = condition.get_location().unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let comparison_type = condition.get_comparison_type();
        let resource_type = condition.get_resource_type();
        if comparison_type == Some("score") {
            if condition.get_location().is_none()
                && condition
                    .get_source()
                    .is_some_and(super::is_moved_source)
            {
                return self.count_for_preceding_moved_score(condition);
            }
            // 「自分と相手の成功ライブカード置き場にあるすべてのライブカードのスコアの
            // 合計が10以上であるかぎり」 (PL!-PR-024-PR / PL!N-PR-034-PR) aggregates
            // BOTH seats. `resolve_condition_player("both")` deliberately collapses to
            // self — it exists for the self-vs-opponent comparison shape, where
            // `comparison_target` supplies the other side — so a plain
            // `get_count_for_target(condition, target)` reads only my zone and the
            // opponent's lives are silently dropped from the sum. Add the opponent's
            // half here, where the score total is actually assembled.
            // The sum is only half of the comparison. When `comparison_target` is set
            // (PL!S-pb1-022-L 「スコアが相手と等しい場合」), `comparison_target_count`
            // already supplies the opponent's total as the RIGHT side, and the LEFT
            // side must stay mine alone — folding theirs in here would compare
            // (mine+theirs) against theirs and flip every tied-score clause in the
            // corpus. Only the literal-threshold shape (「…合計が10以上」) needs the
            // other seat folded in, and that one has no `comparison_target`.
            let mut total = self.get_count_for_target(condition, target);
            if self.resolve_target_for_scope(condition) == "both"
                && condition.get_comparison_target().is_none()
            {
                let opp = self.get_count_for_target(condition, "opponent");
                log::debug!(
                    target: "rabuka_engine::ability::condition::card",
                    "[SCORE_BOTH] self={} opponent={} combined={}",
                    total,
                    opp,
                    total.saturating_add(opp)
                );
                total = total.saturating_add(opp);
            }
            return total;
        }
        if comparison_type == Some("cost") {
            if condition.get_location().is_none()
                || condition.get_location() == Some("revealed_cards")
            {
                let card_db = &self.game_state.card_database;
                let ct = condition.get_card_type().map(|ct| ct.as_str());
                return self
                    .revealed_cards_scoped(condition)
                    .iter()
                    .filter(|&&id| ct.is_none() || util::card_matches_type(card_db, id, ct))
                    .map(|&id| {
                        let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0);
                        crate::constants::effective_stat(
                            base,
                            self.game_state.mods.get_cost_modifier(id),
                        )
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
                let exclude_id = if condition.get_exclude_self().unwrap_or(false) {
                    self.activating_card_id
                } else {
                    None
                };
                let mut filter = condition.filter_subset();
                filter.exclude_self = exclude_id;
                let groups_vec = condition.get_group_names().map(|v| v.to_vec());
                filter.groups = groups_vec.as_deref();
                let mut max_cost: u8 = 0;
                for id in util::zone_cards_occupied(player, location) {
                    if !filter.matches(card_db, id, false) {
                        continue;
                    }
                    if let Some(base) = card_db.get_card(id).and_then(|c| c.cost) {
                        let v = crate::constants::effective_stat(
                            base,
                            self.game_state.mods.get_cost_modifier(id),
                        );
                        if v > max_cost {
                            max_cost = v;
                        }
                    }
                }
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
                    let total: u8 = util::zone_cards_occupied(player, loc)
                        .filter(|&id| {
                            if let Some(groups) = condition.get_group_names() {
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
                        .map(|id| {
                            let base = card_db.get_card(id).and_then(|c| c.cost).unwrap_or(0);
                            crate::constants::effective_stat(
                                base,
                                self.game_state.mods.get_cost_modifier(id),
                            )
                        })
                        .sum();
                    return total;
                }
            }
        }
        if resource_type == Some("hand_count") {
            let player = self.resolve_condition_player(target);
            return player.hand.len().u8_count();
        }
        if let Some(rt) = resource_type {
            if rt.starts_with("heart") {
                let clean: String = rt.chars().filter(|&c| c != '_').collect();
                let color = crate::card::parse_heart_color(&clean);
                let player = self.resolve_condition_player(target);
                let card_db = &self.game_state.card_database;
                let position = condition.get_position().and_then(|p| p.get_position());
                return Self::heart_count_at_or_all(player, card_db, color, position);
            }
        }
        if resource_type == Some("surplus_heart") {
            return self.count_surplus_heart(condition, target);
        }
        // NOTE: 「自分のステージにエネルギーカードが下にあるメンバーがいる場合」
        // (PL!N-bp5-013-N) also carries `resource_type: energy`, and the branch below
        // short-circuits to the energy-zone length without reading `location` — which is
        // exactly the misreading that made the clause ask 「do I have energy?」 instead of
        // 「does anyone on my stage have energy beneath them?」. It is handled in
        // `resolve_zone_card_count`, which is where a `card_count_condition` is actually
        // counted; this function is not on that path.
        if resource_type == Some("energy") {
            let player = self.resolve_condition_player(target);
            let count = player.energy_zone.cards.len().u8_count();
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
            && condition
                .get_source()
                .is_some_and(super::is_moved_source)
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
                .count()
                .u8_count();
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
            let total = self.total_effective_cost(self.moved_cards);
            if total > 0 {
                return total;
            }
        }
        // Fallback for conditions referencing revealed cards without
        // explicit comparison_type (e.g. "選んだ数以下の場合" in Kosuzu).
        // Use the cost of revealed cards when available.
        if location.is_empty() && !self.game_state.revealed_cards.is_empty() {
            return self.total_effective_cost(&self.game_state.revealed_cards);
        }
        let player = self.resolve_condition_player(target);
        // Character-name zone count: when a comparison condition specifies
        // `characters` (e.g. "これにより「桜内梨子」か「渡辺曜」を手札に加えた場合")
        // against a location, count the matching cards rather than returning the
        // raw zone length. Otherwise a hand with any cards would satisfy count>=1
        // regardless of which character was actually moved.
        if let Some(chars) = condition.get_characters() {
            if !chars.is_empty() && !location.is_empty() {
                let card_db = &self.game_state.card_database;
                // Prefer cards moved by the preceding effect (this is a
                // "これにより…を手札に加えた場合" result check). This avoids a
                // false positive when a matching character was ALREADY in the
                // zone before the effect ran. Fall back to the whole zone.
                let source_cards: SmallVec<[i16; 8]> = if !self.moved_cards.is_empty() {
                    self.moved_cards.iter().copied().collect()
                } else {
                    util::zone_cards(player, location).iter().copied().collect()
                };
                let matching: u8 = source_cards
                    .iter()
                    .filter(|&&cid| util::card_matches_characters(card_db, cid, Some(chars)))
                    .count()
                    .u8_count();
                return matching;
            }
        }
        self.zone_len(player, location)
    }

    fn count_surplus_heart(&self, condition: &Condition, target: &str) -> u8 {
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
        member_hearts.saturating_sub(needed)
    }

    pub(crate) fn get_count_for_target(&self, condition: &Condition, target: &str) -> u8 {
        let location = condition.get_location().unwrap_or("");
        let resource_type = condition.get_resource_type();
        let comparison_type = condition.get_comparison_type();
        if resource_type == Some("energy") {
            let player = self.resolve_condition_player(target);
            return player.energy_zone.cards.len().u8_count();
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
                Some(Zone::LiveCardZone) => self.sum_group_filtered_zone(
                    &player.live_card_zone.cards,
                    ct,
                    group_name,
                    |card| {
                        card.need_heart
                            .as_ref()
                            .map(|nh| nh.hearts.values_sum())
                            .unwrap_or(0)
                    },
                ),
                Some(Zone::SuccessLiveZone) => self.sum_group_filtered_zone(
                    &player.success_live_card_zone.cards,
                    ct,
                    group_name,
                    |card| card.score.unwrap_or(0),
                ),
                _ => 0,
            };
        }

        let ct = condition.get_card_type().map(|ct| ct.as_str());
        let exc = condition.get_exclude_characters();

        // Preceding-moved path: check recently moved cards instead of a zone.
        if condition
            .get_source()
            .is_some_and(super::is_moved_source)
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
                let bm_flat: HashMap<i16, i16> = self
                    .game_state
                    .mods
                    .blade_modifiers
                    .iter()
                    .map(|(&k, e)| (k, e.total()))
                    .collect();
                card_ids
                    .iter()
                    .map(|&cid| {
                        let base = card_db.get_card(cid).map(|c| c.blade).unwrap_or(0);
                        let modifier = bm_flat.get(&cid).copied().unwrap_or(0);
                        crate::constants::effective_stat(base, modifier)
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
