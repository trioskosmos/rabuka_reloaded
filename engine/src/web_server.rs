use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use actix_files as fs;
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
    pub main_deck_count: usize,
    pub energy_deck_count: usize,
    pub waitroom_count: usize,
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
    pub use_baton_touch: Option<bool>, // Whether to use baton touch cost reduction
    // Card grouping information for improved UI
    pub card_name: Option<String>,
    pub card_no: Option<String>,
    pub base_cost: Option<u32>,
    pub final_cost: Option<u32>,
    pub available_areas: Option<Vec<crate::game_setup::AreaInfo>>,
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
    pub stage_area: Option<String>,
}

pub struct AppState {
    pub game_state: Arc<Mutex<GameState>>,
}

pub fn card_to_display(card: &crate::card::Card, orientation: Option<crate::zones::Orientation>) -> CardDisplay {
    CardDisplay {
        card_no: card.card_no.clone(),
        name: card.name.clone(),
        card_type: format!("{:?}", card.card_type),
        orientation: orientation.map(|o| format!("{:?}", o)),
    }
}

pub fn zone_to_display_from_cardinzone(cards: &[crate::zones::CardInZone]) -> ZoneDisplay {
    ZoneDisplay {
        cards: cards.iter().map(|c| card_to_display(&c.card, c.orientation)).collect(),
    }
}

pub fn zone_to_display_from_card(cards: &[crate::card::Card]) -> ZoneDisplay {
    ZoneDisplay {
        cards: cards.iter().map(|c| card_to_display(c, None)).collect(),
    }
}

pub fn stage_to_display(stage: &crate::zones::Stage) -> StageDisplay {
    StageDisplay {
        left_side: stage.left_side.as_ref().map(|c| card_to_display(&c.card, c.orientation)),
        center: stage.center.as_ref().map(|c| card_to_display(&c.card, c.orientation)),
        right_side: stage.right_side.as_ref().map(|c| card_to_display(&c.card, c.orientation)),
    }
}

pub fn player_to_display(player: &crate::player::Player) -> PlayerDisplay {
    PlayerDisplay {
        hand: zone_to_display_from_card(&player.hand.cards),
        energy: zone_to_display_from_cardinzone(&player.energy_zone.cards),
        stage: stage_to_display(&player.stage),
        live_zone: zone_to_display_from_card(&player.live_card_zone.cards),
        success_live_card_zone: zone_to_display_from_card(&player.success_live_card_zone.cards),
        main_deck_count: player.main_deck.cards.len(),
        energy_deck_count: player.energy_deck.cards.len(),
        waitroom_count: player.waitroom.cards.len(),
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
    
    // Auto-advance automatic phases
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
            _ => break,
        }
    }
    
    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(display)
}

async fn get_actions(data: web::Data<AppState>) -> impl Responder {
    let mut game_state = data.game_state.lock().unwrap();
    
    // Auto-advance automatic phases
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
            _ => break,
        }
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
        action_type: sa.action_type,
        parameters: sa.parameters.map(|p| ActionParameters {
            card_index: p.card_index,
            card_indices: p.card_indices,
            stage_area: p.stage_area,
            use_baton_touch: p.use_baton_touch,
            card_name: p.card_name,
            card_no: p.card_no,
            base_cost: p.base_cost,
            final_cost: p.final_cost,
            available_areas: p.available_areas,
        }),
    }).collect()
}

async fn execute_action(
    data: web::Data<AppState>,
    req: web::Json<ExecuteActionRequest>,
) -> impl Responder {
    let action_index = req.action_index;
    let requested_stage_area = req.stage_area.clone();
    let mut game_state = data.game_state.lock().unwrap();
    
    // Get possible actions
    let actions = generate_possible_actions(&game_state);
    
    if action_index >= actions.len() {
        return HttpResponse::BadRequest().json("Invalid action index");
    }
    
    let action = &actions[action_index];
    
    // Use stage_area from request if provided, otherwise use from action parameters
    let stage_area = requested_stage_area.or_else(|| action.parameters.as_ref().and_then(|p| p.stage_area.clone()));
    
    // Get use_baton_touch from parameters
    let use_baton_touch = action.parameters.as_ref().and_then(|p| p.use_baton_touch);
    
    // Execute the action using turn engine
    let result = crate::turn::TurnEngine::execute_main_phase_action(
        &mut game_state,
        &action.action_type,
        action.parameters.as_ref().and_then(|p| p.card_index),
        action.parameters.as_ref().and_then(|p| p.card_indices.clone()),
        stage_area,
        use_baton_touch,
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
    
    // Use first deck for both players (can be enhanced later to allow deck selection)
    let deck1 = &deck_lists[0];
    let deck2 = &deck_lists[0];
    
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);
    
    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
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
    
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
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
    
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    // Create fresh players
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    // Create fresh game state
    let mut game_state = GameState::new(player1, player2);
    
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
