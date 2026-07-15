use rand::Rng;

use crate::game_setup::Action;
use crate::game_state::GameState;

use super::determinization::DeterminizationSampler;
use super::neural::ValueNetwork;
use super::observation::PublicObservation;
use super::BotConfig;

pub fn search(
    obs: &PublicObservation,
    actions: &[Action],
    config: &BotConfig,
    sampler: &DeterminizationSampler,
    network: &ValueNetwork,
) -> usize {
    let n_actions = actions.len();
    if n_actions == 0 {
        return 0;
    }
    if n_actions == 1 {
        return 0;
    }

    let mut root = InfoSetNode::new(n_actions);
    let c = config.exploration_constant;
    let pw_k = config.progressive_widening_k;

    let capacity = root.children.capacity();

    for _iter in 0..config.iterations {
        let mut gs = sampler.sample(obs);
        let mut path: Vec<usize> = Vec::new();

        {
            let mut cur_untried = &root.untried[..];
            let mut cur_children: &[(usize, EdgeStats)] = &root.children;
            let mut cur_visits = root.total_visits;

            loop {
                if !cur_untried.is_empty() {
                    let idx = fastrand(0, cur_untried.len());
                    let action_idx = cur_untried[idx];
                    path.push(action_idx);
                    gs = clone_and_settle(&gs, &actions[action_idx]);
                    break;
                }

                if cur_children.is_empty() {
                    break;
                }

                let max_kids = (pw_k * (cur_visits as f64).sqrt()).ceil() as usize;
                if cur_children.len() < max_kids && cur_children.len() < capacity {
                    let remaining = capacity - cur_children.len();
                    let available = cur_children.len().min(remaining);
                    if available > 0 {
                        let idx = fastrand(0, available);
                        let action_idx = idx;
                        path.push(action_idx);
                        gs = clone_and_settle(&gs, &actions[action_idx]);
                        break;
                    }
                }

                let mut best_score = f64::NEG_INFINITY;
                let mut best_action_idx = 0;
                for &(a_idx, ref stats) in cur_children {
                    let score = ucb_value(stats.visit_count, cur_visits, stats.total_reward, c);
                    if score > best_score {
                        best_score = score;
                        best_action_idx = a_idx;
                    }
                }

                path.push(best_action_idx);
                gs = clone_and_settle(&gs, &actions[best_action_idx]);

                if gs.game_result != crate::game_state::GameResult::Ongoing {
                    break;
                }

                cur_untried = &[];
                cur_children = &[];
                cur_visits = 0;
            }
        }

        let result = rollout(&gs, config, network);

        for &action_idx in &path {
            let mut found = false;
            for child in root.children.iter_mut() {
                if child.0 == action_idx {
                    child.1.visit_count += 1;
                    child.1.total_reward += result;
                    found = true;
                    break;
                }
            }
            if !found {
                root.children.push((
                    action_idx,
                    EdgeStats {
                        total_reward: result,
                        visit_count: 1,
                    },
                ));
                if let Some(pos) = root.untried.iter().position(|&x| x == action_idx) {
                    root.untried.swap_remove(pos);
                }
            }
            root.total_visits += 1;
        }
    }

    root.best_action()
}

struct EdgeStats {
    total_reward: f64,
    visit_count: u32,
}

struct InfoSetNode {
    total_visits: u32,
    children: Vec<(usize, EdgeStats)>,
    untried: Vec<usize>,
}

impl InfoSetNode {
    fn new(n_actions: usize) -> Self {
        Self {
            total_visits: 0,
            children: Vec::with_capacity(n_actions),
            untried: (0..n_actions).collect(),
        }
    }

    fn best_action(&self) -> usize {
        let mut best_idx = 0;
        let mut best_count = 0u32;
        for &(a_idx, ref stats) in &self.children {
            if stats.visit_count > best_count {
                best_count = stats.visit_count;
                best_idx = a_idx;
            }
        }
        best_idx
    }
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

fn rollout(state: &GameState, config: &BotConfig, network: &ValueNetwork) -> f64 {
    let mut gs = state.clone();
    let mut steps = 0u32;
    let max_steps = config.rollout_depth.min(200).max(50);
    loop {
        steps += 1;
        if steps > max_steps {
            return heuristic_eval(&gs, network);
        }
        crate::turn::TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != crate::game_state::GameResult::Ongoing {
            return match gs.game_result {
                crate::game_state::GameResult::Draw => 0.0,
                crate::game_state::GameResult::FirstAttackerWins => 1.0,
                crate::game_state::GameResult::SecondAttackerWins => -1.0,
                _ => 0.0,
            };
        }

        let actions = crate::game_setup::generate_possible_actions(&gs);
        if actions.is_empty() {
            crate::turn::TurnEngine::advance_phase(&mut gs);
            continue;
        }

        let action = if config.use_heuristic_rollout {
            super::evaluation::pick_rollout_action(&actions, &gs)
        } else {
            let idx = fastrand(0, actions.len());
            actions[idx].clone()
        };
        gs = clone_and_settle(&gs, &action);
    }
}

fn heuristic_eval(state: &GameState, network: &ValueNetwork) -> f64 {
    let p1_success = state.player1.success_live_card_zone.cards.len() as f64;
    let p2_success = state.player2.success_live_card_zone.cards.len() as f64;
    if p1_success > 0.0 || p2_success > 0.0 {
        return ((p1_success - p2_success) / 3.0).clamp(-1.0, 1.0);
    }
    let my_cards: Vec<i16> = state
        .player1
        .stage
        .stage
        .iter()
        .chain(state.player1.hand.cards.iter())
        .copied()
        .collect();
    let opp_cards: Vec<i16> = state
        .player2
        .stage
        .stage
        .iter()
        .chain(state.player2.hand.cards.iter())
        .copied()
        .collect();
    network.predict(&my_cards, &opp_cards) as f64
}

fn ucb_value(n: u32, total: u32, reward: f64, c: f64) -> f64 {
    if n == 0 {
        return f64::INFINITY;
    }
    let exploit = reward / n as f64;
    let explore = c * ((total as f64).ln() / n as f64).sqrt();
    exploit + explore
}

pub(super) fn fastrand(lo: usize, hi: usize) -> usize {
    if hi <= lo {
        return lo;
    }
    rand::thread_rng().gen_range(lo..hi)
}
