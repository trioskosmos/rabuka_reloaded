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
use crate::game_setup::{Action, ActionType};
use crate::game_state::{GameState, Phase};
use crate::player::Player;

fn player_ref(gs: &GameState, me: u8) -> (&Player, &Player) {
    if me == 0 {
        (&gs.player1, &gs.player2)
    } else {
        (&gs.player2, &gs.player1)
    }
}

fn player_of(gs: &GameState, me: u8) -> &Player {
    if me == 0 {
        &gs.player1
    } else {
        &gs.player2
    }
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
                db.get_card(c).map(|x| x.blade as i32).unwrap_or(0)
                    + gs.mods.get_blade_modifier(c) as i32
            }
        })
        .sum()
}

/// Passable lives under the buff-aware mean pool.
fn passable_count_buffed(gs: &GameState, me: u8, db: &CardDatabase) -> usize {
    let p = player_of(gs, me);
    let pool = heart_pool_buffed(gs, me, db, 1.0);
    hand_lives(p, db)
        .iter()
        .filter(|(_, _, need)| alloc(&pool, need).is_some())
        .count()
}

/// Orientation fingerprint for the no-op breaker (waiting kills blades).
fn wait_fingerprint(gs: &GameState, me: u8) -> Vec<bool> {
    player_of(gs, me)
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

/// MAIN PHASE: v6 tempo + energy/draw vision on non-deploy actions + buff
/// vision + buff-aware no-op breaker. Member-deploy economics are v6-verbatim.
///
/// Dev levers (ablation only; unset in production):
/// - `V7_MAIN_V6=1` → delegate to v6's main (isolates live-set changes).
pub fn choose_action_v7(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if std::env::var("V7_MAIN_V6").is_ok() {
        return crate::bot::strategy_v6::choose_action_v6(gs, actions, me);
    }
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let dbg = std::env::var("V7_DEBUG").is_ok();
    let db = &gs.card_database;
    let my_now = player_of(gs, me);
    let base_hand_len = my_now.hand.cards.len() as i32;
    let base_passable = passable_count_buffed(gs, me, db);
    let base_ammo = lives_in_hand(my_now, db);
    let base_stage = stage_hearts_of(my_now, gs, me, db);
    let base_blades = total_blades_of(my_now, gs, db);
    let base_energy = my_now.energy_zone.active_count() as i32;
    let base_buffs = stage_buff_hearts(gs, me);
    let base_wait = wait_fingerprint(gs, me);
    let stage_cost = |p: &Player| -> i32 {
        p.stage.stage.iter().filter_map(|&id| db.get_card(id))
            .map(|card| i32::from(card.cost.unwrap_or(0))).sum()
    };
    let base_cost = stage_cost(my_now);
    let development = std::env::var("V7_DEVELOPMENT").is_ok();
    let upgrade_baton = std::env::var("V7_UPGRADE_BATON").is_ok();

    let deck_lives = my_now
        .main_deck
        .cards
        .iter()
        .filter(|&&c| db.get_card(c).map_or(false, |x| x.card_type == CardType::Live))
        .count();
    let deck_len = my_now.main_deck.cards.len().max(1);
    let p_life_draw = deck_lives as f64 / deck_len as f64;
    let waitroom_lives = my_now
        .waitroom
        .cards
        .iter()
        .filter(|&&c| db.get_card(c).map_or(false, |x| x.card_type == CardType::Live))
        .count();

    let mut vals: Vec<f64> = vec![f64::NEG_INFINITY; actions.len()];
    let mut dbg_lines: Vec<String> = Vec::new();

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if crate::game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        crate::game_setup::settle_single_player_state(&mut sim);
        let my_sim = player_of(&sim, me);

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
        val += 3.0 * d_stage as f64;
        if d_stage != 0 {
            parts.push(format!("hearts{d_stage:+}"));
        }
        let d_blades = total_blades_of(my_sim, &sim, db) - base_blades;
        val += 6.0 * d_blades as f64;
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
        // 4->9->13). Strongly favored.
        let cost_growth = stage_cost(my_sim) - base_cost;
        if development {
            val += 8.0 * f64::from(cost_growth);
        }
        if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true)
            || ((std::env::var("V7_BATON_VISION").is_ok() || (upgrade_baton && cost_growth > 0))
                && a.action_type == ActionType::PlayMemberToStage
                && a.parameters.as_ref().is_some_and(|p| {
                    p.available_areas.as_ref().is_some_and(|areas| {
                        areas.iter().any(|area| {
                            Some(&area.area) == p.stage_area.as_ref() && area.is_baton_touch
                        })
                    })
                }))
        {
            val += 45.0;
            parts.push("baton+45".into());
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
                .filter(|&&c| db.get_card(c).map_or(false, |x| x.card_type == CardType::Live))
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
            let baton_mark = if a.parameters.as_ref().and_then(|p| p.use_baton_touch) == Some(true) {
                "[BATON]"
            } else {
                ""
            };
            dbg_lines.push(format!(
                "    [{}] {:?} {} {} -> {}",
                i, a.action_type, card_no, baton_mark, parts.join(" ")
            ));
        }

        vals[i] = val;
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
    actions[best_idx].clone()
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
    let me = if gs.active_player().id == gs.player1.id {
        0u8
    } else {
        1u8
    };
    let (my, opp) = player_ref(gs, me);
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
            .filter(|&&cid| db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live))
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
                        && db.get_card(cid).map_or(false, |c| c.card_type != CardType::Live)
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
    let (my, opp) = player_ref(gs, me);
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
    let (my, _) = player_ref(gs, me);
    let pool = heart_pool_buffed(gs, me, db, 1.0);
    hand_lives(my, db)
        .into_iter()
        .filter(|(_, _, need)| alloc(&pool, need).is_some())
        .min_by_key(|(hi, cid, _)| {
            (
                db.get_card(*cid).and_then(|c| c.score).unwrap_or(0),
                *hi,
            )
        })
        .map(|(hi, _, _)| hi)
}

fn emit(gs: &GameState, actions: &[Action], desired: &[usize]) -> Action {
    crate::bot::strategy_common::emit_live_set(gs, actions, desired)
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
                if c == a || c == b || bridge <= first || u16::from(*bridge) > u16::from(*first) + 5 {
                    continue;
                }
                for (d, finish) in costs.iter().enumerate() {
                    let Some(finish) = finish else { continue };
                    if d == a || d == b || d == c || finish <= bridge || u16::from(*finish) > u16::from(*bridge) + 6 {
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
    let costs: Vec<Option<u8>> = gs.active_player().hand.cards.iter().map(|&id| {
        db.get_card(id).and_then(|card| {
            (card.card_type == CardType::Member).then_some(card.cost).flatten()
        })
    }).collect();
    let keep = opening_curve_keep(&costs);
    if keep.is_empty() {
        return crate::bot::strategy_v4::choose_mulligan_v4(gs, actions, db);
    }
    let mut desired = Vec::new();
    let mut lives = 0;
    for (index, &id) in gs.active_player().hand.cards.iter().enumerate() {
        if db.get_card(id).is_some_and(|card| card.card_type == CardType::Live) {
            lives += 1;
            if lives > 3 {
                desired.push(index);
            }
        }
    }
    let mut members: Vec<(usize, u8)> = costs.iter().enumerate()
        .filter_map(|(index, cost)| cost.map(|cost| (index, cost)))
        .filter(|(index, _)| !keep.contains(index)).collect();
    members.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost));
    for (index, _) in members {
        if desired.len() >= 3 { break; }
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
    actions.iter().find(|action| matches!(action.action_type,
        ActionType::ConfirmMulligan | ActionType::SkipMulligan))
        .or_else(|| actions.first()).cloned().expect("mulligan actions non-empty")
}

pub fn choose_mulligan_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    if std::env::var("V7_MULLIGAN_CURVE").is_ok() {
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
        assert_eq!(opening_curve_keep(&[Some(2), Some(2), Some(7), Some(11), None, None]), vec![0, 1, 2, 3]);
        assert!(opening_curve_keep(&[Some(2), Some(2), Some(11), Some(17), None, None]).is_empty());
        assert!(opening_curve_keep(&[Some(2), Some(4), Some(7), Some(11), None, None]).is_empty());
    }

    #[test]
    fn mulligan_default_matches_v4_and_curve_variant_recovers_selection() {
        let cards = crate::card_loader::CardLoader::load_cards_from_file(
            std::path::Path::new("../cards/cards.json")).unwrap();
        let db = crate::Arc::new(CardDatabase::load_or_create(cards));
        let names = ["PL!SP-bp1-005-R", "PL!SP-sd1-019-SD", "PL!SP-bp4-011-R＋",
            "PL!SP-bp5-006-R", "PL!SP-sd2-023-SD2", "PL!SP-bp4-025-L"];
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
        assert_eq!(default_choice.parameters.as_ref().and_then(|p| p.card_index),
            v4.parameters.as_ref().and_then(|p| p.card_index));
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
        assert_eq!(choose_mulligan_curve(&gs, &actions, &db).action_type, ActionType::ConfirmMulligan);
    }
}
