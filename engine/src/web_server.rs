use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

use crate::game_state::GameState;
use crate::player::Player;
use crate::card_loader;
use crate::deck_parser;
use crate::deck_builder;

#[derive(Serialize, Deserialize, Clone)]
pub struct CardDisplay {
    pub card_no: String,
    pub name: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub orientation: Option<String>,
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

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct WebAreaInfo {
    pub area: String,
    pub available: bool,
    pub cost: u32,
    pub is_baton_touch: bool,
    pub existing_member_name: Option<String>,
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
    pub stage_area: Option<String>, // Accept string from webapp, will parse to MemberArea
    pub action_type: Option<String>,
    pub card_id: Option<i16>, // Database card ID - reliable identifier
    pub card_index: Option<usize>, // Array position - kept for backward compatibility
    pub card_indices: Option<Vec<usize>>,
    pub card_no: Option<String>,
    pub use_baton_touch: Option<bool>,
}

#[derive(Deserialize)]
pub struct InitGameRequest {
    pub deck: Option<String>,
}

pub struct AppState {
    pub game_state: Arc<Mutex<GameState>>,
}

pub fn card_to_display(card_id: i16, card_db: &crate::card::CardDatabase, orientation: Option<crate::zones::Orientation>) -> Option<CardDisplay> {
    if let Some(card) = card_db.get_card(card_id) {
        Some(CardDisplay {
            card_no: card.card_no.clone(),
            name: card.name.clone(),
            card_type: format!("{:?}", card.card_type),
            orientation: orientation.map(|o| format!("{:?}", o)),
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

    PlayerDisplay {
        hand: zone_to_display_from_card_ids(&player.hand.cards, card_db),
        energy: energy_display,
        stage: stage_to_display(&player.stage, card_db),
        live_zone: zone_to_display_from_card_ids(&player.live_card_zone.cards, card_db),
        success_live_card_zone: zone_to_display_from_card_ids(&player.success_live_card_zone.cards, card_db),
        waitroom: zone_to_display_from_card_ids(&player.waitroom.cards, card_db),
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
    let game_state = data.game_state.lock().unwrap();
    
    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(display)
}

async fn get_actions(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = data.game_state.lock().unwrap();
    
    // Auto-advance automatic phases (Active, Energy, Draw) to ensure energy is activated
    loop {
        let current_phase = game_state.current_phase.clone();
        match current_phase {
            crate::game_state::Phase::Active |
            crate::game_state::Phase::Energy |
            crate::game_state::Phase::Draw => {
                crate::turn::TurnEngine::advance_phase(&mut game_state);
            }
            // LiveCardSet - auto-advance if both players are done
            crate::game_state::Phase::LiveCardSet => {
                if game_state.live_card_set_player1_done && game_state.live_card_set_player2_done {
                    // Both players finished, directly advance since advance_phase returns early for LiveCardSet
                    crate::turn::TurnEngine::check_timing(&mut game_state);
                    game_state.current_phase = crate::game_state::Phase::FirstAttackerPerformance;
                } else {
                    break;
                }
            }
            // Live phase automatic phases - auto-advance to prevent softlock
            crate::game_state::Phase::FirstAttackerPerformance |
            crate::game_state::Phase::SecondAttackerPerformance |
            crate::game_state::Phase::LiveVictoryDetermination => {
                crate::turn::TurnEngine::advance_phase(&mut game_state);
            }
            _ => break,
        }
    }
    
    // Auto-play for Player 2: keep executing random actions until it's Player 1's turn
    loop {
        // Check if it's Player 2's turn (active player is player2)
        let is_player2_turn = game_state.active_player().id == game_state.player2.id;
        
        if !is_player2_turn {
            break; // It's Player 1's turn, exit the loop
        }
        
        // Generate possible actions for Player 2
        let setup_actions = crate::game_setup::generate_possible_actions(&game_state);
        
        if setup_actions.is_empty() {
            // No actions available, advance phase
            crate::turn::TurnEngine::advance_phase(&mut game_state);
            
            // Continue loop to check if we need to keep auto-playing for Player 2
            continue;
        }
        
        // Use AI to choose a random action for Player 2
        let ai = crate::bot::ai::AIPlayer::new("Player2AI".to_string());
        let chosen_index = ai.choose_action(&setup_actions);
        let chosen_action = &setup_actions[chosen_index];
        
        println!("Player 2 (AI) choosing action: {}", chosen_action.description);
        
        // Execute the chosen action
        let result = crate::turn::TurnEngine::execute_main_phase_action(
            &mut game_state,
            &chosen_action.action_type,
            chosen_action.parameters.as_ref().and_then(|p| p.card_id),
            chosen_action.parameters.as_ref().and_then(|p| p.card_indices.clone()),
            chosen_action.parameters.as_ref().and_then(|p| p.stage_area),
            chosen_action.parameters.as_ref().and_then(|p| p.use_baton_touch),
        );
        
        if let Err(e) = result {
            eprintln!("Player 2 auto-action execution error: {}", e);
            break; // Exit on error
        }
        
        // Auto-advance automatic phases after Player 2's action
        loop {
            let current_phase = game_state.current_phase.clone();
            match current_phase {
                crate::game_state::Phase::FirstAttackerPerformance |
                crate::game_state::Phase::SecondAttackerPerformance |
                crate::game_state::Phase::LiveVictoryDetermination => {
                    crate::turn::TurnEngine::advance_phase(&mut game_state);
                }
                crate::game_state::Phase::Active |
                crate::game_state::Phase::Energy |
                crate::game_state::Phase::Draw => {
                    crate::turn::TurnEngine::advance_phase(&mut game_state);
                }
                _ => break,
            }
        }
        
        // Check if it's still Player 2's turn - if not, break to let Player 1 play
        let still_player2_turn = game_state.active_player().id == game_state.player2.id;
        if !still_player2_turn {
            break; // Turn passed to Player 1
        }
        
        // Otherwise continue the loop to play another action for Player 2
    }
    
    println!("Current phase: {:?}", game_state.current_phase);
    
    // Generate possible actions based on current game state
    let actions = generate_possible_actions(&game_state);
    println!("Generated {} actions", actions.len());
    for action in &actions {
        println!("  - {}: {}", action.action_type, action.description);
    }
    
    HttpResponse::Ok().json(ActionsResponse { actions })
}

fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    // Use game_setup.rs generate_possible_actions function
    let setup_actions = crate::game_setup::generate_possible_actions(game_state);
    
    // Convert game_setup Action to web_server Action
    setup_actions.into_iter().map(|sa| Action {
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
    }).collect()
}

async fn execute_action(
    data: web::Data<AppState>,
    req: web::Json<ExecuteActionRequest>,
) -> impl Responder {
    let _action_index = req.action_index;
    let requested_stage_area = req.stage_area.as_ref()
        .and_then(|s| s.parse::<crate::zones::MemberArea>().ok());
    let _requested_action_type = req.action_type.clone();
    let requested_card_id = req.card_id;
    let _requested_card_index = req.card_index;
    let _requested_card_no = req.card_no.clone();
    let mut game_state = data.game_state.lock().unwrap();
    
    // Parse action type from request
    let action_type = req.action_type.as_ref()
        .and_then(|t| t.parse::<crate::game_setup::ActionType>().ok())
        .unwrap_or_else(|| crate::game_setup::ActionType::Pass);
    
    // For SelectMulligan, convert card_index to card_indices to handle duplicate cards
    let card_indices = if action_type == crate::game_setup::ActionType::SelectMulligan {
        req.card_index.map(|idx| vec![idx])
    } else {
        req.card_indices.clone()
    };

    // Execute the action using turn engine with card_id directly
    // Turn engine handles card_id to card_index lookup internally
    let result = crate::turn::TurnEngine::execute_main_phase_action(
        &mut game_state,
        &action_type,
        requested_card_id,
        card_indices,
        requested_stage_area,
        req.use_baton_touch,
    );
    
    match result {
        Ok(_) => {
            // Auto-advance automatic phases after action execution
            loop {
                let current_phase = game_state.current_phase.clone();
                match current_phase {
                    // Live phase phases - LiveCardSet is manual, others are automatic
                    crate::game_state::Phase::FirstAttackerPerformance |
                    crate::game_state::Phase::SecondAttackerPerformance |
                    crate::game_state::Phase::LiveVictoryDetermination => {
                        crate::turn::TurnEngine::advance_phase(&mut game_state);
                    }
                    // Early game automatic phases
                    crate::game_state::Phase::Active |
                    crate::game_state::Phase::Energy |
                    crate::game_state::Phase::Draw => {
                        crate::turn::TurnEngine::advance_phase(&mut game_state);
                    }
                    // LiveCardSet is manual - only advance when both players explicitly finish
                    _ => break,
                }
            }
            let display = game_state_to_display(&game_state);
            HttpResponse::Ok().json(display)
        }
        Err(e) => {
            eprintln!("Action execution error: {}", e);
            HttpResponse::BadRequest().json(e)
        }
    }
}

async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let game_state = data.game_state.lock().unwrap();
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
    
    // Replace the game state in the mutex
    let mut state_guard = data.game_state.lock().unwrap();
    *state_guard = game_state;
    
    let display = game_state_to_display(&state_guard);
    HttpResponse::Ok().json(display)
}

pub async fn run_web_server(game_state: Arc<Mutex<GameState>>) -> std::io::Result<()> {
    let app_state = web::Data::new(AppState { game_state });

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
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
