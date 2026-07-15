use rand::Rng;

use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;

use super::determinization::DeterminizationSampler;
use super::neural::PolicyNet;
use super::observation::PublicObservation;

/// 1-ply search using VALUE HEAD: apply each action, evaluate state, pick best.
pub fn search_1ply(
    obs: &PublicObservation,
    actions: &[Action],
    _config: &super::BotConfig,
    sampler: &DeterminizationSampler,
    network: &PolicyNet,
) -> usize {
    let mut best_idx = 0usize;
    let mut best_score = f64::NEG_INFINITY;
    let base = sampler.sample(obs);

    for (i, action) in actions.iter().enumerate() {
        let post = clone_and_settle(&base, action);
        let my_cards: Vec<i16> = obs
            .me
            .stage
            .iter()
            .chain(post.player1.hand.cards.iter())
            .copied()
            .collect();
        let opp_cards: Vec<i16> = obs
            .opp
            .stage
            .iter()
            .chain(post.player2.hand.cards.iter())
            .copied()
            .collect();
        // Use VALUE HEAD: predict expected success zone count
        let value = network.state_value(&my_cards, &opp_cards) as f64;
        if value > best_score {
            best_score = value;
            best_idx = i;
        }
    }
    best_idx
}

fn clone_and_settle(state: &GameState, action: &Action) -> GameState {
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

pub(super) fn fastrand(lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    rand::thread_rng().gen_range(lo..hi)
}
