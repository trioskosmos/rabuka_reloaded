// Multiplayer routing: receive/execute opponent actions and send local actions.

use rabuka_engine::game_setup;
use rabuka_engine::game_state::GameState;
use rabuka_engine::turn;

use crate::ffi::_3ds_debug_print;
use crate::uds;

/// Phase-aware multiplayer turn check.
/// Returns true if the given player (0=P1, 1=P2) should be able to act.
pub fn mp_can_act(gs: &GameState, player_id: i32) -> bool {
    gs.can_player_act(player_id)
}

/// Execute an action received from the opponent on this console's engine copy.
/// Both consoles run the same engine with the same seed, so executing the same
/// action in the same order keeps the two GameStates identical — no full-state
/// sync is ever needed during gameplay.
pub fn execute_received_action(gs: &mut GameState, sync: &uds::ActionSync) {
    let action_type = match sync.action_tag {
        0 => game_setup::ActionType::RockChoice,
        1 => game_setup::ActionType::PaperChoice,
        2 => game_setup::ActionType::ScissorsChoice,
        3 => game_setup::ActionType::ChooseFirstAttacker,
        4 => game_setup::ActionType::SelectMulligan,
        5 => game_setup::ActionType::SkipMulligan,
        6 => game_setup::ActionType::PlayMemberToStage,
        7 => game_setup::ActionType::SetLiveCard,
        8 => game_setup::ActionType::FinishLiveCardSet,
        9 => game_setup::ActionType::EnergyCharge,
        10 => game_setup::ActionType::ChoiceDecision,
        11 => game_setup::ActionType::ChoiceSelect,
        12 => game_setup::ActionType::ChoiceSkip,
        13 => game_setup::ActionType::ChoiceOption,
        14 => game_setup::ActionType::ChoicePosition,
        15 => game_setup::ActionType::UseAbility,
        16 => game_setup::ActionType::ChooseSecondAttacker,
        17 => game_setup::ActionType::ConfirmMulligan,
        18 => game_setup::ActionType::SelectLiveCard,
        19 => game_setup::ActionType::ConfirmLiveCardSet,
        20 => game_setup::ActionType::SkipLiveCardSet,
        21 => game_setup::ActionType::PassRemaining,
        22 => game_setup::ActionType::Pass,
        _ => game_setup::ActionType::Pass,
    };
    let stage_area = match sync.stage_area {
        1 => Some(rabuka_engine::zones::MemberArea::LeftSide),
        2 => Some(rabuka_engine::zones::MemberArea::Center),
        3 => Some(rabuka_engine::zones::MemberArea::RightSide),
        _ => None,
    };
    // RPS choices route by the acting player encoded in the wire message.
    if matches!(
        action_type,
        game_setup::ActionType::RockChoice
            | game_setup::ActionType::PaperChoice
            | game_setup::ActionType::ScissorsChoice
    ) {
        gs.pending_rps_player_id = Some(sync.player_id as i32);
    }
    let _ = turn::TurnEngine::execute_main_phase_action_with_ability_index(
        gs,
        &action_type,
        sync.card_id,
        if sync.card_indices.is_empty() {
            None
        } else {
            Some(sync.card_indices.clone())
        },
        stage_area,
        if sync.use_baton_touch {
            Some(true)
        } else {
            None
        },
        sync.ability_index.map(|x| x as usize),
    );
    gs.reset_loop_detection();
}

/// Convert an ActionType to its wire tag (matches ActionSync::from_bytes).
pub fn action_tag_of(at: &game_setup::ActionType) -> u16 {
    match at {
        game_setup::ActionType::RockChoice => 0,
        game_setup::ActionType::PaperChoice => 1,
        game_setup::ActionType::ScissorsChoice => 2,
        game_setup::ActionType::ChooseFirstAttacker => 3,
        game_setup::ActionType::ChooseSecondAttacker => 16,
        game_setup::ActionType::SelectMulligan => 4,
        game_setup::ActionType::SkipMulligan => 5,
        game_setup::ActionType::ConfirmMulligan => 17,
        game_setup::ActionType::PlayMemberToStage => 6,
        game_setup::ActionType::SetLiveCard => 7,
        game_setup::ActionType::FinishLiveCardSet => 8,
        game_setup::ActionType::EnergyCharge => 9,
        game_setup::ActionType::ChoiceDecision => 10,
        game_setup::ActionType::ChoiceSelect => 11,
        game_setup::ActionType::ChoiceSkip => 12,
        game_setup::ActionType::ChoiceOption => 13,
        game_setup::ActionType::ChoicePosition => 14,
        game_setup::ActionType::UseAbility => 15,
        game_setup::ActionType::SelectLiveCard => 18,
        game_setup::ActionType::ConfirmLiveCardSet => 19,
        game_setup::ActionType::SkipLiveCardSet => 20,
        game_setup::ActionType::PassRemaining => 21,
        game_setup::ActionType::Pass => 22,
        _ => 0,
    }
}

/// Execute a locally-chosen action. Single player: just run the engine.
/// Multiplayer: this console is the actor — execute on its own engine copy AND
/// send the action to the opponent, whose identical copy executes it too. Both
/// consoles then settle automatic phases themselves, keeping states identical.
/// Returns true (the action was executed locally).
#[allow(clippy::too_many_arguments)]
pub fn route_authoritative_action(
    gs: &mut GameState,
    action: &game_setup::Action,
    is_multiplayer: bool,
    is_host: bool,
    waiting_for_opponent: &mut bool,
    pending_client_action: &mut Option<Vec<u8>>,
    next_action_seq: &mut u32,
) -> bool {
    let p = action.parameters.clone();
    let my_id = if is_host { 0 } else { 1 };
    // RPS choices route by the acting player (this console's player id).
    if is_multiplayer
        && matches!(
            action.action_type,
            game_setup::ActionType::RockChoice
                | game_setup::ActionType::PaperChoice
                | game_setup::ActionType::ScissorsChoice
        )
    {
        gs.pending_rps_player_id = Some(my_id);
    }
    let result = turn::TurnEngine::execute_main_phase_action_with_ability_index(
        gs,
        &action.action_type,
        p.as_ref().and_then(|x| x.card_id),
        p.as_ref().and_then(|x| x.card_indices.clone()),
        p.as_ref()
            .and_then(|x| x.stage_area.as_ref().and_then(|s| s.parse().ok())),
        p.as_ref().and_then(|x| x.use_baton_touch),
        p.as_ref().and_then(|x| x.ability_index),
    );
    if let Err(ref e) = result {
        unsafe {
            _3ds_debug_print(format!("[ERR] {}\n\0", e).as_ptr());
        }
    }
    gs.reset_loop_detection();
    if is_multiplayer {
        // Send the action so the opponent's identical engine copy executes it.
        let stage_area = match p
            .as_ref()
            .and_then(|x| x.stage_area.as_ref())
            .map(|s| s.as_str())
        {
            Some("left") => 1u8,
            Some("center") => 2,
            Some("right") => 3,
            _ => 0,
        };
        let sync = uds::ActionSync {
            action_tag: action_tag_of(&action.action_type),
            card_id: p.as_ref().and_then(|x| x.card_id),
            card_indices: p
                .as_ref()
                .and_then(|x| x.card_indices.clone())
                .unwrap_or_default(),
            stage_area,
            use_baton_touch: p.as_ref().and_then(|x| x.use_baton_touch).unwrap_or(false),
            ability_index: p.as_ref().and_then(|x| x.ability_index).map(|x| x as u16),
            action_seq: *next_action_seq,
            player_id: my_id as u8,
        };
        *next_action_seq = next_action_seq.wrapping_add(1);
        let bytes = sync.to_bytes();
        *pending_client_action = Some(bytes.clone());
        let _ = uds::uds_send(&bytes);
        *waiting_for_opponent = !mp_can_act(gs, my_id);
    }
    true
}
