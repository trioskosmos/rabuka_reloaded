use rabuka_engine::*;
use rabuka_engine::game_setup::{ActionParameters, AreaInfo};
use rabuka_engine::player::Player;
use rabuka_engine::game_state::GameState;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Action {
    pub description: String,
    pub action_type: String,
    pub parameters: Option<ActionParameters>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ActionsResponse {
    actions: Vec<Action>,
}

pub fn game_state_to_display(game_state: &game_state::GameState) -> display::GameStateDisplay {
    display::game_state_to_display(game_state)
}

static GAME_STATE: Mutex<Option<game_state::GameState>> = Mutex::new(None);

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 {
        match args[1].as_str() {
            "get-state" => {
                output_game_state();
            }
            "get-actions" => {
                output_actions();
            }
            "execute-action" => {
                if args.len() > 2 {
                    if let Ok(index) = args[2].parse::<usize>() {
                        execute_action(index);
                    }
                }
            }
            "init" => {
                initialize_game();
            }
            "web-server" => {
                run_web_server();
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
            }
        }
    } else {
        initialize_game();
    }
}



fn output_game_state() {
    let game_state = GAME_STATE.lock().unwrap();
    if let Some(ref state) = *game_state {
        let display = game_state_to_display(state);
        println!("{}", serde_json::to_string(&display).unwrap());
    } else {
        eprintln!("Game not initialized. Run 'init' command first.");
    }
}

fn initialize_game() {
    // Load cards from cards.json
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => cards,
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };

    // Create CardDatabase from loaded cards
    let card_database = Arc::new(rabuka_engine::card::CardDatabase::load_or_create(cards));

    // Load sample decks from game/decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };

    // Let players choose decks
    let deck1 = choose_deck(&deck_lists, "Player 1");
    let deck2 = choose_deck(&deck_lists, "Player 2");

    // Build decks from chosen deck lists using card IDs
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck2);

    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_database(&card_database, card_numbers1) {
        Ok(mut deck) => {
            deck.shuffle_main_deck();
            deck.shuffle_energy_deck();
            deck
        }
        Err(e) => {
            eprintln!("Failed to build deck for Player 1: {}", e);
            return;
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
            return;
        }
    };

    // Add default energy cards if needed
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player1_deck, &card_database);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards_from_database(&mut player2_deck, &card_database);

    // Initialize players with decks
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);

    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);

    // Initialize game state with CardDatabase
    let mut game_state = GameState::new(player1, player2, card_database);

    // Game setup (Rule 6.2)
    game_setup::setup_game(&mut game_state);

    // Store in global state
    *GAME_STATE.lock().unwrap() = Some(game_state);

    println!("Game initialized successfully");
}

fn output_actions() {
    let game_state = GAME_STATE.lock().unwrap();
    if let Some(ref state) = *game_state {
        let actions = game_setup::generate_possible_actions(state)
            .into_iter()
            .map(|sa| Action {
                description: sa.description,
                action_type: sa.action_type.to_string(),
                parameters: sa.parameters.map(|p| ActionParameters {
                    card_id: p.card_id,
                    card_index: p.card_index,
                    card_indices: p.card_indices,
                    stage_area: p.stage_area,
                    use_baton_touch: p.use_baton_touch,
                    card_name: p.card_name,
                    card_no: p.card_no,
                    base_cost: p.base_cost,
                    final_cost: p.final_cost,
                    available_areas: p.available_areas.map(|areas| areas.into_iter().map(|ai| AreaInfo {
                        area: ai.area,
                        available: ai.available,
                        cost: ai.cost,
                        is_baton_touch: ai.is_baton_touch,
                        existing_member_name: ai.existing_member_name,
                    }).collect()),
                }),
            })
            .collect();
        let response = ActionsResponse { actions };
        println!("{}", serde_json::to_string(&response).unwrap());
    } else {
        eprintln!("Game not initialized. Run 'init' command first.");
    }
}

fn execute_action(index: usize) {
    let mut game_state = GAME_STATE.lock().unwrap();
    if let Some(ref mut state) = *game_state {
        let actions = game_setup::generate_possible_actions(state);
        
        if index >= actions.len() {
            eprintln!("Invalid action index");
            return;
        }
        
        let action = &actions[index];
        println!("Executing action: {}", action.description);
        
        // Execute the action (simplified - in real implementation would call execute_action from web_server)
        match action.action_type.to_string().as_str() {
            "activate_energy" => {
                state.player1.activate_all_energy();
            }
            "draw_card" => {
                state.player1.draw_card();
            }
            _ => {
                // Handle unknown action types gracefully
                eprintln!("Warning: Unknown action type '{}' - no specific handler implemented", action.action_type);
            }
        }
        
        println!("Action executed successfully");
    } else {
        eprintln!("Game not initialized. Run 'init' command first.");
    }
}

fn choose_deck(deck_lists: &[deck_parser::DeckList], player_name: &str) -> deck_parser::DeckList {
    // For now, just pick the first deck
    // In a real implementation, this would prompt the player for their choice
    println!("{} chose: {}", player_name, deck_lists[0].name);
    deck_lists[0].clone()
}

fn run_web_server() {
    println!("Web server starting on http://127.0.0.1:8080");
    match tokio::runtime::Runtime::new() {
        Ok(runtime) => {
            match runtime.block_on(web_server::run_web_server()) {
                Ok(_) => println!("Server shutdown gracefully"),
                Err(e) => eprintln!("Server error: {}", e),
            }
        }
        Err(e) => eprintln!("Failed to create runtime: {}", e),
    }
}
