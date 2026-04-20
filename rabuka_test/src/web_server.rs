use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use actix_files as fs;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

use crate::game_state::GameState;

#[derive(Serialize, Deserialize, Clone)]
pub struct CardDisplay {
    pub card_no: String,
    pub name: String,
    #[serde(rename = "type")]
    pub card_type: String,
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

#[derive(Serialize, Deserialize)]
pub struct ActionParameters {
    pub card_index: Option<usize>,
    pub card_indices: Option<Vec<usize>>, // For selecting multiple cards (e.g., live cards)
    pub stage_area: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Action {
    pub description: String,
    pub action_type: String,
    pub parameters: Option<ActionParameters>,
}

#[derive(Serialize, Deserialize)]
pub struct ActionsResponse {
    pub actions: Vec<Action>,
}

#[derive(Deserialize)]
pub struct ExecuteActionRequest {
    pub action_index: usize,
}

pub struct AppState {
    pub game_state: Arc<Mutex<GameState>>,
}

pub fn card_to_display(card: &crate::card::Card) -> CardDisplay {
    CardDisplay {
        card_no: card.card_no.clone(),
        name: card.name.clone(),
        card_type: format!("{:?}", card.card_type),
    }
}

pub fn zone_to_display(cards: &[crate::card::Card]) -> ZoneDisplay {
    ZoneDisplay {
        cards: cards.iter().map(card_to_display).collect(),
    }
}

pub fn stage_to_display(stage: &crate::zones::Stage) -> StageDisplay {
    StageDisplay {
        left_side: stage.left_side.as_ref().map(|c| card_to_display(&c.card)),
        center: stage.center.as_ref().map(|c| card_to_display(&c.card)),
        right_side: stage.right_side.as_ref().map(|c| card_to_display(&c.card)),
    }
}

pub fn player_to_display(player: &crate::player::Player) -> PlayerDisplay {
    let energy_cards: Vec<crate::card::Card> = player.energy_zone.cards.iter().map(|c| c.card.clone()).collect();
    PlayerDisplay {
        hand: zone_to_display(&player.hand.cards),
        energy: zone_to_display(&energy_cards),
        stage: stage_to_display(&player.stage),
        live_zone: zone_to_display(&player.live_card_zone.cards),
        success_live_card_zone: zone_to_display(&player.success_live_card_zone.cards),
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?}", game_state.current_phase),
        player1: player_to_display(&game_state.player1),
        player2: player_to_display(&game_state.player2),
    }
}

async fn get_game_state(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = data.game_state.lock().unwrap();
    
    // Auto-advance live phase phases (Rule 8.1.2: Live phase is automatic)
    loop {
        let current_phase = game_state.current_phase.clone();
        match current_phase {
            crate::game_state::Phase::LiveCardSet |
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                crate::turn::TurnEngine::advance_phase(&mut game_state);
            }
            _ => break,
        }
    }
    
    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(display)
}

async fn get_actions(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = data.game_state.lock().unwrap();
    
    // Auto-advance live phase phases (Rule 8.1.2: Live phase is automatic)
    loop {
        let current_phase = game_state.current_phase.clone();
        match current_phase {
            crate::game_state::Phase::LiveCardSet |
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                crate::turn::TurnEngine::advance_phase(&mut game_state);
            }
            _ => break,
        }
    }
    
    // Generate possible actions based on current game state
    let actions = generate_possible_actions(&game_state);
    
    HttpResponse::Ok().json(ActionsResponse { actions })
}

fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    // Use game_setup.rs generate_possible_actions function
    let setup_actions = crate::game_setup::generate_possible_actions(game_state);
    
    // Convert game_setup Action to web_server Action
    setup_actions.into_iter().map(|sa| Action {
        description: sa.description,
        action_type: sa.action_type,
        parameters: sa.parameters.map(|p| ActionParameters {
            card_index: p.card_index,
            card_indices: p.card_indices,
            stage_area: p.stage_area,
        }),
    }).collect()
}

async fn execute_action(
    data: web::Data<AppState>,
    req: web::Json<ExecuteActionRequest>,
) -> impl Responder {
    let action_index = req.action_index;
    let mut game_state = data.game_state.lock().unwrap();
    
    // Get possible actions
    let actions = generate_possible_actions(&game_state);
    
    if action_index >= actions.len() {
        return HttpResponse::BadRequest().json("Invalid action index");
    }
    
    let action = &actions[action_index];
    
    // Execute the action using turn engine
    let result = crate::turn::TurnEngine::execute_main_phase_action(
        &mut game_state,
        &action.action_type,
        action.parameters.as_ref().and_then(|p| p.card_index),
        action.parameters.as_ref().and_then(|p| p.card_indices.clone()),
        action.parameters.as_ref().and_then(|p| p.stage_area.clone()),
    );
    
    match result {
        Ok(_) => {
            // Auto-advance live phase phases after action execution
            loop {
                let current_phase = game_state.current_phase.clone();
                match current_phase {
                    crate::game_state::Phase::LiveCardSet |
                    crate::game_state::Phase::FirstAttackerPerformance |
                    crate::game_state::Phase::SecondAttackerPerformance |
                    crate::game_state::Phase::LiveVictoryDetermination => {
                        crate::turn::TurnEngine::advance_phase(&mut game_state);
                    }
                    _ => break,
                }
            }
            let display = game_state_to_display(&game_state);
            HttpResponse::Ok().json(display)
        }
        Err(e) => {
            HttpResponse::BadRequest().json(e)
        }
    }
}

async fn init_game(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = data.game_state.lock().unwrap();
    crate::game_setup::setup_game(&mut game_state);
    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(display)
}

pub async fn run_web_server(game_state: Arc<Mutex<GameState>>) -> std::io::Result<()> {
    let app_state = web::Data::new(AppState { game_state });
    
    // Get the web directory path
    let web_dir = PathBuf::from("../web");
    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .route("/api/game-state", web::get().to(get_game_state))
            .route("/api/actions", web::get().to(get_actions))
            .route("/api/execute-action", web::post().to(execute_action))
            .route("/api/init", web::post().to(init_game))
            .service(fs::Files::new("/decks", PathBuf::from("../game/decks")))
            .service(fs::Files::new("/cards", PathBuf::from("../cards")))
            .service(fs::Files::new("/", web_dir.clone()).index_file("index.html"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
