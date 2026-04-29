use crate::card::AbilityCost;
use crate::game_state::GameState;
use crate::player::Player;
use super::types::CostCalculation;

/// Calculate if a cost can be paid and return detailed information
pub fn calculate_cost(
    cost: &AbilityCost,
    player: &Player,
    game_state: &GameState,
) -> CostCalculation {
    match cost.cost_type.as_deref() {
        Some("move_cards") => calculate_move_cards_cost(cost, player, game_state),
        Some("pay_energy") => calculate_pay_energy_cost(cost, player, game_state),
        Some("change_state") => calculate_change_state_cost(cost, player, game_state),
        Some("choice_condition") => calculate_choice_condition_cost(cost, player, game_state),
        Some("energy_condition") => calculate_energy_condition_cost(cost, player, game_state),
        Some("reveal") => calculate_reveal_cost(cost, player, game_state),
        _ => CostCalculation {
            payable: false,
            reason: Some(format!("Unknown cost type: {:?}", cost.cost_type)),
            cost_description: cost.text.clone(),
        },
    }
}

fn calculate_move_cards_cost(
    cost: &AbilityCost,
    player: &Player,
    game_state: &GameState,
) -> CostCalculation {
    let source = cost.source.as_deref().unwrap_or("");
    let card_type = cost.card_type.as_deref().unwrap_or("");
    let count_needed = cost.count.unwrap_or(1) as usize;

    let has_card = match source {
        "stage" => {
            if card_type == "member_card" {
                let count = (player.stage.stage[0] != -1) as usize +
                               (player.stage.stage[1] != -1) as usize +
                               (player.stage.stage[2] != -1) as usize;
                count >= count_needed
            } else {
                false
            }
        }
        "hand" => {
            let card_db = &game_state.card_database;
            if card_type == "member_card" {
                player.hand.cards.iter().filter(|&id| {
                    card_db.get_card(*id).map_or(false, |c| c.is_member())
                }).count() >= count_needed
            } else if card_type == "live_card" {
                player.hand.cards.iter().filter(|&id| {
                    card_db.get_card(*id).map_or(false, |c| c.is_live())
                }).count() >= count_needed
            } else {
                player.hand.cards.len() >= count_needed
            }
        }
        "discard" => {
            let card_db = &game_state.card_database;
            if card_type == "member_card" {
                player.waitroom.cards.iter().filter(|&id| {
                    card_db.get_card(*id).map_or(false, |c| c.is_member())
                }).count() >= count_needed
            } else if card_type == "live_card" {
                player.waitroom.cards.iter().filter(|&id| {
                    card_db.get_card(*id).map_or(false, |c| c.is_live())
                }).count() >= count_needed
            } else {
                player.waitroom.cards.len() >= count_needed
            }
        }
        "deck" => {
            player.main_deck.cards.len() >= count_needed
        }
        "success_live_zone" => {
            player.success_live_card_zone.cards.len() >= count_needed
        }
        "live_card_zone" => {
            player.live_card_zone.cards.len() >= count_needed
        }
        "energy_zone" => {
            player.energy_zone.cards.len() >= count_needed
        }
        _ => false,
    };

    if has_card {
        CostCalculation {
            payable: true,
            reason: None,
            cost_description: cost.text.clone(),
        }
    } else {
        CostCalculation {
            payable: false,
            reason: Some(format!(
                "Not enough {} cards in {} (need {}, have {})",
                card_type,
                source,
                count_needed,
                match source {
                    "stage" => (player.stage.stage[0] != -1) as usize +
                                   (player.stage.stage[1] != -1) as usize +
                                   (player.stage.stage[2] != -1) as usize,
                    "hand" => player.hand.cards.len(),
                    "discard" => player.waitroom.cards.len(),
                    "deck" => player.main_deck.cards.len(),
                    "success_live_zone" => player.success_live_card_zone.cards.len(),
                    "live_card_zone" => player.live_card_zone.cards.len(),
                    "energy_zone" => player.energy_zone.cards.len(),
                    _ => 0,
                }
            )),
            cost_description: cost.text.clone(),
        }
    }
}

fn calculate_pay_energy_cost(
    cost: &AbilityCost,
    player: &Player,
    _game_state: &GameState,
) -> CostCalculation {
    let energy_needed = cost.energy.unwrap_or(1) as usize;
    let active_energy = player.count_active_energy();

    if active_energy >= energy_needed {
        CostCalculation {
            payable: true,
            reason: None,
            cost_description: cost.text.clone(),
        }
    } else {
        CostCalculation {
            payable: false,
            reason: Some(format!(
                "Need {} active energy, have {}",
                energy_needed, active_energy
            )),
            cost_description: cost.text.clone(),
        }
    }
}

fn calculate_change_state_cost(
    cost: &AbilityCost,
    player: &Player,
    _game_state: &GameState,
) -> CostCalculation {
    let state = cost.state_change.as_deref().unwrap_or("");

    match state {
        "wait" | "ウェイト" => {
            let has_active = player.stage.stage[0] != -1
                || player.stage.stage[1] != -1
                || player.stage.stage[2] != -1;

            if has_active {
                CostCalculation {
                    payable: true,
                    reason: None,
                    cost_description: cost.text.clone(),
                }
            } else {
                CostCalculation {
                    payable: false,
                    reason: Some("No card in active state to change to wait".to_string()),
                    cost_description: cost.text.clone(),
                }
            }
        }
        _ => CostCalculation {
            payable: false,
            reason: Some(format!("Unknown state: {}", state)),
            cost_description: cost.text.clone(),
        },
    }
}

fn calculate_choice_condition_cost(
    cost: &AbilityCost,
    player: &Player,
    _game_state: &GameState,
) -> CostCalculation {
    let source = cost.source.as_deref().unwrap_or("");
    let count = cost.count.unwrap_or(1) as usize;

    let has_options = match source {
        "hand" => player.hand.cards.len() >= count,
        "deck" => player.main_deck.cards.len() >= count,
        "discard" => player.waitroom.cards.len() >= count,
        "stage" => {
            (player.stage.stage[0] != -1) as usize +
            (player.stage.stage[1] != -1) as usize +
            (player.stage.stage[2] != -1) as usize >= count
        }
        _ => true,
    };

    if has_options {
        CostCalculation {
            payable: true,
            reason: None,
            cost_description: cost.text.clone(),
        }
    } else {
        CostCalculation {
            payable: false,
            reason: Some(format!("Not enough options in {} to make a choice", source)),
            cost_description: cost.text.clone(),
        }
    }
}

fn calculate_energy_condition_cost(
    cost: &AbilityCost,
    player: &Player,
    _game_state: &GameState,
) -> CostCalculation {
    let energy_needed = cost.energy.unwrap_or(1) as usize;
    let active_energy = player.count_active_energy();

    if active_energy >= energy_needed {
        CostCalculation {
            payable: true,
            reason: None,
            cost_description: cost.text.clone(),
        }
    } else {
        CostCalculation {
            payable: false,
            reason: Some(format!(
                "Energy condition not met: need {}, have {}",
                energy_needed, active_energy
            )),
            cost_description: cost.text.clone(),
        }
    }
}

fn calculate_reveal_cost(
    cost: &AbilityCost,
    player: &Player,
    _game_state: &GameState,
) -> CostCalculation {
    let source = cost.source.as_deref().unwrap_or("");
    let count = cost.count.unwrap_or(1) as usize;

    let has_cards = match source {
        "hand" => player.hand.cards.len() >= count,
        "deck" => player.main_deck.cards.len() >= count,
        _ => true,
    };

    if has_cards {
        CostCalculation {
            payable: true,
            reason: None,
            cost_description: cost.text.clone(),
        }
    } else {
        CostCalculation {
            payable: false,
            reason: Some(format!("Not enough cards in {} to reveal", source)),
            cost_description: cost.text.clone(),
        }
    }
}
