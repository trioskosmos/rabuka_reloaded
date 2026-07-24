use crate::game_setup::{self, Action};
use crate::game_state::GameResult;
use crate::turn::TurnEngine;

use super::determinization::DeterminizationSampler;
use super::evaluation::pick_rollout_action;
use super::observation::PublicObservation;

const MAX_ROLLOUT_DEPTH: u32 = 300;

/// Flat Monte Carlo: for each action, N independent rollouts, score = avg(success_delta / turn_delta).
pub fn search(
    obs: &PublicObservation,
    actions: &[Action],
    config: &super::BotConfig,
    sampler: &DeterminizationSampler,
) -> Action {
    if actions.is_empty() {
        return Action {
            description: "pass".into(),
            action_type: crate::game_setup::ActionType::Pass,
            parameters: None,
        };
    }
    if actions.len() == 1 {
        return actions[0].clone();
    }

    let start_success = obs.me.success_zone.len() as f64 - obs.opp.success_zone.len() as f64;
    let n_actions = actions.len();
    let per_action = (config.iterations / n_actions as u32).max(5);

    let mut scores = vec![0.0f64; n_actions];
    let mut counts = vec![0u32; n_actions];

    for (i, action) in actions.iter().enumerate() {
        for _ in 0..per_action {
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

            let end_s = state.player1.success_live_card_zone.cards.len() as f64
                - state.player2.success_live_card_zone.cards.len() as f64;
            let turns = (state.turn_number - obs.turn_number).max(1) as f64;
            scores[i] += (end_s - start_success) / turns;
            counts[i] += 1;
        }
    }

    let mut best_idx = 0usize;
    let mut best_avg = f64::NEG_INFINITY;
    for (i, &c) in counts.iter().enumerate() {
        let avg = scores[i] / c as f64;
        if avg > best_avg {
            best_avg = avg;
            best_idx = i;
        }
    }

    actions[best_idx].clone()
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
