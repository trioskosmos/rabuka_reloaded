mod card;
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

use player::Player;
use game_state::GameState;
use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use serde_json;

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionsResponse {
    pub actions: Vec<game_setup::Action>,
}

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
    }
}

pub fn game_state_to_display(game_state: &GameState) -> GameStateDisplay {
    GameStateDisplay {
        turn: game_state.turn_number,
        phase: format!("{:?} - {:?}", game_state.current_turn_phase, game_state.current_phase),
        player1: player_to_display(&game_state.player1),
        player2: player_to_display(&game_state.player2),
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
    let mut game_state = GameState::new(player1, player2);
    
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
        Ok(cards) => {
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
        Ok(decks) => decks,
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
    
    // Initialize players with decks
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    // Initialize game state
    let mut game_state = GameState::new(player1, player2);
    
    // Game setup (Rule 6.2)
    game_setup::setup_game(&mut game_state);
    
    // Store in global state
    *GAME_STATE.lock().unwrap() = Some(game_state);
    
    println!("Game initialized successfully");
}

fn output_actions() {
    let game_state = GAME_STATE.lock().unwrap();
    if let Some(ref state) = *game_state {
        let actions = game_setup::generate_possible_actions(state);
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
        match action.action_type.as_str() {
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
    // TODO: Implement actual player choice
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
    
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player1_deck, &cards);
    let _ = deck_builder::DeckBuilder::add_default_energy_cards(&mut player2_deck, &cards);
    
    let mut player1 = Player::new("player1".to_string(), "Player 1".to_string(), true);
    let mut player2 = Player::new("player2".to_string(), "Player 2".to_string(), false);
    
    player1.set_main_deck(player1_deck.main_deck);
    player1.set_energy_deck(player1_deck.energy_deck);
    
    player2.set_main_deck(player2_deck.main_deck);
    player2.set_energy_deck(player2_deck.energy_deck);
    
    let mut game_state = GameState::new(player1, player2);
    game_setup::setup_game(&mut game_state);
    
    // Start web server with game state
    let game_state = Arc::new(Mutex::new(game_state));
    
    println!("Web server starting on http://127.0.0.1:8080");
    
    // This will block until the server is stopped
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _ = runtime.block_on(web_server::run_web_server(game_state));
}
