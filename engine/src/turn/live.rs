use crate::ability::debug::ABILITY_DEBUG;
use crate::ability::enums::Zone;
use crate::card::{BaseHeart, BladeColor, CardDatabase, HeartColor};
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

const EMPTY_H8: [u8; 8] = [0u8; 8];

/// Effective heart need for a live card during allocation.
struct CardNeed {
    name: crate::types::ArcStr,
    need: [u8; 8],
}

/// Icon tallies contributed by one yell-revealed card (rule 8.3.12).
pub(crate) struct YellIconOutcome {
    pub blade_hearts: [u8; 8],
    pub note_icons: u8,
    pub draw_icons: u8,
}

/// Rule 8.3.11 recolor mapping for set_blade_type effects.
pub(crate) fn blade_color_to_heart(bc: BladeColor) -> HeartColor {
    match bc {
        BladeColor::Peach => HeartColor::Heart01,
        BladeColor::Red => HeartColor::Heart02,
        BladeColor::Yellow => HeartColor::Heart03,
        BladeColor::Green => HeartColor::Heart04,
        BladeColor::Blue => HeartColor::Heart05,
        BladeColor::Purple => HeartColor::Heart06,
        // ALL blade = "any one color heart" (rule 2.1.1.3), i.e. icon_all
        // (HeartColor::All, index 7) — NOT colorless Heart00.
        BladeColor::All => HeartColor::All,
    }
}

/// Process the blade-heart and special-heart icons of ONE yell-revealed card.
///
/// Single source of truth shared by the primary yell (`player_perform_live`)
/// and the re-yell rebuild (`execute_performance_phase`). Implements:
/// - rule 8.3.15.1.1: ALL-blade counts as any one color heart (index 7 wildcard);
/// - rule 8.3.12.1: each draw icon yields 1 draw (`draw_icons`; the caller executes it);
/// - rule 8.4.2.1: each score icon adds +1 to the live total (`cheer_count`);
/// - b_heart07: colorless hearts count double but never satisfy specific colors;
/// - set_blade_type recoloring applies to colored blades only (Draw/Score pass through).
///
/// Heart contributions are added to `owned_hearts` and `total_hearts`. Callers
/// that recompute the heart pool afterwards may pass a scratch `owned_hearts`.
pub(crate) fn process_yell_revealed_card_icons(
    card: &crate::card::Card,
    override_color: Option<HeartColor>,
    owned_hearts: &mut BaseHeart,
    total_hearts: &mut [u8; 8],
    cheer_count: &mut u8,
) -> YellIconOutcome {
    let mut bh_arr = EMPTY_H8;
    let mut note_icons = 0u8;
    let mut draw_icons = 0u8;

    if let Some(ref bh) = card.blade_heart {
        for (color, count) in &bh.hearts {
            // b_heart07 mechanic: the key parses to HeartColor::Heart00
            // (colorless), and `b_heart07: N` means 2×N colorless hearts.
            // A colorless heart can ONLY be used to replace heart0
            // requirements — never a specific color (heart01-heart06).
            // The ×2 is applied on the ORIGINAL color (before any
            // set_blade_type recoloring), so a recolored b_heart07 still
            // contributes 2 hearts of the new color.
            let amount = if *color == HeartColor::Heart00 {
                count * 2
            } else {
                *count
            };
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
                *owned_hearts.hearts.entry_or_default(HeartColor::All) += amount;
                bh_arr[7] += amount;
                total_hearts[7] += amount;
            } else if effective_color == HeartColor::Draw {
                draw_icons += amount;
            // Q44: Each score icon revealed during yell adds 1 to total score.
            } else if effective_color == HeartColor::Score {
                note_icons += amount;
                *cheer_count += amount;
            } else {
                let idx = effective_color.index();
                if idx < 8 {
                    *owned_hearts.hearts.entry_or_default(effective_color) += amount;
                    bh_arr[idx] += amount;
                    total_hearts[idx] += amount;
                }
            }
        }
    }

    // Special hearts printed on the revealed card itself (e.g. the ドロー icon
    // on Solitude Rain when it is milled back into the deck and revealed).
    if let Some(ref sh) = card.special_heart {
        for (color, count) in &sh.hearts {
            if *color == HeartColor::Draw {
                draw_icons += count;
            } else if *color == HeartColor::Score {
                note_icons += count;
                *cheer_count += count;
            }
        }
    }

    YellIconOutcome {
        blade_hearts: bh_arr,
        note_icons,
        draw_icons,
    }
}

/// Pre-trigger per-seat live results (docs/TEST_HARDENING_PLAN_2026-08-26.md
/// §4, engine bug #5): ライブ成功時 triggers fire at the TOP of
/// execute_live_victory_determination, but the authoritative verdicts and
/// scores are only computed further down — so conditions like Strawberry
/// Trapper's 「相手が余剰のハートを持たずにライブを成功させていた場合」 saw
/// stale state in every real flow (synthetic-flag tests masked it).
///
/// This records each seat's outcome BEFORE the trigger block runs, using the
/// SAME score formula as the post-trigger totals (extras = 0 at this point,
/// since trigger bonuses land in pX_extra afterwards). The later
/// authoritative writes remain as post-extras truth for display/subsequent
/// turns; opponent_live_success_flow_test pins the two together.
fn record_pretrigger_live_results(
    gs: &mut GameState,
    need_heart_flat: &HashMap<i16, HashMap<crate::card::HeartColor, ModifierEntry>>,
    pre_score_flat: &HashMap<i16, i32>,
) {
    // (won, no_excess) per seat, computed from finalised allocations.
    let mut results = [(false, false); 2];
    let mut recorded = [false; 2];
    for seat_index in 0..2 {
        let pid = if seat_index == 0 {
            gs.player1.id.clone()
        } else {
            gs.player2.id.clone()
        };
        let seat = if seat_index == 0 {
            &gs.player1
        } else {
            &gs.player2
        };
        let Some(snap) = gs
            .performance_snapshots
            .iter()
            .rev()
            .find(|s| s.player_id == pid)
        else {
            continue;
        };
        if snap.lives.is_empty() || seat.live_card_zone.cards.is_empty() {
            continue;
        }
        let mut all_passed = true;
        let mut filled_total = [0u8; 8];
        for (li, l) in snap.lives.iter().enumerate() {
            let Some(card) = gs.card_database.get_card(l.card_id) else {
                all_passed = false;
                break;
            };
            let Some(nh) = card.need_heart.as_ref() else {
                continue;
            };
            let mut filled = [0u8; 8];
            for alloc in &snap.breakdown.allocations {
                if alloc.target_idx == li as u8 {
                    filled[alloc.color as usize] += alloc.amount;
                }
            }
            for c in 0..8 {
                filled_total[c] += filled[c];
            }
            let mut required = [0u8; 8];
            for (color, needed) in &nh.hearts {
                required[color.index()] = *needed;
            }
            if let Some(card_mods) = gs.mods.need_heart_modifiers.get(&l.card_id) {
                for (color, me) in card_mods {
                    if me.set != 0 {
                        required[color.index()] = me.set as u8;
                    }
                }
                for (color, me) in card_mods {
                    if me.additive != 0 {
                        let idx = color.index();
                        let current = required[idx] as i32;
                        required[idx] =
                            crate::constants::saturate_u8(current + me.additive as i32);
                    }
                }
            }
            // Same acceptance rules as the authoritative PASS/FAIL pass:
            // total coverage, then heart0 bucket, then per-color deficits
            // coverable by icon_all. Additive modifiers apply in i32 with a
            // saturating write-back (mirroring the authoritative pass) so a
            // negative additive cannot wrap into a huge u8 requirement.
            let mut icon_all = filled[7];
            let total_filled: u16 = filled.iter().map(|&v| u16::from(v)).sum();
            let total_required: u16 = required.iter().map(|&v| u16::from(v)).sum();
            let mut ok = total_filled >= total_required;
            if ok && required[0] > 0 {
                let any_hearts: u16 =
                    filled[1..7].iter().map(|&v| u16::from(v)).sum::<u16>() + u16::from(filled[0]);
                if any_hearts + u16::from(icon_all) < u16::from(required[0]) {
                    ok = false;
                } else {
                    let used = u16::from(required[0].saturating_sub(any_hearts as u8));
                    icon_all = icon_all.saturating_sub(used as u8);
                }
            }
            if ok {
                for idx in 1..7 {
                    if filled[idx] < required[idx] {
                        let deficit = required[idx] - filled[idx];
                        if icon_all >= deficit {
                            icon_all -= deficit;
                        } else {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            if !ok {
                all_passed = false;
            }
        }
        let no_excess = (0..8).all(|c| {
            snap.total_hearts[c] >= filled_total[c]
                && snap.total_hearts[c] - filled_total[c] == 0
        });
        // Same score formula as the post-trigger totals (extras are zero
        // here by construction — trigger bonuses land in pX_extra after
        // this point).
        let seat = if seat_index == 0 {
            &gs.player1
        } else {
            &gs.player2
        };
        let cheer = if seat_index == 0 {
            gs.player1_cheer_blade_heart_count
        } else {
            gs.player2_cheer_blade_heart_count
        };
        let bonus = if seat_index == 0 {
            gs.mods.p1_constant_total_score_bonus
        } else {
            gs.mods.p2_constant_total_score_bonus
        };
        let pre_score = seat.live_card_zone.calculate_live_score(
            &gs.card_database,
            cheer,
            seat.stage_hearts.as_ref(),
            Some(need_heart_flat),
            Some(pre_score_flat),
            bonus,
        );
        let won = all_passed && pre_score > 0;
        results[seat_index] = (won, no_excess);
        recorded[seat_index] = true;
    }
    // Seats that performed no lives are LEFT untouched: nothing was learned
    // about them, and synthetic/legacy flows may have armed them explicitly.
    if recorded[0] {
        gs.p1_live_success_this_turn = results[0].0;
        gs.p1_live_success_no_excess = results[0].0 && results[0].1;
    }
    if recorded[1] {
        gs.p2_live_success_this_turn = results[1].0;
        gs.p2_live_success_no_excess = results[1].0 && results[1].1;
    }
    // Diagnostic detail: why each seat was skipped/written.
    for seat_index in 0..2 {
        let pid = if seat_index == 0 {
            gs.player1.id.clone()
        } else {
            gs.player2.id.clone()
        };
        let seat = if seat_index == 0 {
            &gs.player1
        } else {
            &gs.player2
        };
        let info = gs
            .performance_snapshots
            .iter()
            .rev()
            .find(|s| s.player_id == pid)
            .map(|s| (s.lives.len(), s.total_hearts.to_vec()))
            .unwrap_or((usize::MAX, vec![]));
        log::debug!(
            "[EARLY_SEAT_DETAIL] seat={} lives={:?} zone_cards={} total_hearts={:?}",
            pid,
            info,
            seat.live_card_zone.cards.len(),
            info.1
        );
    }
    log::debug!(
        "[EARLY_SEAT] p1(won={},no_excess={}) p2(won={},no_excess={})",
        results[0].0,
        results[0].1,
        results[1].0,
        results[1].1
    );
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

    fn drain_pending_live_success_choices(
        game_state: &mut GameState,
        p1_id: &str,
        p2_id: &str,
    ) -> bool {
        game_state.process_pending_auto_abilities(p1_id);
        if game_state.has_pending_choice() {
            log::debug!("[LIVE] pending choice after draining p1 live_success queue — early return");
            return true;
        }
        game_state.process_pending_auto_abilities(p2_id);
        if game_state.has_pending_choice() {
            log::debug!("[LIVE] pending choice after draining p2 live_success queue — early return");
            return true;
        }
        false
    }

    fn compute_pregame_scores(
        game_state: &GameState,
        need_heart_flat: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        pre_score_flat: &HashMap<i16, i32>,
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
        log::debug!("[LIVE_SCORE] p1={} p2={} extras p1={} p2={}", p1, p2, p1_extra, p2_extra);
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
                .map_or(false, |s| !s.lives.is_empty() && s.lives.iter().all(|l| l.passed));
        let p2_all = p2_has
            && game_state
                .performance_snapshots
                .iter()
                .rev()
                .find(|s| s.player_id == p2_id)
                .map_or(false, |s| !s.lives.is_empty() && s.lives.iter().all(|l| l.passed));
        if ABILITY_DEBUG.load(Ordering::Relaxed) {
            log::debug!("[LIVE-DBG] === VICTORY DETERMINATION ===");
            log::debug!(
                "[LIVE-DBG] P1 score={} has={} all_passed={} zone={:?}",
                p1_score, p1_has, p1_all, game_state.player1.live_card_zone.cards
            );
            log::debug!(
                "[LIVE-DBG] P2 score={} has={} all_passed={} zone={:?}",
                p2_score, p2_has, p2_all, game_state.player2.live_card_zone.cards
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
        log::debug!("[LIVE_WINNERS] p1_won={} p2_won={} p1_all={} p2_all={}", res.0, res.1, p1_all, p2_all);
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
                let Some(card) = game_state.card_database.get_card(lc_id).cloned() else {
                    continue;
                };
                snap.lives[i].card_id = lc_id;
                snap.lives[i].card_no = crate::types::ArcStr::from(card.card_no.as_ref());
                if card.need_heart.is_none() {
                    snap.lives[i].passed = true;
                    let base_score = card.get_score() as i32;
                    let set_score = game_state.mods.get_score_set_modifier(lc_id);
                    let additive = game_state.mods.get_score_modifier(lc_id) - set_score;
                    let effective_base = if set_score != 0 { set_score } else { base_score };
                    snap.lives[i].score = crate::constants::saturate_u8(effective_base + additive);
                    continue;
                }
                let nh = card.need_heart.as_ref().unwrap();
                let mut filled = EMPTY_H8;
                for alloc in &snap.breakdown.allocations {
                    if alloc.target_idx == i as u8 {
                        filled[alloc.color as usize] += alloc.amount;
                    }
                }
                if ABILITY_DEBUG.load(Ordering::Relaxed) {
                    log::debug!("[LIVE-DBG] live[{}] card={} filled_from_allocs={:?}", i, card.card_no, filled);
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
                    log::debug!("[LIVE-DBG] live[{}] required (via pipeline)={:?}", i, required_arr);
                }
                let passed = {
                    let mut icon_all = filled[7];
                    let mut ok = true;
                    let total_filled: u8 = filled.iter().sum();
                    let total_required: u8 = required_arr.iter().sum();
                    if ABILITY_DEBUG.load(Ordering::Relaxed) {
                        log::debug!(
                            "[LIVE-DBG] live[{}] total_filled={} total_required={} icon_all={}",
                            i, total_filled, total_required, icon_all
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
                        for idx in 1..7 {
                            if filled[idx] < required_arr[idx] {
                                let deficit = required_arr[idx] - filled[idx];
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
                            i, ok, filled, required_arr
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
                                value: total as i16,
                                color: color.index() as u8,
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
                let base_score = card.get_score() as i32;
                let set_score = game_state.mods.get_score_set_modifier(lc_id);
                let additive = game_state.mods.get_score_modifier(lc_id) - set_score;
                let effective_base = if set_score != 0 { set_score } else { base_score };
                snap.lives[i].score = crate::constants::saturate_u8(effective_base + additive);
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
        p2_id: &str,
    ) {
        for snap in game_state.performance_snapshots.iter_mut() {
            let mut cumulative_used = EMPTY_H8;
            for i in 0..snap.lives.len() {
                for alloc in &snap.breakdown.allocations {
                    if alloc.target_idx == i as u8 {
                        let source_idx = match alloc.phase {
                            crate::types::AllocPhase::H00Wild | crate::types::AllocPhase::Wildcard => 0,
                            crate::types::AllocPhase::AllWild
                            | crate::types::AllocPhase::CAll
                            | crate::types::AllocPhase::AllCleanup => 7,
                            _ => alloc.color as usize,
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
                    log::debug!("[LIVE-DBG] player={} live_card_zone empty → total_score forced to 0", snap.player_id);
                }
                snap.total_score = 0;
            }
            snap.success = snap.lives.iter().all(|l| l.passed) && snap.total_score > 0;
            if ABILITY_DEBUG.load(Ordering::Relaxed) {
                log::debug!("[LIVE-DBG] player={} SUCCESS={} total_score={} all_passed={}", snap.player_id, snap.success, snap.total_score, snap.lives.iter().all(|l| l.passed));
            }
            snap.base_score_total = snap.lives.iter().filter(|l| l.passed).map(|l| l.score).sum();
            snap.card_bonus_total = snap.lives.iter().filter(|l| l.passed).map(|l| l.score.saturating_sub(l.base_score)).sum();
            for mc in &mut snap.member_contributions {
                let mut ability_per_color = [0u8; 8];
                for ab in &mc.ability_heart_bonuses {
                    if let Some(color_idx) = ab.color {
                        if color_idx < 8 {
                            ability_per_color[color_idx as usize] += ab.amount;
                        }
                    }
                }
                for i in 0..8 {
                    mc.transform_delta[i] = mc.bonus_hearts[i].saturating_sub(ability_per_color[i]);
                }
            }
        }
        log::debug!("[FINALIZE_SNAPSHOT] p1_won={} p2_won={} p1_score={} p2_score={}", p1_won, p2_won, p1_score, p2_score);
    }

    fn revert_live_success_score_modifiers(game_state: &mut GameState, pre_score_flat: &HashMap<i16, i32>) {
        let post: HashMap<i16, i32> = game_state.mods.score_modifiers.iter().map(|(&k, e)| (k, e.total())).collect();
        for (&cid, post_total) in &post {
            let pre = pre_score_flat.get(&cid).copied().unwrap_or(0);
            let delta = post_total - pre;
            if delta != 0 {
                game_state.mods.add_score_modifier(cid, -delta as i16);
            }
        }
        for (&cid, &pre_total) in pre_score_flat {
            if !post.contains_key(&cid) {
                game_state.mods.set_score_modifier(cid, pre_total as i16);
            }
        }
        log::debug!("[REVERT_SCORE] reverted {} late score modifiers", post.len());
    }

    fn process_delayed_gained_effects(game_state: &mut GameState) {
        if game_state.delayed_gained_effects.is_empty() {
            return;
        }
        let saved_revealed = core::mem::take(&mut game_state.revealed_cards);
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
                let alt_cond = gained.compound.alternative_condition.as_ref();
                let base_cond = gained.condition.as_ref();
                let alt_met = alt_cond.is_some_and(|c| ctx.evaluate_condition(c));
                let base_met = base_cond.is_some_and(|c| ctx.evaluate_condition(c));
                if alt_met || base_met {
                    let alt_eff = gained.alternative_effect_any();
                    let prim_eff = gained.compound.primary_effect.as_ref().map(|b| &**b);
                    let effect_to_apply = if alt_met { alt_eff } else { prim_eff };
                    if let Some(apply) = effect_to_apply {
                        let mut resolver = AbilityResolver::new(game_state.card_database.clone(), Some(*card_id));
                        resolver.activating_card_id = Some(*card_id);
                        let _ = resolver.execute_effect(game_state, apply);
                    }
                }
            }
        }
        game_state.revealed_cards = saved_revealed;
        log::debug!("[DELAYED_GAINED] processed {} delayed effects", delayed.len());
    }

    fn merge_late_score_apps(game_state: &mut GameState, p1_id: &str, p2_id: &str) {
        let late_apps = core::mem::take(&mut game_state.ability_applications);
        if late_apps.is_empty() {
            return;
        }
        let p1_cards = &game_state.player1.live_card_zone.cards.clone();
        let p2_cards = &game_state.player2.live_card_zone.cards.clone();
        for snap in game_state.performance_snapshots.iter_mut() {
            let player_cards = if snap.player_id == p1_id { p1_cards } else { p2_cards };
            for app in &late_apps {
                if (app.effect_type == crate::types::EffectType::ScoreBonus || app.effect_type == crate::types::EffectType::ScoreSet) && player_cards.contains(&app.target_card_id) {
                    snap.breakdown.scores.push(crate::types::ScoreLine { source: app.ability_text.to_string(), value: app.amount.unsigned_abs() as u8 });
                }
            }
        }
        log::debug!("[LATE_SCORE] merged {} late score apps", late_apps.len());
    }

    fn compute_surplus_and_flags(game_state: &mut GameState, p1_won: bool, p2_won: bool, p1_id: &str, p2_id: &str) {
        let mut p2_surplus = 0u8;
        let mut p1_surplus = 0u8;
        for snap in &mut game_state.performance_snapshots {
            let total_available: u8 = snap.total_hearts.iter().sum();
            let total_filled: u8 = snap.lives.iter().flat_map(|l| l.filled.iter()).sum();
            let surplus = total_available.saturating_sub(total_filled);
            let mut per_color_surplus = [0u8; 8];
            for color in 0..8 {
                let total_color = snap.total_hearts[color];
                let filled_color: u8 = snap.lives.iter().map(|l| l.filled[color]).sum();
                per_color_surplus[color] = total_color.saturating_sub(filled_color);
            }
            snap.surplus_hearts = per_color_surplus;
            log::debug!("[SURPLUS] player={} total_avail={} total_filled={} surplus={} per_color={:?}", snap.player_id, total_available, total_filled, surplus, per_color_surplus);
            if snap.player_id == p2_id {
                p2_surplus = surplus;
            } else {
                p1_surplus = surplus;
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
        log::debug!("[SURPLUS_FLAGS] p1_surplus={} p2_surplus={} p1_won={} p2_won={}", p1_surplus, p2_surplus, p1_won, p2_won);
    }

    fn apply_deferred_reyell(game_state: &mut GameState) {
        // Rule 8.3.13.1 corrective path
        let Some(rb) = game_state.pending_reyell_rebuild.take() else {
            return;
        };
        let is_p1 = rb.owner == game_state.player1.id;
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
        if is_p1 {
            game_state.player1_cheer_blade_heart_count = rb.note_icons;
        } else {
            game_state.player2_cheer_blade_heart_count = rb.note_icons;
        }
        game_state.re_yell_occurred = false;
        log::debug!("[REYELL_DEFERRED] applied pending rebuild for owner={}", rb.owner);
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

    pub fn execute_live_victory_determination(game_state: &mut GameState) {
        Self::apply_deferred_reyell(game_state);

        game_state.evaluate_success_zone_constant_abilities();
        game_state.restore_performance_need_heart_modifiers();

        Self::rebuild_stage_hearts_with_yell(game_state);

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

        // BUG#5: record per-seat outcomes BEFORE LiveSuccess triggers fire
        // (Q36: they resolve at determination timing and must see both
        // performances' results).
        record_pretrigger_live_results(
            game_state,
            &need_heart_flat,
            &pre_score_flat,
        );

        // Q48: A live can be won even with total score 0 or less
        // (score comparison determines the winner regardless of absolute value).
        let p1_extra: u8;
        let p2_extra: u8;
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
                    let total_hearts: u8 = snap.total_hearts.iter().sum();
                    let player = if snap.player_id == player1_id {
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
                    // Compute per-color surplus NOW, before LiveSuccess abilities fire.
                    // Allocation is already finalised (performance phase completed).
                    // snap.surplus_hearts is read by color-filtered surplus conditions
                    // (e.g. La Bella Patria heart04 >= 1, Q174).
                    // NOTE: snap.lives[i].filled is NOT yet populated at this point —
                    // that happens later in this function. Derive filled from
                    // snap.breakdown.allocations, which are already finalised.
                    let mut per_color = [0u8; 8];
                    {
                        let mut filled_per_color = [0u8; 8];
                        for alloc in &snap.breakdown.allocations {
                            if alloc.color < 8 {
                                filled_per_color[alloc.color as usize] = filled_per_color
                                    [alloc.color as usize]
                                    .saturating_add(alloc.amount);
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
                p1_extra = crate::constants::saturate_u8(Self::score_delta_since(
                    &score_cur,
                    &pre_score_flat,
                    &game_state.player1.live_card_zone.cards,
                ));
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
            p2_extra = crate::constants::saturate_u8(Self::score_delta_since(
                &score_cur2,
                &pre_score_flat,
                &game_state.player2.live_card_zone.cards,
            ));
            game_state.live_success_p2_extra = p2_extra;
        }

        // Drain any remaining LiveSuccess auto-abilities (choice-gated re-entries)
        if Self::drain_pending_live_success_choices(game_state, &player1_id, &player2_id) {
            return;
        }

        // A4: centralised scoring + pass/fail uses stats_pipeline helpers internally
        let (player1_score, player2_score) =
            Self::compute_pregame_scores(game_state, &need_heart_flat, &pre_score_flat, p1_extra, p2_extra);
        Self::populate_live_verdicts(game_state);
        // NOTE: populate_live_verdicts must run before victory determination so
        // snap.lives[i].passed is populated (Q47/Q48). Removed duplicated inline
        // pass/fail block here — the extracted helper above is now the single source.

        let (player1_won, player2_won) = Self::determine_winners(
            game_state,
            &player1_id,
            &player2_id,
            player1_score,
            player2_score,
        );

        Self::finalize_snapshot_fields(game_state, player1_won, player2_won, player1_score, player2_score, &player1_id, &player2_id);
        Self::revert_live_success_score_modifiers(game_state, &pre_score_flat);
        Self::process_delayed_gained_effects(game_state);
        Self::merge_late_score_apps(game_state, &player1_id, &player2_id);
        Self::compute_surplus_and_flags(game_state, player1_won, player2_won, &player1_id, &player2_id);

        // Push performance summary to rule log

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

        // NOTE: cannot-place restrictions (メビウスループ PL!S-pb1-022-L) are NOT
        // pre-discarded here. Their conditions (「合計スコアが同じ場合」) are
        // evaluated when the ライブ成功時 ability resolves: on a tie it pushes a
        // dynamic prohibition_effects entry and move_live_to_success_and_
        // handle_wins routes the card to the waitroom via can_place_card_in_zone;
        // untied, no prohibition exists and the winner places normally.
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
                    verdict,
                    p1_sum,
                    p1_now,
                    p1_added as u8,
                    p2_sum,
                    p2_now,
                    p2_added as u8
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
        heart_override: &HashMap<i16, (HeartColor, u8)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        blade_type_modifiers: &HashMap<i16, BladeColor>,
        orientation_modifiers: &HashMap<i16, crate::core::game_modifiers::CardOrientation>,
        need_heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
        heart_copy: &HashMap<i16, i16>,
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

        // A1+A2: Use stats_pipeline for heart+blade layering (single source of truth)
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
                slot: i as u8,
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

        // Q32/Rule 8.3.6: If the live card zone is empty, no yell, no live card processing.
        // But member contributions and stage hearts are still returned.
        if player.live_card_zone.cards.is_empty() {
            let mut total_hearts_arr = EMPTY_H8;
            for mc in &member_contributions {
                for c in 0..8 {
                    total_hearts_arr[c] += mc.base_hearts[c] + mc.bonus_hearts[c];
                }
            }
            let total_blade: u8 = member_contributions
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
        let from_bottom = player.yell_from_bottom;
        for _ in 0..total_blade {
            if player.main_deck.cards.is_empty() && !player.waitroom.cards.is_empty() {
                player.refresh();
            }
            // G8: 恋になりたいAQUARIUM makes the yell reveal from the deck bottom.
            let card_id = if from_bottom {
                player.main_deck.draw_bottom()
            } else {
                player.main_deck.draw()
            };
            if let Some(card_id) = card_id {
                resolution_zone.cards.push(card_id);
            }
        }

        // Compute owned hearts from stage
        let mut owned_hearts = player.stage.get_available_hearts(
            card_db,
            heart_override,
            heart_modifiers,
            heart_color_multiplier,
            heart_copy,
        );

        let blade_to_heart = blade_color_to_heart;
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
        let mut cheer_icon_count = 0u8;
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
        let mut total_draw_icons = 0u8;

        for card_id in &resolution_zone.cards {
            if let Some(card) = card_db.get_card(*card_id) {
                let outcome = process_yell_revealed_card_icons(
                    card,
                    override_color,
                    &mut owned_hearts,
                    &mut total_hearts_arr,
                    &mut cheer_icon_count,
                );
                total_draw_icons += outcome.draw_icons;

                yell_cards.push(YellCardResult {
                    card_id: *card_id,
                    blade_hearts: outcome.blade_hearts,
                    note_icons: outcome.note_icons,
                    draw_icons: outcome.draw_icons,
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

        // NOTE: Special hearts on cards sitting in the live_card_zone do NOT
        // apply here. Both the draw note (rule 8.3.12.1) and the score note
        // (rule 8.4.2.1) are scoped to icons revealed by the yell
        // (「エールで出た」) — i.e. cards in the resolution zone only. A live
        // card in the live zone contributes its icon only when an effect has
        // put it into the deck and the yell reveals it.

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
    ///   3a_colored_surplus — leftover colored hearts → Heart00 req (demand-aware:
    ///                        prefers colors with most surplus vs future demand)
    ///   3b_h00           — Heart00 (COLORLESS, e.g. from b_heart07) → Heart00 req.
    ///                      A colorless heart can NEVER fill a specific color req —
    ///                      it only counts toward the heart0/total bucket
    ///                      (rule 2.1.1.2 / 2.11.3).
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
        let mut pool = [0u8; 8];
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
                let mut need = [0u8; 8];
                if let Some(ref nh) = card.need_heart {
                    // Start from the card's base requirements for every color.
                    for (color, count) in &nh.hearts {
                        need[color.index()] = *count;
                    }
                    if let Some(card_mods) = need_heart_modifiers.get(&lc_id) {
                        // Q115/Q127: Set-to-X applies first (per-color), then additive stacks.
                        // A set modifier on one color does NOT erase other colors' requirements.
                        for (color, me) in card_mods {
                            if me.set != 0 {
                                let idx = color.index();
                                need[idx] = me.set as u8;
                            }
                        }
                        for (color, me) in card_mods {
                            if me.additive != 0 {
                                let idx = color.index();
                                let current = need[idx] as i32;
                                need[idx] = crate::constants::saturate_u8(current + me.additive as i32);
                            }
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
    fn compute_future_demand(card_needs: &[CardNeed]) -> Vec<[u8; 8]> {
        let n = card_needs.len();
        let mut demand = vec![[0u8; 8]; n];
        let mut running = [0u8; 8];
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
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        future_demand: &[[u8; 8]],
    ) -> Vec<Allocation> {
        let mut allocs = Vec::new();
        for (live_idx, cn) in card_needs.iter().enumerate() {
            let need = cn.need;
            // Track per-color totals for this card (direct + wildcard already assigned)
            let mut filled = [0u8; 8];
            let card_name = &cn.name;

            // Phase 1a: matching colored hearts → specific color req
            for c in 1..7 {
                if need[c] > 0 && pool[c] > 0 {
                    let take = pool[c].min(need[c]);
                    allocs.push(Allocation {
                        target_idx: live_idx as u8,
                        target_name: card_name.clone(),
                        source_type: SourceType::Stage,
                        source_name: SourceName::StageHearts,
                        source_slot: None,
                        wildcard: false,
                        color: c as u8,
                        amount: take,
                        is_bonus: false,
                        phase: AllocPhase::Colored,
                    });
                    pool[c] -= take;
                    filled[c] += take;
                }
            }

            // NOTE: There is deliberately NO phase where Heart00 (COLORLESS)
            // hearts fill a specific color requirement. A colorless heart
            // (e.g. from b_heart07) can ONLY replace heart0 requirements
            // (rule 2.1.1.2), never a heart01-heart06 note (rule 2.11.3).

            // Phase 3a: total remaining deficit = total_required - total_filled_so_far.
            // Need[0] is the "any" portion, but the total must also be met.
            let total_filled_so_far: u8 = filled.iter().sum();
            let total_required: u8 = need.iter().sum();
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
                let mut filled_h00 = 0u8;
                for &c in &surplus_colors {
                    if filled_h00 >= h00_deficit {
                        break;
                    }
                    if pool[c] > 0 {
                        let take = pool[c].min(h00_deficit - filled_h00);
                        allocs.push(Allocation {
                            target_idx: live_idx as u8,
                            target_name: card_name.clone(),
                            source_type: SourceType::Stage,
                            source_name: SourceName::StageHearts,
                            source_slot: None,
                            wildcard: false,
                            color: c as u8,
                            amount: take,
                            is_bonus: false,
                            phase: AllocPhase::ColoredSurplus,
                        });
                        pool[c] -= take;
                        filled_h00 += take;
                        filled[c] += take;
                    }
                }

                // Phase 3b: Heart00 (COLORLESS) → remaining Heart00 deficit.
                // Colorless hearts (e.g. b_heart07) go ONLY into the heart0
                // bucket — never a specific color.
                if filled_h00 < h00_deficit && pool[0] > 0 {
                    let take = pool[0].min(h00_deficit - filled_h00);
                    allocs.push(Allocation {
                        target_idx: live_idx as u8,
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
                            target_idx: live_idx as u8,
                            target_name: card_name.clone(),
                            source_type: SourceType::Stage,
                            source_name: SourceName::AllHeartIconAll,
                            source_slot: None,
                            wildcard: true,
                            color: c as u8,
                            amount: take,
                            is_bonus: false,
                            phase: AllocPhase::AllCleanup,
                        });
                        pool[7] -= take;
                        filled[c] += take;
                    }
                }
                // Remaining icon_all → heart00 deficit
                let total_colored: u8 = filled[1..7].iter().sum();
                let h00_remaining = need[0].saturating_sub(total_colored);
                if h00_remaining > 0 && pool[7] > 0 {
                    // Also include any previous filled[0] from Phase 3b
                    let already_filled_h00 = filled[0];
                    let h00_still_needed = h00_remaining.saturating_sub(already_filled_h00);
                    if h00_still_needed > 0 && pool[7] > 0 {
                        let take = pool[7].min(h00_still_needed);
                        allocs.push(Allocation {
                            target_idx: live_idx as u8,
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
        let mut per_card_filled = vec![[0u8; 8]; num_cards];
        for a in allocs {
            if (a.target_idx as usize) < num_cards {
                per_card_filled[a.target_idx as usize][a.color as usize] += a.amount;
            }
        }
        // Check each card
        for (i, cn) in card_needs.iter().enumerate() {
            let filled = per_card_filled[i];
            let req = cn.need;
            let mut ok = true;
            let total_filled: u8 = filled.iter().sum();
            let total_required: u8 = req.iter().sum();
            if total_filled < total_required {
                ok = false;
            }
            // COLORLESS hearts (filled[0], e.g. from b_heart07) count toward the
            // heart0/total bucket but can NEVER be used as a specific color.
            // Only icon_all (filled[7]) can cover a colored-note deficit.
            let mut icon_all = filled[7];
            if ok && req[0] > 0 {
                let any_hearts: u8 = filled[1..7].iter().sum::<u8>() + filled[0];
                if any_hearts + icon_all < req[0] {
                    ok = false;
                } else {
                    icon_all = icon_all.saturating_sub(req[0].saturating_sub(any_hearts));
                }
            }
            if ok {
                for idx in 1..7 {
                    if filled[idx] < req[idx] {
                        let deficit = req[idx] - filled[idx];
                        if icon_all >= deficit {
                            icon_all -= deficit;
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
    fn backtrack_allocate(pool: &[u8; 8], card_needs: &[CardNeed]) -> Option<Vec<Allocation>> {
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
        pool: &mut [u8; 8],
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
        let mut filled = [0u8; 8];
        for c in 1..7 {
            if need[c] > 0 && pool[c] > 0 {
                let take = pool[c].min(need[c]);
                allocs.push(Allocation {
                    target_idx: idx as u8,
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::StageHearts,
                    source_slot: None,
                    wildcard: false,
                    color: c as u8,
                    amount: take,
                    is_bonus: false,
                    phase: AllocPhase::Colored,
                });
                pool[c] -= take;
                filled[c] += take;
            }
        }

        // NOTE: There is deliberately NO phase where Heart00 (COLORLESS)
        // hearts fill a specific color requirement. A colorless heart
        // (e.g. from b_heart07) can ONLY replace heart0 requirements
        // (rule 2.1.1.2), never a heart01-heart06 note (rule 2.11.3).

        // ----- Choice phases: Phase 3a (which surplus colors → heart00) -----
        let total_filled_so_far: u8 = filled.iter().sum();
        let total_required: u8 = need.iter().sum();
        let h00_deficit = total_required.saturating_sub(total_filled_so_far);

        // Collect available surplus colors
        let mut surplus_colors: Vec<usize> = (1..7).filter(|&c| pool[c] > 0).collect();
        surplus_colors.sort();
        let total_surplus: u8 = surplus_colors.iter().map(|&c| pool[c]).sum();
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
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        idx: usize,
        colors: &[usize],
        remaining: u8,
        color_idx: usize,
        allocs: &mut Vec<Allocation>,
        filled: [u8; 8],
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
                    target_idx: idx as u8,
                    target_name: card_name.clone(),
                    source_type: SourceType::Stage,
                    source_name: SourceName::StageHearts,
                    source_slot: None,
                    wildcard: false,
                    color: c as u8,
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
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        idx: usize,
        allocs: &mut Vec<Allocation>,
        mut filled: [u8; 8],
    ) -> bool {
        let saved_pool = *pool;
        let saved_len = allocs.len();
        let cn = &card_needs[idx];
        let card_name = &cn.name;
        let need = cn.need;

        // Count all hearts allocated so far (1a + 3a)
        let total_filled_so_far: u8 = filled.iter().sum();
        let total_required: u8 = need.iter().sum();
        let h00_deficit = total_required.saturating_sub(total_filled_so_far);

        // Phase 3b: Heart00 (COLORLESS) → remaining heart0/total deficit.
        // Colorless hearts (e.g. b_heart07) go ONLY into the heart0 bucket —
        // never a specific color. Forced (no choice).
        if h00_deficit > 0 && pool[0] > 0 {
            let take = pool[0].min(h00_deficit);
            allocs.push(Allocation {
                target_idx: idx as u8,
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
        let total_filled_now: u8 = filled.iter().sum();
        let total_required: u8 = need.iter().sum();
        let h00_still_needed = total_required.saturating_sub(total_filled_now);

        let mut color_deficits: Vec<(usize, u8)> = Vec::new();
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
        pool: &mut [u8; 8],
        card_needs: &[CardNeed],
        idx: usize,
        allocs: &mut Vec<Allocation>,
        filled: [u8; 8],
        deficit_indices: &[usize],
        deficits: &[(usize, u8)],
        remaining: u8,
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
                let alloc_color = if target_color == 0 {
                    7
                } else {
                    target_color as u8
                };
                allocs.push(Allocation {
                    target_idx: idx as u8,
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
    /// COLORLESS hearts (filled[0], e.g. from b_heart07) count toward the
    /// heart0/total bucket but can NEVER be used as a specific color — only
    /// icon_all (filled[7]) can cover a colored-note deficit.
    fn card_ok_with_wildcard(filled: [u8; 8], need: [u8; 8]) -> bool {
        let mut icon_all = filled[7];
        let total_filled: u8 = filled.iter().sum();
        let total_required: u8 = need.iter().sum();
        if total_filled < total_required {
            return false;
        }
        if need[0] > 0 {
            let any_hearts: u8 = filled[1..7].iter().sum::<u8>() + filled[0];
            if any_hearts + icon_all < need[0] {
                return false;
            }
            icon_all = icon_all.saturating_sub(need[0].saturating_sub(any_hearts));
        }
        for idx in 1..7 {
            if filled[idx] < need[idx] {
                let deficit = need[idx] - filled[idx];
                if icon_all >= deficit {
                    icon_all -= deficit;
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
        heart_override: &HashMap<i16, (HeartColor, u8)>,
        heart_modifiers: &HashMap<i16, HashMap<HeartColor, ModifierEntry>>,
        heart_color_multiplier: &HashMap<i16, HeartColor>,
        heart_copy: &HashMap<i16, i16>,
        live_card_ids: &[i16],
        _allocations: &[Allocation],
        yell_cards: &[YellCardResult],
        total_blade: u8,
        cheer_icon_count: u8,
        member_contributions: &[MemberContribution],
        total_hearts_arr: &[u8; 8],
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
            heart_copy,
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

        let mut per_card_filled: Vec<[u8; 8]> = vec![EMPTY_H8; live_card_ids.len()];
        for alloc in &allocations {
            if (alloc.target_idx as usize) < per_card_filled.len() {
                per_card_filled[alloc.target_idx as usize][alloc.color as usize] += alloc.amount;
            }
        }

        // Q259: Required heart check is only performed at live success judgment timing.
        // Subsequent changes do NOT retroactively fail a live.
        let any_requirement_failed = live_card_ids.iter().enumerate().any(|(live_idx, &lc_id)| {
            card_db.get_card(lc_id).is_some_and(|card| {
                // Build effective need_heart by starting from base requirements,
                // then applying per-color set/additive modifiers.
                // Q115/Q127: A set modifier on one color does NOT erase other colors.
                let base_nh = match card.need_heart.as_ref() {
                    Some(nh) => nh,
                    None => return false,
                };
                if base_nh.hearts.is_empty() {
                    return false;
                }
                let mut required_arr = EMPTY_H8;
                // Populate from base card requirements.
                for (color, needed) in &base_nh.hearts {
                    required_arr[color.index()] = *needed;
                }
                // Apply set modifiers (per-color override).
                if let Some(card_mods) = need_heart_modifiers.get(&lc_id) {
                    // Q115: Set-to-X applies first, then additive stacks.
                    for (color, me) in card_mods {
                        if me.set != 0 {
                            required_arr[color.index()] = me.set as u8;
                        }
                    }
                    for (color, me) in card_mods {
                        if me.additive != 0 {
                            let idx = color.index();
                            let current = required_arr[idx] as i32;
                            required_arr[idx] = crate::constants::saturate_u8(current + me.additive as i32);
                        }
                    }
                }
                let filled = per_card_filled[live_idx];
                // COLORLESS hearts (filled[0], e.g. from b_heart07) count toward the
                // heart0/total bucket but can NEVER be used as a specific color —
                // only icon_all (filled[7]) can cover a colored-note deficit.
                let mut icon_all = filled[7];
                let mut ok = true;
                let total_filled: u8 = filled.iter().sum();
                let total_required: u8 = required_arr.iter().sum();
                if total_filled < total_required {
                    ok = false;
                }
                if ok && required_arr[0] > 0 {
                    let any_hearts: u8 = filled[1..7].iter().sum::<u8>() + filled[0];
                    if any_hearts + icon_all < required_arr[0] {
                        ok = false;
                    } else {
                        icon_all =
                            icon_all.saturating_sub(required_arr[0].saturating_sub(any_hearts));
                    }
                }
                if ok {
                    for idx in 1..7 {
                        if filled[idx] < required_arr[idx] {
                            let deficit = required_arr[idx] - filled[idx];
                            if icon_all >= deficit {
                                icon_all -= deficit;
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
                        amount: app.amount.unsigned_abs() as u8,
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
                        amount: app.amount.unsigned_abs() as u8,
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
                    value: app.amount.unsigned_abs() as u8,
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
    turn: u8,
    player_id: &str,
    perf: &LivePerformanceData,
    card_db: &CardDatabase,
    note_icons: u8,
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
        base_score_total: 0,
        card_bonus_total: 0,
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
