use super::ConditionContext;
use crate::ability::enums::Zone;
use crate::ability::util::{self, compare_counts};
use crate::card::{CardProperty, Condition, HeartColor, TriggerEvent};
use crate::core::constants::CountCast;
use crate::HashSet;
use smallvec::SmallVec;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

impl<'a> ConditionContext<'a> {
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
                        let src_ok =
                            source_zone.is_empty() || m.source_zone.matches_source(source_zone);
                        let cause_ok = (condition.get_self_target().unwrap_or(false)
                            && !require_self_effect)
                            || m.cause_player_id == target_id;
                        src_ok
                            && cause_ok
                            && m.dest_zone.matches_dest(dest_zone)
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
                            let src_ok = m.source_zone.matches_source(source_zone);
                            let cause_ok = (condition.get_self_target().unwrap_or(false)
                                && !require_self_effect)
                                || m.cause_player_id == target_id;
                            src_ok
                                && cause_ok
                              && m.dest_zone.matches_dest(dest_zone)
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
                        let src_ok =
                            source_zone.is_empty() || m.source_zone.matches_source(source_zone);
                        let cause_ok = (condition.get_self_target().unwrap_or(false)
                            && !require_self_effect)
                            || m.cause_player_id == target_id;
                        src_ok
                            && cause_ok
                              && m.dest_zone.matches_dest(dest_zone)
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
                    return is_new_movement && condition.get_resource_type() == Some("energy");
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
                            .filter(|s| !super::is_moved_source(s))
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
                                let src_ok = m.source_zone.matches_source(src_zone);
                                src_ok
                                      && m.dest_zone.matches_dest(dest)
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
        let unit_is_types = condition.get_unit() == Some("types");
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
            colors.len().u8_count()
        } else {
            moved_ids.len().u8_count()
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
        let is_from = condition.get_area_direction() == Some("from");
        events.iter().any(|event| {
            if condition.get_self_target().unwrap_or(false)
                && self.activating_card_id != Some(event.moved_card_id)
            {
                return false;
            }
            if let Some(groups) = condition.get_group_names() {
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
            if let Some(req_pos) = condition.get_position().and_then(|p| p.get_position()) {
                let check_pos = if is_from {
                    event.old_position
                } else {
                    event.new_position
                };
                if (check_pos as usize) >= pos_names.len()
                    || pos_names[check_pos as usize] != req_pos
                {
                    return false;
                }
            }
            true
        })
    }

    /// A cost-limited 「〜何人かのカードが動いた場合」 condition is satisfied
    /// when at least one recently moved card's cost compares as the limit says.
    /// A missing move log fails the condition rather than passing it: a
    /// condition about movement cannot be true when nothing moved.
    fn moved_cards_match_cost_limit(&self, condition: &Condition) -> bool {
        let Some(cost_limit) = condition.get_cost_limit() else {
            return true;
        };
        let op = condition
            .get_cost_limit_operator()
            .map(|o| o.as_str())
            .unwrap_or(">=");
        let Some(moved) = self.game_state.recently_moved_cards.as_ref() else {
            return false;
        };
        moved.iter().any(|&cid| {
            self.game_state
                .card_database
                .get_card(cid)
                .is_some_and(|c| c.cost.is_some_and(|cost| compare_counts(Some(op), cost, cost_limit)))
        })
    }

    /// A 「バトンタッチした」 condition.
    ///
    /// The condition is written from the point of view of ONE member — the
    /// one whose own ability declares the trigger. Which card the group's
    /// qualifier (group_names / cost_limit / card_property) applies to depends
    /// on where that member ended up: replaced members sit in a waitroom, so a
    /// waitroom `location` means the qualifier describes the ARRIVING member,
    /// `true` only when `ABILITY_DEBUG_BATON_TOUCH` is set.
    ///
    /// These traces are load-bearing for whoever is debugging the baton-touch
    /// condition and must not be removed — but they cannot be left on by
    /// default. `evaluate_baton_touch_condition` runs from the condition
    /// evaluator, so an unconditional `eprintln!` here fires on every ability
    /// check in every game and dumps the entire waitroom each time. That makes
    /// the engine **UNTRACED**: `BOT_STRATEGY.md` section 8.4 records that
    /// logging perturbs allocation layout and changes outcomes, and a harness
    /// that measured a run with this on was measuring the logging. It is
    /// roughly 40 lines of stderr per game, which also buries the `--brief`
    /// verdict line the sweeps read.
    ///
    /// So: off unless asked for, and the flag is in the `ARENA --env` form so a
    /// run that needs it can record that it used it.
    #[cfg(not(feature = "no_std"))]
    fn baton_touch_trace() -> bool {
        // Cached. This is called from `evaluate_baton_touch_condition`, i.e. once per
        // baton-touch condition evaluation, and `std::env::var_os` linearly scans the
        // process environment on every call — a real cost on a path the doc above already
        // records as sensitive ("logging perturbs allocation layout and changes
        // outcomes"). The flag is supplied before the process starts, via `ARENA --env`,
        // so reading it once is behaviourally identical.
        static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *TRACE.get_or_init(|| std::env::var_os("ABILITY_DEBUG_BATON_TOUCH").is_some())
    }

    /// `no_std` console targets have no process environment to probe, so the
    /// trace gate is hard-wired off there. Every `if trace { .. }` body in
    /// `evaluate_baton_touch_condition` is empty, so returning `false` here is
    /// behaviourally identical to a host build run without the flag set — the
    /// debug block is simply gated out on the targets that cannot reach it.
    #[cfg(feature = "no_std")]
    fn baton_touch_trace() -> bool {
        false
    }

    /// while a stage `location` means it describes the REPLACED one.
    fn evaluate_baton_touch_condition(
        &self,
        condition: &Condition,
        te: Option<&TriggerEvent>,
        location: &str,
        player: &crate::player::Player,
    ) -> bool {
            let triggered = condition.get_baton_touch_trigger().unwrap_or(false);
            let trace = Self::baton_touch_trace();
            if trace {
            }
            if !triggered {
                return false;
            }
            // Check per-player baton touch count
            let player_id = player.id.as_str();
            let bt_count = self.game_state.get_baton_touch_count(player_id);
            if trace {
            }
            if bt_count == 0 {
                if trace {
                }
                return false;
            }
            if let Some(min_count) = condition
                .get_min_baton_touch_count()
                .or_else(|| te.and_then(|t| t.min_count))
            {
                if bt_count < min_count {
                    return false;
                }
            }
            let replaced_id = match self.game_state.baton_touch_replaced_member_id {
                Some(id) => id,
                None => {
                    if trace {
                    }
                    return false;
                }
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
            if !location.is_empty()
                && (Zone::from_str(location) == Some(Zone::Discard)
                    || Zone::from_str(location) == Some(Zone::Waitroom))
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
            if condition.get_exclude_self().unwrap_or(false)
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
            if let Some(groups) = condition.get_group_names() {
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
                    if let Some(af) = condition.get_ability_filter() {
                        match &**af {
                            "no_ability" => {
                                if !card.abilities.is_empty() {
                                    return false;
                                }
                            }
                            "has_ability"
                                if card.abilities.is_empty() => {
                                    return false;
                                }
                            _ => {}
                        }
                    }
                    if let Some(ct) = condition.get_card_type() {
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
                .get_baton_touch_source()
                .or_else(|| te.and_then(|t| t.source_character.as_deref()));
            if let Some(source_name) = bt_source {
                if let Some(card) = self.game_state.card_database.get_card(replaced_id) {
                    // `source_name` (`baton_touch_source` / `source_character`) is
                    // squashed by the parser; `name_key` is the card side.
                    if !card.name_key.contains(source_name) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if let Some(cost_limit) = condition.get_cost_limit() {
                if let Some(check_card) = check_id_for_group {
                    if let Some(card) = self.game_state.card_database.get_card(check_card) {
                        let op = condition
                            .get_cost_limit_operator()
                            .map(|o| o.as_str())
                            .unwrap_or(">=");
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
            if let Some(prop) = condition.get_card_property() {
                if let Some(check_card) = check_id_for_group {
                    let has_prop = match prop {
                        CardProperty::HasBladeHeart => self
                            .game_state
                            .card_database
                            .get_card(check_card)
                            .is_some_and(|c| c.has_blade_heart()),
                        _ => false,
                    };
                    if condition.get_negation().unwrap_or(false) == has_prop {
                        return false;
                    }
                }
            }
            let has_cost_comparison = condition.get_comparison_type() == Some("cost")
                || te.is_some_and(|t| t.cost_comparison.is_some());
            if has_cost_comparison {
                if let Some(replaced_cost) = self.game_state.baton_touch_replaced_member_cost {
                    if let Some(activating_id) = self.game_state.activating_card {
                        if let Some(card) =
                            self.game_state.card_database.get_card(activating_id)
                        {
                            if let Some(current_cost) = card.cost {
                                let op = condition.get_operator().or_else(|| {
                                    te.and_then(|t| {
                                        t.cost_comparison
                                            .as_ref()
                                            .and_then(|cc| cc.operator.map(|o| o.as_str()))
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

    pub(crate) fn evaluate_movement_condition(&self, condition: &Condition) -> bool {
        let movement = condition.get_movement().unwrap_or("");
        let te = condition.get_trigger_event();
        let location = condition
            .get_location()
            .or_else(|| te.and_then(|t| t.location.as_deref()))
            .unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);

        match movement {
            "moved" => {
                self.evaluate_has_moved(condition, player)
                    && self.moved_cards_match_cost_limit(condition)
            }
            "position_change" => {
                let pce_match = self.position_event_matches_filters(
                    &self.game_state.position_change_events,
                    condition,
                );
                let tm_match = if !pce_match && self.activating_card_id.is_some() {
                    // Cross-player watcher entries (「対戦相手のカードの効果でも
                    // 発動する。」) are enqueued by fire_opponent_cause_watchers
                    // with a trigger_moved_cards SNAPSHOT, but the enqueuing
                    // player's batch loop clears recently_moved /
                    // position_change_events before the other seat's queue
                    // resolves. The entry snapshot is the surviving evidence.
                    let entry_snapshot = self.game_state.entry_trigger_moved_cards();
                    log::debug!(
                        "[POS_CHANGE_EVAL] activating={:?} pce_match={} entry_snapshot={:?} recently={:?}",
                        self.activating_card_id,
                        pce_match,
                        entry_snapshot.as_ref().map(|v| v.to_vec()),
                        self.game_state.recently_moved_cards.as_ref().map(|v| v.to_vec()),
                    );
                    let in_current_batch = |cid: i16| {
                        cid == self.activating_card_id.unwrap_or(-1)
                            && (self.moved_cards.contains(&cid)
                                || entry_snapshot.as_ref().is_some_and(|v| v.contains(&cid))
                                || self
                                    .game_state
                                    .recently_moved_cards
                                    .as_ref()
                                    .is_some_and(|v| v.contains(&cid)))
                    };
                    self.game_state.turn_area_movements.iter().any(|m| {
                        self.activating_card_id == Some(m.moved_card_id)
                            && m.effect_only
                            && in_current_batch(m.moved_card_id)
                    })
                } else {
                    false
                };
                (pce_match || tm_match) && self.moved_cards_match_cost_limit(condition)
            }
            "notmoved" => true,
            "live_success" => self.game_state.live_success_triggered_this_turn,
            "baton_touch" => self.evaluate_baton_touch_condition(
                condition,
                te,
                location,
                player,
            ),
            "moves" => {
                let self_effect_only = condition
                    .get_self_effect_only()
                    .or_else(|| te.and_then(|t| t.self_effect_only));
                let energy_placed = condition
                    .get_energy_placed()
                    .or_else(|| te.and_then(|t| t.energy_placed));

                let snapshot_area = self.game_state.entry_snapshot_last_area_move_card_id();
                let snapshot_area_player =
                    self.game_state.entry_snapshot_last_area_move_by_player();

                // Sources: position_change_events (batch-scoped, from position
                // change executors) + turn_area_movements (turn-scoped, from
                // push_movement_event) with a batch-scoped guard using the
                // event's moved_cards or recently_moved_cards.
                // This prevents stale turn-scoped data from matching when
                // the card did not actually move in the current batch.
                let this_card_moved = self.activating_card_id.is_some_and(|cid| {
                    let in_pce = self
                        .game_state
                        .position_change_events
                        .iter()
                        .any(|e| e.moved_card_id == cid);
                    let in_tam = self
                        .game_state
                        .turn_area_movements
                        .iter()
                        .any(|m| m.moved_card_id == cid);
                    let in_current_batch = self.moved_cards.contains(&cid)
                        || self
                            .game_state
                            .recently_moved_cards
                            .as_ref()
                            .is_some_and(|v| v.contains(&cid));
                    // Deferred cross-seat resolutions run AFTER the movement
                    // batch is cleared; the enqueue-time snapshot of the
                    // triggering batch is the durable record that THIS card's
                    // move caused this very trigger.
                    let in_trigger_batch = self
                        .game_state
                        .entry_trigger_moved_cards()
                        .is_some_and(|tm| tm.contains(&cid));
                    in_pce || (in_tam && in_current_batch) || in_trigger_batch
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
                    // Optional destination constraint (e.g. "メンバーの下に置かれた").
                    let want_dest = condition
                        .get_destination()
                        .or_else(|| te.and_then(|t| t.destination.as_deref()))
                        .unwrap_or("");
                    // Dest-aware placement match: a bare "置き場に置かれた"
                    // watches ONLY energy/energy_zone arrivals. A
                    // zone->under_member move satisfies ONLY an explicit
                    // want_dest=="under_member" watcher (and vice versa), so
                    // effects parking energy under a member never arm Ren.
                    let is_watched_placement = |m: &crate::types::MovementEvent| {
                        let dz_ok = if want_dest.is_empty() {
                            m.dest_zone == "energy" || m.dest_zone == "energy_zone"
                        } else {
                            m.dest_zone == want_dest
                        };
                        dz_ok && m.effect_only
                    };
                    let snapshot_has = self
                        .game_state
                        .ability_queue
                        .current_entry()
                        .map(|e| e.snapshot_movements.iter().any(&is_watched_placement))
                        .unwrap_or(false);
                    let live_has = self
                        .game_state
                        .batch_movements
                        .iter()
                        .any(&is_watched_placement);
                    // Pre-enqueue scans have no current entry yet, so the live
                    // batch governs; post-enqueue the durable snapshot does.
                    let energy_val = snapshot_has || live_has;
                    // Verify the placed energy's destination matches, when constrained.
                    // Check both the entry snapshot (persistent) and the live batch.
                    let dest_ok = if want_dest.is_empty() {
                        true
                    } else {
                        let snapshot_dest_ok = self
                            .game_state
                            .ability_queue
                            .current_entry()
                            .map(|e| {
                                e.snapshot_movements.iter().any(|m| {
                                    (m.dest_zone == "energy"
                                        || m.dest_zone == "energy_zone"
                                        || m.dest_zone == "under_member")
                                        && m.dest_zone == want_dest
                                })
                            })
                            .unwrap_or(false);
                        let live_dest_ok = self.game_state.batch_movements.iter().rev().any(|m| {
                            (m.dest_zone == "energy"
                                || m.dest_zone == "energy_zone"
                                || m.dest_zone == "under_member")
                                && m.dest_zone == want_dest
                        });
                        snapshot_dest_ok || live_dest_ok
                    };
                    let snapshot_player = self
                        .game_state
                        .ability_queue
                        .current_entry()
                        .and_then(|e| {
                            e.snapshot_movements
                                .iter()
                                .rev()
                                .find(|m| is_watched_placement(m))
                                .map(|m| m.cause_player_id.clone())
                        });
                    let live_player = self
                        .game_state
                        .batch_movements
                        .iter()
                        .rev()
                        .find(|m| is_watched_placement(m))
                        .map(|m| m.cause_player_id.clone());
                    // Most-recent watched placement wins: a mixed batch whose
                    // latest placement is opponent-caused must NOT satisfy
                    // self_effect_only (the old first-match lookup did).
                    let energy_player = snapshot_player.or(live_player);
                    let player_ok = !self_effect_only.unwrap_or(false)
                        || energy_player.as_deref() == Some(player.id.as_str());
                    let verdict = energy_val && dest_ok && player_ok;
                    log::debug!(
                        "[ENERGY_PLACED_VERDICT] want_dest='{}' snapshot={} live={} val={} dest_ok={} player={:?} self_only={:?} verdict={}",
                        want_dest,
                        snapshot_has,
                        live_has,
                        energy_val,
                        dest_ok,
                        energy_player,
                        self_effect_only,
                        verdict
                    );
                    verdict
                });
                // watches_area_move disambiguates pure-energy texts (Ren
                // bp7-005 ab#1 — no move language, flag absent because the
                // pipeline normalizer strips False) from 〜か compounds
                // (Sumire bp5-004 ab#0: True — either disjunct arms it).
                // Fallback preserves legacy behavior: an energy watcher
                // without the flag watches no area move; anything else keeps
                // the old formula (including the vacuous-true for triggers
                // like baton_touch that check neither).
                let watches_area = condition
                    .get_watches_area_move()
                    .or_else(|| te.and_then(|t| t.watches_area_move));
                let has_area_check = watches_area.unwrap_or(false)
                    || (energy_placed.is_none()
                        && (self_effect_only.is_some()
                            || condition.get_movement().unwrap_or("") == "moves"));
                let has_energy_check = energy_placed.is_some();
                log::debug!(
                    "[MOVES_VERDICT] watches_area={:?} has_area={} has_energy={} area_ok={} energy_ok={}",
                    watches_area, has_area_check, has_energy_check, area_ok, energy_ok
                );
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
    /// NOT YET ENFORCED — see the note below before changing this.
    ///
    /// The subject of 「…が…移動した」 is not always the activating card. Two
    /// qualifier shapes are parsed and this evaluator discards both:
    ///   * `exclude_self` — 「このターン、**ほかの**メンバーがエリアを移動して
    ///     いる場合」 (PL!SP-bp5-014-N 嵐 千砂都): any card but this one.
    ///   * `group_names` (+ `position` / `location`) — 「自分のステージの
    ///     センターエリアにいる『Liella!』のメンバーが、このターン中に移動して
    ///     いる場合」 (PL!SP-bp4-025-L ab#1): the mover is a filtered stage
    ///     member, not the activating card.
    ///
    /// A fix was attempted and reverted: with `subject_is_activating` forced
    /// true (this original behaviour) and with the qualifier branch active, the
    /// whole 3613-test suite passes identically, and no test distinguishes the
    /// two — so the branch is unverified and must not ship. Fixing this needs a
    /// test that actually reaches the two subject shapes and fails without them
    /// (drive a REAL position change of a qualifying member into the center, and
    /// of a non-qualifying member, then check the live-success score).
    fn evaluate_has_moved(&self, condition: &Condition, _player: &crate::player::Player) -> bool {
        let _ = condition;
        self.game_state
            .ability_queue
            .current_entry()
            .and_then(|entry| entry.triggering_member_id)
            .or(self.activating_card_id)
            .is_some_and(|cid| {
                self.game_state
                    .position_change_events
                    .iter()
                    .any(|event| event.moved_card_id == cid)
            })
    }

}
