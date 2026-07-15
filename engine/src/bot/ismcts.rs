use rand::Rng;

use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;

use super::determinization::DeterminizationSampler;
use super::neural::ValueNetwork;
use super::observation::PublicObservation;
use super::BotConfig;

/// Deep-clone state and apply an action.
fn clone_and_settle_from_action(state: &GameState, action: &Action) -> GameState {
    let mut next = state.clone();
    let params = action.parameters.clone();
    let _ = crate::turn::TurnEngine::execute_main_phase_action(
        &mut next,
        &action.action_type,
        params.as_ref().and_then(|p| p.card_id),
        params.as_ref().and_then(|p| p.card_indices.clone()),
        params
            .as_ref()
            .and_then(|p| p.stage_area.as_deref().and_then(|s| s.parse().ok())),
        params.as_ref().and_then(|p| p.use_baton_touch),
    );
    crate::game_setup::settle_single_player_state(&mut next);
    next
}

/// Score an action by cheap heuristics — no clone needed.
fn heuristic_score(action: &Action, obs: &PublicObservation) -> f64 {
    match action.action_type {
        ActionType::PlayMemberToStage => {
            if let Some(ref p) = action.parameters {
                if let Some(cid) = p.card_id {
                    let free = obs.me.stage.iter().any(|&id| id < 0);
                    if free {
                        return 2.0;
                    }
                    return 1.5;
                }
            }
            1.0
        }
        ActionType::UseAbility => 1.3,
        ActionType::SetLiveCard | ActionType::ConfirmLiveCardSet | ActionType::SelectLiveCard => {
            3.0
        }
        ActionType::Pass => 0.0,
        ActionType::SkipMulligan => 0.0,
        _ => 0.5,
    }
}

/// 1-ply search: filter to top-K by heuristic, then evaluate each with neural net.
pub fn search_1ply(
    obs: &PublicObservation,
    actions: &[Action],
    _config: &BotConfig,
    sampler: &DeterminizationSampler,
    network: &ValueNetwork,
) -> usize {
    if actions.len() <= 5 {
        // Small action set — evaluate all
        return eval_all(obs, actions, sampler, network);
    }

    // Score all actions by cheap heuristic, pick top 5
    let mut scored: Vec<(f64, usize)> = actions
        .iter()
        .enumerate()
        .map(|(i, a)| (heuristic_score(a, obs), i))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let top_n = 5.min(actions.len());
    let top_indices: Vec<usize> = scored[..top_n].iter().map(|(_, i)| *i).collect();

    // Neural eval on the top candidates only
    let mut best_idx = top_indices[0];
    let mut best_score = f64::NEG_INFINITY;
    let base = sampler.sample(obs);
    let my_cards: Vec<i16> = obs
        .me
        .stage
        .iter()
        .chain(obs.me.hand.iter())
        .copied()
        .collect();
    let opp_cards_base: Vec<i16> = obs.opp.stage.iter().copied().collect();

    for &idx in &top_indices {
        let post = clone_and_settle_from_action(&base, &actions[idx]);
        let opp_cards: Vec<i16> = opp_cards_base
            .iter()
            .chain(post.player2.hand.cards.iter())
            .copied()
            .collect();
        let score = network.predict(&my_cards, &opp_cards) as f64;
        if score > best_score {
            best_score = score;
            best_idx = idx;
        }
    }
    best_idx
}

fn eval_all(
    obs: &PublicObservation,
    actions: &[Action],
    sampler: &DeterminizationSampler,
    network: &ValueNetwork,
) -> usize {
    let mut best_idx = 0usize;
    let mut best_score = f64::NEG_INFINITY;
    let base = sampler.sample(obs);
    let my_cards: Vec<i16> = obs
        .me
        .stage
        .iter()
        .chain(obs.me.hand.iter())
        .copied()
        .collect();
    let opp_cards_base: Vec<i16> = obs.opp.stage.iter().copied().collect();
    for (i, action) in actions.iter().enumerate() {
        let post = clone_and_settle_from_action(&base, action);
        let opp_cards: Vec<i16> = opp_cards_base
            .iter()
            .chain(post.player2.hand.cards.iter())
            .copied()
            .collect();
        let score = network.predict(&my_cards, &opp_cards) as f64;
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }
    best_idx
}

pub(super) fn fastrand(lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    rand::thread_rng().gen_range(lo..hi)
}
