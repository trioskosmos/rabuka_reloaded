use actix_cors::Cors;
use actix_files as fs;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::card::CardDatabase;
use crate::card_loader;
use crate::deck_builder;
use crate::deck_parser;
use crate::display;
use crate::game_setup::{ActionParameters, ActionType};
use crate::game_state::GameState;
use crate::player::Player;

pub use crate::display::{CardDisplay, GameStateDisplay, PlayerDisplay, StageDisplay, ZoneDisplay};

// ====================================================================
// Frame snapshot — lightweight board state using card IDs only
// (no full CardDisplay objects — the card database has all names/data)
// ====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameSnapshot {
    pub frame: u64,
    pub turn: u32,
    pub phase: String,
    pub active_player: String,
    pub label: String,
    pub p1: FramePlayerState,
    pub p2: FramePlayerState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramePlayerState {
    pub hand: Vec<i16>,
    pub hand_count: usize,
    pub energy_count: usize,
    pub deck_count: usize,
    pub discard: Vec<i16>,
    pub stage: [i16; 3],
    pub stage_under: [Vec<i16>; 3],
    pub live_zone: Vec<i16>,
    pub success_live_zone: Vec<i16>,
}

impl FrameSnapshot {
    pub fn capture(game_state: &crate::game_state::GameState, frame: u64, label: String) -> Self {
        let p = |player: &crate::player::Player| FramePlayerState {
            hand: player.hand.cards.iter().copied().collect(),
            hand_count: player.hand.cards.len(),
            energy_count: player.energy_zone.active_count(),
            deck_count: player.main_deck.cards.len(),
            discard: player.waitroom.cards.iter().copied().collect(),
            stage: player.stage.stage,
            stage_under: [
                player.stage.under_cards[0].iter().copied().collect(),
                player.stage.under_cards[1].iter().copied().collect(),
                player.stage.under_cards[2].iter().copied().collect(),
            ],
            live_zone: player.live_card_zone.cards.iter().copied().collect(),
            success_live_zone: player
                .success_live_card_zone
                .cards
                .iter()
                .copied()
                .collect(),
        };
        FrameSnapshot {
            frame,
            turn: game_state.turn_number,
            phase: format!("{:?}", game_state.current_phase),
            active_player: game_state.active_player().id.clone(),
            label,
            p1: p(&game_state.player1),
            p2: p(&game_state.player2),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActionIndex {
    pub description: String,
    pub action_type: String,
    pub parameters: Option<ActionParameters>,
    pub index: usize,
}

#[derive(Serialize)]
struct GameStateResponse {
    #[serde(flatten)]
    game_state: display::GameStateDisplay,
    #[serde(skip_serializing_if = "Option::is_none")]
    legal_actions: Option<Vec<ActionIndex>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ui_config: Option<UiConfig>,
}

#[derive(Deserialize)]

pub struct ExecuteActionRequest {
    pub action_index: usize,

    pub stage_area: Option<String>, // Accept string from webapp, will parse to MemberArea

    pub action_type: Option<String>,

    pub card_id: Option<i16>, // Database card ID - reliable identifier

    pub card_index: Option<usize>, // Array position - kept for backward compatibility

    pub card_indices: Option<Vec<usize>>,

    pub card_no: Option<String>,

    pub use_baton_touch: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone)]

pub struct RoomSession {
    pub session_id: String,

    pub player_id: i32,

    pub username: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]

pub struct Room {
    pub room_id: String,

    pub mode: String, // "sandbox" or "pvp"

    pub public: bool,

    pub created_at: u64,

    pub last_active: u64,

    pub sessions: HashMap<String, RoomSession>, // session_id -> session

    pub usernames: HashMap<i32, String>, // player_id -> username

    pub custom_decks: Option<HashMap<i32, CustomDeck>>,

    #[serde(skip)]
    #[allow(dead_code)]
    pub game_state: Option<Arc<RwLock<GameState>>>, // Per-room game state

    // Per-room state (completely isolated from other rooms)
    #[serde(skip)]
    pub history: Vec<GameState>,
    #[serde(skip)]
    pub future: Vec<GameState>,
    #[serde(skip)]
    pub frame_counter: u64,
    #[serde(skip)]
    pub frame_history: Vec<FrameSnapshot>,
    #[serde(skip)]
    pub cached_actions: Vec<ActionIndex>,
    #[serde(skip)]
    pub actions_dirty: bool,
}

#[derive(Serialize, Deserialize, Clone)]

pub struct CustomDeck {
    pub main: Vec<String>,

    pub energy: Vec<String>,
}

#[derive(Deserialize)]

pub struct CreateRoomRequest {
    pub mode: Option<String>,

    pub public: Option<bool>,

    pub username: Option<String>,

    pub p0_deck: Option<Vec<String>>,

    pub p0_energy: Option<Vec<String>>,

    pub p1_deck: Option<Vec<String>>,

    pub p1_energy: Option<Vec<String>>,

    pub is_ai: Option<bool>,
}

#[derive(Deserialize)]

pub struct JoinRoomRequest {
    pub room_id: String,

    pub username: Option<String>,
}

#[derive(Deserialize)]
pub struct SetUiConfigRequest {
    pub current_lang: Option<String>,
    pub perspective_player: Option<i32>,
    pub selected_turn: Option<i32>,
    pub selected_perf_turn: Option<i32>,
    pub show_friendly_abilities: Option<bool>,
    pub hotseat_mode: Option<bool>,
    pub replay_mode: Option<bool>,
}

#[derive(Deserialize)]

pub struct InitGameRequest {
    pub deck: Option<String>,
}

#[derive(Deserialize)]

pub struct ExecCodeRequest {
    pub code: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UiConfig {
    pub current_lang: String,          // "jp" or "en"
    pub perspective_player: i32,       // 0 or 1
    pub selected_turn: i32,            // -1 means all
    pub selected_perf_turn: i32,       // -1 means latest
    pub show_friendly_abilities: bool, // Display friendly ability names
    pub hotseat_mode: bool,
    pub replay_mode: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            current_lang: "jp".to_string(),
            perspective_player: 0,
            selected_turn: -1,
            selected_perf_turn: -1,
            show_friendly_abilities: false,
            hotseat_mode: false,
            replay_mode: false,
        }
    }
}

pub struct AppState {
    pub game_state: Arc<RwLock<GameState>>,

    pub rooms: Arc<Mutex<HashMap<String, Room>>>,

    pub ui_config: Arc<Mutex<UiConfig>>,

    pub card_database: Arc<CardDatabase>,

    pub deck_lists: Arc<Vec<deck_parser::DeckList>>,

    pub card_registry: Arc<serde_json::Value>,

    pub history: Arc<Mutex<Vec<GameState>>>,

    pub future: Arc<Mutex<Vec<GameState>>>,

    pub custom_decks: Arc<Mutex<HashMap<i32, Vec<String>>>>,

    pub custom_energy_decks: Arc<Mutex<HashMap<i32, Vec<String>>>>,

    pub frame_counter: Arc<Mutex<u64>>,

    pub frame_history: Arc<Mutex<Vec<FrameSnapshot>>>,

    pub cached_actions: Arc<Mutex<Vec<ActionIndex>>>,

    pub actions_dirty: Arc<Mutex<bool>>,

    pub room_broadcasts: Arc<Mutex<HashMap<String, tokio::sync::broadcast::Sender<()>>>>,
}

fn get_room_id_from_req(req: &actix_web::HttpRequest) -> Option<String> {
    req.headers()
        .get("X-Room-Id")
        .or_else(|| req.headers().get("x-room-id"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_uppercase())
}

fn get_session_token_from_req(req: &actix_web::HttpRequest) -> Option<String> {
    req.headers()
        .get("X-Session-Token")
        .or_else(|| req.headers().get("x-session-token"))
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn resolve_game_state_arc(data: &AppState, req: &actix_web::HttpRequest) -> Arc<RwLock<GameState>> {
    if let Some(room_id) = get_room_id_from_req(req) {
        let rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get(&room_id) {
            if let Some(ref gs) = room.game_state {
                return gs.clone();
            }
        }
    }
    // No X-Room-Id header or room not found: return global state.
    // Per-room requests must supply X-Room-Id to get a room-specific state.
    data.game_state.clone()
}

macro_rules! lock_state {
    ($arc:expr, $mode:ident) => {{
        match ($arc).$mode() {
            Ok(guard) => guard,
            Err(e) => {
                log::debug!("Lock poisoned: {}", e);
                return HttpResponse::InternalServerError().json("Internal error");
            }
        }
    }};
}

fn invalidate_actions(data: &AppState, room_id: Option<&str>) {
    if let Some(rid) = room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                room.actions_dirty = true;
            }
        }
    } else if let Ok(mut dirty) = data.actions_dirty.lock() {
        *dirty = true;
    }
}

fn read_actions(data: &AppState, room_id: Option<&str>) -> Vec<ActionIndex> {
    if let Some(rid) = room_id {
        if let Ok(rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get(rid) {
                return room.cached_actions.clone();
            }
        }
    }
    // Fallback to global cached actions
    if let Ok(cache) = data.cached_actions.lock() {
        return cache.clone();
    }
    Vec::new()
}

fn ensure_actions(data: &AppState, game_state: &GameState, room_id: Option<&str>) {
    if let Some(rid) = room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                if room.actions_dirty {
                    room.actions_dirty = false;
                    room.cached_actions = crate::game_setup::generate_possible_actions(game_state)
                        .into_iter()
                        .enumerate()
                        .map(|(i, a)| ActionIndex {
                            description: a.description,
                            action_type: a.action_type.to_string(),
                            parameters: a.parameters,
                            index: i,
                        })
                        .collect::<Vec<_>>();
                }
                return;
            }
        }
    }
    // Fallback to global cached actions
    if let Ok(mut dirty) = data.actions_dirty.lock() {
        if *dirty {
            *dirty = false;
            if let Ok(mut cache) = data.cached_actions.lock() {
                *cache = crate::game_setup::generate_possible_actions(game_state)
                    .into_iter()
                    .enumerate()
                    .map(|(i, a)| ActionIndex {
                        description: a.description,
                        action_type: a.action_type.to_string(),
                        parameters: a.parameters,
                        index: i,
                    })
                    .collect::<Vec<_>>();
            }
        }
    }
}

/// Lightweight version endpoint for polling — returns a sequence number
/// that increments on each state change. Clients should only fetch the full
/// /api/game-state when this version differs from their last known value.
async fn get_game_state_version(
    data: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let room_id = get_room_id_from_req(&req);
    let version = if let Some(ref rid) = room_id {
        let rooms = data.rooms.lock().unwrap();
        rooms.get(rid).map(|r| r.frame_counter).unwrap_or(0)
    } else {
        *data.frame_counter.lock().unwrap()
    };
    HttpResponse::Ok().json(serde_json::json!({ "version": version }))
}

pub async fn get_game_state(
    data: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let room_id_str = get_room_id_from_req(&req);
    let session_token = get_session_token_from_req(&req);

    // If room exists but game_state is not yet initialized, return a waiting response
    if let Some(ref rid) = room_id_str {
        let rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get(rid) {
            if room.game_state.is_none() {
                let players_ready = room.custom_decks.as_ref().map(|d| d.len()).unwrap_or(0);
                return HttpResponse::Ok().json(serde_json::json!({
                    "room_not_ready": true,
                    "room_id": rid,
                    "mode": room.mode,
                    "players_ready": players_ready,
                }));
            }
        } else {
            // Room was destroyed (opponent left or game ended)
            return HttpResponse::Ok().json(serde_json::json!({
                "room_closed": true,
                "room_id": rid,
            }));
        }
    }

    let gs_arc = resolve_game_state_arc(&data, &req);
    let game_state = lock_state!(gs_arc, read);
    ensure_actions(&data, &game_state, room_id_str.as_deref());
    let mut display = crate::display::game_state_to_display(&game_state);
    let actions = read_actions(&data, room_id_str.as_deref());
    drop(game_state);

    let mut requester_player_id = None;
    if let Some(ref rid) = room_id_str {
        let rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get(rid) {
            display.mode = room.mode.clone();
            if room.mode == "pvp" || room.mode == "pve" {
                requester_player_id = session_token
                    .as_ref()
                    .and_then(|token| room.sessions.get(token))
                    .map(|sess| sess.player_id);

                if let Some(pid) = requester_player_id {
                    let gs = lock_state!(gs_arc, read);
                    filter_display_for_player(&mut display, &gs, pid);
                    drop(gs);
                } else {
                    // PVP room but no session — treat as spectator, block all actions
                    display.waiting_for_opponent = true;
                }
            }
        }
    }
    if let Some(pid) = requester_player_id {
        let gs = lock_state!(gs_arc, read);
        if !pvp_player_can_act(&gs, pid) {
            display.waiting_for_opponent = true;
        }
        drop(gs);
    }

    let ui_config = data.ui_config.lock().unwrap().clone();
    let final_actions = if display.waiting_for_opponent {
        None
    } else {
        Some(actions)
    };
    HttpResponse::Ok().json(GameStateResponse {
        game_state: display,
        legal_actions: final_actions,
        ui_config: Some(ui_config),
    })
}

async fn get_actions(data: web::Data<AppState>, req: actix_web::HttpRequest) -> impl Responder {
    let room_id_str = get_room_id_from_req(&req);
    let gs_arc = resolve_game_state_arc(&data, &req);
    let game_state = lock_state!(gs_arc, read);
    ensure_actions(&data, &game_state, room_id_str.as_deref());
    let actions = read_actions(&data, room_id_str.as_deref());
    drop(game_state);
    HttpResponse::Ok().json(serde_json::json!({ "actions": actions }))
}

fn actions_with_index(game_state: &GameState) -> Vec<ActionIndex> {
    crate::game_setup::generate_possible_actions(game_state)
        .into_iter()
        .enumerate()
        .map(|(i, a)| ActionIndex {
            description: a.description,
            action_type: a.action_type.to_string(),
            parameters: a.parameters,
            index: i,
        })
        .collect()
}

// Thin wrappers — canonical implementations live in game_setup so all
// callers (web_server, harness, rabuka_3ds) stay in sync.
fn is_automatic_phase(game_state: &GameState) -> bool {
    crate::game_setup::is_automatic_phase(game_state)
}

fn is_live_card_set_phase(game_state: &GameState) -> bool {
    crate::game_setup::is_live_card_set_phase(game_state)
}

fn settle_single_player_state(game_state: &mut GameState) -> Result<(), String> {
    crate::game_setup::settle_single_player_state(game_state);
    Ok(())
}

/// For PVP: determine if `player_id` (0 or 1) is allowed to act in the current game state.
/// Returns `true` if the player can submit actions, `false` if they should wait.
pub fn pvp_player_can_act(game_state: &GameState, player_id: i32) -> bool {
    use crate::game_state::Phase;

    // Pending choices with routing info override phase-based defaults.
    let pid_str = || -> &'static str {
        if player_id == 0 {
            "p1"
        } else {
            "p2"
        }
    };
    if game_state.has_pending_choice() {
        if let Some(cpid) = game_state.get_pending_choice_player_id() {
            return cpid == pid_str();
        }
        if let Some(choice) = game_state.get_pending_choice() {
            match choice {
                crate::ability::types::Choice::SelectAutoAbility {
                    player_id: cpid, ..
                }
                | crate::ability::types::Choice::SelectLiveSuccess {
                    player_id: cpid, ..
                } => {
                    return *cpid == pid_str();
                }
                _ => {}
            }
        }
    }

    match game_state.current_phase {
        Phase::RockPaperScissors => {
            if player_id == 0 {
                game_state.player1_rps_choice.is_none()
            } else {
                game_state.player2_rps_choice.is_none()
            }
        }
        Phase::ChooseFirstAttacker => {
            let winner_idx = game_state.rps_winner;
            // rps_winner is 1-indexed (1 = P1 wins, 2 = P2 wins)
            winner_idx == Some(if player_id == 0 { 1 } else { 2 })
        }
        Phase::MulliganFirstAttacker => {
            if game_state.player1.is_first_attacker {
                player_id == 0
            } else {
                player_id == 1
            }
        }
        Phase::MulliganSecondAttacker => {
            if game_state.player1.is_first_attacker {
                player_id == 1
            } else {
                player_id == 0
            }
        }
        Phase::LiveCardSetFirstAttacker => {
            if game_state.player1.is_first_attacker {
                player_id == 0
            } else {
                player_id == 1
            }
        }
        Phase::LiveCardSetSecondAttacker => {
            if game_state.player1.is_first_attacker {
                player_id == 1
            } else {
                player_id == 0
            }
        }
        Phase::FirstAttackerPerformance => {
            if game_state.player1.is_first_attacker {
                player_id == 0
            } else {
                player_id == 1
            }
        }
        Phase::SecondAttackerPerformance => {
            if game_state.player1.is_first_attacker {
                player_id == 1
            } else {
                player_id == 0
            }
        }
        _ => {
            let active = game_state.active_player();
            let is_player1 = player_id == 0;
            (active.id == game_state.player1.id) == is_player1
        }
    }
}

/// Filter the game state display to hide information from the opponent
/// based on the requesting player's perspective. Only applied in PVP mode.
fn filter_display_for_player(
    display: &mut GameStateDisplay,
    game_state: &GameState,
    requester_player_id: i32,
) {
    let hide_card = |card: &mut CardDisplay| {
        card.card_no = "-1".to_string();
        card.name = "Hidden Card".to_string();
        card.card_type = "Hidden".to_string();
        card.ability_text = None;
        card.base_heart = None;
        card.id = -1;
        card.hidden = true;
        card.blade = 0;
        card.total_blade = 0;
        card.bonus_blade = 0;
        card.bonus_hearts.clear();
        card.bonus_score = 0;
        card.bonus_cost = 0;
        card.heart_transform = None;
    };

    // Determine opponent player index (0 for player1, 1 for player2)
    let opp_idx = if requester_player_id == 0 { 1 } else { 0 };

    // 1. Always hide opponent's hand cards
    let opp_hand = if opp_idx == 1 {
        &mut display.player2.hand
    } else {
        &mut display.player1.hand
    };
    for card in &mut opp_hand.cards {
        hide_card(card);
    }

    // 2. Hide opponent's live zone cards if they haven't performed yet
    //    Opponent has performed when:
    //    - Phase is LiveVictoryDetermination or SecondAttackerPerformance (both performed)
    //    - Phase is FirstAttackerPerformance AND opponent is the first attacker
    let opponent_is_first_attacker = if opp_idx == 0 {
        game_state.player1.is_first_attacker
    } else {
        !game_state.player1.is_first_attacker
    };

    let opponent_performed = match game_state.current_phase {
        crate::game_state::Phase::LiveVictoryDetermination
        | crate::game_state::Phase::SecondAttackerPerformance => true,
        crate::game_state::Phase::FirstAttackerPerformance => opponent_is_first_attacker,
        _ => false,
    };

    if !opponent_performed {
        let opp_live = if opp_idx == 1 {
            &mut display.player2.live_zone
        } else {
            &mut display.player1.live_zone
        };
        for card in &mut opp_live.cards {
            hide_card(card);
        }
    }

    // 3. Hide opponent's RPS choice until resolution is complete
    //    (prevents P2 from seeing P1's choice before submitting their own).
    if game_state.rps_winner.is_none() {
        if requester_player_id == 0 {
            display.player2_rps_choice = None;
        } else {
            display.player1_rps_choice = None;
        }
    }

    // 4. If the pending choice is routed to a player different from the requester,
    //    remove the entire pending_choice so the opponent's card data doesn't leak.
    if let Some(ref pending) = display.pending_choice {
        let cpid = pending.get("choice_player_id").and_then(|v| v.as_str());
        if let Some(cpid) = cpid {
            let requester_str = if requester_player_id == 0 { "p1" } else { "p2" };
            if cpid != requester_str {
                display.pending_choice = None;
            }
        }
    }
}

pub async fn execute_action(
    data: web::Data<AppState>,
    req: web::Json<ExecuteActionRequest>,
    http_req: actix_web::HttpRequest,
) -> impl Responder {
    // PVP: verify the requesting player is allowed to act
    // IMPORTANT: do NOT call resolve_game_state_arc while holding the rooms lock
    // (std::sync::Mutex is not reentrant — same thread would deadlock)
    let exec_room_id_str = get_room_id_from_req(&http_req);
    let pvp_player_pid = exec_room_id_str.as_deref().and_then(|rid| {
        let rooms = data.rooms.lock().unwrap();
        rooms.get(rid).and_then(|room| {
            if room.mode != "pvp" && room.mode != "pve" {
                return None;
            }
            let token = get_session_token_from_req(&http_req)?;
            room.sessions.get(&token).map(|s| s.player_id)
        })
    });
    let gs_arc = resolve_game_state_arc(&data, &http_req);
    if let Some(pid) = pvp_player_pid {
        let gs = lock_state!(gs_arc, read);
        if !pvp_player_can_act(&gs, pid) {
            return HttpResponse::Forbidden().json(serde_json::json!({
                "error": "It's not your turn. Waiting for opponent."
            }));
        }
        drop(gs);
    }

    // Check if there was already a pending choice before this action
    // (i.e. this is a resume, not a fresh action).
    // Hold the read lock for both checks atomically to avoid TOCTOU races.
    let (had_choice_before, snapshot) = {
        let gs = lock_state!(gs_arc, read);
        (gs.has_pending_choice(), gs.clone())
    };
    let mut game_state = lock_state!(gs_arc, write);

    let action_type = req
        .action_type
        .as_ref()
        .and_then(|t| t.parse::<ActionType>().ok())
        .unwrap_or(ActionType::Pass);

    // PVP RPS: set transient player_id so the handler routes to the correct player
    if matches!(
        action_type,
        ActionType::RockChoice | ActionType::PaperChoice | ActionType::ScissorsChoice
    ) {
        game_state.pending_rps_player_id = pvp_player_pid;
    }

    let result = crate::turn::TurnEngine::execute_main_phase_action(
        &mut game_state,
        &action_type,
        req.card_id,
        req.card_indices.as_ref().cloned(),
        req.stage_area
            .as_ref()
            .and_then(|s| s.parse::<crate::zones::MemberArea>().ok()),
        req.use_baton_touch,
    );

    match result {
        Ok(_) => {
            if let Err(e) = settle_single_player_state(&mut game_state) {
                return HttpResponse::BadRequest().json(serde_json::json!({ "error": e }));
            }

            // Use per-room state when available
            if let Some(ref rid) = exec_room_id_str {
                if let Ok(mut rooms) = data.rooms.lock() {
                    if let Some(room) = rooms.get_mut(rid) {
                        // Fresh action: push before-state as undo point.
                        // Resume: the before-state is already in history (was pushed
                        // as the choice-boundary snapshot from the previous call), so
                        // skip it to avoid duplicates.
                        if !had_choice_before {
                            room.history.push(snapshot);
                        }
                        // If the ability engine created a new pending choice at a
                        // choice boundary, push the current state so undo can step
                        // back to just before this choice.
                        if game_state.ability_queue.snapshot_requested {
                            game_state.ability_queue.snapshot_requested = false;
                            room.history.push(game_state.clone());
                        }
                        room.future.clear();
                        room.actions_dirty = true;
                        room.frame_counter += 1;
                        let frame = room.frame_counter;
                        let label = format!(
                            "{}{}",
                            req.action_type.as_deref().unwrap_or("?"),
                            req.card_no
                                .as_deref()
                                .map(|n| format!(": {}", n))
                                .unwrap_or_default(),
                        );
                        room.frame_history
                            .push(FrameSnapshot::capture(&game_state, frame, label));
                        room.last_active = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                    }
                }
            } else {
                let mut history = data.history.lock().unwrap();
                if !had_choice_before {
                    history.push(snapshot);
                }
                if game_state.ability_queue.snapshot_requested {
                    game_state.ability_queue.snapshot_requested = false;
                    history.push(game_state.clone());
                }
                drop(history);
                data.future.lock().unwrap().clear();
                invalidate_actions(&data, None);
                let mut fc = data.frame_counter.lock().unwrap();
                *fc += 1;
                let frame = *fc;
                let label = format!(
                    "{}{}",
                    req.action_type.as_deref().unwrap_or("?"),
                    req.card_no
                        .as_deref()
                        .map(|n| format!(": {}", n))
                        .unwrap_or_default(),
                );
                data.frame_history
                    .lock()
                    .unwrap()
                    .push(FrameSnapshot::capture(&game_state, frame, label));
            }

            // Release write lock before building display (read-only work)
            drop(game_state);

            // Notify other SSE clients that state changed (skip in sandbox mode — single client)
            if let Some(rid) = get_room_id_from_req(&http_req) {
                let is_multiplayer = data.rooms.lock().ok()
                    .and_then(|r| r.get(&rid).map(|room| room.mode.as_str() == "pvp"))
                    .unwrap_or(false);
                if is_multiplayer {
                    notify_room_clients(&data, &rid);
                }
            }

            let game_state = lock_state!(gs_arc, read);
            let mut display = crate::display::game_state_to_display(&game_state);
            if let Some(ref rid) = exec_room_id_str {
                if let Ok(rooms) = data.rooms.lock() {
                    if let Some(room) = rooms.get(rid) {
                        display.mode = room.mode.clone();
                    }
                }
            }
            if let Some(pid) = pvp_player_pid {
                filter_display_for_player(&mut display, &game_state, pid);
                if !pvp_player_can_act(&game_state, pid) {
                    display.waiting_for_opponent = true;
                }
            }
            ensure_actions(&data, &game_state, exec_room_id_str.as_deref());
            let actions = read_actions(&data, exec_room_id_str.as_deref());
            let final_actions = if display.waiting_for_opponent {
                None
            } else {
                Some(actions)
            };
            HttpResponse::Ok().json(GameStateResponse {
                game_state: display,
                legal_actions: final_actions,
                ui_config: None,
            })
        }
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let members = data.card_database.cards.len();
    HttpResponse::Ok().json(serde_json::json!({
        "status": "rust_server",
        "members": members,
        "lives": 0,
        "instance_id": 1
    }))
}

async fn set_ui_config(
    data: web::Data<AppState>,
    req: web::Json<SetUiConfigRequest>,
) -> impl Responder {
    let mut ui_config = data.ui_config.lock().unwrap();

    if let Some(lang) = &req.current_lang {
        ui_config.current_lang = lang.clone();
    }
    if let Some(perspective) = req.perspective_player {
        ui_config.perspective_player = perspective;
    }
    if let Some(turn) = req.selected_turn {
        ui_config.selected_turn = turn;
    }
    if let Some(perf) = req.selected_perf_turn {
        ui_config.selected_perf_turn = perf;
    }
    if let Some(abilities) = req.show_friendly_abilities {
        ui_config.show_friendly_abilities = abilities;
    }
    if let Some(hotseat) = req.hotseat_mode {
        ui_config.hotseat_mode = hotseat;
    }
    if let Some(replay) = req.replay_mode {
        ui_config.replay_mode = replay;
    }

    HttpResponse::Ok().json(serde_json::json!({"success": true, "ui_config": &*ui_config}))
}

async fn set_ai(_data: web::Data<AppState>, _req: web::Json<serde_json::Value>) -> impl Responder {
    // Placeholder for AI mode setting

    HttpResponse::Ok().json(serde_json::json!({

        "success": true

    }))
}

fn pvp_mode_for_room(data: &AppState, room_id: Option<&str>) -> bool {
    room_id.is_some_and(|rid| {
        data.rooms
            .lock()
            .ok()
            .and_then(|rooms| rooms.get(rid).map(|r| r.mode == "pvp"))
            .unwrap_or(false)
    })
}

async fn undo(data: web::Data<AppState>, req: actix_web::HttpRequest) -> impl Responder {
    let undo_room_id = get_room_id_from_req(&req);
    let snapshot = if let Some(ref rid) = undo_room_id {
        let mut rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(rid) {
            room.history
                .pop()
                .ok_or_else(|| HttpResponse::BadRequest().json("No history to undo"))
        } else {
            return HttpResponse::BadRequest().json("Room not found");
        }
    } else {
        data.history
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| HttpResponse::BadRequest().json("No history to undo"))
    };
    let snapshot = match snapshot {
        Ok(s) => s,
        Err(e) => return e,
    };
    let gs_arc = resolve_game_state_arc(&data, &req);
    let mut game_state = lock_state!(gs_arc, write);
    let is_pvp = pvp_mode_for_room(&data, undo_room_id.as_deref());
    if let Some(ref rid) = undo_room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                room.future.push(game_state.clone());
            }
        }
    } else {
        data.future.lock().unwrap().push(game_state.clone());
    }
    *game_state = snapshot;

    if !is_pvp {
        if let Err(e) = settle_single_player_state(&mut game_state) {
            log::debug!("Single-player settle error after undo: {}", e);
        }
    }
    invalidate_actions(&data, undo_room_id.as_deref());
    if let Some(ref rid) = undo_room_id {
        notify_room_clients(&data, rid);
    }
    // Update frame counter so polling clients detect the change
    if let Some(ref rid) = undo_room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                room.frame_counter += 1;
                room.last_active = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }
    } else if let Ok(mut fc) = data.frame_counter.lock() {
        *fc += 1;
    }
    let display = crate::display::game_state_to_display(&game_state);
    drop(game_state);
    let ui_config = data.ui_config.lock().unwrap().clone();
    HttpResponse::Ok().json(GameStateResponse {
        game_state: display,
        legal_actions: None,
        ui_config: Some(ui_config),
    })
}

async fn redo(data: web::Data<AppState>, req: actix_web::HttpRequest) -> impl Responder {
    let redo_room_id = get_room_id_from_req(&req);
    let snapshot = if let Some(ref rid) = redo_room_id {
        let mut rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(rid) {
            room.future
                .pop()
                .ok_or_else(|| HttpResponse::BadRequest().json("No future to redo"))
        } else {
            return HttpResponse::BadRequest().json("Room not found");
        }
    } else {
        data.future
            .lock()
            .unwrap()
            .pop()
            .ok_or_else(|| HttpResponse::BadRequest().json("No future to redo"))
    };
    let snapshot = match snapshot {
        Ok(s) => s,
        Err(e) => return e,
    };
    let gs_arc = resolve_game_state_arc(&data, &req);
    let mut game_state = lock_state!(gs_arc, write);
    let is_pvp = pvp_mode_for_room(&data, redo_room_id.as_deref());
    if let Some(ref rid) = redo_room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                room.history.push(game_state.clone());
            }
        }
    } else {
        data.history.lock().unwrap().push(game_state.clone());
    }
    *game_state = snapshot;

    if !is_pvp {
        if let Err(e) = settle_single_player_state(&mut game_state) {
            log::debug!("Single-player settle error after redo: {}", e);
        }
    }
    invalidate_actions(&data, redo_room_id.as_deref());
    if let Some(ref rid) = redo_room_id {
        notify_room_clients(&data, rid);
    }
    // Update frame counter so polling clients detect the change
    if let Some(ref rid) = redo_room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                room.frame_counter += 1;
                room.last_active = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }
    } else if let Ok(mut fc) = data.frame_counter.lock() {
        *fc += 1;
    }
    let display = crate::display::game_state_to_display(&game_state);
    drop(game_state);
    let ui_config = data.ui_config.lock().unwrap().clone();
    HttpResponse::Ok().json(GameStateResponse {
        game_state: display,
        legal_actions: None,
        ui_config: Some(ui_config),
    })
}

async fn exec_code(
    data: web::Data<AppState>,
    req: web::Json<ExecCodeRequest>,
    http_req: actix_web::HttpRequest,
) -> impl Responder {
    let gs_arc = resolve_game_state_arc(&data, &http_req);
    let mut game_state = lock_state!(gs_arc, write);
    let code = &req.code;

    // Parse key=value pairs from Rust-like code
    fn parse_param(code: &str, key: &str) -> Option<String> {
        for segment in code.split(';') {
            let seg = segment.trim();
            // handle "let key = value" and "key=value" formats
            let without_let = if let Some(rest) = seg.strip_prefix("let ") {
                rest
            } else {
                seg
            };
            if let Some(eq_pos) = without_let.find('=') {
                let k = without_let[..eq_pos].trim().to_lowercase();
                if k == key {
                    let v = without_let[eq_pos + 1..]
                        .trim()
                        .trim_matches('"')
                        .to_string();
                    return Some(v);
                }
            }
        }
        None
    }

    let player_idx: usize = parse_param(code, "player_idx")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    macro_rules! with_player {
        ($p:ident, $body:block) => {
            let $p = if player_idx == 0 {
                &mut game_state.player1
            } else {
                &mut game_state.player2
            };
            $body
        };
    }

    let amount: u32 = parse_param(code, "amount")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    if code.contains("draw_energy") {
        with_player!(p, {
            for _ in 0..amount {
                let _ = p.draw_energy();
            }
        });
    }

    if code.contains("add_card") && code.contains("card_no") {
        let card_no = parse_param(code, "card_no").unwrap_or_default();
        if let Some(card_id) = game_state.card_database.get_card_id(&card_no) {
            with_player!(p, {
                p.hand.add_card(card_id);
            });
        }
    }

    if code.contains("draw_card") {
        for _ in 0..amount {
            with_player!(p, {
                if let Some(cid) = p.main_deck.draw() {
                    p.hand.add_card(cid);
                }
            });
        }
    }

    if code.contains("clear_hand") {
        with_player!(p, {
            let cards: Vec<i16> = p.hand.cards.drain(..).collect();
            for cid in cards {
                p.waitroom.add_card(cid);
            }
        });
    }

    if code.contains("force_win") {
        let p = &mut game_state.player1;
        if let Some(cid) = p.live_card_zone.cards.first().copied() {
            if cid >= 0 {
                for _ in 0..3 {
                    p.success_live_card_zone.cards.push(cid);
                }
            }
        }
    }

    if code.contains("reshuffle_deck") {
        with_player!(p, {
            p.main_deck.shuffle();
        });
    }

    if code.contains("add_live_to_zone") {
        let card_no = parse_param(code, "card_no").unwrap_or_default();
        if let Some(card_id) = game_state.card_database.get_card_id(&card_no) {
            with_player!(p, {
                p.live_card_zone.cards.push(card_id);
            });
        }
    }

    if code.contains("add_stage") {
        let card_no = parse_param(code, "card_no").unwrap_or_default();
        if let Some(card_id) = game_state.card_database.get_card_id(&card_no) {
            with_player!(p, {
                for slot in 0..3 {
                    if p.stage.stage[slot] == -1 {
                        p.stage.stage[slot] = card_id;
                        break;
                    }
                }
            });
        }
    }

    if code.contains("negative_energy") {
        with_player!(p, {
            if let Some(cid) = p.energy_zone.cards.pop() {
                p.energy_deck.cards.push(cid);
                p.energy_zone.set_active_count(p.energy_zone.active_count().min(p.energy_zone.cards.len()));
            }
        });
    }

    if code.contains("to_success") {
        let card_no = parse_param(code, "card_no").unwrap_or_default();
        if let Some(card_id) = game_state.card_database.get_card_id(&card_no) {
            with_player!(p, {
                p.success_live_card_zone.cards.push(card_id);
            });
        }
    }

    if code.contains("remove_success") {
        with_player!(p, {
            p.success_live_card_zone.cards.clear();
        });
        game_state.recalculate_constants();
    }

    if code.contains("to_discard") {
        let card_no = parse_param(code, "card_no").unwrap_or_default();
        if let Some(card_id) = game_state.card_database.get_card_id(&card_no) {
            with_player!(p, {
                p.waitroom.add_card(card_id);
            });
        }
    }

    if code.contains("add_to_deck_top") {
        let card_no = parse_param(code, "card_no").unwrap_or_default();
        if let Some(card_id) = game_state.card_database.get_card_id(&card_no) {
            with_player!(p, {
                p.main_deck.cards.insert(0, card_id);
            });
        }
    }

    let exec_room_id = get_room_id_from_req(&http_req);
    // Push snapshot to history so exec_code can be undone like any other action
    if let Some(ref rid) = exec_room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                room.history.push(game_state.clone());
                room.future.clear();
                room.actions_dirty = true;
                room.frame_counter += 1;
            }
        }
    } else {
        data.history.lock().unwrap().push(game_state.clone());
        data.future.lock().unwrap().clear();
        if let Ok(mut fc) = data.frame_counter.lock() {
            *fc += 1;
        }
    }
    if let Some(rid) = exec_room_id.as_ref() {
        notify_room_clients(&data, rid);
    }
    ensure_actions(&data, &game_state, exec_room_id.as_deref());
    let display = crate::display::game_state_to_display(&game_state);
    let actions = read_actions(&data, exec_room_id.as_deref());
    let mut response = serde_json::to_value(display).unwrap_or_default();
    response["legal_actions"] =
        serde_json::to_value(&actions).unwrap_or(serde_json::Value::Array(vec![]));

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "state": response
    }))
}

async fn debug_rewind(data: web::Data<AppState>, req: actix_web::HttpRequest) -> impl Responder {
    let rewind_room_id = get_room_id_from_req(&req);
    let snapshot = if let Some(ref rid) = rewind_room_id {
        let mut rooms = data.rooms.lock().unwrap();
        rooms.get_mut(rid).and_then(|r| r.history.pop())
    } else {
        data.history.lock().unwrap().pop()
    };
    let snapshot = match snapshot {
        Some(s) => s,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"success": false, "error": "No history"}))
        }
    };
    let gs_arc = resolve_game_state_arc(&data, &req);
    let mut game_state = lock_state!(gs_arc, write);
    if let Some(ref rid) = rewind_room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                room.future.push(game_state.clone());
            }
        }
    } else {
        data.future.lock().unwrap().push(game_state.clone());
    }
    *game_state = snapshot;
    invalidate_actions(&data, rewind_room_id.as_deref());
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

async fn debug_redo(data: web::Data<AppState>, req: actix_web::HttpRequest) -> impl Responder {
    let redo_room_id = get_room_id_from_req(&req);
    let snapshot = if let Some(ref rid) = redo_room_id {
        let mut rooms = data.rooms.lock().unwrap();
        rooms.get_mut(rid).and_then(|r| r.future.pop())
    } else {
        data.future.lock().unwrap().pop()
    };
    let snapshot = match snapshot {
        Some(s) => s,
        None => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"success": false, "error": "No future"}))
        }
    };
    let gs_arc = resolve_game_state_arc(&data, &req);
    let mut game_state = lock_state!(gs_arc, write);
    if let Some(ref rid) = redo_room_id {
        if let Ok(mut rooms) = data.rooms.lock() {
            if let Some(room) = rooms.get_mut(rid) {
                room.history.push(game_state.clone());
            }
        }
    } else {
        data.history.lock().unwrap().push(game_state.clone());
    }
    *game_state = snapshot;
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

async fn debug_snapshot(
    data: web::Data<AppState>,
    http_req: actix_web::HttpRequest,
) -> impl Responder {
    let gs_arc = resolve_game_state_arc(&data, &http_req);
    let game_state = lock_state!(gs_arc, read);
    let display = crate::display::game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "state": display}))
}

async fn debug_dump_state(
    data: web::Data<AppState>,
    http_req: actix_web::HttpRequest,
) -> impl Responder {
    let gs_arc = resolve_game_state_arc(&data, &http_req);
    let game_state = lock_state!(gs_arc, read);
    let display = crate::display::game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "state": display}))
}

/// GET /api/debug/frames — lightweight frame index
/// Returns frame metadata (no card IDs), suitable for UI counter display.
async fn debug_frames(data: web::Data<AppState>) -> impl Responder {
    let frames = data.frame_history.lock().unwrap();
    let current = frames.len().saturating_sub(1);
    let index: Vec<serde_json::Value> = frames
        .iter()
        .map(|f| {
            serde_json::json!({
                "frame": f.frame,
                "turn": f.turn,
                "phase": f.phase,
                "active_player": f.active_player,
                "label": f.label,
                "current": f.frame == current as u64,
            })
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "current_frame": current,
        "total_frames": frames.len(),
        "frames": index,
    }))
}

/// GET /api/debug/dump_frames — download all frame snapshots as JSON
async fn debug_dump_frames(data: web::Data<AppState>) -> impl Responder {
    let frames = data.frame_history.lock().unwrap();
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "total_frames": frames.len(),
        "frames": *frames,
    }))
}

async fn debug_conditions(data: web::Data<AppState>) -> impl Responder {
    let game_state = lock_state!(data.game_state, read);
    let mut results = Vec::new();

    let card_db = &game_state.card_database;

    for (player_idx, player) in [&game_state.player1, &game_state.player2]
        .iter()
        .enumerate()
    {
        let zone_defs: [(&str, &[i16]); 6] = [
            ("stage", &player.stage.stage),
            ("hand", &player.hand.cards),
            ("energy", &player.energy_zone.cards),
            ("waitroom", &player.waitroom.cards),
            ("live_zone", &player.live_card_zone.cards),
            ("success_live_zone", &player.success_live_card_zone.cards),
        ];

        for &(zone_name, cards) in &zone_defs {
            for &card_id in cards {
                if card_id < 0 {
                    continue;
                }

                if let Some(card) = card_db.get_card(card_id) {
                    for (ability_idx, ability) in card.abilities.iter().enumerate() {
                        if let Some(ref effect) = ability.effect {
                            let condition_fields: [(&str, &Option<crate::card::Condition>); 4] = [
                                (
                                    "activation_condition_parsed",
                                    &effect.activation_condition_parsed,
                                ),
                                ("condition", &effect.condition),
                                (
                                    "alternative_condition",
                                    &effect.compound.alternative_condition,
                                ),
                                ("result_condition", &effect.compound.result_condition),
                            ];

                            for &(field_name, condition_opt) in &condition_fields {
                                if let Some(ref condition) = condition_opt {
                                    results.push((
                                        player_idx,
                                        zone_name,
                                        card_id,
                                        card.name.to_string(),
                                        ability_idx,
                                        field_name,
                                        condition.clone(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let state_clone = game_state.clone();

    drop(game_state);

    let ctx = crate::ability::condition::ConditionContext::new(&state_clone);

    let evaluated: Vec<serde_json::Value> = results
        .into_iter()
        .map(
            |(player_idx, zone_name, card_id, card_name, ability_idx, field_name, condition)| {
                let (result, actual_value) = ctx.evaluate_condition_debug(&condition);

                serde_json::json!({
                    "player": player_idx,
                    "zone": zone_name,
                    "card_id": card_id,
                    "card_name": card_name,
                    "ability_index": ability_idx,
                    "field": field_name,
                    "condition_type": condition.condition_type,
                    "condition_text": condition.text,
                    "condition_data": serde_json::to_value(&condition).unwrap_or_default(),
                    "result": result,
                    "actual_value": actual_value,
                })
            },
        )
        .collect();

    HttpResponse::Ok().json(serde_json::json!({"success": true, "conditions": evaluated}))
}

async fn export_game(data: web::Data<AppState>) -> impl Responder {
    let game_state = lock_state!(data.game_state, read);
    let display = crate::display::game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "game_state": display}))
}

fn deck_files() -> Vec<PathBuf> {
    let decks_dir = PathBuf::from("../web_ui/decks");
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&decks_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "txt").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn deck_name_from_path(path: &PathBuf) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| {
            s.split('_')
                .map(|w| {
                    let mut c = w.chars();
                    c.next()
                        .map(|f| f.to_uppercase().to_string() + c.as_str())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn parse_deck_text(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .flat_map(|l| {
            let parts: Vec<&str> = l.split(" x ").collect();
            if parts.len() == 2 {
                let (card_no, quantity) = if let Ok(q) = parts[0].trim().parse::<u32>() {
                    (parts[1].trim().to_string(), q)
                } else if let Ok(q) = parts[1].trim().parse::<u32>() {
                    (parts[0].trim().to_string(), q)
                } else {
                    return Vec::new();
                };
                if card_no.contains('-') {
                    return std::iter::repeat_n(card_no, quantity as usize).collect();
                }
                return Vec::new();
            }
            // Single card per line (quantity = 1)
            let card_no = l.trim();
            if card_no.contains('-') {
                return vec![card_no.to_string()];
            }
            Vec::new()
        })
        .collect()
}

async fn get_decks(_data: web::Data<AppState>) -> impl Responder {
    let decks: Vec<serde_json::Value> = deck_files()
        .into_iter()
        .map(|path| {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let name = deck_name_from_path(&path);
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            let card_count = content
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| l.split(" x ").nth(1))
                .filter_map(|q| q.trim().parse::<u32>().ok())
                .sum::<u32>();
            let main = parse_deck_text(&content);
            serde_json::json!({
                "id": id,
                "name": name,
                "card_count": card_count,
                "content": content,
                "main": main,
                "energy": [],
            })
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "decks": decks }))
}

async fn get_random_deck(_data: web::Data<AppState>) -> impl Responder {
    let files = deck_files();
    if files.is_empty() {
        return HttpResponse::NotFound()
            .json(serde_json::json!({ "success": false, "error": "No decks found" }));
    }
    use rand::seq::SliceRandom;
    let chosen = files.choose(&mut rand::thread_rng()).unwrap();
    let content = std::fs::read_to_string(chosen).unwrap_or_default();
    HttpResponse::Ok()
        .json(serde_json::json!({ "success": true, "content": content, "energy": [] }))
}

async fn get_test_deck(_data: web::Data<AppState>) -> impl Responder {
    let path = PathBuf::from("../web_ui/decks/aqours_cup.txt");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "content": content }))
}

pub async fn set_deck(
    data: web::Data<AppState>,
    req: web::Json<serde_json::Value>,
) -> impl Responder {
    let player = req.get("player").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let room_id = req
        .get("room_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let card_numbers: Vec<String> = if let Some(arr) = req.get("deck").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| deck_parser::DeckParser::normalize_card_no(s)))
            .collect()
    } else {
        let deck_content = req.get("deck").and_then(|v| v.as_str()).unwrap_or("");
        if deck_content.is_empty() {
            Vec::new()
        } else {
            deck_parser::DeckParser::parse_deck_content(deck_content)
        }
    };
    if card_numbers.is_empty() {
        return HttpResponse::Ok()
            .json(serde_json::json!({ "success": false, "status": "empty_deck" }));
    }

    // If room_id is present, store deck in the room's custom_decks
    let mut init_game = false;
    if let Some(ref rid) = room_id {
        let mut rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(rid) {
            let decks = room.custom_decks.get_or_insert_with(HashMap::new);
            let deck_entry = decks.entry(player).or_insert_with(|| CustomDeck {
                main: Vec::new(),
                energy: Vec::new(),
            });
            deck_entry.main = card_numbers.clone();
            if let Some(energy_arr) = req.get("energy_deck").and_then(|v| v.as_array()) {
                deck_entry.energy = energy_arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| deck_parser::DeckParser::normalize_card_no(s)))
                    .collect();
            }
            // Check if both players have submitted
            if decks.contains_key(&0) && decks.contains_key(&1) {
                if try_init_room_game_state(room, &data) {
                    init_game = true;
                    // Notify SSE clients that game state is ready
                    notify_room_clients(&data, rid);
                }
            }
        }
    } else {
        // Legacy sandbox mode: store in global custom_decks
        if !card_numbers.is_empty() {
            data.custom_decks
                .lock()
                .unwrap()
                .insert(player, card_numbers);
        }
        if let Some(energy_arr) = req.get("energy_deck").and_then(|v| v.as_array()) {
            let energy_cards: Vec<String> = energy_arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !energy_cards.is_empty() {
                data.custom_energy_decks
                    .lock()
                    .unwrap()
                    .insert(player, energy_cards);
            }
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "status": "ok",
        "room_init": init_game,
        "room_id": room_id
    }))
}

async fn rooms_list(data: web::Data<AppState>) -> impl Responder {
    let rooms = data.rooms.lock().unwrap();
    let public_rooms: Vec<serde_json::Value> = rooms
        .values()
        .filter(|r| r.public)
        .map(|r| {
            serde_json::json!({
                "room_id": r.room_id,
                "mode": r.mode,
                "player_count": r.sessions.len(),
                "created_at": r.created_at,
            })
        })
        .collect();
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "rooms": public_rooms }))
}

/// Notify all SSE clients in a room that state has changed.
fn notify_room_clients(data: &AppState, room_id: &str) {
    let broadcasts = data.room_broadcasts.lock().unwrap();
    if let Some(sender) = broadcasts.get(room_id) {
        let count = sender.receiver_count();
        let _ = sender.send(());
        log::debug!("[SSE] Notified room {} ({} clients)", room_id, count);
    } else {
        log::debug!("[SSE] No broadcast sender for room {}", room_id);
    }
}

/// SSE endpoint: clients connect here to receive push updates when game state changes.
/// Room ID is passed as query param because EventSource doesn't support custom headers.
async fn sse_events(data: web::Data<AppState>, req: actix_web::HttpRequest) -> impl Responder {
    use actix_web::rt::spawn;
    use bytes::Bytes;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;

    let room_id = actix_web::web::Query::<std::collections::HashMap<String, String>>::from_query(
        req.query_string(),
    )
    .ok()
    .and_then(|params| params.get("room_id").cloned())
    .unwrap_or_default();

    if room_id.is_empty() {
        return HttpResponse::BadRequest().body("Missing room_id");
    }

    // Get or create broadcast sender for this room
    let sender = {
        let mut broadcasts = data.room_broadcasts.lock().unwrap();
        broadcasts
            .entry(room_id.clone())
            .or_insert_with(|| {
                let (tx, _) = tokio::sync::broadcast::channel::<()>(32);
                tx
            })
            .clone()
    };

    log::debug!("[SSE] Client connected to room {}", room_id);

    let mut rx = sender.subscribe();
    let (tx, rx_stream) = mpsc::unbounded_channel::<Result<Bytes, actix_web::Error>>();

    // Clone data Arc for cleanup on disconnect
    let cleanup_data = data.clone();
    let cleanup_room_id = room_id.clone();

    // Spawn task: listen for broadcast events + heartbeat every 30s
    spawn(async move {
        tx.send(Ok(Bytes::from("data: connected\n\n"))).ok();
        loop {
            tokio::select! {
                result = rx.recv() => {
                    if result.is_ok() {
                        if tx.send(Ok(Bytes::from("data: update\n\n"))).is_err() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    if tx.send(Ok(Bytes::from(": heartbeat\n\n"))).is_err() {
                        break;
                    }
                }
            }
        }
        // Client disconnected — clean up rooms with no remaining sessions
        let mut rooms = cleanup_data.rooms.lock().unwrap();
        if let Some(room) = rooms.get(&cleanup_room_id) {
            if room.sessions.is_empty() {
                rooms.remove(&cleanup_room_id);
                log::debug!("[SSE] Room {} cleaned up (no sessions)", cleanup_room_id);
            }
        }
        let mut broadcasts = cleanup_data.room_broadcasts.lock().unwrap();
        broadcasts.remove(&cleanup_room_id);
    });

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .streaming(UnboundedReceiverStream::new(rx_stream))
}

async fn get_card_registry(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(&*data.card_registry)
}

/// Initialize a room's game state once both PVP players have submitted decks.
fn try_init_room_game_state(room: &mut Room, data: &AppState) -> bool {
    let decks = match room.custom_decks.as_ref() {
        Some(d) => d,
        None => return false,
    };
    if !decks.contains_key(&0) || !decks.contains_key(&1) {
        return false;
    }
    let p0_deck = &decks[&0];
    let p1_deck = &decks[&1];

    let card_numbers1 = p0_deck.main.clone();
    let card_numbers2 = p1_deck.main.clone();
    let energy_nos1 = p0_deck.energy.clone();
    let energy_nos2 = p1_deck.energy.clone();

    let mut card_database = data.card_database.clone();

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(
        &mut card_database,
        card_numbers1,
    ) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            log::debug!("Room init: failed to build deck for p1: {}", e);
            return false;
        }
    };
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(
        &mut card_database,
        card_numbers2,
    ) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            log::debug!("Room init: failed to build deck for p2: {}", e);
            return false;
        }
    };
    for eid in &energy_nos1 {
        if let Some(tid) = card_database.get_card_id(eid) {
            let cid = Arc::make_mut(&mut card_database).create_copy(tid);
            player1_deck.energy_deck.push_back(cid);
        }
    }
    for eid in &energy_nos2 {
        if let Some(tid) = card_database.get_card_id(eid) {
            let cid = Arc::make_mut(&mut card_database).create_copy(tid);
            player2_deck.energy_deck.push_back(cid);
        }
    }
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut player1_deck,
        &mut card_database,
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut player2_deck,
        &mut card_database,
    );

    let mut p1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
    let mut p2 = Player::new("p2".to_string(), "Player 2".to_string(), false);
    p1.set_main_deck(player1_deck.main_deck);
    p1.set_energy_deck(player1_deck.energy_deck);
    p2.set_main_deck(player2_deck.main_deck);
    p2.set_energy_deck(player2_deck.energy_deck);

    let mut game_state = GameState::new(p1, p2, card_database);
    crate::game_setup::setup_game(&mut game_state);
    room.game_state = Some(Arc::new(RwLock::new(game_state)));
    room.actions_dirty = true;
    true
}

pub async fn rooms_create(
    data: web::Data<AppState>,
    req: web::Json<CreateRoomRequest>,
) -> impl Responder {
    // Skip card database loading for now to avoid deserialization errors

    println!("DEBUG: rooms_create called");

    let room_id: String = (0..4)
        .map(|_| {
            let idx = rand::thread_rng().gen_range(0..26);
            (b'A' + idx) as char
        })
        .collect();

    let mode = req.mode.clone().unwrap_or_else(|| "sandbox".to_string().into());
    // pve mode aliases to sandbox gameplay but preserves the mode string
    // so the frontend can distinguish vs AI from sandbox

    let public = req.public.unwrap_or(false);

    let username = req.username.clone();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Build custom decks if provided

    let mut custom_decks: Option<HashMap<i32, CustomDeck>> = None;

    if req.p0_deck.is_some() || req.p1_deck.is_some() {
        let mut decks = HashMap::new();

        if let Some(p0_deck) = req.p0_deck.clone() {
            decks.insert(
                0,
                CustomDeck {
                    main: p0_deck,

                    energy: req.p0_energy.clone().unwrap_or_default(),
                },
            );
        }

        if let Some(p1_deck) = req.p1_deck.clone() {
            decks.insert(
                1,
                CustomDeck {
                    main: p1_deck,

                    energy: req.p1_energy.clone().unwrap_or_default(),
                },
            );
        }

        custom_decks = Some(decks);
    }

    // For PVP rooms, delay game init until both decks are submitted.
    // For sandbox mode, init game state immediately.
    let is_ai_game = req.is_ai.unwrap_or(false);
    let room_game_state: Option<Arc<RwLock<GameState>>> = if mode == "pvp" && !is_ai_game {
        None
    } else {
        let card_database = data.card_database.clone();
        let player1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
        let player2 = Player::new("p2".to_string(), "Player 2".to_string(), false);
        let mut fresh_game_state = GameState::new(player1, player2, card_database);
        crate::game_setup::setup_game(&mut fresh_game_state);
        println!(
            "DEBUG: Fresh room game state initialized with phase: {:?}",
            fresh_game_state.current_phase
        );
        Some(Arc::new(RwLock::new(fresh_game_state)))
    };

    let room = Room {
        room_id: room_id.clone(),

        mode: mode.clone(),

        public,

        created_at: now,

        last_active: now,

        sessions: HashMap::new(),

        usernames: HashMap::new(),

        custom_decks,

        game_state: room_game_state,
        history: Vec::new(),
        future: Vec::new(),
        frame_counter: 0,
        frame_history: Vec::new(),
        cached_actions: Vec::new(),
        actions_dirty: true,
    };

    println!("DEBUG: Inserting room with ID: {}", room_id);

    {
        let mut rooms = data.rooms.lock().unwrap();

        rooms.insert(room_id.clone(), room);

        println!("DEBUG: Room inserted, total rooms: {}", rooms.len());

        // Explicitly drop the lock to ensure room is stored

        drop(rooms);

        println!("DEBUG: Room lock dropped, room should be stored");
    }

    // Auto-join creator

    let session_id = Uuid::new_v4().to_string();

    let player_id = 0; // Creator always gets player 0

    let ai_session_id;

    {
        let mut rooms = data.rooms.lock().unwrap();

        if let Some(room) = rooms.get_mut(&room_id) {
            room.sessions.insert(
                session_id.clone(),
                RoomSession {
                    session_id: session_id.clone(),

                    player_id,

                    username: username.clone(),
                },
            );

            if is_ai_game {
                ai_session_id = Uuid::new_v4().to_string();
                room.sessions.insert(
                    ai_session_id.clone(),
                    RoomSession {
                        session_id: ai_session_id.clone(),
                        player_id: 1,
                        username: Some("AI".to_string()),
                    },
                );
            } else {
                ai_session_id = String::new();
            }

            if let Some(name) = username {
                room.usernames.insert(player_id, name);
            }

            room.last_active = now;
        } else {
            ai_session_id = String::new();
        }
    }

    let mut response = serde_json::json!({
        "success": true,
        "room_id": room_id,
        "mode": mode,
        "session": {
            "session_id": session_id,
            "player_id": player_id
        }
    });

    if is_ai_game {
        response["ai_session"] = serde_json::json!({
            "session_id": ai_session_id,
            "player_id": 1
        });
    }

    HttpResponse::Ok().json(response)
}

pub async fn rooms_join(
    data: web::Data<AppState>,
    req: web::Json<JoinRoomRequest>,
) -> impl Responder {
    let room_id = req.room_id.to_uppercase();

    let username = req.username.clone();

    let session_id = Uuid::new_v4().to_string();

    let mut player_id = -1;

    {
        let mut rooms = data.rooms.lock().unwrap();

        if let Some(room) = rooms.get_mut(&room_id) {
            // Check for recovery by username

            if let Some(name) = &username {
                for (pid, existing_name) in &room.usernames {
                    if existing_name == name {
                        player_id = *pid;

                        room.sessions.insert(
                            session_id.clone(),
                            RoomSession {
                                session_id: session_id.clone(),

                                player_id,

                                username: Some(name.clone()),
                            },
                        );

                        room.last_active = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();

                        return HttpResponse::Ok().json(serde_json::json!({

                            "success": true,

                            "room_id": room_id,

                            "mode": room.mode,

                            "session": {

                                "session_id": session_id,

                                "player_id": player_id

                            },

                            "recovered": true

                        }));
                    }
                }
            }

            // Assign new player

            let taken_pids: std::collections::HashSet<i32> =
                room.sessions.values().map(|s| s.player_id).collect();

            if !taken_pids.contains(&0) {
                player_id = 0;
            } else if !taken_pids.contains(&1) {
                player_id = 1;
            }

            if player_id >= 0 {
                room.sessions.insert(
                    session_id.clone(),
                    RoomSession {
                        session_id: session_id.clone(),

                        player_id,

                        username: username.clone(),
                    },
                );

                if let Some(name) = username {
                    room.usernames.insert(player_id, name);
                }

                room.last_active = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        } else {
            return HttpResponse::NotFound().json(serde_json::json!({

                "success": false,

                "error": "Room not found"

            }));
        }
    }

    if player_id < 0 {
        return HttpResponse::BadRequest().json(serde_json::json!({

            "success": false,

            "error": "Room is full"

        }));
    }

    let mode = {
        let rooms = data.rooms.lock().unwrap();

        rooms
            .get(&room_id)
            .map(|r| r.mode.clone())
            .unwrap_or_else(|| "sandbox".to_string().into())
    };

    HttpResponse::Ok().json(serde_json::json!({

        "success": true,

        "room_id": room_id,

        "mode": mode,

        "session": {

            "session_id": session_id,

            "player_id": player_id

        }

    }))
}

pub async fn rooms_leave(
    data: web::Data<AppState>,
    req: web::Json<serde_json::Value>,
    http_req: actix_web::HttpRequest,
) -> impl Responder {
    // Try body first, fallback to headers
    let room_id = req
        .get("room_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_uppercase())
        .or_else(|| get_room_id_from_req(&http_req));
    let _session_token: Option<String> = req
        .get("session_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| get_session_token_from_req(&http_req));

    let room_id = match room_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Room ID required"
            }))
        }
    };

    // Notify other SSE clients that the room is closing, then destroy it
    {
        let broadcasts = data.room_broadcasts.lock().unwrap();
        if let Some(sender) = broadcasts.get(&room_id) {
            // Send a special "closed" event so other players know to redirect
            let _ = sender.send(());
            log::debug!("[SSE] Room {} closing: notified remaining clients", room_id);
        }
    }
    // Remove broadcast channel
    {
        let mut broadcasts = data.room_broadcasts.lock().unwrap();
        broadcasts.remove(&room_id);
    }

    // Destory the room entirely — a leave means the match is over
    {
        let mut rooms = data.rooms.lock().unwrap();
        rooms.remove(&room_id);
    }

    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

async fn init_game(
    data: web::Data<AppState>,
    req: Option<web::Json<InitGameRequest>>,
    http_req: actix_web::HttpRequest,
) -> impl Responder {
    let mut card_database = data.card_database.clone();
    let deck_lists = data.deck_lists.clone();

    // Map frontend deck names to deck file names

    let deck_name_mapping = std::collections::HashMap::from([
        ("Aqours Cup", "aqours_cup"),
        ("Muse Cup", "muse_cup"),
        ("Nijigaku Cup", "nijigaku_cup"),
        ("Liella Cup", "liella_cup"),
        ("Hasunosora Cup", "hasunosora_cup"),
        ("Fade Deck", "fade deck"),
    ]);

    // Select deck based on request, default to first deck if not specified or not found

    let selected_deck_name = req.as_ref().and_then(|r| r.deck.as_deref());

    let deck_index = if let Some(name) = selected_deck_name {
        if let Some(file_name) = deck_name_mapping.get(name) {
            deck_lists.iter().position(|d| d.name == *file_name)
        } else {
            None
        }
    } else {
        None
    };

    // Check for custom decks: first try room's custom_decks, then global fallback
    let init_room_id_custom = get_room_id_from_req(&http_req);
    let (card_numbers1, card_numbers2, energy_nos1, energy_nos2) = {
        // Try room's custom decks first
        let room_decks = init_room_id_custom.as_ref().and_then(|rid| {
            let rooms = data.rooms.lock().unwrap();
            rooms.get(rid).and_then(|r| r.custom_decks.clone())
        });
        if let Some(decks) = room_decks {
            let p0 = decks.get(&0).map(|d| d.main.clone()).unwrap_or_default();
            let p1 = decks.get(&1).map(|d| d.main.clone()).unwrap_or_default();
            let e0 = decks.get(&0).map(|d| d.energy.clone()).unwrap_or_default();
            let e1 = decks.get(&1).map(|d| d.energy.clone()).unwrap_or_default();
            (p0, p1, e0, e1)
        } else {
            let mut custom = data.custom_decks.lock().unwrap();
            let mut custom_energy = data.custom_energy_decks.lock().unwrap();
            if custom.contains_key(&0) || custom.contains_key(&1) {
                let p0 = custom.remove(&0).unwrap_or_default();
                let p1 = custom.remove(&1).unwrap_or_else(|| p0.clone());
                let e0 = custom_energy.remove(&0).unwrap_or_default();
                let e1 = custom_energy.remove(&1).unwrap_or_else(|| e0.clone());
                (p0, p1, e0, e1)
            } else if deck_lists.is_empty() {
                return HttpResponse::InternalServerError().json(
                    "No decks available. Ensure deck files are present in web_ui/decks/."
                );
            } else {
                let deck = if let Some(idx) = deck_index {
                    if idx >= deck_lists.len() {
                        &deck_lists[0]
                    } else {
                        &deck_lists[idx]
                    }
                } else {
                    &deck_lists[0]
                };
                let nos = deck_parser::DeckParser::deck_list_to_card_numbers(deck);
                (nos.clone(), nos, Vec::new(), Vec::new())
            }
        }
    };

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(
        &mut card_database,
        card_numbers1,
    ) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();

            deck.shuffle_energy_deck();

            deck
        }

        Err(e) => {
            log::debug!("Failed to build deck for Player 1: {}", e);

            return HttpResponse::InternalServerError().json("Failed to build deck for Player 1");
        }
    };

    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(
        &mut card_database,
        card_numbers2,
    ) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();

            deck.shuffle_energy_deck();

            deck
        }

        Err(e) => {
            log::debug!("Failed to build deck for Player 2: {}", e);

            return HttpResponse::InternalServerError().json("Failed to build deck for Player 2");
        }
    };

    // Merge separately-provided energy cards into decks
    {
        let deck_refs = [
            (&mut player1_deck, &energy_nos1),
            (&mut player2_deck, &energy_nos2),
        ];
        for (deck, energy_ids) in deck_refs {
            for eid in energy_ids {
                if let Some(template_id) = card_database.get_card_id(eid) {
                    let card_id = Arc::make_mut(&mut card_database).create_copy(template_id);
                    deck.energy_deck.push_back(card_id);
                }
            }
        }
    }

    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut player1_deck,
        &mut card_database,
    );
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(
        &mut player2_deck,
        &mut card_database,
    );

    // Create fresh players

    let mut player1 = Player::new("p1".to_string(), "Player 1".to_string(), true);

    let mut player2 = Player::new("p2".to_string(), "Player 2".to_string(), false);

    player1.set_main_deck(player1_deck.main_deck);

    player1.set_energy_deck(player1_deck.energy_deck);

    player2.set_main_deck(player2_deck.main_deck);

    player2.set_energy_deck(player2_deck.energy_deck);

    // Create fresh game state with CardDatabase

    let mut game_state = GameState::new(player1, player2, card_database);

    // Setup game (Rule 6.2)

    crate::game_setup::setup_game(&mut game_state);

    // Don't call settle_single_player_state here - game should start in RockPaperScissors phase

    println!(
        "DEBUG: init_game complete, phase: {:?}",
        game_state.current_phase
    );

    let init_room_id = get_room_id_from_req(&http_req);
    if let Some(ref rid) = init_room_id {
        // Room context: write to room's state, history, frames
        let mut rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(rid) {
            room.actions_dirty = true;
            room.history.clear();
            room.future.clear();
            room.frame_counter = 0;
            room.frame_history.clear();
            let gs = Arc::new(RwLock::new(game_state));
            room.frame_history.push(FrameSnapshot::capture(
                &gs.read().unwrap(),
                0,
                "Game start".into(),
            ));
            let display = crate::display::game_state_to_display(&gs.read().unwrap());
            let actions = actions_with_index(&gs.read().unwrap());
            room.game_state = Some(gs);
            drop(rooms);
            notify_room_clients(&data, rid);
            let ui_config = data.ui_config.lock().unwrap().clone();
            return HttpResponse::Ok().json(GameStateResponse {
                game_state: display,
                legal_actions: Some(actions),
                ui_config: Some(ui_config),
            });
        }
    }

    // No room context: use global state
    let mut state_guard = lock_state!(data.game_state, write);
    *state_guard = game_state;
    invalidate_actions(&data, None);
    data.history.lock().unwrap().clear();
    data.future.lock().unwrap().clear();
    *data.frame_counter.lock().unwrap() = 0;
    data.frame_history.lock().unwrap().clear();
    let frame0 = FrameSnapshot::capture(&state_guard, 0, "Game start".into());
    data.frame_history.lock().unwrap().push(frame0);

    let display = crate::display::game_state_to_display(&state_guard);
    let actions = actions_with_index(&state_guard);
    drop(state_guard);

    let ui_config = data.ui_config.lock().unwrap().clone();
    HttpResponse::Ok().json(GameStateResponse {
        game_state: display,
        legal_actions: Some(actions),
        ui_config: Some(ui_config),
    })
}

fn build_cached_card_registry(card_database: &CardDatabase) -> serde_json::Value {
    let mut cards_with_abilities = Vec::new();
    for (card_id, card) in card_database.cards.iter() {
        let card_data = serde_json::json!({
            "id": card_id,
            "name": card.name,
            "card_no": card.card_no,
            "card_type": format!("{:?}", card.card_type),
            "blade": card.blade,
            "abilities": card.abilities.iter().map(|ability| {
                serde_json::json!({
                    "text": ability.full_text,
                    "trigger": format!("{:?}", ability.triggers),
                    "triggers": ability.triggers,
                    "use_limit": ability.use_limit,
                    "is_null": ability.is_null
                })
            }).collect::<Vec<_>>()
        });
        cards_with_abilities.push(card_data);
    }
    serde_json::json!({
        "success": true,
        "count": card_database.cards.len(),
        "cards": cards_with_abilities
    })
}

/// Fetch the public ngrok URL using the ngrok API via raw TCP.
fn fetch_ngrok_url() -> Option<String> {
    use std::io::{Read, Write};
    let addr = "127.0.0.1:40439";
    let request = "GET /api/tunnels HTTP/1.1\r\nHost: localhost:40439\r\nConnection: close\r\n\r\n";
    if let Ok(mut stream) = std::net::TcpStream::connect(addr) {
        let _ = stream.write_all(request.as_bytes());
        let mut response = String::new();
        let _ = stream.read_to_string(&mut response);
        if let Some(body_start) = response.find("\r\n\r\n") {
            let body = &response[body_start + 4..];
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                if let Some(tunnels) = json.get("tunnels").and_then(|t| t.as_array()) {
                    for tunnel in tunnels {
                        if let Some(uri) = tunnel.get("public_url").and_then(|u| u.as_str()) {
                            return Some(uri.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Launch cloudflared synchronously. Blocks until the tunnel URL is obtained,
/// then detaches the cloudflared process. This ensures the URL is printed before
/// the HTTP server starts, so it's visible in the console.
fn launch_cloudflared_sync(port: u16) {
    let port_str = format!("{}", port);
    // Resolve cloudflared.exe relative to the engine directory (CWD at startup)
    let cwd = std::env::current_dir().unwrap_or_default();
    let cwd_bin = cwd.join("cloudflared.exe");
    let cloudflared = if cwd_bin.exists() {
        std::fs::canonicalize(&cwd_bin).unwrap_or(cwd_bin)
    } else if let Ok(path) = std::env::var("PATH") {
        let paths: Vec<std::path::PathBuf> = std::env::split_paths(&path).collect();
        paths
            .iter()
            .map(|p| p.join("cloudflared.exe"))
            .find(|p| p.exists())
            .or_else(|| {
                paths
                    .iter()
                    .map(|p| p.join("cloudflared"))
                    .find(|p| p.exists())
            })
            .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
            .unwrap_or_else(|| {
                println!("cloudflared not found in PATH or engine/ directory.");
                println!(
                    "Install from: https://developers.cloudflare.com/cloudflared/quick-start/"
                );
                std::path::PathBuf::from("cloudflared_NOT_FOUND")
            })
    } else {
        println!("cloudflared: no PATH variable found.");
        std::path::PathBuf::from("cloudflared_NOT_FOUND")
    };
    if cloudflared.to_string_lossy().contains("NOT_FOUND") {
        return;
    }

    println!("Launching cloudflared tunnel...");
    let url = format!("http://localhost:{}", port_str);
    let mut child = match std::process::Command::new(&cloudflared)
        .args(["tunnel", "--url", &url])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            println!("Failed to launch cloudflared: {}", e);
            return;
        }
    };

    // Read stderr synchronously (cloudflared outputs URL to stderr)
    use std::io::BufRead;
    let stderr = child.stderr.take().unwrap();
    for line in std::io::BufReader::new(stderr).lines() {
        if let Ok(l) = line {
            if l.contains("trycloudflare.com") {
                if let Some(start) = l.find("https://") {
                    let tunnel_url: String = l[start..]
                        .chars()
                        .take_while(|c| !c.is_whitespace() && *c != '|')
                        .collect();
                    println!("========================================");
                    println!("  Internet access: {}", tunnel_url);
                    println!("  Share this URL with your opponent!");
                    println!("========================================");
                    // Detach — cloudflared keeps running in background
                    let _ = child;
                    return;
                }
            }
        }
    }
    println!("cloudflared tunnel URL not found. Check cloudflared output above.");
    let _ = child.kill();
}

/// Launch ngrok as a subprocess that tunnels the given port.
async fn launch_ngrok(port: u16, auth_token: Option<String>) {
    let port_str = format!("{}", port);
    let mut cmd = std::process::Command::new("ngrok");
    cmd.args(["http", &port_str, "--host-header", "rewrite"]);
    if let Some(ref token) = auth_token {
        cmd.env("NGROK_AUTHTOKEN", token);
    }
    match cmd.spawn() {
        Ok(child) => {
            // Give ngrok a moment to start
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let ngrok_url = fetch_ngrok_url();
            if let Some(url) = ngrok_url {
                println!("Internet access (ngrok): {}", url);
            } else {
                println!("ngrok started; check dashboard at https://dashboard.ngrok.com");
            }
            // Detach — don't wait for ngrok to exit
            let _ = child;
        }
        Err(e) => {
            println!(
                "Failed to launch ngrok: {}. Install from https://ngrok.com/download",
                e
            );
        }
    }
}

pub async fn run_web_server() -> std::io::Result<()> {
    run_web_server_with_ngrok(None).await
}

pub async fn run_web_server_with_ngrok(ngrok_authtoken: Option<String>) -> std::io::Result<()> {
    // Enable structured verdict items in the in-game rule log when RABUKA_RULE_LOG=1
    if std::env::var("RABUKA_RULE_LOG").as_deref() == Ok("1") {
        crate::ability::debug::set_rule_log_verbose(true);
    }

    let rooms = Arc::new(Mutex::new(HashMap::new()));

    // Initialize card database (only loaded once at startup)
    let cards_path = PathBuf::from("../cards/cards.json");
    let card_database = match card_loader::CardLoader::load_cards_from_file(&cards_path) {
        Ok(cards) => Arc::new(CardDatabase::load_or_create(cards)),
        Err(e) => {
            log::debug!("Failed to load cards: {}", e);
            Arc::new(CardDatabase::new())
        }
    };

    // Load deck lists once at startup and cache them
    let deck_lists = Arc::new(deck_parser::DeckParser::parse_all_decks().unwrap_or_default());

    // Build card registry JSON once at startup
    let card_registry = Arc::new(build_cached_card_registry(&card_database));

    // Create default players
    let player1 = Player::new("p1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("p2".to_string(), "Player 2".to_string(), false);

    let game_state = Arc::new(RwLock::new(GameState::new(
        player1.clone(),
        player2.clone(),
        card_database.clone(),
    )));

    let app_state = web::Data::new(AppState {
        game_state: game_state.clone(),
        rooms: rooms.clone(),
        ui_config: Arc::new(Mutex::new(UiConfig::default())),
        card_database: card_database.clone(),
        deck_lists: deck_lists.clone(),
        card_registry: card_registry.clone(),
        history: Arc::new(Mutex::new(Vec::new())),
        future: Arc::new(Mutex::new(Vec::new())),
        custom_decks: Arc::new(Mutex::new(HashMap::new())),
        custom_energy_decks: Arc::new(Mutex::new(HashMap::new())),
        frame_counter: Arc::new(Mutex::new(0)),
        frame_history: Arc::new(Mutex::new(Vec::new())),
        cached_actions: Arc::new(Mutex::new(Vec::new())),
        actions_dirty: Arc::new(Mutex::new(true)),
        room_broadcasts: Arc::new(Mutex::new(HashMap::new())),
    });

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let bind_addr = format!("0.0.0.0:{}", port);
    let local_ip = local_ip_address::local_ip().unwrap_or_else(|_| "127.0.0.1".parse().unwrap());
    println!("Game UI: http://127.0.0.1:{}", port);
    println!("LAN access: http://{}:{}", local_ip, port);

    if let Some(token) = ngrok_authtoken {
        let ngrok_future = launch_ngrok(port, Some(token));
        tokio::spawn(ngrok_future);
    }
    // Try to launch cloudflared tunnel for internet access (silent if not found)
    launch_cloudflared_sync(port);

    // Periodic room cleanup: every 60s, remove rooms with no sessions or stale rooms
    {
        let rooms = rooms.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let mut rooms_lock = rooms.lock().unwrap();
                let before = rooms_lock.len();
                rooms_lock.retain(|id, room| {
                    let empty = room.sessions.is_empty();
                    let stale = now.saturating_sub(room.last_active) > 1200; // 20 min
                    let keep = !empty && !stale;
                    if !keep {
                        let reason = if empty { "no sessions" } else { "idle 20m" };
                        println!("[CLEANUP] Removing stale room {} ({})", id, reason);
                    }
                    keep
                });
                let removed = before - rooms_lock.len();
                if removed > 0 {
                    println!(
                        "[CLEANUP] Removed {} stale room(s), {} remaining",
                        removed,
                        rooms_lock.len()
                    );
                }
            }
        });
    }

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/api/game-state", web::get().to(get_game_state))
            .route(
                "/api/game-state/version",
                web::get().to(get_game_state_version),
            )
            .route("/api/events", web::get().to(sse_events))
            .route("/api/actions", web::get().to(get_actions))
            .route("/api/execute-action", web::post().to(execute_action))
            .route("/api/init", web::post().to(init_game))
            .route("/api/status", web::get().to(get_status))
            .route("/api/set_ai", web::post().to(set_ai))
            .route("/api/ui/config", web::post().to(set_ui_config))
            .route("/api/undo", web::post().to(undo))
            .route("/api/redo", web::post().to(redo))
            .route("/api/exec", web::post().to(exec_code))
            .route("/api/debug/rewind", web::post().to(debug_rewind))
            .route("/api/debug/redo", web::post().to(debug_redo))
            .route("/api/debug/snapshot", web::get().to(debug_snapshot))
            .route("/api/debug/dump_state", web::get().to(debug_dump_state))
            .route("/api/debug/frames", web::get().to(debug_frames))
            .route("/api/debug/dump_frames", web::get().to(debug_dump_frames))
            .route("/api/debug/conditions", web::get().to(debug_conditions))
            .route("/api/export_game", web::get().to(export_game))
            .route("/api/get_decks", web::get().to(get_decks))
            .route("/api/get_random_deck", web::get().to(get_random_deck))
            .route("/api/get_test_deck", web::get().to(get_test_deck))
            .route("/api/get_card_registry", web::get().to(get_card_registry))
            .route("/api/set_deck", web::post().to(set_deck))
            .route("/api/rooms/list", web::get().to(rooms_list))
            .route("/api/rooms/create", web::post().to(rooms_create))
            .route("/api/rooms/join", web::post().to(rooms_join))
            .route("/api/rooms/leave", web::post().to(rooms_leave))
            .service(fs::Files::new("/engine", "../engine").prefer_utf8(true))
            .service(fs::Files::new("/cards", "../cards").prefer_utf8(true))
            .service(
                fs::Files::new("/", "../web_ui")
                    .index_file("index.html")
                    .prefer_utf8(true),
            )
    })
    .bind(&bind_addr)
    .map_err(|e| {
        log::debug!("Failed to bind to address: {}", e);
        std::io::Error::new(std::io::ErrorKind::AddrInUse, e)
    })?
    .run()
    .await
}
