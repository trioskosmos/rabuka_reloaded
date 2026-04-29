use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::game_state::GameState;
use crate::player::Player;
use crate::card_loader;
use crate::card::CardDatabase;
use crate::deck_parser;
use crate::deck_builder;

#[derive(Serialize, Deserialize, Clone)]
pub struct CardDisplay {
    pub card_no: String,
    pub name: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub orientation: Option<String>,
    pub base_heart: Option<std::collections::HashMap<String, u32>>,
    pub blade: u32,
    pub id: i16,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ZoneDisplay {
    pub cards: Vec<CardDisplay>,
}

#[derive(Serialize, Deserialize)]
pub struct PlayerDisplay {
    pub hand: ZoneDisplay,
    pub energy: ZoneDisplay,
    pub stage: StageDisplay,
    pub live_zone: ZoneDisplay,
    pub success_live_card_zone: ZoneDisplay,
    pub waitroom: ZoneDisplay,
    pub discard: ZoneDisplay, // Alias for waitroom for frontend compatibility
    pub main_deck_count: usize,
    pub energy_deck_count: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StageDisplay {
    pub left_side: Option<CardDisplay>,
    pub center: Option<CardDisplay>,
    pub right_side: Option<CardDisplay>,
}

#[derive(Serialize, Deserialize)]
pub struct GameStateDisplay {
    pub turn: u32,
    pub phase: String,
    pub player1: PlayerDisplay,
    pub player2: PlayerDisplay,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionParameters {
    pub card_id: Option<i16>, // Database card ID - reliable identifier
    pub card_index: Option<usize>, // Array position - kept for backward compatibility
    pub card_indices: Option<Vec<usize>>, // For selecting multiple cards (e.g., live cards)
    pub stage_area: Option<String>,
    pub use_baton_touch: Option<bool>, // Whether to use baton touch cost reduction
    // Card grouping information for improved UI
    pub card_name: Option<String>,
    pub card_no: Option<String>,
    pub base_cost: Option<u32>,
    pub final_cost: Option<u32>,
    pub available_areas: Option<Vec<WebAreaInfo>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WebAreaInfo {
    pub area: String,
    pub available: bool,
    pub cost: u32,
    pub is_baton_touch: bool,
    pub existing_member_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Action {
    pub description: String,
    pub action_type: String,
    pub parameters: Option<ActionParameters>,
    pub index: usize,
}

#[derive(Serialize, Deserialize)]
pub struct ActionsResponse {
    pub actions: Vec<Action>,
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
    pub mode: String, // "pve" or "pvp"
    pub public: bool,
    pub created_at: u64,
    pub last_active: u64,
    pub sessions: HashMap<String, RoomSession>, // session_id -> session
    pub usernames: HashMap<i32, String>, // player_id -> username
    pub custom_decks: Option<HashMap<i32, CustomDeck>>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub game_state: Option<Arc<Mutex<GameState>>>, // Per-room game state
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
}

#[derive(Deserialize)]
pub struct JoinRoomRequest {
    pub room_id: String,
    pub username: Option<String>,
}

#[derive(Deserialize)]
pub struct InitGameRequest {
    pub deck: Option<String>,
}

#[derive(Deserialize)]
pub struct ExecCodeRequest {
    pub code: String,
}

pub struct AppState {
    pub game_state: Arc<Mutex<GameState>>,
    pub rooms: Arc<Mutex<HashMap<String, Room>>>,
}

pub fn card_to_display(card_id: i16, card_db: &crate::card::CardDatabase, orientation: Option<crate::zones::Orientation>) -> Option<CardDisplay> {
    if let Some(card) = card_db.get_card(card_id) {
        Some(CardDisplay {
            card_no: card.card_no.clone(),
            name: card.name.clone(),
            card_type: format!("{:?}", card.card_type),
            orientation: orientation.map(|o| format!("{:?}", o)),
            base_heart: card.base_heart.as_ref().map(|bh| {
                bh.hearts.iter().map(|(color, count)| {
                    let color_str = match color {
                        crate::card::HeartColor::Heart00 => "heart00",
                        crate::card::HeartColor::Heart01 => "heart01",
                        crate::card::HeartColor::Heart02 => "heart02",
                        crate::card::HeartColor::Heart03 => "heart03",
                        crate::card::HeartColor::Heart04 => "heart04",
                        crate::card::HeartColor::Heart05 => "heart05",
                        crate::card::HeartColor::Heart06 => "heart06",
                        crate::card::HeartColor::BAll => "b_all",
                        crate::card::HeartColor::Draw => "draw",
                        crate::card::HeartColor::Score => "score",
                    };
                    (color_str.to_string(), *count)
                }).collect()
            }),
            blade: card.blade,
            id: card_id,
        })
    } else {
        None
    }
}

pub fn zone_to_display_from_card_ids(cards: &[i16], card_db: &crate::card::CardDatabase) -> ZoneDisplay {
    ZoneDisplay {
        cards: cards.iter().filter_map(|&card_id| card_to_display(card_id, card_db, None)).collect(),
    }
}

pub fn stage_to_display(stage: &crate::zones::Stage, card_db: &crate::card::CardDatabase) -> StageDisplay {
    StageDisplay {
        left_side: if stage.stage[0] != -1 { card_to_display(stage.stage[0], card_db, None) } else { None },
        center: if stage.stage[1] != -1 { card_to_display(stage.stage[1], card_db, None) } else { None },
        right_side: if stage.stage[2] != -1 { card_to_display(stage.stage[2], card_db, None) } else { None },
    }
}

pub fn player_to_display(player: &crate::player::Player, card_db: &crate::card::CardDatabase) -> PlayerDisplay {
    // Calculate orientation for energy cards based on active_energy_count
    let energy_cards: Vec<(i16, Option<crate::zones::Orientation>)> = player.energy_zone.cards.iter()
        .enumerate()
        .map(|(i, &card_id)| {
            // First active_energy_count cards are active, rest are wait
            let orientation = if i < player.energy_zone.active_energy_count {
                Some(crate::zones::Orientation::Active)
            } else {
                Some(crate::zones::Orientation::Wait)
            };
            (card_id, orientation)
        })
        .collect();

    let energy_display = ZoneDisplay {
        cards: energy_cards.iter()
            .filter_map(|(card_id, orientation)| {
                card_to_display(*card_id, card_db, *orientation)
            })
            .collect(),
    };

    let waitroom_display = zone_to_display_from_card_ids(&player.waitroom.cards, card_db);
    
    PlayerDisplay {
        hand: zone_to_display_from_card_ids(&player.hand.cards, card_db),
        energy: energy_display,
        stage: stage_to_display(&player.stage, card_db),
        live_zone: zone_to_display_from_card_ids(&player.live_card_zone.cards, card_db),
        success_live_card_zone: zone_to_display_from_card_ids(&player.success_live_card_zone.cards, card_db),
        waitroom: waitroom_display.clone(),
        discard: waitroom_display, // Same as waitroom for frontend compatibility
        main_deck_count: player.main_deck.cards.len(),
        energy_deck_count: player.energy_deck.cards.len(),
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?}", game_state.current_phase),
        player1: player_to_display(&game_state.player1, &game_state.card_database),
        player2: player_to_display(&game_state.player2, &game_state.card_database),
    }
}

async fn get_game_state(data: web::Data<AppState>) -> impl Responder {
    // Check if there's an active room and use its game state
    let rooms = data.rooms.lock().unwrap();
    let game_state_to_use = if rooms.len() >= 1 {
        // If there's at least one room, use the most recently created room
        let mut latest_room = None;
        let mut latest_time = 0u64;
        
        for room in rooms.values() {
            if room.created_at > latest_time {
                latest_time = room.created_at;
                latest_room = Some(room);
            }
        }
        
        if let Some(room) = latest_room {
            if let Some(room_game_state) = &room.game_state {
                room_game_state.clone()
            } else {
                data.game_state.clone()
            }
        } else {
            data.game_state.clone()
        }
    } else {
        data.game_state.clone()
    };
    
    let game_state = match game_state_to_use.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in get_game_state: {}", e);
            return HttpResponse::InternalServerError().json("Game state mutex poisoned");
        }
    };

    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(display)
}

async fn get_actions(data: web::Data<AppState>) -> impl Responder {
    // Check if there's an active room and use its game state
    let rooms = data.rooms.lock().unwrap();
    let game_state_to_use = if rooms.len() >= 1 {
        // If there's at least one room, use the most recently created room
        let mut latest_room = None;
        let mut latest_time = 0u64;
        
        for room in rooms.values() {
            if room.created_at > latest_time {
                latest_time = room.created_at;
                latest_room = Some(room);
            }
        }
        
        if let Some(room) = latest_room {
            if let Some(room_game_state) = &room.game_state {
                room_game_state.clone()
            } else {
                data.game_state.clone()
            }
        } else {
            data.game_state.clone()
        }
    } else {
        data.game_state.clone()
    };
    
    let game_state = match game_state_to_use.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in get_actions: {}", e);
            return HttpResponse::InternalServerError().json("Game state mutex poisoned");
        }
    };

    // Generate possible actions based on current game state
    let actions = generate_possible_actions(&game_state);
    HttpResponse::Ok().json(ActionsResponse { actions })
}

fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    // Use game_setup.rs generate_possible_actions function
    let setup_actions = crate::game_setup::generate_possible_actions(game_state);
    
    // Convert game_setup Action to web_server Action with proper indexing
    setup_actions.into_iter().enumerate().map(|(index, sa)| Action {
        description: sa.description,
        action_type: sa.action_type.to_string(),
        parameters: sa.parameters.map(|p| ActionParameters {
            card_id: p.card_id,
            card_index: p.card_index,
            card_indices: p.card_indices,
            stage_area: p.stage_area.map(|a| a.to_string()),
            use_baton_touch: p.use_baton_touch,
            card_name: p.card_name,
            card_no: p.card_no,
            base_cost: p.base_cost,
            final_cost: p.final_cost,
            available_areas: p.available_areas.map(|areas| areas.into_iter().map(|ai| WebAreaInfo {
                area: ai.area.to_string(),
                available: ai.available,
                cost: ai.cost,
                is_baton_touch: ai.is_baton_touch,
                existing_member_name: ai.existing_member_name,
            }).collect()),
        }),
        index: index,
    }).collect()
}

fn is_automatic_phase(game_state: &GameState) -> bool {
    matches!(
        game_state.current_phase,
        crate::game_state::Phase::Active
            | crate::game_state::Phase::Energy
            | crate::game_state::Phase::Draw
            | crate::game_state::Phase::FirstAttackerPerformance
            | crate::game_state::Phase::SecondAttackerPerformance
            | crate::game_state::Phase::LiveVictoryDetermination
    )
}

fn is_live_card_set_phase(game_state: &GameState) -> bool {
    matches!(
        game_state.current_phase,
        crate::game_state::Phase::LiveCardSetP1Turn
            | crate::game_state::Phase::LiveCardSetP2Turn
            | crate::game_state::Phase::LiveCardSet
    )
}

fn is_human_decision_phase(game_state: &GameState) -> bool {
    match game_state.current_phase {
        // All phases are human-controlled (both players controlled by human)
        crate::game_state::Phase::RockPaperScissors
        | crate::game_state::Phase::ChooseFirstAttacker
        | crate::game_state::Phase::MulliganP1Turn
        | crate::game_state::Phase::MulliganP2Turn
        | crate::game_state::Phase::LiveCardSetP1Turn
        | crate::game_state::Phase::LiveCardSetP2Turn
        | crate::game_state::Phase::Main => true,
        crate::game_state::Phase::Mulligan => {
            // Legacy phase: both players controlled by human
            game_state.current_mulligan_player_idx == 0 || game_state.current_mulligan_player_idx == 1
        },
        crate::game_state::Phase::LiveCardSet => {
            // Legacy phase: both players controlled by human
            game_state.current_live_card_set_player == 0 || game_state.current_live_card_set_player == 1
        },
        _ => false,
    }
}

fn is_player2_decision_phase(game_state: &GameState) -> bool {
    match game_state.current_phase {
        crate::game_state::Phase::MulliganP2Turn
        | crate::game_state::Phase::LiveCardSetP2Turn => true,
        crate::game_state::Phase::Mulligan => game_state.current_mulligan_player_idx == 1,
        crate::game_state::Phase::LiveCardSet => game_state.current_live_card_set_player == 1,
        crate::game_state::Phase::Main => game_state.active_player().id == game_state.player2.id,
        _ => false,
    }
}

fn execute_player2_ai_action(game_state: &mut GameState) -> Result<bool, String> {
    if !is_player2_decision_phase(game_state) {
        return Ok(false);
    }

    let actions = crate::game_setup::generate_possible_actions(game_state);
    if actions.is_empty() {
        return Err(format!(
            "Player 2 reached {:?} with no legal actions",
            game_state.current_phase
        ));
    }

    let ai = crate::bot::ai::AIPlayer::new("Player2AI".to_string());
    let chosen_index = ai.choose_action(&actions);
    let chosen_action = actions
        .get(chosen_index)
        .ok_or_else(|| format!("Player 2 chose invalid action index {}", chosen_index))?;

    crate::turn::TurnEngine::execute_main_phase_action(
        game_state,
        &chosen_action.action_type,
        chosen_action.parameters.as_ref().and_then(|p| p.card_id),
        chosen_action
            .parameters
            .as_ref()
            .and_then(|p| p.card_indices.as_ref())
            .cloned(),
        chosen_action.parameters.as_ref().and_then(|p| p.stage_area),
        chosen_action.parameters.as_ref().and_then(|p| p.use_baton_touch),
    )?;

    Ok(true)
}

fn settle_single_player_state(game_state: &mut GameState) -> Result<(), String> {
    // Keep auto-advancing until we reach a human decision phase
    loop {
        if is_automatic_phase(game_state) {
            let old_phase = game_state.current_phase.clone();
            crate::turn::TurnEngine::advance_phase(game_state);
            println!("DEBUG: Auto-advanced from {:?} to {:?}", old_phase, game_state.current_phase);
        } else if is_live_card_set_phase(game_state) {
            // Live card set phases are manual - don't auto-advance
            println!("DEBUG: Live card set phase reached, stopping auto-advance");
            break;
        } else {
            // Reached a human decision phase, stop auto-advancing
            break;
        }
    }
    Ok(())
}

async fn execute_action(
    data: web::Data<AppState>,
    req: web::Json<ExecuteActionRequest>,
) -> impl Responder {
        
    // Check if there's an active room and use its game state
    let rooms = data.rooms.lock().unwrap();
    let game_state_to_use = if rooms.len() >= 1 {
        // If there's at least one room, use the most recently created room
        let mut latest_room = None;
        let mut latest_time = 0u64;
        
        for room in rooms.values() {
            if room.created_at > latest_time {
                latest_time = room.created_at;
                latest_room = Some(room);
            }
        }
        
        if let Some(room) = latest_room {
            if let Some(room_game_state) = &room.game_state {
                room_game_state.clone()
            } else {
                data.game_state.clone()
            }
        } else {
            data.game_state.clone()
        }
    } else {
        data.game_state.clone()
    };
    
    let mut game_state = match game_state_to_use.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in execute_action: {}", e);
            return HttpResponse::InternalServerError().json("Game state mutex poisoned");
        }
    };
    
    // Parse action type from request
    let action_type = req.action_type.as_ref()
        .and_then(|t| t.parse::<crate::game_setup::ActionType>().ok())
        .unwrap_or_else(|| crate::game_setup::ActionType::Pass);
    
    // Execute the action
    let result = crate::turn::TurnEngine::execute_main_phase_action(
        &mut game_state,
        &action_type,
        req.card_id,
        req.card_indices.as_ref().cloned(),
        req.stage_area.as_ref()
            .and_then(|s| s.parse::<crate::zones::MemberArea>().ok()),
        req.use_baton_touch,
    );
    
        
    match result {
        Ok(_) => {
            // For human decision phases like ChooseFirstAttacker, the phase transition 
            // happens in the action handler itself, not in settle_single_player_state
            if let Err(e) = settle_single_player_state(&mut game_state) {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "error": e
                }));
            }

            let display = game_state_to_display(&game_state);
            
            // Include legal actions in the response
            let actions = generate_possible_actions(&game_state);
            let mut response = serde_json::to_value(&display).unwrap_or_default();
            response["legal_actions"] = serde_json::to_value(&actions).unwrap_or(serde_json::Value::Array(vec![]));
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
                        HttpResponse::BadRequest().json(serde_json::json!({
                "error": e
            }))
        }
    }
}

async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in get_status: {}", e);
            return HttpResponse::InternalServerError().json("Game state mutex poisoned");
        }
    };
    let members = game_state.card_database.cards.len();
    HttpResponse::Ok().json(serde_json::json!({
        "status": "rust_server",
        "members": members,
        "lives": 0,
        "instance_id": 1
    }))
}

async fn set_ai(_data: web::Data<AppState>, _req: web::Json<serde_json::Value>) -> impl Responder {
    // Placeholder for AI mode setting
    HttpResponse::Ok().json(serde_json::json!({
        "success": true
    }))
}

async fn undo(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in undo: {}", e);
            return HttpResponse::InternalServerError().json("Game state mutex poisoned");
        }
    };
    
    match game_state.undo() {
        Ok(_) => {
            if let Err(e) = settle_single_player_state(&mut game_state) {
                eprintln!("Single-player settle error after undo: {}", e);
                return HttpResponse::BadRequest().json(serde_json::json!({"error": e}));
            }
            let display = game_state_to_display(&game_state);
            HttpResponse::Ok().json(display)
        }
        Err(e) => {
            HttpResponse::BadRequest().json(e)
        }
    }
}

async fn redo(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in redo: {}", e);
            return HttpResponse::InternalServerError().json("Game state mutex poisoned");
        }
    };

    match game_state.redo() {
        Ok(_) => {
            if let Err(e) = settle_single_player_state(&mut game_state) {
                eprintln!("Single-player settle error after redo: {}", e);
                return HttpResponse::BadRequest().json(serde_json::json!({"error": e}));
            }
            let display = game_state_to_display(&game_state);
            HttpResponse::Ok().json(display)
        }
        Err(e) => {
            HttpResponse::BadRequest().json(e)
        }
    }
}

async fn exec_code(
    data: web::Data<AppState>,
    req: web::Json<ExecCodeRequest>,
) -> impl Responder {
    let mut game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in exec_code: {}", e);
            return HttpResponse::InternalServerError().json("Game state mutex poisoned");
        }
    };

    // Parse and execute the code
    let code = &req.code;

    // Simple parsing for cheat commands
    // Format: player_idx = N; operations...
    if code.contains("draw_energy") {
        // Extract player_idx
        let player_idx = code.lines()
            .find(|l| l.contains("player_idx"))
            .and_then(|l| l.split('=').nth(1))
            .and_then(|v| v.trim().split(';').next())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        // Extract amount
        let amount = code.lines()
            .find(|l| l.contains("amount"))
            .and_then(|l| l.split('=').nth(1))
            .and_then(|v| v.trim().split(';').next())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1);

        // Execute draw_energy amount times
        let player = if player_idx == 0 {
            &mut game_state.player1
        } else {
            &mut game_state.player2
        };

        for _ in 0..amount {
            let _ = player.draw_energy();
        }
    } else if code.contains("add_card") && code.contains("card_no") {
        // Extract player_idx
        let player_idx = code.lines()
            .find(|l| l.contains("player_idx"))
            .and_then(|l| l.split('=').nth(1))
            .and_then(|v| v.trim().split(';').next())
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0);

        // Extract card_no
        let card_no = code.lines()
            .find(|l| l.contains("card_no"))
            .and_then(|l| l.split('=').nth(1))
            .and_then(|v| v.trim().split(';').next())
            .map(|v| v.trim().trim_matches('"'))
            .unwrap_or("");

        // Look up card and add to hand
        if let Some(card_id) = game_state.card_database.get_card_id(card_no) {
            let player = if player_idx == 0 {
                &mut game_state.player1
            } else {
                &mut game_state.player2
            };
            player.hand.add_card(card_id);
        }
    }

    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "state": display
    }))
}

async fn debug_rewind(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in debug_rewind: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"success": false}));
        }
    };
    
    match game_state.undo() {
        Ok(_) => {
            HttpResponse::Ok().json(serde_json::json!({"success": true}))
        }
        Err(e) => {
            HttpResponse::BadRequest().json(serde_json::json!({"success": false, "error": e}))
        }
    }
}

async fn debug_redo(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in debug_redo: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"success": false}));
        }
    };
    
    match game_state.redo() {
        Ok(_) => {
            HttpResponse::Ok().json(serde_json::json!({"success": true}))
        }
        Err(e) => {
            HttpResponse::BadRequest().json(serde_json::json!({"success": false, "error": e}))
        }
    }
}

async fn debug_snapshot(data: web::Data<AppState>) -> impl Responder {
    let game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in debug_snapshot: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"success": false}));
        }
    };
    
    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "state": display}))
}

async fn debug_dump_state(data: web::Data<AppState>) -> impl Responder {
    let game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in debug_dump_state: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"success": false}));
        }
    };
    
    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "state": display}))
}

async fn export_game(data: web::Data<AppState>) -> impl Responder {
    let game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in export_game: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"success": false}));
        }
    };
    
    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "game_state": display}))
}

async fn get_decks(_data: web::Data<AppState>) -> impl Responder {
    let decks = vec![
        "Aqours Cup".to_string(),
        "Muse Cup".to_string(),
        "Nijigaku Cup".to_string(),
        "Liella Cup".to_string(),
        "Hasunosora Cup".to_string(),
        "Fade Deck".to_string(),
    ];
    HttpResponse::Ok().json(serde_json::json!({"success": true, "decks": decks}))
}

async fn get_random_deck(_data: web::Data<AppState>) -> impl Responder {
    let decks = vec![
        "Aqours Cup".to_string(),
        "Muse Cup".to_string(),
        "Nijigaku Cup".to_string(),
        "Liella Cup".to_string(),
        "Hasunosora Cup".to_string(),
        "Fade Deck".to_string(),
    ];
    use rand::seq::SliceRandom;
    let random_deck = decks.choose(&mut rand::thread_rng()).unwrap_or(&decks[0]);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "deck": random_deck}))
}

async fn get_test_deck(_data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"success": true, "deck": "Aqours Cup"}))
}

async fn get_card_registry(data: web::Data<AppState>) -> impl Responder {
    let game_state = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in get_card_registry: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({"success": false}));
        }
    };
    
    let members = game_state.card_database.cards.len();
    HttpResponse::Ok().json(serde_json::json!({"success": true, "count": members}))
}

async fn rooms_create(data: web::Data<AppState>, req: web::Json<CreateRoomRequest>) -> impl Responder {
    // Skip card database loading for now to avoid deserialization errors
    println!("DEBUG: rooms_create called");
    println!("DEBUG: rooms_create called");
    let room_id = Uuid::new_v4().to_string().to_uppercase();
    let mode = req.mode.clone().unwrap_or_else(|| "pve".to_string());
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
            decks.insert(0, CustomDeck {
                main: p0_deck,
                energy: req.p0_energy.clone().unwrap_or_default(),
            });
        }
        if let Some(p1_deck) = req.p1_deck.clone() {
            decks.insert(1, CustomDeck {
                main: p1_deck,
                energy: req.p1_energy.clone().unwrap_or_default(),
            });
        }
        custom_decks = Some(decks);
    }
    
    // Initialize FRESH game state for the room with proper setup
    // Get card database from app state
    let card_database = {
        let app_state = data.game_state.lock().unwrap();
        app_state.card_database.clone()
    };
    
    // Create default players
    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    let mut fresh_game_state = GameState::new(player1, player2, card_database);
    crate::game_setup::setup_game(&mut fresh_game_state);
    println!("DEBUG: Fresh room game state initialized with phase: {:?}", fresh_game_state.current_phase);
    let room_game_state = Arc::new(Mutex::new(fresh_game_state));
    
    let room = Room {
        room_id: room_id.clone(),
        mode: mode.clone(),
        public,
        created_at: now,
        last_active: now,
        sessions: HashMap::new(),
        usernames: HashMap::new(),
        custom_decks,
        game_state: Some(room_game_state),
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
    
    {
        let mut rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(&room_id) {
            room.sessions.insert(session_id.clone(), RoomSession {
                session_id: session_id.clone(),
                player_id,
                username: username.clone(),
            });
            if let Some(name) = username {
                room.usernames.insert(player_id, name);
            }
            room.last_active = now;
        }
    }
    
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

async fn rooms_join(data: web::Data<AppState>, req: web::Json<JoinRoomRequest>) -> impl Responder {
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
                        room.sessions.insert(session_id.clone(), RoomSession {
                            session_id: session_id.clone(),
                            player_id,
                            username: Some(name.clone()),
                        });
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
            let taken_pids: std::collections::HashSet<i32> = room.sessions.values()
                .map(|s| s.player_id)
                .collect();
            
            if !taken_pids.contains(&0) {
                player_id = 0;
            } else if !taken_pids.contains(&1) {
                player_id = 1;
            }
            
            if player_id >= 0 {
                room.sessions.insert(session_id.clone(), RoomSession {
                    session_id: session_id.clone(),
                    player_id,
                    username: username.clone(),
                });
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
        rooms.get(&room_id).map(|r| r.mode.clone()).unwrap_or_else(|| "pve".to_string())
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

async fn rooms_leave(data: web::Data<AppState>, req: web::Json<serde_json::Value>) -> impl Responder {
    let room_id = req.get("room_id").and_then(|v| v.as_str()).unwrap_or("").to_uppercase();
    let session_token = req.get("session_id").and_then(|v| v.as_str());
    
    if room_id.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Room ID required"
        }));
    }
    
    {
        let mut rooms = data.rooms.lock().unwrap();
        if let Some(room) = rooms.get_mut(&room_id) {
            if let Some(token) = session_token {
                room.sessions.remove(token);
            }
            
            room.last_active = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            // Delete room if no sessions
            if room.sessions.is_empty() {
                rooms.remove(&room_id);
            }
        } else {
            return HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "error": "Room not found"
            }));
        }
    }
    
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

async fn rooms_list(data: web::Data<AppState>) -> impl Responder {
    let rooms = data.rooms.lock().unwrap();
    let public_rooms: Vec<_> = rooms.values()
        .filter(|r| r.public)
        .map(|r| {
            let occupied_slots = r.sessions.values()
                .filter(|s| s.player_id >= 0)
                .map(|s| s.player_id)
                .collect::<std::collections::HashSet<_>>()
                .len();
            
            serde_json::json!({
                "room_id": r.room_id,
                "mode": r.mode,
                "players": occupied_slots,
                "created_at": r.created_at
            })
        })
        .collect();
    
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "rooms": public_rooms
    }))
}

async fn init_game(data: web::Data<AppState>, req: web::Json<InitGameRequest>) -> impl Responder {
    // Load cards from cards.json
    let cards_path = PathBuf::from("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(&cards_path) {
        Ok(cards) => {
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return HttpResponse::InternalServerError().json("Failed to load cards");
        }
    };

    // Load deck lists
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return HttpResponse::InternalServerError().json("Failed to load decks");
        }
    };

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
    let selected_deck_name = req.deck.as_deref();
    let deck_index = if let Some(name) = selected_deck_name {
        if let Some(file_name) = deck_name_mapping.get(name) {
            deck_lists.iter().position(|d| d.name == *file_name)
        } else {
            None
        }
    } else {
        None
    };

    let deck1 = if let Some(idx) = deck_index {
        &deck_lists[idx]
    } else {
        &deck_lists[0]
    };
    let deck2 = deck1; // Use same deck for both players

    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);

    // Create CardDatabase from loaded cards - convert HashMap values to Vec
    let card_vec: Vec<crate::card::Card> = cards.into_values().collect();
    let card_database = Arc::new(crate::card::CardDatabase::load_or_create(card_vec));

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 1: {}", e);
            return HttpResponse::InternalServerError().json("Failed to build deck for Player 1");
        }
    };

    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers2) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 2: {}", e);
            return HttpResponse::InternalServerError().json("Failed to build deck for Player 2");
        }
    };

    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck, &card_database);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck, &card_database);

    // Create fresh players
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);

    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);

    // Create fresh game state with CardDatabase
    let mut game_state = GameState::new(player1, player2, card_database);
    
    // Setup game (Rule 6.2)
    crate::game_setup::setup_game(&mut game_state);
    // Don't call settle_single_player_state here - game should start in RockPaperScissors phase
    println!("DEBUG: init_game complete, phase: {:?}", game_state.current_phase);
    
    // Replace the game state in the mutex
    let mut state_guard = match data.game_state.lock() {
        Ok(guard) => guard,
        Err(e) => {
            eprintln!("Mutex poisoned in init_game: {}", e);
            return HttpResponse::InternalServerError().json("Game state mutex poisoned");
        }
    };
    *state_guard = game_state;
    
    let display = game_state_to_display(&state_guard);
    HttpResponse::Ok().json(display)
}

pub async fn run_web_server() -> std::io::Result<()> {
    println!("DEBUG: Starting web server function...");
    let rooms = Arc::new(Mutex::new(HashMap::new()));
    
    // Initialize card database
    let cards_path = PathBuf::from("../cards/cards.json");
    let _cards = match card_loader::CardLoader::load_cards_from_file(&cards_path) {
        Ok(cards) => {
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            std::collections::HashMap::new()
        }
    };
    
    let card_database = Arc::new(CardDatabase::new());
    
    // Create default players
    let player1 = Player::new("0".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("1".to_string(), "Player 2".to_string(), false);
    
    let game_state = Arc::new(Mutex::new(GameState::new(player1.clone(), player2.clone(), card_database.clone())));
    
    let app_state = web::Data::new(AppState {
        game_state: game_state.clone(),
        rooms: rooms.clone(),
    });
    println!("DEBUG: App state created...");

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/api/game-state", web::get().to(get_game_state))
            .route("/api/actions", web::get().to(get_actions))
            .route("/api/execute-action", web::post().to(execute_action))
            .route("/api/init", web::post().to(init_game))
            .route("/api/status", web::get().to(get_status))
            .route("/api/set_ai", web::post().to(set_ai))
            .route("/api/undo", web::post().to(undo))
            .route("/api/redo", web::post().to(redo))
            .route("/api/exec", web::post().to(exec_code))
            .route("/api/debug/rewind", web::post().to(debug_rewind))
            .route("/api/debug/redo", web::post().to(debug_redo))
            .route("/api/debug/snapshot", web::get().to(debug_snapshot))
            .route("/api/debug/dump_state", web::get().to(debug_dump_state))
            .route("/api/export_game", web::get().to(export_game))
            .route("/api/get_decks", web::get().to(get_decks))
            .route("/api/get_random_deck", web::get().to(get_random_deck))
            .route("/api/get_test_deck", web::get().to(get_test_deck))
            .route("/api/get_card_registry", web::get().to(get_card_registry))
            .route("/api/rooms/create", web::post().to(rooms_create))
            .route("/api/rooms/join", web::post().to(rooms_join))
            .route("/api/rooms/leave", web::post().to(rooms_leave))
            .route("/api/debug/dump_state", web::get().to(debug_dump_state))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
