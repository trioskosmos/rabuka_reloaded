use crate::game_setup::Action;
use crate::game_state::GameState;

/// Pick rollout action by success_delta / turns (clone-and-eval each action).
pub fn pick_rollout_action(actions: &[Action], state: &GameState) -> Action {
    if actions.len() <= 1 {
        return actions[0].clone();
    }

    let base_s = state.player1.success_live_card_zone.cards.len() as f64
        - state.player2.success_live_card_zone.cards.len() as f64;
    let base_t = state.turn_number;

    let mut best_idx = 0usize;
    let mut best_score = f64::NEG_INFINITY;

    for (i, a) in actions.iter().enumerate() {
        let mut sim = state.clone();
        let p = a.parameters.clone();
        let _ = crate::turn::TurnEngine::execute_main_phase_action(
            &mut sim,
            &a.action_type,
            p.as_ref().and_then(|p| p.card_id),
            p.as_ref().and_then(|p| p.card_indices.clone()),
            p.as_ref().and_then(|p| {
                p.stage_area.as_deref().and_then(|s| match s {
                    "left" => Some(crate::zones::MemberArea::LeftSide),
                    "center" => Some(crate::zones::MemberArea::Center),
                    "right" => Some(crate::zones::MemberArea::RightSide),
                    _ => None,
                })
            }),
            p.as_ref().and_then(|p| p.use_baton_touch),
        );
        crate::game_setup::settle_single_player_state(&mut sim);

        let s = sim.player1.success_live_card_zone.cards.len() as f64
            - sim.player2.success_live_card_zone.cards.len() as f64;
        let t = (sim.turn_number - base_t).max(1) as f64;
        let score = (s - base_s) / t;
        if score > best_score {
            best_score = score;
            best_idx = i;
        }
    }
    actions[best_idx].clone()
}
