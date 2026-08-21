//! Experimental strategy bot v3 — general planning.
//!
//! Adds a planning layer on top of v2's yell-math live-set policy:
//!
//! 1. **Archetype / rush window**: cheap lives in hand early → play them
//!    turns 1–4 ("zerg rush" the checks) while still growing the stage.
//!    High-score decks ramp instead.
//! 2. **Curve milestones**: the guides' stage-cost arc (~2 → 9 → 13/15 →
//!    22+) as explicit per-turn targets; the eval penalizes falling behind
//!    the curve.
//! 3. **Acquisition reasoning**: main-phase actions are valued partly by
//!    whether they *acquire wanted cards* — lives into hand during the rush
//!    window, ammo recycled from the waitroom, playable-cost members for the
//!    next curve step. (Outcome-delta approximation; semantic ability
//!    understanding is v4/v5.)
//!
//! Fairness: same rules as v2 — own deck/hand is known, opponent info is
//! public-only.

use crate::card::{CardDatabase, CardType};
use crate::game_setup::{self, Action};
use crate::game_state::GameState;
use crate::bot::strategy::StrategyWeights;
use crate::bot::strategy_v2::V2Policy;

/// Stage-total-cost milestones per turn (index 0 = turn 1). Derived from the
/// competitive guides' standard development: T1 opener, T2 baton to ~9,
/// T3 ~13–15, T4 the big center (~22+), then steady growth.
const CURVE: [i32; 8] = [2, 9, 15, 24, 30, 36, 42, 48];

/// Turns during which a cheap-life rush is viable.
const RUSH_WINDOW_END: u8 = 4;

pub struct V3Plan {
    /// Deck/hand skews cheap → flood early checks.
    pub rush_archetype: bool,
}

impl V3Plan {
    pub fn detect(gs: &GameState, me_player: u8, db: &CardDatabase) -> Self {
        let my = if me_player == 0 {
            &gs.player1
        } else {
            &gs.player2
        };
        // Cheap lives visible in hand + waitroom at game start indicate a
        // rush skeleton. (Deck order unknown pre-game; hand/waitroom are ours.)
        let mut cheap = 0usize;
        let mut total_lives = 0usize;
        for &cid in my.hand.cards.iter().chain(my.waitroom.cards.iter()) {
            if let Some(card) = db.get_card(cid) {
                if matches!(card.card_type, CardType::Live) {
                    total_lives += 1;
                    if card.score.unwrap_or(9) <= 2 {
                        cheap += 1;
                    }
                }
            }
        }
        let rush_archetype = total_lives > 0 && cheap * 2 >= total_lives;
        Self { rush_archetype }
    }

    pub fn in_rush_window(&self, turn: u8) -> bool {
        self.rush_archetype && turn <= RUSH_WINDOW_END
    }

    pub fn milestone(&self, turn: u8) -> i32 {
        CURVE[(turn as usize - 1).min(CURVE.len() - 1)]
    }
}

/// Cards-we-want snapshot used for acquisition deltas.
#[derive(Debug, Clone, Copy)]
struct AcqFeatures {
    lives_in_hand: usize,
    playable_members_in_hand: usize,
    lives_in_waitroom: usize,
}

fn acq_features(p: &crate::player::Player, db: &CardDatabase) -> AcqFeatures {
    let mut f = AcqFeatures {
        lives_in_hand: 0,
        playable_members_in_hand: 0,
        lives_in_waitroom: 0,
    };
    for &cid in p.hand.cards.iter() {
        if let Some(card) = db.get_card(cid) {
            match card.card_type {
                CardType::Live => f.lives_in_hand += 1,
                CardType::Member => {
                    if card.cost.unwrap_or(99) <= 15 {
                        f.playable_members_in_hand += 1;
                    }
                }
                CardType::Energy => {}
            }
        }
    }
    for &cid in p.waitroom.cards.iter() {
        if let Some(card) = db.get_card(cid) {
            if matches!(card.card_type, CardType::Live) {
                f.lives_in_waitroom += 1;
            }
        }
    }
    f
}

/// Curve adherence: penalize being UNDER the milestone (over-curving is fine
/// — extra stage cost is never bad, per S1).
fn curve_term(gs: &GameState, me_player: u8, plan: &V3Plan, w: &StrategyWeights) -> f64 {
    let my = if me_player == 0 {
        &gs.player1
    } else {
        &gs.player2
    };
    let mut stage_cost = 0i32;
    for &cid in my.stage.stage.iter() {
        if cid >= 0 {
            if let Some(card) = gs.card_database.get_card(cid) {
                stage_cost += card.cost.unwrap_or(0) as i32;
            }
        }
    }
    let milestone = plan.milestone(gs.turn_number);
    let shortfall = (milestone - stage_cost).max(0);
    -(shortfall as f64) * w.stage_cost * 0.8
}

/// V3 evaluation: v2 evaluation + curve adherence.
pub fn evaluate_state_v3(
    gs: &GameState,
    me: u8,
    w: &StrategyWeights,
    plan: &V3Plan,
) -> f64 {
    crate::bot::strategy_v2::evaluate_state_v2(gs, me, w) + curve_term(gs, me, plan, w)
}

/// V3 main-phase choice: v2's clone-and-eval plus acquisition deltas —
/// reward actions that obtain cards we want right now.
pub fn choose_action_heuristic_v3(
    gs: &GameState,
    actions: &[Action],
    me: u8,
    plan: &V3Plan,
) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }

    let db = &gs.card_database;
    let my_before = if me == 0 {
        acq_features(&gs.player1, db)
    } else {
        acq_features(&gs.player2, db)
    };
    let rush = plan.in_rush_window(gs.turn_number);

    let mut best_idx = 0usize;
    let mut best_val = f64::NEG_INFINITY;

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        if game_setup::execute_action(&mut sim, a).is_err() {
            continue;
        }
        game_setup::settle_single_player_state(&mut sim);
        let mut val =
            crate::bot::strategy_v2::evaluate_state_v2(&sim, me, &StrategyWeights::fair())
                + curve_term(&sim, me, plan, &StrategyWeights::fair());

        // Acquisition deltas (what did this action get me?).
        let my_after = if me == 0 {
            acq_features(&sim.player1, db)
        } else {
            acq_features(&sim.player2, db)
        };
        let d_lives = my_after.lives_in_hand as i32 - my_before.lives_in_hand as i32;
        let d_members = my_after.playable_members_in_hand as i32
            - my_before.playable_members_in_hand as i32;
        let d_wr_lives =
            my_after.lives_in_waitroom as i32 - my_before.lives_in_waitroom as i32;

        // Lives into hand: strong want during the rush window (ammo for the
        // flood), moderate otherwise.
        val += d_lives as f64 * if rush { 60.0 } else { 25.0 };
        // Recycled ammo (waitroom -> hand shows as wr decrease + hand gain);
        // slight extra credit because waitroom lives are "free" wins.
        if d_lives > 0 && d_wr_lives < 0 {
            val += 20.0;
        }
        // Playable members keep the curve moving.
        val += d_members as f64 * 12.0;

        if val > best_val {
            best_val = val;
            best_idx = i;
        }
    }

    actions[best_idx].clone()
}

/// V3 live-set policy: v2's yell-math selection, with rush-window tuning —
/// during turns 1–4 with a cheap-life skeleton, be more willing to gamble
/// (lower floor) since flooding early checks is the whole plan.
pub fn choose_live_set_action_v3(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
    policy: &V2Policy,
    plan: &V3Plan,
) -> Action {
    if plan.in_rush_window(gs.turn_number) {
        let relaxed = V2Policy {
            mc_trials: policy.mc_trials,
            gamble_floor: (policy.gamble_floor * 0.5).max(0.04),
            urgent_gamble_floor: policy.urgent_gamble_floor * 0.5,
        };
        return crate::bot::strategy_v2::choose_live_set_action_v2(gs, actions, db, &relaxed);
    }
    crate::bot::strategy_v2::choose_live_set_action_v2(gs, actions, db, policy)
}
