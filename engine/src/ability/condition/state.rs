use super::ConditionContext;
use crate::ability::enums::Zone;
use crate::ability::util;
use crate::card::Condition;
use crate::core::constants::CountCast;
use crate::game_state::Phase;
use crate::HashSet;
#[cfg(feature = "no_std")]
use alloc::string::ToString;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;

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
        let temporal = condition.get_temporal().unwrap_or("");
        let phase = condition.get_phase();
        log::debug!(
            "[TEMPORAL_EVAL] temporal={:?} phase={:?} has_nested={} aggregate={:?}",
            temporal,
            phase,
            condition.get_condition().is_some(),
            condition.get_aggregate()
        );

        match temporal {
            "this_turn" => {
                if let Some(count) = condition.get_count() {
                    if Zone::from_str(condition.get_location().unwrap_or("")) == Some(Zone::Stage)
                        && condition.get_card_type().map(|ct| ct.as_str()) == Some("member_card")
                    {
                        let target = condition.get_target().unwrap_or("self");
                        let player = self.resolve_condition_player(target);
                        if let Some(groups) = condition.get_group_names() {
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
                    .get_temporal_scope()
                    .as_ref()
                    .and_then(|s| s.parse::<u8>().ok())
                {
                    created_turn == self.game_state.turn_number
                } else {
                    if let Some(nested_condition) = condition.get_condition() {
                        let nested_movement = match nested_condition {
                            Condition::Movement { movement, .. } => {
                                movement.as_deref().unwrap_or("not_moved")
                            }
                            _ => "",
                        };
                        log::debug!("[TEMPORAL_TURN] nested movement={:?}", nested_movement);
                        match nested_movement {
                            "not_moved" => {
                                if let Some(activating_card_id) = self.activating_card_id {
                                    !self.game_state.has_card_moved_this_turn(activating_card_id)
                                } else {
                                    true
                                }
                            }
                            "has_moved" => {
                                let check_card = self
                                    .game_state
                                    .ability_queue
                                    .current_entry()
                                    .and_then(|entry| entry.triggering_member_id)
                                    .or_else(|| {
                                        condition.get_position().and_then(|pos| {
                                            pos.get_position().and_then(|pos_str| {
                                                let target =
                                                    condition.get_target().unwrap_or("self");
                                                let player = self.resolve_condition_player(target);
                                                util::stage_position_index(pos_str).and_then(
                                                    |idx| {
                                                        if idx < 3 && player.stage.stage[idx] != -1
                                                        {
                                                            Some(player.stage.stage[idx])
                                                        } else {
                                                            None
                                                        }
                                                    },
                                                )
                                            })
                                        })
                                    });
                                if let Some(card_id) = check_card {
                                    // Positioned subject (「センターエリアにいる…メン
                                    // バー」): the member CURRENTLY at the named slot
                                    // must match the group filter and have moved.
                                    let groups = nested_condition
                                        .get_group_names()
                                        .map(|g| g.to_vec())
                                        .or_else(|| {
                                            condition.get_group_names().map(|g| g.to_vec())
                                        });
                                    let card_db = &self.game_state.card_database;
                                    let in_group = groups.as_ref().is_none_or(|g| {
                                        g.iter().any(|name| {
                                            crate::ability::util::card_matches_group_str(
                                                card_db,
                                                card_id,
                                                Some(name),
                                            )
                                        })
                                    });
                                    let moved = self.game_state.has_card_moved_this_turn(card_id);
                                    log::debug!(
                                        "[TEMPORAL_TURN] has_moved positioned card={} in_group={} moved={}",
                                        card_id, in_group, moved
                                    );
                                    in_group && moved
                                } else if condition.get_position().is_some() {
                                    // Position named but the slot is empty: nobody
                                    // currently satisfies 「<position>にいる」 — never
                                    // widen to a whole-stage scan (Special Color's
                                    // center Liella! leaving center must stop scoring).
                                    log::debug!(
                                        "[TEMPORAL_TURN] has_moved position slot empty -> false"
                                    );
                                    false
                                } else if self.game_state.position_change_occurred_this_turn {
                                    let target = condition.get_target().unwrap_or("self");
                                    let player = self.resolve_condition_player(target);
                                    // Group restriction (「『Liella!』のメンバーが…移動し
                                    // ているかぎり」): only members matching the nested/
                                    // outer group filter count as satisfying the gate.
                                    let groups = nested_condition
                                        .get_group_names()
                                        .map(|g| g.to_vec())
                                        .or_else(|| {
                                            condition.get_group_names().map(|g| g.to_vec())
                                        });
                                    let card_db = &self.game_state.card_database;
                                    player.stage.stage.iter().any(|&cid| {
                                        cid != -1
                                            && self.game_state.has_card_moved_this_turn(cid)
                                            && groups.as_ref().is_none_or(|g| {
                                                g.iter().any(|name| {
                                                    crate::ability::util::card_matches_group_str(
                                                        card_db,
                                                        cid,
                                                        Some(name),
                                                    )
                                                })
                                            })
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
                        log::debug!(
                            "[TEMPORAL_TURN] no nested condition for temporal=this_turn -> default true"
                        );
                        true
                    }
                }
            }
            "live_end" => matches!(
                self.game_state.current_phase,
                crate::game_state::Phase::LiveVictoryDetermination
            ),
            "during_live" | "this_live" => {
                if !self.game_state.current_phase.is_live_phase() {
                    log::debug!(
                        "[DURING_LIVE] phase gate failed: phase={:?}",
                        self.game_state.current_phase
                    );
                    return false;
                }
                // Build list of zones to check from condition.locations or condition.location
                let zones_to_check: Vec<&str> = {
                    let mut zones = Vec::new();
                    if let Some(locs) = condition.get_locations() {
                        for z in locs {
                            zones.push(z.as_str());
                        }
                    } else if let Some(loc) = condition.get_location() {
                        zones.push(loc);
                    } else if condition.get_heart_colors().is_some() {
                        // If checking heart colors but no zone specified, default to live_card_zone
                        zones.push("live_card_zone");
                    }
                    zones
                };
                let has_success_or_live_zone = zones_to_check
                    .iter()
                    .any(|z| *z == "success_live_card_zone" || *z == "live_card_zone");
                if has_success_or_live_zone {
                    let target = condition.get_target().unwrap_or("self");
                    let player = self.resolve_condition_player(target);
                    log::debug!(
                        "[DURING_LIVE] zones={:?} live_cards={:?}",
                        zones_to_check,
                        player.live_card_zone.cards
                    );

                    // Aggregate total with heart_colors: sum need_heart across all live cards
                    if condition.get_aggregate() == Some("total")
                        && condition.get_heart_colors().is_some_and(|c| !c.is_empty())
                    {
                        let location = condition.get_location().unwrap_or("live_card_zone");
                        let res = self
                            .check_aggregate_total(condition, player, location)
                            .unwrap_or(false);
                        log::debug!("[DURING_LIVE] aggregate_total -> {}", res);
                        return res;
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
                            .load(core::sync::atomic::Ordering::Relaxed)
                        {
                            log::debug!(
                                "[TEMP_DIAG] checking zone={} {} cards={:?}",
                                zone_name,
                                cards.len(),
                                cards
                            );
                        }
                        for &cid in &cards {
                            if let Some(card) = self.game_state.card_database.get_card(cid) {
                                let group_ok = condition.get_group_names().is_none_or(|groups| {
                                    groups.iter().any(|g| {
                                        crate::ability::util::card_matches_group_str(
                                            &self.game_state.card_database,
                                            cid,
                                            Some(g),
                                        )
                                    })
                                });
                                if crate::ability::debug::ABILITY_DEBUG
                                    .load(core::sync::atomic::Ordering::Relaxed)
                                {
                                    log::debug!(
                                        "[TEMP_DIAG]   card={} name={} group_ok={} nh={:?}",
                                        cid,
                                        card.name,
                                        group_ok,
                                        card.need_heart
                                    );
                                }
                                if !group_ok {
                                    continue;
                                }
                                if let Some(hc_list) = condition.get_heart_colors() {
                                    if let Some(ref nh) = card.need_heart {
                                        // Check if ALL specified heart colors meet the count threshold
                                        let threshold = condition.get_count().unwrap_or(1);
                                        let all_hearts_present = hc_list.iter().all(|color_str| {
                                            let color = crate::card::parse_heart_color(color_str);
                                            nh.hearts.get(&color).copied().unwrap_or(0) >= threshold
                                        });
                                        if crate::ability::debug::ABILITY_DEBUG
                                            .load(core::sync::atomic::Ordering::Relaxed)
                                        {
                                            log::debug!(
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
                                            .load(core::sync::atomic::Ordering::Relaxed)
                                        {
                                            log::debug!("[TEMP_DIAG]   no need_heart on card");
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
                            .load(core::sync::atomic::Ordering::Relaxed)
                        {
                            log::debug!("[TEMP_DIAG] CONDITION FAILED: no matching card in zones");
                        }
                        return false;
                    }
                    if crate::ability::debug::ABILITY_DEBUG
                        .load(core::sync::atomic::Ordering::Relaxed)
                    {
                        log::debug!("[TEMP_DIAG] CONDITION PASSED");
                    }
                }
                true
            }
            "before_live" => !self.game_state.current_phase.is_live_phase(),
            "first_turn" => self.game_state.is_first_turn(),
            _ => {
                let turn_ok = match condition.get_turn_number() {
                    Some(tn) => self.game_state.turn_number == tn,
                    None => true,
                };
                if !turn_ok {
                    return false;
                }
                if let Some(phase_str) = phase {
                    // NOTE: the phase key is intentionally matched verbatim, and
                    // the catch-all stays `_ => true`. Restriction-shaped
                    // conditions name the phase to say WHEN THE LOCK BITES
                    // (「…はアクティブフェイズにアクティブにならない」 — a static
                    // restriction that must be registered so those members
                    // never become active), not "only check me during that
                    // phase". Normalising "active_phase" -> "active" here made
                    // the lock disappear outside the Active phase and broke
                    // PL!HS-pb1-008-R and 絢瀬 かな. The real phase gate lives in
                    // condition.rs::check_phase_gate, which does normalise.
                    match phase_str {
                        "active" => matches!(
                            self.game_state.current_phase,
                            crate::game_state::Phase::Active
                        ),
                        "live_phase" | "live" => self.game_state.current_phase.is_live_phase(),
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
        let state = condition.get_state().map(|s| s.as_str()).unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let resource_type = condition.get_resource_type();
        let all_cards = condition.get_all().unwrap_or(false);
        let player = self.resolve_condition_player(target);

        if resource_type == Some("energy") {
            match state {
                "active" => {
                    if all_cards {
                        player.energy_zone.active_count() as usize == player.energy_zone.cards.len()
                    } else {
                        player.energy_zone.active_count() > 0
                    }
                }
                "wait" => {
                    if all_cards {
                        player.energy_zone.active_count() == 0
                    } else {
                        player.energy_zone.active_count()
                            < player.energy_zone.cards.len().u8_count()
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
                    // For "このメンバーがウェイト..." self-state, card_type=member_card is parser default and should NOT be treated as an ANY filter.
                    // Otherwise every waited member on stage would grant the heart to every Erena copy (two-copy bug).
                    let is_self = condition.get_self_target().unwrap_or(false)
                        || condition.get_check_self().unwrap_or(false);
                    let has_filter = if is_self {
                        // Force self-branch for このメンバー self-state regardless of generic card_type.
                        false
                    } else {
                        condition.get_group_names().is_some()
                            || condition.get_card_type().is_some()
                            || condition.get_characters().is_some_and(|c| !c.is_empty())
                    };
                    let check_orientation = |cid: i16| -> bool {
                        crate::ability::util::orientation_matches_state(
                            self.game_state.mods.get_orientation_modifier(cid),
                            state,
                        )
                    };
                    // Three distinct shapes, and the third used to be read as the
                    // first. `has_filter` is forced false whenever the condition
                    // says it is about 自己, so the old `if has_filter {…} else
                    // {activating card}` collapsed "explicitly self" together with
                    // "no filter at all" and answered both with the activating
                    // card.
                    //
                    // 「自分のステージのメンバーがウェイト状態」 states no group, no
                    // card type and no self_target, and means ANY member in that
                    // state. Answering it about the activating card is a plausible
                    // wrong answer rather than a failure: the copy that triggered it
                    // satisfies its own condition.
                    //
                    // No card in the corpus produces that shape — all ten
                    // stage-state conditions carry a group or a card type — so this
                    // is the default, not a fix to current output.
                    if is_self {
                        self.activating_card_id
                            .is_some_and(|cid| stage_cards.contains(&cid) && check_orientation(cid))
                    } else if has_filter {
                        stage_cards.iter().any(|&cid| {
                            check_orientation(cid)
                                && self.card_matches_count_filters(
                                    cid,
                                    condition.get_card_type().map(|ct| ct.as_str()),
                                    condition.get_group_names(),
                                    &[],
                                    condition.get_cost_limit(),
                                    condition.get_cost_limit_operator().map(|o| o.as_str()),
                                    false,
                                    condition,
                                )
                        })
                    } else {
                        stage_cards.iter().any(|&cid| check_orientation(cid))
                    }
                }
                _ => true,
            }
        }
    }

    pub(crate) fn evaluate_energy_state_condition(&self, condition: &Condition) -> bool {
        let energy_state = condition.get_energy_state().unwrap_or("");
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let result = match energy_state {
            "active" => player.energy_zone.active_count() > 0,
            _ => true,
        };
        log::debug!(
            "[ENERGY_STATE] target={} energy_state={} active_count={} total={} result={} activating={:?} self_player={:?}",
            target,
            energy_state,
            player.energy_zone.active_count(),
            player.energy_zone.cards.len(),
            result,
            self.activating_card_id,
            self.self_player.map(|p| p.id.as_str())
        );
        result
    }

    pub(crate) fn evaluate_score_threshold_condition(&self, condition: &Condition) -> bool {
        let count = condition.get_count().unwrap_or(1);
        let operator = condition.get_operator();
        let target = condition.get_target().unwrap_or("self");
        let player = self.resolve_condition_player(target);
        let cheer_count = if player.is_first_attacker {
            self.game_state.player1_cheer_blade_heart_count
        } else {
            self.game_state.player2_cheer_blade_heart_count
        };
        util::compare_counts(operator, cheer_count, count)
    }

    pub(crate) fn evaluate_choice_condition(&self, condition: &Condition) -> bool {
        if let Some(options) = condition.get_options() {
            !options.is_empty()
        } else {
            true
        }
    }

    pub(crate) fn evaluate_state_change_condition(&self, condition: &Condition) -> bool {
        let _during_main_phase = condition
            .get_text()
            .map(|t| t.contains("main_phase"))
            .unwrap_or(false);
        if _during_main_phase && self.game_state.current_phase != Phase::Main {
            return false;
        }
        if let (Some(from), Some(to)) = (condition.get_from_state(), condition.get_to_state()) {
            if let Some(target_count) = condition.get_count() {
                if condition.get_operator() == Some(">=") {
                    let actual = self.game_state.last_state_change_wait_to_active_count;
                    return actual >= target_count;
                }
            }
            let target_name = condition.get_target().unwrap_or("self");
            let is_opponent = target_name == "opponent";
            // 「自分か相手のステージにいるメンバーがウェイト状態になるたび」
            // (PL!-PR-023-PR) — the transitioned member may belong to either
            // player, so membership is tested against the union of stages.
            let either_side = target_name == "either" || target_name == "both";
            // First pass: check recently_state_changed for actual transitions.
            // This is the primary source — only cards that actually changed state
            // should satisfy the condition.
            let target_player = if is_opponent {
                self.resolve_condition_player("opponent")
            } else {
                self.resolve_condition_player("self")
            };
            let stage_set: HashSet<i16> = if either_side {
                self.game_state
                    .player1
                    .stage
                    .stage
                    .iter()
                    .chain(self.game_state.player2.stage.stage.iter())
                    .copied()
                    .collect()
            } else {
                target_player.stage.stage.iter().copied().collect()
            };
            log::debug!(
                "[STATE_CHANGE_COND] eval target={} either_side={} staged_members={}",
                target_name,
                either_side,
                stage_set.len()
            );
            // 自分のカードの効果 scoping (e.g. Maki pb1-015 ab#1): only
            // transitions caused by the watcher's own side satisfy the
            // trigger. The cause is recorded at push time; "" = unknown and
            // does NOT satisfy a scoping requirement.
            let require_self_effect = condition.get_self_effect_only().unwrap_or(false);
            let owner_id = self.resolve_condition_player("self").id.clone();
            for (cid, cfrom, cto, cause) in &self.game_state.recently_state_changed {
                if !stage_set.contains(cid) {
                    continue;
                }
                if cfrom != from || cto != to {
                    continue;
                }
                if require_self_effect && cause != &owner_id {
                    log::debug!(
                        "[STATE_CHANGE_COND] card={} transition {}→{} rejected: cause='{}' != owner='{}' (self_effect_only)",
                        cid, cfrom, cto, cause, owner_id
                    );
                    continue;
                }
                // Apply extra filters (cost_limit, etc.)
                if let Some(cl) = condition.get_cost_limit() {
                    let card_db = &self.game_state.card_database;
                    let cost_ok = card_db.get_card(*cid).is_some_and(|c| {
                        let card_cost = c.cost.unwrap_or(0);
                        let op = condition
                            .get_cost_limit_operator()
                            .map(|o| o.as_str())
                            .unwrap_or("<=");
                        // An operator we do not know is a parser gap, not a
                        // cost that failed the limit, so it admits the card.
                        crate::ability::util::compare_with_operator(op, card_cost, cl)
                            .unwrap_or(true)
                    });
                    if !cost_ok {
                        continue;
                    }
                }
                // Apply group filter to the transitioned card itself (e.g. a
                // Niji member became wait — not just any wait while a group
                // member stands around). Mirrors the cost filter above.
                if let Some(groups) = condition.get_group_names() {
                    if !groups.is_empty() {
                        let card_db = &self.game_state.card_database;
                        let group_ok = groups.iter().any(|g| {
                            crate::ability::util::card_matches_group_str(card_db, *cid, Some(g))
                        });
                        if !group_ok {
                            continue;
                        }
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
            // Turn-scoped attributed fallback: temporal conditions
            // (「このターン…アクティブにしていた場合」, Q203) look back across the
            // WHOLE turn — including activations by earlier main-phase effects —
            // and can filter on the SOURCE effect's group (『虹ヶ咲』のカードの効果)
            // and the TARGET kind (energy vs member).
            if condition.get_temporal() == Some("this_turn") {
                let card_db = &self.game_state.card_database;
                let want_kind = condition.get_card_type().map(|ct| ct.as_str().to_string());
                // The target card must belong to the conditioned player.
                let mut owned: HashSet<i16> = target_player.stage.stage.iter().copied().collect();
                owned.extend(target_player.energy_zone.cards.iter().copied());
                owned.extend(target_player.hand.cards.iter().copied());
                let groups = condition.get_group_names();
                let groups_nonempty = groups.is_some_and(|g| !g.is_empty());
                for (source, cid, cfrom, cto) in &self.game_state.turn_state_changes {
                    if cfrom != from || cto != to {
                        continue;
                    }
                    if !owned.contains(cid) {
                        continue;
                    }
                    if let Some(want) = &want_kind {
                        let matches_kind = card_db
                            .get_card(*cid)
                            .is_some_and(|c| c.card_type.as_card_str() == want.as_str());
                        if !matches_kind {
                            continue;
                        }
                    }
                    if groups_nonempty {
                        let src_ok = *source >= 0
                            && groups.unwrap().iter().any(|g| {
                                crate::ability::util::card_matches_group_str(
                                    card_db,
                                    *source,
                                    Some(g),
                                )
                            });
                        if !src_ok {
                            continue;
                        }
                    }
                    log::debug!(
                        "[STATE_CHANGE_COND] card={} {}→{} via source={} matches (turn_state_changes)",
                        cid,
                        cfrom,
                        cto,
                        source
                    );
                    return true;
                }
                log::debug!("[STATE_CHANGE_COND] no matching transition in turn_state_changes");
            }
            return false;
        }
        true
    }

    pub(crate) fn evaluate_opponent_choice_condition(&self, _condition: &Condition) -> bool {
        // Was `self.game_state.opponent_choice_declined`, a field that was
        // initialised `false` and never written, so this reduced to
        // `!negation` for every input. Kept as a named constant rather than
        // deleted because the condition type is still routed here; if the
        // mechanic is ever implemented, this is the one place that has to
        // change. Do not reintroduce a stored flag for it — derive from the
        // choice state that would set it.
        let negation = _condition.get_negation().unwrap_or(false);
        !negation
    }

    pub(crate) fn evaluate_opponent_live_success_condition(&self, condition: &Condition) -> bool {
        // Owner-relative: 「相手」 is the opponent OF THE ACTIVATING CARD'S
        // OWNER, not the fixed second attacker. A P2-owned card's opponent is
        // P1 (the first attacker), which the legacy attacker-order globals
        // could never express.
        let owner_is_p1 = self
            .activating_card_id
            .map(|cid| {
                self.game_state.player1.contains_card(cid)
                    || !self.game_state.player2.contains_card(cid)
            })
            .unwrap_or(true);
        let (opp_won, opp_no_excess) = if owner_is_p1 {
            (
                self.game_state.p2_live_success_this_turn,
                self.game_state.p2_live_success_no_excess,
            )
        } else {
            (
                self.game_state.p1_live_success_this_turn,
                self.game_state.p1_live_success_no_excess,
            )
        };
        log::debug!(
            "[OPP_LIVE_SUCCESS_EVAL] activating={:?} owner_is_p1={} opp_won={} opp_no_excess={}",
            self.activating_card_id,
            owner_is_p1,
            opp_won,
            opp_no_excess
        );
        if !opp_won {
            return false;
        }
        if condition.get_no_excess_heart().unwrap_or(false) {
            return opp_no_excess;
        }
        true
    }

    pub(crate) fn evaluate_no_excess_heart_condition(&self, condition: &Condition) -> bool {
        let target = condition.get_target().unwrap_or("self");
        self.no_excess_heart_flag(target)
    }

    pub(crate) fn evaluate_complex_condition(&self, condition: &Condition) -> bool {
        if let Some(cause) = condition.get_cause() {
            if !self.evaluate_condition(cause) {
                return false;
            }
        }
        // Check the effect sub-condition if present.
        if let Some(effect) = condition.get_effect() {
            // The effect stores a condition-like AbilityEffect. Evaluate it
            // via evaluate_effect_condition which checks negated card presence.
            if effect.conditional_negation().unwrap_or(false) {
                if let Some(card_type) = &effect.card_type_any() {
                    let loc_binding = effect.location_any();
                    let loc = loc_binding.unwrap_or("hand");
                    let zone = crate::ability::enums::Zone::from_str(loc);
                    let target = effect.target_name();
                    let player = self.resolve_condition_player(target);
                    let cards = util::zone_cards(player, loc);
                    let matching = match zone {
                        Some(crate::ability::enums::Zone::RevealedCards) => {
                            // Same 「自分の」 scope as the card-side helpers: the
                            // pool holds both performers' reveals, tagged by seat.
                            self.revealed_cards_scoped_to(Some(target))
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
                                    if *card_type == &crate::card::CardType::Live {
                                        c.is_live()
                                    } else if *card_type == &crate::card::CardType::Member {
                                        c.is_member()
                                    } else {
                                        true
                                    }
                                })
                        })
                        .count()
                        .u8_count();
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
