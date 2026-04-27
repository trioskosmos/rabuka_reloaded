mod card;
mod constants;
mod zones;
mod player;
mod game_state;
mod turn;
mod card_loader;
mod deck_builder;
mod deck_parser;
mod web_server;
mod bot;
mod game_setup;
mod ability_resolver;

use player::Player;
use game_state::GameState;
use card::CardDatabase;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use serde_json;
use game_setup::{ActionParameters, AreaInfo};

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionsResponse {
    pub actions: Vec<Action>,
}

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

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize, Clone)]
pub struct Action {
    pub description: String,
    pub action_type: String,
    pub parameters: Option<ActionParameters>,
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

pub fn zone_to_display(card_ids: &[i16], card_db: &crate::card::CardDatabase) -> ZoneDisplay {
    ZoneDisplay {
        cards: card_ids.iter().filter_map(|&id| card_to_display(id, card_db, None)).collect(),
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
    let energy_cards: Vec<(i16, Option<crate::zones::Orientation>)> = player.energy_zone.cards.iter()
        .enumerate()
        .map(|(i, &card_id)| {
            // Simplified: first active_energy_count cards are active, rest are wait
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
        energy: energy_display,
        hand: zone_to_display(&player.hand.cards, card_db),
        stage: stage_to_display(&player.stage, card_db),
        live_zone: zone_to_display(&player.live_card_zone.cards, card_db),
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?} - {:?}", game_state.current_turn_phase, game_state.current_phase),
        player1: player_to_display(&game_state.player1, &game_state.card_database),
        player2: player_to_display(&game_state.player2, &game_state.card_database),
    }
}

// Global game state for CLI commands
static GAME_STATE: Mutex<Option<GameState>> = Mutex::new(None);

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
            "headless" => {
                bot::headless::run_headless_game();
            }
            "interactive" => {
                bot::headless::run_interactive_headless();
            }
            "test" => {
                bot::test_mode::run_test_mode();
            }
            "ability" => {
                bot::ability_test::run_ability_test();
            }
            "tournament" => {
                bot::tournament::run_tournament();
            }
            _ => {
                eprintln!("Unknown command: {}", args[1]);
            }
        }
    } else {
        // Default: run full game initialization
        run_game();
    }
}

fn run_game() {
    println!("Love Live! Card Game Engine");
    
    // Load cards from cards.json
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            println!("Loaded {} cards", cards.len());
            // Convert Vec<Card> to HashMap<String, Card>
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    // Load sample decks from game/decks
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => {
            println!("Loaded {} sample decks:", decks.len());
            for deck in &decks {
                println!("  - {}", deck.name);
            }
            decks
        }
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };
    
    // Let players choose decks
    let deck1 = choose_deck(&deck_lists, "Player 1");
    let deck2 = choose_deck(&deck_lists, "Player 2");
    
    // Build decks from chosen deck lists
    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(&deck2);
    
    let mut player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
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
    
    let mut player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
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
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    println!("Player 1 deck: {}", deck1.name);
    println!("  Main deck: {} cards", player1_deck.main_deck.len());
    println!("  Energy deck: {} cards", player1_deck.energy_deck.len());
    
    println!("Player 2 deck: {}", deck2.name);
    println!("  Main deck: {} cards", player2_deck.main_deck.len());
    println!("  Energy deck: {} cards", player2_deck.energy_deck.len());
    
    // Initialize players with decks
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    // Initialize game state
    let card_database = Arc::new(CardDatabase::load_or_create(
        cards.values().cloned().collect()
    ));
    let mut game_state = GameState::new(player1, player2, card_database);
    
    println!("Game initialized");
    println!("Turn: {}", game_state.turn_number);
    println!("Phase: {:?}", game_state.current_phase);
    
    // Game setup (Rule 6.2)
    game_setup::setup_game(&mut game_state);
    
    println!("Game setup complete");
    println!("Player 1 hand: {} cards", game_state.player1.hand.len());
    println!("Player 2 hand: {} cards", game_state.player2.hand.len());
    println!("Player 1 energy: {} cards", game_state.player1.energy_zone.cards.len());
    println!("Player 2 energy: {} cards", game_state.player2.energy_zone.cards.len());
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
    let card_database = Arc::new(CardDatabase::load_or_create(cards));

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
                eprintln!("Action execution not implemented for: {}", action.action_type);
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
    println!("Starting web server...");
    
    // Load cards and initialize game state
    let cards_path = std::path::Path::new("../cards/cards.json");
    let cards = match card_loader::CardLoader::load_cards_from_file(cards_path) {
        Ok(cards) => {
            let mut card_map = std::collections::HashMap::new();
            for card in cards {
                card_map.insert(card.card_no.clone(), card);
            }
            card_map
        }
        Err(e) => {
            eprintln!("Failed to load cards: {}", e);
            return;
        }
    };
    
    let deck_lists = match deck_parser::DeckParser::parse_all_decks() {
        Ok(decks) => decks,
        Err(e) => {
            eprintln!("Failed to load decks: {}", e);
            return;
        }
    };
    
    let deck1 = &deck_lists[0];
    let deck2 = &deck_lists[0];

    let card_numbers1 = deck_parser::DeckParser::deck_list_to_card_numbers(deck1);
    let card_numbers2 = deck_parser::DeckParser::deck_list_to_card_numbers(deck2);

    // Build decks before consuming cards
    let player1_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers1) {
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

    let player2_deck = match deck_builder::DeckBuilder::build_deck_from_card_map(&cards, card_numbers2) {
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

    // Create CardDatabase from loaded cards - convert HashMap values to Vec
    let card_vec: Vec<crate::card::Card> = cards.into_values().collect();
    let card_database = std::sync::Arc::new(crate::card::CardDatabase::load_or_create(card_vec));
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);

    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);

    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);

    let mut game_state = GameState::new(player1, player2, card_database.clone());
    game_setup::setup_game(&mut game_state);
    
    // Start web server with game state
    let game_state = Arc::new(Mutex::new(game_state));
    
    println!("Web server starting on http://127.0.0.1:8080");
    
    // This will block until the server is stopped
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _ = runtime.block_on(web_server::run_web_server(game_state));
}
