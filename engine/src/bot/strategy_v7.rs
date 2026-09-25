//! Strategy bot v7 — v6 tempo plus main-phase vision fixes.
//!
//! v6 is the strongest heuristic bot. Trace analysis (bot_arena --trace,
//! 5CP3Z idou mirror) showed v6 takes `UseAbility` ~1% of the times it is
//! offered. Two mechanisms, both fixed here with rule-derived mechanics:
//!
//! 1. ABILITY BLINDNESS. v6's hearts/blades accounting reads only PRINTED
//!    `base_heart`/`blade`, so buff-granting abilities (e.g. "wait: +1 heart
//!    until live end") price at ~0 or negative, and free card draw (e.g.
//!    "cost-13+ member out: draw 1") shows no passable/ammo delta and ties
//!    `Pass`. v7 uses buff-aware accounting and values net draws on
//!    non-deploy actions.
//! 2. NO-OP BLINDNESS. v6's no-op breaker compares zone COUNTS only, so a
//!    pure-buff ability (no count changes) scores -1000 and is never taken.
//!    v7 extends the breaker to buffs + orientations.
//!
//! Deliberately NOT changed (all measured neutral-to-negative in arena):
//! - Member-deploy economics are v6-verbatim (energy is NOT taxed: unspent
//!   energy refreshes next turn, so taxing deploys only flips marginal-but-
//!   real development into Pass-ties, measured +4pp empty mains).
//! - The live set is v6-verbatim. Two principled attempts regressed ~3pp:
//!   (a) ranking by a stricter per-color pass probability prefers lower-
//!   score "safe" sets, but a safe set that loses the comparison places
//!   NOTHING, exactly like a failed check — score must stay maximized;
//!   (b) demanding a strict score beat over the opponent's estimated
//!   ceiling at closeout forces flaky oversized portfolios: the estimate is
//!   their CEILING, not their set, and the forced portfolio forfeits the
//!   gift of placing on their outright failure.
//!
//! Fairness: everything reads own hand/deck + public board/modifiers only.
//! No opponent hidden zones, no decklist peeking.

use crate::bot::strategy_v4::{
    alloc, flip_stats, hand_lives, heart_pool, heart_pool_buffed, heart_pool_inner, lives_in_hand,
    stage_buff_hearts,
};
use crate::bot::strategy_v5::binom_ge;
use crate::card::{CardDatabase, CardType};
use crate::core::stats_pipeline;
use crate::game_setup::{Action, ActionType};
use crate::game_state::{GameState, Phase};
use crate::player::Player;

fn env_weight(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default)
}

/// Stage hearts + active buffs (what actually passes checks).
fn stage_hearts_of(p: &Player, gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let base: i32 = p
        .stage
        .stage
        .iter()
        .filter(|&&c| c >= 0)
        .map(|&c| {
            db.get_card(c)
                .and_then(|x| x.base_heart.as_ref())
                .map(|bh| bh.hearts.values_sum() as i32)
                .unwrap_or(0)
        })
        .sum();
    let buffs = stage_buff_hearts(gs, me);
    // Draw/Score buffs (idx 8/9) never feed checks; stage_buff_hearts already
    // skips them, so a plain sum is in heart units.
    base + buffs.iter().sum::<i32>()
}

/// Active blades + blade modifiers (waiting members contribute 0, Q133).
fn total_blades_of(p: &Player, gs: &GameState, db: &CardDatabase) -> i32 {
    p.stage
        .stage
        .iter()
        .filter(|&&c| c >= 0)
        .map(|&c| {
            let waiting = gs.mods.get_orientation_modifier(c) == Some("wait");
            if waiting {
                0
            } else {
                db.get_card(c).map(|x| x.blade as i32).unwrap_or(0) + gs.mods.get_blade_modifier(c)
            }
        })
        .sum()
}

fn public_opponent_ceiling(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let (_, opp) = gs.seated_pair(me);
    let hearts = stats_pipeline::stage_hearts(
        &opp.stage.stage,
        db,
        &gs.mods.heart_override,
        &gs.mods.heart_copy,
        &gs.mods.heart_color_multiplier,
        &gs.mods.heart_modifiers,
    );
    let blades = opp.stage.total_blades(
        db,
        &gs.mods.blade_modifiers,
        &gs.mods.orientation_modifiers,
        false,
    );
    let pool = hearts.hearts.values().copied().map(i32::from).sum::<i32>() + (blades as i32) / 2;
    if pool < 3 {
        0
    } else {
        ((pool - 1) / 2).min(12)
    }
}

/// Passable lives under the buff-aware mean pool.
fn passable_count_buffed(gs: &GameState, me: u8, db: &CardDatabase) -> usize {
    let p = gs.seat_player(me);
    let pool = heart_pool_buffed(gs, me, db, 1.0);
    hand_lives(p, db)
        .iter()
        .filter(|(_, _, need)| alloc(&pool, need).is_some())
        .count()
}

/// Orientation fingerprint for the no-op breaker (waiting kills blades).
fn wait_fingerprint(gs: &GameState, me: u8) -> Vec<bool> {
    gs.seat_player(me)
        .stage
        .stage
        .iter()
        .map(|&c| {
            if c < 0 {
                false
            } else {
                gs.mods.get_orientation_modifier(c) == Some("wait")
            }
        })
        .collect()
}

/// Blade-modifier fingerprint for the no-op breaker: an activation whose
/// only effect is granting blades (live-only) changes zone counts nowhere,
/// but it feeds the flip model that converts blades into check passes.
fn blade_mod_fingerprint(gs: &GameState, me: u8) -> Vec<i16> {
    gs.seat_player(me)
        .stage
        .stage
        .iter()
        .map(|&c| {
            if c < 0 {
                0i16
            } else {
                crate::constants::saturate_i16(gs.mods.get_blade_modifier(c))
            }
        })
        .collect()
}

/// MAIN PHASE: v6 tempo + energy/draw vision on non-deploy actions + buff
/// vision + buff-aware no-op breaker. Member-deploy economics are v6-verbatim.
///
/// Dev levers (ablation only; unset in production):
/// - `V7_MAIN_V6=1` → delegate to v6's main (isolates live-set changes).
pub fn choose_action_v7(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if std::env::var_os("V7_MAIN_LEGACY").is_none() {
        return crate::bot::v7_main::choose_action(gs, actions, me);
    }
    if std::env::var("V7_MAIN_V6").is_ok() {
        return crate::bot::strategy_v6::choose_action_v6(gs, actions, me);
    }
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let scores = score_actions(gs, actions, me);
    let mut best = 0;
    for i in 1..scores.len() {
        if scores[i].0 > scores[best].0 {
            best = i;
        }
    }
    actions[best].clone()
}

pub fn score_actions(gs: &GameState, actions: &[Action], me: u8) -> Vec<(f64, String)> {
    if std::env::var_os("V7_MAIN_LEGACY").is_none() {
        return crate::bot::v7_main::score_actions(gs, actions, me);
    }
    if std::env::var("V7_MAIN_V6").is_ok() {
        return crate::bot::strategy_v6::score_actions(gs, actions, me);
    }
    let dbg = std::env::var("V7_DEBUG").is_ok();
    let db = &gs.card_database;
    let my_now = gs.seat_player(me);
    let base_hand_len = my_now.hand.cards.len() as i32;
    let base_passable = passable_count_buffed(gs, me, db);
    let base_ammo = lives_in_hand(my_now, db);
    let base_stage = stage_hearts_of(my_now, gs, me, db);
    let base_blades = total_blades_of(my_now, gs, db);
    let base_energy = my_now.energy_zone.active_count() as i32;
    let base_buffs = stage_buff_hearts(gs, me);
    let base_wait = wait_fingerprint(gs, me);
    let base_blade_mods = blade_mod_fingerprint(gs, me);
    let stage_cost = |p: &Player| -> i32 {
        p.stage
            .stage
            .iter()
            .filter_map(|&id| db.get_card(id))
            .map(|card| i32::from(card.cost.unwrap_or(0)))
            .sum()
    };
    let base_cost = stage_cost(my_now);
    let development = std::env::var("V7_NO_DEVELOPMENT").is_err();
    let heart_weight = env_weight("V7_HEART_WEIGHT", 3.0);
    let blade_weight = env_weight("V7_BLADE_WEIGHT", 6.0);

    let deck_lives = my_now
        .main_deck
        .cards
        .iter()
        .filter(|&&c| {
            db.get_card(c)
                .map_or(false, |x| x.card_type == CardType::Live)
        })
        .count();
    let deck_len = my_now.main_deck.cards.len().max(1);
    let p_life_draw = deck_lives as f64 / deck_len as f64;
    let waitroom_lives = my_now
        .waitroom
        .cards
        .iter()
        .filter(|&&c| {
            db.get_card(c)
                .map_or(false, |x| x.card_type == CardType::Live)
        })
        .count();

    let mut vals: Vec<f64> = vec![f64::NEG_INFINITY; actions.len()];
    let mut dbg_lines: Vec<String> = Vec::new();
    let mut breakdowns = vec![String::new(); actions.len()];

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if crate::game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        crate::game_setup::settle_single_player_state(&mut sim);
        let my_sim = sim.seat_player(me);

        let mut val = 0.0f64;
        let mut parts: Vec<String> = Vec::new();

        // Doctrine 1: passable lives (placements-in-waiting), buff-aware.
        let d_pass = passable_count_buffed(&sim, me, db) as f64 - base_passable as f64;
        val += 60.0 * d_pass;
        if d_pass != 0.0 {
            parts.push(format!("pass{:+}", d_pass));
        }

        // Doctrine 2: ammo — lives in hand are future placements.
        let ammo_after = lives_in_hand(my_sim, db);
        let d_ammo = ammo_after as f64 - base_ammo as f64;
        val += 25.0 * d_ammo;
        if d_ammo != 0.0 {
            parts.push(format!("ammo{:+}", d_ammo));
        }
        if ammo_after == 0 && base_ammo > 0 {
            val -= 120.0; // never burn the whole arsenal
            parts.push("BURN".into());
        }

        // Development in HEARTS and BLADES, buff-aware: every extra active
        // blade is a fresh Binomial trial; buffs feed checks for real.
        let d_stage = stage_hearts_of(my_sim, &sim, me, db) - base_stage;
        val += heart_weight * d_stage as f64;
        if d_stage != 0 {
            parts.push(format!("hearts{d_stage:+}"));
        }
        let d_blades = total_blades_of(my_sim, &sim, db) - base_blades;
        val += blade_weight * d_blades as f64;
        if d_blades != 0 {
            parts.push(format!("blades{d_blades:+}"));
        }

        // Energy pricing and draw vision for NON-DEPLOY actions only: paid
        // abilities must earn their energy, untaps are income, and a free
        // draw beats ending the phase. Member plays are excluded — unspent
        // energy refreshes next turn (activate_all), and their hand cost is
        // already priced via passable/ammo above.
        if a.action_type != ActionType::PlayMemberToStage {
            let d_energy = my_sim.energy_zone.active_count() as i32 - base_energy;
            val += 2.0 * d_energy as f64;
            if d_energy != 0 {
                parts.push(format!("en{d_energy:+}"));
            }
            let d_hand = my_sim.hand.cards.len() as i32 - base_hand_len;
            val += 3.0 * d_hand as f64;
            if d_hand != 0 {
                parts.push(format!("hand{d_hand:+}"));
            }
        }

        // Baton touch: discounted upgrade of the power piece (guide curve
        // 4->9->13). Strongly favored. D1: detect via occupied target /
        // available_areas, never via the unset `use_baton_touch` param.
        // Ablation: V7_PRE_D=1 restores the dead key; V7_NO_BATON=1 disables.
        let cost_growth = stage_cost(my_sim) - base_cost;
        if development {
            val += 8.0 * f64::from(cost_growth);
            parts.push(format!("development{:+}", 8 * cost_growth));
        }
        let baton_hit = a.parameters.as_ref().is_some_and(|p| {
            p.use_baton_touch == Some(true) || crate::bot::v7_main::baton_from_params(gs, me, p)
        });
        // D1: detect-only in the default path — `development` (8× cost growth)
        // already prices batons; a flat +45 double-counted and regressed ~18pp
        // (measured 2026-09-23). Flat bonus kept behind V7_BATON_FLAT for
        // ablation reproduction.
        if baton_hit && std::env::var("V7_BATON_FLAT").is_ok() {
            val += 45.0;
            parts.push("baton+45".into());
        } else if baton_hit {
            parts.push("baton".into());
        }

        // Hand reserve: a 1-card hand can neither set lives nor pay costs.
        if my_sim.hand.cards.len() <= 1 {
            val -= 60.0;
        }

        // Life acquisition when starved: drawing digs toward the next life;
        // milling to waitroom banks lives for retrieval engines.
        if base_ammo <= 1 {
            let drawn = (my_sim.hand.cards.len() as i32 - base_hand_len).max(0);
            val += 70.0 * p_life_draw * drawn as f64;
            let wr_now = my_sim
                .waitroom
                .cards
                .iter()
                .filter(|&&c| {
                    db.get_card(c)
                        .map_or(false, |x| x.card_type == CardType::Live)
                })
                .count();
            if wr_now > waitroom_lives && p_life_draw > 0.0 {
                val += 25.0;
            }
        }

        // No-op breaker: an action that changes nothing useful is worthless.
        // v6 compared zone counts only, which scored pure-buff abilities
        // -1000 and made them unplayable. Buffs and orientations feed checks,
        // so they join the fingerprint.
        if my_sim.hand.cards.len() == my_now.hand.cards.len()
            && my_sim.energy_zone.active_count() == my_now.energy_zone.active_count()
            && my_sim.stage.stage == my_now.stage.stage
            && my_sim.main_deck.cards.len() == my_now.main_deck.cards.len()
            && my_sim.waitroom.cards.len() == my_now.waitroom.cards.len()
            && stage_buff_hearts(&sim, me) == base_buffs
            && wait_fingerprint(&sim, me) == base_wait
            && blade_mod_fingerprint(&sim, me) == base_blade_mods
        {
            val -= 1000.0;
            parts.push("NOOP".into());
        }

        parts.push(format!("={val:.0}"));
        if dbg {
            let card_no = a
                .parameters
                .as_ref()
                .and_then(|p| p.card_id)
                .and_then(|cid| db.get_card(cid))
                .map(|c| c.card_no.clone())
                .unwrap_or_default();
            let baton_mark = if a.parameters.as_ref().is_some_and(|p| {
                p.use_baton_touch == Some(true) || crate::bot::v7_main::baton_from_params(gs, me, p)
            }) {
                "[BATON]"
            } else {
                ""
            };
            dbg_lines.push(format!(
                "    [{}] {:?} {} {} -> {}",
                i,
                a.action_type,
                card_no,
                baton_mark,
                parts.join(" ")
            ));
        }

        vals[i] = val;
        breakdowns[i] = parts.join(" ");
    }

    // v6 fix (kept): `Pass` ends the development phase. It is chosen ONLY
    // when there is no USEFUL deploy available — any non-Pass action with
    // value > 0 beats it, while a 0-contribution waiting member must not clog
    // the stage.
    let best_nonpass = vals
        .iter()
        .enumerate()
        .filter(|(i, _)| actions[*i].action_type != ActionType::Pass)
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    for (i, a) in actions.iter().enumerate() {
        if a.action_type == ActionType::Pass {
            vals[i] = if best_nonpass > 0.0 {
                f64::NEG_INFINITY
            } else {
                0.0
            };
        }
    }
    let mut best_idx = 0usize;
    let mut best_val = f64::NEG_INFINITY;
    for (i, &v) in vals.iter().enumerate() {
        if v > best_val {
            best_val = v;
            best_idx = i;
        }
    }

    if dbg {
        let chosen = &actions[best_idx];
        let chosen_confirm = chosen.action_type == ActionType::Pass;
        eprintln!(
            "V7D t{} phase{:?} hand={} pass={} ammo={} blades={} en={} CONFIRM={}\n{}",
            gs.turn_number,
            gs.current_phase,
            my_now.hand.cards.len(),
            base_passable,
            base_ammo,
            base_blades,
            base_energy,
            chosen_confirm,
            dbg_lines.join("\n")
        );
    }
    log::debug!(
        "v7 main t{} phase{:?} hand={} pass={} ammo={} blades={} en={} best_nonpass={:.0} pass={}",
        gs.turn_number,
        gs.current_phase,
        my_now.hand.cards.len(),
        base_passable,
        base_ammo,
        base_blades,
        base_energy,
        best_nonpass,
        actions[best_idx].action_type,
    );
    vals.into_iter().zip(breakdowns).collect()
}

/// LIVE SET: role-dependent portfolios (measured doctrine).
///
/// Arena transcripts (v6 mirror, 5CP3Z, 2450 live checks) show placements
/// come from PASSING ALONE, not outscoring: the winner averages 3.4 while
/// the loser averages 0.6 (usually failed outright or empty), and 45% of
/// checks are NO-CONTEST. Both-pass comparisons are rare — EXCEPT against
/// an opponent that already committed lives, who passes ~86% of the time.
/// Hence:
/// - FIRST attacker (opponent unset — comparison unlikely, they fold or fail
///   ~2/3 of turns): set the SAFEST single life (max P(pass)), greedily
///   adding more lives only while the bundle stays ≥0.85. Reliability
///   converts; score is nearly irrelevant here.
/// - SECOND attacker vs a SET opponent, or anyone at opponent match point
///   (comparison likely — they will pass something): v6 score-max EV.
/// - SECOND attacker vs empty zone: free win (cheapest passer, 8.4.3.2).
/// - No passer: v6 gamble + junk-dig fallback, unchanged.
///
/// (Deliberately dropped: per-color ranking — it preferred lower-score
/// "safe" sets even as second attacker, measured -3pp; strict closeout
/// beats — the ceiling estimate is not their set, measured -3pp with the
/// above confounded in.)
pub fn choose_live_set_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    if std::env::var("V7_LIVE_V6").is_ok() {
        return crate::bot::strategy_v6::choose_live_set_v6(gs, actions, db);
    }
    if std::env::var("V7_ROLLOUT").is_ok() {
        return crate::bot::rollout::choose_live_set_v7(gs, actions, db);
    }
    if std::env::var_os("V7_LIVE_LEGACY").is_none() {
        return choose_live_set_experiment(gs, actions, db);
    }
    let me = gs.active_player_index();
    let (my, opp) = gs.seated_pair(me);
    let my_succ = my.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;
    let is_second = gs.current_phase == Phase::LiveCardSetSecondAttacker;
    let opp_committed = !opp.live_card_zone.cards.is_empty();

    // Comparison likely: opponent committed lives, or must contest at match
    // point → v6 score-max EV (proven there).
    if (is_second && opp_committed) || opp_succ >= 2 {
        return crate::bot::strategy_v6::choose_live_set_v6(gs, actions, db);
    }

    // Free win (8.4.3.2): second attacker, opponent zone empty.
    if is_second && !opp_committed {
        if let Some(hi) = cheapest_deterministic_life(gs, me, db) {
            return emit(gs, actions, &[hi]);
        }
        // No deterministic passer: fall through to safest/gamble below.
    }

    // First attacker (or second vs empty with no deterministic passer):
    // safest-single search.
    let mut desired = safest_portfolio(gs, me, db);

    if desired.is_empty() {
        // v6 gamble + junk-dig fallback, unchanged.
        return crate::bot::strategy_v6::choose_live_set_v6(gs, actions, db);
    }

    // Junk filter (v6-verbatim): spare slots take dead non-live cards —
    // discarded before the check, each drawing a replacement.
    {
        let deck_lives = my
            .main_deck
            .cards
            .iter()
            .filter(|&&cid| {
                db.get_card(cid)
                    .map_or(false, |c| c.card_type == CardType::Live)
            })
            .count();
        let max_slots = (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
        if desired.len() < max_slots && deck_lives > 0 {
            let mut junk: Vec<(usize, u8)> = my
                .hand
                .cards
                .iter()
                .enumerate()
                .filter(|&(i, &cid)| {
                    !desired.contains(&i)
                        && db
                            .get_card(cid)
                            .map_or(false, |c| c.card_type != CardType::Live)
                })
                .map(|(i, &cid)| (i, db.get_card(cid).and_then(|c| c.cost).unwrap_or(0)))
                .collect();
            junk.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost));
            for &(hi, _) in &junk {
                if desired.len() >= max_slots {
                    break;
                }
                desired.push(hi);
            }
        }
    }

    if std::env::var("V7_TRACE").is_ok() {
        let my_score: i32 = desired
            .iter()
            .filter_map(|&hi| my.hand.cards.get(hi).copied())
            .filter_map(|cid| db.get_card(cid))
            .map(|c| c.score.unwrap_or(0) as i32)
            .sum();
        eprintln!(
            "V7L t{} me{} SAFE n={} score={} (my_succ={} opp_succ={})",
            gs.turn_number,
            me,
            desired.len(),
            my_score,
            my_succ,
            opp_succ
        );
    }
    emit(gs, actions, &desired)
}

/// Safest-single search: rank hand lives by total-shortfall pass probability
/// (v5's calibrated model), tie-break by score; set the safest, greedily
/// adding the next-safest while the whole bundle stays ≥0.85 pass. Returns
/// empty when nothing clears the stance floor (caller falls back to gamble).
fn safest_portfolio(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<usize> {
    let (my, opp) = gs.seated_pair(me);
    let pool = heart_pool(gs, me, db);
    let board = heart_pool_inner(gs, me, db, 0.0);
    let lives = hand_lives(my, db);
    let max_slots = (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    if lives.is_empty() || max_slots == 0 {
        return Vec::new();
    }
    let my_succ = my.success_live_card_zone.cards.len() as i32;
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;
    let floor = if opp_succ >= 2 {
        0.35
    } else if my_succ >= 2 {
        0.60
    } else {
        0.45
    };
    let (blades, density) = flip_stats(gs, me, db);
    let board_supply: i32 = (0..=7).chain(std::iter::once(10)).map(|i| board[i]).sum();

    // Score every single life: pass probability under the calibrated model.
    let mut ranked: Vec<(f64, i32, usize)> = Vec::new();
    for &(hi, cid, ref need) in &lives {
        if alloc(&pool, need).is_none() {
            continue;
        }
        let req: i32 = (0..=7).chain(std::iter::once(10)).map(|k| need[k]).sum();
        let shortfall = (req - board_supply.min(req)).max(0);
        let p = if shortfall == 0 {
            1.0
        } else {
            binom_ge(blades, shortfall, density)
        };
        if p < floor {
            continue;
        }
        let score = db.get_card(cid).and_then(|c| c.score).unwrap_or(0) as i32;
        ranked.push((p, score, hi));
    }
    // Safest first, then highest score.
    ranked.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.cmp(&a.1))
    });
    // TEMP ANALYSIS: full candidate dump behind V5_LIVE_DEBUG.
    if std::env::var("V5_LIVE_DEBUG").is_ok() {
        let hand_desc: Vec<String> = lives
            .iter()
            .map(|(hi, cid, need)| {
                let req: i32 = (0..=7).chain(std::iter::once(10)).map(|k| need[k]).sum();
                format!(
                    "hi{}:{}s:req{}",
                    hi,
                    db.get_card(*cid).and_then(|c| c.score).unwrap_or(0),
                    req
                )
            })
            .collect();
        eprintln!(
            "V7LD t{} me{} hand=[{}] blades={} dens={:.2} floor={:.2} my{} opp{}",
            gs.turn_number,
            me,
            hand_desc.join(" "),
            blades,
            density,
            floor,
            my_succ,
            opp_succ
        );
        for (p, score, hi) in ranked.iter().take(6) {
            eprintln!("    single hi{} score={} p={:.2}", hi, score, p);
        }
    }
    if ranked.is_empty() {
        return Vec::new();
    }
    // Greedy bundle: start with the safest, add next-safest while the whole
    // bundle stays reliable (≥0.85). Extra reliable score wins the rare
    // both-pass comparison for free; anything flakier is left for later.
    let mut desired = vec![ranked[0].2];
    let mut need_total = [0i32; 11];
    {
        let (_, _, first_hi) = ranked[0];
        if let Some((_, _, need)) = lives.iter().find(|(hi, _, _)| *hi == first_hi) {
            need_total = *need;
        }
    }
    for &(_, _, hi) in &ranked[1..] {
        if desired.len() >= max_slots {
            break;
        }
        let Some((_, _, need)) = lives.iter().find(|(hh, _, _)| *hh == hi) else {
            continue;
        };
        let mut grown = need_total;
        for k in 0..11 {
            grown[k] += need[k];
        }
        // Bundle must still alloc against the mean pool (all-or-nothing).
        if alloc(&pool, &grown).is_none() {
            continue;
        }
        let req: i32 = (0..=7).chain(std::iter::once(10)).map(|k| grown[k]).sum();
        let shortfall = (req - board_supply.min(req)).max(0);
        let p = if shortfall == 0 {
            1.0
        } else {
            binom_ge(blades, shortfall, density)
        };
        if p >= 0.85 {
            desired.push(hi);
            need_total = grown;
        }
    }
    log::debug!(
        "v7 safest t{} me{} n={} top_p={:.2}",
        gs.turn_number,
        me,
        desired.len(),
        ranked[0].0
    );
    desired
}

fn cheapest_deterministic_life(gs: &GameState, me: u8, db: &CardDatabase) -> Option<usize> {
    // Buff-aware pool (same accounting as the v7 main phase; equal to v6's
    // whenever no buff modifiers are active).
    let (my, _) = gs.seated_pair(me);
    let pool = heart_pool_buffed(gs, me, db, 1.0);
    hand_lives(my, db)
        .into_iter()
        .filter(|(_, _, need)| alloc(&pool, need).is_some())
        .min_by_key(|(hi, cid, _)| (db.get_card(*cid).and_then(|c| c.score).unwrap_or(0), *hi))
        .map(|(hi, _, _)| hi)
}

const EXPERIMENT_SAMPLES: usize = 256;

type ExperimentPortfolio = (f64, i32, Vec<usize>);
type ExperimentSingle = (f64, i32, usize, [i32; 11]);

fn experiment_flip_categories(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
) -> (Vec<([i32; 8], usize)>, usize) {
    let (my, _) = gs.seated_pair(me);
    let override_color = my.stage.stage.iter().find_map(|cid| {
        gs.mods
            .blade_type_modifiers
            .get(cid)
            .copied()
            .map(crate::turn::live::blade_color_to_heart)
    });
    let mut cats: Vec<([i32; 8], usize)> = Vec::new();
    for &cid in &my.main_deck.cards {
        let mut total = [0u8; 8];
        if let Some(card) = db.get_card(cid) {
            let mut hearts = crate::card::BaseHeart {
                hearts: crate::card::HeartMap::new(),
            };
            let mut cheer = 0;
            crate::turn::live::process_yell_revealed_card_icons(
                card,
                override_color,
                &mut hearts,
                &mut total,
                &mut cheer,
            );
        }
        let vector = total.map(i32::from);
        if let Some((_, count)) = cats.iter_mut().find(|(v, _)| *v == vector) {
            *count += 1;
        } else {
            cats.push((vector, 1));
        }
    }
    cats.sort_unstable_by_key(|(vector, _)| *vector);
    (cats, my.main_deck.cards.len())
}

fn experiment_feasible(pool: &[i32; 8], need: &[i32; 11]) -> bool {
    let mut wildcard = pool[7];
    let mut bucket_supply = pool[0];
    for c in 1..=6 {
        let have = pool[c];
        let want = need[c];
        if have >= want {
            bucket_supply += have - want;
        } else {
            let deficit = want - have;
            if wildcard < deficit {
                return false;
            }
            wildcard -= deficit;
        }
    }
    bucket_supply += wildcard;
    bucket_supply >= need[0] + need[7] + need[10]
}

fn experiment_pass_probability(
    cats: &[([i32; 8], usize)],
    deck_len: usize,
    blades: i32,
    board: &[i32; 8],
    need: &[i32; 11],
) -> f64 {
    if blades <= 0 || cats.is_empty() || deck_len == 0 {
        return if experiment_feasible(board, need) {
            1.0
        } else {
            0.0
        };
    }
    let mut deck = Vec::with_capacity(deck_len);
    for &(vector, count) in cats {
        deck.extend(std::iter::repeat_n(vector, count));
    }
    deck.resize(deck_len, [0; 8]);
    deck.sort_unstable();
    let draws = usize::try_from(blades).unwrap_or(usize::MAX).min(deck_len);
    let mut rng = 0x6a09e667f3bcc909u64;
    let mut hits = 0usize;
    for _ in 0..EXPERIMENT_SAMPLES {
        let mut sampled = deck.clone();
        let mut pool = *board;
        for k in 0..draws {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let index = k + usize::try_from(rng % (deck_len - k) as u64)
                .expect("sample offset fits deck length");
            sampled.swap(k, index);
            for (have, extra) in pool.iter_mut().zip(sampled[k]) {
                *have += extra;
            }
        }
        if experiment_feasible(&pool, need) {
            hits += 1;
        }
    }
    hits as f64 / EXPERIMENT_SAMPLES as f64
}

fn live_payoff(my_success: usize, opp_success: usize, my_place: bool, opp_place: bool) -> f64 {
    let my_total = my_success + usize::from(my_place);
    let opp_total = opp_success + usize::from(opp_place);
    if my_total >= 3 && opp_total >= 3 {
        0.0
    } else if my_total >= 3 {
        1.0
    } else if opp_total >= 3 {
        -1.0
    } else {
        0.1 * (my_total as f64 - opp_total as f64)
    }
}

fn expected_live_value(
    pass: f64,
    score: i32,
    set_size: usize,
    my_success: usize,
    opp_success: usize,
    opp_pass: f64,
    opp_score: i32,
    opp_set_size: usize,
) -> f64 {
    let both_pass = pass * opp_pass;
    let self_only = pass * (1.0 - opp_pass);
    let opp_only = (1.0 - pass) * opp_pass;
    let mut value = self_only * live_payoff(my_success, opp_success, true, false)
        + opp_only * live_payoff(my_success, opp_success, false, true);
    if score > opp_score {
        value += both_pass * live_payoff(my_success, opp_success, true, false);
    } else if score < opp_score {
        value += both_pass * live_payoff(my_success, opp_success, false, true);
    } else {
        let my_place = set_size < 2;
        let opp_place = opp_set_size < 2;
        value += both_pass * live_payoff(my_success, opp_success, my_place, opp_place);
    }
    value
}

fn experiment_sample_pools(
    cats: &[([i32; 8], usize)],
    deck_len: usize,
    blades: i32,
    board: &[i32; 8],
) -> Vec<[i32; 8]> {
    if blades <= 0 || cats.is_empty() || deck_len == 0 {
        return vec![*board; EXPERIMENT_SAMPLES];
    }
    let mut deck = Vec::with_capacity(deck_len);
    for &(vector, count) in cats {
        deck.extend(std::iter::repeat_n(vector, count));
    }
    deck.resize(deck_len, [0; 8]);
    deck.sort_unstable();
    let draws = usize::try_from(blades).unwrap_or(usize::MAX).min(deck_len);
    let mut rng = 0x6a09e667f3bcc909u64;
    let mut pools = Vec::with_capacity(EXPERIMENT_SAMPLES);
    for _ in 0..EXPERIMENT_SAMPLES {
        let mut sampled = deck.clone();
        let mut pool = *board;
        for k in 0..draws {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let index = k + usize::try_from(rng % (deck_len - k) as u64)
                .expect("sample offset fits deck length");
            sampled.swap(k, index);
            for (have, extra) in pool.iter_mut().zip(sampled[k]) {
                *have += extra;
            }
        }
        pools.push(pool);
    }
    pools
}

fn experiment_pass_probability_pools(pools: &[[i32; 8]], need: &[i32; 11]) -> f64 {
    if pools.is_empty() {
        return 0.0;
    }
    pools
        .iter()
        .filter(|pool| experiment_feasible(pool, need))
        .count() as f64
        / pools.len() as f64
}

fn experiment_board_pool(gs: &GameState, me: u8, db: &CardDatabase) -> [i32; 8] {
    let (my, _) = gs.seated_pair(me);
    let hearts = my.stage.get_available_hearts(
        db,
        &gs.mods.heart_override,
        &gs.mods.heart_modifiers,
        &gs.mods.heart_color_multiplier,
        &gs.mods.heart_copy,
    );
    let mut pool = [0i32; 8];
    for (color, count) in &hearts.hearts {
        pool[color.index()] += i32::from(*count);
    }
    pool
}

fn experiment_score_of(db: &CardDatabase, cid: i16) -> i32 {
    db.get_card(cid).and_then(|c| c.score).unwrap_or(0) as i32
}

fn experiment_expected_yell_score(gs: &GameState, me: u8, db: &CardDatabase, blades: i32) -> i32 {
    if blades <= 0 {
        return 0;
    }
    let (my, _) = gs.seated_pair(me);
    let deck_len = my.main_deck.cards.len();
    if deck_len == 0 {
        return 0;
    }
    let score_icons: usize = my
        .main_deck
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|card| card.special_heart.as_ref())
        .filter_map(|hearts| hearts.hearts.get(&crate::card::HeartColor::Score).copied())
        .map(|count| usize::from(count))
        .sum();
    let draws = usize::try_from(blades).unwrap_or(usize::MAX).min(deck_len);
    (score_icons * draws / deck_len) as i32
}

fn experiment_lives(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<(usize, i16, [i32; 11])> {
    let (my, _) = gs.seated_pair(me);
    hand_lives(my, db)
        .into_iter()
        .map(|(hi, cid, _)| {
            let mut need = [0; 11];
            let base = db.get_card(cid).and_then(|card| card.need_heart.as_ref());
            if let Some(effective) = crate::core::stats_pipeline::effective_need_heart(
                base,
                cid,
                &gs.mods.need_heart_modifiers,
            ) {
                for (color, count) in &effective.hearts {
                    need[color.index()] += i32::from(*count);
                }
            }
            (hi, cid, need)
        })
        .collect()
}

fn experiment_blades(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let (my, _) = gs.seated_pair(me);
    i32::from(my.stage.total_blades(
        db,
        &gs.mods.blade_modifiers,
        &gs.mods.orientation_modifiers,
        false,
    ))
}

fn experiment_junk_fill(gs: &GameState, me: u8, db: &CardDatabase, desired: &mut Vec<usize>) {
    let (my, _) = gs.seated_pair(me);
    let deck_lives = my
        .main_deck
        .cards
        .iter()
        .filter(|&&cid| {
            db.get_card(cid)
                .is_some_and(|c| c.card_type == CardType::Live)
        })
        .count();
    let max_slots = usize::from(3u8.saturating_sub(my.live_card_set_limit_reduction));
    if desired.len() >= max_slots || deck_lives == 0 {
        return;
    }
    let future_junk = std::env::var_os("V7_NO_FUTURE_JUNK").is_none();
    let max_stage_cost = my
        .stage
        .stage
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|card| card.cost)
        .max()
        .unwrap_or(0) as i32;
    let budget = i32::from(u16::from(my.energy_zone.active_count())) + 1;
    let mut junk: Vec<(usize, u8, i32)> = my
        .hand
        .cards
        .iter()
        .enumerate()
        .filter(|&(i, &cid)| {
            !desired.contains(&i)
                && db
                    .get_card(cid)
                    .is_some_and(|c| c.card_type != CardType::Live)
        })
        .map(|(i, &cid)| {
            let card = db.get_card(cid);
            let cost = card.and_then(|c| c.cost).unwrap_or(0);
            let utility = if future_junk && card.is_some_and(|c| c.card_type == CardType::Member) {
                let hearts = card
                    .and_then(|c| c.base_heart.as_ref())
                    .map(|h| h.hearts.values_sum() as i32)
                    .unwrap_or(0);
                let delay = (i32::from(cost) - budget - max_stage_cost).max(0) as f64;
                (2.0 * f64::from(cost)
                    + 2.0 * f64::from(card.map(|c| c.blade).unwrap_or(0))
                    + f64::from(hearts))
                    / (1.0 + delay)
            } else {
                0.0
            };
            let class = if future_junk && card.is_some_and(|c| c.card_type == CardType::Energy) {
                2
            } else if future_junk {
                1
            } else {
                0
            };
            (i, cost, class * 1_000_000 + utility as i32)
        })
        .collect();
    junk.sort_by_key(|&(_, cost, utility)| {
        if future_junk {
            (utility, std::cmp::Reverse(cost))
        } else {
            (0, std::cmp::Reverse(cost))
        }
    });
    for &(hi, _, _) in &junk {
        if desired.len() >= max_slots {
            break;
        }
        desired.push(hi);
    }
}

fn experiment_free_win(gs: &GameState, me: u8, db: &CardDatabase) -> Option<usize> {
    let pool = experiment_board_pool(gs, me, db);
    experiment_lives(gs, me, db)
        .into_iter()
        .filter(|(_, _, need)| experiment_feasible(&pool, need))
        .min_by_key(|(hi, cid, _)| (db.get_card(*cid).and_then(|c| c.score).unwrap_or(0), *hi))
        .map(|(hi, _, _)| hi)
}

fn experiment_portfolio_rank(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
) -> (Vec<ExperimentPortfolio>, Vec<ExperimentSingle>) {
    let (my, _) = gs.seated_pair(me);
    let lives = hand_lives(my, db);
    let max_slots = usize::from(3u8.saturating_sub(my.live_card_set_limit_reduction));
    if lives.is_empty() || max_slots == 0 {
        return (Vec::new(), Vec::new());
    }
    let lives = experiment_lives(gs, me, db);
    let n = lives.len();
    let (cats, deck_len) = experiment_flip_categories(gs, me, db);
    let blades = experiment_blades(gs, me, db);
    let board = experiment_board_pool(gs, me, db);
    let shared_pools = std::env::var_os("V7_NO_SHARED_SAMPLES")
        .is_none()
        .then(|| experiment_sample_pools(&cats, deck_len, blades, &board));
    let yell_score = if std::env::var_os("V7_YELL_SCORE").is_some() {
        experiment_expected_yell_score(gs, me, db, blades)
    } else {
        0
    };
    let mut needs: Vec<[i32; 11]> = Vec::with_capacity(n);
    let mut scores: Vec<i32> = Vec::with_capacity(n);
    for &(_, cid, ref need) in lives.iter().take(n) {
        needs.push(*need);
        scores.push(experiment_score_of(db, cid));
    }
    let mut singles: Vec<ExperimentSingle> = Vec::new();
    for bit in 0..n {
        if needs[bit][8] > 0 || needs[bit][9] > 0 {
            continue;
        }
        let p = shared_pools.as_ref().map_or_else(
            || experiment_pass_probability(&cats, deck_len, blades, &board, &needs[bit]),
            |pools| experiment_pass_probability_pools(pools, &needs[bit]),
        );
        if p > 0.0 {
            singles.push((p, scores[bit] + yell_score, lives[bit].0, needs[bit]));
        }
    }
    singles.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    let mut out = Vec::new();
    for mask in 1..(1u32 << n) {
        let cnt = mask.count_ones() as usize;
        if cnt > max_slots {
            continue;
        }
        let mut need_total = [0i32; 11];
        let mut score = yell_score;
        let mut idxs = Vec::with_capacity(cnt);
        let mut unpassable = false;
        for bit in 0..n {
            if mask & (1 << bit) != 0 {
                for k in 0..11 {
                    need_total[k] += needs[bit][k];
                }
                if needs[bit][8] > 0 || needs[bit][9] > 0 {
                    unpassable = true;
                }
                score += scores[bit];
                idxs.push(lives[bit].0);
            }
        }
        if unpassable {
            continue;
        }
        let p = shared_pools.as_ref().map_or_else(
            || experiment_pass_probability(&cats, deck_len, blades, &board, &need_total),
            |pools| experiment_pass_probability_pools(pools, &need_total),
        );
        let (_, opp) = gs.seated_pair(me);
        let floor = if opp.success_live_card_zone.cards.len() >= 2 {
            0.35
        } else if my.success_live_card_zone.cards.len() >= 2 {
            0.60
        } else {
            0.45
        };
        if p < floor {
            continue;
        }
        out.push((p * score as f64, score, idxs));
    }
    out.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| a.2.len().cmp(&b.2.len()))
    });
    (out, singles)
}

fn count_lives(gs: &GameState, desired: &[usize], db: &CardDatabase) -> usize {
    let (my, _) = gs.seated_pair(gs.active_player_index());
    desired
        .iter()
        .filter(|&&hi| {
            my.hand.cards.get(hi).copied().map_or(false, |cid| {
                db.get_card(cid)
                    .is_some_and(|c| c.card_type == CardType::Live)
            })
        })
        .count()
}

pub(crate) fn choose_live_set_experiment(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
) -> Action {
    let me = gs.active_player_index();
    let (my, opp) = gs.seated_pair(me);
    let my_succ = my.success_live_card_zone.cards.len();
    let opp_succ = opp.success_live_card_zone.cards.len();
    let is_second = gs.current_phase == Phase::LiveCardSetSecondAttacker;
    let opp_committed = !opp.live_card_zone.cards.is_empty();
    let floor = if opp_succ >= 2 {
        0.35
    } else if my_succ >= 2 {
        0.60
    } else {
        0.45
    };

    let mut desired: Vec<usize> = Vec::new();

    if my.live_card_set_limit_reduction >= 3 || gs.cannot_live_players.contains(&my.id) {
        return emit(gs, actions, &desired);
    }
    if is_second && !opp_committed {
        if let Some(hi) = experiment_free_win(gs, me, db) {
            desired.push(hi);
            if std::env::var_os("V7_FREE_JUNK").is_some() {
                experiment_junk_fill(gs, me, db, &mut desired);
            }
            return emit(gs, actions, &desired);
        }
    }

    let (ranked, singles) = experiment_portfolio_rank(gs, me, db);
    let contested = (is_second && opp_committed) || opp_succ >= 2;
    if contested {
        // D3: comparison-aware sizing. If even our best floor-clearing
        // portfolio cannot beat the opponent's PUBLIC ceiling by a real
        // margin, a multi-life score-max set only donates ammo — prefer the
        // single safest life (max P(we pass); placement then rides on their
        // failure). Fold only when the opponent zone is still empty (nothing
        // to gift) and no single clears the floor. Reverse-gate: V7_PRE_D=1
        // keeps pure score-max (old behavior).
        let ceiling_enabled =
            std::env::var_os("V7_PRE_D").is_none() && std::env::var_os("V7_NO_CEILING").is_none();
        let mut chose_single = false;
        if my_succ >= 2 && opp_succ >= 2 {
            if let Some(&(_, _, first_hi, _)) = singles.iter().max_by_key(|(_, _, hi, _)| {
                db.get_card(my.hand.cards.get(*hi).copied().unwrap_or(-1))
                    .and_then(|c| c.score)
                    .unwrap_or(0)
            }) {
                desired.push(first_hi);
                chose_single = true;
                log::debug!(
                    "v7 score-race t{} me{} my{} opp{} hi={}",
                    gs.turn_number,
                    me,
                    my_succ,
                    opp_succ,
                    first_hi
                );
            }
        }
        if !chose_single
            && std::env::var_os("V7_PAYOFF_MODEL").is_some()
            && (my_succ >= 2 || opp_succ >= 2)
        {
            let e_opp = public_opponent_ceiling(gs, me, db);
            let opp_pass = if opp_committed { 0.86 } else { 0.0 };
            let opp_set_size = opp.live_card_zone.cards.len();
            let mut best: Option<(f64, Vec<usize>)> = None;
            for (ev, score, idxs) in &ranked {
                let pass = if *score > 0 { *ev / *score as f64 } else { 1.0 };
                let value = expected_live_value(
                    pass,
                    *score,
                    idxs.len(),
                    my_succ,
                    opp_succ,
                    opp_pass,
                    e_opp,
                    opp_set_size,
                );
                if best.as_ref().is_none_or(|current| value > current.0) {
                    best = Some((value, idxs.clone()));
                }
            }
            if let Some((value, idxs)) = best {
                log::debug!(
                    "v7 payoff-model t{} me{} value={:.3} idxs={:?}",
                    gs.turn_number,
                    me,
                    value,
                    idxs
                );
                desired = idxs;
                chose_single = true;
            }
        }
        if !chose_single && ceiling_enabled {
            let e_opp = public_opponent_ceiling(gs, me, db);
            let best_score = ranked.iter().map(|(_, s, _)| *s).max();
            if let Some(best_score) = best_score {
                let gap = e_opp - best_score;
                if gap > 2 && my_succ < 2 && opp_succ < 2 {
                    if let Some(&(_, _, first_hi, _)) = singles.first().filter(|s| s.0 >= floor) {
                        desired.push(first_hi);
                        chose_single = true;
                        log::debug!(
                            "v7 ceiling-single t{} me{} e_opp={} best={} gap={} hi={} p={:.2}",
                            gs.turn_number,
                            me,
                            e_opp,
                            best_score,
                            gap,
                            first_hi,
                            singles.first().map(|s| s.0).unwrap_or(0.0)
                        );
                    } else if !opp_committed {
                        log::debug!(
                            "v7 ceiling-fold t{} me{} e_opp={} best={} gap={} floor={:.2}",
                            gs.turn_number,
                            me,
                            e_opp,
                            best_score,
                            gap,
                            floor
                        );
                        return emit(gs, actions, &desired);
                    }
                }
            }
        }
        if ceiling_enabled && std::env::var_os("V7_NO_MIN_WIN").is_none() {
            let e_opp = public_opponent_ceiling(gs, me, db);
            let mut best: Option<(f64, i32, Vec<usize>)> = None;
            for (ev, score, idxs) in &ranked {
                let required = e_opp + i32::from(idxs.len() >= 2);
                if *score < required {
                    continue;
                }
                let probability = if *score > 0 { *ev / *score as f64 } else { 1.0 };
                let replace = if std::env::var_os("V7_NO_TRUE_MIN_WIN").is_none() {
                    best.as_ref().is_none_or(|current| {
                        *score < current.1
                            || (*score == current.1
                                && (probability > current.0 + f64::EPSILON
                                    || ((probability - current.0).abs() <= f64::EPSILON
                                        && idxs.len() < current.2.len())))
                    })
                } else {
                    best.as_ref().is_none_or(|current| {
                        probability > current.0 + f64::EPSILON
                            || ((probability - current.0).abs() <= f64::EPSILON
                                && (*score < current.1
                                    || (*score == current.1 && idxs.len() < current.2.len())))
                    })
                };
                if replace {
                    best = Some((probability, *score, idxs.clone()));
                }
            }
            if let Some((probability, score, idxs)) = best {
                desired = idxs;
                chose_single = true;
                log::debug!(
                    "v7 minimum-win t{} me{} e_opp={} score={} p={:.2} idxs={:?}",
                    gs.turn_number,
                    me,
                    e_opp,
                    score,
                    probability,
                    desired
                );
            }
        }
        if !chose_single
            && my_succ >= 2
            && std::env::var_os("V7_NO_STRICT_CLOSE").is_none()
            && std::env::var_os("V7_PRE_D").is_none()
        {
            let e_opp = public_opponent_ceiling(gs, me, db);
            let best_score = ranked.first().map(|(_, score, _)| *score).unwrap_or(0);
            if best_score < e_opp {
                if let Some(&(_, _, first_hi, _)) = singles.first().filter(|s| s.0 >= floor) {
                    desired.push(first_hi);
                    chose_single = true;
                    log::debug!(
                        "v7 strict-close t{} me{} e_opp={} best={} hi={} p={:.2}",
                        gs.turn_number,
                        me,
                        e_opp,
                        best_score,
                        first_hi,
                        singles.first().map(|s| s.0).unwrap_or(0.0)
                    );
                }
            }
        }
        if !chose_single {
            if let Some((_, _, idxs)) = ranked.first() {
                desired = idxs.clone();
            } else if let Some(&(_, _, first_hi, _)) = singles.first().filter(|s| s.0 >= floor) {
                desired.push(first_hi);
            }
        }
    } else if let Some(&(_, _, first_hi, _)) = singles.first().filter(|s| s.0 >= floor) {
        desired.push(first_hi);
        let lives = experiment_lives(gs, me, db);
        let mut covered = lives
            .iter()
            .find(|(h, _, _)| *h == first_hi)
            .map(|(_, _, n)| *n)
            .unwrap_or([0i32; 11]);
        let cats = experiment_flip_categories(gs, me, db);
        let blades = experiment_blades(gs, me, db);
        let board = experiment_board_pool(gs, me, db);
        let max_slots = usize::from(3u8.saturating_sub(my.live_card_set_limit_reduction));
        for &(_, _, hi, need) in singles.iter().skip(1) {
            if desired.len() >= max_slots {
                break;
            }
            let mut grown = covered;
            for k in 0..11 {
                grown[k] += need[k];
            }
            let pg = experiment_pass_probability(&cats.0, cats.1, blades, &board, &grown);
            if pg >= 0.85 {
                desired.push(hi);
                covered = grown;
            }
        }
    }

    if contested && std::env::var("V7_ROLLOUT_ASSIST").is_ok() {
        let candidates = crate::bot::rollout::enumerate_candidates(gs, me, db);
        if !candidates.is_empty() {
            let index = crate::bot::rollout::price_portfolios(gs, me, &candidates, actions);
            desired = candidates[index].clone();
        }
    }

    // D2b desperation life: MEASURED 2026-09-23 (seed 11, 3000×2) — picking a
    // below-floor life instead of pure junk cost ~1pp vs v6 (3175-2437 vs
    // ceiling-only 3231-2373). Below-floor lives lose the card for near-zero
    // place chance; junk at least draws replacements. OFF by default;
    // V7_D2B=1 re-enables for ablation.
    let d2b_enabled = std::env::var("V7_D2B").is_ok()
        && std::env::var_os("V7_PRE_D").is_none()
        && std::env::var_os("V7_PURE_JUNK").is_none();
    if desired.is_empty() && d2b_enabled {
        // D2b: never emit a pure-junk set. Gamble one near-miss life at the
        // usual floor; if even that fails, still take the best single life
        // (p > 0) — a failed life check at least contests; 3 junk cards
        // auto-fail while burning the turn. Reverse-gates: V7_PRE_D=1,
        // V7_PURE_JUNK=1.
        let gamble_floor = if opp_succ >= 2 { 0.10 } else { 0.25 };
        if let Some((_, _, hi, _)) = singles.first().filter(|single| single.0 >= gamble_floor) {
            desired.push(*hi);
        } else if let Some((p, _, hi, _)) = singles.iter().find(|single| single.0 > 0.0) {
            desired.push(*hi);
            log::debug!(
                "v7 desperation-life t{} me{} hi={} p={:.2} (below floor {:.2})",
                gs.turn_number,
                me,
                hi,
                p,
                gamble_floor
            );
        }
    } else if desired.is_empty() {
        let gamble_floor = if opp_succ >= 2 { 0.10 } else { 0.25 };
        if let Some((_, _, hi, _)) = singles.first().filter(|single| single.0 >= gamble_floor) {
            desired.push(*hi);
        }
    }

    let lives_before_junk = count_lives(gs, &desired, db);
    experiment_junk_fill(gs, me, db, &mut desired);
    let n_lives = count_lives(gs, &desired, db);
    log::debug!(
        "v7 experiment t{} me{} n={} lives={} junk={} contested={} floor={:.2} my{} opp{}",
        gs.turn_number,
        me,
        desired.len(),
        n_lives,
        desired.len() - n_lives,
        contested,
        floor,
        my_succ,
        opp_succ
    );
    debug_assert!(
        lives_before_junk == n_lives,
        "junk fill must never displace a life (D2b)"
    );
    if std::env::var("V7_TRACE").is_ok() {
        let score: i32 = desired
            .iter()
            .filter_map(|&hi| my.hand.cards.get(hi).copied())
            .filter_map(|cid| db.get_card(cid))
            .map(|c| c.score.unwrap_or(0) as i32)
            .sum();
        eprintln!(
            "V7LE t{} me{} n={} score={} my{} opp{}",
            gs.turn_number,
            me,
            desired.len(),
            score,
            my_succ,
            opp_succ
        );
    }
    emit(gs, actions, &desired)
}

pub fn live_set_audit_note(gs: &GameState) -> Option<String> {
    if !matches!(
        gs.current_phase,
        Phase::LiveCardSetFirstAttacker | Phase::LiveCardSetSecondAttacker
    ) {
        return None;
    }
    let me = gs.active_player_index();
    let (my, opp) = gs.seated_pair(me);
    let db = &gs.card_database;
    let selected = gs
        .live_card_selected_indices
        .iter()
        .filter_map(|&hi| my.hand.cards.get(hi as usize).copied())
        .map(|cid| {
            let card = db.get_card(cid);
            format!(
                "{}:{}:{}",
                card.map(|c| c.card_no.to_string()).unwrap_or_default(),
                card.map(|c| if c.card_type == CardType::Live {
                    "L"
                } else {
                    "J"
                })
                .unwrap_or("?"),
                card.and_then(|c| c.score).unwrap_or(0)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    Some(format!(
        "t{} me{} selected={} my{} opp{} opp_live={} second={}",
        gs.turn_number,
        me,
        selected,
        my.success_live_card_zone.cards.len(),
        opp.success_live_card_zone.cards.len(),
        opp.live_card_zone.cards.len(),
        gs.current_phase == Phase::LiveCardSetSecondAttacker
    ))
}

fn emit(gs: &GameState, actions: &[Action], desired: &[usize]) -> Action {
    crate::bot::strategy_common::emit_live_set(gs, actions, desired)
}

fn reachable_curve_keep(costs: &[Option<u8>]) -> Vec<usize> {
    let members: Vec<(usize, u8)> = costs
        .iter()
        .enumerate()
        .filter_map(|(index, cost)| cost.map(|cost| (index, cost)))
        .collect();
    let mut best = Vec::new();
    let mut best_rank = (0u32, 0u32, 0u32);
    for mask in 1u32..(1u32 << members.len()) {
        let line: Vec<(usize, u8)> = members
            .iter()
            .enumerate()
            .filter_map(|(bit, &(index, cost))| (mask & (1 << bit) != 0).then_some((index, cost)))
            .collect();
        if line.is_empty() || line.len() > 4 || line[0].1 > 4 {
            continue;
        }
        if line.len() >= 2 && line[0].1 + line[1].1 > 4 {
            continue;
        }
        if line
            .windows(2)
            .any(|pair| pair[1].1 < pair[0].1 || pair[1].1 > pair[0].1 + 6)
        {
            continue;
        }
        let rank = (
            line.len() as u32,
            line.last()
                .copied()
                .map(|(_, cost)| u32::from(cost))
                .unwrap_or(0),
            line.iter().map(|(_, cost)| u32::from(*cost)).sum(),
        );
        if rank > best_rank {
            best_rank = rank;
            best = line.into_iter().map(|(index, _)| index).collect();
        }
    }
    best.sort_unstable();
    best
}

fn opening_curve_keep(costs: &[Option<u8>]) -> Vec<usize> {
    let mut best = Vec::new();
    let mut best_rank = (0, 0, 0);
    for (a, first) in costs.iter().enumerate() {
        let Some(first) = first else { continue };
        for (b, second) in costs.iter().enumerate() {
            let Some(second) = second else { continue };
            if a == b || u16::from(*first) + u16::from(*second) > 4 {
                continue;
            }
            for (c, bridge) in costs.iter().enumerate() {
                let Some(bridge) = bridge else { continue };
                if c == a || c == b || bridge <= first || u16::from(*bridge) > u16::from(*first) + 5
                {
                    continue;
                }
                for (d, finish) in costs.iter().enumerate() {
                    let Some(finish) = finish else { continue };
                    if d == a
                        || d == b
                        || d == c
                        || finish <= bridge
                        || u16::from(*finish) > u16::from(*bridge) + 6
                    {
                        continue;
                    }
                    let rank = (*first + *second, *bridge, *finish);
                    if rank > best_rank {
                        best_rank = rank;
                        best = vec![a, b, c, d];
                    }
                }
            }
        }
    }
    best
}

/// Mulligan: v4 policy (measured stronger), with the curve-keep experiment
/// preserved behind `V7_MULLIGAN_CURVE=1` for reproduction.
///
/// MEASURED 2026-09-17 (5CP3Z idou mirror, 3000 games x both seats, seed 11):
/// curve-keep 2805 wins vs v4-mulligan 2853 across 6000 games — direction
/// consistent in both seats (A: 1358<1385, B: 1447<1468), i.e. a small real
/// regression, not noise. Audit analysis (200 games, analyze_audit.py): the
/// curve-keep path fires on only ~17% of hands (4 distinct members with a
/// connected 2->7->11 ladder are rare in this deck's live-heavy opens), and
/// when it does fire it preserves members whose value the one-ply Main eval
/// already captures from redraws. v4's expensive-first replacement stays.
fn choose_mulligan_curve(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let costs: Vec<Option<u8>> = gs
        .active_player()
        .hand
        .cards
        .iter()
        .map(|&id| {
            db.get_card(id).and_then(|card| {
                (card.card_type == CardType::Member)
                    .then_some(card.cost)
                    .flatten()
            })
        })
        .collect();
    let keep = if std::env::var_os("V7_MULLIGAN_REACHABLE").is_some() {
        reachable_curve_keep(&costs)
    } else {
        opening_curve_keep(&costs)
    };
    if keep.is_empty() {
        return crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db);
    }
    let mut desired = Vec::new();
    let mut lives = 0;
    for (index, &id) in gs.active_player().hand.cards.iter().enumerate() {
        if db
            .get_card(id)
            .is_some_and(|card| card.card_type == CardType::Live)
        {
            lives += 1;
            if lives > 3 {
                desired.push(index);
            }
        }
    }
    let mut members: Vec<(usize, u8)> = costs
        .iter()
        .enumerate()
        .filter_map(|(index, cost)| cost.map(|cost| (index, cost)))
        .filter(|(index, _)| !keep.contains(index))
        .collect();
    members.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost));
    for (index, _) in members {
        if desired.len() >= 3 {
            break;
        }
        desired.push(index);
    }
    log::debug!("v7 mulligan curve keep={:?} replace={:?}", keep, desired);
    for action in actions {
        if action.action_type == ActionType::SelectMulligan {
            if let Some(index) = action.parameters.as_ref().and_then(|p| p.card_index) {
                if action.selected == Some(!desired.contains(&index)) {
                    return action.clone();
                }
            }
        }
    }
    actions
        .iter()
        .find(|action| {
            matches!(
                action.action_type,
                ActionType::ConfirmMulligan | ActionType::SkipMulligan
            )
        })
        .or_else(|| actions.first())
        .cloned()
        .expect("mulligan actions non-empty")
}

pub fn choose_mulligan_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    if std::env::var("V7_MULLIGAN_CURVE").is_ok() || std::env::var("V7_MULLIGAN_REACHABLE").is_ok()
    {
        return choose_mulligan_curve(gs, actions, db);
    }
    crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db)
}

// Re-exports with generic names for the registry (bot/registry.rs): adding
// v8 never renames these, it only adds a dispatch line there.
pub use choose_action_v7 as choose_action;
pub use choose_live_set_v7 as choose_live_set;
pub use choose_mulligan_v7 as choose_mulligan;

#[cfg(test)]
mod mulligan_tests {
    use super::*;

    #[test]
    fn opening_curve_keep_finds_connected_ladder_only() {
        assert_eq!(
            opening_curve_keep(&[Some(2), Some(2), Some(7), Some(11), None, None]),
            vec![0, 1, 2, 3]
        );
        assert!(opening_curve_keep(&[Some(2), Some(2), Some(11), Some(17), None, None]).is_empty());
        assert!(opening_curve_keep(&[Some(2), Some(4), Some(7), Some(11), None, None]).is_empty());
    }

    #[test]
    fn reachable_curve_accepts_short_and_duplicate_cost_lines() {
        assert_eq!(
            reachable_curve_keep(&[Some(2), Some(2), Some(7)]),
            vec![0, 1, 2]
        );
        assert_eq!(
            reachable_curve_keep(&[Some(2), Some(2), Some(7), Some(13)]),
            vec![0, 1, 2, 3]
        );
        assert!(reachable_curve_keep(&[Some(7), Some(11)]).is_empty());
    }

    #[test]
    fn mulligan_default_matches_v4_and_curve_variant_recovers_selection() {
        let cards = crate::card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
            "../cards/cards.json",
        ))
        .unwrap();
        let db = crate::Arc::new(CardDatabase::load_or_create(cards));
        let names = [
            "PL!SP-bp1-005-R",
            "PL!SP-sd1-019-SD",
            "PL!SP-bp4-011-R＋",
            "PL!SP-bp5-006-R",
            "PL!SP-sd2-023-SD2",
            "PL!SP-bp4-025-L",
        ];
        let mut p1 = Player::new("p1".into(), "P1".into(), true);
        for name in names {
            let id = db.get_card_id(name).unwrap();
            assert_eq!(db.get_card(id).unwrap().card_no, name);
            p1.hand.add_card(id);
        }
        let p2 = Player::new("p2".into(), "P2".into(), false);
        let mut gs = GameState::new(p1, p2, crate::Arc::clone(&db));
        gs.current_phase = Phase::MulliganFirstAttacker;
        let actions = crate::game_setup::generate_possible_actions(&gs);
        let v4 = crate::bot::strategy_v4::choose_mulligan_v4(&gs, &actions, &db);
        assert_eq!(v4.parameters.as_ref().and_then(|p| p.card_index), Some(3));
        let default_choice = choose_mulligan_v7(&gs, &actions, &db);
        assert_eq!(default_choice.action_type, v4.action_type);
        assert_eq!(
            default_choice
                .parameters
                .as_ref()
                .and_then(|p| p.card_index),
            v4.parameters.as_ref().and_then(|p| p.card_index)
        );
        gs.mulligan_selected_indices.extend([3, 2, 0]);
        for _ in 0..3 {
            let actions = crate::game_setup::generate_possible_actions(&gs);
            let action = choose_mulligan_curve(&gs, &actions, &db);
            assert_eq!(action.action_type, ActionType::SelectMulligan);
            assert_eq!(action.selected, Some(true));
            crate::game_setup::execute_action(&mut gs, &action).unwrap();
        }
        assert!(gs.mulligan_selected_indices.is_empty());
        let actions = crate::game_setup::generate_possible_actions(&gs);
        assert_eq!(
            choose_mulligan_curve(&gs, &actions, &db).action_type,
            ActionType::ConfirmMulligan
        );
    }
}

#[cfg(test)]
mod live_experiment_tests {
    use super::*;

    fn db_real() -> crate::Arc<CardDatabase> {
        let cards = crate::card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
            "../cards/cards.json",
        ))
        .unwrap();
        crate::Arc::new(CardDatabase::load_or_create(cards))
    }

    fn second_attacker_gs(db: &crate::Arc<CardDatabase>) -> (GameState, Player, Player) {
        let p1 = Player::new("p1".into(), "P1".into(), true);
        let mut p2 = Player::new("p2".into(), "P2".into(), false);
        p2.is_first_attacker = false;
        let mut gs = GameState::new(p1.clone(), p2.clone(), crate::Arc::clone(db));
        gs.current_phase = Phase::LiveCardSetSecondAttacker;
        gs.current_turn_phase = crate::game_state::TurnPhase::Live;
        (gs, p1, p2)
    }

    fn add_member_deck(db: &CardDatabase, p: &mut Player, name: &str, n: usize) {
        let id = db.get_card_id(name).unwrap();
        assert_eq!(db.get_card(id).unwrap().card_no, name);
        for _ in 0..n {
            p.main_deck.cards.push(id);
        }
    }

    #[test]
    fn experiment_feasible_matches_engine_bucket_semantics() {
        let need = [0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut pool = [0i32; 8];
        pool[1] = 2;
        assert!(experiment_feasible(&pool, &need));
        pool[1] = 1;
        assert!(!experiment_feasible(&pool, &need));
        pool[7] = 1;
        assert!(experiment_feasible(&pool, &need));
        pool[7] = 0;
        pool[0] = 1;
        assert!(
            !experiment_feasible(&pool, &need),
            "colorless must not fill heart01"
        );
        let bucket_need = [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut pool2 = [0i32; 8];
        pool2[0] = 2;
        assert!(!experiment_feasible(&pool2, &bucket_need));
        pool2[0] = 6;
        assert!(experiment_feasible(&pool2, &bucket_need));
        let surplus_need = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut pool3 = [0i32; 8];
        pool3[1] = 2;
        assert!(
            experiment_feasible(&pool3, &surplus_need),
            "surplus feeds bucket"
        );
        pool3[1] = 0;
        pool3[2] = 2;
        assert!(
            !experiment_feasible(&pool3, &surplus_need),
            "wrong color cannot fill specific"
        );
    }

    #[test]
    fn experiment_pass_probability_is_per_color_and_deterministic() {
        let mut cats: Vec<([i32; 8], usize)> = Vec::new();
        let mut v1 = [0i32; 8];
        v1[1] = 1;
        cats.push((v1, 3));
        let mut v6 = [0i32; 8];
        v6[6] = 1;
        cats.push((v6, 3));
        let need = [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0];
        let board = [0i32; 8];
        let p = experiment_pass_probability(&cats, 6, 2, &board, &need);
        let p2 = experiment_pass_probability(&cats, 6, 2, &board, &need);
        assert!((p - p2).abs() < f64::EPSILON, "deterministic");
        assert!(
            (p - 0.75).abs() < 0.15,
            "two flips over a half h06 deck should land near P(>=1 of 2)=0.75, got {p}"
        );
        let h06_only = [0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0];
        let p_two = experiment_pass_probability(&cats, 6, 2, &board, &h06_only);
        assert!(
            p_two < p && (p_two - 0.2).abs() < 0.08,
            "without replacement: two h06 has probability 3/6 * 2/5, got {p_two}"
        );
        let need_zero = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            experiment_pass_probability(&cats, 6, 2, &board, &need_zero),
            1.0
        );
        let mut colorless = [0i32; 8];
        colorless[0] = 3;
        let bucket_need = [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            experiment_pass_probability(&cats, 6, 0, &colorless, &bucket_need),
            1.0
        );
        assert_eq!(
            experiment_pass_probability(&cats, 6, 0, &colorless, &need),
            0.0,
            "colorless cannot fill h06 even with pool"
        );
    }

    #[test]
    fn experiment_ignores_deck_order_and_opponent_hidden_cards() {
        let db = db_real();
        let (mut gs, _, _) = second_attacker_gs(&db);
        let live = db.get_card_id("PL!-sd1-019-SD").unwrap();
        assert_eq!(db.get_card(live).unwrap().card_no, "PL!-sd1-019-SD");
        gs.player2.hand.cards.push(live);
        add_member_deck(&db, &mut gs.player2, "PL!-sd1-005-SD", 8);
        add_member_deck(&db, &mut gs.player2, "PL!-sd1-002-SD", 8);
        let member = db.get_card_id("PL!-sd1-010-SD").unwrap();
        gs.player2.stage.stage = [member, -1, -1];
        gs.mods.set_blade_modifier(member, 8);
        let before = experiment_flip_categories(&gs, 1, &db);
        let rank = experiment_portfolio_rank(&gs, 1, &db);
        gs.player2.main_deck.cards.reverse();
        gs.player1.hand.cards.extend([live, member]);
        gs.player1.main_deck.cards.extend([member, live, live]);
        assert_eq!(before, experiment_flip_categories(&gs, 1, &db));
        assert_eq!(rank, experiment_portfolio_rank(&gs, 1, &db));
        assert!(experiment_free_win(&gs, 1, &db).is_none());
        gs.mods.heart_modifiers.entry(member).or_default().insert(
            crate::card::HeartColor::Heart06,
            crate::core::game_modifiers::ModifierEntry {
                additive: 1,
                ..Default::default()
            },
        );
        assert_eq!(experiment_free_win(&gs, 1, &db), Some(0));
        let mut guard = 0;
        while gs.current_phase == Phase::LiveCardSetSecondAttacker && guard < 4 {
            let actions = crate::game_setup::generate_possible_actions(&gs);
            let action = choose_live_set_experiment(&gs, &actions, &db);
            crate::game_setup::execute_action(&mut gs, &action).unwrap();
            guard += 1;
        }
        assert!(guard < 4, "selection must converge to confirmation");
    }

    #[test]
    fn free_win_requires_true_board_coverage_not_mean_flips() {
        let db = db_real();
        let live = db.get_card_id("PL!-sd1-019-SD").unwrap();
        let need = db.get_card(live).unwrap().need_heart.clone().unwrap();
        assert_eq!(need.hearts.len(), 3);

        let id005 = db.get_card_id("PL!-sd1-005-SD").unwrap();
        let id010 = db.get_card_id("PL!-sd1-010-SD").unwrap();
        let id002 = db.get_card_id("PL!-sd1-002-SD").unwrap();

        let (mut gs, _p1, mut p2) = second_attacker_gs(&db);
        p2.hand.add_card(live);
        add_member_deck(&db, &mut p2, "PL!-sd1-005-SD", 8);
        add_member_deck(&db, &mut p2, "PL!-sd1-010-SD", 8);
        add_member_deck(&db, &mut p2, "PL!-sd1-002-SD", 8);
        p2.stage.stage = [id005, id010, id002];
        gs.player1 = _p1;
        gs.player2 = p2;
        let actions = crate::game_setup::generate_possible_actions(&gs);
        let chosen = choose_live_set_experiment(&gs, &actions, &db);
        assert_eq!(
            chosen.action_type,
            ActionType::SelectLiveCard,
            "full board coverage must take the free win"
        );

        let (mut gs2, p1b, mut p2b) = second_attacker_gs(&db);
        p2b.hand.add_card(live);
        add_member_deck(&db, &mut p2b, "PL!-sd1-005-SD", 12);
        add_member_deck(&db, &mut p2b, "PL!-sd1-010-SD", 12);
        p2b.stage.stage = [id010, -1, -1];
        gs2.player1 = p1b;
        gs2.player2 = p2b;
        let actions2 = crate::game_setup::generate_possible_actions(&gs2);
        let chosen2 = choose_live_set_experiment(&gs2, &actions2, &db);
        assert_ne!(
            chosen2.action_type,
            ActionType::SelectLiveCard,
            "h06 missing from board and no b_heart06 flips: must not set the live"
        );
    }
}
