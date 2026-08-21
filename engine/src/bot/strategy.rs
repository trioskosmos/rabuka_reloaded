//! Experimental heuristic strategy for the Loveca bot.
//!
//! Encodes deck-agnostic competitive principles researched from Japanese
//! play guides (see `docs/BOT_STRATEGY.md`):
//! - S1 cost curve / stage total-cost race
//! - S2 score-band prediction from public hearts + blades
//! - S3 live-card discipline (don't throw lives you can't win)
//! - S4 concede-a-turn vs opponent-at-2 urgency
//! - S5 blade/yell economics
//! - S6 turn-order (first attacker) tempo value
//!
//! Fairness: evaluation only reads public information about the opponent
//! (stage, energy count, waitroom, success zone, hand *size*). It never reads
//! the opponent's hand contents or deck order, even when the full GameState
//! is available.

use crate::card::{CardDatabase, CardType};
use crate::game_setup::{self, Action};
use crate::game_state::{GameResult, GameState};
use crate::player::Player;

/// Weights for the heuristic evaluation. Tuned by hand to start; a tuning
/// harness (self-play vs random / vs flat MC) should refine these later.
#[derive(Debug, Clone)]
pub struct StrategyWeights {
    /// Terminal: won the game.
    pub win: f64,
    /// Terminal: lost the game.
    pub loss: f64,
    /// Per success-live-card advantage (scaled by urgency when opp at 2).
    pub success_card: f64,
    /// Extra multiplier on success terms when the opponent has 2 success
    /// cards and we have fewer (千秋楽 — must contest or lose).
    pub urgency_mult: f64,
    /// Per point of stage total-cost difference (S1).
    pub stage_cost: f64,
    /// Per heart on stage difference (S2).
    pub heart: f64,
    /// Per blade difference (S5).
    pub blade: f64,
    /// Per active energy difference (S1 efficiency).
    pub active_energy: f64,
    /// Per card of hand-size difference.
    pub hand_size: f64,
    /// Bonus per live card held in hand (S3 discipline — lives are ammo).
    pub hand_live_card: f64,
    /// Bonus for being first attacker (S6).
    pub first_attacker: f64,
}

impl Default for StrategyWeights {
    fn default() -> Self {
        Self {
            win: 10_000.0,
            loss: -10_000.0,
            success_card: 300.0,
            urgency_mult: 3.0,
            stage_cost: 8.0,
            heart: 12.0,
            blade: 6.0,
            active_energy: 3.0,
            hand_size: 4.0,
            hand_live_card: 15.0,
            first_attacker: 20.0,
        }
    }
}

impl StrategyWeights {
    pub const fn fair() -> Self {
        Self {
            win: 10_000.0,
            loss: -10_000.0,
            success_card: 300.0,
            urgency_mult: 3.0,
            stage_cost: 8.0,
            heart: 12.0,
            blade: 6.0,
            active_energy: 3.0,
            hand_size: 4.0,
            hand_live_card: 15.0,
            first_attacker: 20.0,
        }
    }
}

/// Public-ish view of one side used by the evaluator. Only fields a real
/// player may see are populated from the opponent.
pub struct SideView {
    pub success_count: usize,
    pub stage_total_cost: i32,
    pub stage_hearts: i32,
    pub stage_blades: i32,
    pub active_energy: i32,
    pub hand_size: usize,
    pub hand_live_cards: usize,
    pub is_first_attacker: bool,
}

fn view_of(p: &Player, db: &CardDatabase, public_only: bool) -> SideView {
    let mut stage_total_cost = 0i32;
    let mut stage_hearts = 0i32;
    let mut stage_blades = 0i32;
    for &cid in p.stage.stage.iter() {
        if cid < 0 {
            continue;
        }
        if let Some(card) = db.get_card(cid) {
            stage_total_cost += card.cost.unwrap_or(0) as i32;
            stage_blades += card.blade as i32;
            if let Some(bh) = &card.base_heart {
                stage_hearts += bh.hearts.values_sum() as i32;
            }
        }
    }

    // Hand contents are only visible for our own side.
    let mut hand_live_cards = 0usize;
    if !public_only {
        for &cid in p.hand.cards.iter() {
            if let Some(card) = db.get_card(cid) {
                if matches!(card.card_type, CardType::Live) {
                    hand_live_cards += 1;
                }
            }
        }
    }

    SideView {
        success_count: p.success_live_card_zone.cards.len(),
        stage_total_cost,
        stage_hearts,
        stage_blades,
        active_energy: p.energy_zone.active_count() as i32,
        hand_size: p.hand.cards.len(),
        hand_live_cards,
        is_first_attacker: p.is_first_attacker,
    }
}

/// Heuristic evaluation of the position from `me`'s perspective (0 or 1).
/// Opponent info is restricted to public data.
pub fn evaluate_state(gs: &GameState, me: u8, w: &StrategyWeights) -> f64 {
    match &gs.game_result {
        GameResult::Ongoing => {}
        GameResult::Draw => return 0.0,
        GameResult::FirstAttackerWins => {
            let i_am_first = if me == 0 {
                gs.player1.is_first_attacker
            } else {
                gs.player2.is_first_attacker
            };
            return if i_am_first { w.win } else { w.loss };
        }
        GameResult::SecondAttackerWins => {
            let i_am_first = if me == 0 {
                gs.player1.is_first_attacker
            } else {
                gs.player2.is_first_attacker
            };
            return if i_am_first { w.loss } else { w.win };
        }
    }

    let db = &gs.card_database;
    let (my, opp) = if me == 0 {
        (
            view_of(&gs.player1, db, false),
            view_of(&gs.player2, db, true),
        )
    } else {
        (
            view_of(&gs.player2, db, false),
            view_of(&gs.player1, db, true),
        )
    };

    // S4: urgency — opponent at 2 success cards while we have fewer means the
    // next live check is effectively decisive.
    let urgency = if opp.success_count >= 2 && my.success_count < opp.success_count {
        w.urgency_mult
    } else {
        1.0
    };

    let success_diff = my.success_count as f64 - opp.success_count as f64;

    (success_diff * w.success_card) * urgency
        + ((my.stage_total_cost - opp.stage_total_cost) as f64) * w.stage_cost
        + ((my.stage_hearts - opp.stage_hearts) as f64) * w.heart
        + ((my.stage_blades - opp.stage_blades) as f64) * w.blade
        + ((my.active_energy - opp.active_energy) as f64) * w.active_energy
        + ((my.hand_size as f64 - opp.hand_size as f64)) * w.hand_size
        + (my.hand_live_cards as f64) * w.hand_live_card
        + if my.is_first_attacker { w.first_attacker } else { 0.0 }
}

/// S2: estimate the maximum live score a side can produce this turn from its
/// public board (hearts + blades). Score bands need roughly 2N+1..2N+2 hearts
/// (source 3), so max score ≈ (hearts + expected flips − 1) / 2.
pub fn estimate_max_score(view: &SideView) -> f64 {
    let total = view.stage_hearts + view.stage_blades;
    if total <= 0 {
        return 0.0;
    }
    ((total - 1) as f64 / 2.0).floor().max(0.0)
}

/// Live Card Set phase policy (S3): select up to 3 live cards, highest score
/// first, then confirm. Never deselects; confirms immediately once no
/// unselected live cards remain (or the limit is reached).
pub fn choose_live_set_action(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
) -> Action {
    let selected_count = gs.live_card_selected_indices.len();
    let mut best_select: Option<(usize, u8)> = None; // (action idx, score)

    for (i, a) in actions.iter().enumerate() {
        if a.action_type != game_setup::ActionType::SelectLiveCard {
            continue;
        }
        if a.selected == Some(true) {
            continue; // already selected; never deselect
        }
        let cid = match a.parameters.as_ref().and_then(|p| p.card_id) {
            Some(cid) => cid,
            None => continue,
        };
        let score = db.get_card(cid).and_then(|c| c.score).unwrap_or(0);
        if best_select.map_or(true, |(_, s)| score > s) {
            best_select = Some((i, score));
        }
    }

    match best_select {
        Some((i, _)) if selected_count < 3 => actions[i].clone(),
        _ => actions
            .iter()
            .find(|a| a.action_type == game_setup::ActionType::ConfirmLiveCardSet)
            .or_else(|| actions.first())
            .cloned()
            .expect("live set actions non-empty"),
    }
}

/// One-ply greedy action choice: clone-and-eval each legal action with the
/// heuristic. Used both as the rollout policy inside search and as a cheap
/// standalone "strategy bot". Falls back to the first action on ties/errors.
pub fn choose_action_heuristic(gs: &GameState, actions: &[Action], me: u8) -> Action {
    if actions.len() == 1 {
        return actions[0].clone();
    }

    let mut best_idx = 0usize;
    let mut best_val = f64::NEG_INFINITY;

    for (i, a) in actions.iter().enumerate() {
        let mut sim = gs.clone();
        let result = game_setup::execute_action(&mut sim, a);
        if result.is_err() {
            continue;
        }
        game_setup::settle_single_player_state(&mut sim);
        let val = evaluate_state(&sim, me, &StrategyWeights::fair());
        if val > best_val {
            best_val = val;
            best_idx = i;
        }
    }

    actions[best_idx].clone()
}
