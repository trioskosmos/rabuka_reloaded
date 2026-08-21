use crate::game_setup::Action;
use crate::game_state::GameState;

use super::strategy;

/// Rollout policy: one-ply greedy heuristic choice (see `strategy.rs`).
/// In determinized rollout states, player1 is always the bot's perspective.
pub fn pick_rollout_action(actions: &[Action], state: &GameState) -> Action {
    if actions.len() <= 1 {
        return actions[0].clone();
    }
    strategy::choose_action_heuristic(state, actions, 0)
}
