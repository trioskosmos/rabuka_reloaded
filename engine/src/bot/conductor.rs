//! Conductor — the plan-layer bot.
//!
//! Everything before this chose actions greedily and hoped a strategy
//! emerged. Conductor inverts that: the guides print the winning script
//! (T2 baton→9 + 2点, T3 + 3点, T4 multi-life 5–6点, T5 closeout), so each
//! turn has a TARGET SCORE, and every decision serves one question:
//!
//!   "does this raise P(I place ≥target this live phase)?"
//!
//! Live set: set the MINIMAL portfolio reaching the target (extra points
//! above a won comparison are wasted ammo — one card places regardless,
//! rule 8.4.7), fill spare slots with junk draws. If the race read says the
//! opponent's ceiling beats anything we can pass, take the PLANNED
//! development line (junk-dig + grow) instead of donating a life — that is
//! 温存 as arithmetic, not mood.
//!
//! Reuses the calibrated machinery from strategy_v4/v5 (per-color flip
//! model, binomial floors); adds zero new tuning knobs beyond the curve.

use crate::bot::strategy_v4::{alloc, hand_lives, heart_pool, lives_in_hand, passable_count};
use crate::card::{CardDatabase, CardType};
use crate::game_setup::{self, Action};
use crate::game_state::GameState;

use super::strategy_v5::player_ref;

/// Guide curve: the score a developed deck should be able to place per turn.
pub fn curve_target(turn: u8) -> i32 {
    match turn {
        0 | 1 => 1,
        2 => 2,
        3 => 3,
        4 => 5,
        5 => 6,
        _ => 8,
    }
}

/// The plan is a RACE AGAINST OURSELVES: hit the curve every turn, place
/// every turn, and let the opponent answer it. No ceiling comparisons, no
/// race reads, no digging because "they're bigger" — that machinery kept
/// producing bots that never place (measured repeatedly). Opponent state
/// enters ONLY through rule facts: either side at 2 successes changes stakes.
struct Plan {
    /// Score the script calls for this turn.
    target: i32,
}

fn read_plan(gs: &GameState) -> Plan {
    Plan {
        target: curve_target(gs.turn_number),
    }
}

// ── Main phase ──────────────────────────────────────────────────────────

pub fn choose_main_conductor(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }
    let db = &gs.card_database;
    let (my_now, _) = player_ref(gs, me);
    let plan = read_plan(gs);
    let base_hand_len = my_now.hand.cards.len() as i32;
    let base_passable = passable_count(gs, me, db);
    let base_ammo = lives_in_hand(my_now, db);
    // PROGRESS SIGNAL: the unfloored achievable ceiling. The floored
    // projection reads 0 most of the early game, which flattened the
    // gradient to zero -- conductor passed every main phase, hoarded
    // energy, and never grew into a placeable position (measured).
    let ceiling = |gs_: &GameState| -> i32 {
        passing_portfolios(gs_, me, db)
            .iter()
            .map(|(s, _)| *s)
            .max()
            .unwrap_or(0)
    };
    let base_proj = ceiling(gs);
    let stage_hearts_of = |p: &crate::player::Player| -> i32 {
        p.stage.stage.iter().filter(|&&c| c >= 0)
            .map(|&c| db.get_card(c).and_then(|x| x.base_heart.as_ref()).map(|bh| bh.hearts.values_sum() as i32).unwrap_or(0))
            .sum()
    };
    let base_stage = stage_hearts_of(my_now);
    let clamp = |x: i32| x.min(plan.target);
    let base_progress = clamp(base_proj);

    let deck_lives = my_now
        .main_deck
        .cards
        .iter()
        .filter(|&&c| db.get_card(c).map_or(false, |x| x.card_type == CardType::Live))
        .count();
    let deck_len = my_now.main_deck.cards.len().max(1);
    let p_life_draw = deck_lives as f64 / deck_len as f64;

    let mut best_idx = 0usize;
    let mut best_val = f64::NEG_INFINITY;

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        game_setup::settle_single_player_state(&mut sim);
        let my_sim = if me == 0 { &sim.player1 } else { &sim.player2 };

        let mut val = 0.0f64;

        // Passable-lives delta (proven).
        let passable_after = passable_count(&sim, me, db);
        val += 60.0 * (passable_after as f64 - base_passable as f64);

        // Ammo + burn penalty (proven).
        let ammo_after = lives_in_hand(my_sim, db);
        val += 25.0 * (ammo_after as f64 - base_ammo as f64);
        if ammo_after == 0 && base_ammo > 0 {
            val -= 120.0;
        }

        // PLAN PROGRESS: climbing toward this turn's target counts in full;
        // overshoot past target still helps the comparison, but less.
        let proj_after = ceiling(&sim);
        let progress_after = clamp(proj_after);
        let mut gain = 12.0 * (progress_after - base_progress) as f64;
        gain += 3.0 * ((proj_after - progress_after) - (base_proj - base_progress)) as f64;
        if base_progress < plan.target && progress_after >= plan.target {
            gain += 40.0; // crossing the threshold this turn is the whole job
        }
        val += gain;

        // Hand reserve.
        if my_sim.hand.cards.len() <= 1 {
            val -= 60.0;
        }

        // Starvation digging (proven).
        if base_ammo <= 1 {
            let drawn = (my_sim.hand.cards.len() as i32 - base_hand_len).max(0);
            val += 70.0 * p_life_draw * drawn as f64;
        }

        // Development gradient (proven v4 term): hearts gained ALWAYS count,
        // even when nothing passes the floor yet -- without this the bot had
        // no reason to play any particular member while below the curve.
        let stage_after = stage_hearts_of(my_sim);
        val += 3.0 * (stage_after - base_stage) as f64;

        // No-op breaker.
        if my_sim.hand.cards.len() == my_now.hand.cards.len()
            && my_sim.energy_zone.active_count() == my_now.energy_zone.active_count()
            && my_sim.stage.stage == my_now.stage.stage
            && my_sim.main_deck.cards.len() == my_now.main_deck.cards.len()
            && my_sim.waitroom.cards.len() == my_now.waitroom.cards.len()
        {
            val -= 1000.0;
        }

        if val > best_val {
            best_val = val;
            best_idx = i;
        }
    }
    actions[best_idx].clone()
}

// ── Live set ────────────────────────────────────────────────────────────

/// Passing portfolios (mean pool), scored. Returns (score, indices).
fn passing_portfolios(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<(i32, Vec<usize>)> {
    let (my, _) = player_ref(gs, me);
    let pool = heart_pool(gs, me, db);
    let lives = hand_lives(my, db);
    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    let mut out = Vec::new();
    if lives.is_empty() || max_slots == 0 {
        return out;
    }
    let n = lives.len().min(8);
    for mask in 1..(1u32 << n) {
        let cnt = mask.count_ones() as usize;
        if cnt > max_slots {
            continue;
        }
        let mut p = pool;
        let mut score = 0i32;
        let mut idxs = Vec::with_capacity(cnt);
        let mut ok = true;
        for bit in 0..n {
            if mask & (1 << bit) != 0 {
                let (hi, cid, ref need) = lives[bit];
                match alloc(&p, need) {
                    Some(next) => {
                        p = next;
                        score += db.get_card(cid).and_then(|c| c.score).unwrap_or(0) as i32;
                        idxs.push(hi);
                    }
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
        }
        if ok {
            out.push((score, idxs));
        }
    }
    out
}

pub fn choose_live_set_conductor(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let (my, opp) = player_ref(gs, me);
    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;

    // MEAN-POOL passable only (no binomial floor): measured vs v2, floor-
    // filtering starved placements — failed checks cost almost nothing
    // (lives recycle via refresh) while refusing to place loses the race.
    let cands: Vec<(i32, Vec<usize>)> = passing_portfolios(gs, me, db);

    // Free win: sole passer places regardless of score (8.4.3.2).
    if gs.current_phase == crate::game_state::Phase::LiveCardSetSecondAttacker
        && opp.live_card_zone.cards.is_empty()
    {
        if let Some((_, idxs)) = cands.iter().min_by_key(|(s, _)| *s) {
            let mut desired = idxs.clone();
            fill_junk(gs, me, db, &mut desired, max_slots);
            return super::strategy_v5::emit(gs, actions, &desired);
        }
    }
    // Selection: MAXIMIZE EXPECTED PLACED SCORE among reliable portfolios.
    // Measured: minimal-curve-target placement loses ~every contested
    // comparison against an opponent that dumps max score every turn --
    // margins DO matter in bot play even though one card places regardless.
    // The plan layer lives in the MAIN phase (development targets); here we
    // simply want the portfolio most likely to place.
    let mut desired: Vec<usize> = super::strategy_v5::best_portfolio_scored(gs, me, db).0;

    // Spare slots / planned dig: junk draws (8.2 + 8.3.4 hand filtering).
    fill_junk(gs, me, db, &mut desired, max_slots);
    super::strategy_v5::emit(gs, actions, &desired)
}

fn fill_junk(gs: &GameState, me: u8, db: &CardDatabase, desired: &mut Vec<usize>, max_slots: usize) {
    let (my, _) = player_ref(gs, me);
    let deck_lives = my
        .main_deck
        .cards
        .iter()
        .filter(|&&cid| db.get_card(cid).map_or(false, |c| c.card_type == CardType::Live))
        .count();
    if desired.len() >= max_slots || deck_lives == 0 {
        return;
    }
    // Protect the late curve: never junk a member we can afford now or
    // reasonably next turn. Energy cards and unaffordable members go first.
    let budget = my.energy_zone.active_count() as i32 + 4;
    let mut junk: Vec<(usize, i32)> = my
        .hand
        .cards
        .iter()
        .enumerate()
        .filter(|&(i, &cid)| {
            !desired.contains(&i)
                && db.get_card(cid).map_or(false, |c| c.card_type != CardType::Live)
        })
        .map(|(i, &cid)| {
            let cost = db.get_card(cid).and_then(|c| c.cost).unwrap_or(0) as i32;
            (i, cost)
        })
        .collect();
    junk.sort_by_key(|&(_, cost)| std::cmp::Reverse(cost.min(budget)));
    for (hi, _cost) in junk {
        if desired.len() >= max_slots {
            break;
        }
        desired.push(hi);
    }
}
