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
    let action_type = game_setup::ActionType::from_tag(sync.action_tag);
    let stage_area = rabuka_engine::zones::MemberArea::from_tag(sync.stage_area);
    // RPS choices route by the acting player encoded in the wire message.
    if matches!(
        action_type,
        game_setup::ActionType::RockChoice
            | game_setup::ActionType::PaperChoice
            | game_setup::ActionType::ScissorsChoice
    ) {
        gs.pending_rps_player_id = Some(sync.player_id);
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
    at.to_tag()
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
        let stage_area = p
            .as_ref()
            .and_then(|x| x.stage_area.as_ref())
            .and_then(|s| s.parse::<rabuka_engine::zones::MemberArea>().ok())
            .map(|m| m.to_tag())
            .unwrap_or(0);
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
        *waiting_for_opponent = !mp_can_act(gs, my_id as i32);
    }
    true
}
