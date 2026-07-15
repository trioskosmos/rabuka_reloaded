use crate::game_setup::{Action, ActionType};
use crate::game_state::GameState;
use crate::zones::MemberArea;

/// Rollout policy — picks actions that actually make progress toward scoring.
/// Prioritizes: members on stage → energy → live cards → abilities → pass.
pub fn pick_rollout_action(actions: &[Action], state: &GameState) -> Action {
    if actions.is_empty() {
        return Action {
            description: "pass".into(),
            action_type: ActionType::Pass,
            parameters: None,
        };
    }
    if actions.len() == 1 {
        return actions[0].clone();
    }

    // Find a free stage slot
    let free_slot = if state.player1.stage.stage[0] < 0 {
        Some(MemberArea::LeftSide)
    } else if state.player1.stage.stage[1] < 0 {
        Some(MemberArea::Center)
    } else if state.player1.stage.stage[2] < 0 {
        Some(MemberArea::RightSide)
    } else {
        None
    };

    let my_energy = state.player1.energy_zone.active_count() as u32;

    // Priority 1: Play member cards to fill empty stage slots
    if free_slot.is_some() {
        for a in actions {
            if a.action_type == ActionType::PlayMemberToStage {
                if let Some(ref p) = a.parameters {
                    if let Some(cid) = p.card_id {
                        if let Some(card) = state.card_database.get_card(cid) {
                            let cost = card.cost.unwrap_or(1);
                            if cost <= my_energy {
                                return a.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    // Priority 2: Set live cards (always good)
    for a in actions {
        if a.action_type == ActionType::SetLiveCard
            || a.action_type == ActionType::ConfirmLiveCardSet
            || a.action_type == ActionType::SelectLiveCard
        {
            return a.clone();
        }
    }

    // Priority 3: Use abilities (some are free, some help)
    for a in actions {
        if a.action_type == ActionType::UseAbility {
            return a.clone();
        }
    }

    // Priority 4: Play a member even to a full stage (baton touch)
    for a in actions {
        if a.action_type == ActionType::PlayMemberToStage {
            if let Some(ref p) = a.parameters {
                if let Some(cid) = p.card_id {
                    if let Some(card) = state.card_database.get_card(cid) {
                        let cost = card.cost.unwrap_or(1);
                        if cost <= my_energy {
                            return a.clone();
                        }
                    }
                }
            }
        }
    }

    // Priority 5: Pass
    for a in actions {
        if a.action_type == ActionType::Pass {
            return a.clone();
        }
    }

    actions[super::ismcts::fastrand(0, actions.len())].clone()
}
