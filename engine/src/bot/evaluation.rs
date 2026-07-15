use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;
use crate::zones::MemberArea;

/// Rollout policy — picks the BEST action by blade/cost ratio.
pub fn pick_rollout_action(actions: &[Action], state: &GameState) -> Action {
    if actions.len() <= 1 {
        return actions[0].clone();
    }

    let my_energy = state.player1.energy_zone.active_count() as u32;
    let may_baton = state.player1.stage.stage.iter().all(|&id| id >= 0);

    // Score each playable member by blade/cost ratio
    let mut best_member: Option<(usize, f64)> = None;
    for (i, a) in actions.iter().enumerate() {
        if a.action_type != ActionType::PlayMemberToStage {
            continue;
        }
        if let Some(ref p) = a.parameters {
            if let Some(cid) = p.card_id {
                if let Some(card) = state.card_database.get_card(cid) {
                    let cost = card.cost.unwrap_or(1);
                    if cost <= my_energy {
                        // Check if a slot is available (or if baton touch is ok)
                        if may_baton || can_play_to(&state.player1.stage.stage, p) {
                            let score = card.blade as f64 / cost as f64;
                            if best_member.map_or(true, |(_, s)| score > s) {
                                best_member = Some((i, score));
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some((idx, _)) = best_member {
        return actions[idx].clone();
    }

    // Use abilities
    for a in actions {
        if a.action_type == ActionType::UseAbility {
            return a.clone();
        }
    }

    // Play a member even if we must baton touch
    for a in actions {
        if a.action_type == ActionType::PlayMemberToStage {
            return a.clone();
        }
    }

    // Pass
    for a in actions {
        if a.action_type == ActionType::Pass {
            return a.clone();
        }
    }

    actions[0].clone()
}

fn can_play_to(stage: &[i16; 3], p: &crate::game_setup::ActionParameters) -> bool {
    if let Some(ref area) = p.stage_area {
        let idx = match area.as_str() {
            "left" => 0,
            "center" => 1,
            "right" => 2,
            _ => 0,
        };
        stage[idx] < 0
    } else {
        false
    }
}
