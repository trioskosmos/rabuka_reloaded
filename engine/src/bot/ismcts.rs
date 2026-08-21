use crate::game_setup::{self, Action};
use crate::game_state::GameResult;
use crate::turn::TurnEngine;

use super::determinization::DeterminizationSampler;
use super::evaluation::pick_rollout_action;
use super::observation::PublicObservation;

const MAX_ROLLOUT_DEPTH: u32 = 300;

/// Adaptive-sampling Monte Carlo (PIMC): actions are explored with a UCB1
/// bandit over determinized rollouts, so strong actions get more simulations
/// instead of a fixed even split. Rollouts use the heuristic policy from
/// `strategy.rs` (via `pick_rollout_action`).
pub fn search(
    obs: &PublicObservation,
    actions: &[Action],
    config: &super::BotConfig,
    sampler: &DeterminizationSampler,
) -> Action {
    if actions.is_empty() {
        return Action {
            description: "pass".into(),
            description_ja: None,
            action_type: crate::game_setup::ActionType::Pass,
            parameters: None,
            selected: None,
        };
    }
    if actions.len() == 1 {
        return actions[0].clone();
    }

    let start_success = obs.me.success_zone.len() as f64 - obs.opp.success_zone.len() as f64;
    let n_actions = actions.len();
    let mut scores = vec![0.0f64; n_actions];
    let mut counts = vec![0u32; n_actions];

    // One visit per action first so every option is tried at least once.
    let mut total_visits = 0u32;
    for i in 0..n_actions {
        let (value, won) = run_rollout(obs, sampler, config, &actions[i], start_success);
        scores[i] = value;
        counts[i] = 1;
        total_visits += 1;
        if won {
            return actions[i].clone();
        }
    }

    let budget = config.iterations.max(n_actions as u32);
    while total_visits < budget {
        // UCB1 selection.
        let mut best_idx = 0usize;
        let mut best_ucb = f64::NEG_INFINITY;
        for (i, &c) in counts.iter().enumerate() {
            let mean = scores[i] / c as f64;
            let explore =
                config.exploration_constant * (total_visits as f64).ln().max(0.0) / c as f64;
            let ucb = mean + explore.sqrt();
            if ucb > best_ucb {
                best_ucb = ucb;
                best_idx = i;
            }
        }

        let (value, won) = run_rollout(obs, sampler, config, &actions[best_idx], start_success);
        scores[best_idx] += value;
        counts[best_idx] += 1;
        total_visits += 1;

        if won {
            return actions[best_idx].clone();
        }
    }

    // Final pick: highest mean with a modest visit floor to avoid noise.
    let mut best_idx = 0usize;
    let mut best_rank = f64::NEG_INFINITY;
    for (i, &c) in counts.iter().enumerate() {
        let mean = scores[i] / c as f64;
        let rank = if c >= 3 { mean } else { mean - 100.0 };
        if rank > best_rank {
            best_rank = rank;
            best_idx = i;
        }
    }

    actions[best_idx].clone()
}

/// Run one determinized rollout for `action`; returns (normalized value, immediate_win).
fn run_rollout(
    obs: &PublicObservation,
    sampler: &DeterminizationSampler,
    _config: &super::BotConfig,
    action: &Action,
    start_success: f64,
) -> (f64, bool) {
    let mut state = sampler.sample(obs);

    // Apply this action
    let p = action.parameters.clone();
    let _ = TurnEngine::execute_main_phase_action(
        &mut state,
        &action.action_type,
        p.as_ref().and_then(|p| p.card_id),
        p.as_ref().and_then(|p| p.card_indices.clone()),
        p.as_ref()
            .and_then(|p| p.stage_area.as_deref().and_then(parse_area)),
        p.as_ref().and_then(|p| p.use_baton_touch),
    );
    game_setup::settle_single_player_state(&mut state);

    // Rollout
    for _ in 0..MAX_ROLLOUT_DEPTH {
        TurnEngine::check_victory_condition(&mut state);
        if state.game_result != GameResult::Ongoing {
            break;
        }
        if !state.has_pending_choice() {
            if crate::game_setup::is_automatic_phase(&state) {
                TurnEngine::advance_phase(&mut state);
                continue;
            }
        }
        let ra = game_setup::generate_possible_actions(&state);
        if ra.is_empty() {
            TurnEngine::advance_phase(&mut state);
            continue;
        }
        let chosen = pick_rollout_action(&ra, &state);
        let p2 = chosen.parameters.clone();
        let _ = TurnEngine::execute_main_phase_action(
            &mut state,
            &chosen.action_type,
            p2.as_ref().and_then(|p| p.card_id),
            p2.as_ref().and_then(|p| p.card_indices.clone()),
            p2.as_ref()
                .and_then(|p| p.stage_area.as_deref().and_then(parse_area)),
            p2.as_ref().and_then(|p| p.use_baton_touch),
        );
        game_setup::settle_single_player_state(&mut state);
    }

    TurnEngine::check_victory_condition(&mut state);
    // In sampled states player1 is always the bot's side.
    let me_is_first = state.player1.is_first_attacker;
    match state.game_result {
        GameResult::FirstAttackerWins => {
            if me_is_first {
                (1.0e4, true)
            } else {
                (-1.0e4, false)
            }
        }
        GameResult::SecondAttackerWins => {
            if me_is_first {
                (-1.0e4, false)
            } else {
                (1.0e4, true)
            }
        }
        GameResult::Draw => (0.0, false),
        GameResult::Ongoing => {
            let end_s = state.player1.success_live_card_zone.cards.len() as f64
                - state.player2.success_live_card_zone.cards.len() as f64;
            let turns = (state.turn_number - obs.turn_number).max(1) as f64;
            ((end_s - start_success) / turns, false)
        }
    }
}

pub fn search_1ply(
    obs: &PublicObservation,
    actions: &[Action],
    config: &super::BotConfig,
    sampler: &DeterminizationSampler,
    _network: &super::neural::PolicyNet,
) -> usize {
    let action = search(obs, actions, config, sampler);
    actions
        .iter()
        .position(|a| {
            a.action_type == action.action_type
                && a.parameters.as_ref().and_then(|p| p.card_id)
                    == action.parameters.as_ref().and_then(|p| p.card_id)
        })
        .unwrap_or(0)
}

fn parse_area(s: &str) -> Option<crate::zones::MemberArea> {
    match s {
        "left" => Some(crate::zones::MemberArea::LeftSide),
        "center" => Some(crate::zones::MemberArea::Center),
        "right" => Some(crate::zones::MemberArea::RightSide),
        _ => None,
    }
}
