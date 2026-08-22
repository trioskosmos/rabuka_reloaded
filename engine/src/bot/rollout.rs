//! Rollout-priced live-set decision (v7).
//!
//! Prices full live portfolios by playing them out: for each candidate
//! portfolio, clone the real state, apply the selection + confirm through
//! engine actions, then let BOTH sides continue with the proven v4/v5
//! policies until the end of the next turn (or game end). Average outcome
//! over N rollouts picks the winner.
//!
//! Fairness: policies never read the opponent's hidden zones, so rolling out
//! on clones of the true state leaks nothing; trajectory variance comes from
//! engine RNG (yell shuffles) across rollouts.
//!
//! Cost control: pricing runs only for CONTESTED checks (opponent zone
//! non-empty or opponent at match point); uncontested checks fall back to
//! the cheap heuristic path, which already plays them correctly.

use crate::bot::strategy_v4::{alloc, flip_stats, hand_lives, heart_pool};
use crate::card::CardDatabase;
use crate::game_setup::{self, Action};
use crate::game_state::{GameResult, GameState};

use super::strategy_v5::{binom_ge, nearest_miss_life, player_ref};

/// Number of candidate portfolios priced per contested decision.
const TOP_K: usize = 4;
/// Rollouts per candidate.
const SIMS_PER_CANDIDATE: usize = 6;
/// Play out until this many turns beyond the decision turn have STARTED.
const HORIZON_TURNS: u8 = 2;
/// Hard iteration cap per rollout.
const MAX_ITERS: usize = 500;

fn value_outcome(gs: &GameState, me: u8, start_succ: (i32, i32)) -> f64 {
    let (my, opp) = player_ref(gs, me);
    match gs.game_result {
        GameResult::FirstAttackerWins => {
            if me == 0 {
                10000.0
            } else {
                -10000.0
            }
        }
        GameResult::SecondAttackerWins => {
            if me == 0 {
                -10000.0
            } else {
                10000.0
            }
        }
        GameResult::Draw => 0.0,
        GameResult::Ongoing => {
            let my_now = my.success_live_card_zone.cards.len() as i32;
            let opp_now = opp.success_live_card_zone.cards.len() as i32;
            100.0 * ((my_now - start_succ.0) - (opp_now - start_succ.1)) as f64
        }
    }
}

/// Apply a full portfolio (select toggles + confirm) in `sim` using the
/// currently offered action list shape. Returns false if execution failed.
fn apply_portfolio(sim: &mut GameState, desired: &[usize]) -> bool {
    // Deselect everything first so indices map cleanly onto our list.
    loop {
        let acts = game_setup::generate_possible_actions(sim);
        let deselect = acts.iter().find(|a| {
            a.action_type == game_setup::ActionType::SelectLiveCard && a.selected == Some(true)
        });
        match deselect {
            Some(a) => {
                if game_setup::execute_action(sim, a).is_err() {
                    return false;
                }
                game_setup::settle_single_player_state(sim);
            }
            None => break,
        }
    }
    for &hi in desired {
        let acts = game_setup::generate_possible_actions(sim);
        let Some(a) = acts.iter().find(|a| {
            a.action_type == game_setup::ActionType::SelectLiveCard
                && a.selected == Some(false)
                && a.parameters.as_ref().and_then(|p| p.card_index) == Some(hi)
        }) else {
            return false;
        };
        if game_setup::execute_action(sim, a).is_err() {
            return false;
        }
        game_setup::settle_single_player_state(sim);
    }
    let acts = game_setup::generate_possible_actions(sim);
    match acts.iter().find(|a| a.action_type == game_setup::ActionType::ConfirmLiveCardSet) {
        Some(a) => game_setup::execute_action(sim, a).is_ok(),
        None => false,
    }
}

fn rollout_value(
    sim: &mut GameState,
    me: u8,
    start_turn: u8,
    start_succ: (i32, i32),
    horizon_end: u8,
) -> f64 {
    let mut iters = 0usize;
    while iters < MAX_ITERS {
        iters += 1;
        crate::turn::TurnEngine::check_victory_condition(sim);
        if sim.game_result != GameResult::Ongoing {
            break;
        }
        let turn_done = sim.turn_number > horizon_end || sim.turn_number.wrapping_sub(start_turn) > HORIZON_TURNS;
        if turn_done {
            break;
        }
        if game_setup::auto_advance_one(sim) {
            continue;
        }
        let actions = game_setup::generate_possible_actions(sim);
        if actions.is_empty() {
            crate::turn::TurnEngine::advance_phase(sim);
            continue;
        }
        let active_is_p1 = sim.active_player().id == "p1";
        let side_me = if active_is_p1 { 0u8 } else { 1u8 };

        let action = match sim.current_phase {
            crate::game_state::Phase::RockPaperScissors
            | crate::game_state::Phase::ChooseFirstAttacker => {
                actions[0].clone()
            }
            crate::game_state::Phase::MulliganFirstAttacker
            | crate::game_state::Phase::MulliganSecondAttacker => {
                super::strategy_v4::choose_mulligan_v4(sim, &actions, &sim.card_database)
            }
            crate::game_state::Phase::LiveCardSetFirstAttacker
            | crate::game_state::Phase::LiveCardSetSecondAttacker => {
                super::strategy_v5::choose_live_set_v5(sim, &actions, &sim.card_database)
            }
            _ => {
                if side_me == me {
                    super::strategy_v5::choose_action_v6(sim, &actions, side_me)
                } else {
                    super::strategy_v4::choose_action_v4(sim, &actions, side_me)
                }
            }
        };
        if game_setup::execute_action(sim, &action).is_err() {
            // Illegal offer under rollout state 窶・drop it and move on.
            crate::turn::TurnEngine::advance_phase(sim);
            continue;
        }
        game_setup::settle_single_player_state(sim);
    }
    crate::turn::TurnEngine::check_victory_condition(sim);
    value_outcome(sim, me, start_succ)
}

/// Price candidate portfolios by rollout. Returns the index into
/// `candidates` of the highest-average-value portfolio.
pub fn price_portfolios(
    gs: &GameState,
    me: u8,
    candidates: &[Vec<usize>],
    offered: &[Action],
) -> usize {
    let start_turn = gs.turn_number;
    let start_succ = (
        gs.player1.success_live_card_zone.cards.len() as i32,
        gs.player2.success_live_card_zone.cards.len() as i32,
    );
    let horizon_end = start_turn.saturating_add(HORIZON_TURNS);
    let mut totals = vec![0.0f64; candidates.len()];
    for (ci, cand) in candidates.iter().enumerate() {
        for _ in 0..SIMS_PER_CANDIDATE {
            let mut sim = gs.clone();
            if !apply_portfolio(&mut sim, cand) {
                totals[ci] -= 500.0 / SIMS_PER_CANDIDATE as f64;
                continue;
            }
            // Re-generate offers post-application; confirm may already have
            // advanced the phase, in which case the rollout continues below.
            if sim.current_phase == gs.current_phase {
                let acts = game_setup::generate_possible_actions(&sim);
                if let Some(a) = acts
                    .iter()
                    .find(|a| a.action_type == game_setup::ActionType::ConfirmLiveCardSet)
                {
                    let _ = game_setup::execute_action(&mut sim, a);
                    game_setup::settle_single_player_state(&mut sim);
                }
            }
            let v = rollout_value(&mut sim, me, start_turn, start_succ, horizon_end);
            totals[ci] += v;
        }
        totals[ci] /= SIMS_PER_CANDIDATE as f64;
    }
    let mut best = 0usize;
    for (i, &t) in totals.iter().enumerate() {
        if t > totals[best] {
            best = i;
        }
    }
    let _ = offered;
    best
}

/// Enumerate candidate portfolios for rollout pricing: every mask that
/// allocs against the MEAN pool, ranked by quick EV (P(pass) ﾃ・score),
/// capped at TOP_K; plus the empty (junk-dig) baseline; plus the best
/// probability-gated gamble life when nothing deterministic passes.
pub fn enumerate_candidates(gs: &GameState, me: u8, db: &CardDatabase) -> Vec<Vec<usize>> {
    let (my, _) = player_ref(gs, me);
    let pool = heart_pool(gs, me, db);
    let lives = hand_lives(my, db);
    let max_slots =
        (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
    let (blades, density) = flip_stats(gs, me, db);

    let mut scored: Vec<(f64, Vec<usize>)> = Vec::new();
    if !lives.is_empty() && max_slots > 0 {
        let n = lives.len().min(8);
        for mask in 1..(1u32 << n) {
            let cnt = mask.count_ones() as usize;
            if cnt > max_slots {
                continue;
            }
            let mut p = pool;
            let mut score = 0i32;
            let mut total_req = 0i32;
            let mut idxs = Vec::with_capacity(cnt);
            let mut ok = true;
            for bit in 0..n {
                if mask & (1 << bit) != 0 {
                    let (hi, cid, ref need) = lives[bit];
                    match alloc(&p, need) {
                        Some(next) => {
                            p = next;
                            score +=
                                db.get_card(cid).and_then(|c| c.score).unwrap_or(0) as i32;
                            total_req += (0..=7)
                                .chain(std::iter::once(10))
                                .map(|i| need[i])
                                .sum::<i32>();
                            idxs.push(hi);
                        }
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
            }
            if !ok {
                continue;
            }
            let ev = score as f64 * binom_ge(blades, total_req.max(0), density);
            scored.push((ev, idxs));
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(TOP_K);

    let any_passing = !scored.is_empty();
    let mut candidates: Vec<Vec<usize>> = scored.into_iter().map(|(_, idxs)| idxs).collect();
    // NOTE: no empty/junk-dig baseline here -- measured pathology (2026-08-22):
    // with a 2-turn horizon the rollout valued immediate hand-refresh over
    // committing lives (junk-dig won argmax -> placement starvation -> losses).
    // Set-vs-dig stays the heuristic path's job; rollouts only choose among
    // portfolios that pass.
    if !any_passing {
        if let Some((_p, _deficit, hi)) = nearest_miss_life(gs, me, db) {
            candidates.push(vec![hi]);
        }
        candidates.push(Vec::new());
    }
    candidates
}

// ・ｽE・ｽ・ｽE・ｽ・ｽE・ｽ・ｽE・ｽ v7 entry point: plan-cache keeps multi-tick commitments stable ・ｽE・ｽ・ｽE・ｽ・ｽE・ｽ・ｽE・ｽ
//
// Live-set decisions resolve over several action ticks (select, select,
// confirm) while `choose_live_set_*` is called statelessly each tick.
// Rollout pricing is stochastic and expensive, so the chosen plan is cached
// per (turn, phase-side, hand fingerprint) and replayed until confirmed.

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

thread_local! {
    static PLAN_CACHE: RefCell<HashMap<(u8, u8, u64), Vec<usize>>> =
        RefCell::new(HashMap::new());
}

fn plan_key(gs: &GameState, me: u8) -> (u8, u8, u64) {
    let (my, _) = player_ref(gs, me);
    let side = if matches!(
        gs.current_phase,
        crate::game_state::Phase::LiveCardSetFirstAttacker
    ) {
        0u8
    } else {
        1u8
    };
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for &cid in &my.hand.cards {
        cid.hash(&mut h);
    }
    my.success_live_card_zone.cards.len().hash(&mut h);
    gs.turn_number.hash(&mut h);
    (side, gs.turn_number, h.finish())
}

/// Rollout-priced live-set decision. Contested checks are priced by playing
/// candidate portfolios out with both sides on proven policies; uncontested
/// checks delegate to the cheap heuristic path (v5 logic).
pub fn choose_live_set_v7(gs: &GameState, actions: &[Action], db: &CardDatabase) -> Action {
    let me = if gs.active_player().id == gs.player1.id { 0u8 } else { 1u8 };
    let (_, opp) = player_ref(gs, me);
    let opp_succ = opp.success_live_card_zone.cards.len() as i32;
    let contested = !opp.live_card_zone.cards.is_empty() || opp_succ >= 2;

    let key = plan_key(gs, me);
    let cached = PLAN_CACHE.with(|c| c.borrow().get(&key).cloned());
    if let Some(plan) = cached {
        return super::strategy_v5::emit(gs, actions, &plan);
    }

    let desired: Vec<usize> = if contested {
        let candidates = enumerate_candidates(gs, me, db);
        let idx = price_portfolios(gs, me, &candidates, actions);
        let mut plan = candidates[idx].clone();
        // Fill spare slots with junk draws exactly like the heuristic path,
        // so the priced comparison matches what will actually be set.
        let (my, _) = player_ref(gs, me);
        let max_slots =
            (3i32 - i32::from(my.live_card_set_limit_reduction)).max(0) as usize;
        let deck_lives = my
            .main_deck
            .cards
            .iter()
            .filter(|&&cid| {
                db.get_card(cid).map_or(false, |c| {
                    c.card_type == crate::card::CardType::Live
                })
            })
            .count();
        if !plan.is_empty() && plan.len() < max_slots && deck_lives > 0 {
            let mut junk: Vec<usize> = my
                .hand
                .cards
                .iter()
                .enumerate()
                .filter(|&(i, &cid)| {
                    !plan.contains(&i)
                        && db.get_card(cid).map_or(false, |c| {
                            c.card_type != crate::card::CardType::Live
                        })
                })
                .map(|(i, _)| i)
                .collect();
            junk.sort_by_key(|&i| {
                std::cmp::Reverse(
                    db.get_card(my.hand.cards[i])
                        .and_then(|c| c.cost)
                        .unwrap_or(0),
                )
            });
            for hi in junk {
                if plan.len() >= max_slots {
                    break;
                }
                plan.push(hi);
            }
        } else if plan.is_empty() && deck_lives > 0 {
            let mut junk: Vec<usize> = my
                .hand
                .cards
                .iter()
                .enumerate()
                .filter(|&(_, &cid)| {
                    db.get_card(cid).map_or(false, |c| {
                        c.card_type != crate::card::CardType::Live
                    })
                })
                .map(|(i, _)| i)
                .collect();
            junk.sort_by_key(|&i| {
                std::cmp::Reverse(
                    db.get_card(my.hand.cards[i])
                        .and_then(|c| c.cost)
                        .unwrap_or(0),
                )
            });
            for hi in junk {
                if plan.len() >= max_slots.min(3) {
                    break;
                }
                plan.push(hi);
            }
        }
        PLAN_CACHE.with(|c| {
            let mut m = c.borrow_mut();
            if m.len() > 4096 {
                m.clear();
            }
            m.insert(key, plan.clone());
        });
        plan
    } else {
        // Uncontested: the stateless heuristic path is already correct and
        // cheap 窶・no cache needed since it is deterministic tick-to-tick.
        return super::strategy_v5::choose_live_set_v5(gs, actions, db);
    };
    super::strategy_v5::emit(gs, actions, &desired)
}


