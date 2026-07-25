use crate::ability::debug::ABILITY_DEBUG;
use crate::ability::enums::Zone;
use crate::card::{BaseHeart, BladeColor, CardDatabase, HeartColor, HeartMap};
use crate::core::game_modifiers::ModifierEntry;
use crate::game_state::GameState;
use crate::types::{
    AdjustmentType, AllocPhase, Allocation, ArcStr, BladeSource, HeartSource, LivePerformanceData,
    MemberContribution, SourceName, SourceType, YellCardResult,
};
use crate::{HashMap, HashSet};
#[cfg(feature = "no_std")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::Ordering;

const EMPTY_H8: [u32; 8] = [0u32; 8];

/// Effective heart need for a live card during allocation.
struct CardNeed {
    name: crate::types::ArcStr,
    need: [u32; 8],
}

impl super::TurnEngine {
    fn score_delta_since(
        current: &HashMap<i16, i32>,
        snapshot: &HashMap<i16, i32>,
        zone_cards: &[i16],
    ) -> i32 {
        let mut total = 0i32;
        for &cid in zone_cards {
            let cur = current.get(&cid).copied().unwrap_or(0);
            let prev = snapshot.get(&cid).copied().unwrap_or(0);
            total += cur - prev;
        }
        total
    }

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        // Evaluate constant modify_required_hearts abilities on cards in the
        // success_live_card_zone (e.g. PL!-bp6-022-L) before any scoring or
        // heart requirement checks.
        game_state.evaluate_success_zone_constant_abilities();

        // Restore performance-time need_heart_modifiers that were cleared by
        // evaluate_success_zone_constant_abilities. This preserves modifications
        // from live_start triggers and other non-constant sources.
        // Deduplicate (cid,color) pairs to avoid double-counting when the same
        // global modifier is captured in multiple players' snapshots.
        let mut restored: HashSet<(i16, crate::card::HeartColor)> = HashSet::default();
        for snap in &game_state.performance_snapshots {
            for &(cid, color, ref entry) in &snap.performance_need_heart_modifiers {
                if !restored.insert((cid, color)) {
                    continue;
                }
                let target = game_state
                    .mods
                    .need_heart_modifiers
                    .entry(cid)
                    .or_default()
                    .entry(color)
                    .or_insert(ModifierEntry::default());
                if entry.set != 0 && target.set == 0 {
                    target.set = entry.set;
                }
                target.additive += entry.additive;
            }
        }

        let mult_ref = &game_state.mods.heart_color_multiplier;
        let mut p1_stage = game_state.player1.calculate_stage_hearts(
            &game_state.card_database,
            mult_ref,
            &game_state.mods.heart_override,
            &game_state.mods.heart_modifiers,
        );
        let mut p2_stage = game_state.player2.calculate_stage_hearts(
            &game_state.card_database,
            mult_ref,
            &game_state.mods.heart_override,
            &game_state.mods.heart_modifiers,
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
        game_state.player1.stage_hearts = Some(p1_stage);
        game_state.player2.stage_hearts = Some(p2_stage);

        let player1_id = game_state.player1.id.clone();
        let player2_id = game_state.player2.id.clone();

        // Capture pre-trigger score modifiers so we can avoid double-counting:
        // calculate_live_score gets the PRE values, and pX_extra carries only
        // the delta from LiveSuccess-triggered abilities.
        let pre_score_flat: HashMap<i16, i32> = game_state
            .mods
            .score_modifiers
            .iter()
            .map(|(&k, e)| (k, e.total()))
            .collect();
        // Q48: A live can be won even with total score 0 or less
        // (score comparison determines the winner regardless of absolute value).
        let p1_extra: u32;
        let p2_extra: u32;
        if game_state.live_success_triggered_this_turn && game_state.live_success_p2_fired {
            // Re-entry after BOTH players' triggers already resolved.
            // Restore saved extras (e.g. if a later auto-ability creates a choice).
            p1_extra = game_state.live_success_p1_extra;
            p2_extra = game_state.live_success_p2_extra;
        } else {
            if !game_state.live_success_triggered_this_turn {
                // First entry: init state, process surplus, fire P1 triggers.
                game_state.live_success_triggered_this_turn = true;
                game_state.live_success_p2_fired = false;

                for snap in &mut game_state.performance_snapshots {
                    let total_hearts: u32 = snap.total_hearts.iter().sum();
                    let player = if snap.player_id == player1_id {
                        &game_state.player1
                    } else {
                        &game_state.player2
                    };
                    let required: u32 = player
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
                    // Compute per-color surplus NOW, before LiveSuccess abilities fire.
                    // Allocation is already finalised (performance phase completed).
                    // snap.surplus_hearts is read by color-filtered surplus conditions
                    // (e.g. La Bella Patria heart04 >= 1, Q174).
                    let mut per_color = [0u32; 8];
                    for color in 0..8 {
                        let total_color = snap.total_hearts[color];
                        let filled_color: u32 = snap.lives.iter().map(|l| l.filled[color]).sum();
                        per_color[color] = total_color.saturating_sub(filled_color);
                    }
                    snap.surplus_hearts = per_color;
                }
                game_state.live_surplus_ready_this_turn = true;

                Self::trigger_live_success_abilities(game_state, &player1_id);
                Self::trigger_auto_abilities_for_player(game_state, &player1_id);
                game_state.process_pending_auto_abilities(&player1_id);
                if game_state.has_pending_choice() {
                    return;
                }
                // each_time LIVE_SUCCESS triggers fire post-resolution
                // in process_current_ability (abilities.rs)
                let score_cur: HashMap<i16, i32> = game_state
                    .mods
                    .score_modifiers
                    .iter()
                    .map(|(&k, e)| (k, e.total()))
                    .collect();
                p1_extra = Self::score_delta_since(
                    &score_cur,
                    &pre_score_flat,
                    &game_state.player1.live_card_zone.cards,
                )
                .max(0) as u32;
                game_state.live_success_p1_extra = p1_extra;
                game_state.live_success_p2_fired = true;
            } else {
                // Re-entry after P1 triggers resolved but P2 still pending.
                p1_extra = game_state.live_success_p1_extra;
            }

            // P2 trigger block (shared between first entry and re-entry paths)
            // Set flag BEFORE triggering so that if P2's ability creates a choice
            // and the function returns early, P2 is not re-triggered on re-entry.
            game_state.live_success_p2_fired = true;
            Self::trigger_live_success_abilities(game_state, &player2_id);
            Self::trigger_auto_abilities_for_player(game_state, &player2_id);
            game_state.process_pending_auto_abilities(&player2_id);
            if game_state.has_pending_choice() {
                return;
            }
            // each_time LIVE_SUCCESS triggers fire post-resolution
            // in process_current_ability (abilities.rs)
            let score_cur2: HashMap<i16, i32> = game_state
                .mods
                .score_modifiers
                .iter()
                .map(|(&k, e)| (k, e.total()))
                .collect();
            p2_extra = Self::score_delta_since(
                &score_cur2,
                &pre_score_flat,
                &game_state.player2.live_card_zone.cards,
            )
            .max(0) as u32;
            game_state.live_success_p2_extra = p2_extra;
        }

        // Process any remaining auto-abilities that were queued but not yet resolved
        // (e.g. when multiple cards have LiveSuccess triggers and a previous call
        // returned early after the first ability created a pending choice).
        game_state.process_pending_auto_abilities(&player1_id);
        if game_state.has_pending_choice() {
            return;
        }
        game_state.process_pending_auto_abilities(&player2_id);
        if game_state.has_pending_choice() {
            return;
        }

        // Determine winner — use PRE-trigger modifiers so LiveSuccess
        // triggered changes only apply via pX_extra (no double-count).
        let need_heart_flat: HashMap<i16, HashMap<crate::card::HeartColor, ModifierEntry>> =
            game_state
                .mods
                .need_heart_modifiers
                .iter()
                .map(|(&k, colors)| {
                    let flat: HashMap<HeartColor, ModifierEntry> =
                        colors.iter().map(|(&c, e)| (c, *e)).collect();
                    (k, flat)
                })
                .collect();
        let player1_score = game_state.player1.live_card_zone.calculate_live_score(
            &game_state.card_database,
            game_state.player1_cheer_blade_heart_count,
            game_state.player1.stage_hearts.as_ref(),
            Some(&need_heart_flat),
            Some(&pre_score_flat),
            game_state.mods.p1_constant_total_score_bonus,
        ) + p1_extra;
        let player2_score = game_state.player2.live_card_zone.calculate_live_score(
            &game_state.card_database,
            game_state.player2_cheer_blade_heart_count,
            game_state.player2.stage_hearts.as_ref(),
            Some(&need_heart_flat),
            Some(&pre_score_flat),
            game_state.mods.p2_constant_total_score_bonus,
        ) + p2_extra;
        let player1_has_cards = !game_state.player1.live_card_zone.cards.is_empty();
        let player2_has_cards = !game_state.player2.live_card_zone.cards.is_empty();

        let (player1_won, player2_won) = if !player1_has_cards && !player2_has_cards {
            (false, false)
        } else if player1_has_cards && !player2_has_cards {
            (true, false)
        } else if !player1_has_cards && player2_has_cards {
            (false, true)
        } else if player1_score > player2_score {
            (true, false)
        } else if player2_score > player1_score {
            (false, true)
        } else {
            (true, true) // Rule 8.4.6.2: equal scores = both win
        };

        // Finalize snapshots for both players
        for snap in game_state.performance_snapshots.iter_mut() {
            let _player = if snap.player_id == player1_id {
                &game_state.player1
            } else {
                &game_state.player2
            };
            // Determine pass/fail for each live card.
            // NOTE: We iterate over snap.lives (built from perf.live_card_ids captured
            // before the zone was cleared by Rule 8.3.16) rather than
            // player.live_card_zone.cards which may have been emptied.
            for i in 0..snap.lives.len() {
                let lc_id = snap.lives[i].card_id;
                if let Some(card) = game_state.card_database.get_card(lc_id) {
                    snap.lives[i].card_id = lc_id;
                    snap.lives[i].card_no = crate::types::ArcStr::from(card.card_no.as_ref());
                    if let Some(ref nh) = card.need_heart {
                        let mut filled = EMPTY_H8;
                        // Build filled array from heart allocations targeting this live
                        for alloc in &snap.breakdown.allocations {
                            if alloc.target_idx == i {
                                filled[alloc.color] += alloc.amount;
                            }
                        }
                        let mut required_arr = EMPTY_H8;
                        // Build effective need_heart with set/additive logic
                        let has_set = game_state
                            .mods
                            .need_heart_modifiers
                            .get(&lc_id)
                            .is_some_and(|m| m.values().any(|e| e.set != 0));
                        if has_set {
                            // Q115/Q127: Set-to-X applies first, then additive stacks.
                            for (color, me) in game_state
                                .mods
                                .need_heart_modifiers
                                .get(&lc_id)
                                .into_iter()
                                .flatten()
                            {
                                if me.set != 0 {
                                    required_arr[color.index()] = me.set as u32;
                                }
                                if me.additive != 0 {
                                    let idx = color.index();
                                    let current = required_arr[idx] as i32;
                                    required_arr[idx] = (current + me.additive).max(0) as u32;
                                }
                            }
                        } else {
                            for (color, needed) in &nh.hearts {
                                let idx = color.index();
                                let mut val = *needed as i32;
                                if let Some(color_mods) =
                                    game_state.mods.need_heart_modifiers.get(&lc_id)
                                {
                                    if let Some(me) = color_mods.get(color) {
                                        val = (val + me.additive).max(0);
                                    }
                                }
                                required_arr[idx] = val as u32;
                            }
                        }
                        // Determine passed by comparing filled vs required (not vs total pool).
                        // heart00 (index 0) is a wildcard that fills deficits in any color.
                        // The Heart00 requirement (required_arr[0]) must also be met.
                        // Phase 3 allocates leftover colored hearts and Heart00 wildcards
                        // to fill required_arr[0]; those show up in filled[1..6] (colored)
                        // and filled[0] (Heart00 wildcard).
                        let passed = {
                            let mut wildcard = filled[0] + filled[7];
                            let mut ok = true;
                            // Rule 2.11.3 bullet 2: total provided >= total required
                            let total_filled: u32 = filled.iter().sum();
                            let total_required: u32 = required_arr.iter().sum();
                            if total_filled < total_required {
                                ok = false;
                            }
                            if ok && required_arr[0] > 0 {
                                let h00_satisfied: u32 = filled[1..7].iter().sum();
                                if h00_satisfied + wildcard < required_arr[0] {
                                    ok = false;
                                } else {
                                    // Fix C: decrement wildcard by amount consumed for Heart00
                                    let used = required_arr[0].saturating_sub(h00_satisfied);
                                    wildcard = wildcard.saturating_sub(used);
                                }
                            }
                            // Use remaining wildcard to cover deficits in specific colors
                            if ok {
                                for idx in 1..7 {
                                    if filled[idx] < required_arr[idx] {
                                        let deficit = required_arr[idx] - filled[idx];
                                        if wildcard >= deficit {
                                            wildcard -= deficit;
                                        } else {
                                            ok = false;
                                            break;
                                        }
                                    }
                                }
                            }
                            ok
                        };
                        // Populate adjustments and requirements from need_heart_modifiers
                        let mut adjustments = Vec::new();
                        if let Some(color_mods) = game_state.mods.need_heart_modifiers.get(&lc_id) {
                            let verbose = ABILITY_DEBUG.load(Ordering::Relaxed);
                            let req_source = verbose.then(|| format!("{} req modifier", card.name));
                            for (color, entry) in color_mods {
                                let total = entry.total();
                                if total != 0 {
                                    let color_label = heart_color_debug_name(color);
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
                                        value: total,
                                        color: color.index(),
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
                    } else {
                        snap.lives[i].passed = true;
                    }
                    let base_score = card.get_score() as i32;
                    let set_score = game_state.mods.get_score_set_modifier(lc_id);
                    let additive = game_state.mods.get_score_modifier(lc_id) - set_score;
                    let effective_base = if set_score != 0 {
                        set_score
                    } else {
                        base_score
                    };
                    snap.lives[i].score = (effective_base + additive).max(0) as u32;
                }
            }

            // Compute per-card spare (余剰ハート): remaining hearts from the pool
            // after this card's allocation. For each live card, spare = total available
            // minus all allocations up to and including this card.
            // Wildcard allocations use alloc.color as the TARGET color, but the
            // actual pool deduction is from heart00 (index 0) or icon_all (index 7).
            // We map to the source pool index so spare reflects the real pool.
            let mut cumulative_used = EMPTY_H8;
            for i in 0..snap.lives.len() {
                for alloc in &snap.breakdown.allocations {
                    if alloc.target_idx == i {
                        let source_idx = match alloc.phase {
                            crate::types::AllocPhase::H00Wild
                            | crate::types::AllocPhase::Wildcard => 0,
                            crate::types::AllocPhase::AllWild
                            | crate::types::AllocPhase::CAll
                            | crate::types::AllocPhase::AllCleanup => 7,
                            _ => alloc.color,
                        };
                        cumulative_used[source_idx] += alloc.amount;
                    }
                }
                let mut spare = EMPTY_H8;
                for idx in 0..8 {
                    spare[idx] = snap.total_hearts[idx].saturating_sub(cumulative_used[idx]);
                }
                snap.lives[i].spare = spare;
            }

            let is_first = snap.player_id == player1_id;
            snap.p0_wins = player1_won;
            snap.p1_wins = player2_won;
            snap.total_score = if is_first {
                player1_score
            } else {
                player2_score
            };
            // Rule 8.3.16: If ANY live card's need_heart could not be satisfied,
            // ALL live cards fail. Success requires ALL cards to pass.
            snap.success = snap.lives.iter().all(|l| l.passed) && snap.total_score > 0;
        }

        // Revert score modifiers added by LiveSuccess-triggered abilities.
        // The snapshot already captured the correct final score (including bonuses)
        // at lines 431-439. Delayed gained effects below will re-apply their
        // bonuses on the cleared state.
        {
            let post: HashMap<i16, i32> = game_state
                .mods
                .score_modifiers
                .iter()
                .map(|(&k, e)| (k, e.total()))
                .collect();
            for (&cid, post_total) in &post {
                let pre = pre_score_flat.get(&cid).copied().unwrap_or(0);
                let delta = post_total - pre;
                if delta != 0 {
                    game_state.mods.add_score_modifier(cid, -delta);
                }
            }
            for (&cid, &pre_total) in &pre_score_flat {
                if !post.contains_key(&cid) {
                    game_state.mods.set_score_modifier(cid, pre_total);
                }
            }
        }

        // Process delayed gained effects (e.g. constant gain_ability with
        // conditional_alternative gained_effect that checks revealed_cards).
        // These couldn't be evaluated at constant-evaluation time because the
        // yell result wasn't available yet.
        if !game_state.delayed_gained_effects.is_empty() {
            let saved_revealed = core::mem::take(&mut game_state.revealed_cards);
            // Populate revealed_cards from the first snapshot's yell data.
            if let Some(snap) = game_state.performance_snapshots.first() {
                game_state.revealed_cards = snap.yell_cards.iter().map(|yc| yc.card_id).collect();
            }
            let delayed = core::mem::take(&mut game_state.delayed_gained_effects);
            for (card_id, gained) in &delayed {
                use crate::ability::condition::ConditionContext;
                use crate::ability::enums::ActionType;
                use crate::ability::resolver::AbilityResolver;
                let ctx = ConditionContext::new(game_state);
                if gained.action == ActionType::ConditionalAlternative {
                    // Evaluate the conditional_alternative: check alternative
                    // condition first, then base condition.
                    let alt_cond = gained.compound.alternative_condition.as_ref();
                    let base_cond = gained.condition.as_ref();
                    let alt_met = alt_cond.is_some_and(|c| ctx.evaluate_condition(c));
                    let base_met = base_cond.is_some_and(|c| ctx.evaluate_condition(c));
                    if alt_met || base_met {
                        let alt_eff = gained.alternative_effect_any();
                        let prim_eff = gained.compound.primary_effect.as_ref();
                        let effect_to_apply = if alt_met {
                            alt_eff.as_ref()
                        } else {
                            prim_eff.as_ref()
                        };
                        if let Some(apply) = effect_to_apply {
                            let mut resolver = AbilityResolver::new(
                                game_state.card_database.clone(),
                                Some(*card_id),
                            );
                            resolver.activating_card_id = Some(*card_id);
                            let _ = resolver.execute_effect(game_state, apply);
                        }
                    }
                }
            }
            game_state.revealed_cards = saved_revealed;
        }

        // Merge LiveSuccess-triggered ability applications into breakdown.scores.
        // These were recorded after enrich_from_applications ran (in execute_performance_phase),
        // so they weren't picked up yet.
        let late_apps = core::mem::take(&mut game_state.ability_applications);
        if !late_apps.is_empty() {
            let p1_cards = &game_state.player1.live_card_zone.cards;
            let p2_cards = &game_state.player2.live_card_zone.cards;
            for snap in game_state.performance_snapshots.iter_mut() {
                let player_cards = if snap.player_id == player1_id {
                    &p1_cards
                } else {
                    &p2_cards
                };
                for app in &late_apps {
                    if (app.effect_type == crate::types::EffectType::ScoreBonus
                        || app.effect_type == crate::types::EffectType::ScoreSet)
                        && player_cards.contains(&app.target_card_id)
                    {
                        snap.breakdown.scores.push(crate::types::ScoreLine {
                            source: app.ability_text.to_string(),
                            value: app.amount.unsigned_abs(),
                        });
                    }
                }
            }
        }

        // Compute surplus from finalized snapshots.
        // Surplus = remaining hearts after filling all live card requirements.
        // Uses actual filled allocations (not just required) to handle cases where
        // available hearts are less than required.
        let mut p2_surplus = 0u32;
        let mut p1_surplus = 0u32;
        for snap in &mut game_state.performance_snapshots {
            let total_available: u32 = snap.total_hearts.iter().sum();
            let total_filled: u32 = snap.lives.iter().flat_map(|l| l.filled.iter()).sum();
            let surplus = total_available.saturating_sub(total_filled);
            // Compute per-color surplus
            let mut per_color_surplus = [0u32; 8];
            for color in 0..8 {
                let total_color = snap.total_hearts[color];
                let filled_color: u32 = snap.lives.iter().map(|l| l.filled[color]).sum();
                per_color_surplus[color] = total_color.saturating_sub(filled_color);
            }
            snap.surplus_hearts = per_color_surplus;
            log::debug!(
                "[SURPLUS] player={} total_avail={} total_filled={} surplus={} per_color={:?} lives={}",
                snap.player_id,
                total_available,
                total_filled,
                surplus,
                per_color_surplus,
                snap.lives.len()
            );
            for (i, l) in snap.lives.iter().enumerate() {
                log::debug!(
                    "[SURPLUS]   live[{}] passed={} required={:?} filled={:?} spare={:?}",
                    i,
                    l.passed,
                    l.required,
                    l.filled,
                    l.spare
                );
            }
            for a in &snap.breakdown.allocations {
                log::debug!(
                    "[SURPLUS]   alloc target={} color={} amount={} wildcard={}",
                    a.target_idx,
                    a.color,
                    a.amount,
                    a.wildcard
                );
            }
            if snap.player_id == player2_id {
                p2_surplus = surplus;
            } else {
                p1_surplus = surplus;
            }
        }
        game_state.opponent_live_surplus_count = p2_surplus;
        game_state.self_live_surplus_count = p1_surplus;
        game_state.live_surplus_ready_this_turn = true;
        if player2_won {
            game_state.set_opponent_live_success(p2_surplus == 0);
        }
        if player1_won {
            game_state.self_no_excess_heart_this_turn = p1_surplus == 0;
        }

        // Push performance summary to rule log

        let card_db = game_state.card_database.clone();
        if ABILITY_DEBUG.load(Ordering::Relaxed) {
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

        Self::move_restricted_cards_to_discard(&mut game_state.player1, &card_db);
        Self::move_restricted_cards_to_discard(&mut game_state.player2, &card_db);
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
    }

    fn move_restricted_cards_to_discard(
        player: &mut crate::player::Player,
        card_db: &CardDatabase,
    ) {
        let mut cards_to_remove = Vec::new();
        for (index, card_id) in player.live_card_zone.cards.iter().enumerate() {
            if let Some(card) = card_db.get_card(*card_id) {
                let has_restriction = card.abilities.iter().any(|ar| {
                    let ability = ar.resolve();
                    if let Some(ref effect) = ability.effect {
                        let rd_binding = effect.restricted_destination_any();
                        let dest_binding = effect.destination.as_deref();
                        let restricted_dest = rd_binding.or(dest_binding);
                        effect.action == crate::ability::enums::ActionType::Restriction
                            && effect.restriction_type_any().as_deref() == Some("cannot_place")
                            && matches!(
                                restricted_dest.and_then(Zone::from_str),
                                Some(Zone::SuccessLiveZone | Zone::LiveCardZone)
                            )
                    } else {
                        false
                    }
                });
                if has_restriction {
                    cards_to_remove.push(index);
                }
            }
        }
        for &idx in cards_to_remove.iter().rev() {
            if idx < player.live_card_zone.cards.len() {
                let card_id = player.live_card_zone.cards.remove(idx);
                player.waitroom.cards.push(card_id);
            }
        }
    }

    fn process_player_live_result(
        player: &mut crate::player::Player,
        won: bool,
        must_skip: bool,
        can_place: bool,
    ) {
        let card_count = player.live_card_zone.cards.len();
        if won && !must_skip && card_count > 0 {
            let card_id = player.live_card_zone.cards.remove(0);
            if can_place {
                player.success_live_card_zone.cards.push(card_id);
            } else {
                player.waitroom.cards.push(card_id);
            }
        }
        while !player.live_card_zone.cards.is_empty() {
            let card_id = player.live_card_zone.cards.remove(0);
            player.waitroom.cards.push(card_id);
        }
    }

    fn try_take_success_zone_choice(
        game_state: &mut GameState,
        won: bool,
        must_skip: bool,
        cards_count: usize,
        cards: Vec<i16>,
        player_id: &str,
    ) -> bool {
        if !won || must_skip || cards_count <= 1 {
            return false;
        }
        let can_place = cards.iter().any(|&cid| {
            game_state.can_place_card_in_zone(cid, Zone::SuccessLiveZone.to_str(), player_id)
        });
        if !can_place {
            return false;
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
        let choice = crate::ability::types::Choice::SelectLiveSuccess {
            player_id: player_id.to_string(),
            count: 1,
            options,
            description: "Choose which live card goes to your success zone".to_string(),
            description_en: Some("Choose which live card goes to your success zone".to_string()),
            description_ja: Some("ライブエリアからサクセスゾーンに送るカードを選択".to_string()),
        };
        game_state.ability_queue.pause_for_choice(choice);
        true
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
        let p1_must_skip = player1_won && player2_won && p1_cards >= 2;
        let p2_must_skip = player1_won && player2_won && p2_cards >= 2;

        log::debug!(
            "[MULTI_LIVE] p1_won={} p2_won={} p1_cards={} p2_cards={} p1_must={} p2_must={}",
            player1_won,
            player2_won,
            p1_cards,
            p2_cards,
            p1_must_skip,
            p2_must_skip
        );
        if Self::try_take_success_zone_choice(
            game_state,
            player1_won,
            p1_must_skip,
            p1_cards,
            game_state.player1.live_card_zone.cards.to_vec(),
            &p1_id,
        ) || Self::try_take_success_zone_choice(
            game_state,
            player2_won,
            p2_must_skip,
            p2_cards,
            game_state.player2.live_card_zone.cards.to_vec(),
            &p2_id,
        ) {
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

        // Record what cards actually move to the waitroom (discard)
        let mut moved_to_waitroom = Vec::new();

        let p1_live_before = game_state.player1.live_card_zone.cards.clone();
        let p2_live_before = game_state.player2.live_card_zone.cards.clone();

        Self::process_player_live_result(
            &mut game_state.player1,
            player1_won,
            p1_must_skip,
            p1_can_place,
        );
        Self::process_player_live_result(
            &mut game_state.player2,
            player2_won,
            p2_must_skip,
            p2_can_place,
        );

        // Find which cards ended up in waitroom
        for cid in p1_live_before {
            if game_state.player1.waitroom.cards.contains(&cid) {
                moved_to_waitroom.push(cid);
            }
        }
        for cid in p2_live_before {
            if game_state.player2.waitroom.cards.contains(&cid) {
                moved_to_waitroom.push(cid);
            }
        }

        if !moved_to_waitroom.is_empty() {
            game_state.recently_moved_cards = Some(moved_to_waitroom.into());
            game_state.recently_moved_from_zone = Some("live_card_zone".to_string());

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
            let is_constant = ability
                .triggers
                .as_ref()
                .is_some_and(|t| t.contains(crate::triggers::CONSTANT));
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
            let alt_source = alt.source.as_deref().unwrap_or("");
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
            entry.player_id = player_id.to_string();
            entry.choice_player_id = Some(player_id.to_string());
            if let Some(card) = game_state.card_database.get_card(card_id) {
                entry.card_no = card.card_no.to_string();
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
        while !player.live_card_zone.cards.is_empty() {
            player
                .waitroom
                .add_card(player.live_card_zone.cards.remove(0));
        }
        Ok(())
    }

    pub fn player_perform_live(
        player: &mut crate::player::Player,
        resolution_zone: &mut crate::zones::ResolutionZone,
        _player_id: &str,
        card_db: &CardDatabase,
        blade_modifiers: &HashMap<i16, ModifierEntry>,
        heart_override: &HashMap<i16, (HeartColor, u32)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, i32>>,
        blade_type_modifiers: &HashMap<i16, BladeColor>,
        orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
        cannot_live: bool,
    ) -> LivePerformanceData {
        #[cfg(not(feature = "no_std"))]
        let _t = crate::timer::Timer::start("player_perform_live");
        // Q68/Rule: "cannot_live" discards live cards during performance; no yell, no live.
        if cannot_live {
            let moved: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
            player
                .waitroom
                .cards
                .extend(player.live_card_zone.cards.drain(..));
            return LivePerformanceData {
                yell_count: 0,
                note_icons: 0,
                revealed_ids: Vec::new(),
                member_contributions: Vec::new(),
                yell_cards: Vec::new(),
                total_hearts: [0; 8],
                allocations: Vec::new(),
                heart_sources: Vec::new(),
                blade_sources: Vec::new(),
                draw_effects_occurred: false,
                live_card_ids: vec![],
                moved_live_card_ids: moved,
            };
        }

        // Capture member contributions (base values + modifiers)
        // Always computed — even if live card zone is empty, stage members still contribute.
        let mut member_contributions = Vec::new();
        for i in 0..3 {
            let cid = player.stage.stage[i];
            if cid == -1 {
                continue;
            }
            let mut base_h = EMPTY_H8;
            let mut bonus_h = EMPTY_H8;
            let mut base_blades = 0u32;
            let mut draw_icons = 0u32;
            let ability_heart_bonuses = Vec::new();
            let ability_blade_bonuses = Vec::new();

            if let Some(card) = card_db.get_card(cid) {
                base_blades = card.blade;
                if let Some(ref bh) = card.base_heart {
                    for (color, count) in &bh.hearts {
                        let idx = color.index();
                        if idx < 8 {
                            base_h[idx] += count;
                        }
                    }
                }
                if let Some(ref sh) = card.special_heart {
                    for (color, count) in &sh.hearts {
                        if *color == HeartColor::Draw {
                            draw_icons += count;
                        }
                    }
                }
            }

            let entry = blade_modifiers.get(&cid).copied().unwrap_or_default();
            let (effective_base_blades, bonus_blades) = if entry.set != 0 {
                // set replaces the base blade — additive stacks on top
                (entry.total().max(0) as u32, 0u32)
            } else {
                (base_blades, entry.total().max(0) as u32)
            };
            base_blades = effective_base_blades;

            if let Some(mods) = heart_modifiers.get(&cid) {
                for (color, delta) in mods {
                    let idx = color.index();
                    if idx < 8 && *delta > 0 {
                        bonus_h[idx] += *delta as u32;
                    }
                }
            }

            // Apply heart_color_multiplier (set_heart_type): transform all hearts to one color
            if let Some(override_color) = heart_color_multiplier.get(&cid) {
                let total: u32 = base_h.iter().sum();
                base_h = EMPTY_H8;
                base_h[override_color.index()] = total;
            }

            // Check for heart override
            if let Some(&(override_color, override_count)) = heart_override.get(&cid) {
                base_h = EMPTY_H8;
                let idx = override_color.index();
                if idx < 8 {
                    base_h[idx] = override_count;
                }
            }

            let is_wait = orientation_modifiers
                .get(&cid)
                .map(|o| *o == crate::core::game_modifiers::CardOrientation::Wait)
                .unwrap_or(false);

            member_contributions.push(MemberContribution {
                source_id: cid,
                slot: i,
                base_hearts: base_h,
                bonus_hearts: bonus_h,
                base_blades,
                bonus_blades,
                base_notes: 0,
                bonus_notes: 0,
                draw_icons,
                ability_heart_bonuses,
                ability_blade_bonuses,
                card_no: card_db
                    .get_card(cid)
                    .map(|c| crate::types::ArcStr::from(c.card_no.as_ref()))
                    .unwrap_or_default(),
                is_wait,
            });
        }

        // Q32/Rule 8.3.6: If the live card zone is empty, no yell, no live card processing.
        // But member contributions and stage hearts are still returned.
        if player.live_card_zone.cards.is_empty() {
            let mut total_hearts_arr = EMPTY_H8;
            for mc in &member_contributions {
                for c in 0..8 {
                    total_hearts_arr[c] += mc.base_hearts[c] + mc.bonus_hearts[c];
                }
            }
            let total_blade: u32 = member_contributions
                .iter()
                .filter(|m| !m.is_wait)
                .map(|m| m.base_blades + m.bonus_blades)
                .sum();
            return LivePerformanceData {
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
            };
        }

        let total_blade =
            player
                .stage
                .total_blades(card_db, blade_modifiers, orientation_modifiers, false);

        // Q40: Yell must complete ALL checks — even if hearts are already satisfied,
        // the full blade-count of yell cards is always revealed.
        let mut yell_cards = Vec::new();
        // Q104 / Q100 / Rule 10.2.1: refresh from waitroom when deck runs out mid-draw.
        for _ in 0..total_blade {
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
            }
            if let Some(card_id) = player.main_deck.draw() {
                resolution_zone.cards.push(card_id);
            }
        }

        // Compute owned hearts from stage
        let mut owned_hearts = player.stage.get_available_hearts(
            card_db,
            heart_override,
            heart_modifiers,
            heart_color_multiplier,
        );

        let blade_to_heart = |bc: BladeColor| -> HeartColor {
            match bc {
                BladeColor::Peach => HeartColor::Heart01,
                BladeColor::Red => HeartColor::Heart02,
                BladeColor::Yellow => HeartColor::Heart03,
                BladeColor::Green => HeartColor::Heart04,
                BladeColor::Blue => HeartColor::Heart05,
                BladeColor::Purple => HeartColor::Heart06,
                BladeColor::All => HeartColor::Heart00,
            }
        };
        let override_color = (0..3)
            .filter_map(|i| {
                let cid = player.stage.stage[i];
                if cid == -1 {
                    None
                } else {
                    blade_type_modifiers.get(&cid).copied().map(blade_to_heart)
                }
            })
            .next();

        // Process yell cards and build YellCardResult + track heart allocations
        let mut cheer_icon_count = 0u32;
        let mut heart_sources: Vec<HeartSource> = Vec::new();
        let mut blade_sources: Vec<BladeSource> = Vec::new();
        let mut total_hearts_arr = EMPTY_H8;

        // Stage hearts source
        let stage_heart_str: crate::types::ArcStr = "Stage (base)".into();
        let mut stage_heart_arr = EMPTY_H8;
        for (color, count) in &owned_hearts.hearts {
            let idx = color.index();
            if idx < 8 {
                stage_heart_arr[idx] += count;
            }
        }
        heart_sources.push(HeartSource {
            source_type: SourceType::Stage,
            source: stage_heart_str,
            value: stage_heart_arr,
        });
        for (color, count) in &owned_hearts.hearts {
            let idx = color.index();
            if idx < 8 {
                total_hearts_arr[idx] += count;
            }
        }

        // Add stage blade source
        blade_sources.push(BladeSource {
            source_type: SourceType::Stage,
            source: crate::types::ArcStr::from("Stage members"),
            value: total_blade,
        });

        // Q42: Defer draw effects until after all yell cards have been revealed.
        // Count draw icons during the loop, then process all draws at once after.
        let mut total_draw_icons = 0u32;

        for card_id in &resolution_zone.cards {
            if let Some(card) = card_db.get_card(*card_id) {
                let mut bh_arr = EMPTY_H8;
                let mut note_icons = 0u32;
                let mut draw_icons = 0u32;

                if let Some(ref bh) = card.blade_heart {
                    for (color, count) in &bh.hearts {
                        // Draw/Score special icons are never converted by
                        // set_blade_type — they pass through unchanged.
                        let effective_color =
                            if matches!(*color, HeartColor::Draw | HeartColor::Score) {
                                *color
                            } else {
                                override_color.unwrap_or(*color)
                            };
                        // Q45: ALL-blade (BAll) can be treated as any color heart.
                        // Mapped to HeartColor::All (icon_all, index 7) so the UI
                        // displays icon_all.png for BAll yell hearts.
                        if effective_color == HeartColor::BAll {
                            *owned_hearts.hearts.entry_or_default(HeartColor::All) += count;
                            bh_arr[7] += count;
                            total_hearts_arr[7] += count;
                        } else if effective_color == HeartColor::Draw {
                            draw_icons += count;
                        // Q44: Each score icon revealed during yell adds 1 to total score.
                        } else if effective_color == HeartColor::Score {
                            note_icons += count;
                            cheer_icon_count += count;
                        } else {
                            let idx = effective_color.index();
                            if idx < 8 {
                                *owned_hearts.hearts.entry_or_default(effective_color) += count;
                                bh_arr[idx] += count;
                                total_hearts_arr[idx] += count;
                            }
                        }
                    }
                }

                // Also process special_heart on yell cards (e.g. draw-from-エール)
                if let Some(ref sh) = card.special_heart {
                    for (color, count) in &sh.hearts {
                        if *color == HeartColor::Draw {
                            draw_icons += count;
                        } else if *color == HeartColor::Score {
                            note_icons += count;
                            cheer_icon_count += count;
                        }
                    }
                }

                total_draw_icons += draw_icons;

                yell_cards.push(YellCardResult {
                    card_id: *card_id,
                    blade_hearts: bh_arr,
                    note_icons,
                    draw_icons,
                    card_no: card_db
                        .get_card(*card_id)
                        .map(|c| crate::types::ArcStr::from(c.card_no.as_ref()))
                        .unwrap_or_default(),
                });
            }
        }

        // Q43: Each draw icon revealed during yell draws 1 card.
        // Process all deferred draw effects after all yell cards are revealed (Q42).
        // Q104 / Rule 10.2.1: refresh from waitroom when deck runs out mid-draw.
        for _ in 0..total_draw_icons {
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
            }
            if let Some(new_card) = player.main_deck.draw() {
                player.hand.add_card(new_card);
            }
        }

        // Yell heart source
        let mut yell_heart_arr = EMPTY_H8;
        for yc in &yell_cards {
            for i in 0..8 {
                yell_heart_arr[i] += yc.blade_hearts[i];
            }
        }
        if yell_heart_arr.iter().any(|&v| v > 0) {
            heart_sources.push(HeartSource {
                source_type: SourceType::Yell,
                source: crate::types::ArcStr::from("Yell cards"),
                value: yell_heart_arr,
            });
        }
        blade_sources.push(BladeSource {
            source_type: SourceType::Yell,
            source: format!("{} blades", total_blade).into(),
            value: total_blade,
        });

        // Live card special hearts
        // Collect draw counts first (immutable), then draw with refresh (mutable).
        let mut special_draw_count = 0u32;
        for &lc_id in &player.live_card_zone.cards {
            if let Some(card) = card_db.get_card(lc_id) {
                if let Some(ref sh) = card.special_heart {
                    for (color, count) in &sh.hearts {
                        if *color == HeartColor::Draw {
                            special_draw_count += count;
                        } else if *color == HeartColor::Score {
                            cheer_icon_count += count;
                        }
                    }
                }
            }
        }
        // Q104 / Rule 10.2.1: refresh from waitroom mid-draw
        for _ in 0..special_draw_count {
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
            }
            if let Some(new_card) = player.main_deck.draw() {
                player.hand.add_card(new_card);
            }
        }

        let live_card_ids: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
        let allocations =
            Self::compute_allocations(&owned_hearts, &live_card_ids, card_db, need_heart_modifiers);

        // Return yell-phase data WITHOUT draining resolution_zone or checking hearts.
        // execute_performance_phase will populate revealed_cards, trigger auto abilities
        // (8.3.13 check timing), then call check_live_success (8.3.14-8.3.16).
        let revealed_ids: Vec<i16> = resolution_zone.cards.iter().copied().collect();
        let draw_effects_occurred = yell_cards.iter().any(|yc| yc.draw_icons > 0);
        LivePerformanceData {
            yell_count: total_blade,
            note_icons: cheer_icon_count,
            revealed_ids,
            member_contributions,
            yell_cards,
            total_hearts: total_hearts_arr,
            allocations,
            heart_sources,
            blade_sources,
            draw_effects_occurred,
            live_card_ids,
            moved_live_card_ids: Vec::new(),
        }
    }

    /// Build heart allocations for live cards from the available heart pool.
    /// Shared between the yell phase and check_live_success (which recomputes
    /// owned_hearts after ability-granted hearts are added at 8.3.13).
    ///
    /// Strategy (in order per card):
    ///   1a_colored       — matching colored hearts → specific color req
    ///   1b_h00_wild      — Heart00 wildcard → remaining color deficit (NO icon_all yet)
    ///   2_wildcard       — remaining Heart00 wild → color deficit (second pass)
    ///   3a_colored_surplus — leftover colored hearts → Heart00 req (demand-aware:
    ///                        prefers colors with most surplus vs future demand)
    ///   3b_h00           — Heart00 → remaining Heart00 req
    ///   4_all_cleanup    — icon_all → ANY remaining deficit (color first, then heart00)
    ///                     NO icon_all is used before this phase.
    ///
    /// If smart greedy fails, falls back to exhaustive backtracking over
    /// Phase 3a / Phase 4 choices to guarantee a solution when one exists.
    pub fn compute_allocations(
        owned_hearts: &BaseHeart,
        live_card_ids: &[i16],
        card_db: &CardDatabase,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
    ) -> Vec<Allocation> {
        // Build normalized card needs array + future demand
        let card_needs = Self::build_card_needs(live_card_ids, card_db, need_heart_modifiers);
        let future_demand = Self::compute_future_demand(&card_needs);

        // Initialize pool as array for deterministic access
        let mut pool = [0u32; 8];
        for (color, count) in &owned_hearts.hearts {
            pool[color.index()] += count;
        }

        // Try smart greedy first
        let mut pool_copy = pool;
        let greedy = Self::greedy_allocate(&mut pool_copy, &card_needs, &future_demand);
        if Self::allocations_pass(&greedy, &card_needs) {
            return greedy;
        }

        // Greedy failed — backtrack over Phase 3a + icon_all choices
        let pool_arr = pool;
        if let Some(bt) = Self::backtrack_allocate(&pool_arr, &card_needs) {
            return bt;
        }

        // Fallback: return greedy result even if some cards failed
        greedy
    }

    fn build_card_needs(
        live_card_ids: &[i16],
        card_db: &CardDatabase,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
    ) -> Vec<CardNeed> {
        let mut needs = Vec::new();
        for &lc_id in live_card_ids {
            if let Some(card) = card_db.get_card(lc_id) {
                let mut need = [0u32; 8];
                let has_set = need_heart_modifiers
                    .get(&lc_id)
                    .is_some_and(|m| m.values().any(|e| e.set != 0));
                if let Some(ref nh) = card.need_heart {
                    if has_set {
                        // Q115/Q127: Set-to-X applies first, then additive stacks.
                        for (color, me) in need_heart_modifiers.get(&lc_id).into_iter().flatten() {
                            if me.set != 0 {
                                need[color.index()] = me.set as u32;
                            }
                            if me.additive != 0 {
                                let idx = color.index();
                                let current = need[idx] as i32;
                                need[idx] = (current + me.additive).max(0) as u32;
                            }
                        }
                    } else {
                        for (color, count) in &nh.hearts {
                            let idx = color.index();
                            let mut val = *count as i32;
                            if let Some(mods) = need_heart_modifiers.get(&lc_id) {
                                if let Some(me) = mods.get(color) {
                                    val = (val + me.additive).max(0);
                                }
                            }
                            need[idx] = val as u32;
                        }
                    }
                }
                needs.push(CardNeed {
                    name: crate::types::ArcStr::from(card.name.as_ref()),
                    need,
                });
            }
        }
        needs
    }

    /// Compute future demand per card: for cards i+1..N, sum of non-heart00 needs.
    fn compute_future_demand(card_needs: &[CardNeed]) -> Vec<[u32; 8]> {
        let n = card_needs.len();
        let mut demand = vec![[0u32; 8]; n];
        let mut running = [0u32; 8];
        for i in (0..n).rev() {
            if i + 1 < n {
                for c in 1..7 {
                    demand[i][c] = running[c];
                }
            }
            for c in 1..7 {
                running[c] += card_needs[i].need[c];
            }
        }
        demand
    }

    /// Smart greedy allocation: demand-aware Phase 3a + icon_all-last Phase 4.
    fn greedy_allocate(
        pool: &mut [u32; 8],
        card_needs: &[CardNeed],
        future_demand: &[[u32; 8]],
    ) -> Vec<Allocation> {
        let mut allocs = Vec::new();
        for (live_idx, cn) in card_needs.iter().enumerate() {
            let need = cn.need;
            // Track per-color totals for this card (direct + wildcard already assigned)
            let mut filled = [0u32; 8];
            let card_name = &cn.name;

            // Phase 1a: matching colored hearts → specific color req
            for c in 1..7 {
                if need[c] > 0 && pool[c] > 0 {
                    let take = pool[c].min(need[c]);
                    allocs.push(Allocation {
                        target_idx: live_idx,
                        target_name: card_name.clone(),
                        source_type: SourceType::Stage,
                        source_name: SourceName::StageHearts,
                        source_slot: None,
                        wildcard: false,
                        color: c,
                        amount: take,
                        is_bonus: false,
                        phase: AllocPhase::Colored,
                    });
                    pool[c] -= take;
                    filled[c] += take;
                }
            }

            // Phase 1b: Heart00 wild → remaining color deficit (no icon_all)
            for c in 1..7 {
                if need[c] > filled[c] && pool[0] > 0 {
                    let deficit = need[c] - filled[c];
                    let take = pool[0].min(deficit);
                    allocs.push(Allocation {
                        target_idx: live_idx,
                        target_name: card_name.clone(),
                        source_type: SourceType::Stage,
                        source_name: SourceName::WildcardHeart00,
                        source_slot: None,
                        wildcard: true,
                        color: c,
                        amount: take,
                        is_bonus: false,
                        phase: AllocPhase::H00Wild,
                    });
                    pool[0] -= take;
                    filled[c] += take;
                }
            }

            // Phase 2: remaining Heart00 wild → color deficit (second pass for multi-deficit)
            for c in 1..7 {
                if need[c] > filled[c] && pool[0] > 0 {
                    let deficit = need[c] - filled[c];
                    let take = pool[0].min(deficit);
                    allocs.push(Allocation {
                        target_idx: live_idx,
                        target_name: card_name.clone(),
                        source_type: SourceType::Stage,
                        source_name: SourceName::WildcardHeart00,
                        source_slot: None,
                        wildcard: true,
                        color: c,
                        amount: take,
                        is_bonus: false,
                        phase: AllocPhase::Wildcard,
                    });
                    pool[0] -= take;
                    filled[c] += take;
                }
            }

            // Phase 3a: total remaining deficit = total_required - total_filled_so_far.
            // Need[0] is the "any" portion, but the total must also be met.
            let total_filled_so_far: u32 = filled.iter().sum();
            let total_required: u32 = need.iter().sum();
            let h00_deficit = total_required.saturating_sub(total_filled_so_far);
            if h00_deficit > 0 {
                // Demand-aware: sort colors by (pool - future_demand) descending
                // so colors with most surplus vs. future cards get consumed first.
                let mut surplus_colors: Vec<usize> = (1..7).collect();
                surplus_colors.sort_by(|&a, &b| {
                    let score_a = pool[a] as i32 - future_demand[live_idx][a] as i32;
                    let score_b = pool[b] as i32 - future_demand[live_idx][b] as i32;
                    score_b.cmp(&score_a)
                });
                let mut filled_h00 = 0u32;
                for &c in &surplus_colors {
                    if filled_h00 >= h00_deficit {
                        break;
                    }
                    if pool[c] > 0 {
                        let take = pool[c].min(h00_deficit - filled_h00);
                        allocs.push(Allocation {
                            target_idx: live_idx,
                            target_name: card_name.clone(),
                            source_type: SourceType::Stage,
                            source_name: SourceName::StageHearts,
                            source_slot: None,
                            wildcard: false,
                            color: c,
                            amount: take,
                            is_bonus: false,
                            phase: AllocPhase::ColoredSurplus,
                        });
                        pool[c] -= take;
                        filled_h00 += take;
                        filled[c] += take;
                    }
                }

                // Phase 3b: Heart00 → remaining Heart00 deficit
                if filled_h00 < h00_deficit && pool[0] > 0 {
                    let take = pool[0].min(h00_deficit - filled_h00);
                    allocs.push(Allocation {
                        target_idx: live_idx,
                        target_name: card_name.clone(),
                        source_type: SourceType::Stage,
                        source_name: SourceName::StageHearts,
                        source_slot: None,
                        wildcard: false,
                        color: 0,
                        amount: take,
                        is_bonus: false,
                        phase: AllocPhase::H00,
                    });
                    pool[0] -= take;
                    filled_h00 += take;
                    let _ = filled_h00;
                }
            }

            // Phase 4: icon_all → ANY remaining deficit (color deficits first, then heart00)
            if pool[7] > 0 {
                // Color deficits first (otherwise they'd need heart00 which we might not have)
                for c in 1..7 {
                    if need[c] > filled[c] && pool[7] > 0 {
                        let deficit = need[c] - filled[c];
                        let take = pool[7].min(deficit);
                        allocs.push(Allocation {
                            target_idx: live_idx,
                            target_name: card_name.clone(),
                            source_type: SourceType::Stage,
                            source_name: SourceName::AllHeartIconAll,
                            source_slot: None,
                            wildcard: true,
                            color: c,
                            amount: take,
                            is_bonus: false,
                            phase: AllocPhase::AllCleanup,
                        });
                        pool[7] -= take;
                        filled[c] += take;
                    }
                }
                // Remaining icon_all → heart00 deficit
                let total_colored: u32 = filled[1..7].iter().sum();
                let h00_remaining = need[0].saturating_sub(total_colored);
                if h00_remaining > 0 && pool[7] > 0 {
                    // Also include any previous filled[0] from Phase 3b
                    let already_filled_h00 = filled[0];
                    let h00_still_needed = h00_remaining.saturating_sub(already_filled_h00);
                    if h00_still_needed > 0 && pool[7] > 0 {
                        let take = pool[7].min(h00_still_needed);
                        allocs.push(Allocation {
                            target_idx: live_idx,
                            target_name: card_name.clone(),
                            source_type: SourceType::Stage,
                            source_name: SourceName::AllHeartIconAll,
                            source_slot: None,
                            wildcard: false,
                            color: 7,
                            amount: take,
                            is_bonus: false,
                            phase: AllocPhase::AllCleanup,
                        });
                        pool[7] -= take;
                        filled[0] += take;
                        let _ = filled;
                    }
                }
            }
        }
        allocs
    }

    /// Check whether all cards' heart requirements are satisfied by the allocations.
    /// Uses the same logic as the pass/fail check in execute_live_victory_determination.
    fn allocations_pass(allocs: &[Allocation], card_needs: &[CardNeed]) -> bool {
        if card_needs.is_empty() {
            return true;
        }
        // Build per-card filled arrays
        let num_cards = card_needs.len();
        let mut per_card_filled = vec![[0u32; 8]; num_cards];
        for a in allocs {
            if a.target_idx < num_cards {
                per_card_filled[a.target_idx][a.color] += a.amount;
            }
        }
        // Check each card
        for (i, cn) in card_needs.iter().enumerate() {
            let filled = per_card_filled[i];
            let req = cn.need;
            let mut wildcard = filled[0] + filled[7];
            let mut ok = true;
            let total_filled: u32 = filled.iter().sum();
            let total_required: u32 = req.iter().sum();
            if total_filled < total_required {
                ok = false;
            }
            if ok && req[0] > 0 {
                let h00_satisfied: u32 = filled[1..7].iter().sum();
                if h00_satisfied + wildcard < req[0] {
                    ok = false;
                } else {
                    wildcard = wildcard.saturating_sub(req[0].saturating_sub(h00_satisfied));
                }
            }
            if ok {
                for idx in 1..7 {
                    if filled[idx] < req[idx] {
                        let deficit = req[idx] - filled[idx];
                        if wildcard >= deficit {
                            wildcard -= deficit;
                        } else {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            if !ok {
                return false;
            }
        }
        true
    }

    /// Exhaustive backtracking search over Phase 3a + Phase 4 choices.
    /// Tries all valid ways to extract hearts from the pool per card.
    fn backtrack_allocate(pool: &[u32; 8], card_needs: &[CardNeed]) -> Option<Vec<Allocation>> {
        let mut allocs = Vec::new();
        let mut current_pool = *pool;
        if Self::bt_search(&mut current_pool, card_needs, 0, &mut allocs) {
            Some(allocs)
        } else {
            None
        }
    }

    /// Recursive backtracking: try all valid allocations for card `idx` then recurse.
    fn bt_search(
        pool: &mut [u32; 8],
        card_needs: &[CardNeed],
        idx: usize,
        allocs: &mut Vec<Allocation>,
    ) -> bool {
        if idx >= card_needs.len() {
            return true;
        }
        let saved_pool = *pool;
        let saved_len = allocs.len();
        let cn = &card_needs[idx];
        let need = cn.need;
        let card_name = &cn.name;

        // ----- Forced phases (no choice) -----

        // Phase 1a: matching colored hearts → color req (no choice)
        let mut filled = [0u32; 8];
        for c in 1..7 {
            if need[c] > 0 && pool[c] > 0 {
                let take = pool[c].min(need[c]);
                allocs.push(Allocation {
                    target_idx: idx,
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::StageHearts,
                    source_slot: None,
                    wildcard: false,
                    color: c,
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::Colored,
                });
                pool[c] -= take;
                filled[c] += take;
            }
        }

        // Phase 1b: Heart00 → color deficit (no choice, uses only heart00)
        for c in 1..7 {
            if need[c] > filled[c] && pool[0] > 0 {
                let deficit = need[c] - filled[c];
                let take = pool[0].min(deficit);
                allocs.push(Allocation {
                    target_idx: idx,
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::WildcardHeart00,
                    source_slot: None,
                    wildcard: true,
                    color: c,
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::H00Wild,
                });
                pool[0] -= take;
                filled[c] += take;
            }
        }

        // Phase 2: second pass Heart00 → color deficit
        for c in 1..7 {
            if need[c] > filled[c] && pool[0] > 0 {
                let deficit = need[c] - filled[c];
                let take = pool[0].min(deficit);
                allocs.push(Allocation {
                    target_idx: idx,
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::WildcardHeart00,
                    source_slot: None,
                    wildcard: true,
                    color: c,
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::Wildcard,
                });
                pool[0] -= take;
                filled[c] += take;
            }
        }

        // ----- Choice phases: Phase 3a (which surplus colors → heart00) -----
        let total_filled_so_far: u32 = filled.iter().sum();
        let total_required: u32 = need.iter().sum();
        let h00_deficit = total_required.saturating_sub(total_filled_so_far);

        // Collect available surplus colors
        let mut surplus_colors: Vec<usize> = (1..7).filter(|&c| pool[c] > 0).collect();
        surplus_colors.sort();
        let total_surplus: u32 = surplus_colors.iter().map(|&c| pool[c]).sum();
        let h00_from_surplus = h00_deficit.min(total_surplus);

        let found = Self::try_surplus_compositions(
            pool,
            card_needs,
            idx,
            &surplus_colors,
            h00_from_surplus,
            0,
            allocs,
            filled,
        );
        if found {
            return true;
        }

        // Undo: restore pool and truncate allocs
        *pool = saved_pool;
        allocs.truncate(saved_len);
        false
    }

    /// Recursively enumerate all compositions of `remaining` hearts from `colors[color_idx..]`.
    fn try_surplus_compositions(
        pool: &mut [u32; 8],
        card_needs: &[CardNeed],
        idx: usize,
        colors: &[usize],
        remaining: u32,
        color_idx: usize,
        allocs: &mut Vec<Allocation>,
        filled: [u32; 8],
    ) -> bool {
        let cn = &card_needs[idx];
        let card_name = &cn.name;

        if color_idx >= colors.len() {
            if remaining > 0 {
                return false;
            }
            return Self::try_phase4(pool, card_needs, idx, allocs, filled);
        }

        let saved_pool = *pool;
        let saved_len = allocs.len();
        let c = colors[color_idx];
        let max_take = pool[c].min(remaining);
        for take in 0..=max_take {
            let mut new_filled = filled;
            if take > 0 {
                allocs.push(Allocation {
                    target_idx: idx,
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::StageHearts,
                    source_slot: None,
                    wildcard: false,
                    color: c,
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::ColoredSurplus,
                });
                pool[c] -= take;
                new_filled[c] += take;
            }

            let result = Self::try_surplus_compositions(
                pool,
                card_needs,
                idx,
                colors,
                remaining - take,
                color_idx + 1,
                allocs,
                new_filled,
            );
            if result {
                return true;
            }

            // Undo
            *pool = saved_pool;
            allocs.truncate(saved_len);
        }
        false
    }

    /// After Phase 3a choices are made, try Phase 3b (heart00 → heart00 deficit)
    /// and Phase 4 (icon_all → remaining deficits).
    fn try_phase4(
        pool: &mut [u32; 8],
        card_needs: &[CardNeed],
        idx: usize,
        allocs: &mut Vec<Allocation>,
        mut filled: [u32; 8],
    ) -> bool {
        let saved_pool = *pool;
        let saved_len = allocs.len();
        let cn = &card_needs[idx];
        let card_name = &cn.name;
        let need = cn.need;

        // Count all hearts allocated so far (1a + 3a)
        let total_filled_so_far: u32 = filled.iter().sum();
        let total_required: u32 = need.iter().sum();
        let h00_deficit = total_required.saturating_sub(total_filled_so_far);

        // Phase 3b: Heart00 → remaining deficit (no choice, forced)
        if h00_deficit > 0 && pool[0] > 0 {
            let take = pool[0].min(h00_deficit);
            allocs.push(Allocation {
                target_idx: idx,
                target_name: card_name.clone(),
                source_type: SourceType::Stage,
                source_name: SourceName::StageHearts,
                source_slot: None,
                wildcard: false,
                color: 0,
                amount: take,
                is_bonus: false,
                phase: AllocPhase::H00,
            });
            pool[0] -= take;
            filled[0] += take;
        }

        // Now compute all remaining deficits
        let total_filled_now: u32 = filled.iter().sum();
        let total_required: u32 = need.iter().sum();
        let h00_still_needed = total_required.saturating_sub(total_filled_now);

        let mut color_deficits: Vec<(usize, u32)> = Vec::new();
        for c in 1..7 {
            if filled[c] < need[c] {
                color_deficits.push((c, need[c] - filled[c]));
            }
        }
        if h00_still_needed > 0 {
            color_deficits.push((0, h00_still_needed));
        }

        // Phase 4: icon_all → try all distributions to deficits
        let all_count = pool[7];
        if all_count == 0 && color_deficits.is_empty() {
            if Self::card_ok_with_wildcard(filled, need) {
                return Self::bt_search(pool, card_needs, idx + 1, allocs);
            }
        } else if all_count == 0 {
            // No icon_all but deficits exist
            *pool = saved_pool;
            allocs.truncate(saved_len);
            return false;
        } else {
            let deficit_indices: Vec<usize> = {
                let mut v: Vec<usize> = color_deficits.iter().map(|&(c, _)| c).collect();
                v.sort();
                v
            };
            if Self::try_all_distribution(
                pool,
                card_needs,
                idx,
                allocs,
                filled,
                &deficit_indices,
                &color_deficits,
                all_count,
                0,
            ) {
                return true;
            }
        }

        // Undo
        *pool = saved_pool;
        allocs.truncate(saved_len);
        false
    }

    /// Try all distributions of `remaining` icon_all hearts to deficit types starting at `di`.
    fn try_all_distribution(
        pool: &mut [u32; 8],
        card_needs: &[CardNeed],
        idx: usize,
        allocs: &mut Vec<Allocation>,
        filled: [u32; 8],
        deficit_indices: &[usize],
        deficits: &[(usize, u32)],
        remaining: u32,
        di: usize,
    ) -> bool {
        let saved_pool = *pool;
        let saved_len = allocs.len();
        let cn = &card_needs[idx];
        let card_name = &cn.name;

        if di >= deficit_indices.len() {
            if remaining > 0 {
                // icon_all left but no deficits → use for heart00 surplus
                *pool = saved_pool;
                allocs.truncate(saved_len);
                return false;
            }
            if Self::card_ok_with_wildcard(filled, cn.need) {
                return Self::bt_search(pool, card_needs, idx + 1, allocs);
            }
            *pool = saved_pool;
            allocs.truncate(saved_len);
            return false;
        }

        let target_color = deficit_indices[di];
        let deficit_amt = deficits
            .iter()
            .find(|&&(c, _)| c == target_color)
            .map(|&(_, d)| d)
            .unwrap_or(0);

        let max_take = remaining.min(deficit_amt);
        for take in 0..=max_take {
            let mut new_filled = filled;
            if take > 0 {
                let alloc_color = if target_color == 0 { 7 } else { target_color };
                allocs.push(Allocation {
                    target_idx: idx,
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::AllHeartIconAll,
                    source_slot: None,
                    wildcard: target_color == 0,
                    color: alloc_color,
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::AllCleanup,
                });
                pool[7] -= take;
                if target_color == 0 {
                    new_filled[0] += take;
                } else {
                    new_filled[target_color] += take;
                }
            }

            let result = Self::try_all_distribution(
                pool,
                card_needs,
                idx,
                allocs,
                new_filled,
                deficit_indices,
                deficits,
                remaining - take,
                di + 1,
            );
            if result {
                return true;
            }

            *pool = saved_pool;
            allocs.truncate(saved_len);
        }
        false
    }

    /// Check if a single card's requirements are satisfied with its filled array.
    fn card_ok_with_wildcard(filled: [u32; 8], need: [u32; 8]) -> bool {
        let mut wildcard = filled[0] + filled[7];
        let total_filled: u32 = filled.iter().sum();
        let total_required: u32 = need.iter().sum();
        if total_filled < total_required {
            return false;
        }
        if need[0] > 0 {
            let h00_satisfied: u32 = filled[1..7].iter().sum();
            if h00_satisfied + wildcard < need[0] {
                return false;
            }
            wildcard = wildcard.saturating_sub(need[0].saturating_sub(h00_satisfied));
        }
        for idx in 1..7 {
            if filled[idx] < need[idx] {
                let deficit = need[idx] - filled[idx];
                if wildcard >= deficit {
                    wildcard -= deficit;
                } else {
                    return false;
                }
            }
        }
        true
    }

    /// Rule 8.3.14-8.3.16: Check heart requirements, determine live success/failure,
    /// drain resolution zone. Called AFTER the 8.3.13 check timing so hearts granted
    /// by "when you yell" abilities are included in the live success check.
    /// `heart_override`/`heart_modifiers`/`heart_color_multiplier` must come from the
    /// current game state (post-ability-trigger) so ability-granted hearts are counted.
    pub fn check_live_success(
        player: &mut crate::player::Player,
        resolution_zone: &mut crate::zones::ResolutionZone,
        card_db: &CardDatabase,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        heart_override: &HashMap<i16, (HeartColor, u32)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, i32>>,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
        live_card_ids: &[i16],
        _allocations: &[Allocation],
        yell_cards: &[YellCardResult],
        total_blade: u32,
        cheer_icon_count: u32,
        member_contributions: &[MemberContribution],
        total_hearts_arr: &[u32; 8],
        heart_sources: &[HeartSource],
        blade_sources: &[BladeSource],
    ) -> LivePerformanceData {
        // Recompute hearts from current state (includes ability-granted hearts
        // from the 8.3.13 check timing) + yell blade heart hearts.
        let mut owned_hearts = player.stage.get_available_hearts(
            card_db,
            heart_override,
            heart_modifiers,
            heart_color_multiplier,
        );
        for yc in yell_cards {
            for i in 0..8 {
                if yc.blade_hearts[i] > 0 {
                    *owned_hearts
                        .hearts
                        .entry_or_default(HeartColor::from_index(i)) += yc.blade_hearts[i];
                }
            }
        }
        // Recompute allocations from the updated pool.
        let allocations =
            Self::compute_allocations(&owned_hearts, live_card_ids, card_db, need_heart_modifiers);

        let mut per_card_filled: Vec<[u32; 8]> = vec![EMPTY_H8; live_card_ids.len()];
        for alloc in &allocations {
            if alloc.target_idx < per_card_filled.len() {
                per_card_filled[alloc.target_idx][alloc.color] += alloc.amount;
            }
        }

        // Q259: Required heart check is only performed at live success judgment timing.
        // Subsequent changes do NOT retroactively fail a live.
        let any_requirement_failed = live_card_ids.iter().enumerate().any(|(live_idx, &lc_id)| {
            card_db.get_card(lc_id).is_some_and(|card| {
                let nh = match card.need_heart.as_ref() {
                    Some(nh) => {
                        let has_set = need_heart_modifiers
                            .get(&lc_id)
                            .is_some_and(|m| m.values().any(|e| e.set != 0));
                        let mut adjusted = if has_set {
                            BaseHeart {
                                hearts: HeartMap::new(),
                            }
                        } else {
                            nh.clone()
                        };
                        if let Some(card_mods) = need_heart_modifiers.get(&lc_id) {
                            for (color, me) in card_mods {
                                // Q115: Set-to-X applies first, then add/subtract modifiers stack.
                                if me.set != 0 {
                                    adjusted.hearts.insert(*color, me.set as u32);
                                }
                                if me.additive != 0 {
                                    *adjusted.hearts.entry_or_default(*color) =
                                        (adjusted.hearts.get(color).copied().unwrap_or(0) as i32
                                            + me.additive)
                                            .max(0) as u32;
                                }
                            }
                        }
                        adjusted
                    }
                    None => return false,
                };
                if nh.hearts.is_empty() {
                    return false;
                }
                let mut required_arr = EMPTY_H8;
                for (color, needed) in &nh.hearts {
                    required_arr[color.index()] = *needed;
                }
                let filled = per_card_filled[live_idx];
                let mut wildcard = filled[0] + filled[7];
                let mut ok = true;
                let total_filled: u32 = filled.iter().sum();
                let total_required: u32 = required_arr.iter().sum();
                if total_filled < total_required {
                    ok = false;
                }
                if ok && required_arr[0] > 0 {
                    let h00_satisfied: u32 = filled[1..7].iter().sum();
                    if h00_satisfied + wildcard < required_arr[0] {
                        ok = false;
                    } else {
                        wildcard =
                            wildcard.saturating_sub(required_arr[0].saturating_sub(h00_satisfied));
                    }
                }
                if ok {
                    for idx in 1..7 {
                        if filled[idx] < required_arr[idx] {
                            let deficit = required_arr[idx] - filled[idx];
                            if wildcard >= deficit {
                                wildcard -= deficit;
                            } else {
                                ok = false;
                                break;
                            }
                        }
                    }
                }
                !ok
            })
        });
        let moved_live_card_ids: Vec<i16> = if any_requirement_failed {
            log::debug!("[LIVE] Heart requirement not met — sending all live cards to waitroom");
            let moved: Vec<i16> = player.live_card_zone.cards.iter().copied().collect();
            while !player.live_card_zone.cards.is_empty() {
                let card_id = player.live_card_zone.cards.remove(0);
                player.waitroom.cards.push(card_id);
            }
            moved
        } else {
            Vec::new()
        };

        let revealed_ids: Vec<i16> = resolution_zone.cards.iter().copied().collect();
        player.last_resolution_cards = revealed_ids.clone().into();
        // Q41: Yell-revealed cards go to waitroom during live victory determination,
        // after successful cards are placed in the success zone (Rule 8.4.7).
        for card_id in resolution_zone.cards.drain(..) {
            player.waitroom.add_card(card_id);
        }
        let draw_effects_occurred = yell_cards.iter().any(|yc| yc.draw_icons > 0);

        LivePerformanceData {
            yell_count: total_blade,
            note_icons: cheer_icon_count,
            revealed_ids,
            member_contributions: member_contributions.to_vec(),
            yell_cards: yell_cards.to_vec(),
            total_hearts: *total_hearts_arr,
            allocations: allocations.to_vec(),
            heart_sources: heart_sources.to_vec(),
            blade_sources: blade_sources.to_vec(),
            draw_effects_occurred,
            live_card_ids: live_card_ids.to_vec(),
            moved_live_card_ids,
        }
    }
}

// ============== SNAPSHOT BUILDING ==============

/// Process the ability_applications recorded during the live performance
/// and populate MemberContribution ability bonuses, score lines, etc.
pub fn enrich_from_applications(
    member_contributions: &mut Vec<MemberContribution>,
    breakdown: &mut crate::types::Breakdown,
    triggered_abilities: &mut Vec<crate::types::TriggeredAbility>,
    applications: &[crate::types::AbilityApplication],
    card_db: &crate::card::CardDatabase,
) {
    let mut seen = HashSet::<(i16, &ArcStr)>::default();
    for app in applications {
        if let Some(mc) = member_contributions
            .iter_mut()
            .find(|m| m.source_id == app.target_card_id)
        {
            match app.effect_type {
                crate::types::EffectType::HeartBonus => {
                    mc.ability_heart_bonuses.push(crate::types::AbilityBonus {
                        source: if ABILITY_DEBUG.load(Ordering::Relaxed) {
                            let source_name = card_db
                                .get_card(app.source_card_id)
                                .map(|c| c.name.to_string())
                                .unwrap_or_else(|| format!("#{}", app.source_card_id));
                            format!("Ability: {}", source_name).into()
                        } else {
                            crate::types::ArcStr::default()
                        },
                        amount: app.amount.unsigned_abs(),
                        color: app.heart_color,
                        ability_text: app.ability_text.clone().into(),
                    });
                }
                crate::types::EffectType::BladeBonus => {
                    mc.ability_blade_bonuses.push(crate::types::AbilityBonus {
                        source: if ABILITY_DEBUG.load(Ordering::Relaxed) {
                            let source_name = card_db
                                .get_card(app.source_card_id)
                                .map(|c| c.name.to_string())
                                .unwrap_or_else(|| format!("#{}", app.source_card_id));
                            format!("Ability: {}", source_name).into()
                        } else {
                            crate::types::ArcStr::default()
                        },
                        amount: app.amount.unsigned_abs(),
                        color: app.heart_color,
                        ability_text: app.ability_text.clone().into(),
                    });
                }
                _ => {}
            }
        }
        match app.effect_type {
            crate::types::EffectType::ScoreBonus | crate::types::EffectType::ScoreSet => {
                breakdown.scores.push(crate::types::ScoreLine {
                    source: if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        app.ability_text.to_string()
                    } else {
                        String::new()
                    },
                    value: app.amount.unsigned_abs(),
                });
            }
            crate::types::EffectType::Transform => {
                breakdown.transforms.push(crate::types::EffectEntry {
                    source: if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        app.ability_text.to_string()
                    } else {
                        String::new()
                    },
                    desc: if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        format!(
                            "All hearts become type {}",
                            app.heart_color.map_or(0, |c| c)
                        )
                    } else {
                        String::new()
                    },
                    value: String::new(),
                });
            }
            _ => {}
        }
        let key = (app.source_card_id, &app.ability_text);
        if seen.insert(key) && !app.ability_text.is_empty() {
            let card = card_db.get_card(app.source_card_id);
            triggered_abilities.push(crate::types::TriggeredAbility {
                source_card_id: app.source_card_id,
                name: if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    format!("Ability #{}", triggered_abilities.len() + 1)
                } else {
                    String::new()
                },
                card_name: card
                    .map(|c| crate::types::ArcStr::from(c.name.as_ref()))
                    .unwrap_or_default(),
                effect_text: app.ability_text.clone().into(),
                condition_text: None,
                is_public: true,
            });
        }
    }
}

pub fn build_snapshot(
    turn: u32,
    player_id: &str,
    perf: &LivePerformanceData,
    card_db: &CardDatabase,
    note_icons: u32,
    performance_need_heart_modifiers: &[(i16, HeartColor, ModifierEntry)],
) -> crate::types::PerformanceSnapshot {
    let mut lives = Vec::new();
    // Use perf.live_card_ids (captured before heart check cleared the zone)
    for &lc_id in &perf.live_card_ids {
        let score = card_db.get_card(lc_id).map(|c| c.get_score()).unwrap_or(0);
        let card_no = card_db
            .get_card(lc_id)
            .map(|c| crate::types::ArcStr::from(c.card_no.as_ref()))
            .unwrap_or_default();
        lives.push(crate::types::LiveCardResult {
            passed: false,
            score,
            base_score: score,
            spare: EMPTY_H8,
            required: EMPTY_H8,
            filled: EMPTY_H8,
            adjustments: Vec::new(),
            card_id: lc_id,
            card_no,
        });
    }

    crate::types::PerformanceSnapshot {
        turn,
        player_id: player_id.to_string(),
        lives,
        member_contributions: perf.member_contributions.clone(),
        yell_cards: perf.yell_cards.clone(),
        total_hearts: perf.total_hearts,
        total_score: 0,
        success: false,
        note_icons,
        yell_count: perf.yell_count,
        breakdown: crate::types::Breakdown {
            hearts: perf.heart_sources.clone(),
            blades: perf.blade_sources.clone(),
            allocations: perf.allocations.clone(),
            requirements: Vec::new(),
            transforms: Vec::new(),
            scores: Vec::new(),
        },
        triggered_abilities: {
            let mut seen = HashSet::<&ArcStr>::default();
            let mut tas = Vec::new();
            for mc in &perf.member_contributions {
                for ab in mc
                    .ability_heart_bonuses
                    .iter()
                    .chain(mc.ability_blade_bonuses.iter())
                {
                    if !seen.insert(&ab.ability_text) {
                        continue;
                    }
                    let card = card_db.get_card(mc.source_id);
                    tas.push(crate::types::TriggeredAbility {
                        source_card_id: mc.source_id,
                        name: ab.source.to_string(),
                        card_name: card
                            .map(|c| crate::types::ArcStr::from(c.name.as_ref()))
                            .unwrap_or_default(),
                        effect_text: ab.ability_text.clone(),
                        condition_text: None,
                        is_public: true,
                    });
                }
            }
            if perf.draw_effects_occurred {
                tas.push(crate::types::TriggeredAbility {
                    source_card_id: -1,
                    name: "Draw Effect".to_string(),
                    card_name: crate::types::ArcStr::default(),
                    effect_text: "カードを引く効果が発動しました".to_string().into(),
                    condition_text: None,
                    is_public: true,
                });
            }
            tas
        },
        surplus_hearts: [0; 8],
        revealed_ids: perf.revealed_ids.clone(),
        p0_wins: false,
        p1_wins: false,
        performance_need_heart_modifiers: performance_need_heart_modifiers.to_vec(),
    }
}

fn card_name_by_no(card_db: &CardDatabase, card_no: &str) -> String {
    if card_no.is_empty() {
        return "?".to_string();
    }
    card_db
        .get_card_by_no(card_no)
        .map(|c| c.name.to_string())
        .unwrap_or_else(|| card_no.to_string().into())
}

fn heart_color_debug_name(color: &HeartColor) -> &'static str {
    match color {
        HeartColor::Heart00 => "heart00",
        HeartColor::Heart01 => "heart01",
        HeartColor::Heart02 => "heart02",
        HeartColor::Heart03 => "heart03",
        HeartColor::Heart04 => "heart04",
        HeartColor::Heart05 => "heart05",
        HeartColor::Heart06 => "heart06",
        HeartColor::BAll => "b_all",
        HeartColor::Draw => "draw",
        HeartColor::Score => "score",
        HeartColor::All => "all",
    }
}

fn fmt_player_id(id: &str) -> String {
    let mut digits = id.chars().filter(|c| c.is_ascii_digit());
    match digits.next() {
        None => "?".to_string(),
        Some(first) => {
            let mut s = String::with_capacity(8);
            s.push('P');
            s.push(first);
            s.extend(digits);
            s
        }
    }
}

fn fmt_hearts(arr: &[u32; 8]) -> String {
    arr.iter()
        .enumerate()
        .filter(|(_, &v)| v > 0)
        .map(|(i, v)| {
            format!(
                "{}:{}",
                crate::card::HeartColor::from_index(i).short_label(),
                v
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn fmt_heart_vec(arr: &[u32; 8]) -> String {
    arr.iter()
        .enumerate()
        .map(|(i, v)| {
            format!(
                "{}:{}",
                crate::card::HeartColor::from_index(i).short_label(),
                v
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn snapshot_to_rule_log(
    snap: &crate::types::PerformanceSnapshot,
    card_db: &CardDatabase,
) -> Vec<String> {
    let player = fmt_player_id(&snap.player_id);
    let heart_labels = ["h00", "h01", "h02", "h03", "h04", "h05", "h06"];
    let mut lines = Vec::new();

    lines.push(format!("[Turn {}] ── {} Performance ──", snap.turn, player));

    // ── Stage members ──
    let mut _stage_total_blades = 0u32;
    let mut stage_total_hearts = [0u32; 8];
    for mc in &snap.member_contributions {
        let name = card_name_by_no(card_db, &mc.card_no);
        let total_blade = mc.base_blades + mc.bonus_blades;
        _stage_total_blades += total_blade;
        for i in 0..7 {
            stage_total_hearts[i] += mc.base_hearts[i] + mc.bonus_hearts[i];
        }

        // Base hearts
        let base_h = fmt_hearts(&mc.base_hearts);
        let blade_str = if mc.bonus_blades > 0 {
            format!("★{} (+{} from abilities)", total_blade, mc.bonus_blades)
        } else {
            format!("★{}", total_blade)
        };
        if base_h.is_empty() {
            lines.push(format!("  Stage: {}  {}", name, blade_str));
        } else {
            lines.push(format!("  Stage: {}  {}  ♥[{}]", name, blade_str, base_h));
        }

        // Ability bonuses
        for ab in &mc.ability_heart_bonuses {
            let color_str = ab
                .color
                .map(|c| heart_labels[c].to_string())
                .unwrap_or_default();
            lines.push(format!(
                "    Ability: {}  ♥{}+{}",
                ab.source, color_str, ab.amount
            ));
        }
        for ab in &mc.ability_blade_bonuses {
            lines.push(format!("    Ability: {}  ★+{}", ab.source, ab.amount));
        }
    }

    // ── Yell cards ──
    let mut yell_total_hearts = [0u32; 8];
    if snap.yell_count > 0 {
        lines.push(format!("  Yell ({} cards):", snap.yell_count));
        for yc in &snap.yell_cards {
            let name = card_name_by_no(card_db, &yc.card_no);
            let bh = fmt_hearts(&yc.blade_hearts);
            for i in 0..8 {
                yell_total_hearts[i] += yc.blade_hearts[i];
            }
            if bh.is_empty() {
                lines.push(format!(
                    "    {}  ♪{}  ⎋{}",
                    name, yc.note_icons, yc.draw_icons
                ));
            } else {
                lines.push(format!(
                    "    {}  ♥[{}]  ♪{}  ⎋{}",
                    name, bh, yc.note_icons, yc.draw_icons
                ));
            }
        }
    }

    // ── Total hearts breakdown ──
    lines.push("  Hearts breakdown:".to_string());
    lines.push(format!(
        "    Base hearts:      [{}]",
        fmt_heart_vec(&stage_total_hearts)
    ));
    if yell_total_hearts.iter().any(|&v| v > 0) {
        lines.push(format!(
            "    Yell hearts:      [{}]",
            fmt_heart_vec(&yell_total_hearts)
        ));
    }
    if !snap.breakdown.hearts.is_empty() {
        for hs in &snap.breakdown.hearts {
            let hv = fmt_heart_vec(&hs.value);
            if hv.chars().any(|c| {
                c != '0'
                    && c != ':'
                    && c != 'h'
                    && c != '0'
                    && c != '1'
                    && c != '2'
                    && c != '3'
                    && c != '4'
                    && c != '5'
                    && c != '6'
            }) {
                // has non-zero values
                lines.push(format!("    {}: [{}]", hs.source, hv));
            }
        }
    }
    let total_h = fmt_heart_vec(&snap.total_hearts);
    lines.push(format!("    Total hearts:     [{}]", total_h));

    // ── Live cards ──
    for live in &snap.lives {
        let name = card_name_by_no(card_db, &live.card_no);
        let need_h = fmt_hearts(&live.required);
        let filled_h = fmt_hearts(&live.filled);
        let spare_h = fmt_hearts(&live.spare);
        let result = if live.passed { "PASS" } else { "FAIL" };
        lines.push(format!(
            "  Live: {}  need[{}]  filled[{}]  spare[{}]  score +{}  → {}",
            name, need_h, filled_h, spare_h, live.score, result
        ));
    }

    // ── Score ──
    let pass_str = if snap.success { "PASS" } else { "FAIL" };
    lines.push(format!("  Score: {}  {}", snap.total_score, pass_str));

    lines
}
