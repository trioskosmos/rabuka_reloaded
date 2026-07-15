use rand::Rng;

use crate::game_setup::Action;
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

/// 1-ply search: evaluate each action by determinizing + applying + neural eval.
pub fn search_1ply(
    obs: &PublicObservation,
    actions: &[Action],
    _config: &BotConfig,
    sampler: &DeterminizationSampler,
    network: &ValueNetwork,
) -> usize {
    let mut best_idx = 0usize;
    let mut best_score = f64::NEG_INFINITY;
    // Single determinization for consistency across all actions
    let base = sampler.sample(obs);
    // Use only my hand + stage for "my" cards (known), opponent's stage only (public)
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
        // Evaluate: my cards (from observation) + opponent stage + sampled hand (from determinization)
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
