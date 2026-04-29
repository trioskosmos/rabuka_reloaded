use crate::card::Condition;
use crate::game_state::GameState;
use crate::player::Player;

/// Evaluate if a condition is met
pub fn evaluate_condition(
    condition: &Condition,
    player: &Player,
    game_state: &GameState,
) -> bool {
    match condition.condition_type.as_deref() {
        Some("location_condition") => evaluate_location_condition(condition, player, game_state),
        Some("count_condition") => evaluate_count_condition(condition, player, game_state),
        Some("character_presence_condition") => evaluate_character_presence(condition, player, game_state),
        Some("group_presence_condition") => evaluate_group_presence(condition, player, game_state),
        Some("energy_state_condition") => evaluate_energy_state(condition, player),
        _ => true,
    }
}

fn evaluate_location_condition(
    condition: &Condition,
    player: &Player,
    game_state: &GameState,
) -> bool {
    let location = condition.location.as_deref().unwrap_or("");
    let card_type = condition.card_type.as_deref().unwrap_or("");

    match location {
        "stage" => {
            if card_type == "member_card" {
                player.stage.stage[0] != -1
                    || player.stage.stage[1] != -1
                    || player.stage.stage[2] != -1
            } else {
                false
            }
        }
        "hand" => {
            let card_db = &game_state.card_database;
            if card_type == "member_card" {
                player.hand.cards.iter().map(|&id| {
                    card_db.get_card(id).map_or(false, |c| c.is_member())
                }).any(|x| x)
            } else if card_type == "live_card" {
                player.hand.cards.iter().any(|&id| {
                    card_db.get_card(id).map_or(false, |c| c.is_live())
                })
            } else {
                !player.hand.is_empty()
            }
        }
        _ => false,
    }
}

fn evaluate_count_condition(
    condition: &Condition,
    player: &Player,
    _game_state: &GameState,
) -> bool {
    let location = condition.location.as_deref().unwrap_or("");
    let count = condition.count.unwrap_or(0) as usize;
    let operator = condition.operator.as_deref().unwrap_or(">=");

    let actual_count = player.count_cards_in_zone(location);

    match operator {
        ">=" => actual_count >= count,
        "<=" => actual_count <= count,
        "==" => actual_count == count,
        ">" => actual_count > count,
        "<" => actual_count < count,
        _ => false,
    }
}

fn evaluate_character_presence(condition: &Condition, player: &Player, game_state: &GameState) -> bool {
    if let Some(ref characters) = condition.characters {
        if characters.is_empty() {
            return true;
        }
        characters.iter().any(|name| player.has_character_on_stage(name, &game_state.card_database))
    } else {
        true
    }
}

fn evaluate_group_presence(condition: &Condition, player: &Player, game_state: &GameState) -> bool {
    if let Some(ref group) = condition.group {
        let group_str = group.as_str().unwrap_or("");
        player.has_group_on_stage(group_str, &game_state.card_database)
    } else if let Some(ref group_names) = condition.group_names {
        if group_names.is_empty() {
            return true;
        }
        group_names.iter().any(|name| player.has_group_on_stage(name, &game_state.card_database))
    } else {
        true
    }
}

fn evaluate_energy_state(condition: &Condition, player: &Player) -> bool {
    let state = condition.energy_state.as_deref().unwrap_or("");

    match state {
        "active" => player.count_active_energy() > 0,
        "wait" => player.count_wait_energy() > 0,
        _ => false,
    }
}
