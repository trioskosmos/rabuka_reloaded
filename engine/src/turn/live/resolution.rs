use crate::ability::debug::ABILITY_DEBUG;
use crate::ability::enums::Zone;
use crate::card::{CardDatabase, HeartColor};
use crate::core::game_modifiers::ModifierEntry;
use crate::game_state::GameState;
use crate::turn::TurnEngine;
use crate::types::{
    AdjustmentType, LivePerformanceData,
    MemberContribution,
};
use crate::HashMap;
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::Ordering;

use super::scoring::EMPTY_H8;
use super::cheer::record_pretrigger_live_results;
use super::fmt_player_id;
impl TurnEngine {
    fn drain_pending_live_success_choices(
        game_state: &mut GameState,
        p1_id: &str,
        p2_id: &str,
    ) -> bool {
        game_state.process_pending_auto_abilities(p1_id);
        if game_state.has_pending_choice() {
            log::debug!(
                "[LIVE] pending choice after draining p1 live_success queue — early return"
            );
            return true;
        }
        game_state.process_pending_auto_abilities(p2_id);
        if game_state.has_pending_choice() {
            log::debug!(
                "[LIVE] pending choice after draining p2 live_success queue — early return"
            );
            return true;
        }
        false
    }

    fn compute_pregame_scores(
        game_state: &GameState,
        need_heart_flat: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        pre_score_flat: &HashMap<i16, i16>,
        p1_extra: u8,
        p2_extra: u8,
    ) -> (u8, u8) {
        let p1 = game_state.player1.live_card_zone.calculate_live_score(
            &game_state.card_database,
            game_state.player1_cheer_blade_heart_count,
            game_state.player1.stage_hearts.as_ref(),
            Some(need_heart_flat),
            Some(pre_score_flat),
            game_state.mods.p1_constant_total_score_bonus,
        ) + p1_extra;
        let p2 = game_state.player2.live_card_zone.calculate_live_score(
            &game_state.card_database,
            game_state.player2_cheer_blade_heart_count,
            game_state.player2.stage_hearts.as_ref(),
            Some(need_heart_flat),
            Some(pre_score_flat),
            game_state.mods.p2_constant_total_score_bonus,
        ) + p2_extra;
        log::debug!(
            "[LIVE_SCORE] p1={} p2={} extras p1={} p2={}",
            p1,
            p2,
            p1_extra,
            p2_extra
        );
        (p1, p2)
    }

    fn determine_winners(
        game_state: &GameState,
        p1_id: &str,
        p2_id: &str,
        p1_score: u8,
        p2_score: u8,
    ) -> (bool, bool) {
        let p1_has = !game_state.player1.live_card_zone.cards.is_empty();
        let p2_has = !game_state.player2.live_card_zone.cards.is_empty();
        let p1_all = p1_has
            && game_state
                .performance_snapshots
                .iter()
                .rev()
                .find(|s| s.player_id == p1_id)
                .is_some_and(|s| !s.lives.is_empty() && s.lives.iter().all(|l| l.passed));
        let p2_all = p2_has
            && game_state
                .performance_snapshots
                .iter()
                .rev()
                .find(|s| s.player_id == p2_id)
                .is_some_and(|s| !s.lives.is_empty() && s.lives.iter().all(|l| l.passed));
        if ABILITY_DEBUG.load(Ordering::Relaxed) {
            log::debug!("[LIVE-DBG] === VICTORY DETERMINATION ===");
            log::debug!(
                "[LIVE-DBG] P1 score={} has={} all_passed={} zone={:?}",
                p1_score,
                p1_has,
                p1_all,
                game_state.player1.live_card_zone.cards
            );
            log::debug!(
                "[LIVE-DBG] P2 score={} has={} all_passed={} zone={:?}",
                p2_score,
                p2_has,
                p2_all,
                game_state.player2.live_card_zone.cards
            );
        }
        let res = if !p1_all && !p2_all {
            (false, false)
        } else if p1_all && !p2_all {
            (true, false)
        } else if !p1_all && p2_all {
            (false, true)
        } else if p1_score > p2_score {
            (true, false)
        } else if p2_score > p1_score {
            (false, true)
        } else {
            (true, true)
        };
        log::debug!(
            "[LIVE_WINNERS] p1_won={} p2_won={} p1_all={} p2_all={}",
            res.0,
            res.1,
            p1_all,
            p2_all
        );
        res
    }

    /// Pass 1: populate snap.lives[].passed / required / filled / adjustments.
    /// Extracted from execute_live_victory_determination to keep that function
    /// focused on orchestration (B1). Logic unchanged — A4 pipeline helpers used inside.
    fn populate_live_verdicts(game_state: &mut GameState) {
        for snap in game_state.performance_snapshots.iter_mut() {
            if ABILITY_DEBUG.load(Ordering::Relaxed) {
                log::debug!(
                    "[LIVE-DBG] === PASS/FAIL CHECK player={} lives={} total_hearts={:?} ===",
                    snap.player_id,
                    snap.lives.len(),
                    snap.total_hearts
                );
            }
            for i in 0..snap.lives.len() {
                let lc_id = snap.lives[i].card_id;
                // Borrowed, not cloned: everything below only reads, and the
                // borrow of `game_state` is immutable, so it does not conflict
                // with the `snap` writes. `.cloned()` copied the whole Card
                // (328 B, up to ten heap allocations) for every live card in
                // every performance-phase snapshot.
                let Some(card) = game_state.card_database.get_card(lc_id) else {
                    continue;
                };
                snap.lives[i].card_id = lc_id;
                // `card_no` is the card's printed number — immutable,
                // and already stamped onto every live card by
                // `build_snapshot` when the performance snapshot was
                // first pushed. This pass runs once per turn but
                // walks EVERY snapshot accumulated so far, so an
                // unconditional re-`ArcStr::from` here re-allocated
                // the same handle for every prior performance on
                // every turn rollover (O(turns²) allocations). The
                // value can only ever be the one `build_snapshot`
                // already wrote, so fill it only when a snapshot
                // somehow arrived without it.
                if snap.lives[i].card_no.is_empty() {
                    snap.lives[i].card_no =
                        crate::types::ArcStr::from(card.card_no.as_ref());
                }
                if card.need_heart.is_none() {
                    snap.lives[i].passed = true;
                    let base_score = card.get_score();
                    let set_score = game_state.mods.get_score_set_modifier(lc_id);
                    let additive = game_state.mods.get_score_modifier(lc_id) - set_score;
                    // A non-zero set replaces the printed score outright, so
                    // the two are selected, never added. `overridden_stat`
                    // because the set half is a modifier, not a printed u8.
                    let effective = if set_score != 0 {
                        crate::constants::overridden_stat(set_score, additive)
                    } else {
                        crate::constants::effective_stat(base_score, additive)
                    };
                    snap.lives[i].score = effective;
                    continue;
                }
                let nh = card.need_heart.as_ref().unwrap();
                let mut filled = EMPTY_H8;
                for alloc in &snap.breakdown.allocations {
                    if alloc.target_idx == u8::try_from(i).unwrap() {
                        filled[alloc.color as usize] += alloc.amount;
                    }
                }
                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    log::debug!(
                        "[LIVE-DBG] live[{}] card={} filled_from_allocs={:?}",
                        i,
                        card.card_no,
                        filled
                    );
                }
                // Use stats_pipeline::effective_need_heart for required array
                let eff = crate::core::stats_pipeline::effective_need_heart(
                    Some(nh),
                    lc_id,
                    &game_state.mods.need_heart_modifiers,
                )
                .unwrap_or_else(|| nh.clone());
                let mut required_arr = EMPTY_H8;
                for (color, needed) in &eff.hearts {
                    required_arr[color.index()] = *needed;
                }
                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    log::debug!(
                        "[LIVE-DBG] live[{}] required (via pipeline)={:?}",
                        i,
                        required_arr
                    );
                }
                let passed = {
                    let mut icon_all = filled[7];
                    let mut ok = true;
                    let total_filled: u8 = filled.iter().sum();
                    let total_required: u8 = required_arr.iter().sum();
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!(
                            "[LIVE-DBG] live[{}] total_filled={} total_required={} icon_all={}",
                            i,
                            total_filled,
                            total_required,
                            icon_all
                        );
                    }
                    if total_filled < total_required {
                        ok = false;
                    }
                    if ok && required_arr[0] > 0 {
                        let any_hearts: u8 = filled[1..7].iter().sum::<u8>() + filled[0];
                        if any_hearts + icon_all < required_arr[0] {
                            ok = false;
                        } else {
                            let used = required_arr[0].saturating_sub(any_hearts);
                            icon_all = icon_all.saturating_sub(used);
                        }
                    }
                    if ok {
                        for (idx, (&f, &r)) in filled[1..7]
                            .iter()
                            .zip(required_arr[1..7].iter())
                            .enumerate()
                        {
                            if f < r {
                                let actual_idx = idx + 1;
                                let deficit = required_arr[actual_idx] - filled[actual_idx];
                                if icon_all >= deficit {
                                    icon_all -= deficit;
                                } else {
                                    ok = false;
                                    break;
                                }
                            }
                        }
                    }
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!(
                            "[LIVE-DBG] live[{}] VERDICT: passed={} filled={:?} required={:?}",
                            i,
                            ok,
                            filled,
                            required_arr
                        );
                    }
                    ok
                };
                let mut adjustments = Vec::new();
                if let Some(color_mods) = game_state.mods.need_heart_modifiers.get(&lc_id) {
                    let verbose = ABILITY_DEBUG.load(Ordering::Relaxed);
                    let req_source = verbose.then(|| format!("{} req modifier", card.name));
                    for (color, entry) in color_mods {
                        let total = entry.total();
                        if total != 0 {
                            let color_label = color.to_string();
                            adjustments.push(crate::types::Adjustment {
                                adjustment_type: AdjustmentType::Requirement,
                                desc: if verbose {
                                    format!(
                                        "{} {}",
                                        if entry.set != 0 {
                                            "="
                                        } else if total > 0 {
                                            "+"
                                        } else {
                                            ""
                                        },
                                        total,
                                    )
                                } else {
                                    String::new()
                                },
                                // `total` is already the i16 `Adjustment.value`
                                // type; the `try_from` only existed to undo an
                                // i32 widening that no longer happens.
                                value: total,
                                color: u8::try_from(color.index()).unwrap(),
                                source: if verbose {
                                    format!("{} req modifier ({})", card.name, color_label)
                                } else {
                                    String::new()
                                },
                            });
                            let op_str = if verbose && entry.set != 0 {
                                format!("= {}", entry.set)
                            } else if verbose && entry.additive > 0 {
                                format!("+{}", entry.additive)
                            } else if verbose {
                                format!("{}", entry.additive)
                            } else {
                                String::new()
                            };
                            let req_desc = if verbose {
                                format!("Requirement {}", op_str)
                            } else {
                                String::new()
                            };
                            snap.breakdown.requirements.push(crate::types::EffectEntry {
                                source: req_source.clone().unwrap_or_default(),
                                value: op_str,
                                desc: req_desc,
                            });
                        }
                    }
                }
                snap.lives[i].adjustments = adjustments;
                snap.lives[i].required = required_arr;
                snap.lives[i].filled = filled;
                snap.lives[i].passed = passed;
                let base_score = card.get_score();
                let set_score = game_state.mods.get_score_set_modifier(lc_id);
                let additive = game_state.mods.get_score_modifier(lc_id) - set_score;
                let effective = if set_score != 0 {
                    crate::constants::overridden_stat(set_score, additive)
                } else {
                    crate::constants::effective_stat(base_score, additive)
                };
                snap.lives[i].score = effective;
            }
        }
    }

    fn finalize_snapshot_fields(
        game_state: &mut GameState,
        p1_won: bool,
        p2_won: bool,
        p1_score: u8,
        p2_score: u8,
        p1_id: &str,
    ) {
        for snap in game_state.performance_snapshots.iter_mut() {
            let mut cumulative_used = EMPTY_H8;
            for i in 0..snap.lives.len() {
                for alloc in &snap.breakdown.allocations {
                    if alloc.target_idx == u8::try_from(i).unwrap() {
                        let source_idx = match alloc.phase {
                            crate::types::AllocPhase::H00Wild
                            | crate::types::AllocPhase::Wildcard => 0,
                            crate::types::AllocPhase::AllWild
                            | crate::types::AllocPhase::CAll
                            | crate::types::AllocPhase::AllCleanup => 7,
                            _ => alloc.color as usize,
                        };
                        cumulative_used[source_idx] += alloc.amount;
                    }
                }
                let mut spare = EMPTY_H8;
                for (idx, (&cum, &tot)) in cumulative_used
                    .iter()
                    .zip(snap.total_hearts.iter())
                    .enumerate()
                {
                    spare[idx] = tot.saturating_sub(cum);
                }
                snap.lives[i].spare = spare;
            }
            let is_first = snap.player_id == p1_id;
            snap.p0_wins = p1_won;
            snap.p1_wins = p2_won;
            snap.total_score = if is_first { p1_score } else { p2_score };
            let zone_empty = if is_first {
                game_state.player1.live_card_zone.cards.is_empty()
            } else {
                game_state.player2.live_card_zone.cards.is_empty()
            };
            if zone_empty {
                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    log::debug!(
                        "[LIVE-DBG] player={} live_card_zone empty → total_score forced to 0",
                        snap.player_id
                    );
                }
                snap.total_score = 0;
            }
            snap.success = snap.lives.iter().all(|l| l.passed) && snap.total_score > 0;
            if ABILITY_DEBUG.load(Ordering::Relaxed) {
                log::debug!(
                    "[LIVE-DBG] player={} SUCCESS={} total_score={} all_passed={}",
                    snap.player_id,
                    snap.success,
                    snap.total_score,
                    snap.lives.iter().all(|l| l.passed)
                );
            }
            snap.base_score_total = snap
                .lives
                .iter()
                .filter(|l| l.passed)
                .map(|l| l.score)
                .sum();
            snap.card_bonus_total = snap
                .lives
                .iter()
                .filter(|l| l.passed)
                .map(|l| l.score.saturating_sub(l.base_score))
                .sum();
            for mc in &mut snap.member_contributions {
                let mut ability_per_color = [0u8; 8];
                for ab in &mc.ability_heart_bonuses {
                    if let Some(color_idx) = ab.color {
                        if color_idx < 8 {
                            ability_per_color[color_idx as usize] += ab.amount;
                        }
                    }
                }
                for (i, (&bh, &apc)) in mc
                    .bonus_hearts
                    .iter()
                    .zip(ability_per_color.iter())
                    .enumerate()
                {
                    mc.transform_delta[i] = bh.saturating_sub(apc);
                }
            }
        }
        log::debug!(
            "[FINALIZE_SNAPSHOT] p1_won={} p2_won={} p1_score={} p2_score={}",
            p1_won,
            p2_won,
            p1_score,
            p2_score
        );
    }

    fn revert_live_success_score_modifiers(
        game_state: &mut GameState,
        pre_score_flat: &HashMap<i16, i16>,
    ) {
        let post: HashMap<i16, i16> = game_state
            .mods
            .score_modifiers
            .iter()
            .map(|(&k, e)| (k, e.total()))
            .collect();
        for (&cid, post_total) in &post {
            let pre = pre_score_flat.get(&cid).copied().unwrap_or(0);
            // `saturating_sub`/`saturating_neg` rather than `-`: both
            // operands are a card's running score modifier, and a wrap would
            // produce a small delta that then looks like a legitimate small
            // correction. Saturating keeps the direction of any overflow.
            let delta = post_total.saturating_sub(pre);
            if delta != 0 {
                game_state.mods.add_score_modifier(cid, delta.saturating_neg());
            }
        }
        for (&cid, &pre_total) in pre_score_flat {
            if !post.contains_key(&cid) {
                // Restores a card whose modifier entry was dropped entirely.
                // NOTE: this writes the *total* into the `set` half of the
                // entry, so the value is later read back as a set-override
                // rather than as an additive delta. That is pre-existing
                // behaviour and is preserved exactly here; it is correct only
                // because the entry's `additive` is zero at this point.
                game_state.mods.set_score_modifier(cid, pre_total);
            }
        }
        log::debug!(
            "[REVERT_SCORE] reverted {} late score modifiers",
            post.len()
        );
    }

    fn process_delayed_gained_effects(game_state: &mut GameState) {
        if game_state.delayed_gained_effects.is_empty() {
            return;
        }
        let saved_revealed = core::mem::take(&mut game_state.revealed_cards);
        let saved_revealed_meta = core::mem::take(&mut game_state.revealed_card_meta);
        let saved_activating_card = game_state.activating_card;
        let saved_queue = core::mem::take(&mut game_state.ability_queue);
        let delayed = core::mem::take(&mut game_state.delayed_gained_effects);
        for (card_id, gained) in &delayed {
            let Some(owner) = game_state.owner_of_card(*card_id) else {
                continue;
            };
            let Some(snap) = game_state
                .performance_snapshots
                .iter()
                .rev()
                .find(|snap| snap.player_id == owner.id && snap.turn == game_state.turn_number)
            else {
                continue;
            };
            if owner.live_card_zone.cards.is_empty()
                || snap.lives.is_empty()
                || !snap.lives.iter().all(|live| live.passed)
            {
                log::debug!(
                    "[DELAYED_GAINED] source={} owner={} skipped unsuccessful performance",
                    card_id,
                    owner.id
                );
                continue;
            }
            log::debug!(
                "[DELAYED_GAINED] source={} owner={} yell_cards={}",
                card_id,
                owner.id,
                snap.yell_cards.len()
            );
            let seat = game_state.seat_index_by_id(&owner.id);
            game_state.revealed_cards = snap.yell_cards.iter().map(|yc| yc.card_id).collect();
            // Owner tags for the swapped-in snapshot: every yell card in it was
            // revealed by the snapshot's performer, so a `target=self` condition
            // still sees its own reveals. Meta stays index-aligned with the
            // temporary card list; the saved meta is restored below.
            game_state.revealed_card_meta = snap
                .yell_cards
                .iter()
                .map(|_| crate::game_state::RevealedCardMeta {
                    source: None,
                    source_name: None,
                    is_private: false,
                    owner: Some(seat),
                    reveal_type: "yell".into(),
                })
                .collect();
            game_state.activating_card = Some(*card_id);
            use crate::ability::condition::ConditionContext;
            use crate::ability::enums::ActionType;
            use crate::ability::resolver::AbilityResolver;
            let ctx = ConditionContext::new(game_state);
            if gained.action == ActionType::ConditionalAlternative {
                let alt_cond = gained.alternative_condition();
                let base_cond = gained.condition.as_ref();
                let alt_met = alt_cond.is_some_and(|c| ctx.evaluate_condition(c));
                let base_met = base_cond.is_some_and(|c| ctx.evaluate_condition(c));
                if alt_met || base_met {
                    let alt_eff = gained.alternative_effect_any();
                    let prim_eff = gained.primary_effect().map(|pe| pe.as_ref());
                    let effect_to_apply = if alt_met { alt_eff } else { prim_eff };
                    if let Some(apply) = effect_to_apply {
                        let mut resolver =
                            AbilityResolver::new(game_state.card_database.clone(), Some(*card_id));
                        resolver.session.activating_card_id = Some(*card_id);
                        let _ = resolver.execute_effect(game_state, apply);
                    }
                }
            }
        }
        game_state.revealed_cards = saved_revealed;
        game_state.revealed_card_meta = saved_revealed_meta;
        game_state.activating_card = saved_activating_card;
        game_state.ability_queue = saved_queue;
        log::debug!(
            "[DELAYED_GAINED] processed {} delayed effects",
            delayed.len()
        );
    }

    fn merge_late_score_apps(game_state: &mut GameState, p1_id: &str, p2_id: &str) {
        let late_apps = core::mem::take(&mut game_state.ability_applications);
        if late_apps.is_empty() {
            return;
        }
        let p1_cards = &game_state.player1.live_card_zone.cards;
        let p2_cards = &game_state.player2.live_card_zone.cards;
        for snap in game_state.performance_snapshots.iter_mut() {
            let player_cards = if snap.player_id == p1_id {
                p1_cards
            } else if snap.player_id == p2_id {
                p2_cards
            } else {
                continue;
            };
            for app in &late_apps {
                if (app.effect_type == crate::types::EffectType::ScoreBonus
                    || app.effect_type == crate::types::EffectType::ScoreSet)
                    && player_cards.contains(&app.target_card_id)
                {
                    snap.breakdown.scores.push(crate::types::ScoreLine {
                        source: app.ability_text.to_string(),
                        value: u8::try_from(app.amount.unsigned_abs()).unwrap(),
                    });
                }
            }
        }
        log::debug!("[LATE_SCORE] merged {} late score apps", late_apps.len());
    }

    fn compute_surplus_and_flags(
        game_state: &mut GameState,
        p1_won: bool,
        p2_won: bool,
        p1_id: &str,
        p2_id: &str,
    ) {
        let mut p2_surplus = 0u8;
        let mut p1_surplus = 0u8;
        for snap in &mut game_state.performance_snapshots {
            // Surplus only exists for successful lives (Q142: surplus means hearts exceed requirements).
            // Rule 8.3.16: failed lives have no surplus. Q259: heart calculation only at success timing.
            if !snap.success {
                snap.surplus_hearts = [0u8; 8];
                log::debug!(
                    "[SURPLUS] player={} FAILED live → surplus_hearts=0",
                    snap.player_id
                );
                continue;
            }
            let total_available: u8 = snap.total_hearts.iter().sum();
            let total_filled: u8 = snap.lives.iter().flat_map(|l| l.filled.iter()).sum();
            let surplus = total_available.saturating_sub(total_filled);
            let mut per_color_surplus = [0u8; 8];
            for (color, &total_color) in snap.total_hearts.iter().enumerate() {
                let filled_color: u8 = snap.lives.iter().map(|l| l.filled[color]).sum();
                per_color_surplus[color] = total_color.saturating_sub(filled_color);
            }
            snap.surplus_hearts = per_color_surplus;
            log::debug!(
                "[SURPLUS] player={} total_avail={} total_filled={} surplus={} per_color={:?}",
                snap.player_id,
                total_available,
                total_filled,
                surplus,
                per_color_surplus
            );
            if snap.player_id == p1_id {
                p1_surplus = surplus;
            } else if snap.player_id == p2_id {
                p2_surplus = surplus;
            }
        }
        game_state.opponent_live_surplus_count = p2_surplus;
        game_state.self_live_surplus_count = p1_surplus;
        game_state.live_surplus_ready_this_turn = true;
        if p2_won {
            game_state.set_opponent_live_success(p2_surplus == 0);
        }
        if p1_won {
            game_state.self_no_excess_heart_this_turn = p1_surplus == 0;
        }
        game_state.p1_live_success_this_turn = p1_won;
        game_state.p1_live_success_no_excess = p1_surplus == 0;
        game_state.p2_live_success_this_turn = p2_won;
        game_state.p2_live_success_no_excess = p2_surplus == 0;
        log::debug!(
            "[SURPLUS_FLAGS] p1_surplus={} p2_surplus={} p1_won={} p2_won={}",
            p1_surplus,
            p2_surplus,
            p1_won,
            p2_won
        );
    }

    fn apply_deferred_reyell(game_state: &mut GameState) {
        // Rule 8.3.13.1 corrective path
        let Some(rb) = game_state.pending_reyell_rebuild.take() else {
            return;
        };
        log::debug!(
            "[REYELL_APPLY_DEFERRED] owner={} n={} note_icons={} prev={}",
            rb.owner,
            rb.yell_cards.len(),
            rb.note_icons,
            rb.prev_note_icons
        );
        if let Some(snap) = game_state
            .performance_snapshots
            .iter_mut()
            .rev()
            .find(|s| s.player_id == rb.owner)
        {
            snap.yell_cards = rb.yell_cards.clone();
            snap.total_hearts = rb.total_hearts;
        }
        // The owner id already selects the player: one id-keyed write
        // replaces the `is_p1` branch (phases.rs shares the setter).
        game_state.set_cheer_count_by_id(&rb.owner.clone(), rb.note_icons);
        game_state.re_yell_occurred = false;
        log::debug!(
            "[REYELL_DEFERRED] applied pending rebuild for owner={}",
            rb.owner
        );
    }

    fn rebuild_stage_hearts_with_yell(game_state: &mut GameState) {
        // A1: now delegates to unified stats_pipeline::stage_hearts internally
        let mult_ref = &game_state.mods.heart_color_multiplier;
        let mut p1_stage = game_state.player1.calculate_stage_hearts(
            &game_state.card_database,
            mult_ref,
            &game_state.mods.heart_override,
            &game_state.mods.heart_modifiers,
            &game_state.mods.heart_copy,
        );
        let mut p2_stage = game_state.player2.calculate_stage_hearts(
            &game_state.card_database,
            mult_ref,
            &game_state.mods.heart_override,
            &game_state.mods.heart_modifiers,
            &game_state.mods.heart_copy,
        );
        for snap in &game_state.performance_snapshots {
            let target = if snap.player_id == game_state.player1.id {
                &mut p1_stage
            } else {
                &mut p2_stage
            };
            for yc in &snap.yell_cards {
                for (i, &count) in yc.blade_hearts.iter().enumerate() {
                    if count > 0 {
                        let color = crate::card::HeartColor::from_index(i);
                        *target.hearts.entry_or_default(color) += count;
                    }
                }
            }
        }
        if ABILITY_DEBUG.load(Ordering::Relaxed) {
            log::debug!("[LIVE-DBG] === STAGE HEARTS AFTER YELL ===");
            log::debug!("[LIVE-DBG] P1 stage_hearts={:?}", p1_stage.hearts);
            log::debug!("[LIVE-DBG] P2 stage_hearts={:?}", p2_stage.hearts);
        }
        game_state.player1.stage_hearts = Some(p1_stage);
        game_state.player2.stage_hearts = Some(p2_stage);
    }

    /// Pre-trigger score-modifier snapshots (flat maps) so LiveSuccess
    /// deltas are measured without double-counting. ONE home for the two
    /// `mods.*.iter().map(|(&k, e)| ...)` snapshots below.
    fn snapshot_score_flats(
        game_state: &GameState,
    ) -> (
        HashMap<i16, i16>,
        HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
    ) {
        // Capture pre-trigger score modifiers so we can avoid double-counting:
        // calculate_live_score gets the PRE values, and pX_extra carries only
        // the delta from LiveSuccess-triggered abilities.
        let pre_score_flat: HashMap<i16, i16> = game_state
            .mods
            .score_modifiers
            .iter()
            .map(|(&k, e)| (k, e.total()))
            .collect();
        // Determine winner — use PRE-trigger modifiers so LiveSuccess
        // triggered changes only apply via pX_extra (no double-count).
        let need_heart_flat: HashMap<i16, HashMap<HeartColor, ModifierEntry>> = game_state
            .mods
            .need_heart_modifiers
            .iter()
            .map(|(&k, colors)| {
                let flat: HashMap<HeartColor, ModifierEntry> =
                    colors.iter().map(|(&c, e)| (c, *e)).collect();
                (k, flat)
            })
            .collect();
        (pre_score_flat, need_heart_flat)
    }

    /// Per-snapshot heart surplus, computed BEFORE LiveSuccess abilities
    /// fire (allocation is already finalised). Feeds color-filtered surplus
    /// conditions (e.g. La Bella Patria heart04 >= 1, Q174).
    fn compute_snapshot_surplus(game_state: &mut GameState, player2_id: &str) {
        for snap in &mut game_state.performance_snapshots {
            let total_hearts: u8 = snap.total_hearts.iter().sum();
            // Disjoint-field borrows (same shape as the inline original):
            // compare ids without holding a whole-state borrow.
            let is_p1 = snap.player_id == game_state.player1.id;
            let player = if is_p1 {
                &game_state.player1
            } else {
                &game_state.player2
            };
            let required: u8 = player
                .live_card_zone
                .cards
                .iter()
                .filter_map(|&id| game_state.card_database.get_card(id))
                .filter_map(|c| c.need_heart.as_ref())
                .flat_map(|nh| nh.hearts.values())
                .sum();
            let surplus = total_hearts.saturating_sub(required);
            if snap.player_id == player2_id {
                game_state.opponent_live_surplus_count = surplus;
            } else {
                game_state.self_live_surplus_count = surplus;
            }
            // NOTE: snap.lives[i].filled is NOT yet populated at this point —
            // that happens later in this function. Derive filled from
            // snap.breakdown.allocations, which are already finalised.
            let mut per_color = [0u8; 8];
            {
                let mut filled_per_color = [0u8; 8];
                for alloc in &snap.breakdown.allocations {
                    if alloc.color < 8 {
                        filled_per_color[alloc.color as usize] =
                            filled_per_color[alloc.color as usize].saturating_add(alloc.amount);
                    }
                }
                for color in 0..8 {
                    per_color[color] =
                        snap.total_hearts[color].saturating_sub(filled_per_color[color]);
                }
            }
            snap.surplus_hearts = per_color;
        }
        game_state.live_surplus_ready_this_turn = true;
    }

    /// Score delta since the pre-trigger snapshot for one side's live zone.
    fn side_score_extra(
        game_state: &GameState,
        pre_score_flat: &HashMap<i16, i16>,
        live_zone: &[i16],
    ) -> u8 {
        // Wide accumulator kept deliberately: each term is a signed difference
        // and this sums one per live card, so `i32` stops a single addition
        // from being the thing that wraps. Each term is an i16 difference.
        let mut total = 0i32;
        for &cid in live_zone {
            let current = game_state
                .mods
                .score_modifiers
                .get(&cid)
                .map_or(0, |entry| entry.total());
            total += i32::from(
                current.saturating_sub(pre_score_flat.get(&cid).copied().unwrap_or(0)),
            );
        }
        crate::constants::saturate_u8(total)
    }

    /// LiveSuccess trigger orchestration across re-entries: restores saved
    /// extras when both sides resolved, otherwise fires the pending side's
    /// triggers and measures its score delta. Returns `None` when a choice
    /// pends (caller must return early).
    fn resolve_live_success_extras(
        game_state: &mut GameState,
        player1_id: &str,
        player2_id: &str,
        pre_score_flat: &HashMap<i16, i16>,
    ) -> Option<(u8, u8)> {
        if !game_state.live_success_triggered_this_turn {
            game_state.live_success_triggered_this_turn = true;
            game_state.live_success_p1_fired = false;
            game_state.live_success_p2_fired = false;
            Self::compute_snapshot_surplus(game_state, player2_id);
        }

        if !game_state.player1.is_first_attacker && !game_state.live_success_p2_fired {
            game_state.live_success_p2_fired = true;
            Self::trigger_live_success_abilities(game_state, player2_id);
            Self::trigger_auto_abilities_for_player(game_state, player2_id);
            game_state.process_pending_auto_abilities(player2_id);
            if game_state.has_pending_choice() {
                return None;
            }
            game_state.live_success_p2_extra = Self::side_score_extra(
                game_state,
                pre_score_flat,
                &game_state.player2.live_card_zone.cards,
            );
        }

        if !game_state.live_success_p1_fired {
            game_state.live_success_p1_fired = true;
            Self::trigger_live_success_abilities(game_state, player1_id);
            Self::trigger_auto_abilities_for_player(game_state, player1_id);
            game_state.process_pending_auto_abilities(player1_id);
            if game_state.has_pending_choice() {
                return None;
            }
            game_state.live_success_p1_extra = Self::side_score_extra(
                game_state,
                pre_score_flat,
                &game_state.player1.live_card_zone.cards,
            );
        }

        if !game_state.live_success_p2_fired {
            game_state.live_success_p2_fired = true;
            Self::trigger_live_success_abilities(game_state, player2_id);
            Self::trigger_auto_abilities_for_player(game_state, player2_id);
            game_state.process_pending_auto_abilities(player2_id);
            if game_state.has_pending_choice() {
                return None;
            }
            game_state.live_success_p2_extra = Self::side_score_extra(
                game_state,
                pre_score_flat,
                &game_state.player2.live_card_zone.cards,
            );
        }

        Some((game_state.live_success_p1_extra, game_state.live_success_p2_extra))
    }

    /// Debug-only per-snapshot performance lines for the rule log.
    fn log_performance_summary(game_state: &mut GameState) {
        if !ABILITY_DEBUG.load(Ordering::Relaxed) {
            return;
        }
        for snap in &game_state.performance_snapshots {
            let player = fmt_player_id(&snap.player_id);
            let mut live_details = String::new();
            for (i, live) in snap.lives.iter().enumerate() {
                let live_result = if live.passed { "PASS" } else { "FAIL" };
                if i > 0 {
                    live_details.push_str(", ");
                }
                let _ = core::fmt::Write::write_fmt(
                    &mut live_details,
                    format_args!("live score+{} → {}", live.score, live_result),
                );
            }
            let perf_result = if snap.success { "PASS" } else { "FAIL" };
            let detail_str = if live_details.is_empty() {
                String::new()
            } else {
                format!(" [{}]", live_details)
            };
            GameState::push_rule_log_to(
                &mut game_state.rule_log,
                format!(
                    "[Turn {}] {} [[log_performance:score={},result={}]]{}",
                    snap.turn, player, snap.total_score, perf_result, detail_str,
                ),
            );
        }
    }

    /// Move live cards to success and apply Rule 8.4.13 (sole mover becomes
    /// first attacker). Returns `(p1_now, p1_added, p2_now, p2_added)` for
    /// the structured summary below.
    fn move_to_success_and_update_attacker(
        game_state: &mut GameState,
        player1_won: bool,
        player2_won: bool,
    ) -> (usize, bool, usize, bool) {
        let p1_before = game_state.player1.success_live_card_zone.cards.len();
        let p2_before = game_state.player2.success_live_card_zone.cards.len();
        log::debug!(
            "[MULTI_DEBUG] About to call move_live_to_success p1_won={} p2_won={}",
            player1_won,
            player2_won
        );
        Self::move_live_to_success_and_handle_wins(game_state, player1_won, player2_won);

        // Rule 8.4.13: If only one player moved a card to success this live, they become first attacker
        let p1_now = game_state.player1.success_live_card_zone.cards.len();
        let p2_now = game_state.player2.success_live_card_zone.cards.len();
        let p1_added = p1_now > p1_before;
        let p2_added = p2_now > p2_before;
        if p1_added && !p2_added {
            game_state.player1.is_first_attacker = true;
            game_state.player2.is_first_attacker = false;
        } else if p2_added && !p1_added {
            game_state.player1.is_first_attacker = false;
            game_state.player2.is_first_attacker = true;
        }
        (p1_now, p1_added, p2_now, p2_added)
    }

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        Self::apply_deferred_reyell(game_state);

        game_state.evaluate_success_zone_constant_abilities();
        game_state.restore_performance_need_heart_modifiers();

        Self::rebuild_stage_hearts_with_yell(game_state);

        let player1_id = game_state.player1.id.clone();
        let player2_id = game_state.player2.id.clone();

        let (pre_score_flat, need_heart_flat) = Self::snapshot_score_flats(game_state);

        // BUG#5: record per-seat outcomes BEFORE LiveSuccess triggers fire
        // (Q36: they resolve at determination timing and must see both
        // performances' results).
        record_pretrigger_live_results(game_state, &need_heart_flat, &pre_score_flat);

        // Q48: A live can be won even with total score 0 or less
        // (score comparison determines the winner regardless of absolute value).
        let (p1_extra, p2_extra) = match Self::resolve_live_success_extras(
            game_state,
            &player1_id,
            &player2_id,
            &pre_score_flat,
        ) {
            Some(extras) => extras,
            None => return,
        };

        // Drain any remaining LiveSuccess auto-abilities (choice-gated re-entries)
        if Self::drain_pending_live_success_choices(game_state, &player1_id, &player2_id) {
            return;
        }

        Self::populate_live_verdicts(game_state);
        // NOTE: populate_live_verdicts must run before victory determination so
        // snap.lives[i].passed is populated (Q47/Q48). Removed duplicated inline
        // pass/fail block here — the extracted helper above is now the single source.
        let (player1_score, player2_score) = {
            // Q36: ライブ成功時 delayed gains (e.g. Mari bp2-008R＋ gained
            // ライブの合計スコア) resolve in the victory-determination phase
            // BEFORE the winner is decided, so they must land in the owner
            // accumulator before compute_pregame_scores reads it. The success
            // gate inside uses the pass/fail verdicts populated directly above.
            Self::process_delayed_gained_effects(game_state);
            Self::compute_pregame_scores(
                game_state,
                &need_heart_flat,
                &pre_score_flat,
                p1_extra,
                p2_extra,
            )
        };

        let (player1_won, player2_won) = Self::determine_winners(
            game_state,
            &player1_id,
            &player2_id,
            player1_score,
            player2_score,
        );

        Self::finalize_snapshot_fields(
            game_state,
            player1_won,
            player2_won,
            player1_score,
            player2_score,
            &player1_id,
        );
        Self::revert_live_success_score_modifiers(game_state, &pre_score_flat);
        log::debug!(
            "[LIVE_TOTAL_TIMING] post-finalize accum p1={} p2={} snap_totals={:?}",
            game_state.mods.p1_constant_total_score_bonus,
            game_state.mods.p2_constant_total_score_bonus,
            game_state
                .performance_snapshots
                .iter()
                .map(|s| (s.player_id.clone(), s.total_score))
                .collect::<Vec<_>>()
        );
        Self::merge_late_score_apps(game_state, &player1_id, &player2_id);
        Self::compute_surplus_and_flags(
            game_state,
            player1_won,
            player2_won,
            &player1_id,
            &player2_id,
        );

        // Push performance summary to rule log
        Self::log_performance_summary(game_state);

        // NOTE: cannot-place restrictions (メビウスループ PL!S-pb1-022-L) are NOT
        // pre-discarded here. Their conditions (「合計スコアが同じ場合」) are
        // evaluated when the ライブ成功時 ability resolves: on a tie it pushes a
        // dynamic prohibition_effects entry and move_live_to_success_and_
        // handle_wins routes the card to the waitroom via can_place_card_in_zone;
        // untied, no prohibition exists and the winner places normally.
        let (p1_now, p1_added, p2_now, p2_added) =
            Self::move_to_success_and_update_attacker(game_state, player1_won, player2_won);

        // ── Structured live-check summary (one line: verdict + both sides).
        // Only the engine knows pass/fail/scores — emit them here so replays
        // and benchmark logs are readable WITHOUT per-action tracing.
        {
            let summarize = |gs: &crate::game_state::GameState, pid: &str| -> String {
                let Some(snap) = gs
                    .performance_snapshots
                    .iter()
                    .rev()
                    .find(|s| s.player_id == pid)
                else {
                    return "no-performance".to_string();
                };
                let mut parts: Vec<String> = Vec::new();
                for l in &snap.lives {
                    let name = gs
                        .card_database
                        .get_card(l.card_id)
                        .map(|c| c.name.to_string())
                        .unwrap_or_else(|| l.card_no.to_string());
                    if l.passed {
                        parts.push(format!("{}:{}pt", name, l.score));
                    } else {
                        let deficit: Vec<String> = (0..8)
                            .filter(|&i| l.filled[i] < l.required[i])
                            .map(|i| format!("c{} {}/{}", i, l.filled[i], l.required[i]))
                            .collect();
                        parts.push(format!("{}:FAIL({})", name, deficit.join(",")));
                    }
                }
                let hits: usize = snap
                    .yell_cards
                    .iter()
                    .map(|y| y.blade_hearts.iter().sum::<u8>() as usize)
                    .sum();
                format!(
                    "set[{}] yell {}/{}flips score={}",
                    parts.join(" "),
                    hits,
                    snap.yell_count,
                    snap.total_score
                )
            };
            // Check whether this summary is wanted BEFORE building it.
            //
            // `summarize` allocates a card-name String per live card, a
            // `format!` per live, and for every *failed* live up to eight
            // `format!`s plus a `join`. The `logging_enabled` guard used to sit
            // *below* this line, so all of that was built and then discarded
            // whenever logging was off — which is the case for the benchmark,
            // every bot rollout, and every training run.
            //
            // `summarize` only reads game state and returns a String, so
            // hoisting the guard above it is behaviour-preserving. Measured
            // 10,848,527 -> 10,794,138 allocations over a 78,036-action run
            // (139.02 -> 138.33 per action), deterministically.
            if !crate::core::runtime_flags::logging_enabled() {
                return;
            }
            let p1_sum = summarize(game_state, &player1_id);
            let p2_sum = summarize(game_state, &player2_id);
            let verdict = match (player1_won, player2_won) {
                (true, true) => "TIE-both-place",
                (true, false) => "P1-WINS",
                (false, true) => "P2-WINS",
                _ => "NO-CONTEST",
            };
            game_state.push_structured_log(crate::types::LogEntry {
                text: format!(
                    "LIVE {} | P1 {} → succ={}(+{}) | P2 {} → succ={}(+{})",
                    verdict, p1_sum, p1_now, p1_added as u8, p2_sum, p2_now, p2_added as u8
                ),
                turn: game_state.turn_number,
                player_label: "SYSTEM".into(),
                source_card_id: None,
                source_card_name: None,
                category: "live_result".into(),
                metadata: None,
            });
        }
    }

    fn process_player_live_result(
        player: &mut crate::player::Player,
        won: bool,
        must_skip: bool,
        can_place: bool,
    ) -> Vec<i16> {
        let card_count = player.live_card_zone.cards.len();
        let mut moved_to_waitroom = Vec::new();
        if won && !must_skip && card_count > 0 {
            let card_id = player.live_card_zone.cards.remove(0);
            if can_place {
                player.success_live_card_zone.cards.push(card_id);
            } else {
                player.waitroom.cards.push(card_id);
                moved_to_waitroom.push(card_id);
            }
        }
        let remaining = core::mem::take(&mut player.live_card_zone.cards);
        moved_to_waitroom.extend_from_slice(&remaining);
        player.waitroom.cards.extend(remaining);
        moved_to_waitroom
    }

    fn build_success_zone_choice(
        game_state: &GameState,
        won: bool,
        must_skip: bool,
        cards_count: usize,
        cards: &[i16],
        player_id: &str,
    ) -> Option<crate::ability::types::Choice> {
        if !won || must_skip || cards_count <= 1 {
            return None;
        }
        let can_place = cards.iter().any(|&cid| {
            game_state.can_place_card_in_zone(cid, Zone::SuccessLiveZone.to_str(), player_id)
        });
        if !can_place {
            return None;
        }
        let options: Vec<crate::ability::types::LiveSuccessOption> = cards
            .iter()
            .enumerate()
            .map(|(i, &cid)| crate::ability::types::LiveSuccessOption {
                card_name: game_state
                    .card_database
                    .get_card(cid)
                    .map(|c| c.name.to_string())
                    .unwrap_or_default(),
                card_index: i,
            })
            .collect();
        Some(crate::ability::types::Choice::SelectLiveSuccess {
            player_id: player_id.to_string(),
            count: 1,
            options,
            description: "Choose which live card goes to your success zone".to_string(),
            description_en: Some("Choose which live card goes to your success zone".to_string()),
            description_ja: Some("ライブエリアからサクセスゾーンに送るカードを選択".to_string()),
        })
    }

    fn pause_success_zone_choice(
        game_state: &mut GameState,
        choice: crate::ability::types::Choice,
        player_id: &str,
    ) {
        game_state.ability_queue.pause_for_choice(choice);
        if let Some(entry) = game_state.ability_queue.current_entry_mut() {
            entry.choice_player_id = Some(crate::core::player::PlayerId::from(player_id));
        }
    }

    pub fn move_live_to_success_and_handle_wins(
        game_state: &mut GameState,
        player1_won: bool,
        player2_won: bool,
    ) {
        let p1_id = game_state.player1.id.clone();
        let p2_id = game_state.player2.id.clone();
        let p1_cards = game_state.player1.live_card_zone.cards.len();
        let p2_cards = game_state.player2.live_card_zone.cards.len();
        let p1_successes = game_state.player1.success_live_card_zone.cards.len();
        let p2_successes = game_state.player2.success_live_card_zone.cards.len();
        let p1_must_skip = player1_won && player2_won && p1_successes >= 2;
        let p2_must_skip = player1_won && player2_won && p2_successes >= 2;

        log::debug!(
            "[MULTI_LIVE] p1_won={} p2_won={} p1_cards={} p2_cards={} p1_successes={} p2_successes={} p1_must={} p2_must={}",
            player1_won,
            player2_won,
            p1_cards,
            p2_cards,
            p1_successes,
            p2_successes,
            p1_must_skip,
            p2_must_skip
        );
        if let Some(choice) = Self::build_success_zone_choice(
            game_state,
            player1_won,
            p1_must_skip,
            p1_cards,
            &game_state.player1.live_card_zone.cards,
            &p1_id,
        ) {
            Self::pause_success_zone_choice(game_state, choice, &p1_id);
            return;
        }
        if let Some(choice) = Self::build_success_zone_choice(
            game_state,
            player2_won,
            p2_must_skip,
            p2_cards,
            &game_state.player2.live_card_zone.cards,
            &p2_id,
        ) {
            Self::pause_success_zone_choice(game_state, choice, &p2_id);
            return;
        }

        // Single card or no choice needed — process normally
        let p1_top = game_state.player1.live_card_zone.cards.last().copied();
        let p2_top = game_state.player2.live_card_zone.cards.last().copied();
        let p1_can_place = p1_top.is_some_and(|cid| {
            game_state.can_place_card_in_zone(cid, Zone::SuccessLiveZone.to_str(), &p1_id)
        });
        let p2_can_place = p2_top.is_some_and(|cid| {
            game_state.can_place_card_in_zone(cid, Zone::SuccessLiveZone.to_str(), &p2_id)
        });

        for (won, must_skip, cards_count, top, player_id) in [
            (player1_won, p1_must_skip, p1_cards, p1_top, &p1_id),
            (player2_won, p2_must_skip, p2_cards, p2_top, &p2_id),
        ] {
            if won && !must_skip && cards_count == 1 {
                if let Some(card_id) = top {
                    if Self::try_create_success_replacement_choice(game_state, card_id, player_id) {
                        return;
                    }
                }
            }
        }

        let mut moved_to_waitroom = Self::process_player_live_result(
            &mut game_state.player1,
            player1_won,
            p1_must_skip,
            p1_can_place,
        );
        moved_to_waitroom.extend(Self::process_player_live_result(
            &mut game_state.player2,
            player2_won,
            p2_must_skip,
            p2_can_place,
        ));

        if !moved_to_waitroom.is_empty() {
            game_state.set_recently_moved_batch(moved_to_waitroom.into(), Some("live_card_zone"));

            // Scan and queue triggers for both players
            Self::trigger_auto_abilities_for_player(game_state, &p1_id);
            Self::trigger_auto_abilities_for_player(game_state, &p2_id);

            game_state.process_pending_auto_abilities(&p1_id);
            game_state.process_pending_auto_abilities(&p2_id);
        }
    }

    /// Check if a card has a success zone replacement ability (常時 + conditional_alternative).
    /// Returns the group names from the effect if found.
    pub(crate) fn get_success_replacement_info(
        game_state: &GameState,
        card_id: i16,
    ) -> Option<Vec<String>> {
        let card = game_state.card_database.get_card(card_id)?;
        for ar in &card.abilities {
            let ability = ar.resolve();
            let is_constant = ability.has_trigger(crate::triggers::TriggerKind::Constant);
            if !is_constant {
                continue;
            }
            let effect = match &ability.effect {
                Some(e) => e,
                None => continue,
            };
            if effect.action != crate::ability::enums::ActionType::ConditionalAlternative {
                continue;
            }
            let cond_matches = effect.condition.as_ref().is_some_and(|c| {
                c.get_location()
                    .is_some_and(|loc| Zone::from_str(loc) == Some(Zone::SuccessLiveZone))
            });
            if !cond_matches {
                continue;
            }
            let alt_binding = effect.alternative_effect_any();
            let alt = match alt_binding.as_ref() {
                Some(a) => a,
                None => continue,
            };
            if alt.action != crate::ability::enums::ActionType::MoveCards {
                continue;
            }
            let alt_source = alt.source.map(|z| z.as_str()).unwrap_or("");
            if Zone::from_str(alt_source) != Some(Zone::Discard) && alt_source != "discard" {
                continue;
            }
            let group_names = effect.group_names_any().cloned().unwrap_or_default();
            if group_names.is_empty() {
                let alt_groups = alt.group_names_any().cloned().unwrap_or_default();
                if !alt_groups.is_empty() {
                    return Some(alt_groups);
                }
                return Some(group_names);
            }
            return Some(group_names);
        }
        None
    }

    /// Try to create a success zone replacement choice for a player's card.
    /// Returns true if a choice was created (caller should return early).
    fn try_create_success_replacement_choice(
        game_state: &mut GameState,
        card_id: i16,
        player_id: &str,
    ) -> bool {
        let group_names = match Self::get_success_replacement_info(game_state, card_id) {
            Some(gn) => gn,
            None => return false,
        };
        let player = if player_id == game_state.player1.id {
            &game_state.player1
        } else {
            &game_state.player2
        };
        let has_valid_targets = player.waitroom.cards.iter().any(|&cid| {
            game_state.card_database.get_card(cid).is_some_and(|c| {
                c.is_live()
                    && group_names.iter().any(|gn| {
                        crate::ability::util::card_matches_group_str(
                            &game_state.card_database,
                            cid,
                            Some(gn),
                        )
                    })
            })
        });
        if !has_valid_targets {
            return false;
        }
        let group_name = group_names.into_iter().next().unwrap_or_default();
        game_state.pending_success_replacement_card_id = Some(card_id);
        game_state.pending_success_replacement_player_id = Some(player_id.to_string());
        let description = "Choose a μ's live card from discard to place in your success zone (or skip to place the original card)".to_string();
        let choice = crate::ability::types::Choice::select_cards(
            crate::ability::enums::Zone::Discard.to_str(),
            1,
            description,
            true,
        )
        .description_ja(Some("控え室から成功ゾーンに置くμ'sのライブカードを選んでください（スキップで元のカードを置きます）".to_string()))
        .card_type(Some("live_card".to_string()))
        .group(Some(group_name))
        .target_player_id(Some("self".to_string()))
        .build();
        game_state.ability_queue.pause_for_choice(choice);
        if let Some(entry) = game_state.ability_queue.current_entry_mut() {
            entry.card_id = Some(card_id);
            entry.player_id = crate::core::player::PlayerId::from(player_id);
            entry.choice_player_id = Some(crate::core::player::PlayerId::from(player_id));
            if let Some(card) = game_state.card_database.get_card(card_id) {
                entry.card_no = card.card_no.clone();
            }
        }
        true
    }

    /// Handle the result of a live success zone card choice.
    pub(crate) fn handle_live_success_choice(
        game_state: &mut GameState,
        selected_index: usize,
        player_id: &str,
    ) -> Result<(), String> {
        let player = if player_id == game_state.player1.id {
            &mut game_state.player1
        } else if player_id == game_state.player2.id {
            &mut game_state.player2
        } else {
            return Err("Player not found for live success choice".to_string());
        };
        if selected_index >= player.live_card_zone.cards.len() {
            return Err("Invalid card index for live success choice".to_string());
        }
        let card_id = player.live_card_zone.cards.remove(selected_index);
        player.success_live_card_zone.cards.push(card_id);
        let remaining = core::mem::take(&mut player.live_card_zone.cards);
        player.waitroom.cards.extend(remaining);
        log::debug!(
            "[LIVE_SUCCESS_CHOICE] player={} card={} success_count={}",
            player_id,
            card_id,
            player.success_live_card_zone.cards.len()
        );
        let success_count = player.success_live_card_zone.cards.len();
        Self::check_victory_condition(game_state);
        game_state.push_structured_log(crate::types::LogEntry {
            text: format!(
                "LIVE_SUCCESS_HANDLER player={} card={} success_count={} ended={} result={:?}",
                player_id, card_id, success_count, game_state.game_ended, game_state.game_result
            ),
            turn: game_state.turn_number,
            player_label: "SYSTEM".into(),
            source_card_id: Some(card_id),
            source_card_name: None,
            category: "live_result".into(),
            metadata: None,
        });
        Ok(())
    }

    /// Q68/Rule: "cannot_live" discards live cards during performance.
    /// Returns the discarded card ids for `moved_live_card_ids`.
    pub(crate) fn discard_live_on_cannot_live(player: &mut crate::player::Player) -> Vec<i16> {
        let moved: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
        player
            .waitroom
            .cards
            .extend(player.live_card_zone.cards.drain(..));
        moved
    }

    /// A1+A2: member heart+blade layering via stats_pipeline (single source
    /// of truth). One entry per occupied stage slot.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn build_member_contributions(
        player: &crate::player::Player,
        card_db: &CardDatabase,
        blade_modifiers: &HashMap<i16, ModifierEntry>,
        heart_override: &HashMap<i16, (HeartColor, u8)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
        heart_copy: &HashMap<i16, i16>,
        orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
    ) -> Vec<MemberContribution> {
        let mut member_contributions = Vec::new();
        for i in 0..3 {
            let cid = player.stage.stage[i];
            if cid == -1 {
                continue;
            }
            let card_opt = card_db.get_card(cid);
            let mut draw_icons = 0u8;
            if let Some(card) = card_opt.as_ref() {
                if let Some(ref sh) = card.special_heart {
                    for (color, count) in &sh.hearts {
                        if *color == HeartColor::Draw {
                            draw_icons += count;
                        }
                    }
                }
            }
            // heart pipeline: base vs bonus via stats_pipeline
            let (base_h, bonus_h) = crate::core::stats_pipeline::member_heart_detail(
                card_db,
                cid,
                heart_override,
                heart_copy,
                heart_color_multiplier,
                heart_modifiers,
            );
            // blade pipeline: effective blade parts
            let printed_blade = card_opt.map(|c| c.blade).unwrap_or(0);
            let entry = blade_modifiers.get(&cid).copied().unwrap_or_default();
            let (base_blades, bonus_blades) =
                crate::core::stats_pipeline::effective_blade_parts(&entry, printed_blade);

            let is_wait = orientation_modifiers
                .get(&cid)
                .map(|o| *o == crate::core::game_modifiers::CardOrientation::Wait)
                .unwrap_or(false);

            member_contributions.push(MemberContribution {
                source_id: cid,
                slot: u8::try_from(i).unwrap(),
                base_hearts: base_h,
                bonus_hearts: bonus_h,
                base_blades,
                bonus_blades,
                base_notes: 0,
                bonus_notes: 0,
                draw_icons,
                ability_heart_bonuses: Vec::new(),
                ability_blade_bonuses: Vec::new(),
                card_no: card_opt
                    .map(|c| crate::types::ArcStr::from(c.card_no.as_ref()))
                    .unwrap_or_default(),
                is_wait,
                transform_delta: [0u8; 8],
            });
            log::debug!(
                "[MEMBER_CONTRIB] cid={} slot={} base_hearts={:?} bonus_hearts={:?} base_blades={} bonus_blades={}",
                cid, i, base_h, bonus_h, base_blades, bonus_blades
            );
        }
        member_contributions
    }

    /// Q32/Rule 8.3.6: empty live zone means no yell and no live card
    /// processing — but member contributions and stage hearts still apply.
    pub(crate) fn empty_live_performance(
        member_contributions: Vec<MemberContribution>,
    ) -> LivePerformanceData {
        let mut total_hearts_arr = EMPTY_H8;
        for mc in &member_contributions {
            for (c, (&base, &bonus)) in mc
                .base_hearts
                .iter()
                .zip(mc.bonus_hearts.iter())
                .enumerate()
            {
                total_hearts_arr[c] += base + bonus;
            }
        }
        let total_blade: u8 = member_contributions
            .iter()
            .filter(|m| !m.is_wait)
            .map(|m| m.base_blades + m.bonus_blades)
            .sum();
        LivePerformanceData {
            yell_count: total_blade,
            note_icons: 0,
            revealed_ids: Vec::new(),
            member_contributions,
            yell_cards: Vec::new(),
            total_hearts: total_hearts_arr,
            allocations: Vec::new(),
            heart_sources: Vec::new(),
            blade_sources: Vec::new(),
            draw_effects_occurred: false,
            live_card_ids: vec![],
            moved_live_card_ids: Vec::new(),
        }
    }

}
