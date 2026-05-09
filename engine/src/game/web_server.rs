use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_files as fs;
use actix_cors::Cors;
use std::sync::{Arc, Mutex, RwLock};
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
use crate::game_setup::{ActionParameters, ActionType};
use crate::ability::resolver::AbilityResolver;
use crate::display;

pub use crate::display::{CardDisplay, ZoneDisplay, PlayerDisplay, StageDisplay, GameStateDisplay};



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

    pub mode: String, // "pve" or "pvp"

    pub public: bool,

    pub created_at: u64,

    pub last_active: u64,

    pub sessions: HashMap<String, RoomSession>, // session_id -> session

    pub usernames: HashMap<i32, String>, // player_id -> username

    pub custom_decks: Option<HashMap<i32, CustomDeck>>,

    #[serde(skip)]

    #[allow(dead_code)]

    pub game_state: Option<Arc<RwLock<GameState>>>, // Per-room game state

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

}



fn resolve_game_state_arc(data: &AppState) -> Arc<RwLock<GameState>> {
    let rooms = data.rooms.lock().unwrap();
    if rooms.is_empty() {
        return data.game_state.clone();
    }
    let latest = rooms.values().max_by_key(|r| r.created_at).unwrap();
    latest.game_state.as_ref().unwrap_or(&data.game_state).clone()
}

macro_rules! lock_state {
    ($arc:expr, $mode:ident) => {{
        match ($arc).$mode() {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("Lock poisoned: {}", e);
                return HttpResponse::InternalServerError().json("Internal error");
            }
        }
    }};
}

async fn get_game_state(data: web::Data<AppState>) -> impl Responder {
    let gs_arc = resolve_game_state_arc(&data);
    let game_state = lock_state!(gs_arc, read);
    let display = crate::display::game_state_to_display(&game_state);
    let actions = crate::game_setup::generate_possible_actions(&game_state).into_iter().enumerate().map(|(i, a)| ActionIndex {
        description: a.description,
        action_type: a.action_type.to_string(),
        parameters: a.parameters,
        index: i,
    }).collect::<Vec<_>>();
    drop(game_state);

    let ui_config = data.ui_config.lock().unwrap().clone();
    HttpResponse::Ok().json(GameStateResponse { game_state: display, legal_actions: Some(actions), ui_config: Some(ui_config) })
}



async fn get_actions(data: web::Data<AppState>) -> impl Responder {
    let gs_arc = resolve_game_state_arc(&data);
    let game_state = lock_state!(gs_arc, read);
    let actions = crate::game_setup::generate_possible_actions(&game_state).into_iter().enumerate().map(|(i, a)| ActionIndex {
        description: a.description,
        action_type: a.action_type.to_string(),
        parameters: a.parameters,
        index: i,
    }).collect::<Vec<_>>();
    HttpResponse::Ok().json(serde_json::json!({ "actions": actions }))
}




fn actions_with_index(game_state: &GameState) -> Vec<ActionIndex> {
    crate::game_setup::generate_possible_actions(game_state).into_iter().enumerate().map(|(i, a)| ActionIndex {
        description: a.description,
        action_type: a.action_type.to_string(),
        parameters: a.parameters,
        index: i,
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

    )
}



fn settle_single_player_state(game_state: &mut GameState) -> Result<(), String> {

    loop {

        // If the player needs to make a choice, stop and wait for their input
        if game_state.pending_choice.is_some() { break; }

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
    let gs_arc = resolve_game_state_arc(&data);
    let snapshot = lock_state!(gs_arc, read).clone();
    let mut game_state = lock_state!(gs_arc, write);

    let action_type = req.action_type.as_ref()
        .and_then(|t| t.parse::<ActionType>().ok())
        .unwrap_or(ActionType::Pass);

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
            if let Err(e) = settle_single_player_state(&mut game_state) {
                return HttpResponse::BadRequest().json(serde_json::json!({ "error": e }));
            }

            data.history.lock().unwrap().push(snapshot);
            data.future.lock().unwrap().clear();

            let display = crate::display::game_state_to_display(&game_state);
            let actions = actions_with_index(&game_state);
            HttpResponse::Ok().json(GameStateResponse { game_state: display, legal_actions: Some(actions), ui_config: None })
        }
        Err(e) => {
            HttpResponse::BadRequest().json(serde_json::json!({ "error": e }))
        }
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



async fn set_ui_config(data: web::Data<AppState>, req: web::Json<SetUiConfigRequest>) -> impl Responder {
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



async fn undo(data: web::Data<AppState>) -> impl Responder {
    let snapshot = {
        let mut history = data.history.lock().unwrap();
        if let Some(prev) = history.pop() {
            prev
        } else {
            return HttpResponse::BadRequest().json("No history to undo");
        }
    };
    let gs_arc = resolve_game_state_arc(&data);
    let mut game_state = lock_state!(gs_arc, write);
    data.future.lock().unwrap().push(game_state.clone());
    *game_state = snapshot;

    if let Err(e) = settle_single_player_state(&mut game_state) {
        eprintln!("Single-player settle error after undo: {}", e);
    }
    let display = crate::display::game_state_to_display(&game_state);
    drop(game_state);
    let ui_config = data.ui_config.lock().unwrap().clone();
    HttpResponse::Ok().json(GameStateResponse { game_state: display, legal_actions: None, ui_config: Some(ui_config) })
}

async fn redo(data: web::Data<AppState>) -> impl Responder {
    let snapshot = {
        let mut future = data.future.lock().unwrap();
        if let Some(next) = future.pop() {
            next
        } else {
            return HttpResponse::BadRequest().json("No future to redo");
        }
    };
    let gs_arc = resolve_game_state_arc(&data);
    let mut game_state = lock_state!(gs_arc, write);
    data.history.lock().unwrap().push(game_state.clone());
    *game_state = snapshot;

    if let Err(e) = settle_single_player_state(&mut game_state) {
        eprintln!("Single-player settle error after redo: {}", e);
    }
    let display = crate::display::game_state_to_display(&game_state);
    drop(game_state);
    let ui_config = data.ui_config.lock().unwrap().clone();
    HttpResponse::Ok().json(GameStateResponse { game_state: display, legal_actions: None, ui_config: Some(ui_config) })
}




async fn exec_code(
    data: web::Data<AppState>,
    req: web::Json<ExecCodeRequest>,
) -> impl Responder {
    let mut game_state = lock_state!(data.game_state, write);



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



    let display = crate::display::game_state_to_display(&game_state);
    let actions = actions_with_index(&game_state);
    let mut response = serde_json::to_value(display).unwrap_or_default();
    response["legal_actions"] = serde_json::to_value(&actions).unwrap_or(serde_json::Value::Array(vec![]));

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "state": response
    }))
}



async fn debug_rewind(data: web::Data<AppState>) -> impl Responder {
    let snapshot = {
        let mut history = data.history.lock().unwrap();
        if let Some(prev) = history.pop() {
            prev
        } else {
            return HttpResponse::BadRequest().json(serde_json::json!({"success": false, "error": "No history"}));
        }
    };
    let gs_arc = resolve_game_state_arc(&data);
    let mut game_state = lock_state!(gs_arc, write);
    data.future.lock().unwrap().push(game_state.clone());
    *game_state = snapshot;
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

async fn debug_redo(data: web::Data<AppState>) -> impl Responder {
    let snapshot = {
        let mut future = data.future.lock().unwrap();
        if let Some(next) = future.pop() {
            next
        } else {
            return HttpResponse::BadRequest().json(serde_json::json!({"success": false, "error": "No future"}));
        }
    };
    let gs_arc = resolve_game_state_arc(&data);
    let mut game_state = lock_state!(gs_arc, write);
    data.history.lock().unwrap().push(game_state.clone());
    *game_state = snapshot;
    HttpResponse::Ok().json(serde_json::json!({"success": true}))
}

async fn debug_snapshot(data: web::Data<AppState>) -> impl Responder {
    let game_state = lock_state!(data.game_state, read);
    let display = crate::display::game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "state": display}))
}



async fn debug_dump_state(data: web::Data<AppState>) -> impl Responder {
    let game_state = lock_state!(data.game_state, read);
    let display = crate::display::game_state_to_display(&game_state);
    HttpResponse::Ok().json(serde_json::json!({"success": true, "state": display}))
}



async fn debug_conditions(data: web::Data<AppState>) -> impl Responder {
    let game_state = lock_state!(data.game_state, read);
    let mut results = Vec::new();

    let card_db = &game_state.card_database;

    for (player_idx, player) in [&game_state.player1, &game_state.player2].iter().enumerate() {

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

                if card_id < 0 { continue; }

                if let Some(card) = card_db.get_card(card_id) {

                    for (ability_idx, ability) in card.abilities.iter().enumerate() {

                        if let Some(ref effect) = ability.effect {

                            let condition_fields: [(&str, &Option<crate::card::Condition>); 4] = [

                                ("activation_condition_parsed", &effect.activation_condition_parsed),

                                ("condition", &effect.condition),

                                ("alternative_condition", &effect.compound.alternative_condition),

                                ("result_condition", &effect.compound.result_condition),

                            ];

                            for &(field_name, ref condition_opt) in &condition_fields {

                                if let Some(ref condition) = *condition_opt {

                                    results.push((player_idx, zone_name, card_id, card.name.clone(), ability_idx, field_name, condition.clone()));

                                }

                            }

                        }

                    }

                }

            }

        }

    }

    let mut state_clone = game_state.clone();

    drop(game_state);

    let resolver = AbilityResolver::new(&mut state_clone);

    let evaluated: Vec<serde_json::Value> = results.into_iter().map(|(player_idx, zone_name, card_id, card_name, ability_idx, field_name, condition)| {

        let result = resolver.evaluate_condition(&condition);

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

        })

    }).collect();

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
                    c.next().map(|f| f.to_uppercase().to_string() + c.as_str()).unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn parse_deck_text(content: &str) -> Vec<String> {
    content.lines()
        .filter(|l| !l.trim().is_empty())
        .flat_map(|l| {
            let parts: Vec<&str> = l.split(" x ").collect();
            if parts.len() != 2 { return Vec::new(); }
            let (card_no, quantity) = if let Ok(q) = parts[0].trim().parse::<u32>() {
                (parts[1].trim().to_string(), q)
            } else if let Ok(q) = parts[1].trim().parse::<u32>() {
                (parts[0].trim().to_string(), q)
            } else { return Vec::new(); };
            if card_no.contains('-') {
                std::iter::repeat(card_no).take(quantity as usize).collect()
            } else { Vec::new() }
        })
        .collect()
}

async fn get_decks(_data: web::Data<AppState>) -> impl Responder {
    let decks: Vec<serde_json::Value> = deck_files().into_iter().map(|path| {
        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let name = deck_name_from_path(&path);
        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let card_count = content.lines()
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
    }).collect();
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "decks": decks }))
}

async fn get_random_deck(_data: web::Data<AppState>) -> impl Responder {
    let files = deck_files();
    if files.is_empty() {
        return HttpResponse::NotFound().json(serde_json::json!({ "success": false, "error": "No decks found" }));
    }
    use rand::seq::SliceRandom;
    let chosen = files.choose(&mut rand::thread_rng()).unwrap();
    let content = std::fs::read_to_string(chosen).unwrap_or_default();
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "content": content, "energy": [] }))
}

async fn get_test_deck(_data: web::Data<AppState>) -> impl Responder {
    let path = PathBuf::from("../web_ui/decks/aqours_cup.txt");
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "content": content }))
}

async fn set_deck(data: web::Data<AppState>, req: web::Json<serde_json::Value>) -> impl Responder {
    let player = req.get("player").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let card_numbers: Vec<String> = if let Some(arr) = req.get("deck").and_then(|v| v.as_array()) {
        arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
    } else {
        let deck_content = req.get("deck").and_then(|v| v.as_str()).unwrap_or("");
        if deck_content.is_empty() {
            Vec::new()
        } else {
            deck_parser::DeckParser::parse_deck_content(deck_content)
        }
    };
    if !card_numbers.is_empty() {
        data.custom_decks.lock().unwrap().insert(player, card_numbers);
    }
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "status": "ok" }))
}

async fn rooms_list(data: web::Data<AppState>) -> impl Responder {
    let rooms = data.rooms.lock().unwrap();
    let public_rooms: Vec<serde_json::Value> = rooms.values()
        .filter(|r| r.public)
        .map(|r| serde_json::json!({
            "room_id": r.room_id,
            "mode": r.mode,
            "player_count": r.sessions.len(),
            "created_at": r.created_at,
        }))
        .collect();
    HttpResponse::Ok().json(serde_json::json!({ "success": true, "rooms": public_rooms }))
}



async fn get_card_registry(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(&*data.card_registry)
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

    let card_database = data.card_database.clone();

    // Create default players

    let player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);

    let player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    

    let mut fresh_game_state = GameState::new(player1, player2, card_database);

    crate::game_setup::setup_game(&mut fresh_game_state);

    println!("DEBUG: Fresh room game state initialized with phase: {:?}", fresh_game_state.current_phase);

    let room_game_state = Arc::new(RwLock::new(fresh_game_state));

    

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





async fn init_game(data: web::Data<AppState>, req: Option<web::Json<InitGameRequest>>) -> impl Responder {

    let card_database = data.card_database.clone();
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



    // Check for custom decks set via set_deck endpoint
    let (card_numbers1, card_numbers2) = {
        let mut custom = data.custom_decks.lock().unwrap();
        if custom.contains_key(&0) || custom.contains_key(&1) {
            let p0 = custom.remove(&0).unwrap_or_default();
            let p1 = custom.remove(&1).unwrap_or_else(|| p0.clone());
            (p0, p1)
        } else {
            let deck = if let Some(idx) = deck_index { &deck_lists[idx] } else { &deck_lists[0] };
            (deck_parser::DeckParser::deck_list_to_card_numbers(deck), deck_parser::DeckParser::deck_list_to_card_numbers(deck))
        }
    };

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

    let mut state_guard = lock_state!(data.game_state, write);
    *state_guard = game_state;
    data.history.lock().unwrap().clear();
    data.future.lock().unwrap().clear();

    let display = crate::display::game_state_to_display(&state_guard);
    let actions = actions_with_index(&state_guard);
    drop(state_guard);

    let ui_config = data.ui_config.lock().unwrap().clone();
    HttpResponse::Ok().json(GameStateResponse { game_state: display, legal_actions: Some(actions), ui_config: Some(ui_config) })

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

pub async fn run_web_server() -> std::io::Result<()> {

    let rooms = Arc::new(Mutex::new(HashMap::new()));

    // Initialize card database (only loaded once at startup)
    let cards_path = PathBuf::from("../cards/cards.json");
    let card_database = match card_loader::CardLoader::load_cards_from_file(&cards_path) {
        Ok(cards) => Arc::new(CardDatabase::load_or_create(cards)),
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            Arc::new(CardDatabase::new())
        }
    };

    // Load deck lists once at startup and cache them
    let deck_lists = Arc::new(
        deck_parser::DeckParser::parse_all_decks().unwrap_or_default()
    );

    // Build card registry JSON once at startup
    let card_registry = Arc::new(build_cached_card_registry(&card_database));

    // Create default players
    let player1 = Player::new("0".to_string(), "Player 1".to_string(), true);
    let player2 = Player::new("1".to_string(), "Player 2".to_string(), false);

    let game_state = Arc::new(RwLock::new(GameState::new(player1.clone(), player2.clone(), card_database.clone())));

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
    });

    println!("Game UI: http://127.0.0.1:8080");



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
            .route("/api/ui/config", web::post().to(set_ui_config))
            .route("/api/undo", web::post().to(undo))
            .route("/api/redo", web::post().to(redo))
            .route("/api/exec", web::post().to(exec_code))
            .route("/api/debug/rewind", web::post().to(debug_rewind))
            .route("/api/debug/redo", web::post().to(debug_redo))
            .route("/api/debug/snapshot", web::get().to(debug_snapshot))
            .route("/api/debug/dump_state", web::get().to(debug_dump_state))
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
            .service(fs::Files::new("/engine", "../engine"))
            .service(fs::Files::new("/", "../web_ui/dist").index_file("index.html"))
    })

    .bind("127.0.0.1:8080")

    .map_err(|e| {

        eprintln!("Failed to bind to address: {}", e);

        std::io::Error::new(std::io::ErrorKind::AddrInUse, e)

    })?

    .run()

    .await

}

