use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

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
pub struct Action {
    pub description: String,
    pub action_type: String,
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
    PlayerDisplay {
        hand: zone_to_display(&player.hand.cards),
        energy: zone_to_display(&player.energy_zone.cards.iter().map(|c| &c.card).collect::<Vec<_>>()),
        stage: stage_to_display(&player.stage),
        live_zone: zone_to_display(&player.live_card_zone.cards),
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
    let game_state = data.game_state.lock().unwrap();
    let display = game_state_to_display(&game_state);
    HttpResponse::Ok().json(display)
}

async fn get_actions(data: web::Data<AppState>) -> impl Responder {
    let game_state = data.game_state.lock().unwrap();
    
    // Generate possible actions based on current game state
    let actions = generate_possible_actions(&game_state);
    
    HttpResponse::Ok().json(ActionsResponse { actions })
}

fn generate_possible_actions(game_state: &GameState) -> Vec<Action> {
    let mut actions = Vec::new();
    let active_player = game_state.active_player();
    
    // Filter actions based on current phase and legal action validation
    match game_state.current_phase {
        crate::game_state::Phase::Active => {
            // Rule 7.1: Active Phase - Can activate energy cards
            if active_player.can_activate_energy() {
                actions.push(Action {
                    description: "Activate all energy cards".to_string(),
                    action_type: "activate_energy".to_string(),
                });
            }
        }
        crate::game_state::Phase::Energy => {
            // Rule 7.2: Energy Phase - Can play energy cards from hand
            if active_player.can_play_energy_to_zone() {
                actions.push(Action {
                    description: "Play energy card from hand to energy zone".to_string(),
                    action_type: "play_energy_to_zone".to_string(),
                });
            }
        }
        crate::game_state::Phase::Draw => {
            // Rule 8.1: Draw Phase - Can draw card
            if active_player.can_draw_card() {
                actions.push(Action {
                    description: "Draw card from main deck".to_string(),
                    action_type: "draw_card".to_string(),
                });
            }
        }
        crate::game_state::Phase::Main => {
            // Rule 8.2: Main Phase - Can play member cards to stage
            if active_player.can_play_member_to_stage() {
                actions.push(Action {
                    description: "Play member card from hand to stage".to_string(),
                    action_type: "play_member_to_stage".to_string(),
                });
            }
            
            // Rule 5.5: Can shuffle deck if not empty
            if active_player.can_shuffle_zone("deck") {
                actions.push(Action {
                    description: "Shuffle main deck".to_string(),
                    action_type: "shuffle_deck".to_string(),
                });
            }
            
            // Rule 5.7: Can look at top cards
            if active_player.can_look_at_top(1) {
                actions.push(Action {
                    description: "Look at top card of main deck".to_string(),
                    action_type: "look_at_top".to_string(),
                });
            }
            
            // Rule 5.8: Can swap cards between areas
            if active_player.can_swap_cards(
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
            ) {
                actions.push(Action {
                    description: "Swap left side and center members".to_string(),
                    action_type: "swap_left_center".to_string(),
                });
            }
            if active_player.can_swap_cards(
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            ) {
                actions.push(Action {
                    description: "Swap center and right side members".to_string(),
                    action_type: "swap_center_right".to_string(),
                });
            }
            if active_player.can_swap_cards(
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::RightSide,
            ) {
                actions.push(Action {
                    description: "Swap left side and right side members".to_string(),
                    action_type: "swap_left_right".to_string(),
                });
            }
            
            // Rule 5.9: Can pay energy
            if active_player.can_pay_energy(1) {
                actions.push(Action {
                    description: "Pay 1 energy".to_string(),
                    action_type: "pay_energy_1".to_string(),
                });
            }
            if active_player.can_pay_energy(2) {
                actions.push(Action {
                    description: "Pay 2 energy".to_string(),
                    action_type: "pay_energy_2".to_string(),
                });
            }
            
            // Rule 5.10: Can place energy under member
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::LeftSide) {
                actions.push(Action {
                    description: "Place energy under left side member".to_string(),
                    action_type: "place_energy_under_left".to_string(),
                });
            }
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::Center) {
                actions.push(Action {
                    description: "Place energy under center member".to_string(),
                    action_type: "place_energy_under_center".to_string(),
                });
            }
            if active_player.can_place_energy_under_member(crate::zones::MemberArea::RightSide) {
                actions.push(Action {
                    description: "Place energy under right side member".to_string(),
                    action_type: "place_energy_under_right".to_string(),
                });
            }
        }
        crate::game_state::Phase::LiveCardSet => {
            // Rule 9.1: Live Card Set Phase - Can place cards in live zone
            if active_player.can_place_in_live_zone() {
                actions.push(Action {
                    description: "Place card in Live Card Zone".to_string(),
                    action_type: "place_in_live_zone".to_string(),
                });
            }
        }
        crate::game_state::Phase::FirstAttackerPerformance
        | crate::game_state::Phase::SecondAttackerPerformance
        | crate::game_state::Phase::LiveVictoryDetermination => {
            // Live phase actions - currently no specific actions
        }
    }
    
    actions
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
    let active_player = game_state.active_player_mut();
    
    // Execute the action
    let result = match action.action_type.as_str() {
        "activate_energy" => {
            active_player.activate_all_energy();
            Ok(())
        }
        "play_energy_to_zone" => {
            // TODO: Implement card selection from hand
            Err("Card selection not implemented".to_string())
        }
        "draw_card" => {
            active_player.draw_card().ok_or("Deck empty".to_string())?;
            Ok(())
        }
        "play_member_to_stage" => {
            // TODO: Implement card selection and area selection
            Err("Card selection not implemented".to_string())
        }
        "shuffle_deck" => {
            active_player.shuffle_zone("deck")
        }
        "look_at_top" => {
            // This just looks at cards, doesn't change state
            // In a real implementation, this would return the cards to the client
            Ok(())
        }
        "swap_left_center" => {
            active_player.swap_cards(
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::Center,
            )
        }
        "swap_center_right" => {
            active_player.swap_cards(
                crate::zones::MemberArea::Center,
                crate::zones::MemberArea::RightSide,
            )
        }
        "swap_left_right" => {
            active_player.swap_cards(
                crate::zones::MemberArea::LeftSide,
                crate::zones::MemberArea::RightSide,
            )
        }
        "pay_energy_1" => {
            active_player.pay_energy(1)
        }
        "pay_energy_2" => {
            active_player.pay_energy(2)
        }
        "place_energy_under_left" => {
            // TODO: Implement energy card selection
            Err("Energy selection not implemented".to_string())
        }
        "place_energy_under_center" => {
            // TODO: Implement energy card selection
            Err("Energy selection not implemented".to_string())
        }
        "place_energy_under_right" => {
            // TODO: Implement energy card selection
            Err("Energy selection not implemented".to_string())
        }
        "place_in_live_zone" => {
            // TODO: Implement card selection
            Err("Card selection not implemented".to_string())
        }
        _ => {
            Err("Unknown action type".to_string())
        }
    };
    
    match result {
        Ok(_) => {
            let display = game_state_to_display(&game_state);
            HttpResponse::Ok().json(display)
        }
        Err(e) => {
            HttpResponse::BadRequest().json(e)
        }
    }
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
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
