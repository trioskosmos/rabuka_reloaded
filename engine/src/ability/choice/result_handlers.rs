use crate::ability::resolver::AbilityResolver;
use crate::ability::types::{Choice, ChoiceResult, ChoiceRoute, ExecutionContext};
use crate::core::constants::CountCast;
use crate::game_state::GameState;

#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};

fn handle_select_card(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    choice: &Choice,
    result: &ChoiceResult,
    context: ExecutionContext,
) -> Result<(), String> {
    let Choice::SelectCard {
        zone,
        card_type,
        count,
        allow_skip,
        cost_limit,
        cost_limit_operator,
        cost_total,
        cost_total_operator,
        group,
        characters,
        filtered_indices,
        is_select_action,
        target_player_id,
        blind,
        destination,
        discard_remaining,
        ..
    } = choice
    else {
        return Err("Choice result does not match pending choice".to_string());
    };
    let ChoiceResult::CardSelected { indices } = result else {
        return Err("Choice result does not match pending choice".to_string());
    };
    resolver.handle_select_card(
        gs,
        choice,
        zone,
        context,
        crate::ability::choice::SelectionContext {
            card_type: card_type.clone(),
            count: *count,
            allow_skip: *allow_skip,
            indices: indices.to_vec(),
            cost_limit: *cost_limit,
            cost_limit_operator: cost_limit_operator.clone(),
            cost_total: *cost_total,
            cost_total_operator: cost_total_operator.clone(),
            group: group.clone(),
            characters: characters.clone(),
            filtered_indices: filtered_indices.clone(),
            is_select_action: *is_select_action,
            target_player_id: target_player_id.clone(),
            destination: destination.clone(),
            discard_remaining: *discard_remaining,
            blind: *blind,
            is_reveal: matches!(choice, Choice::SelectCard { is_reveal: true, .. }),
        },
    )
}

fn handle_any_number_skip(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
) -> Result<(), String> {
    resolver.clear_choice_state_and_resume(gs)
}

fn handle_order_skip() -> Result<(), String> {
    Err("Deck ordering cannot be skipped".to_string())
}

fn handle_general_skip(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    context: ExecutionContext,
) -> Result<(), String> {
    gs.ability_queue.take_pending_actions();
    resolver.clear_choice_state(gs);
    resolver.resume_execution(gs, context)
}

fn handle_area_select(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    choice: &Choice,
    result: &ChoiceResult,
) -> Result<(), String> {
    let ChoiceResult::TargetSelected { target: selected } = result else {
        return Err("Choice result does not match pending choice".to_string());
    };
    let Choice::SelectTarget { options: Some(opts), .. } = choice else {
        resolver.selection.area = Some(selected.clone());
        resolver.clear_choice_state(gs);
        return resolver.resume_pending_actions(gs);
    };
    let area = if let Ok(idx) = selected.parse::<usize>() {
        opts.get(idx).map(|s| s.as_str()).unwrap_or("left").to_string()
    } else if opts.contains(selected) {
        selected.clone()
    } else {
        opts.first().cloned().unwrap_or_else(|| "left".to_string())
    };
    resolver.selection.area = Some(area);
    resolver.clear_choice_state(gs);
    resolver.resume_pending_actions(gs)
}

fn handle_select_target(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    choice: &Choice,
    result: &ChoiceResult,
) -> Result<(), String> {
    let Choice::SelectTarget { target, .. } = choice else {
        return Err("Choice result does not match pending choice".to_string());
    };
    let ChoiceResult::TargetSelected { target: selected } = result else {
        return Err("Choice result does not match pending choice".to_string());
    };
    if target == "choice_string" {
        let conditional_choice = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.conditional_choice.clone());
        let choice_card_no = gs
            .ability_queue
            .current_entry()
            .and_then(|e| e.choice_card_no.clone());
        if matches!(choice_card_no, Some(ChoiceRoute::ChoiceString)) {
            crate::ability::compound::conditional_on::handle_choice_string_selection(
                resolver,
                gs,
                selected,
                conditional_choice,
            )
        } else {
            crate::ability::compound::conditional_on::handle_choice_string_store(
                resolver,
                gs,
                selected,
                conditional_choice,
            )
        }
    } else {
        resolver.handle_select_target(gs, target, selected)
    }
}

fn handle_select_position(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    result: &ChoiceResult,
    context: ExecutionContext,
) -> Result<(), String> {
    let ChoiceResult::PositionSelected { position } = result else {
        return Err("Choice result does not match pending choice".to_string());
    };
    resolver.handle_select_position(gs, position, context)
}

fn handle_heart_color(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    choice: &Choice,
    result: &ChoiceResult,
) -> Result<(), String> {
    let ChoiceResult::HeartColorSelected { colors } = result else {
        return Err("Choice result does not match pending choice".to_string());
    };
    let Choice::SelectHeartColor { count, .. } = choice else {
        return Err("Choice result does not match pending choice".to_string());
    };
        resolver.handle_heart_selection(gs, count.u8_count(), colors)
}

fn handle_heart_type(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    choice: &Choice,
    result: &ChoiceResult,
) -> Result<(), String> {
    let ChoiceResult::HeartTypeSelected { types } = result else {
        return Err("Choice result does not match pending choice".to_string());
    };
    let Choice::SelectHeartType { count, .. } = choice else {
        return Err("Choice result does not match pending choice".to_string());
    };
        resolver.handle_heart_selection(gs, count.u8_count(), types)
}

pub fn dispatch_choice_result(
    resolver: &mut AbilityResolver,
    gs: &mut GameState,
    choice: &Choice,
    result: &ChoiceResult,
    context: ExecutionContext,
) -> Result<(), String> {
    match (choice, result) {
        (
            Choice::SelectCard {
                count: 0,
                allow_skip: true,
                ..
            },
            ChoiceResult::Skip,
        ) => handle_any_number_skip(resolver, gs),
        (
            Choice::SelectTarget {
                target,
                allow_skip: false,
                ..
            },
            ChoiceResult::Skip,
        ) if target == "order" => handle_order_skip(),
        (Choice::SelectCard { .. }, ChoiceResult::CardSelected { .. }) => {
            handle_select_card(resolver, gs, choice, result, context)
        }
        (Choice::SelectCard { .. } | Choice::SelectTarget { .. }, ChoiceResult::Skip) => {
            handle_general_skip(resolver, gs, context)
        }
        (Choice::SelectTarget { target, .. }, ChoiceResult::TargetSelected { .. })
            if target == "area_select" =>
        {
            handle_area_select(resolver, gs, choice, result)
        }
        (Choice::SelectTarget { .. }, ChoiceResult::TargetSelected { .. }) => {
            handle_select_target(resolver, gs, choice, result)
        }
        (Choice::SelectPosition { .. }, ChoiceResult::PositionSelected { .. }) => {
            handle_select_position(resolver, gs, result, context)
        }
        (Choice::SelectHeartColor { .. }, ChoiceResult::HeartColorSelected { .. }) => {
            handle_heart_color(resolver, gs, choice, result)
        }
        (Choice::SelectHeartType { .. }, ChoiceResult::HeartTypeSelected { .. }) => {
            handle_heart_type(resolver, gs, choice, result)
        }
        // SelectAutoAbility and SelectLiveSuccess used to have their own arm
        // here with a body identical to this one, so it was dead the moment it
        // was written.
        _ => Err("Choice result does not match pending choice".to_string()),
    }
}
