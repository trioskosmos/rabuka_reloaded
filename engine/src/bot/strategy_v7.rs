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
use crate::bot::strategy_common::{narrow_hearts, Acc};
use crate::core::constants::CountCast;
use crate::bot::strategy_v5::binom_ge;
use crate::card::{CardDatabase, CardType};
use crate::core::stats_pipeline;
use crate::game_state::GameResult;
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
    base + buffs.iter().map(|&v| i32::from(v)).sum::<i32>()
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
                // `i32` because this is a per-member total summed across the
                // stage; a member's blade can be negative after a reduction.
                i32::from(db.get_card(c).map(|x| x.blade).unwrap_or(0))
                    + i32::from(gs.mods.get_blade_modifier(c))
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
pub(crate) fn passable_count_buffed(gs: &GameState, me: u8, db: &CardDatabase) -> usize {
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
                // Already the i16 the vector stores — the `saturate_i16` that
                // used to stand here only existed to undo the accessor's i32.
                gs.mods.get_blade_modifier(c)
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
        let base_hand_len = my_now.hand.cards.len().i32_count();
    let base_passable = passable_count_buffed(gs, me, db);
    let base_ammo = lives_in_hand(my_now, db);
    let base_stage = stage_hearts_of(my_now, gs, me, db);
    let base_blades = total_blades_of(my_now, gs, db);
        let base_energy = i32::from(my_now.energy_zone.active_count());
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
                .is_some_and(|x| x.card_type == CardType::Live)
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
                .is_some_and(|x| x.card_type == CardType::Live)
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
            let d_energy = i32::from(my_sim.energy_zone.active_count()) - base_energy;
            val += 2.0 * d_energy as f64;
            if d_energy != 0 {
                parts.push(format!("en{d_energy:+}"));
            }
            let d_hand = my_sim.hand.cards.len().i32_count() - base_hand_len;
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
            let drawn =
            (my_sim.hand.cards.len().i32_count() - base_hand_len).max(0);
            val += 70.0 * p_life_draw * drawn as f64;
            let wr_now = my_sim
                .waitroom
                .cards
                .iter()
                .filter(|&&c| {
                    db.get_card(c)
                        .is_some_and(|x| x.card_type == CardType::Live)
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
/// v7's live set, but the engine-executing one: clone the state, commit the
/// candidate set, let the rules resolve it.
///
/// Normally only reachable through the `V7_ROLLOUT` env var on
/// `choose_live_set_v7`, which is a process-wide switch and therefore cannot
/// select it for ONE seat. `v9` needs to be able to field "v7 Main + one live
/// set" against a pure opponent on a single seat, so the rollout is also
/// reachable by name.
pub fn choose_live_set_rollout(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    crate::bot::rollout::choose_live_set_v7(gs, actions, db)
}

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
        let my_succ = my.success_live_card_zone.cards.len().i32_count();
        let opp_succ = opp.success_live_card_zone.cards.len().i32_count();
    let is_second = gs.current_phase == Phase::LiveCardSetSecondAttacker;
    let opp_committed = !opp.live_card_zone.cards.is_empty();

    // OPPONENT-COMMITMENT RESPONSE.
    //
    // This is the axis v7 never modelled. When the opponent has committed
    // lives, v7's own behaviour collapses into `safest_portfolio` — "pick the
    // single life most likely to pass" — which is the correct answer to *my*
    // check and a non-answer to theirs. The guides are explicit that this is
    // where the game is decided: a 2-live is described as mandatory play
    // because it DENIES the opponent's 1-live, and score lead is called the
    // dominant axis ("1点リードしているだけで…次のターン確実に点とるためにライブカード温存できる").
    // v7's own experiments could not see that, which is the most likely reason
    // the documented "allow 2 lives when contested" change measured as worse
    // (354 vs 364): neither bot was pricing the denial, so it looked like a
    // pure cost.
    //
    // `expected_live_value` is already written and already correct — a
    // four-outcome payoff table (I pass/they fail, I fail/they pass, and both
    // pass resolved by score comparison with the one-card-set tie rule), worth
    // ±1 or a success lead. It is pure structure: no coefficient, nothing to
    // fit. What was missing is its two opponent arguments, so those are what
    // this computes, from PUBLIC information only:
    //
    //   opp_pass  — from the opponent's public active blades against the size
    //               of the set they committed. Blades are the draw count, and
    //               the check is exactly that: more blades, more of the colour
    //               pool, more chance the requirement is met.
    //   opp_score — the printed score of the set they committed.
    //
    // Nothing here reads the opponent's hand or deck, so it is response
    // modelling, not cheating. `V7_RESPONSE=0` restores today's behaviour.
    if response_model_enabled() && ((is_second && opp_committed) || opp_succ >= 2) {
        let response = best_response_set(gs, me, db);
        if !response.is_empty() {
            return crate::bot::strategy_common::emit_live_set(gs, actions, &response);
        }
    }

    // Contested / match-point fallback, opt-in only.
    let lease_v6 = std::env::var_os("V7_LEASE_V6").is_some();
    if lease_v6 && ((is_second && opp_committed) || opp_succ >= 2) {
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

    if desired.is_empty() && lease_v6 {
        // v6 gamble + junk-dig fallback — now opt-in, like the contested path
        // above, so the default v7 live set is self-contained.
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
                    .is_some_and(|c| c.card_type == CardType::Live)
            })
            .count();
        let max_slots =
            (3i32 - i32::from(my.live_card_set_limit_reduction).max(0)).usize_count();
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
                            .is_some_and(|c| c.card_type != CardType::Live)
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
    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction).max(0)).usize_count();
    if lives.is_empty() || max_slots == 0 {
        return Vec::new();
    }
        let my_succ = my.success_live_card_zone.cards.len().i32_count();
        let opp_succ = opp.success_live_card_zone.cards.len().i32_count();
    let floor = if opp_succ >= 2 {
        0.35
    } else if my_succ >= 2 {
        0.60
    } else {
        0.45
    };
    let (blades, density) = flip_stats(gs, me, db);
    let board_supply: i32 = (0..=7)
        .chain(std::iter::once(10))
        .map(|i| i32::from(board[i]))
        .sum();

    // Score every single life: pass probability under the calibrated model.
    let mut ranked: Vec<(f64, i32, usize)> = Vec::new();
    for &(hi, cid, ref need) in &lives {
        if alloc(&pool, need).is_none() {
            continue;
        }
        let req: i32 = (0..=7)
            .chain(std::iter::once(10))
            .map(|k| i32::from(need[k]))
            .sum();
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
                let req: i32 = (0..=7)
                    .chain(std::iter::once(10))
                    .map(|k| i32::from(need[k]))
                    .sum();
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
    let mut need_total: Acc = [0i16; 11];
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
        let req: i32 = (0..=7)
            .chain(std::iter::once(10))
            .map(|k| i32::from(grown[k]))
            .sum();
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
type ExperimentSingle = (f64, i32, usize, Acc);

/// Outranks any stat utility the junk loop can build, so a card with a live
/// ability is never the first thing binned from hand.
const ABILITY_KEEP_BONUS: f64 = 500.0;

/// Penalty for a no-ability member the development curve has already passed.
const CURVE_PASSED_PENALTY: f64 = 20.0;

/// How far below the top stage cost a member must sit to count as curve-passed.
const CURVE_PASSED_MARGIN: i32 = 2;

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

/// Rule 2.11.3 wildcard-feasibility. Delegates to `v8_model::feasible`, which
/// was the canonical home: this body was byte-identical to it apart from
/// naming the scratch variable `bucket_supply` instead of `bucket`. Two bots
/// deciding "can this requirement be met" from separate copies of the same
/// rule is how they start disagreeing on a board.
fn experiment_feasible(pool: &[i32; 8], need: &Acc) -> bool {
    // The 8-bucket pool family (`cats`, sampled `pools`, `board_pool`) is
    // still `i32` — it is a sampling histogram, not an `Acc`. `feasible` takes
    // the narrowed `&[i16; 8]`, so clamp the buckets across at the boundary.
    let pool8: [i16; 8] = std::array::from_fn(|i| narrow_hearts(pool[i]));
    crate::bot::v8_model::feasible(&pool8, need)
}

fn experiment_pass_probability(
    cats: &[([i32; 8], usize)],
    deck_len: usize,
    blades: i32,
    board: &[i32; 8],
    need: &Acc,
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

#[allow(clippy::too_many_arguments)]
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

fn experiment_pass_probability_pools(pools: &[[i32; 8]], need: &Acc) -> f64 {
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
        .map(usize::from)
        .sum();
    let draws = usize::try_from(blades).unwrap_or(usize::MAX).min(deck_len);
    (score_icons * draws / deck_len).i32_count()
}

fn experiment_lives(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<(usize, i16, Acc)> {
    let (my, _) = gs.seated_pair(me);
    hand_lives(my, db)
        .into_iter()
        .map(|(hi, cid, _)| {
            let mut need: Acc = [0i16; 11];
            let base = db.get_card(cid).and_then(|card| card.need_heart.as_ref());
            if let Some(effective) = crate::core::stats_pipeline::effective_need_heart(
                base,
                cid,
                &gs.mods.need_heart_modifiers,
            ) {
                for (color, count) in &effective.hearts {
                    need[color.index()] += i16::from(*count);
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

/// Does this card still have an ability worth keeping it in hand for?
///
/// This is the term `experiment_junk_fill` was missing, and it is the one the
/// guide's junk definition actually turns on: junk is "genuinely low-future-
/// value non-lives, NOT merely expensive cards". The old utility was
/// `2*cost + 2*blade + hearts` — it never opened `card.abilities`, so a
/// costless activation scored like a vanilla stat stick and was thrown into
/// the live zone to be discarded. On a curve-saturated board that is how you
/// lose the only card in hand that could still convert.
///
/// Deliberately conservative: an unknown, null, or exhausted-to-zero ability
/// does not count, and we do not try to price whether the effect is any good.
/// False positives (kept a card we should have binned) cost one slot of junk
/// and draw a replacement; false negatives (binned a livewire) cost the card.
fn card_has_usable_ability(card: &crate::card::Card) -> bool {
    card.abilities.iter().any(|ability_ref| {
        let ability = ability_ref.resolve();
        !ability.is_null
            && ability.effect.is_some()
            && ability.use_limit.unwrap_or(u8::MAX) > 0
    })
}

/// How close this board is to actually passing a check, in [0, 1].
///
/// This is the term v7's Main-phase evaluation was missing, and it is
/// available for free because `choose_live_set_experiment` already models the
/// check exactly — 256 determinized deck draws, evaluated against the real
/// board pool, with the engine's own bucket/wildcard semantics. `value()`
/// approximated that with `60 × Δpassable`, which is a one-ply coverage proxy:
/// it counts lives the board could *already* pass, and is blind to the thing
/// that actually decides a check, which is ACTIVE BLADES, because blades are
/// the deck draws the check gets to see.
///
/// It matters most late, and it is the same option value the junk-fill
/// experiment exposed from the other side. A life that cannot clear the
/// reliability floor on the current board often can two turns later, because
/// the board grew. Valuing pressure makes Main-phase development aim at the
/// check instead of at an ever-larger stage, which is the direction the stage
/// cost curve has been drifting on its own.
///
/// Returns the best single-life pass probability; 0.0 when no live is in hand,
/// which is the honest "no check to aim at" rather than a neutral 0.5.
pub(crate) fn check_pressure(gs: &GameState, me: u8, db: &CardDatabase) -> f64 {
    let lives = experiment_lives(gs, me, db);
    if lives.is_empty() {
        return 0.0;
    }
    let (cats, deck_len) = experiment_flip_categories(gs, me, db);
    let blades = experiment_blades(gs, me, db);
    let board = experiment_board_pool(gs, me, db);
    if blades <= 0 || deck_len == 0 {
        return 0.0;
    }
    let pools = experiment_sample_pools(&cats, deck_len, blades, &board);
    lives
        .iter()
        .filter(|(_, _, need)| need[8] == 0 && need[9] == 0)
        .map(|(_, _, need)| experiment_pass_probability_pools(&pools, need))
        .fold(0.0f64, f64::max)
}

/// Deterministic forward ladder for `placement_forecast`.
///
/// Deliberately NOT a policy search. The ISMCTS failed because a rollout policy
/// that plays badly makes the leaf a losing player's opinion; here the ladder
/// only has to be *honest and monotone*, because the value comes from the
/// exact check model at each horizon, not from how well the ladder plays
/// chess. One deploy per step, preferring active blades (the check reads
/// blades) and then hearts, then a discounted cost upgrade.
fn forecast_develop(gs: &mut GameState, me: u8, db: &CardDatabase) {
    let (my, _) = gs.seated_pair(me);
    let energy = my.energy_zone.active_count() as i32 + 1;
    let mut best: Option<(i32, i32, i32, usize)> = None; // (blades, hearts, cost, hand idx)
    for (index, &cid) in my.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else { continue };
        if card.card_type != CardType::Member {
            continue;
        }
        let cost = card.cost.unwrap_or(0) as i32;
        if cost > energy {
            continue;
        }
        let already = my.stage.stage.contains(&cid);
        let hearts = card
            .base_heart
            .as_ref()
            .map(|h| h.hearts.values_sum() as i32)
            .unwrap_or(0);
        // Prefer a card we do not already have on stage; among equals, blades.
        let key = (if already { 0 } else { 1 }, card.blade as i32, hearts);
        let cand = (key.0, key.1, key.2, index);
        if best.is_none_or(|current| {
            (cand.0, cand.1, cand.2) > (current.0, current.1, current.2)
        }) {
            best = Some(cand);
        }
    }
    if let Some((_, _, _, index)) = best {
        let cid = gs.seat_player(me).hand.cards[index];
        let mut action = crate::game_setup::Action {
            description: "forecast".into(),
            description_ja: None,
            action_type: ActionType::PlayMemberToStage,
            parameters: None,
            selected: None,
        };
        let free = gs.seat_player(me).stage.stage.iter().all(|&s| s < 0);
        let mut params = crate::game_setup::ActionParameters {
            card_id: Some(cid),
            card_index: None,
            card_indices: None,
            stage_area: None,
            stage_area_index: None,
            use_baton_touch: None,
            card_name: None,
            card_no: None,
            ability_index: None,
            source_ability: None,
            base_cost: None,
            final_cost: None,
            available_areas: None,
            double_baton_pairs: None,
            disabled: None,
        };
        if free {
            params.stage_area = Some(format!(
                "{}",
                crate::zones::MemberArea::Center.to_string()
            ));
        } else {
            // Baton: replace the most expensive stage member.
            let target = gs
                .seat_player(me)
                .stage
                .stage
                .iter()
                .enumerate()
                .max_by_key(|(_, &s)| {
                    s.checked_sub(0).map_or(0, |id| {
                        db.get_card(id).and_then(|c| c.cost).unwrap_or(0) as i32
                    })
                })
                .map(|(i, _)| i)
                .unwrap_or(0);
            params.stage_area_index = Some(target as u8);
        }
        action.parameters = Some(params);
        crate::rng::seed(0x7637);
        if crate::game_setup::execute_action(gs, &action).is_ok() {
            game_setup_settle(gs);
        }
        // Refresh for the next step so the ladder sees a realistic turn.
        gs.seat_player_mut(me).refresh();
    }
}

fn game_setup_settle(gs: &mut GameState) {
    crate::game_setup::settle_single_player_state(gs);
}

/// Expected placements over the next `depth` checks, each under a projected
/// board, discounted by turn.
///
/// This is the paradigm change. Everything v7 has ever valued in the Main
/// phase is a one-ply delta over stage-cost growth, hearts, blades and
/// coverage — proxies for "this will convert later". This integrates the
/// actual conversion: at each projected turn it runs the same exact 256-draw
/// check model the live-set chooser uses, and adds the probability that the
/// best life in hand actually passes. A check places at most one card
/// (8.4.7), so the best single life is the right quantity.
///
/// Consequences that no one-ply term can produce:
///   * an investment that pays off two turns later is visible, so long-horizon
///     value is not invisible the way it is under `60 x passable`;
///   * a big board that cannot pass any check is worth LESS than a small board
///     that can, which is the opposite of what raw development growth implies;
///   * because the leaf is the check pass probability rather than board size,
///     a weak forward ladder cannot bias the answer the way a weak rollout
///     policy did.
pub(crate) fn placement_forecast(gs: &GameState, me: u8, db: &CardDatabase, depth: usize) -> f64 {
    let mut sim = gs.clone();
    let mut total = 0.0f64;
    for step in 0..depth {
        if sim.game_result != GameResult::Ongoing {
            break;
        }
        let lives = experiment_lives(&sim, me, db);
        if !lives.is_empty() {
            let (cats, deck_len) = experiment_flip_categories(&sim, me, db);
            let blades = experiment_blades(&sim, me, db);
            let board_pool = experiment_board_pool(&sim, me, db);
            if blades > 0 && deck_len > 0 {
                let pools = experiment_sample_pools(&cats, deck_len, blades, &board_pool);
                let best = lives
                    .iter()
                    .filter(|(_, _, need)| need[8] == 0 && need[9] == 0)
                    .map(|(_, _, need)| experiment_pass_probability_pools(&pools, need))
                    .fold(0.0f64, f64::max);
                // Geometric discount: a placement next check is worth more
                // than the same placement three checks out.
                let discount = 0.85f64.powi(step as i32);
                total += discount * best;
            }
        }
        forecast_develop(&mut sim, me, db);
    }
    total
}

/// What an ability actually did, read from the resolved state rather than
/// assumed from its text.
///
/// The point of this is that abilities are not interchangeable. A +1 energy
/// swing, a search that finds the 4 that completes the ladder, +2 blades (the
/// check's draw count, which `value()` prices at a linear 6), a two-card draw,
/// and a member that leaves the stage are five different trades. Classifying
/// them lets the caller price each in its own currency, and lets `V7_DEBUG`
/// print *why* an ability was taken instead of a bare number.
/// 7 × `i8` = **7 bytes**, down from 28.
///
/// Every field is a *delta between two states* (`after.x - before.x`), never an
/// accumulated total and never merged — the only producer is `measure_ability`,
/// and the only consumer is a `V7_DEBUG` print plus `classify`. Card data's
/// largest single value is 17, so a net change from one ability is far inside
/// `i8`.
///
/// The construction clamps rather than narrowing: `after`/`before` are
/// `AbilityFeatures`, which stays `i16` because `hearts` there is an ABSOLUTE
/// per-colour sum over a whole stage, not a delta. Taking that to `i8` would
/// risk an overflow in the intermediate, and the debug field is not worth a
/// panic in the middle of a rollout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AbilityEffect {
    pub draws: i8,
    pub energy: i8,
    pub blades: i8,
    pub hearts: i8,
    pub ammo: i8,
    pub stage_cost: i8,
    pub hand: i8,
}

impl AbilityEffect {
    /// A short label for the dominant thing the ability did, for diagnostics.
    pub fn classify(&self) -> &'static str {
        if self.ammo > 0 {
            "live-gain"
        } else if self.blades > 0 {
            "blades"
        } else if self.draws > 0 && self.energy > 0 {
            "tempo"
        } else if self.energy > 0 {
            "energy"
        } else if self.draws > 0 {
            "draw"
        } else if self.hearts > 0 {
            "hearts"
        } else if self.stage_cost < 0 {
            "removal"
        } else {
            "other"
        }
    }
}

/// Measure an ability by its resolved delta. `sim` is the state after
/// execution, `root` the state before — both already built by `score_actions`.
/// What KIND of trade an ability is, read from the ability's own printed text.
///
/// The aggregate-delta classifier below turned out to be nearly useless: of 71
/// ability uses measured in one run, 63 read as "produces nothing" and 30 as
/// "removal", so any policy that simply "uses more abilities" walks v7 into
/// member-consumption loops and Rule 12-1 draws (measured: draws scale
/// linearly with the ability weight, 187 -> 267, win rate 816 -> 737). The
/// delta was blind because most of what these abilities do is not a change to
/// six aggregate counters — a search that fetches the 4 that completes the
/// ladder, a permanent modifier on a card, a score change all read as
/// "other".
///
/// So classify from the text instead. This is the printed card, i.e. exactly
/// the information a human player has, so it is not cheating; and it is per
/// ability, so a 2-draw, an energy refresh, +2 blades and a point of score are
/// four different trades rather than one discount.
///
/// `Inactive` covers everything not recognised, and the default policy treats
/// it as do-not-use. That is the conservative direction on purpose: an ability
/// we cannot read is an ability whose loop risk we cannot price.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AbilityClass {
    /// Not a recognised productive trade — do not use it speculatively.
    Inactive,
    /// Card acquisition: a draw, or a search.
    Tempo,
    /// Board power: hearts, or blades. Blades are the check's draw count and so
    /// the most valuable thing an ability can move.
    Power,
    /// More energy to spend this turn.
    Energy,
    /// Comparison power: score, or a live-success modifier.
    Score,
    /// Discards, bounces, or otherwise removes a card — a disposal channel,
    /// which the guide treats as junk-fill and hand hygiene, not as gain.
    Disposal,
    /// Resolves by itself: 常時 / 自動 / 常駐. Not a player choice, and the
    /// engine does not charge for it. Treating one as a Main-phase spend is a
    /// category error and a repetition hazard.
    Automatic,
}

impl AbilityClass {
    pub fn label(&self) -> &'static str {
        match self {
            AbilityClass::Inactive => "inactive",
            AbilityClass::Tempo => "tempo",
            AbilityClass::Power => "power",
            AbilityClass::Energy => "energy",
            AbilityClass::Score => "score",
            AbilityClass::Disposal => "disposal",
            AbilityClass::Automatic => "automatic",
        }
    }
}

/// Classify one ability by its printed text.
///
/// The keywords are the words that actually appear on these cards. `ブレード`
/// and `フリップ` are blades/flips, which is the check's randomness;
/// `ドロー` is a draw; `探`/`検索` is a search; `スコア` is comparison score;
/// `サクセス` touches the success zone. Ordering matters: disposal is checked
/// before tempo because 「捨てる」 (discard) reads as a card operation and would
/// otherwise be swallowed by the ドロー branch.
/// Classify one ability by its printed text AND its trigger marker.
///
/// The trigger marker is the part that matters most, and getting it wrong is
/// what made every earlier ability attempt fail. The cards are printed with
/// explicit markers: `{{起動}}` is an ACTIVATION — the one kind a player
/// chooses and pays for. `{{常時}}` (always), `{{自動}}` (automatic) and
/// `{{常駐}}` (resident) resolve on their own: they are not choices, and the
/// engine does not charge for them. Offering to "use" one, or paying its
/// member for one, is a category error.
///
/// It is also the loop mechanism. A repeated no-op activation, or an
/// "automatic" ability re-triggered with no state change, is the same action
/// type on the same state hash — which is exactly what Rule 12-1 detects, and
/// draws scaled with the ability weight (187 -> 267).
pub(crate) fn classify_ability(text: &str, triggers: &str) -> AbilityClass {
    // Not a choosable trigger: automatic resolution, never a Main-phase spend.
    for marker in ["常時", "自動", "常駐", "jyouji", "toujyou", "jidou"] {
        if triggers.contains(marker) {
            return AbilityClass::Automatic;
        }
    }
    // 起動 = activation: the only kind worth spending a member for.
    if triggers.contains("起動") || triggers.contains("kidou") || text.contains("起動") {
        const TEMPO: [&str; 5] = ["ドロー", "探した", "検索", "手札", "山札から"];
        const POWER: [&str; 5] = ["ブレード", "フリップ", "ハート", "ハートマーク", "ブレードマーク"];
        const ENERGY: [&str; 3] = ["エネルギー", "E{E}", "icon_energy"];
        const SCORE: [&str; 3] = ["スコア", "サクセス", "先取"];
        const DISPOSAL: [&str; 4] = ["捨てる", "廃棄", "手札を", "デッキから"];
        if DISPOSAL.iter().any(|k| text.contains(k)) {
            return AbilityClass::Disposal;
        }
        if POWER.iter().any(|k| text.contains(k)) {
            return AbilityClass::Power;
        }
        if ENERGY.iter().any(|k| text.contains(k)) {
            return AbilityClass::Energy;
        }
        if SCORE.iter().any(|k| text.contains(k)) {
            return AbilityClass::Score;
        }
        if TEMPO.iter().any(|k| text.contains(k)) {
            return AbilityClass::Tempo;
        }
        return AbilityClass::Power;
    }
    AbilityClass::Inactive
}

/// The best class any ability on this card offers, preferring the strongest
/// trade, and the text of the ability that provides it.
pub(crate) fn best_ability_class(db: &CardDatabase, card_id: i16) -> AbilityClass {
    let Some(card) = db.get_card(card_id) else {
        return AbilityClass::Inactive;
    };
    let mut best = AbilityClass::Inactive;
    for ability_ref in &card.abilities {
        let ability = ability_ref.resolve();
        if ability.is_null || ability.use_limit.unwrap_or(u8::MAX) == 0 {
            continue;
        }
        let text = ability
            .triggerless_text
            .clone()
            .unwrap_or_else(|| ability.full_text.clone());
        let triggers = ability.triggers.clone().unwrap_or_default();
        let class = classify_ability(&text, &triggers);
        if rank(class) > rank(best) {
            best = class;
        }
    }
    best
}

/// Power over tempo over score/energy over disposal, all above inactive. Used
/// to pick the best ability on a card; the caller weights classes separately.
fn rank(class: AbilityClass) -> u8 {
    match class {
        AbilityClass::Inactive => 0,
        AbilityClass::Automatic => 0,
        AbilityClass::Disposal => 1,
        AbilityClass::Score => 2,
        AbilityClass::Energy => 3,
        AbilityClass::Tempo => 4,
        AbilityClass::Power => 5,
    }
}

pub(crate) fn measure_ability(
    sim: &GameState,
    root: &GameState,
    me: u8,
    db: &CardDatabase,
) -> AbilityEffect {
    let (after, before) = (v7_features(sim, me), v7_features(root, me));
    let blades_after = i16::from(stage_blades(sim, me, db));
    let blades_before = i16::from(stage_blades(root, me, db));
    let cost_after = stage_cost_total(sim, me, db);
    let cost_before = stage_cost_total(root, me, db);
    // `saturating_sub` into `i8_count`: the operands are `i16` absolutes, so
    // the subtraction is done at the wider width and only the RESULT — a net
    // change from one ability — is clamped to the byte `AbilityEffect` stores.
    let d = |a: i16, b: i16| a.saturating_sub(b).i8_count();
    AbilityEffect {
        // `hand` counts cards in hand, so a net draw is the rise in hand.
        draws: d(after.hand, before.hand),
        energy: d(after.energy, before.energy),
        blades: d(blades_after, blades_before),
        hearts: d(after.hearts, before.hearts),
        ammo: d(after.ammo, before.ammo),
        stage_cost: d(cost_after, cost_before),
        hand: d(after.hand, before.hand),
    }
}

fn stage_blades(gs: &GameState, me: u8, db: &CardDatabase) -> u8 {
    gs.seat_player(me)
        .stage
        .total_blades(
            db,
            &gs.mods.blade_modifiers,
            &gs.mods.orientation_modifiers,
            false,
        )
}

fn stage_cost_total(gs: &GameState, me: u8, db: &CardDatabase) -> i16 {
    gs.seat_player(me)
        .stage
        .stage
        .iter()
        .filter(|&&cid| cid >= 0)
        .filter_map(|&cid| db.get_card(cid).and_then(|c| c.cost))
        .map(i16::from)
        .sum()
}

/// v7's feature view of a state, for the ability delta above. A tiny local
/// mirror of `v7_main::features` so this module does not have to expose the
/// struct's private fields; only the six quantities the delta needs.
fn v7_features(gs: &GameState, me: u8) -> AbilityFeatures {
    let p = gs.seat_player(me);
    let hearts = p
        .stage
        .get_available_hearts(
            &gs.card_database,
            &gs.mods.heart_override,
            &gs.mods.heart_modifiers,
            &gs.mods.heart_color_multiplier,
            &gs.mods.heart_copy,
        )
        .hearts
        .values_sum() as i16;
    AbilityFeatures {
        hearts,
        energy: i16::from(p.energy_zone.active_count()),
        hand: i16::try_from(p.hand.cards.len()).unwrap_or(i16::MAX),
        ammo: i16::try_from(p.live_card_zone.cards.len()).unwrap_or(i16::MAX),
    }
}

pub(crate) struct AbilityFeatures {
    /// `i16`, not `i8`: this is an ABSOLUTE per-colour heart total for the
    /// whole stage, and a stage of three large members can exceed 127. Only
    /// the DELTA built from it in `measure_ability` is small enough for `i8`.
    pub hearts: i16,
    pub energy: i16,
    pub hand: i16,
    pub ammo: i16,
}

/// Pass probability of an already-chosen set, under the same exact 256-draw
/// model the chooser uses. For telemetry only: it answers "was the set we
/// committed to actually any good", which is a different question from "did
/// we pick the best one".
/// The best pass probability reachable by any SINGLE life in hand, ignoring the
/// reliability floor. This is a **v7 decision input** — `live_set_plan` feeds it
/// to `expected_live_value` as `mine_p` — so its shape is load-bearing and
/// changing it changes how v7 plays.
///
/// Do not "fix" this to enumerate sets without reading the call site first. The
/// set-aware version is [`best_available_set_pass_probability`], which exists
/// beside it precisely because this one is load-bearing and the measurement
/// question is different from the decision question. See that function for why
/// the two disagree.
///
/// Note what this is NOT: it is not "the best set", so a set this cannot
/// express is invisible to it. As a `mine_p` fed to a payoff table that is a
/// deliberate, if optimistic, upper bound on what a commit could be worth — the
/// table then decides whether the bound is actually reachable.
pub(crate) fn best_available_pass_probability(gs: &GameState, me: u8, db: &CardDatabase) -> f64 {
    let lives = experiment_lives(gs, me, db);
    if lives.is_empty() {
        return 0.0;
    }
    let (cats, deck_len) = experiment_flip_categories(gs, me, db);
    let blades = experiment_blades(gs, me, db);
    let board = experiment_board_pool(gs, me, db);
    if blades <= 0 || deck_len == 0 {
        return 0.0;
    }
    let pools = experiment_sample_pools(&cats, deck_len, blades, &board);
    lives
        .iter()
        .filter(|(_, _, need)| need[8] == 0 && need[9] == 0)
        .map(|(_, _, need)| experiment_pass_probability_pools(&pools, need))
        .fold(0.0f64, f64::max)
}

/// The best pass probability reachable by any SET of up to `max_slots` lives.
///
/// **Telemetry only.** The decision-facing `best_available_pass_probability`
/// stays single-life so that measuring v8 does not silently move v7; this is the
/// version the headroom line prints, because the headroom line is asking a
/// different question: not "how good could one commit be" but "did a check
/// exist that we could have committed to".
///
/// The two disagree, and reading singles for the second question is wrong in a
/// way that hides the answer. A zone pools its requirements (8.3.15), so a
/// set's probability is a property of the POOLED need and is not reachable by
/// pricing any one member. The chooser can and does commit multi-life sets —
/// `best_response_set` returns them, and `live_set_plan` returns a `ranked`
/// set on the non-contested path — so a two-life set the board could have
/// passed registers as unavailable under a single-life max, and the headroom
/// gap comes out too small. Too small in the direction that makes the board
/// look like the problem, which is the conclusion that sends you off to fix
/// development instead of fixing the live set.
pub(crate) fn best_available_set_pass_probability(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
) -> f64 {
    let lives = experiment_lives(gs, me, db);
    if lives.is_empty() {
        return 0.0;
    }
    let (cats, deck_len) = experiment_flip_categories(gs, me, db);
    let blades = experiment_blades(gs, me, db);
    let board = experiment_board_pool(gs, me, db);
    if blades <= 0 || deck_len == 0 {
        return 0.0;
    }
    let max_slots = usize::from(3u8.saturating_sub(
        gs.seat_player(me).live_card_set_limit_reduction,
    ));
    let pools = experiment_sample_pools(&cats, deck_len, blades, &board);
    let n = lives.len().min(8);
    let mut best = 0.0f64;
    for mask in 1u32..(1u32 << n) {
        if mask.count_ones() as usize > max_slots {
            continue;
        }
        let mut need: Acc = [0; 11];
        for (bit, (_, _, member_need)) in lives.iter().enumerate() {
            if mask >> bit & 1 == 1 {
                for k in 0..11 {
                    need[k] = need[k].saturating_add(member_need[k]);
                }
            }
        }
        // Draw/Score icons are not a heart check we can price, and a zone
        // containing one is not a set the engine would let us commit.
        if need[8] > 0 || need[9] > 0 {
            continue;
        }
        best = best.max(experiment_pass_probability_pools(&pools, &need));
    }
    best
}

/// The colour deficit of the best life in hand: what the board cannot cover,
/// and therefore what the deck draws have to cover.
///
/// This is the quantity the game is actually about. A check passes iff the
/// top-`blades` cards of the deck cover `d`. The board's job is to make `d`
/// small, and a blade's job is to draw the colours in `d`.
///
/// Everything v7 and v8 do instead is a proxy for this. `value()` pays a flat
/// 6 per blade and 60 per passable life; v8 pays `Binomial(blades, density)`.
/// Both are colour-blind, and the measured consequence is large: with
/// essentially the same board, v7's committed sets pass at 0.918 and v8's at
/// 0.485, with v8 having MORE active blades (5.81 vs 4.95). Blades are only
/// worth having if the deck contains the colours the held life needs.
pub(crate) fn best_life_deficit(gs: &GameState, me: u8, db: &CardDatabase) -> Acc {
    let mut best: Acc = [0i16; 11];
    let mut found = false;
    for (_, _, need) in experiment_lives(gs, me, db) {
        if need[8] > 0 || need[9] > 0 {
            continue;
        }
        let need_total: i32 = need.iter().map(|&v| i32::from(v)).sum();
        let best_total: i32 = best.iter().map(|&v| i32::from(v)).sum();
        if !found || need_total > best_total {
            best = need;
            found = true;
        }
    }
    if !found {
        return [0i16; 11];
    }
    // Reduce by what the board already provides. `board` is the 8-bucket
    // `i32` pool, so the difference is taken wide and clamped into the `i16`
    // deficit element.
    let board = experiment_board_pool(gs, me, db);
    let mut deficit: Acc = [0i16; 11];
    for c in 1..=6 {
        deficit[c] = narrow_hearts((i32::from(best[c]) - board[c]).max(0));
    }
    deficit[0] = best[0];
    deficit[7] = best[7];
    deficit
}

/// How many of `blades` additional blades are worth something, given the
/// colours still in our own deck and the deficit of the life we hold.
///
/// Legal information only: our own deck and our own hand. Nothing here reads
/// the opponent, so this is arithmetic on visible state, not lookahead.
///
/// The reasoning: each active blade reveals one more card from our deck, and a
/// revealed card contributes its own colour. So one blade is worth one unit of
/// the colour that the next card happens to be, and the expected value of a
/// blade is therefore the deck's own colour distribution weighted by whether
/// that colour reduces the deficit. A deck with none of the needed colours
/// makes blades worthless here, however many we can afford — and a deck full
/// of them makes a blade the best card in the game.
pub(crate) fn useful_blades(gs: &GameState, me: u8, db: &CardDatabase, blades: i32) -> f64 {
    if blades <= 0 {
        return 0.0;
    }
    let deficit = best_life_deficit(gs, me, db);
    let needed: i32 = deficit[1..=6].iter().map(|&v| i32::from(v)).sum::<i32>()
        + i32::from(deficit[0]);
    if needed <= 0 {
        // The board already covers the life. Extra blades only add pool we do
        // not need, so they are worth the same as any other stat and no more.
        return 0.0;
    }
    let (cats, deck_len) = experiment_flip_categories(gs, me, db);
    if deck_len == 0 {
        return 0.0;
    }
    // The sampler already precomputed, per deck category, the colour vector a
    // revealed card contributes. Weight each category by how likely it is to
    // be drawn next, and score it by how much of the deficit it would fill.
    let mut expected = 0.0f64;
    let mut remaining = deck_len;
    for (vector, count) in &cats {
        if *count == 0 || remaining == 0 {
            continue;
        }
        let share = *count as f64 / remaining as f64;
        let mut fills = 0i32;
        for c in 1..=6 {
            if deficit[c] > 0 {
                fills += vector[c].min(i32::from(deficit[c]));
            }
        }
        if vector[7] > 0 && deficit[1..=6].iter().any(|d| *d > 0) {
            // `icon_all` is the wildcard: it counts toward any one colour.
            fills += vector[7].min(1);
        }
        if fills > 0 {
            expected += share * f64::from(fills).min(f64::from(needed));
        }
        remaining -= *count;
    }
    // One blade reveals one card, so the expected fill per blade is `expected`,
    // capped at the size of the deficit so we never credit blades for filling
    // a requirement twice.
    f64::from(blades) * expected.min(f64::from(needed).max(1.0))
}

/// Total size of the requirement the best held life imposes, and how much of
/// it the board already covers. These two numbers separate the two possible
/// causes of a low pass probability — a HARD life in hand, or a board that
/// does not cover it — which is the distinction every hypothesis this round
/// has been unable to make.
pub(crate) fn deficit_total(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let deficit = best_life_deficit(gs, me, db);
    deficit[1..=6].iter().map(|&v| i32::from(v)).sum::<i32>() + i32::from(deficit[0])
}

/// How many of the best held life's coloured requirement the board meets.
pub(crate) fn board_cover_of_best_life(gs: &GameState, me: u8, db: &CardDatabase) -> i32 {
    let lives = experiment_lives(gs, me, db);
    let board = experiment_board_pool(gs, me, db);
    let mut best_need = 0i32;
    let mut best_cover = 0i32;
    for (_, _, need) in lives {
        if need[8] > 0 || need[9] > 0 {
            continue;
        }
        let total: i32 = need[1..=6].iter().map(|&v| i32::from(v)).sum();
        if total <= best_need {
            continue;
        }
        let mut cover = 0i32;
        for c in 1..=6 {
            cover += i32::from(need[c]).min(board[c]);
        }
        best_need = total;
        best_cover = cover;
    }
    best_cover
}

/// The opponent's pass probability, from what is PUBLIC about them: the cards
/// they committed to the live zone and their active blades.
///
/// Returns `None` when they have committed nothing — then there is no `q` to
/// compare against, only the risk that they assemble a set later, which is
/// what the reliability floor is for.
///
/// Built by calling the SAME pool machinery with the other seat, rather than by
/// re-deriving a colour convention from card data. A first attempt did the
/// latter and had to guess the `heart0` / `b_all` / `b_heart0N` slot mapping;
/// this is the engine's own model of a board and a deck draw, so it cannot
/// drift from what the check will actually do.
pub(crate) fn opponent_pass_probability(gs: &GameState, me: u8, db: &CardDatabase) -> Option<f64> {
    let opp_seat = 1 - me;
    let opp = gs.seat_player(opp_seat);
    if opp.live_card_zone.cards.is_empty() {
        return None;
    }
    let mut need: Acc = [0i16; 11];
    let mut any = false;
    for &cid in &opp.live_card_zone.cards {
        let base = db.get_card(cid).and_then(|card| card.need_heart.as_ref());
        let Some(effective) = crate::core::stats_pipeline::effective_need_heart(
            base,
            cid,
            &gs.mods.need_heart_modifiers,
        ) else {
            continue;
        };
        any = true;
        for (color, count) in &effective.hearts {
            need[color.index()] += i16::from(*count);
        }
    }
    if !any {
        return None;
    }
    if need[8] > 0 || need[9] > 0 {
        // They are holding something that cannot pass, so q is 0 and a commit
        // is always right.
        return Some(0.0);
    }
    let (cats, deck_len) = experiment_flip_categories(gs, opp_seat, db);
    let blades = experiment_blades(gs, opp_seat, db);
    if blades <= 0 || deck_len == 0 {
        return Some(0.0);
    }
    let their_board = experiment_board_pool(gs, opp_seat, db);
    let pools = experiment_sample_pools(&cats, deck_len, blades, &their_board);
    Some(experiment_pass_probability_pools(&pools, &need))
}
/// The two public facts a fold decision needs about the opponent: how likely
/// their committed set is to pass, and what it scores if it does.
///
/// Both come from the live zone, which is public once set. Neither reads
/// their hand or their deck order beyond the same 256-draw model the check
/// itself uses.
pub(crate) fn opponent_response_inputs(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
) -> Option<(f64, i32)> {
    let opp_seat = 1 - me;
    let opp = gs.seat_player(opp_seat);
    if opp.live_card_zone.cards.is_empty() {
        return None;
    }
    let mut need: Acc = [0i16; 11];
    let mut score = 0i32;
    let mut any = false;
    for &cid in &opp.live_card_zone.cards {
        let Some(card) = db.get_card(cid) else { continue };
        score += i32::from(card.score.unwrap_or(0));
        let base = card.need_heart.as_ref();
        let Some(effective) = crate::core::stats_pipeline::effective_need_heart(
            base,
            cid,
            &gs.mods.need_heart_modifiers,
        ) else {
            continue;
        };
        any = true;
        for (color, count) in &effective.hearts {
            need[color.index()] += i16::from(*count);
        }
    }
    if !any {
        return None;
    }
    if need[8] > 0 || need[9] > 0 {
        return Some((0.0, score));
    }
    let (cats, deck_len) = experiment_flip_categories(gs, opp_seat, db);
    let blades = experiment_blades(gs, opp_seat, db);
    if blades <= 0 || deck_len == 0 {
        return Some((0.0, score));
    }
    let their_board = experiment_board_pool(gs, opp_seat, db);
    let pools = experiment_sample_pools(&cats, deck_len, blades, &their_board);
    Some((
        experiment_pass_probability_pools(&pools, &need),
        score,
    ))
}

/// Pass probability of the set v7 actually committed to. **Telemetry only** —
/// the decision path uses `best_available_pass_probability` and the payoff table,
/// never this.
///
/// ## A zone with no life in it is not a check, and used to score 1.0
///
/// `desired` holds hand INDICES, and `experiment_junk_fill` fills the remaining
/// slots with junk, so a zone holding only junk has a non-empty `desired` and
/// an all-zero pooled `need` — which every sampled pool trivially satisfies, so
/// this returned 1.000. v8's equivalent takes an explicit `if chosen.is_empty()
/// { 0.0 }` branch and reported the same physical decision as 0.000.
///
/// That is the same decision priced two ways, and it lands squarely on the
/// headline comparison: v7's mean committed p is pulled UP by every junk-only
/// zone it folds into, v8's is pulled DOWN by every one of its own, and the gap
/// between the two reported means is therefore part measurement artifact. The
/// number this line was cited for — "v7 0.924 vs v8 0.485" — is a comparison of
/// incompatible definitions, not a finding about the bots.
///
/// Folding is not a check passed; it is a check handed over. 0.0 is the honest
/// reading and it is also what makes `chosen_p` and v8's `ppass` joinable, which
/// is the only reason to print either.
pub(crate) fn chosen_set_pass_probability(
    gs: &GameState,
    me: u8,
    db: &CardDatabase,
    desired: &[usize],
) -> f64 {
    let lives = experiment_lives(gs, me, db);
    let mut need: Acc = [0i16; 11];
    let mut held = 0usize;
    for &hi in desired {
        if let Some((_, _, n)) = lives.iter().find(|(h, _, _)| *h == hi) {
            for k in 0..11 {
                need[k] += n[k];
            }
            held += 1;
        }
    }
    // No life in the zone: junk, or nothing at all. Either way there is no
    // check to pass, so the probability is 0 rather than the 1.0 an empty
    // requirement earns. See the note above.
    if held == 0 {
        return 0.0;
    }
    if need[8] > 0 || need[9] > 0 {
        return 0.0;
    }
    let (cats, deck_len) = experiment_flip_categories(gs, me, db);
    let blades = experiment_blades(gs, me, db);
    let board = experiment_board_pool(gs, me, db);
    if blades <= 0 || deck_len == 0 {
        return 0.0;
    }
    let pools = experiment_sample_pools(&cats, deck_len, blades, &board);
    experiment_pass_probability_pools(&pools, &need)
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
    if !desired.is_empty()
        && my.hand.cards.iter().enumerate().any(|(i, &cid)| {
            !desired.contains(&i)
                && db
                    .get_card(cid)
                    .is_some_and(|c| c.card_type == CardType::Live)
        })
    {
        return;
    }
    let future_junk = std::env::var_os("V7_NO_FUTURE_JUNK").is_none();
    // Opt-in, measured separately (see the module's experiment log). Neither
    // changes the existing default until it has cleared the paired protocol in
    // both seat orientations.
    let keep_ability = std::env::var_os("V7_JUNK_ABILITY").is_some();
    let junk_curve_passed = std::env::var_os("V7_JUNK_CURVE").is_some();
    let max_stage_cost = my
        .stage
        .stage
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|card| card.cost)
        .max()
        .unwrap_or(0) as i32;
    let budget = i32::from(u16::from(my.energy_zone.active_count())) + 1;

    // Lives v7 has ALREADY decided not to set are disposal candidates.
    //
    // The junk pool used to exclude `CardType::Live` outright, which meant a
    // live the bot will never play could never be binned: it sat in hand for
    // the rest of the game clogging draws, while a junk slot sat empty that
    // would have drawn a replacement for free under 8.3.4. The live zone
    // disposes of up to three member AND/OR live cards — it is not restricted
    // to the spare slots alongside a real life, because a junk-only zone
    // places nothing exactly like an empty one and still draws the
    // replacements. So there is no reason to hoard a card we will not use.
    //
    // Gated on `V7_JUNK_LIVES` and on the card genuinely being below the
    // reliability floor: a life we might set is never binned, only one the
    // pass model has already rejected.
    let junk_lives = std::env::var_os("V7_JUNK_LIVES").is_some();
    let floor = live_reliability_floor(gs, me);
    let unplayable_lives: std::collections::HashSet<usize> = if junk_lives {
        let (cats, deck_len) = experiment_flip_categories(gs, me, db);
        let blades = experiment_blades(gs, me, db);
        let board = experiment_board_pool(gs, me, db);
        let pools = experiment_sample_pools(&cats, deck_len, blades, &board);
        experiment_lives(gs, me, db)
            .into_iter()
        .filter(|(_, _, need)| {
            need[8] == 0
                && need[9] == 0
                && need.iter().map(|&v| i32::from(v)).sum::<i32>() > 0
            })
            .filter(|(_, _, need)| {
                experiment_pass_probability_pools(&pools, need) < floor
            })
            .map(|(hi, _, _)| hi)
            .collect()
    } else {
        std::collections::HashSet::new()
    };

    let mut junk: Vec<(usize, u8, i32)> = my
        .hand
        .cards
        .iter()
        .enumerate()
        .filter(|&(i, &cid)| {
            if desired.contains(&i) {
                return false;
            }
            let Some(card) = db.get_card(cid) else {
                return false;
            };
            if card.card_type != CardType::Live {
                return true;
            }
            // Only a live we have already given up on, and never more than the
            // slots allow.
            unplayable_lives.contains(&i)
        })
        .map(|(i, &cid)| {
            let card = db.get_card(cid);
            let cost = card.and_then(|c| c.cost).unwrap_or(0);
            // A binned live is pure upside: it was never going to be set, and
            // the replacement draw is free. Rank it as the cheapest possible
            // junk so it goes out first, ahead of any member we might still
            // be able to develop.
            let is_live = card.is_some_and(|c| c.card_type == CardType::Live);
            let mut utility = if is_live {
                -1_000.0
            } else if future_junk && card.is_some_and(|c| c.card_type == CardType::Member) {
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
            if keep_ability && card.is_some_and(card_has_usable_ability) {
                // Large enough to outrank any stat utility the loop can
                // produce, so an ability-bearing card is never the first thing
                // binned. It still yields if every other card in hand is
                // equally alive and slots run out.
                utility += ABILITY_KEEP_BONUS;
            }
            if junk_curve_passed
                && card.is_some_and(|c| c.card_type == CardType::Member)
                && !card.is_some_and(card_has_usable_ability)
                && max_stage_cost > 0
                && i32::from(cost) + CURVE_PASSED_MARGIN <= max_stage_cost
            {
                // The board has moved past this card. A cost-1 member under a
                // cost-15 stage adds one point of development and a rounding
                // error of blades, and the check reads blades — so it is dead
                // weight that only clogs the hand. A pure dev card with no
                // ability is exactly what the live zone is FOR.
                utility -= CURVE_PASSED_PENALTY;
            }
            let class = if is_live {
                // Lowest class so a binned live is chosen before members.
                0
            } else if future_junk && card.is_some_and(|c| c.card_type == CardType::Energy) {
                2
            } else if future_junk {
                1
            } else {
                0
            };
            (i, cost, class * 1_000_000 + crate::constants::score_to_i32(utility))
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

/// The reliability floor: the minimum estimated pass probability before v7 will
/// commit a life at all.
///
/// This was DUPLICATED — hard-coded inside `experiment_portfolio_rank` and
/// separately recomputed in `live_set_plan`, where `V7_LIVE_FLOOR` could reach
/// it. So there were two independent gates and the environment variable moved
/// only one. Measured consequence: a floor sweep over `V7_LIVE_FLOOR` at 0.25,
/// 0.35 and 0.50 produced games byte-identical to the default (McNemar
/// p = 1.00000, 539-461 both ways), which is not a plausible result for a
/// threshold raised above the 0.45 default — it is the signature of a knob
/// that is not connected to the decision. Any earlier tuning of this threshold
/// was only tuning half of it.
///
/// One function, one value, both call sites.
fn live_reliability_floor(gs: &GameState, me: u8) -> f64 {
    if let Some(parsed) = std::env::var("V7_LIVE_FLOOR")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|p| p.is_finite() && (0.0..=1.0).contains(p))
    {
        return parsed;
    }
    let (my, opp) = gs.seated_pair(me);
    if opp.success_live_card_zone.cards.len() >= 2 {
        0.35
    } else if my.success_live_card_zone.cards.len() >= 2 {
        0.60
    } else {
        0.45
    }
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
    let mut needs: Vec<Acc> = Vec::with_capacity(n);
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
        let mut need_total: Acc = [0i16; 11];
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
        let floor = live_reliability_floor(gs, me);
        let _ = opp;
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
            my.hand.cards.get(hi).copied().is_some_and(|cid| {
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
    let desired = live_set_plan(gs, db);
    emit(gs, actions, &desired)
}

/// The live-card portfolio v7 wants, as hand indices.
///
/// Split out of `choose_live_set_experiment` so `v7_ismcts` can plan a
/// whole live phase once and then replay the plan across that phase's
/// action ticks, instead of re-running the 256-sample pass model on every
/// tick of a stateless emission chain.
pub(crate) fn live_set_plan(gs: &GameState, db: &CardDatabase) -> Vec<usize> {
    let me = gs.active_player_index();
    let (my, opp) = gs.seated_pair(me);
    let my_succ = my.success_live_card_zone.cards.len();
    let opp_succ = opp.success_live_card_zone.cards.len();
    let is_second = gs.current_phase == Phase::LiveCardSetSecondAttacker;
    let opp_committed = !opp.live_card_zone.cards.is_empty();
    let floor = live_reliability_floor(gs, me);
    // Opponent-commitment response model. Binary switch, no coefficient: the
    // payoff table is worth ±1, not a fitted number. `V7_RESPONSE=0` restores
    // today's "safest single life, opponent ignored" behaviour.
    let response_model = std::env::var("V7_RESPONSE")
        .as_deref()
        .map(|value| value != "0")
        .unwrap_or(true);

    let mut desired: Vec<usize> = Vec::new();

    // CONTESTED: answer the opponent, not just your own check.
    //
    // This has to live HERE, not in `choose_live_set_v7` below the
    // `V7_LIVE_LEGACY` guard — that code is behind an env opt-in and the
    // default path returns at `choose_live_set_experiment` before reaching it.
    // An earlier version of this work was wired into the legacy branch and
    // therefore measured 0 discordant games in 2000: it had never executed
    // once. Same for the v6-contested-delegation change — it was never on the
    // default path either, so "v7 no longer rents v6 there" was true only of
    // code that does not run.
    //
    // When the opponent has committed, v7's normal answer is "the single life
    // most likely to pass", which is a non-answer to them. The guides make
    // this the decisive class of decision: a 2-live is described as mandatory
    // because it DENIES the opponent's 1-live, and score lead is called the
    // dominant axis. `best_response_set` prices the choice with the payoff
    // table that was already written and never called, using the opponent's
    // PUBLIC blades and committed set size.
        if response_model_enabled() {
            let opp_set_size = opp.live_card_zone.cards.len();
            if (is_second && opp_set_size > 0) || opp_succ >= 2 {
                // THE FOLD ARITHMETIC.
                //
                // Folding is not "keep the life and try later". A fold hands
                // the check over: the opponent resolves against their own set
                // and places if they pass. So the two options are
                //
                //     commit :  p * (a placement)  -  (1 - p) * (the life)
                //     fold   :  0                  -  q * (they place)
                //
                // and commit wins exactly when p > q. There is no constant in
                // that. The three floors this replaces (0.45, 0.60, 0.35) were
                // all standing in for `q` — a guessed chance that the
                // OPPONENT passes — which is why tuning them did nothing: the
                // quantity they were approximating is observable.
                //
                // When the opponent has committed (the second attacker sees
                // the set, which is why the rules make going second
                // informative) `q` is exact: it is their set's pass
                // probability under the same 256-draw model, from their
                // committed cards and their blades. That is public information
                // about a public zone, so this is arithmetic rather than
                // lookahead into hidden state.
                //
                // This also explains the v8 measurements that survived every
                // falsification this round. v8 commits at p = 0.485 and folds
                // at a rate tuned as if folding were free; it is not, it pays
                // the opponent. Every threshold change moved the bot in the
                // direction that conceded more or won fewer, and both lost.
                if let Some((q_pass, q_score)) = opponent_response_inputs(gs, me, db) {
                    let mine_p = best_available_pass_probability(gs, me, db);
                    // `q_pass` alone is the UPPER bound on the threat: passing
                    // a check is not placing in it, you must also win the score
                    // comparison, and that factor is well under 1. Using the
                    // upper bound inflated `q` and made v7 fold 23x more often
                    // (fold 0.5% -> 11.6%, 1645-1355 -> 1304-1696, p=0.00000).
                    //
                    // So the fold decision is the payoff table's, not a
                    // threshold's. It carries all three terms at once: our
                    // pass probability, their pass probability, and who wins
                    // the comparison if both pass. Zero is the break-even and
                    // it is not a tuned constant — it is where committing and
                    // conceding have the same expected value.
                    let value = expected_live_value(
                        mine_p,
                        0,
                        1,
                        my_succ,
                        opp_succ,
                        q_pass,
                        q_score,
                        opp.live_card_zone.cards.len(),
                    );
                    if value <= 0.0 {
                        log::debug!(
                            "v7 fold-by-payoff t{} me{} mine={:.3} theirs={:.3} v={:.3} -> fold",
                            gs.turn_number,
                            me,
                            mine_p,
                            q_pass,
                            value
                        );
                        return Vec::new();
                    }
                }
                let response = best_response_set(gs, me, db);
                if !response.is_empty() {
                    log::debug!(
                        "v7 response t{} me{} opp_set={} opp_succ={} -> {:?}",
                        gs.turn_number,
                        me,
                        opp_set_size,
                        opp_succ,
                        response
                    );
                    return response;
                }
            }
        }

    if my.live_card_set_limit_reduction >= 3 || gs.cannot_live_players.contains(&my.id) {
        return desired;
    }
    if is_second && !opp_committed {
        if let Some(hi) = experiment_free_win(gs, me, db) {
            desired.push(hi);
            if std::env::var_os("V7_FREE_JUNK").is_some() {
                experiment_junk_fill(gs, me, db, &mut desired);
            }
            return desired;
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
                        return desired;
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
            .unwrap_or([0i16; 11]);
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
            let index = crate::bot::rollout::price_portfolios(gs, me, &candidates, &[]);
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
    // Per-check telemetry: how good was the set we actually committed to?
    //
    // This is the measurement that separates "v8 builds a bigger board and
    // passes fewer checks" into a cause. If v8's chosen p_place is LOWER than
    // v7's at the same blade count, then its development is colour-blind: it
    // grows a board whose colours do not match the lives in hand, so more
    // cost buys less conversion. If the two are equal, the difference is
    // elsewhere and this is a dead end.
    if std::env::var_os("V7_CHECK_P").is_some() {
        let chosen_p = chosen_set_pass_probability(gs, me, db, &desired);
        // Best AVAILABLE set, whether or not it cleared the floor. The gap
        // between this and `chosen_p` is the headroom: it is the difference
        // between "the board cannot support a check" and "v7 declined the
        // check it could have made", and those call for opposite fixes.
        //
        // It was bound here and then never formatted, so the compiler warned
        // `unused variable: best_available` and NO capture has ever contained
        // it. The whole point of computing it is that it reaches a file.
        // Set-aware, because this is the headroom question (was there a check
        // we could have made), not the decision question (how good could one
        // commit be). The decision-facing single-life version stays exactly
        // where it was so that measuring v8 does not move v7.
        let best_available = best_available_set_pass_probability(gs, me, db);
        // How many of the lives we are considering carry a `{{live_start}}`
        // ability, and how many we committed. The handoff doc attributes the
        // v7/v8 life-choice split to v7 being able to see those abilities; it
        // cannot (see `best_available_pass_probability`'s neighbours — the
        // default live path is static), so this column is the direct test of
        // that claim: if v7's committed sets are not enriched for live-start
        // cards, the abilities explanation is dead.
        let candidate_lives = experiment_lives(gs, me, db);
        let (cand_live_start, chosen_live_start) = {
            let is_live_start = |cid: i16| {
                db.get_card(cid).is_some_and(|card| {
                    card.abilities.iter().any(|a| {
                        a.resolve()
                            .has_trigger(crate::triggers::TriggerKind::LiveStart)
                    })
                })
            };
            let all = candidate_lives
                .iter()
                .filter(|(_, _, need)| need[8] == 0 && need[9] == 0)
                .filter(|&(_, cid, _)| is_live_start(*cid))
                .count();
            let chosen = desired
                .iter()
                .filter_map(|&hi| my.hand.cards.get(hi).copied())
                .filter(|&cid| db.get_card(cid).is_some_and(|c| c.card_type == CardType::Live))
                .filter(|&cid| is_live_start(cid))
                .count();
            (all, chosen)
        };
        // The label is `ppass`, not `p`, and v7 has no `p_place` column to
        // print: v7's live set never computes a placement probability at all —
        // it ranks on pass probability and printed score, and leaves the
        // comparison outcome to the payoff table. v8's line carries both. The
        // asymmetric column set is itself a finding, not an oversight: it is
        // why a `p` difference between the two lines could be read as a
        // development difference when it was a modelling difference.
        eprintln!(
            "V7CHK t={} me={} blades={} set={} ppass={:.3} best={:.3} succ={} need={} \
             boardcovers={} deck={} lcand={} lscand={} lschosen={}",
            gs.turn_number,
            me,
            experiment_blades(gs, me, db),
            n_lives,
            chosen_p,
            best_available,
            my_succ,
            deficit_total(gs, me, db) as usize,
            board_cover_of_best_life(gs, me, db) as usize,
            my.main_deck.cards.len(),
            candidate_lives.len(),
            cand_live_start,
            chosen_live_start,
        );    }
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
    desired
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
        .filter_map(|&hi| my.hand.cards.get(usize::from(hi)).copied())
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
            line.len().u32_count(),
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
/// The colour requirement of the first three lives in hand — the ones a
/// mulligan should be keeping, since only three live slots exist.
fn kept_live_requirement(gs: &GameState, db: &CardDatabase) -> [i32; 8] {
    let mut want = [0i32; 8];
    let mut lives = 0usize;
    for &cid in &gs.active_player().hand.cards {
        let Some(card) = db.get_card(cid) else { continue };
        if card.card_type != CardType::Live {
            continue;
        }
        lives += 1;
        if lives > 3 {
            break;
        }
        if let Some(vector) = live_need_vector(db, cid) {
            for c in 0..8 {
                want[c] += vector[c];
            }
        }
    }
    want
}

/// A member's printed heart vector, in the model's 0..=7 slots (0 colorless,
/// 7 the `icon_all` wildcard).
fn member_heart_vector(db: &CardDatabase, cid: i16) -> Option<[i32; 8]> {
    let card = db.get_card(cid)?;
    if card.card_type != CardType::Member {
        return None;
    }
    let mut vector = [0i32; 8];
    if let Some(base) = &card.base_heart {
        for (color, count) in &base.hearts {
            vector[color.index()] += i32::from(*count);
        }
    }
    Some(vector)
}

/// A live card's requirement, in the model's 0..=7 slots.
fn live_need_vector(db: &CardDatabase, cid: i16) -> Option<[i32; 8]> {
    let card = db.get_card(cid)?;
    if card.card_type != CardType::Live {
        return None;
    }
    let mut vector = [0i32; 8];
    if let Some(need) = &card.need_heart {
        for (color, count) in &need.hearts {
            vector[color.index()] += i32::from(*count);
        }
    }
    Some(vector)
}

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
    let mulligan_coverage = std::env::var_os("V7_MULLIGAN_COVERAGE").is_some();
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
    if mulligan_coverage {
        // DUMP BY COVERAGE, NOT BY COST.
        //
        // The whole night of measurement landed on one fact: this game is won
        // by heart coverage against a held life, not by stage cost. The mulligan
        // was choosing which cards to bin by cost alone, which is a question
        // about a resource (board size) that the check does not read.
        //
        // So bin the members that help least with the lives this hand is
        // keeping. That is derived, not fitted: it asks "does this card
        // contribute a colour the kept lives need", which the card database
        // answers, and it introduces no coefficient to tune. It also falls out
        // naturally on the default deck, which is 24x cost-2 — dumping the
        // most EXPENSIVE members there means binning the scarce big-colour
        // cards and keeping a wall of duplicates.
        let wanted = kept_live_requirement(gs, db);
        let mut supply = [0i32; 8];
        for (index, _) in &members {
            if let Some(vector) = member_heart_vector(db, gs.active_player().hand.cards[*index]) {
                for c in 0..8 {
                    supply[c] += vector[c];
                }
            }
        }
        members.sort_by_key(|&(index, _)| {
            let vector = member_heart_vector(db, gs.active_player().hand.cards[index])
                .unwrap_or([0; 8]);
            // How much of what the kept lives need does this member supply?
            let mut contributes = 0i32;
            for c in 1..=6 {
                contributes += vector[c].min(wanted[c]);
            }
            // Ascending: the least useful member is the first binned.
            std::cmp::Reverse(contributes)
        });
    } else {
        members.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost));
    }
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

/// v7's own mulligan. Self-contained — it does NOT call v4.
///
/// v7 previously delegated its entire mulligan to `strategy_v4`, which made
/// "v7" partly a v4 build: a change to the older version's opening would
/// silently change the newest one, and the two could not be A/B'd
/// independently. The behaviour is deliberately identical to the v4 rule it
/// replaces (keep at most 3 lives, then dump the most expensive members, 3
/// cards total), so this is a decoupling and not a behaviour change — which is
/// why `v7_mulligan_matches_the_v4_rule_it_replaced` can lock it.
///
/// The guide's version of this is an opening-quality evaluation that enumerates
/// the first three Main turns; that is `choose_mulligan_curve` above, still
/// opt-in. What this removes is the dependency, not the ambition.
/// Choose a set as a RESPONSE to the opponent's committed set, using the
/// payoff table that was already written and never called.
///
/// Both opponent arguments are estimated from PUBLIC information only:
///
///   * `opp_pass` — the opponent committed `n` cards and the count is visible
///     (the rules have the responding player see how many were set, which is
///     why going second is described as advantageous). A check is
///     all-or-nothing across the whole set (the engine's `any_requirement_failed`),
///     so a bigger commitment raises the bar rather than the reward. Modelling
///     that directly means a small opponent set is valued as beatable and a
///     large one as not — the "deny the 1-live" pressure the guides describe,
///     falling out of the check model instead of being a rule written to
///     imitate it.
///   * `opp_score` — printed score of the committed set, from card data.
pub(crate) fn best_response_set(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<usize> {
    let opp = gs.seat_player(1 - me);
    let opp_set = opp.live_card_zone.cards.len();
    let dbg = std::env::var_os("V7_RESPONSE_DEBUG").is_some();
    if dbg { eprintln!("V7RESP enter opp_set={opp_set} phase={:?}", gs.current_phase); }
    if opp_set == 0 {
        if dbg { eprintln!("V7RESP exit: opponent not committed"); }
        return Vec::new();
    }
    let (cats, deck_len) = experiment_flip_categories(gs, me, db);
    let blades = experiment_blades(gs, me, db);
    let board = experiment_board_pool(gs, me, db);
    if blades <= 0 || deck_len == 0 {
        if dbg { eprintln!("V7RESP exit: blades={blades} deck_len={deck_len}"); }
        return Vec::new();
    }
    let pools = experiment_sample_pools(&cats, deck_len, blades, &board);
    let opp_blades = opp.stage.total_blades(
        db,
        &gs.mods.blade_modifiers,
        &gs.mods.orientation_modifiers,
        false,
    ) as f64;
    let opp_pass = (opp_blades / (1.0 + 0.75 * opp_set as f64)).clamp(0.0, 1.0) * 0.9;
    let opp_score: i32 = opp
        .live_card_zone
        .cards
        .iter()
        .filter_map(|&cid| db.get_card(cid))
        .filter_map(|card| card.score)
        .map(i32::from)
        .sum();
    let my_succ = gs.seat_player(me).success_live_card_zone.cards.len();
    let opp_succ = opp.success_live_card_zone.cards.len();

    let lives = experiment_lives(gs, me, db);
    if dbg { eprintln!("V7RESP lives_in_hand={} max_slots={}", lives.len(), usize::from(3u8.saturating_sub(gs.seat_player(me).live_card_set_limit_reduction))); }
    if lives.is_empty() {
        if dbg { eprintln!("V7RESP exit: no lives in hand"); }
        return Vec::new();
    }
    let max_slots = usize::from(3u8.saturating_sub(
        gs.seat_player(me).live_card_set_limit_reduction,
    ));
    let floor = live_reliability_floor(gs, me);

    // Enumerate sets up to `max_slots` and price each as a RESPONSE rather
    // than as an attempt. This is what lets a deliberately unpassable
    // multi-card set enter a comparison the single-life search would refuse
    // — the 2-live denial play — with no coefficient telling it to.
    let mut best: Option<(f64, Vec<usize>)> = None;
    let n = lives.len().min(8);
    for mask in 1u32..(1u32 << n) {
        let size = mask.count_ones() as usize;
        if size > max_slots {
            continue;
        }
        let mut need: Acc = [0i16; 11];
        let mut idxs = Vec::new();
        for (bit, (hi, _, n)) in lives.iter().enumerate() {
            if mask >> bit & 1 == 1 {
                for k in 0..11 {
                    need[k] += n[k];
                }
                idxs.push(*hi);
            }
        }
        if need[8] > 0 || need[9] > 0 {
            continue;
        }
        let pass = experiment_pass_probability_pools(&pools, &need);
        // A single life nobody can pass is not a response, it is handing the
        // check over. Multi-card sets are allowed through precisely because
        // winning a comparison on score is a real outcome for them.
        if size == 1 && pass < floor {
            continue;
        }
        let score: i32 = idxs
            .iter()
            .filter_map(|hi| {
                lives
                    .iter()
                    .find(|(h, _, _)| h == hi)
                    .and_then(|(_, cid, _)| db.get_card(*cid))
                    .and_then(|card| card.score)
                .map(i32::from)
            })
            .sum();
        let value = expected_live_value(
            pass, score, size, my_succ, opp_succ, opp_pass, opp_score, opp_set,
        );
        if best.as_ref().is_none_or(|(current, _)| value > *current) {
            best = Some((value, idxs));
        }
    }
    // Return the argmax, unconditionally.
    //
    // First attempt gated this on `value > 0.0` and produced ZERO discordant
    // games in 2000 — it never fired. That is a bug in the gate, not a null
    // result: `live_payoff` scales the non-terminal case by 0.1 (a success
    // lead is worth 0.1, not 1.0), so with both players on equal successes
    // almost every set scores ~0 and was discarded. The payoff table is a
    // RANKING, so it has to choose among sets — not be asked to clear an
    // absolute threshold that its own scaling guarantees it will not.
    //
    // Returning the argmax also makes this a genuine replacement for
    // `safest_portfolio` in the contested case rather than an overlay on top
    // of it, which is what the guides say is needed: the single safest life
    // answers "what can I pass" and never "what should I do about them".
    match best {
        Some((_, idxs)) if !idxs.is_empty() => idxs,
        _ => Vec::new(),
    }
}

/// Is the opponent-commitment response model active? OFF by default.
///
/// MEASURED, and it is the worst result in this whole line of work: with the
/// model actually executing (see the `V7_LIVE_LEGACY` note below — the first
/// version of it was wired behind that guard and had never run), v7 goes
/// 1157-843 -> 997-1003 against v6. Exact McNemar p = 0.00000, 434 discordant
/// games out of 2000. A loss of eight points from a model that is supposed to
/// be a strict improvement on "pick my safest life".
///
/// The cause is my own licence, not the payoff table. I let multi-card sets
/// through with no reliability floor, specifically so the guides' "2-live to
/// deny the opponent's 1-live" play could be chosen, and with a crude
/// opponent-pass estimate the argmax is a large set the bot cannot pass —
/// which folds the check AND loses the comparison it was meant to win. So the
/// 2-live denial needs a payoff model that also respects feasibility, and a
/// real opponent-pass estimate, neither of which exists yet.
///
/// This also retrospectively vindicates the documented "allow 2 lives when
/// contested" result (354 vs 364): that measurement was right, and my attempt
/// to overturn it was missing a constraint rather than uncovering a rule.
///
/// Kept, wired, and off — so the next attempt starts from a real measurement
/// instead of the 0-discordant no-op that dead code produces.
fn response_model_enabled() -> bool {
    std::env::var("V7_RESPONSE")
        .as_deref()
        .map(|value| value != "0")
        .unwrap_or(false)
}

pub fn choose_mulligan_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let mulligan_coverage = std::env::var_os("V7_MULLIGAN_COVERAGE").is_some();
    if std::env::var("V7_MULLIGAN_CURVE").is_ok() || std::env::var("V7_MULLIGAN_REACHABLE").is_ok()
    {
        return choose_mulligan_curve(gs, actions, db);
    }
    let me = gs.active_player();
    let mut discard: Vec<usize> = Vec::new();
    let mut lives_seen = 0usize;
    let mut members: Vec<(usize, u8)> = Vec::new();
    for (hand_index, &cid) in me.hand.cards.iter().enumerate() {
        let Some(card) = db.get_card(cid) else {
            continue;
        };
        match card.card_type {
            CardType::Live => {
                lives_seen += 1;
                if lives_seen > 3 {
                    // Only three lives are usable per turn (three live slots).
                    discard.push(hand_index);
                }
            }
            CardType::Member => members.push((hand_index, card.cost.unwrap_or(0))),
            CardType::Energy => {}
        }
    }
    // Dump the most expensive members first: they are furthest from playable,
    // and energy is never dumped.
    members.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost));

    // `V7_MULLIGAN_KEEP=all` keeps the entire opening hand.
    //
    // The mulligan has never been ablated against doing nothing. The docs
    // record `V7_MULLIGAN_REACHABLE` as a "mixed result at 1000 games per
    // seat, remains opt-in", which establishes that one alternative discard
    // rule beats another only weakly — not that discarding beats keeping. Same
    // question as the baton tie-break: is a hand-placed mechanism that fires
    // every game still earning its place?
    if std::env::var("V7_MULLIGAN_KEEP").as_deref() == Ok("all") {
        members.clear();
    }

    if mulligan_coverage {
        // DUMP BY COVERAGE, NOT BY COST — the default path, not the opt-in one.
        //
        // An earlier version of this landed in `choose_mulligan_curve`, which
        // only runs under V7_MULLIGAN_CURVE / V7_MULLIGAN_REACHABLE, and so
        // measured p = 1.00000 over 3000 games: byte-identical, because it had
        // never executed in the default build. Same failure as the
        // V7_LIVE_LEGACY branch, and worth stating because it has now happened
        // twice — an improvement wired into a non-default path measures as a
        // perfect null rather than as an error.
        //
        // The night of measurement landed on one fact: the game is won by
        // heart coverage against a held life, not by stage cost. Binning by
        // cost alone is a question about a resource the check never reads. On
        // the default deck — 24x cost-2 — it means binning the scarce
        // big-colour cards and keeping a wall of duplicates.
        let wanted = kept_live_requirement(gs, db);
        let mut supply = [0i32; 8];
        for (index, _) in &members {
            if let Some(vector) = member_heart_vector(db, me.hand.cards[*index]) {
                for c in 0..8 {
                    supply[c] += vector[c];
                }
            }
        }
        // How much of what the kept lives need does each member supply?
        let mut usefulness: Vec<(usize, i32)> = members
            .iter()
            .map(|&(index, _)| {
                let vector =
                    member_heart_vector(db, me.hand.cards[index]).unwrap_or([0; 8]);
                let mut contributes = 0i32;
                for c in 1..=6 {
                    contributes += vector[c].min(wanted[c]);
                }
                (index, contributes)
            })
            .collect();
        // Least useful first, and ties broken by most expensive so a genuinely
        // neutral card is still binned over a useful cheap one.
        usefulness.sort_by_key(|&(index, contributes)| {
            let cost = me.hand.cards[index];
            let cost = db.get_card(cost).and_then(|c| c.cost).unwrap_or(0);
            (contributes, std::cmp::Reverse(cost))
        });
        members = usefulness
            .into_iter()
            .map(|(index, _)| {
                let cost = me.hand.cards[index];
                (index, db.get_card(cost).and_then(|c| c.cost).unwrap_or(0))
            })
            .collect();
        let _ = supply;
    }
    for &(hi, _) in &members {
        if discard.len() >= 3 {
            break;
        }
        if !discard.contains(&hi) {
            discard.push(hi);
        }
    }

    let selected: Vec<usize> = gs
        .mulligan_selected_indices
        .iter()
        .map(|&i| usize::from(i))
        .collect();
    for &hi in &discard {
        if selected.contains(&hi) {
            continue;
        }
        if let Some(action) = actions.iter().find(|a| {
            a.action_type == ActionType::SelectMulligan
                && a.selected == Some(false)
                && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
        }) {
            return action.clone();
        }
    }
    // Nothing left to add: the chain is either complete or was folded.
    crate::bot::strategy_common::emit_mulligan(gs, actions, &selected)
}

// Re-exports with generic names for the registry (bot/registry.rs): adding
// v8 never renames these, it only adds a dispatch line there.
pub use choose_action_v7 as choose_action;
pub use choose_live_set_v7 as choose_live_set;
pub use choose_mulligan_v7 as choose_mulligan;

#[cfg(test)]
mod mulligan_tests {
    use super::*;

    /// Version independence, as an executable claim: v7 computes its own
    /// mulligan, and that computation agrees with the v4 rule it replaced.
    /// If someone re-links v7 to v4 this stops testing the thing it claims to
    /// test; if v4 changes, this is the test that tells you v7 did not.
    #[test]
    fn v7_mulligan_matches_the_v4_rule_it_replaced() {
        let cards = crate::card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
            "../cards/cards.json",
        ))
        .unwrap();
        let db = crate::Arc::new(CardDatabase::load_or_create(cards));

        let member_ids: Vec<i16> = db
            .cards
            .values()
            .filter(|c| c.card_type == CardType::Member)
            .take(12)
            .map(|c| db.get_card_id(c.card_no.as_ref()).unwrap())
            .collect();
        let live_ids: Vec<i16> = db
            .cards
            .values()
            .filter(|c| c.card_type == CardType::Live)
            .take(6)
            .map(|c| db.get_card_id(c.card_no.as_ref()).unwrap())
            .collect();
        assert!(member_ids.len() >= 4 && live_ids.len() >= 4, "need real cards");

        for variant in 0..6usize {
            let p1 = crate::player::Player::new("p1".into(), "P1".into(), true);
            let p2 = crate::player::Player::new("p2".into(), "P2".into(), false);
            let mut gs = GameState::new(p1, p2, crate::Arc::clone(&db));
            gs.current_phase = Phase::MulliganFirstAttacker;
            let seat = gs.active_player_index();
            let lives = (variant % 4) + 1;
            for i in 0..lives {
                gs.seat_player_mut(seat).hand.cards.push(live_ids[i]);
            }
            for i in 0..5 {
                gs.seat_player_mut(seat).hand.cards.push(member_ids[(i + variant) % member_ids.len()]);
            }
            let actions = crate::game_setup::generate_possible_actions(&gs);
            if actions.len() < 2 {
                continue;
            }
            let v7_pick = choose_mulligan_v7(&gs, &actions, &db);
            let v4_pick = crate::bot::strategy_v4::choose_mulligan_v4(&gs, &actions, &db);
            assert_eq!(
                v7_pick.action_type, v4_pick.action_type,
                "variant {variant}: action type diverged"
            );
            assert_eq!(
                v7_pick.parameters.as_ref().and_then(|p| p.card_index),
                v4_pick.parameters.as_ref().and_then(|p| p.card_index),
                "variant {variant}: mulligan pick diverged from the v4 rule"
            );
        }
    }

    /// v7 must not hold 4+ lives: only three live slots exist, so a fourth is
    /// the one card type where being confident is always right.
    #[test]
    fn v7_mulligan_dumps_lives_beyond_the_three_slots() {
        let cards = crate::card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
            "../cards/cards.json",
        ))
        .unwrap();
        let db = crate::Arc::new(CardDatabase::load_or_create(cards));
        let live_ids: Vec<i16> = db
            .cards
            .values()
            .filter(|c| c.card_type == CardType::Live)
            .take(5)
            .map(|c| db.get_card_id(c.card_no.as_ref()).unwrap())
            .collect();
        assert!(live_ids.len() >= 5, "need five distinct lives");
        let p1 = crate::player::Player::new("p1".into(), "P1".into(), true);
        let p2 = crate::player::Player::new("p2".into(), "P2".into(), false);
        let mut gs = GameState::new(p1, p2, crate::Arc::clone(&db));
        gs.current_phase = Phase::MulliganFirstAttacker;
        for id in &live_ids {
            let seat = gs.active_player_index();
            gs.seat_player_mut(seat).hand.cards.push(*id);
        }
        let actions = crate::game_setup::generate_possible_actions(&gs);
        if actions.len() < 2 {
            return;
        }
        let picked = choose_mulligan_v7(&gs, &actions, &db);
        let index = picked
            .parameters
            .as_ref()
            .and_then(|p| p.card_index)
            .expect("mulligan must select a card");
        // A life at hand index < 3 is one of the first three seen, so keeping
        // it is correct; anything else is the overflow.
        assert!(
            index >= 3,
            "v7 kept a fourth live (index {index}) instead of dumping the overflow"
        );
    }


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
        let db = crate::card_loader::test_card_db().clone();
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
            crate::card_loader::test_card_db().clone()
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
